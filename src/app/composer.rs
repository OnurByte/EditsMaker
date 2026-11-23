//! t3code's ChatComposer: one card under the timeline. On top, what is attached (files dropped or picked, each made
//! on its own) and what was picked with "/" (a style, a 3D scene, accounts to post to), each a chip that comes off;
//! then the prompt, growing with what is typed (Enter sends, Shift+Enter is a new line). Typing "/" opens the packs
//! and automations to pick from, and under them the library: a clip made before, to edit again. Under it, t3code's footer: a file on the left, then the model picker (the AI, by
//! provider) and the access mode (supervised or full access), and send on the right.
use crate::app::icons::{icon, logo, Icon};
use crate::app::runs::{Pack, Provider};
use crate::app::theme::{self, brand, Brand, BOLD, CARD, LIGHT, SMALL};
use crate::app::ui::{icon_button, pill, text};
use crate::app::workspace::{Picker, Workspace};
use gpui_kit::component::input::Textarea;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// What "/" offers: a style, a 3D scene, a topic, or an account to post to.
#[derive(Clone, PartialEq, Debug)]
pub enum Slash {
    Style(String),
    Scene(String, String),
    Topic(String, String),
    Post(String),
    /// a clip made before, from the library, attached to edit again
    Clip(std::path::PathBuf),
}

