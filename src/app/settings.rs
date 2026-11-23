//! t3code's settings, ported: a window over the app (the backdrop closes it), its sections on the left
//! (SettingsSidebarNav), the section's groups on the right (SettingsGroup: a card whose rows are split by hairlines,
//! SettingsRow: what it is and why on the left, its control on the right). AI is t3code's providers: the one in use,
//! and every other to add, each set up in place (a key, a server, a model). Appearance is t3code's, with Omarchy's
//! own theme followed live. Each change runs the edits code that owns it, so the CLI and the app agree.
use crate::app::icons::{icon, Icon};
use crate::app::runs::{Provider, Settings};
use crate::app::theme::{self, brand, Brand, Prefs, BOLD, CARD, LIGHT, MONO, SMALL};
use crate::app::ui::{card, pill, primary, text};
use gpui_kit::component::switch::Switch;
use crate::app::workspace::{Section, Workspace};
use gpui_kit::component::input::Input;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::WindowExt;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// The sections, their icons, and the words search finds them by (what their rows are about).
const SECTIONS: &[(Section, &str, Icon, &str)] = &[
    (Section::General, "General", Icon::Settings, "folder where edits go output videos encoder gpu graphics card render discord rich presence playing status"),
    (Section::Ai, "AI", Icon::Brain, "ai provider model claude codex cursor grok gemini antigravity kiro opencode pi acp openai anthropic openrouter ollama key api"),
    (Section::Accounts, "Accounts", Icon::User, "accounts connect sign in tiktok youtube instagram bluesky facebook post"),
    (Section::Agents, "Agents", Icon::Terminal, "agents mcp server skills claude code chatgpt cursor tools"),
    (Section::Usage, "Usage", Icon::Usage, "usage how many edits clips space disk ai calls requests time spent stats"),
    (Section::Appearance, "Appearance", Icon::Palette, "appearance theme dark light system omarchy accent colour color chat width text size font"),
    (Section::About, "About", Icon::Info, "about version github x twitter author issues program updates update rollback latest release check doctor ffmpeg yt-dlp whisper terms of use privacy license legal"),
];

/// What About links to, each a file in the repository on GitHub: (title, line, file).
pub const LEGAL: &[(&str, &str, &str)] = &[
    ("Terms of use", "What you may do with EditsMaker, and what is yours to answer for.", "TERMS.md"),
    ("Privacy", "What stays on this computer (everything), and what goes out only when you ask.", "PRIVACY.md"),
    ("License", "MIT: use it, change it, share it.", "LICENSE"),
    ("Contributing", "Packs, fixes and ideas: how to send them.", "CONTRIBUTING.md"),
];

/// The sections a search finds, all of them for none.
pub fn found(q: &str) -> Vec<Section> {
    let q = q.trim().to_lowercase();
    SECTIONS.iter().filter(|s| q.is_empty() || s.1.to_lowercase().contains(&q) || q.split_whitespace().all(|w| s.3.contains(w))).map(|s| s.0).collect()
}

/// An account as t3code shows it before a click: an email's name hidden, o•••@gmail.com; a plain name as it is.
pub fn masked(who: &str) -> String {
    match who.split_once('@') {
        Some((n, d)) => format!("{}•••@{d}", n.chars().next().unwrap_or('•')),
        None => who.to_string(),
    }
}

/// A SettingsRow: title and description on the left, the control on the right.
fn row(title: &str, about: impl Into<SharedString>, control: impl IntoElement, first: bool, b: &Brand) -> impl IntoElement {
    div().px(px(16.)).py(px(16.)).flex().items_center().gap(px(16.)).when(!first, |d| d.border_t_1().border_color(b.hair))
        .child(div().flex_1().min_w_0().flex().flex_col().gap(px(4.))
            .child(text(title.to_string(), 14., FontWeight::NORMAL, b.ink))
            .child(text(about, 13., LIGHT, b.gray)))
        .child(div().flex_none().flex().flex_wrap().justify_end().max_w(px(360.)).gap(px(8.)).child(control))
}

/// A SettingsGroup: rows on one card.
fn group(title: &str, rows: Vec<AnyElement>, b: &Brand) -> impl IntoElement {
    div().flex().flex_col().gap(px(8.))
        .when(!title.is_empty(), |d| d.child(text(title.to_string(), 13., BOLD, b.gray)))
        .child(div().flex().flex_col().bg(b.page).rounded(CARD).children(rows))
}

/// t3code's ThemeWireframe: a little window in a theme's colours, its sidebar, lines of text and the composer.
fn wireframe(c: &Brand) -> Div {
    let line = |w: f32, o: f32| div().h(px(4.)).w(relative(w)).rounded(px(2.)).bg(c.ink.opacity(o));
    div().h(px(96.)).flex().rounded(SMALL).overflow_hidden().bg(c.page).border_1().border_color(c.hair)
        .child(div().w(relative(0.3)).h_full().p(px(8.)).flex().flex_col().gap(px(6.)).bg(c.card)
            .child(line(0.7, 0.5)).child(line(0.9, 0.25)).child(line(0.6, 0.25)).child(line(0.8, 0.25)))
        .child(div().flex_1().p(px(10.)).flex().flex_col().gap(px(6.))
            .child(line(0.5, 0.6)).child(line(0.85, 0.25)).child(line(0.7, 0.25))
            .child(div().flex_1())
            .child(div().h(px(18.)).rounded(px(9.)).bg(c.card).border_1().border_color(c.hair).flex().items_center().justify_end().px(px(3.))
                .child(div().size(px(12.)).rounded_full().bg(c.accent))))
}

