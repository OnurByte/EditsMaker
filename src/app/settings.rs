//! t3code's settings, ported: a window over the app (the backdrop closes it), its sections on the left
//! (SettingsSidebarNav), the section's groups on the right (SettingsGroup: a card whose rows are split by hairlines,
//! SettingsRow: what it is and why on the left, its control on the right). AI is t3code's providers: the one in use,
//! and every other to add, each set up in place (a key, a server, a model). Appearance is t3code's, with Omarchy's
//! own theme followed live. Each change runs the edits code that owns it, so the CLI and the app agree.
use crate::app::icons::{icon, Icon};
use crate::app::runs::{Provider, Settings};
use crate::app::theme::{self, brand, Brand, Prefs, BOLD, CARD, LIGHT, MONO, SMALL};
use crate::app::ui::{card, pill, primary, text};
use crate::app::workspace::{Section, Workspace};
use gpui_kit::component::input::Input;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::WindowExt;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

const SECTIONS: &[(Section, &str)] = &[
    (Section::General, "General"),
    (Section::Ai, "AI"),
    (Section::Accounts, "Accounts"),
    (Section::Agents, "Agents"),
    (Section::Appearance, "Appearance"),
    (Section::About, "About"),
];

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

/// How a provider is reached, in words.
fn kind(p: &Provider) -> &'static str {
    match (p.kind.as_str(), p.how.as_str()) {
        ("subscription", "acp") => "An agent on your subscription, over ACP",
        ("subscription", _) => "An agent on your subscription",
        ("acp", _) => "Any ACP agent, by its command",
        ("api", _) => "An API key",
        _ => "A server on this computer",
    }
}

fn status(p: &Provider) -> &'static str {
    match (p.in_use, p.ready, p.kind.as_str()) {
        (true, _, _) => "In use",
        (_, true, "subscription") => "Installed",
        (_, true, "api") => "Key saved",
        (_, true, _) => "Ready to point at",
        (_, false, "subscription") => "Not installed",
        (_, false, "acp") => "Needs its command",
        _ => "Needs a key",
    }
}

