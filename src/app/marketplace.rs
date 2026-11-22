//! The marketplace: packs on cards with their covers (a GIF moves), and under it the automations and sources (packs
//! that run something), each from here and from the community's list (packs.json). Every pack is built in, from the
//! community, or made here (yours): any can be turned off; the community's and yours can be deleted; yours can be
//! shared, sent to the marketplace as a GitHub issue. Filter by where they came from and by tag, newest first.
use crate::app::icons::{self, icon, Icon};
use crate::app::runs::{Listed, Pack};
use crate::app::theme::{brand, Brand, BOLD, HEADER, LIGHT, SMALL};
use crate::app::ui::{card, pill, primary, text};
use crate::app::workspace::Workspace;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// Packs that run something: an automation, or a source's program.
fn runs_something(kinds: &[String]) -> bool {
    kinds.iter().any(|k| k == "automation" || k == "source")
}

fn picture(p: &Pack) -> Option<Img> {
    icons::picture(&p.id, || {
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
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join(".config/edits/packs").join(id)
}

fn kind_label(p: &Pack) -> &'static str {
    if p.is("automation") { "Automation" } else if p.is("source") { "Source" } else if p.is("feed") { "RSS" } else { "Pack" }
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
        let busy = self.market.busy.as_deref() == Some(p.id.as_str());
        let mut actions: Vec<AnyElement> = vec![];
        if busy {
            actions.push(text("Working...", 13., FontWeight::NORMAL, b.gray).into_any_element());
        } else {
            if p.enabled && !p.trusted {
                let read = folder(&p.id);
                actions.push(pill(("read", i), Some(Icon::Eye), "Read it", false, b).on_click(move |_, _, cx| cx.open_with_system(&read)).into_any_element());
                actions.push(primary(("trust", i), "Trust", b).on_click(act("trust", p.id.clone(), cx.entity().downgrade())).into_any_element());
            }
            actions.push(pill(("onoff", i), None, if p.enabled { "On" } else { "Off" }, p.enabled, b).on_click(act(if p.enabled { "off" } else { "on" }, p.id.clone(), cx.entity().downgrade())).into_any_element());
            if p.origin == "local" {
                actions.push(pill(("share", i), Some(Icon::Share), "Share", false, b).on_click(act("share", p.id.clone(), cx.entity().downgrade())).into_any_element());
            }
            if p.origin != "built-in" {
                actions.push(pill(("delete", i), Some(Icon::Trash), "Delete", false, b).on_click(act("remove", p.id.clone(), cx.entity().downgrade())).into_any_element());
            }
        }
        let origin = match p.origin.as_str() { "built-in" => "built in", "local" => "yours", o => o };
        card(b).w(px(288.)).gap(px(8.)).when(!p.enabled, |d| d.opacity(0.5))
            .child(div().w_full().h(px(144.)).rounded(SMALL).overflow_hidden().bg(b.hair)
                .children(picture(p).map(|img| img.size_full().object_fit(ObjectFit::Cover))))
            .child(text(p.name.clone(), 15., BOLD, b.ink).truncate())
            .child(text(format!("{} · {} · {}", kind_label(p), p.author, origin), 12., FontWeight::NORMAL, b.gray).truncate())
            .child(text(p.description.clone(), 13., LIGHT, b.gray).h(px(36.)).line_clamp(2))
            .when(!p.tags.is_empty(), |d| d.child(text(p.tags.iter().take(4).map(|t| format!("#{t}")).collect::<Vec<_>>().join("  "), 12., FontWeight::NORMAL, b.gray).truncate()))
            .child(div().pt(px(8.)).flex().flex_wrap().gap(px(8.)).children(actions))
    }

    fn listed_row(&self, i: usize, l: &Listed, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let id = l.id.clone();
        let busy = self.market.busy.as_deref() == Some(l.id.as_str());
        div().py(px(8.)).flex().items_center().gap(px(16.)).when(i > 0, |d| d.border_t_1().border_color(b.hair))
            .child(div().flex_1().min_w_0().flex().flex_col().gap(px(4.))
                .child(text(l.name.clone(), 14., BOLD, b.ink).truncate())
                .child(text(l.description.clone(), 13., LIGHT, b.gray).truncate()))
            .child(if l.added || busy {
                text(if busy { "Working..." } else { "Added" }, 13., FontWeight::NORMAL, b.gray).into_any_element()
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
        for t in list.iter().flat_map(|p| &p.tags) {
            match count.iter_mut().find(|x| &x.0 == t) {
                Some(x) => x.1 += 1,
                None => count.push((t.clone(), 1)),
            }
        }
        count.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        let tags = count.into_iter().take(14).enumerate().map(|(i, (t, _))| {
            let on = self.market.tag.as_deref() == Some(t.as_str());
            pill(("tag", i), None, format!("#{t}"), on, b).on_click(cx.listener(move |this, _, _, cx| {
                this.market.tag = if on { None } else { Some(t.clone()) };
                cx.notify();
            }))
        }).collect::<Vec<_>>();
        div().flex().flex_col().gap(px(8.))
            .child(div().flex().flex_wrap().gap(px(8.)).children(origins).child(div().w(px(1.)).h(px(16.)).bg(b.hair)).child(newest))
            .child(div().flex().flex_wrap().gap(px(8.)).children(tags))
    }

    /// The packs, or with `automations` the automations and sources under them.
    pub fn marketplace(&self, automations: bool, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let mine: Vec<Pack> = self.market.packs.as_ref().and_then(|p| p.as_ref().ok()).map(|l| l.iter().filter(|p| runs_something(&p.kinds) == automations).cloned().collect()).unwrap_or_default();
        let mut shown: Vec<&Pack> = mine.iter()
            .filter(|p| self.market.origin.is_none_or(|o| p.origin == o))
            .filter(|p| self.market.tag.as_ref().is_none_or(|t| p.tags.contains(t))).collect();
        if self.market.newest {
            shown.sort_by_key(|p| std::cmp::Reverse(p.added));
        }
        let here: AnyElement = match &self.market.packs {
            None => text("Reading the packs...", 14., FontWeight::NORMAL, b.gray).into_any_element(),
            Some(Err(e)) => text(e.clone(), 14., FontWeight::NORMAL, b.red_text).into_any_element(),
            Some(Ok(_)) if shown.is_empty() => text("Nothing here with those filters.", 14., LIGHT, b.gray).into_any_element(),
            Some(Ok(_)) => div().flex().flex_wrap().gap(px(16.)).children(shown.iter().enumerate().map(|(i, p)| self.pack_card(i, p, &b, cx))).into_any_element(),
        };
        let community: AnyElement = match &self.market.listed {
            None => text("Reading the list...", 14., FontWeight::NORMAL, b.gray).into_any_element(),
            Some(Err(e)) => text(format!("The list could not be read: {e}"), 14., FontWeight::NORMAL, b.gray).into_any_element(),
            Some(Ok(l)) => {
                let l: Vec<&Listed> = l.iter().filter(|x| runs_something(&x.kinds) == automations).collect();
                if l.is_empty() {
                    text("Nothing from the community yet. Yours could be the first: Share on a pack you made.", 14., LIGHT, b.gray).into_any_element()
                } else {
                    card(&b).children(l.into_iter().enumerate().map(|(i, x)| self.listed_row(i, x, &b, cx))).into_any_element()
                }
            }
        };
        let (i, title, about) = if automations {
            (Icon::Workflow, "Automations", "Packs that run something: an automation on its one site (posting a clip), or a source's own program finding what to make. Each runs alone; one you add runs after you read it and trust it.")
        } else {
            (Icon::Store, "Marketplace", "Packs: an edit's shape, RSS to make videos from, the instructions an agent follows. Built in, from the community, or yours: turn any off, delete the ones you added, share the ones you made.")
        };
        div().size_full().flex().flex_col()
            .child(div().h(HEADER).flex_none().px(px(24.)).flex().items_center().gap(px(8.)).border_b_1().border_color(b.hair)
                .child(icon(i, b.gray).size(px(16.)))
                .child(text(title, 14., BOLD, b.ink).flex_1()))
            .child(div().id("market").flex_1().min_h_0().overflow_y_scroll()
                .child(div().w_full().max_w(px(960.)).mx_auto().p(px(24.)).flex().flex_col().gap(px(24.))
                    .child(text(about, 15., LIGHT, b.gray))
                    .when(automations, |d| d.child(self.flows_section(&b, cx)))
                    .child(text("From the community", 19., BOLD, b.ink))
                    .child(community)
                    .child(text("Here", 19., BOLD, b.ink))
                    .child(self.filters(&mine, &b, cx))
                    .child(here)))
    }
}
