//! The marketplace: packs on cards with their covers (a GIF moves), and on the automations page the flows and the
//! packs that run something. A search over names, lines, tags, presets and topics; filters by where they came from
//! and by tag (each tag its own colour). A card opens the pack's own sheet, with a micro illustration for its kind:
//! what is in it, then turn it on or off, share it (the system's share sheet), export it as a file, delete it, trust
//! it. Packs come in from the community's list, a file or folder, or any git repository.
use crate::app::icons::{self, icon, illustration, Icon};
use crate::app::runs::{Listed, Pack};
use crate::app::theme::{brand, Brand, BOLD, HEADER, LIGHT};
use crate::app::ui::{card, icon_button, pill, primary, sheet, tag, text};
use crate::app::workspace::{Route, Workspace};
use gpui_kit::component::input::Input;
use gpui_kit::component::switch::Switch;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// Packs that run something: an automation, or a source's program.
fn runs_something(kinds: &[String]) -> bool {
    kinds.iter().any(|k| k == "automation" || k == "source")
}

/// A pack's cover standing still: a GIF's first frame. It moves only in the pack's own sheet.
fn still(p: &Pack) -> Option<Img> {
    let gif = crate::packs::built_in_gif(&p.id).map(|g| g.to_vec())
        .or_else(|| p.preview_file.as_ref().filter(|f| f.extension().is_some_and(|e| e.eq_ignore_ascii_case("gif"))).and_then(|f| std::fs::read(f).ok()));
    let Some(gif) = gif else { return picture(p) };
    icons::picture(&format!("{}:still", p.id), || Some((ImageFormat::Png, first_frame(&gif)?)))
}

/// A GIF's first frame as a PNG.
pub fn first_frame(gif: &[u8]) -> Option<Vec<u8>> {
    let img = image::load_from_memory_with_format(gif, image::ImageFormat::Gif).ok()?;
    let mut out = std::io::Cursor::new(vec![]);
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

fn picture(p: &Pack) -> Option<Img> {
    icons::picture(&p.id, || {
        // a built-in pack's moving cover
        if let Some(gif) = crate::packs::built_in_gif(&p.id) {
            return Some((ImageFormat::Gif, gif.to_vec()));
        }
        if let Some(svg) = &p.preview_svg {
            return Some((ImageFormat::Svg, svg.clone().into_bytes()));
        }
        let f = p.preview_file.as_ref()?;
        let format = match f.extension()?.to_string_lossy().to_lowercase().as_str() {
            "svg" => ImageFormat::Svg,
            "png" => ImageFormat::Png,
            "webp" => ImageFormat::Webp,
            "gif" => ImageFormat::Gif,
            _ => ImageFormat::Jpeg,
        };
        Some((format, std::fs::read(f).ok()?))
    })
}

/// An added pack's folder, to read what it runs before trusting it.
fn folder(id: &str) -> std::path::PathBuf {
    crate::util::home().join(".config/edits/packs").join(id)
}

fn kind_label(p: &Pack) -> &'static str {
    if p.is("automation") { "Automation" } else if p.is("source") { "Source" } else if p.is("feed") { "Topics" } else if p.scene { "3D scene" } else if p.is("preset") { "Styles" } else { "Agent skill" }
}

/// The micro illustration for what a pack is.
pub fn art(p: &Pack) -> &'static str {
    if p.is("automation") || p.is("source") { "automations" } else if p.is("feed") { "topics" } else if p.scene { "scene" } else if p.is("preset") { "styles" } else { "skills" }
}

/// Whether a pack holds every word searched: in its name, line, tags, presets or topics.
pub fn found(p: &Pack, q: &str) -> bool {
    let mut hay = format!("{} {} {} {} {}", p.name, p.description, p.author, p.tags.join(" "), kind_label(p));
    for x in &p.presets {
        hay += &format!(" {} {}", x.name, x.about);
    }
    for t in &p.topics {
        hay += &format!(" {} {}", t.key, t.name);
    }
    let hay = hay.to_lowercase();
    q.to_lowercase().split_whitespace().all(|w| hay.contains(w))
}