/// One of t3code's appearance mode tiles: the preview, its name under it, ringed in the accent when picked.
fn mode_tile(id: &'static str, preview: Div, label: &'static str, on: bool, b: &Brand) -> Stateful<Div> {
    let hover = b.accent.opacity(0.06);
    div().id(id).flex_1().flex_basis(px(0.)).min_w_0().p(px(8.)).flex().flex_col().gap(px(6.)).rounded(px(12.)).cursor_pointer()
        .bg(if on { b.accent.opacity(0.14) } else { b.page }).border_1().border_color(if on { b.accent } else { b.hair })
        .when(!on, |d| d.hover(move |s| s.bg(hover)))
        .child(preview)
        .child(div().flex().justify_center().child(text(label, 12., if on { BOLD } else { FontWeight::NORMAL }, if on { b.ink } else { b.gray })))
}

/// t3code's select, as a segmented row: the current value filled; picking one saves `set(value)`.
fn segmented(id: &'static str, values: &[(&'static str, &'static str)], now: &str, b: &Brand, cx: &mut Context<Workspace>, set: fn(&str) -> Prefs) -> Div {
    div().flex().p(px(2.)).gap(px(2.)).rounded(px(8.)).bg(b.card).children(values.iter().enumerate().map(|(i, &(v, label))| {
        let on = v == now;
        let hover = b.hair;
        div().id((id, i)).h(px(26.)).px(px(10.)).flex().items_center().rounded(px(6.)).cursor_pointer()
            .when(on, |d| d.bg(b.page)).when(!on, |d| d.hover(move |s| s.bg(hover)))
            .child(text(label, 12., if on { BOLD } else { FontWeight::NORMAL }, if on { b.ink } else { b.gray }))
            .on_click(cx.listener(move |_, _, window, cx| theme::save(&set(v), window, cx)))
    }))
}