/// Every choice "/" can make, (section, title, line, choice), those whose words hold what follows the "/".
pub fn slash_items(q: &str, packs: &[Pack], accounts: &[String], clips: &[(std::path::PathBuf, String)]) -> Vec<(&'static str, String, String, Slash)> {
    let mut v: Vec<(&'static str, String, String, Slash)> = vec![];
    for p in packs.iter().filter(|p| p.enabled) {
        for n in &p.presets {
            v.push(("Styles", n.name.clone(), format!("{} · {}", p.name, n.about), Slash::Style(n.name.clone())));
        }
        if p.presets.is_empty() {
            if let Some(n) = &p.preset {
                v.push(("Styles", n.clone(), p.description.clone(), Slash::Style(n.clone())));
            }
        }
        if p.scene {
            v.push(("3D scenes", p.name.clone(), p.description.clone(), Slash::Scene(p.short(), p.name.clone())));
        }
        for t in &p.topics {
            v.push(("Topics", t.name.clone(), format!("A video from today's {} news", t.name.to_lowercase()), Slash::Topic(t.key.clone(), t.name.clone())));
        }
    }
    for a in accounts {
        v.push(("Automations", format!("Post to {a}"), "Once the edit is ready, it goes up on this account".into(), Slash::Post(a.clone())));
    }
    // what was made, under the packs and automations: picked, it is attached to be edited again
    for (c, about) in clips {
        v.push(("Library", c.file_stem().unwrap_or_default().to_string_lossy().into_owned(), about.clone(), Slash::Clip(c.clone())));
    }
    let q = q.trim().to_lowercase();
    v.retain(|(s, t, l, _)| q.split_whitespace().all(|w| format!("{s} {t} {l}").to_lowercase().contains(w)));
    v
}

/// The word after a leading "/", while the prompt is one: "/spi" is Some("spi").
pub fn slash_query(text: &str) -> Option<&str> {
    let t = text.strip_prefix('/')?;
    (!t.contains('\n')).then_some(t)
}

impl Workspace {
    /// The flow's topic list (the automations page's picker).
    pub fn picker_list(&self, which: Picker, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let packs = self.market.packs.as_ref().and_then(|p| p.as_ref().ok()).cloned().unwrap_or_default();
        let rows: Vec<(String, String, Option<String>)> = packs.iter().filter(|p| p.enabled).flat_map(|p| {
            let topics = p.topics.iter().map(|t| (t.name.clone(), format!("{} feeds", t.feeds.len()), Some(t.key.clone()))).collect::<Vec<_>>();
            if topics.is_empty() && p.is("feed") { vec![(p.name.clone(), p.description.clone(), Some(p.short()))] } else { topics }
        }).collect();
        let list = rows.into_iter().enumerate().map(|(i, (name, about, value))| {
            let hover = b.hair;
            div().id(("choice", i)).px(px(8.)).py(px(8.)).rounded(SMALL).cursor_pointer().hover(move |s| s.bg(hover)).flex().items_center().gap(px(12.))
                .child(text(name, 14., FontWeight::NORMAL, b.ink).flex_1().truncate())
                .child(text(about, 12., FontWeight::NORMAL, b.gray))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if which == Picker::FlowSource {
                        this.flow_source = value.clone();
                    }
                    this.picker = None;
                    cx.notify();
                }))
        });
        div().id("picker").max_h(px(240.)).overflow_y_scroll().flex().flex_col().gap(px(2.)).children(list)
    }

    /// A pill that opens a picker, saying what is chosen.
    pub fn picker_pill(&self, id: &'static str, i: Icon, label: String, which: Picker, b: &Brand, cx: &mut Context<Self>) -> Stateful<Div> {
        pill(id, Some(i), label, self.picker == Some(which), b).on_click(cx.listener(move |this, _, _, cx| {
            this.toggle_picker(which, cx);
        }))
    }

    pub fn toggle_picker(&mut self, which: Picker, cx: &mut Context<Self>) {
        self.picker = if self.picker == Some(which) { None } else { Some(which) };
        if self.picker == Some(Picker::Model) {
            if self.settings.is_none() {
                self.load_settings(cx);
            }
            // the models of the keyed and local providers, read once
            let want: Vec<(String, String)> = self.settings.as_ref().and_then(|s| s.as_ref().ok()).map(|s| s.providers.iter()
                .filter(|p| p.ready && (p.kind == "api" || p.kind == "local") && !self.model_lists.contains_key(&p.name))
                .map(|p| (p.name.clone(), if p.kind == "local" { p.base.clone() } else { String::new() })).collect()).unwrap_or_default();
            for (name, base) in want {
                cx.spawn(async move |this, cx| {
                    let n = name.clone();
                    let m = cx.background_executor().spawn(async move { crate::ai::models_of(&n, &base) }).await;
                    let _ = this.update(cx, |this, cx| {
                        this.model_lists.insert(name, m);
                        cx.notify();
                    });
                })
                .detach();
            }
        }
        cx.notify();
    }

    /// `edits ai PROVIDER [MODEL]`, as a model is picked.
    fn pick_model(&mut self, p: &Provider, model: &str, window: &mut Window, cx: &mut Context<Self>) {
        let mut cmd = vec!["ai".to_string(), p.name.clone()];
        if !model.is_empty() {
            cmd.push(model.to_string());
        }
        if p.kind == "local" || p.kind == "acp" {
            cmd.extend(["--endpoint".to_string(), if p.kind == "acp" { p.command.clone() } else { p.base.clone() }]);
        }
        self.picker = None;
        self.change(cmd, window, cx);
    }

    /// t3code's ModelPicker: each provider here with its logo, its plan, and its models; the one in use ticked.
    fn model_menu(&self, b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let Some(Ok(s)) = &self.settings else {
            return text("Reading the AI here...", 13., FontWeight::NORMAL, b.gray).p(px(8.)).into_any_element();
        };
        let here: Vec<Provider> = s.providers.iter().filter(|p| p.ready || p.in_use).cloned().collect();
        let mut rows: Vec<AnyElement> = vec![];
        for (gi, p) in here.iter().enumerate() {
            let models: Vec<String> = match p.kind.as_str() {
                "api" | "local" => match self.model_lists.get(&p.name) {
                    Some(Ok(l)) => l.iter().take(30).cloned().collect(),
                    _ => vec![],
                },
                _ => std::iter::once(String::new()).chain(p.models.iter().cloned()).collect(),
            };
            let loading = matches!(p.kind.as_str(), "api" | "local") && !self.model_lists.contains_key(&p.name);
            rows.push(div().px(px(8.)).pt(px(if gi == 0 { 4. } else { 12. })).pb(px(4.)).flex().items_center().gap(px(8.))
                .children(logo(&p.name, b.gray, b.card).map(|l| l.size(px(14.))))
                .child(text(p.label.clone(), 12., BOLD, b.gray))
                .when(!p.plan.is_empty(), |d| d.child(text(p.plan.clone(), 12., FontWeight::NORMAL, b.gray)))
                .when(loading, |d| d.child(text("reading its models...", 12., LIGHT, b.gray)))
                .into_any_element());
            for (mi, m) in models.into_iter().enumerate() {
                let on = p.in_use && m == s.ai.model;
                let (hover, prov, model) = (b.hair, p.clone(), m.clone());
                rows.push(div().id(("model", gi * 1000 + mi)).h(px(32.)).px(px(8.)).flex().items_center().gap(px(8.)).rounded(SMALL).cursor_pointer().hover(move |st| st.bg(hover))
                    .child(div().size(px(16.)).flex_none().when(on, |d| d.child(icon(Icon::Check, b.accent).size(px(16.)))))
                    .child(text(if m.is_empty() { "Default".to_string() } else { m.clone() }, 14., FontWeight::NORMAL, b.ink).flex_1().truncate())
                    .when(m.is_empty(), |d| d.child(text("its own choice", 12., LIGHT, b.gray)))
                    .on_click(cx.listener(move |this, _, window, cx| this.pick_model(&prov, &model, window, cx)))
                    .into_any_element());
            }
        }
        let hover = b.hair;
        rows.push(div().id("model-add").mt(px(4.)).h(px(32.)).px(px(8.)).flex().items_center().gap(px(8.)).rounded(SMALL).cursor_pointer().border_t_1().border_color(b.hair).hover(move |st| st.bg(hover))
            .child(icon(Icon::Plus, b.gray).size(px(16.)))
            .child(text("Add provider", 14., FontWeight::NORMAL, b.gray))
            .on_click(cx.listener(|this, _, _, cx| {
                this.picker = None;
                this.open_settings(crate::app::workspace::Section::Ai, cx);
                this.ai_adding = true;
                cx.notify();
            }))
            .into_any_element());
        div().id("model-menu").max_h(px(320.)).overflow_y_scroll().flex().flex_col().gap(px(2.)).children(rows).into_any_element()
    }

    /// The model picker's pill: the provider's logo and the model, as t3code shows it.
    fn model_pill(&self, b: &Brand, cx: &mut Context<Self>) -> Stateful<Div> {
        let s = self.settings.as_ref().and_then(|s| s.as_ref().ok());
        let p = s.and_then(|s| s.providers.iter().find(|p| p.in_use));
        let label = match (p, s) {
            (Some(_), Some(s)) if !s.ai.model.is_empty() => s.ai.model.clone(),
            (Some(p), _) => p.label.clone(),
            _ => "Pick a model".into(),
        };
        let on = self.picker == Some(Picker::Model);
        let hover = b.hair;
        div().id("model").h(px(32.)).px(px(10.)).flex().items_center().gap(px(6.)).rounded(px(16.)).cursor_pointer()
            .when(on, |d| d.bg(b.accent.opacity(0.16))).hover(move |st| st.bg(hover))
            .children(p.and_then(|p| logo(&p.name, b.ink, b.card)).map(|l| l.size(px(14.))))
            .when(p.is_none(), |d| d.child(icon(Icon::Brain, b.gray).size(px(14.))))
            .child(text(label, 13., FontWeight::NORMAL, b.ink).max_w(px(180.)).truncate())
            .child(icon(Icon::ChevronDown, b.gray).size(px(12.)))
            .on_click(cx.listener(|this, _, _, cx| this.toggle_picker(Picker::Model, cx)))
    }

    /// t3code's runtime mode: supervised (the agent only answers) or full access (its own prompts skipped).
    fn access_pill(&self, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let full = self.settings.as_ref().and_then(|s| s.as_ref().ok()).is_some_and(|s| s.access == "full");
        pill("access", Some(if full { Icon::Unlock } else { Icon::Lock }), if full { "Full access" } else { "Supervised" }, false, b)
            .tooltip(move |w, cx| gpui_kit::component::tooltip::Tooltip::new(if full { "The agent runs with its own permission prompts skipped" } else { "The agent only answers; anything it asks to do is refused" }).build(w, cx))
            .on_click(cx.listener(move |this, _, window, cx| this.change(vec!["ai".into(), "access".into(), if full { "supervised" } else { "full" }.into()], window, cx)))
    }

    fn take_slash(&mut self, s: Slash, window: &mut Window, cx: &mut Context<Self>) {
        let mut text = String::new();
        match s {
            Slash::Style(n) => self.preset = Some(n),
            Slash::Scene(id, name) => self.overlay = Some((id, name)),
            Slash::Topic(key, _) => text = format!("rss {key} -n 1"),
            Slash::Post(a) => {
                if !self.post.contains(&a) {
                    self.post.push(a);
                }
            }
            Slash::Clip(c) => {
                if !self.files.contains(&c) {
                    self.files.push(c);
                }
            }
        }
        self.input.update(cx, |st, cx| st.set_value(text, window, cx));
        self.input.update(cx, |st, cx| st.focus(window, cx));
        cx.notify();
    }

    /// The "/" menu: what can be picked, by section.
    fn slash_menu(&self, q: &str, b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let packs = self.market.packs.as_ref().and_then(|p| p.as_ref().ok()).cloned().unwrap_or_default();
        let accounts: Vec<String> = self.settings.as_ref().and_then(|s| s.as_ref().ok()).map(|s| s.accounts.iter().map(|a| a.name.clone()).collect()).unwrap_or_default();
        let clips: Vec<(std::path::PathBuf, String)> = self.past.iter().take(150)
            .flat_map(|r| crate::app::runs::clips(&r.dir).into_iter().map(move |c| (c, format!("{} · made from {}", r.when, r.subject)))).collect();
        let items = slash_items(q, &packs, &accounts, &clips);
        if items.is_empty() {
            return text("Nothing like that among the packs and automations", 13., LIGHT, b.gray).p(px(8.)).into_any_element();
        }
        let mut rows: Vec<AnyElement> = vec![];
        let mut last = "";
        for (i, (section, title, line, choice)) in items.into_iter().take(60).enumerate() {
            if section != last {
                rows.push(text(section, 12., BOLD, b.gray).px(px(8.)).pt(px(if last.is_empty() { 0. } else { 8. })).into_any_element());
                last = section;
            }
            let i_ = match &choice { Slash::Style(_) => Icon::Sliders, Slash::Scene(..) => Icon::Cube, Slash::Topic(..) => Icon::Rss, Slash::Post(_) => Icon::Send, Slash::Clip(_) => Icon::Library };
            let hover = b.hair;
            rows.push(div().id(("slash", i)).h(px(36.)).px(px(8.)).flex().items_center().gap(px(10.)).rounded(SMALL).cursor_pointer().hover(move |st| st.bg(hover))
                .child(icon(i_, b.gray).size(px(16.)))
                .child(text(title, 14., FontWeight::NORMAL, b.ink).flex_none().max_w(px(220.)).truncate())
                .child(text(line, 12., LIGHT, b.gray).flex_1().min_w_0().truncate())
                .on_click(cx.listener(move |this, _, window, cx| this.take_slash(choice.clone(), window, cx)))
                .into_any_element());
        }
        div().id("slash-menu").max_h(px(280.)).overflow_y_scroll().flex().flex_col().gap(px(2.)).children(rows).into_any_element()
    }

    pub fn composer(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let chip = |id: ElementId, i: Icon, label: String, b: &Brand| {
            div().id(id).h(px(32.)).pl(px(12.)).pr(px(4.)).flex().items_center().gap(px(8.)).rounded(px(16.)).bg(b.hair)
                .child(icon(i, b.ink).size(px(16.)))
                .child(text(label, 13., FontWeight::NORMAL, b.ink).max_w(px(240.)).truncate())
        };
        let x = |id: ElementId, b: &Brand| div().id(id).size(px(24.)).flex().items_center().justify_center().rounded_full().cursor_pointer().child(text("×", 14., FontWeight::NORMAL, b.gray));
        let mut chips: Vec<AnyElement> = self.files.iter().enumerate().map(|(i, f)| {
            chip(("file", i).into(), Icon::Film, f.file_name().unwrap_or_default().to_string_lossy().into_owned(), &b)
                .child(x(("unattach", i).into(), &b).on_click(cx.listener(move |this, _, _, cx| {
                    this.files.remove(i);
                    cx.notify();
                }))).into_any_element()
        }).collect();
        if let Some(p) = &self.preset {
            chips.push(chip("style-chip".into(), Icon::Sliders, p.clone(), &b).child(x("unstyle".into(), &b).on_click(cx.listener(|this, _, _, cx| {
                this.preset = None;
                cx.notify();
            }))).into_any_element());
        }
        if let Some((_, name)) = &self.overlay {
            chips.push(chip("scene-chip".into(), Icon::Cube, name.clone(), &b).child(x("unscene".into(), &b).on_click(cx.listener(|this, _, _, cx| {
                this.overlay = None;
                cx.notify();
            }))).into_any_element());
        }
        for (i, a) in self.post.iter().enumerate() {
            chips.push(chip(("post-chip", i).into(), Icon::Send, format!("Then post to {a}"), &b).child(x(("unpost", i).into(), &b).on_click(cx.listener(move |this, _, _, cx| {
                this.post.remove(i);
                cx.notify();
            }))).into_any_element());
        }
        let typed = self.input.read(cx).value().to_string();
        let menu = match (slash_query(&typed), self.picker) {
            (Some(q), _) => Some(self.slash_menu(q, &b, cx)),
            (None, Some(Picker::Model)) => Some(self.model_menu(&b, cx)),
            _ => None,
        };
        div().w_full().max_w(theme::column()).mx_auto().flex().flex_col().gap(px(8.)).bg(b.card).rounded(CARD).p(px(16.))
            .when(!chips.is_empty(), |d| d.child(div().flex().flex_wrap().gap(px(8.)).children(chips)))
            .children(menu.map(|m| div().pb(px(8.)).border_b_1().border_color(b.hair).child(m)))
            .child(Textarea::new(&self.input).appearance(false))
            .child(div().flex().items_center().gap(px(4.))
                .child(icon_button("pick", Icon::Attach, &b).on_click(cx.listener(|this, _, window, cx| this.pick(window, cx))))
                .child(self.model_pill(&b, cx))
                .child(self.access_pill(&b, cx))
                .child(div().flex_1())
                .child(text("/ for packs", 12., LIGHT, b.gray).mr(px(8.)))
                .child(div().id("send").size(px(32.)).flex().items_center().justify_center().rounded_full().cursor_pointer().bg(b.accent)
                    .child(icon(Icon::Send, white()).size(px(16.)))
                    .on_click(cx.listener(|this, _, window, cx| this.start(window, cx)))))
    }
}