/// A click that runs `edits packs VERB ID`.
fn act(verb: &'static str, id: String, view: WeakEntity<Workspace>) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    move |_, window, cx| {
        let _ = view.update(cx, |this, cx| this.pack_action(verb, id.clone(), window, cx));
    }
}

const ORIGINS: &[(Option<&str>, &str)] = &[(None, "All"), (Some("built-in"), "Built in"), (Some("community"), "Community"), (Some("local"), "Yours")];

impl Workspace {
    fn pack_card(&self, i: usize, p: &Pack, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let id = p.id.clone();
        let origin = match p.origin.as_str() { "built-in" => "Built in", "local" => "Yours", _ => "Community" };
        let count = if !p.presets.is_empty() { format!("{} styles", p.presets.len()) } else if !p.topics.is_empty() { format!("{} topics", p.topics.len()) } else { kind_label(p).to_string() };
        let hover = b.hair;
        div().id(("pack", i)).w(px(288.)).flex().flex_col().gap(px(8.)).p(px(12.)).rounded(px(16.)).bg(b.card).cursor_pointer().hover(move |s| s.bg(hover))
            .when(!p.enabled, |d| d.opacity(0.5))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.market.open = Some(id.clone());
                cx.notify();
            }))
            .child(div().w_full().h(px(144.)).rounded(px(10.)).overflow_hidden().bg(b.hair).relative()
                .children(still(p).map(|img| img.size_full().object_fit(ObjectFit::Cover)))
                .when(!p.enabled, |d| d.child(div().absolute().top(px(8.)).right(px(8.)).px(px(8.)).h(px(22.)).flex().items_center().rounded(px(11.)).bg(black().opacity(0.6)).child(text("Off", 11., BOLD, white())))))
            .child(text(p.name.clone(), 15., BOLD, b.ink).px(px(4.)).truncate())
            .child(text(format!("{count} · {origin}"), 12., FontWeight::NORMAL, b.gray).px(px(4.)).truncate())
            .child(text(p.description.clone(), 13., LIGHT, b.gray).px(px(4.)).h(px(40.)).line_clamp(2))
            .when(!p.tags.is_empty(), |d| d.child(div().px(px(4.)).flex().gap(px(6.)).overflow_hidden().children(p.tags.iter().filter(|t| *t != "rss").take(3).map(|t| tag(t, b)))))
    }

    fn listed_row(&self, i: usize, l: &Listed, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let id = l.id.clone();
        let busy = self.market.busy.as_deref() == Some(l.id.as_str());
        div().py(px(8.)).flex().items_center().gap(px(16.)).when(i > 0, |d| d.border_t_1().border_color(b.hair))
            .child(illustration("community").w(px(40.)).h(px(30.)).flex_none())
            .child(div().flex_1().min_w_0().flex().flex_col().gap(px(4.))
                .child(text(l.name.clone(), 14., BOLD, b.ink).truncate())
                .child(text(l.description.clone(), 13., LIGHT, b.gray).truncate()))
            .child(if l.added || busy {
                text(if busy { "Adding..." } else { "Added" }, 13., FontWeight::NORMAL, b.gray).into_any_element()
            } else {
                pill(("add", i), Some(Icon::Download), "Add", false, b).on_click(cx.listener(move |this, _, window, cx| this.pack_action("add", id.clone(), window, cx))).into_any_element()
            })
    }

    /// Where from, which tag, newest first: the row above the cards.
    fn filters(&self, list: &[Pack], b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let origins = ORIGINS.iter().enumerate().map(|(i, &(o, label))| {
            pill(("origin", i), None, label, self.market.origin == o, b).on_click(cx.listener(move |this, _, _, cx| {
                this.market.origin = o;
                cx.notify();
            }))
        }).collect::<Vec<_>>();
        let newest = pill("newest", None, "Newest first", self.market.newest, b).on_click(cx.listener(|this, _, _, cx| {
            this.market.newest = !this.market.newest;
            cx.notify();
        }));
        // the tags most packs here carry
        let mut count: Vec<(String, usize)> = vec![];
        // "rss" says how, not what: the topics' own tags say what
        for t in list.iter().flat_map(|p| &p.tags).filter(|t| *t != "rss") {
            match count.iter_mut().find(|x| &x.0 == t) {
                Some(x) => x.1 += 1,
                None => count.push((t.clone(), 1)),
            }
        }
        count.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let tags = count.into_iter().take(16).enumerate().map(|(i, (t, _))| {
            let on = self.market.tag.as_deref() == Some(t.as_str());
            div().id(("tag", i)).cursor_pointer().rounded(px(12.)).when(on, |d| d.border_2().border_color(b.ink)).when(self.market.tag.is_some() && !on, |d| d.opacity(0.45))
                .child(tag(&t, b))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.market.tag = if on { None } else { Some(t.clone()) };
                    cx.notify();
                }))
        }).collect::<Vec<_>>();
        div().flex().flex_col().gap(px(12.))
            .child(div().flex().flex_wrap().gap(px(8.)).children(origins).child(div().w(px(1.)).h(px(16.)).bg(b.hair)).child(newest))
            .child(div().flex().flex_wrap().gap(px(6.)).children(tags))
    }

    /// The top of the page: the micro illustration, what this place is, and how packs come in.
    fn hero(&self, automations: bool, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let (art, title, about) = if automations {
            ("automations", "Let it run by itself", "Pick a topic and where it goes: each new story becomes an edit and is posted for you. Packs that post or find things run alone, after you read and trust them.")
        } else {
            ("styles", "Every style, one click away", "Styles for your edits, topics to make videos about as news comes, 3D scenes to lay over them, what an agent knows. Turn any off, share yours, bring one in from a file or a git repository.")
        };
        div().w_full().p(px(24.)).flex().items_center().gap(px(24.)).rounded(px(20.)).bg(b.card)
            .child(illustration(art).w(px(160.)).h(px(120.)).flex_none())
            .child(div().flex_1().min_w_0().flex().flex_col().gap(px(8.))
                .child(text(title, 24., BOLD, b.ink))
                .child(text(about, 14., LIGHT, b.gray))
                .child(div().pt(px(8.)).flex().flex_wrap().gap(px(8.))
                    .child(Input::new(&self.market_search).prefix(icon(Icon::Search, b.gray).size(px(14.))).suffix(crate::app::sidebar::shortcut(b)).cleanable(true).w(px(320.)))
                    .when(!automations, |d| d.child(pill("make-pack", Some(Icon::Package), "Make one from a video", false, b)
                        .on_click(cx.listener(|this, _, window, cx| this.make_pack(window, cx)))))))
    }

    /// Import: a pack file, a folder, or a git repository.
    fn import_row(&self, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex().flex_wrap().items_center().gap(px(8.))
            .child(pill("import-file", Some(Icon::Import), "Import a file", false, b).on_click(cx.listener(|this, _, window, cx| this.import_pack(false, window, cx))))
            .child(pill("import-folder", Some(Icon::Folder), "Import a folder", false, b).on_click(cx.listener(|this, _, window, cx| this.import_pack(true, window, cx))))
            .child(pill("import-git", Some(Icon::Git), "From git", self.market.git, b).on_click(cx.listener(|this, _, _, cx| {
                this.market.git = !this.market.git;
                cx.notify();
            })))
            .when(self.market.git, |d| d
                .child(div().w(px(320.)).child(Input::new(&self.git_url).prefix(icon(Icon::Git, b.gray).size(px(14.)))))
                .child(primary("git-add", "Add", b).on_click(cx.listener(|this, _, window, cx| {
                    let url = this.git_url.read(cx).value().trim().to_string();
                    if url.is_empty() {
                        return;
                    }
                    this.git_url.update(cx, |s, cx| s.set_value("", window, cx));
                    this.market.git = false;
                    this.pack_action("add", url, window, cx);
                }))))
    }

    /// The pack's own sheet: its picture and illustration, what is in it, and everything that can be done with it.
    pub fn pack_sheet(&self, id: &str, cx: &mut Context<Self>) -> Option<AnyElement> {
        let b = brand(cx);
        let p = self.market.packs.as_ref()?.as_ref().ok()?.iter().find(|p| p.id == id)?.clone();
        let busy = self.market.busy.as_deref() == Some(p.id.as_str());
        let view = cx.entity().downgrade();
        let close = cx.listener(|this: &mut Self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>| {
            this.market.open = None;
            cx.notify();
        });
        let (backdrop, card_) = sheet("pack-sheet", 720., &b, close);
        let origin = match p.origin.as_str() { "built-in" => "Built in", "local" => "Made here", _ => "From the community" };
        // what is in it
        let mut inside: Vec<AnyElement> = vec![];
        for (i, x) in p.presets.iter().enumerate() {
            let name = x.name.clone();
            inside.push(div().py(px(8.)).flex().items_center().gap(px(12.)).when(i > 0, |d| d.border_t_1().border_color(b.hair))
                .child(icon(Icon::Sliders, b.gray).size(px(16.)))
                .child(div().flex_1().min_w_0().flex().flex_col().gap(px(2.)).child(text(x.name.clone(), 14., BOLD, b.ink)).child(text(x.about.clone(), 13., LIGHT, b.gray).truncate()))
                .child(pill(("use-style", i), None, "Use", false, &b).on_click(cx.listener(move |this, _, window, cx| {
                    this.preset = Some(name.clone());
                    this.go(Route::New, cx);
                    this.input.update(cx, |s, cx| s.focus(window, cx));
                })))
                .into_any_element());
        }
        if !p.topics.is_empty() {
            inside.push(div().flex().flex_wrap().gap(px(6.)).children(p.topics.iter().map(|t| tag(&t.name, &b))).into_any_element());
        }
        if p.scene {
            let (key, name) = (p.short(), p.name.clone());
            inside.push(div().flex().items_center().gap(px(12.))
                .child(text("Laid over an edit as a 3D overlay; it runs alone, in a browser with no window and nothing of yours.", 13., LIGHT, b.gray).flex_1())
                .child(pill("use-scene", Some(Icon::Cube), "Use", false, &b).on_click(cx.listener(move |this, _, window, cx| {
                    this.overlay = Some((key.clone(), name.clone()));
                    this.go(Route::New, cx);
                    this.input.update(cx, |s, cx| s.focus(window, cx));
                })))
                .into_any_element());
        }
        // what can be done with it
        let mut actions: Vec<AnyElement> = vec![];
        if busy {
            actions.push(text("Working...", 13., FontWeight::NORMAL, b.gray).into_any_element());
        } else {
            let (sid, eid) = (p.id.clone(), p.id.clone());
            actions.push(pill("sheet-share", Some(Icon::Share), "Share", false, &b).on_click(cx.listener(move |this, _, window, cx| this.share_pack(sid.clone(), window, cx))).into_any_element());
            actions.push(pill("sheet-export", Some(Icon::Export), "Export", false, &b).on_click(cx.listener(move |this, _, window, cx| this.export_pack(eid.clone(), window, cx))).into_any_element());
            if p.origin == "local" {
                actions.push(pill("sheet-publish", Some(Icon::Github), "Send to the marketplace", false, &b).on_click(act("share", p.id.clone(), view.clone())).into_any_element());
            }
            if p.enabled && !p.trusted {
                let read = folder(&p.id);
                actions.push(pill("sheet-read", Some(Icon::Eye), "Read what it runs", false, &b).on_click(move |_, _, cx| cx.open_with_system(&read)).into_any_element());
                actions.push(primary("sheet-trust", "Trust it", &b).on_click(act("trust", p.id.clone(), view.clone())).into_any_element());
            }
            if p.origin != "built-in" {
                actions.push(div().flex_1().into_any_element());
                actions.push(pill("sheet-delete", Some(Icon::Trash), "Delete", false, &b).text_color(b.red_text).on_click(act("remove", p.id.clone(), view.clone())).into_any_element());
            }
        }
        let (onid, on) = (p.id.clone(), p.enabled);
        let toggle = Switch::new("sheet-on").checked(on).on_click({
            let view = view.clone();
            move |checked: &bool, window, cx| {
                let (verb, id) = (if *checked { "on" } else { "off" }, onid.clone());
                let _ = view.update(cx, |this, cx| this.pack_action(verb, id, window, cx));
            }
        });
        let close_x = icon_button("sheet-close", Icon::Failed, &b).on_click(cx.listener(|this, _, _, cx| {
            this.market.open = None;
            cx.notify();
        }));
        let body = card_
            .child(div().w_full().h(px(220.)).flex_none().relative().overflow_hidden().bg(b.hair)
                .children(picture(&p).map(|img| img.size_full().object_fit(ObjectFit::Cover)))
                .child(div().absolute().top(px(12.)).right(px(12.)).rounded_full().bg(b.card).child(close_x)))
            .child(div().p(px(24.)).flex().flex_col().gap(px(16.))
                .child(div().flex().items_start().gap(px(16.))
                    .child(div().size(px(72.)).flex_none().flex().items_center().justify_center().rounded(px(16.)).bg(b.page).child(illustration(art(&p)).w(px(64.)).h(px(48.))))
                    .child(div().flex_1().min_w_0().flex().flex_col().gap(px(4.))
                        .child(text(p.name.clone(), 24., BOLD, b.ink))
                        .child(text(format!("{} · {} · {}{}", kind_label(&p), p.author, origin, if p.version.is_empty() { String::new() } else { format!(" · {}", p.version) }), 13., FontWeight::NORMAL, b.gray))
                        .when(!p.tags.is_empty(), |d| d.child(div().pt(px(4.)).flex().flex_wrap().gap(px(6.)).children(p.tags.iter().filter(|t| *t != "rss").map(|t| tag(t, &b))))))
                    .child(div().flex().items_center().gap(px(8.)).child(text(if on { "On" } else { "Off" }, 13., BOLD, if on { b.ink } else { b.gray })).child(toggle)))
                .child(text(p.description.clone(), 15., LIGHT, b.ink))
                .when(!inside.is_empty(), |d| d.child(card(&b).bg(b.page).gap(px(4.))
                    .child(text(if !p.presets.is_empty() { "Its styles" } else if !p.topics.is_empty() { "Its topics" } else { "What it does" }, 13., BOLD, b.gray).pb(px(4.)))
                    .children(inside)))
                .children(p.from.clone().map(|f| text(format!("From {f}"), 12., FontWeight::NORMAL, b.gray)))
                .child(div().pt(px(8.)).border_t_1().border_color(b.hair).flex().flex_wrap().items_center().gap(px(8.)).children(actions)));
        Some(div().absolute().inset_0().flex().items_center().justify_center().child(backdrop).child(body).into_any_element())
    }

    /// The packs, or with `automations` the flows and the packs that run something.
    pub fn marketplace(&self, automations: bool, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let q = self.market_search.read(cx).value().to_string();
        let mine: Vec<Pack> = self.market.packs.as_ref().and_then(|p| p.as_ref().ok())
            .map(|l| l.iter().filter(|p| (runs_something(&p.kinds) || p.is("feed")) == automations).cloned().collect()).unwrap_or_default();
        let mut shown: Vec<&Pack> = mine.iter()
            .filter(|p| self.market.origin.is_none_or(|o| p.origin == o))
            .filter(|p| self.market.tag.as_ref().is_none_or(|t| p.tags.contains(t)))
            .filter(|p| found(p, &q)).collect();
        if self.market.newest {
            shown.sort_by_key(|p| std::cmp::Reverse(p.added));
        }
        let here: AnyElement = match &self.market.packs {
            None => text("Reading the packs...", 14., FontWeight::NORMAL, b.gray).into_any_element(),
            Some(Err(e)) => text(e.clone(), 14., FontWeight::NORMAL, b.red_text).into_any_element(),
            Some(Ok(_)) if shown.is_empty() => div().py(px(24.)).flex().flex_col().items_center().gap(px(8.))
                .child(illustration("search").w(px(160.)).h(px(120.)))
                .child(text("Nothing like that", 17., BOLD, b.ink))
                .child(text("Another word, another tag, or All.", 14., LIGHT, b.gray)).into_any_element(),
            Some(Ok(_)) => div().flex().flex_wrap().gap(px(16.)).children(shown.iter().enumerate().map(|(i, p)| self.pack_card(i, p, &b, cx))).into_any_element(),
        };
        let community: AnyElement = match &self.market.listed {
            None => text("Reading the list...", 14., FontWeight::NORMAL, b.gray).into_any_element(),
            Some(Err(e)) => text(format!("The list could not be read: {e}"), 14., FontWeight::NORMAL, b.gray).into_any_element(),
            Some(Ok(l)) => {
                let l: Vec<&Listed> = l.iter().filter(|x| runs_something(&x.kinds) == automations)
                    .filter(|x| q.to_lowercase().split_whitespace().all(|w| format!("{} {}", x.name, x.description).to_lowercase().contains(w))).collect();
                if l.is_empty() {
                    div().flex().items_center().gap(px(16.)).child(illustration("community").w(px(80.)).h(px(60.)))
                        .child(text("Nothing from the community here yet. Yours could be the first: make a pack, then Send to the marketplace.", 14., LIGHT, b.gray)).into_any_element()
                } else {
                    card(&b).children(l.into_iter().enumerate().map(|(i, x)| self.listed_row(i, x, &b, cx))).into_any_element()
                }
            }
        };
        let (i, title) = if automations { (Icon::Workflow, "Automations") } else { (Icon::Store, "Marketplace") };
        div().size_full().flex().flex_col()
            .child(div().h(HEADER).flex_none().px(px(24.)).flex().items_center().gap(px(8.)).border_b_1().border_color(b.hair)
                .child(icon(i, b.gray).size(px(16.)))
                .child(text(title, 14., BOLD, b.ink))
                .child(text(format!("{} packs", mine.len()), 13., FontWeight::NORMAL, b.gray).flex_1()))
            .child(div().id("market").flex_1().min_h_0().overflow_y_scroll()
                .child(div().w_full().max_w(px(960.)).mx_auto().p(px(24.)).flex().flex_col().gap(px(24.))
                    .child(self.hero(automations, &b, cx))
                    .when(automations, |d| d.child(self.flows_section(&b, cx)))
                    .child(div().flex().items_center().child(text(if automations { "Packs that run" } else { "Here" }, 19., BOLD, b.ink).flex_1()).child(self.import_row(&b, cx)))
                    .child(self.filters(&mine, &b, cx))
                    .child(here)
                    .child(text("From the community", 19., BOLD, b.ink))
                    .child(community)))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_gif_stands_still_on_its_card() {
        let gif = crate::packs::built_in_gif("edits.stardust").expect("stardust has a moving cover");
        let png = super::first_frame(gif).expect("its first frame");
        assert!(png.starts_with(b"\x89PNG"));
    }

    #[test]
    fn search_reaches_into_styles_and_topics() {
        let packs: Vec<super::Pack> = serde_json::from_value(crate::packs_json()).unwrap();
        let by = |id: &str| packs.iter().find(|p| p.id == id).unwrap();
        assert!(super::found(by("edits.spin"), "quarter"), "a family is found by one of its presets");
        assert!(super::found(by("edits.news"), "football"), "the news by a topic");
        assert!(!super::found(by("edits.spin"), "football"));
        assert_eq!(super::art(by("edits.news")), "topics");
        assert_eq!(super::art(by("edits.stardust")), "scene");
    }
}
