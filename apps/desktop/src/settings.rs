//! t3code's settings, ported: a list of sections on the left (SettingsSidebarNav), the section's groups on the right
//! (SettingsSection, SettingsGroup: a card whose rows are split by hairlines, SettingsRow: what it is and why on the
//! left, its control on the right). Each change runs the edits command that owns it, so the CLI and the app agree.
use crate::icons::{icon, Icon};
use crate::runs::Settings;
use crate::theme::{self, brand, Brand, BOLD, HEADER, LIGHT, MONO, SMALL};
use crate::ui::{card, pill, primary, text};
use crate::workspace::{Route, Section, Workspace};
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
        .child(div().flex_none().flex().flex_wrap().justify_end().max_w(px(420.)).gap(px(8.)).child(control))
}

/// A SettingsGroup: rows on one card.
fn group(rows: Vec<AnyElement>, b: &Brand) -> impl IntoElement {
    card(b).p(px(0.)).children(rows)
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

    fn section(&self, section: Section, s: &Settings, b: &Brand, window: &mut Window, cx: &mut Context<Self>) -> Vec<AnyElement> {
        match section {
            Section::General => {
                let out = s.out.clone();
                let updates = if s.updates { "on" } else { "off" };
                vec![group(vec![
                    row("Where edits go", s.out.display().to_string(), pill("out", Some(Icon::Folder), "Open", false, b).on_click(move |_, _, cx| cx.open_with_system(&out)), true, b).into_any_element(),
                    row("What renders the video", "auto uses the graphics card when it works, else the processor.", self.choice("encoder", &s.encoders, &s.encoder, &["encoder"], b, cx), false, b).into_any_element(),
                    row("Updates", "Once a day, the newest release before a run.", self.choice("updates", &["on".into(), "off".into()], updates, &["update"], b, cx), false, b).into_any_element(),
                ], b).into_any_element()]
            }
            Section::Ai => vec![group(vec![
                row("The AI that picks moments", format!("Now: {}{}", s.ai.about, if s.ai.ready { "" } else { ". Not ready: its command or key is missing." }),
                    self.choice("ai", &s.providers, &s.ai.provider, &["ai"], b, cx), true, b).into_any_element(),
                row("API keys", "A key is typed where no one sees it: edits ai key PROVIDER, in a terminal.", div(), false, b).into_any_element(),
            ], b).into_any_element()],
            Section::Accounts => {
                let mut rows: Vec<AnyElement> = s.accounts.iter().enumerate().map(|(i, a)| {
                    row(&a.name, format!("{} · {}", a.platform, a.user), self.button(("disconnect", i), "Disconnect", &["disconnect", &a.name], b, cx), i == 0, b).into_any_element()
                }).collect();
                let first = rows.is_empty();
                let sites = div().flex().gap(px(8.)).children(s.sites.iter().enumerate().map(|(i, site)| self.button(("connect", i), site, &["connect", site], b, cx)));
                rows.push(row("Connect an account", "Signs in once in a window of its own; edits never sees the password. Bluesky and Facebook take a token: edits connect, in a terminal.", sites, first, b).into_any_element());
                vec![group(rows, b).into_any_element()]
            }
            Section::Agents => vec![group(vec![
                row("MCP server", "edits as tools in Claude Code, ChatGPT, Cursor and every other agent on this computer.", self.button("mcp", "Install", &["mcp", "install"], b, cx), true, b).into_any_element(),
                row("Skills", "The packs' agent skills in each agent's skills folder.", self.button("skills", "Install", &["skills", "install"], b, cx), false, b).into_any_element(),
            ], b).into_any_element()],
            Section::Appearance => {
                let now = theme::pref();
                let pills = div().flex().gap(px(8.)).children(["system", "light", "dark"].into_iter().map(|p| {
                    pill(p, None, p, now == p, b).on_click(cx.listener(move |_, _, window, cx| theme::set_pref(p, window, cx)))
                }));
                let _ = window;
                vec![group(vec![row("Theme", "Light or dark, or as the system is.", pills, true, b).into_any_element()], b).into_any_element()]
            }
            Section::About => {
                let doctor: AnyElement = match &self.doctor {
                    Some(d) => card(b).gap(px(4.)).children(d.lines().map(|l| text(l.to_string(), 12., FontWeight::NORMAL, b.gray).font_family(MONO))).into_any_element(),
                    None => div().into_any_element(),
                };
                vec![
                    group(vec![
                        row("EditsMaker", format!("Version {}", s.version), pill("site", None, "GitHub", false, b).on_click(|_, _, cx| cx.open_url("https://github.com/OnurByte/EditsMaker")), true, b).into_any_element(),
                        row("Check this computer", "ffmpeg, yt-dlp, whisper and the AI, as edits doctor sees them.", primary("doctor", "Check", b).on_click(cx.listener(|this, _, _, cx| this.check(cx))), false, b).into_any_element(),
                    ], b).into_any_element(),
                    doctor,
                ]
            }
        }
    }

    pub fn settings_page(&self, section: Section, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let nav = SECTIONS.iter().map(|&(s, label)| {
            let on = s == section;
            let (bg, hover) = (if on { b.card } else { transparent_black() }, b.card);
            div().id(label).h(px(32.)).px(px(8.)).flex().items_center().rounded(SMALL).cursor_pointer().bg(bg).hover(move |st| st.bg(hover))
                .child(text(label, 14., FontWeight::NORMAL, if on { b.ink } else { b.gray }))
                .on_click(cx.listener(move |this, _, _, cx| this.go(Route::Settings(s), cx)))
                .into_any_element()
        }).collect::<Vec<_>>();
        let title = SECTIONS.iter().find(|x| x.0 == section).map_or("", |x| x.1);
        let body: Vec<AnyElement> = match &self.settings {
            None => vec![text("Reading the settings...", 14., FontWeight::NORMAL, b.gray).into_any_element()],
            Some(Err(e)) => vec![text(e.clone(), 14., FontWeight::NORMAL, b.red_text).into_any_element()],
            Some(Ok(s)) => self.section(section, s, &b, window, cx),
        };
        div().size_full().flex().flex_col()
            .child(div().h(HEADER).flex_none().px(px(24.)).flex().items_center().gap(px(8.)).border_b_1().border_color(b.hair)
                .child(icon(Icon::Settings, b.gray).size(px(16.)))
                .child(text("Settings", 14., FontWeight::NORMAL, b.gray))
                .child(text("/", 14., LIGHT, b.gray))
                .child(text(title, 14., BOLD, b.ink).flex_1())
                .children(self.changing.as_ref().map(|c| text(format!("edits {c}..."), 13., FontWeight::NORMAL, b.gray))))
            .child(div().flex_1().min_h_0().flex()
                .child(div().w(px(200.)).flex_none().p(px(16.)).flex().flex_col().gap(px(2.)).children(nav))
                .child(div().id("settings").flex_1().min_w_0().overflow_y_scroll()
                    .child(div().w_full().max_w(px(720.)).p(px(24.)).flex().flex_col().gap(px(16.))
                        .child(text(title, 19., BOLD, b.ink))
                        .children(body))))
    }
}