#[cfg(test)]
mod tests {
    // not super::*: gpui_kit's own `test` attribute would come with it
    use super::{slash_items, slash_query, Pack, Slash};

    #[test]
    fn slash_finds_packs_and_automations_by_any_word() {
        assert_eq!(slash_query("/spin"), Some("spin"));
        assert_eq!(slash_query("spin"), None);
        let packs: Vec<Pack> = serde_json::from_value(crate::packs_json()).unwrap();
        let clip = (std::path::PathBuf::from("/v/sad-01.mp4"), "Made from Madara".to_string());
        let all = slash_items("", &packs, &["tiktok-me".into()], std::slice::from_ref(&clip));
        assert_eq!(all.last().unwrap().0, "Library", "the library comes under the packs and automations");
        assert_eq!(slash_items("sad", &packs, &[], std::slice::from_ref(&clip)).last().unwrap().3, Slash::Clip(clip.0.clone()));
        assert!(all.iter().any(|x| x.3 == Slash::Style("akis-whip".into())), "a family's preset is offered by its own name");
        assert!(all.iter().any(|x| matches!(&x.3, Slash::Topic(k, _) if k == "football")));
        assert!(all.iter().any(|x| matches!(&x.3, Slash::Scene(k, _) if k == "stardust")));
        assert!(slash_items("post tiktok", &packs, &["tiktok-me".into()], &[]).iter().all(|x| x.3 == Slash::Post("tiktok-me".into())));
    }
}