/// Colour dots for an appearance tile: page, card, ink.
fn dots(colors: [Hsla; 3], b: &Brand) -> Div {
    div().flex().gap(px(4.)).children(colors.into_iter().map(|c| div().size(px(16.)).rounded_full().bg(c).border_1().border_color(b.hair)))
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

    /// t3code's provider card, set up in place: the key or the server, then the model.
    fn provider_setup(&self, p: &Provider, s: &Settings, b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let name = p.name.clone();
        let endpoint = self.ai_endpoint.read(cx).value().to_string();
        let mut rows: Vec<AnyElement> = vec![];
        if p.kind == "api" {
            let n = name.clone();
            rows.push(row("API key", format!("Saved where only you can read it. {} works too.", p.key_var),
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
                    }))), true, b).into_any_element());
        }
        if p.kind == "local" {
            let n = name.clone();
            rows.push(row("Server", format!("Where it answers; {} by default.", p.base),
                div().flex().items_center().gap(px(8.)).child(div().w(px(220.)).child(Input::new(&self.ai_endpoint)))
                    .child(pill("models", None, "Look", false, b).on_click(cx.listener(move |this, _, _, cx| {
                        let e = this.ai_endpoint.read(cx).value().to_string();
                        this.load_models(n.clone(), e, cx);
                    }))), true, b).into_any_element());
        }
        if p.kind == "acp" {
            let n = name.clone();
            rows.push(row("Agent's command", "How it starts in ACP mode, as the ACP Registry lists it.",
                div().flex().items_center().gap(px(8.)).child(div().w(px(220.)).child(Input::new(&self.ai_endpoint)))
                    .child(primary("use-acp", "Use it", b).on_click(cx.listener(move |this, _, window, cx| {
                        let e = this.ai_endpoint.read(cx).value().to_string();
                        this.change(vec!["ai".into(), n.clone(), "--endpoint".into(), e], window, cx);
                    }))), true, b).into_any_element());
            return group(&format!("{} · {}", p.label, kind(p)), rows, b).into_any_element();
        }
        let models: AnyElement = match (&self.ai_models, p.kind.as_str()) {
            (_, "subscription") => {
                let cmd = vec!["ai".to_string(), name.clone()];
                div().flex().items_center().gap(px(8.))
                    .child(text(p.command.replace("{model}", ""), 12., FontWeight::NORMAL, b.gray).font_family(MONO).max_w(px(220.)).truncate())
                    .child(primary("use", "Use it", b).on_click(cx.listener(move |this, _, window, cx| this.change(cmd.clone(), window, cx))))
                    .into_any_element()
            }
            (None, _) if p.ready || p.kind == "local" => text("Reading its models...", 13., FontWeight::NORMAL, b.gray).into_any_element(),
            (None, _) => text("Save a key first", 13., FontWeight::NORMAL, b.gray).into_any_element(),
            (Some(Err(e)), _) => text(e.clone(), 13., FontWeight::NORMAL, b.red_text).into_any_element(),
            (Some(Ok(list)), _) => div().id("model-list").max_h(px(160.)).overflow_y_scroll().flex().flex_wrap().justify_end().gap(px(8.))
                .children(list.iter().take(60).enumerate().map(|(i, m)| {
                    let mut cmd = vec!["ai".to_string(), name.clone(), m.clone()];
                    if p.kind == "local" && !endpoint.is_empty() {
                        cmd.extend(["--endpoint".to_string(), endpoint.clone()]);
                    }
                    pill(("model", i), None, m.clone(), p.in_use && *m == s.ai.model, b).on_click(cx.listener(move |this, _, window, cx| this.change(cmd.clone(), window, cx)))
                })).into_any_element(),
        };
        rows.push(row(if p.kind == "subscription" { "Use it" } else { "Model" }, if p.kind == "subscription" { "edits asks it on your subscription, with this command; it picks its own model." } else { "The one edits asks; it becomes the one in use." }, models, rows.is_empty(), b).into_any_element());
        group(&format!("{} · {}", p.label, kind(p)), rows, b).into_any_element()
    }

    fn section(&self, section: Section, s: &Settings, b: &Brand, cx: &mut Context<Self>) -> Vec<AnyElement> {
        match section {
            Section::General => {
                let out = s.out.clone();
                let updates = if s.updates { "on" } else { "off" };
                vec![group("", vec![
                    row("Where edits go", s.out.display().to_string(), pill("out", Some(Icon::Folder), "Open", false, b).on_click(move |_, _, cx| cx.open_with_system(&out)), true, b).into_any_element(),
                    row("What renders the video", "auto uses the graphics card when it works, else the processor.", self.choice("encoder", &s.encoders, &s.encoder, &["encoder"], b, cx), false, b).into_any_element(),
                    row("Updates", "Once a day, the newest release before a run.", self.choice("updates", &["on".into(), "off".into()], updates, &["update"], b, cx), false, b).into_any_element(),
                ], b).into_any_element()]
            }
            Section::Ai => {
                let now = s.providers.iter().find(|p| p.in_use);
                let in_use: AnyElement = match now {
                    Some(p) => group("In use", vec![row(&p.label, format!("{} · {}", kind(p), s.ai.about), text(if s.ai.ready { "Ready" } else { "Not ready" }, 13., BOLD, if s.ai.ready { b.ink } else { b.red_text }), true, b).into_any_element()], b).into_any_element(),
                    None => div().into_any_element(),
                };
                // t3code's Add provider: every provider on a tile, the one opened set up below
                let tiles = s.providers.iter().enumerate().map(|(i, p)| {
                    let (on, hover, name) = (self.ai_open.as_deref() == Some(p.name.as_str()), b.hair, p.name.clone());
                    let (kind_, base) = (p.kind.clone(), p.base.clone());
                    div().id(("provider", i)).w(px(196.)).p(px(16.)).flex().flex_col().gap(px(4.)).rounded(SMALL).cursor_pointer()
                        .bg(if on { b.accent.opacity(0.14) } else { b.page }).hover(move |st| st.bg(hover))
                        .child(div().flex().items_center().gap(px(8.)).child(icon(match p.kind.as_str() { "local" => Icon::Sliders, "api" => Icon::Key, _ => Icon::Bot }, if p.in_use { b.accent } else { b.gray }).size(px(16.))).child(text(p.label.clone(), 14., BOLD, b.ink)))
                        .child(text(status(p), 12., FontWeight::NORMAL, b.gray))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.ai_open = if this.ai_open.as_deref() == Some(name.as_str()) { None } else { Some(name.clone()) };
                            this.ai_models = None;
                            if kind_ == "local" || kind_ == "acp" {
                                this.ai_endpoint.update(cx, |s, cx| s.set_value(base.clone(), window, cx));
                            }
                            if this.ai_open.is_some() && (kind_ == "api" || kind_ == "local") {
                                this.load_models(name.clone(), if kind_ == "local" { base.clone() } else { String::new() }, cx);
                            }
                            cx.notify();
                        }))
                        .into_any_element()
                }).collect::<Vec<_>>();
                let setup = self.ai_open.as_ref().and_then(|n| s.providers.iter().find(|p| &p.name == n)).map(|p| self.provider_setup(p, s, b, cx));
                vec![in_use,
                    div().flex().flex_col().gap(px(8.)).child(text("Add provider", 13., BOLD, b.gray)).child(div().flex().flex_wrap().gap(px(8.)).children(tiles)).into_any_element()]
                    .into_iter().chain(setup).collect()
            }
            Section::Accounts => {
                let mut rows: Vec<AnyElement> = s.accounts.iter().enumerate().map(|(i, a)| {
                    row(&a.name, format!("{} · {}", a.platform, a.user), self.button(("disconnect", i), "Disconnect", &["disconnect", &a.name], b, cx), i == 0, b).into_any_element()
                }).collect();
                let first = rows.is_empty();
                let sites = div().flex().gap(px(8.)).children(s.sites.iter().enumerate().map(|(i, site)| self.button(("connect", i), site, &["connect", site], b, cx)));
                rows.push(row("Connect an account", "Signs in once in a window of its own; edits never sees the password. Bluesky and Facebook take a token: edits connect, in a terminal.", sites, first, b).into_any_element());
                vec![group("", rows, b).into_any_element()]
            }
            Section::Agents => vec![group("", vec![
                row("MCP server", "edits as tools in Claude Code, ChatGPT, Cursor and every other agent on this computer.", self.button("mcp", "Install", &["mcp", "install"], b, cx), true, b).into_any_element(),
                row("Skills", "The packs' agent skills in each agent's skills folder.", self.button("skills", "Install", &["skills", "install"], b, cx), false, b).into_any_element(),
            ], b).into_any_element()],
            Section::Appearance => self.appearance(b, cx),
            Section::About => {
                let doctor: AnyElement = match &self.doctor {
                    Some(d) => card(b).bg(b.page).gap(px(4.)).children(d.lines().map(|l| text(l.to_string(), 12., FontWeight::NORMAL, b.gray).font_family(MONO))).into_any_element(),
                    None => div().into_any_element(),
                };
                vec![
                    group("", vec![
                        row("EditsMaker", format!("Version {}", s.version), pill("site", Some(Icon::Github), "GitHub", false, b).on_click(|_, _, cx| cx.open_url("https://github.com/OnurByte/EditsMaker")), true, b).into_any_element(),
                        row("Check this computer", "ffmpeg, yt-dlp, whisper and the AI, as edits doctor sees them.", primary("doctor", "Check", b).on_click(cx.listener(|this, _, _, cx| this.check(cx))), false, b).into_any_element(),
                    ], b).into_any_element(),
                    doctor,
                ]
            }
        }
    }

    /// t3code's appearance: colours and themes (Omarchy's live one too), the interface, the type.
    fn appearance(&self, b: &Brand, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let p = theme::prefs();
        let hex = |v: u32| -> Hsla { rgb(v).into() };
        let mut modes: Vec<(&str, Icon, String, [Hsla; 3])> = vec![
            ("system", Icon::Sliders, "System".into(), [hex(0xFFFFFF), hex(0x0D1117), hex(0x9A958A)]),
            ("light", Icon::Sun, "Light".into(), [hex(0xFFFFFF), hex(0xF6F4EF), hex(0x0B0B0C)]),
            ("dark", Icon::Moon, "Dark".into(), [hex(0x0D1117), hex(0x161B22), hex(0xF4F1EA)]),
        ];
        if theme::omarchy_dir().is_some() {
            let name = std::fs::read_to_string(crate::util::home().join(".local/state/omarchy/current/theme.name")).unwrap_or_default();
            let ob = if p.mode == "omarchy" { *b } else { theme::brand(cx) };
            modes.push(("omarchy", Icon::Palette, format!("Omarchy · {}", name.trim()), [ob.page, ob.card, ob.ink]));
        }
        let tiles = modes.into_iter().map(|(m, i, label, colors)| {
            let (on, hover) = (p.mode == m, b.hair);
            div().id(m).w(px(160.)).p(px(16.)).flex().flex_col().gap(px(8.)).rounded(SMALL).cursor_pointer().bg(if on { b.accent.opacity(0.14) } else { b.page }).hover(move |st| st.bg(hover))
                .child(div().flex().items_center().justify_between().child(dots(colors, b)).child(icon(i, b.gray).size(px(16.))))
                .child(text(label, 13., if on { BOLD } else { FontWeight::NORMAL }, b.ink).truncate())
                .on_click(cx.listener(move |_, _, window, cx| {
                    theme::save(&Prefs { mode: m.into(), ..theme::prefs() }, window, cx);
                }))
        });
        let width = div().flex().gap(px(8.)).children(["narrow", "normal", "wide"].into_iter().map(|w| {
            pill(w, None, w, p.width == w, b).on_click(cx.listener(move |_, _, window, cx| theme::save(&Prefs { width: w.into(), ..theme::prefs() }, window, cx)))
        }));
        // t3code's accent; Omarchy's own when following Omarchy
        let accent: AnyElement = if p.mode == "omarchy" && theme::omarchy_dir().is_some() {
            text("Omarchy's accent", 13., FontWeight::NORMAL, b.gray).into_any_element()
        } else {
            div().flex().gap(px(8.)).children(theme::ACCENTS.iter().enumerate().map(|(i, &(name, hexv))| {
                let on = p.accent.eq_ignore_ascii_case(hexv);
                div().id(("accent", i)).size(px(28.)).rounded_full().cursor_pointer().bg(theme::hex_of(hexv))
                    .when(on, |d| d.border_2().border_color(b.ink))
                    .tooltip(move |window, cx| gpui_kit::component::tooltip::Tooltip::new(name).build(window, cx))
                    .on_click(cx.listener(move |_, _, window, cx| theme::save(&Prefs { accent: hexv.into(), ..theme::prefs() }, window, cx)))
            })).into_any_element()
        };
        let size = div().flex().gap(px(8.)).children([14.0f32, 15., 16.].into_iter().map(|z| {
            pill(("size", z as usize), None, format!("{z}"), p.font_size == z, b).on_click(cx.listener(move |_, _, window, cx| theme::save(&Prefs { font_size: z, ..theme::prefs() }, window, cx)))
        }));
        vec![
            div().flex().flex_col().gap(px(8.)).child(text("Colors & themes", 13., BOLD, b.gray)).child(div().flex().flex_wrap().gap(px(8.)).children(tiles))
                .when(theme::omarchy_dir().is_some(), |d| d.child(text("Omarchy follows the theme you pick in Omarchy, as it changes.", 13., LIGHT, b.gray))).into_any_element(),
            group("Interface", vec![
                row("Accent", "What is selected, on and focused, and the button that does the thing.", accent, true, b).into_any_element(),
                row("Chat width", "How wide the messages and the composer grow on a large screen.", width, false, b).into_any_element(),
            ], b).into_any_element(),
            group("Typography", vec![row("Text size", "Everything but the frames and the log.", size, true, b).into_any_element()], b).into_any_element(),
        ]
    }

    /// The settings window, over the app.
    pub fn settings_window(&self, at: Section, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let nav = SECTIONS.iter().map(|&(s, label)| {
            let on = s == at;
            let (bg, hover) = (if on { b.accent.opacity(0.14) } else { transparent_black() }, b.hair);
            div().id(label).h(px(32.)).px(px(8.)).flex().items_center().rounded(SMALL).cursor_pointer().bg(bg).hover(move |st| st.bg(hover))
                .child(text(label, 14., FontWeight::NORMAL, if on { b.ink } else { b.gray }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.settings_open = Some(s);
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
        div().absolute().inset_0().flex().items_center().justify_center()
            .child(div().id("backdrop").absolute().inset_0().bg(black().opacity(0.5)).on_click(cx.listener(|this, _, _, cx| {
                this.settings_open = None;
                cx.notify();
            })))
            .child(div().id("settings-window").relative().w(px(920.)).max_w(relative(0.94)).h(relative(0.86)).max_h(px(680.)).flex().rounded(CARD).bg(b.card).overflow_hidden()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(div().w(px(200.)).flex_none().p(px(16.)).flex().flex_col().gap(px(2.))
                    .child(div().px(px(8.)).pb(px(16.)).flex().items_center().gap(px(8.)).child(icon(Icon::Settings, b.gray).size(px(16.))).child(text("Settings", 15., BOLD, b.ink)))
                    .children(nav))
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
                        .child(div().px(px(24.)).pb(px(24.)).flex().flex_col().gap(px(24.)).children(body)))))
    }
}
