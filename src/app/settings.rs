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

/// The sections, their icons, and the words search finds them by (what their rows are about).
const SECTIONS: &[(Section, &str, Icon, &str)] = &[
    (Section::General, "General", Icon::Settings, "folder where edits go output videos encoder gpu graphics card render updates"),
    (Section::Ai, "AI", Icon::Sparkles, "ai provider model claude codex cursor grok gemini antigravity kiro opencode pi acp openai anthropic openrouter ollama key api"),
    (Section::Accounts, "Accounts", Icon::User, "accounts connect sign in tiktok youtube instagram bluesky facebook post"),
    (Section::Agents, "Agents", Icon::Terminal, "agents mcp server skills claude code chatgpt cursor tools"),
    (Section::Appearance, "Appearance", Icon::Palette, "appearance theme dark light system omarchy accent colour color chat width text size font"),
    (Section::About, "About", Icon::Info, "about version github check doctor ffmpeg yt-dlp whisper"),
];

/// The sections a search finds, all of them for none.
pub fn found(q: &str) -> Vec<Section> {
    let q = q.trim().to_lowercase();
    SECTIONS.iter().filter(|s| q.is_empty() || s.1.to_lowercase().contains(&q) || q.split_whitespace().all(|w| s.3.contains(w))).map(|s| s.0).collect()
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

    /// One of the providers here, as t3code's ProviderInstanceCard: its logo, name and line, its status, Select; open,
    /// what it runs, its key or server, its models.
    fn provider_card(&self, i: usize, p: &Provider, s: &Settings, b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let open = self.ai_open.as_deref() == Some(p.name.as_str());
        let (name, kind_, base) = (p.name.clone(), p.kind.clone(), p.base.clone());
        let badge = if p.in_use { ("Selected", b.accent) } else { ("Ready", b.gray) };
        let select: AnyElement = if p.in_use {
            div().into_any_element()
        } else if p.kind == "subscription" || p.kind == "acp" || p.kind == "local" {
            let mut cmd = vec!["ai".to_string(), p.name.clone()];
            if p.kind != "subscription" {
                cmd.extend(["--endpoint".to_string(), if p.kind == "acp" { p.command.clone() } else { p.base.clone() }]);
            }
            pill(("select", i), None, "Select", false, b).on_click(cx.listener(move |this, _, window, cx| this.change(cmd.clone(), window, cx))).into_any_element()
        } else {
            text("Pick a model to select", 12., FontWeight::NORMAL, b.gray).into_any_element()
        };
        let hover = b.hair;
        let head = div().id(("provider", i)).px(px(16.)).py(px(16.)).flex().items_center().gap(px(16.)).cursor_pointer().hover(move |st| st.bg(hover))
            .child(div().size(px(32.)).flex_none().flex().items_center().justify_center().rounded(SMALL).bg(b.card)
                .children(crate::app::icons::logo(&p.name, b.ink, b.card).map(|l| l.size(px(18.)))))
            .child(div().flex_1().min_w_0().flex().flex_col().gap(px(2.))
                .child(div().flex().items_center().gap(px(8.))
                    .child(text(p.label.clone(), 14., BOLD, b.ink))
                    .child(div().px(px(8.)).h(px(20.)).flex().items_center().rounded(px(10.)).bg(badge.1.opacity(0.16)).child(text(badge.0, 11., BOLD, badge.1))))
                .child(text(p.about.clone(), 13., LIGHT, b.gray).truncate()))
            .child(select)
            .child(text(if open { "−" } else { "+" }, 18., LIGHT, b.gray))
            .on_click(cx.listener(move |this, _, window, cx| {
                let opening = this.ai_open.as_deref() != Some(name.as_str());
                this.ai_open = opening.then(|| name.clone());
                this.ai_models = None;
                if opening && (kind_ == "local" || kind_ == "acp") {
                    this.ai_endpoint.update(cx, |s, cx| s.set_value(base.clone(), window, cx));
                }
                if opening && (kind_ == "api" || kind_ == "local") {
                    this.load_models(name.clone(), if kind_ == "local" { base.clone() } else { String::new() }, cx);
                }
                cx.notify();
            }));
        div().flex().flex_col().when(i > 0, |d| d.border_t_1().border_color(b.hair)).child(head)
            .when(open, |d| d.child(div().pb(px(8.)).flex().flex_col().children(self.provider_setup(p, s, b, cx))))
            .into_any_element()
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
            .child(div().id("add-backdrop").absolute().inset_0().bg(black().opacity(0.4)).on_click(cx.listener(|this, _, _, cx| {
                this.ai_adding = false;
                cx.notify();
            })))
            .child(div().id("add-provider").relative().w(relative(0.9)).max_h(relative(0.9)).p(px(24.)).flex().flex_col().gap(px(16.)).rounded(CARD).bg(b.card).overflow_y_scroll()
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
                let updates = if s.updates { "on" } else { "off" };
                vec![group("", vec![
                    row("Where edits go", s.out.display().to_string(), pill("out", Some(Icon::Folder), "Open", false, b).on_click(move |_, _, cx| cx.open_with_system(&out)), true, b).into_any_element(),
                    row("What renders the video", "auto uses the graphics card when it works, else the processor.", self.choice("encoder", &s.encoders, &s.encoder, &["encoder"], b, cx), false, b).into_any_element(),
                    row("Updates", "Once a day, the newest release before a run.", self.choice("updates", &["on".into(), "off".into()], updates, &["update"], b, cx), false, b).into_any_element(),
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
        let shown = found(&self.settings_search.read(cx).value());
        let nav = SECTIONS.iter().filter(|x| shown.contains(&x.0)).map(|&(s, label, i, _)| {
            let on = s == at;
            let (bg, hover) = (if on { b.accent.opacity(0.14) } else { transparent_black() }, b.hair);
            div().id(label).h(px(32.)).px(px(8.)).flex().items_center().gap(px(8.)).rounded(SMALL).cursor_pointer().bg(bg).hover(move |st| st.bg(hover))
                .child(icon(i, if on { b.accent } else { b.gray }).size(px(16.)))
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
        let add = match (&self.settings, self.ai_adding && at == Section::Ai) {
            (Some(Ok(s)), true) => Some(self.add_provider(s, &b, cx)),
            _ => None,
        };
        div().absolute().inset_0().flex().items_center().justify_center()
            .child(div().id("backdrop").absolute().inset_0().bg(black().opacity(0.5)).on_click(cx.listener(|this, _, _, cx| {
                this.settings_open = None;
                cx.notify();
            })))
            .child(div().id("settings-window").relative().w(px(920.)).max_w(relative(0.94)).h(relative(0.86)).max_h(px(680.)).flex().rounded(CARD).bg(b.card).overflow_hidden()
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
        assert_eq!(found("").len(), 6);
        assert!(found("zzz").is_empty());
    }
}