impl Workspace {
    /// Pills that set one value with `edits ARGS... VALUE`, the current one filled.
    fn choice(&self, id: &'static str, values: &[String], now: &str, args: &[&str], b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        div().flex().flex_wrap().justify_end().gap(px(8.)).children(values.iter().enumerate().map(|(i, v)| {
            let mut cmd: Vec<String> = args.iter().map(|a| a.to_string()).collect();
            cmd.push(v.clone());
            pill((id, i), None, v.clone(), v == now, b).on_click(cx.listener(move |this, _, window, cx| this.change(cmd.clone(), window, cx)))
        })).into_any_element()
    }

    fn button(&self, id: impl Into<ElementId>, label: &str, args: &[&str], b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let cmd: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        pill(id, None, label.to_string(), false, b).on_click(cx.listener(move |this, _, window, cx| this.change(cmd.clone(), window, cx))).into_any_element()
    }

    /// The models a provider offers, read in the background.
    fn load_models(&mut self, provider: String, endpoint: String, cx: &mut Context<Self>) {
        self.ai_models = None;
        cx.spawn(async move |this, cx| {
            let m = cx.background_executor().spawn(async move { crate::ai::models_of(&provider, &endpoint) }).await;
            let _ = this.update(cx, |this, cx| {
                this.ai_models = Some(m);
                cx.notify();
            });
        })
        .detach();
    }

    /// What a provider needs before edits can ask it, in place: a key, a server, a command, or (for an agent) nothing.
    fn provider_setup(&self, p: &Provider, s: &Settings, b: &Brand, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let name = p.name.clone();
        let mut rows: Vec<AnyElement> = vec![];
        let first = |rows: &Vec<AnyElement>| rows.is_empty();
        if p.kind == "subscription" || (p.kind == "acp" && !p.command.is_empty()) {
            rows.push(row("Runs", if p.how == "acp" { "Spoken to over ACP; it gets no files, terminal or permission." } else { "Asked once with the prompt, on your own account." },
                text(p.command.replace("{model}", ""), 12., FontWeight::NORMAL, b.gray).font_family(MONO).max_w(px(300.)).truncate(), first(&rows), b).into_any_element());
        }
        if p.kind == "api" {
            let n = name.clone();
            rows.push(row("API key", format!("Kept where only you can read it; {} works too.", p.key_var),
                div().flex().items_center().gap(px(8.)).child(div().w(px(200.)).child(Input::new(&self.ai_key)))
                    .child(pill("save-key", Some(Icon::Key), "Save", false, b).on_click(cx.listener(move |this, _, window, cx| {
                        let k = this.ai_key.read(cx).value().to_string();
                        match crate::ai::save_key(&n, &k) {
                            Ok(()) => {
                                this.ai_key.update(cx, |s, cx| s.set_value("", window, cx));
                                window.push_notification(Notification::success(format!("{n} key saved")), cx);
                                this.load_settings(cx);
                                this.load_models(n.clone(), String::new(), cx);
                            }
                            Err(e) => window.push_notification(Notification::error(e), cx),
                        }
                    }))), first(&rows), b).into_any_element());
        }
        if p.kind == "local" || p.kind == "acp" {
            let n = name.clone();
            let acp = p.kind == "acp";
            rows.push(row(if acp { "Local ACP command" } else { "Server" }, if acp { "How the agent starts in ACP mode, as the ACP Registry lists it." } else { "Where it answers on this computer." },
                div().flex().items_center().gap(px(8.)).child(div().w(px(220.)).child(Input::new(&self.ai_endpoint)))
                    .child(primary("set-endpoint", if acp { "Add" } else { "Use" }, b).on_click(cx.listener(move |this, _, window, cx| {
                        let e = this.ai_endpoint.read(cx).value().to_string();
                        this.change(vec!["ai".into(), n.clone(), "--endpoint".into(), e.clone()], window, cx);
                        if !acp {
                            this.load_models(n.clone(), e, cx);
                        }
                    }))), first(&rows), b).into_any_element());
        }
        if p.kind == "api" || p.kind == "local" {
            let endpoint = self.ai_endpoint.read(cx).value().to_string();
            let models: AnyElement = match &self.ai_models {
                None if p.ready => text("Reading its models...", 13., FontWeight::NORMAL, b.gray).into_any_element(),
                None => text(if p.kind == "api" { "Save a key first" } else { "Point it at its server first" }, 13., FontWeight::NORMAL, b.gray).into_any_element(),
                Some(Err(e)) => text(e.clone(), 13., FontWeight::NORMAL, b.red_text).into_any_element(),
                Some(Ok(list)) => div().id("model-list").max_h(px(160.)).overflow_y_scroll().flex().flex_wrap().justify_end().gap(px(8.))
                    .children(list.iter().take(60).enumerate().map(|(i, m)| {
                        let mut cmd = vec!["ai".to_string(), name.clone(), m.clone()];
                        if p.kind == "local" && !endpoint.is_empty() {
                            cmd.extend(["--endpoint".to_string(), endpoint.clone()]);
                        }
                        pill(("model", i), None, m.clone(), p.in_use && *m == s.ai.model, b).on_click(cx.listener(move |this, _, window, cx| this.change(cmd.clone(), window, cx)))
                    })).into_any_element(),
            };
            rows.push(row("Models", "The one edits asks; picking one selects this provider.", models, first(&rows), b).into_any_element());
        }
        rows
    }

    /// One of the providers here, as t3code's ProviderInstanceCard: its logo and name, then "Authenticated as ... ·
    /// its plan". A click selects it (an API provider needs a model first: its models open instead); the chevron opens
    /// what it runs, its key or server, its models.
    fn provider_card(&self, i: usize, p: &Provider, s: &Settings, b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let open = self.ai_open.as_deref() == Some(p.name.as_str());
        let (name, kind_, base) = (p.name.clone(), p.kind.clone(), p.base.clone());
        let select: Option<Vec<String>> = (!p.in_use && p.kind != "api").then(|| {
            let mut cmd = vec!["ai".to_string(), p.name.clone()];
            if p.kind != "subscription" {
                cmd.extend(["--endpoint".to_string(), if p.kind == "acp" { p.command.clone() } else { p.base.clone() }]);
            }
            cmd
        });
        let status = match (p.account.is_empty(), p.plan.is_empty()) {
            (false, false) => format!("Authenticated as {} · {}", masked(&p.account), p.plan),
            (false, true) => format!("Authenticated as {}", masked(&p.account)),
            (true, false) => p.plan.clone(),
            _ => p.about.clone(),
        };
        let (hover, toggle_name) = (b.hair, name.clone());
        let toggle = cx.listener(move |this: &mut Self, _: &ClickEvent, window: &mut Window, cx: &mut Context<Self>| {
            cx.stop_propagation();
            this.toggle_provider(&toggle_name, &kind_, &base, window, cx);
        });
        let (kind2, base2) = (p.kind.clone(), p.base.clone());
        let head = div().id(("provider", i)).px(px(16.)).py(px(12.)).flex().items_center().gap(px(12.)).cursor_pointer().hover(move |st| st.bg(hover))
            .when(p.in_use, |d| d.bg(b.accent.opacity(0.08)))
            .child(div().size(px(32.)).flex_none().flex().items_center().justify_center().rounded(SMALL).bg(b.card)
                .children(crate::app::icons::logo(&p.name, b.ink, b.card).map(|l| l.size(px(18.)))))
            .child(div().flex_1().min_w_0().flex().flex_col().gap(px(2.))
                .child(text(p.label.clone(), 14., BOLD, b.ink))
                .child(text(status, 12., LIGHT, b.gray).truncate()))
            .when(p.in_use, |d| d.child(div().flex().items_center().gap(px(4.)).child(icon(Icon::Check, b.accent).size(px(14.))).child(text("In use", 12., BOLD, b.accent))))
            .child(div().id(("provider-more", i)).size(px(28.)).flex().items_center().justify_center().rounded(SMALL).hover(move |st| st.bg(hover))
                .tooltip(|w, cx| gpui_kit::component::tooltip::Tooltip::new("Details").build(w, cx))
                .child(icon(if open { Icon::ChevronDown } else { Icon::ChevronRight }, b.gray).size(px(16.)))
                .on_click(toggle))
            .on_click(cx.listener(move |this, _, window, cx| match &select {
                Some(cmd) => this.change(cmd.clone(), window, cx),
                // an API provider is picked with a model: its models, open
                None if !this.ai_open.as_deref().is_some_and(|n| n == name) && kind2 == "api" => this.toggle_provider(&name, &kind2, &base2, window, cx),
                None => {}
            }));
        div().flex().flex_col().when(i > 0, |d| d.border_t_1().border_color(b.hair)).child(head)
            .when(open, |d| d.child(div().pb(px(8.)).flex().flex_col().children(self.provider_setup(p, s, b, cx))))
            .into_any_element()
    }

    /// Opens or closes a provider's details, reading its server and models.
    fn toggle_provider(&mut self, name: &str, kind_: &str, base: &str, window: &mut Window, cx: &mut Context<Self>) {
        let opening = self.ai_open.as_deref() != Some(name);
        self.ai_open = opening.then(|| name.to_string());
        self.ai_models = None;
        if opening && (kind_ == "local" || kind_ == "acp") {
            let base = base.to_string();
            self.ai_endpoint.update(cx, |s, cx| s.set_value(base, window, cx));
        }
        if opening && (kind_ == "api" || kind_ == "local") {
            self.load_models(name.to_string(), if kind_ == "local" { base.to_string() } else { String::new() }, cx);
        }
        cx.notify();
    }

    /// t3code's Add provider: every provider not here yet, on a tile with its logo and line; picked, set up below.
    fn add_provider(&self, s: &Settings, b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let rest: Vec<&Provider> = s.providers.iter().filter(|p| !p.ready && !p.in_use).collect();
        let shown = if self.ai_more { rest.len() } else { rest.len().min(9) };
        let tiles = rest.iter().take(shown).enumerate().map(|(i, p)| {
            let (on, hover, name, kind_, base) = (self.ai_open.as_deref() == Some(p.name.as_str()), b.hair, p.name.clone(), p.kind.clone(), p.base.clone());
            div().id(("add", i)).w(relative(0.32)).p(px(16.)).flex().flex_col().gap(px(8.)).rounded(SMALL).cursor_pointer()
                .bg(if on { b.accent.opacity(0.14) } else { b.page }).hover(move |st| st.bg(hover))
                .child(div().flex().items_center().gap(px(8.))
                    .children(crate::app::icons::logo(&p.name, b.ink, b.page).map(|l| l.size(px(18.))))
                    .child(text(p.label.clone(), 14., BOLD, b.ink)))
                .child(text(p.about.clone(), 12., LIGHT, b.gray).h(px(40.)).line_clamp(2))
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.ai_open = Some(name.clone());
                    this.ai_models = None;
                    if kind_ == "local" || kind_ == "acp" {
                        this.ai_endpoint.update(cx, |s, cx| s.set_value(base.clone(), window, cx));
                    }
                    cx.notify();
                }))
                .into_any_element()
        }).collect::<Vec<_>>();
        let picked = self.ai_open.as_ref().and_then(|n| rest.iter().find(|p| &p.name == n)).copied();
        let setup: AnyElement = match picked {
            Some(p) if p.kind == "subscription" => group(&p.label, vec![row("Not on this computer yet", format!("Install it, sign in, and it shows up among your providers by itself. It runs: {}", p.command.replace("{model}", "")), div(), true, b).into_any_element()], b).into_any_element(),
            Some(p) => group(&p.label, self.provider_setup(p, s, b, cx), b).into_any_element(),
            None => div().into_any_element(),
        };
        div().absolute().inset_0().flex().items_center().justify_center()
            .child(crate::app::ui::backdrop("add-backdrop", b).on_click(cx.listener(|this, _, _, cx| {
                this.ai_adding = false;
                cx.notify();
            })))
            .child(div().id("add-provider").relative().occlude().shadow_2xl().w(relative(0.9)).max_h(relative(0.9)).p(px(24.)).flex().flex_col().gap(px(16.)).rounded(CARD).bg(b.card).overflow_y_scroll()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(div().flex().items_center()
                    .child(div().flex_1().flex().flex_col().gap(px(4.)).child(text("Add provider", 19., BOLD, b.ink)).child(text("Pick one; the agents you install show up by themselves.", 13., LIGHT, b.gray)))
                    .child(div().id("close-add").size(px(32.)).flex().items_center().justify_center().rounded(SMALL).cursor_pointer().child(text("×", 20., LIGHT, b.gray))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.ai_adding = false;
                            cx.notify();
                        }))))
                .child(div().flex().flex_wrap().gap(px(8.)).children(tiles))
                .when(rest.len() > 9, |d| d.child(pill("more-providers", None, if self.ai_more { "Show less" } else { "More providers" }, false, b).on_click(cx.listener(|this, _, _, cx| {
                    this.ai_more = !this.ai_more;
                    cx.notify();
                }))))
                .child(setup))
            .into_any_element()
    }

    fn section(&self, section: Section, s: &Settings, b: &Brand, cx: &mut Context<Self>) -> Vec<AnyElement> {
        match section {
            Section::General => {
                let out = s.out.clone();
                vec![group("", vec![
                    row("Where edits go", s.out.display().to_string(), pill("out", Some(Icon::Folder), "Open", false, b).on_click(move |_, _, cx| cx.open_with_system(&out)), true, b).into_any_element(),
                    row("What renders the video", "auto uses the graphics card when it works, else the processor.", self.choice("encoder", &s.encoders, &s.encoder, &["encoder"], b, cx), false, b).into_any_element(),
                    row("Discord", "Shows Playing EditsMaker on your profile and what you are making. Your friends also see a Download button: Discord never shows you your own.",
                        Switch::new("discord").checked(theme::prefs().discord).on_click(|on: &bool, window, cx| theme::save(&Prefs { discord: *on, ..theme::prefs() }, window, cx)), false, b).into_any_element(),
                ], b).into_any_element()]
            }
            Section::Ai => {
                // t3code's providers: the ones here (installed, keyed, pointed at) already listed, the rest to add
                let here: Vec<&Provider> = s.providers.iter().filter(|p| p.ready || p.in_use).collect();
                let cards = here.iter().enumerate().map(|(i, p)| self.provider_card(i, p, s, b, cx)).collect::<Vec<_>>();
                vec![
                    div().flex().items_center().gap(px(16.))
                        .child(div().flex_1().flex().flex_col().gap(px(4.))
                            .child(text("Providers", 13., BOLD, b.gray))
                            .child(text(format!("edits asks the selected one to pick moments. {}", if s.ai.ready { "It is ready." } else { "It is not ready: its command or key is missing." }), 13., LIGHT, b.gray)))
                        .child(primary("add-provider-open", "Add provider", b).on_click(cx.listener(|this, _, _, cx| {
                            this.ai_adding = true;
                            this.ai_open = None;
                            cx.notify();
                        })))
                        .into_any_element(),
                    div().flex().flex_col().bg(b.page).rounded(CARD).overflow_hidden().children(cards).into_any_element(),
                ]
            }
            Section::Accounts => {
                let mut rows: Vec<AnyElement> = s.accounts.iter().enumerate().map(|(i, a)| {
                    row(&a.name, format!("{} · {}", a.platform, a.user), self.button(("disconnect", i), "Disconnect", &["disconnect", &a.name], b, cx), i == 0, b).into_any_element()
                }).collect();
                let first = rows.is_empty();
                let sites = div().flex().gap(px(8.)).children(s.sites.iter().enumerate().map(|(i, site)| self.button(("connect", i), site, &["connect", site], b, cx)));
                rows.push(row("Connect an account", "Signs in once in a window of its own; edits never sees the password.", sites, first, b).into_any_element());
                let token = |i: usize, platform: &'static str, b: &Brand, cx: &mut Context<Self>| {
                    let [who, secret] = [self.sign_in[i * 2].clone(), self.sign_in[i * 2 + 1].clone()];
                    div().flex().items_center().gap(px(8.))
                        .child(div().w(px(140.)).child(Input::new(&who)))
                        .child(div().w(px(140.)).child(Input::new(&secret)))
                        .child(primary(("sign-in", i), "Connect", b).on_click(cx.listener(move |this, _, window, cx| {
                            let (w, k) = (who.read(cx).value().to_string(), secret.read(cx).value().to_string());
                            let clear = secret.clone();
                            this.changing = Some(format!("connect {platform}"));
                            cx.notify();
                            cx.spawn_in(window, async move |this, cx| {
                                let r = cx.background_executor().spawn(async move { crate::social::connect_token(platform, &w, &k) }).await;
                                let _ = this.update_in(cx, |this, window, cx| {
                                    this.changing = None;
                                    match r {
                                        Ok(said) => {
                                            clear.update(cx, |s, cx| s.set_value("", window, cx));
                                            window.push_notification(Notification::success(said), cx);
                                        }
                                        Err(e) => window.push_notification(Notification::error(e).title(format!("{platform} did not connect")), cx),
                                    }
                                    this.load_settings(cx);
                                });
                            })
                            .detach();
                        })))
                };
                let social = vec![
                    row("Bluesky", "Your handle and an app password (bsky.app, Settings, App passwords): never your main password.", token(0, "bluesky", b, cx), true, b).into_any_element(),
                    row("Facebook Page", "The Page's id and a Page access token that can post (pages_manage_posts).", token(1, "facebook", b, cx), false, b).into_any_element(),
                ];
                vec![group("", rows, b).into_any_element(), group("With a token", social, b).into_any_element()]
            }
            Section::Agents => vec![group("", vec![
                row("MCP server", "edits as tools in Claude Code, ChatGPT, Cursor and every other agent on this computer.", self.button("mcp", "Install", &["mcp", "install"], b, cx), true, b).into_any_element(),
                row("Skills", "The packs' agent skills in each agent's skills folder.", self.button("skills", "Install", &["skills", "install"], b, cx), false, b).into_any_element(),
            ], b).into_any_element()],
            Section::Usage => self.usage_section(b),
            Section::Appearance => self.appearance(b, cx),
            Section::About => {
                let doctor: Option<AnyElement> = self.doctor.as_ref().map(|d| card(b).bg(b.page).gap(px(4.)).children(d.lines().map(|l| text(l.to_string(), 12., FontWeight::NORMAL, b.gray).font_family(MONO))).into_any_element());
                let link = |id: &'static str, i: Icon, label: &'static str, url: String, b: &Brand| pill(id, Some(i), label, false, b).on_click(move |_, _, cx| cx.open_url(&url)).into_any_element();
                let repo = format!("https://github.com/{}", crate::update::REPO);
                let program = std::env::current_exe().unwrap_or_default();
                let (latest_line, newer): (String, bool) = match &self.latest {
                    None => ("Asking GitHub...".into(), false),
                    Some(Ok(t)) if crate::update::newer(t) => (format!("{t} is out; you have {}.", s.version), true),
                    Some(Ok(t)) => (format!("{t}: up to date."), false),
                    Some(Err(_)) => ("Unknown: GitHub did not answer (offline?).".into(), false),
                };
                let update: AnyElement = if newer {
                    primary("update-now", "Update now", b).on_click(cx.listener(|this, _, window, cx| this.launch(vec!["update".into()], window, cx))).into_any_element()
                } else {
                    pill("check-again", Some(Icon::Reset), "Check again", false, b).on_click(cx.listener(|this, _, _, cx| {
                        this.latest = None;
                        this.check_latest(cx);
                    })).into_any_element()
                };
                let mut v = vec![
                    group("", vec![
                        row("EditsMaker", format!("Version {}", s.version), link("site", Icon::Github, "GitHub", repo.clone(), b), true, b).into_any_element(),
                        row("Made by", "Onur Konuk, on GitHub and X.", div().flex().gap(px(8.)).child(link("author-gh", Icon::Github, "OnurByte", crate::update::GITHUB.into(), b)).child(link("author-x", Icon::External, "@onurbinary", crate::update::X.into(), b)), false, b).into_any_element(),
                        row("Issues", "Something broke, or an idea: say it on GitHub.", link("issues", Icon::External, "Open an issue", format!("{repo}/issues"), b), false, b).into_any_element(),
                        row("Program", program.display().to_string(), pill("program", Some(Icon::Folder), "Show", false, b).on_click(move |_, _, cx| cx.reveal_path(&program)), false, b).into_any_element(),
                    ], b).into_any_element(),
                    group("Updates", vec![
                        row("Latest release", latest_line, update, true, b).into_any_element(),
                        row("Update by itself", "Once a day, the newest release before a run.", Switch::new("auto-update").checked(s.updates).on_click(cx.listener(|this, on: &bool, window, cx| {
                            this.change(vec!["update".into(), if *on { "on" } else { "off" }.into()], window, cx);
                        })), false, b).into_any_element(),
                        row("Roll back", "Back to the version the last update replaced.", self.button("rollback", "Roll back", &["update", "rollback"], b, cx), false, b).into_any_element(),
                    ], b).into_any_element(),
                    group("This computer", vec![
                        row("Check this computer", "ffmpeg, yt-dlp, whisper and the AI, as edits doctor sees them.", primary("doctor", "Check", b).on_click(cx.listener(|this, _, _, cx| this.check(cx))), true, b).into_any_element(),
                    ], b).into_any_element(),
                ];
                v.extend(doctor);
                v.push(group("Legal", LEGAL.iter().enumerate().map(|(i, &(title, about, file))| {
                    let url = format!("https://github.com/OnurByte/EditsMaker/blob/main/{file}");
                    row(title, about, pill(("legal", i), Some(Icon::External), "Read", false, b).on_click(move |_, _, cx| cx.open_url(&url)), i == 0, b).into_any_element()
                }).collect(), b).into_any_element());
                v
            }
        }
    }

    /// Usage: what edits made, and what the AI was asked.
    fn usage_section(&self, b: &Brand) -> Vec<AnyElement> {
        let Some(u) = &self.usage else {
            return vec![text("Counting...", 14., FontWeight::NORMAL, b.gray).into_any_element()];
        };
        let tile = |n: String, what: &str, b: &Brand| div().flex_1().p(px(16.)).flex().flex_col().gap(px(4.)).rounded(CARD).bg(b.page)
            .child(text(n, 28., BOLD, b.ink)).child(text(what.to_string(), 13., LIGHT, b.gray));
        let most = u.days.iter().copied().max().unwrap_or(0).max(1);
        let week: usize = u.days.iter().rev().take(7).sum();
        let bars = div().h(px(96.)).flex().items_end().gap(px(6.)).children(u.days.iter().enumerate().map(|(i, &n)| {
            let h = if n == 0 { 3. } else { 8. + 80. * n as f32 / most as f32 };
            div().flex_1().h(px(h)).rounded(px(3.)).bg(if i == u.days.len() - 1 { b.accent } else { b.accent.opacity(if n == 0 { 0.15 } else { 0.5 }) })
        }));
        let ai_rows: Vec<AnyElement> = if u.ai.is_empty() {
            vec![row("No AI calls yet", "Each time edits asks the AI to pick moments, it is counted here.", div(), true, b).into_any_element()]
        } else {
            u.ai.iter().enumerate().map(|(i, (name, a))| {
                let label = self.settings.as_ref().and_then(|s| s.as_ref().ok()).and_then(|s| s.providers.iter().find(|p| &p.name == name)).map_or(name.clone(), |p| p.label.clone());
                row(&label, format!("{} answered of {}, {:.0} s spent, about {} tokens", a.ok, a.calls, a.secs, a.chars / 4),
                    text(format!("{}", a.calls), 20., BOLD, b.ink), i == 0, b).into_any_element()
            }).collect()
        };
        vec![
            div().flex().gap(px(8.)).child(tile(u.runs.to_string(), "edits made", b)).child(tile(u.clips.to_string(), "clips", b))
                .child(tile(crate::app::usage::size(u.bytes), "on disk", b)).child(tile(week.to_string(), "this week", b)).into_any_element(),
            div().flex().flex_col().gap(px(8.)).child(text("The last two weeks", 13., BOLD, b.gray)).child(div().p(px(16.)).rounded(CARD).bg(b.page).child(bars)).into_any_element(),
            group("AI", ai_rows, b).into_any_element(),
        ]
    }

    /// t3code's AppearanceSettingsPanel: the mode tiles (System, Light, Dark, each a little window in its colours),
    /// the theme library (EditsMaker's own, and Omarchy's live one where it runs), then Interface and Typography rows,
    /// each with a reset once it is off its default.
    fn appearance(&self, b: &Brand, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let p = theme::prefs();
        let d = Prefs::default();
        let omarchy = theme::omarchy_dir().is_some();
        let on_omarchy = p.mode == "omarchy" && omarchy;
        let accent = theme::hex_of(&p.accent);
        let (light, dark) = (theme::palette(false, accent), theme::palette(true, accent));
        let tiles = [("system", "System"), ("light", "Light"), ("dark", "Dark")].into_iter().map(|(m, label)| {
            let on = !on_omarchy && p.mode == m;
            let preview = match m {
                "light" => wireframe(&light).w_full(),
                "dark" => wireframe(&dark).w_full(),
                // the light one, its right half covered by the dark one
                _ => div().w_full().h(px(96.)).relative().overflow_hidden().rounded(SMALL)
                    .child(wireframe(&light).absolute().inset_0())
                    .child(div().absolute().top_0().bottom_0().right_0().w(relative(0.5)).overflow_hidden()
                        .child(wireframe(&dark).absolute().top_0().right_0().h_full().w(relative(2.)))),
            };
            mode_tile(m, preview, label, on, b).on_click(cx.listener(move |_, _, window, cx| {
                theme::save(&Prefs { mode: m.into(), mode_picked: true, ..theme::prefs() }, window, cx);
            })).into_any_element()
        }).collect::<Vec<_>>();
        // the theme library: EditsMaker's colours, and Omarchy's
        let mut themes: Vec<(&'static str, String, String, Brand)> = vec![("edits", "EditsMaker".into(), "Ink and paper, one warm grey, your accent".into(), if theme::is_dark(cx) && !on_omarchy { dark } else { light })];
        if omarchy {
            let name = std::fs::read_to_string(crate::util::home().join(".local/state/omarchy/current/theme.name")).unwrap_or_default();
            if let Some(ob) = theme::omarchy_brand() {
                themes.push(("omarchy", format!("Omarchy · {}", name.trim()), "Follows the theme you pick in Omarchy, live".into(), ob));
            }
        }
        let cards = themes.into_iter().map(|(id, label, about, c)| {
            let on = if id == "omarchy" { on_omarchy } else { !on_omarchy };
            let hover = b.accent.opacity(0.06);
            div().id(SharedString::from(format!("theme-{id}"))).flex_1().min_w(px(220.)).p(px(12.)).flex().items_center().gap(px(12.)).rounded(px(12.)).cursor_pointer()
                .bg(if on { b.accent.opacity(0.14) } else { b.page }).border_1().border_color(if on { b.accent } else { b.hair })
                .when(!on, |d| d.hover(move |s| s.bg(hover)))
                .child(div().size(px(40.)).flex_none().rounded_full().overflow_hidden().border_1().border_color(b.hair).flex()
                    .child(div().w(relative(0.5)).h_full().bg(c.page)).child(div().w(relative(0.5)).h_full().bg(c.card).flex().items_center().justify_center().child(div().size(px(8.)).rounded_full().bg(c.accent))))
                .child(div().flex_1().min_w_0().flex().flex_col().gap(px(2.))
                    .child(text(label, 13., BOLD, b.ink).truncate())
                    .child(text(about, 12., LIGHT, b.gray).truncate()))
                .when(on, |d| d.child(icon(Icon::Check, b.accent).size(px(16.))))
                .on_click(cx.listener(move |_, _, window, cx| {
                    let p = theme::prefs();
                    // back from Omarchy: the mode picked before, else the system's
                    let mode = if id == "omarchy" { "omarchy".to_string() } else if p.mode == "omarchy" { "system".into() } else { p.mode.clone() };
                    theme::save(&Prefs { mode, mode_picked: true, ..p }, window, cx);
                }))
                .into_any_element()
        }).collect::<Vec<_>>();
        let reset = |id: &'static str, show: bool, f: fn(Prefs) -> Prefs, b: &Brand, cx: &mut Context<Self>| -> AnyElement {
            if !show {
                return div().into_any_element();
            }
            crate::app::ui::icon_button(id, Icon::Reset, b).size(px(24.))
                .tooltip(|w, cx| gpui_kit::component::tooltip::Tooltip::new("Reset to default").build(w, cx))
                .on_click(cx.listener(move |_, _, window, cx| theme::save(&f(theme::prefs()), window, cx))).into_any_element()
        };
        // t3code's accent; Omarchy's own when following Omarchy
        let accents: AnyElement = if on_omarchy {
            text("Omarchy's accent", 13., FontWeight::NORMAL, b.gray).into_any_element()
        } else {
            div().flex().items_center().gap(px(8.)).child(reset("reset-accent", p.accent != d.accent, |p| Prefs { accent: Prefs::default().accent, ..p }, b, cx))
                .children(theme::ACCENTS.iter().enumerate().map(|(i, &(name, hexv))| {
                    let on = p.accent.eq_ignore_ascii_case(hexv);
                    div().id(("accent", i)).size(px(24.)).rounded_full().cursor_pointer().bg(theme::hex_of(hexv))
                        .when(on, |d| d.border_2().border_color(b.ink))
                        .tooltip(move |window, cx| gpui_kit::component::tooltip::Tooltip::new(name).build(window, cx))
                        .on_click(cx.listener(move |_, _, window, cx| theme::save(&Prefs { accent: hexv.into(), ..theme::prefs() }, window, cx)))
                })).into_any_element()
        };
        let width = div().flex().items_center().gap(px(8.)).child(reset("reset-width", p.width != d.width, |p| Prefs { width: Prefs::default().width, ..p }, b, cx))
            .child(segmented("width", &[("narrow", "Narrow"), ("normal", "Comfortable"), ("wide", "Wide")], &p.width, b, cx, |v| Prefs { width: v.into(), ..theme::prefs() }));
        let size = div().flex().items_center().gap(px(8.)).child(reset("reset-size", p.font_size != d.font_size, |p| Prefs { font_size: Prefs::default().font_size, ..p }, b, cx))
            .child(segmented("size", &[("13", "13"), ("14", "14"), ("15", "15"), ("16", "16"), ("17", "17")], &format!("{}", p.font_size), b, cx, |v| Prefs { font_size: v.parse().unwrap_or(15.), ..theme::prefs() }));
        vec![
            div().flex().flex_col().gap(px(12.))
                .child(div().flex().gap(px(12.)).children(tiles))
                .child(div().flex().flex_wrap().gap(px(8.)).children(cards)).into_any_element(),
            group("Interface", vec![
                row("Accent", "What is selected, on and focused, and the button that does the thing.", accents, true, b).into_any_element(),
                row("Chat width", "How wide the messages and the composer grow on a large screen.", width, false, b).into_any_element(),
            ], b).into_any_element(),
            group("Typography", vec![row("Interface font size", "Everything but the frames and the log.", size, true, b).into_any_element()], b).into_any_element(),
        ]
    }

    /// The settings window, over the app.
    pub fn settings_window(&self, at: Section, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let shown = found(&self.settings_search.read(cx).value());
        let nav = SECTIONS.iter().filter(|x| shown.contains(&x.0)).map(|&(s, label, i, _)| {
            let on = s == at;
            let (bg, hover) = (if on { b.accent.opacity(0.14) } else { transparent_black() }, b.hair);
            div().id(label).h(px(32.)).px(px(8.)).flex().items_center().gap(px(8.)).rounded(SMALL).cursor_pointer().bg(bg).hover(move |st| st.bg(hover))
                .child(icon(i, if on { b.accent } else { b.gray }).size(px(16.)))
                .child(text(label, 14., FontWeight::NORMAL, if on { b.ink } else { b.gray }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings_open = Some(s);
                    if s == Section::Usage {
                        this.usage = Some(crate::app::usage::read(&this.past));
                    }
                    if s == Section::About {
                        this.check_latest(cx);
                    }
                    cx.notify();
                }))
                .into_any_element()
        }).collect::<Vec<_>>();
        let body: Vec<AnyElement> = match &self.settings {
            None => vec![text("Reading the settings...", 14., FontWeight::NORMAL, b.gray).into_any_element()],
            Some(Err(e)) => vec![text(e.clone(), 14., FontWeight::NORMAL, b.red_text).into_any_element()],
            Some(Ok(s)) => self.section(at, s, &b, cx),
        };
        let title = SECTIONS.iter().find(|x| x.0 == at).map_or("", |x| x.1);
        let add = match (&self.settings, self.ai_adding && at == Section::Ai) {
            (Some(Ok(s)), true) => Some(self.add_provider(s, &b, cx)),
            _ => None,
        };
        div().absolute().inset_0().flex().items_center().justify_center()
            .child(crate::app::ui::backdrop("backdrop", &b).on_click(cx.listener(|this, _, _, cx| {
                this.settings_open = None;
                cx.notify();
            })))
            .child(div().id("settings-window").relative().occlude().shadow_2xl().w(px(920.)).max_w(relative(0.94)).h(relative(0.86)).max_h(px(680.)).flex().rounded(CARD).bg(b.card).overflow_hidden()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(div().w(px(200.)).flex_none().p(px(16.)).flex().flex_col().gap(px(2.))
                    .child(div().px(px(8.)).pb(px(16.)).flex().items_center().gap(px(8.)).child(icon(Icon::Settings, b.gray).size(px(16.))).child(text("Settings", 15., BOLD, b.ink)))
                    .child(div().pb(px(8.)).child(Input::new(&self.settings_search).prefix(icon(Icon::Search, b.gray).size(px(14.)))))
                    .children(nav)
                    .when(shown.is_empty(), |d| d.child(text("Nothing like that", 13., LIGHT, b.gray).px(px(8.)))))
                .child(div().flex_1().min_w_0().flex().flex_col()
                    .child(div().h(px(52.)).flex_none().px(px(24.)).flex().items_center().gap(px(8.))
                        .child(text(title, 19., BOLD, b.ink).flex_1())
                        .children(self.changing.as_ref().map(|c| text(format!("edits {c}..."), 13., FontWeight::NORMAL, b.gray)))
                        .child(div().id("close-settings").size(px(32.)).flex().items_center().justify_center().rounded(SMALL).cursor_pointer()
                            .child(text("×", 20., LIGHT, b.gray))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.settings_open = None;
                                cx.notify();
                            }))))
                    .child(div().id("settings-body").flex_1().min_h_0().overflow_y_scroll()
                        .child(div().px(px(24.)).pb(px(24.)).flex().flex_col().gap(px(24.)).children(body))))
                .children(add))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn search_finds_the_section_a_setting_is_in() {
        use super::{found, Section};
        assert_eq!(found("omarchy"), [Section::Appearance]);
        assert_eq!(found("tiktok"), [Section::Accounts]);
        assert!(found("claude").contains(&Section::Ai) && found("claude").contains(&Section::Agents));
        assert_eq!(found("").len(), 7);
        assert_eq!(found("discord"), [Section::General]);
        assert_eq!(found("privacy"), [Section::About]);
        assert_eq!(found("rollback"), [Section::About]);
        for (_, _, f) in super::LEGAL {
            assert!(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(f).is_file(), "{f} is in the repository");
        }
        assert!(found("zzz").is_empty());
        assert_eq!(super::masked("onur@gmail.com"), "o•••@gmail.com");
        assert_eq!(super::masked("onur"), "onur");
    }
}
