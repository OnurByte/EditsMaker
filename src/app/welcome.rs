//! The first time the app opens: t3code's welcome page, in our look. The AI this computer already has (pick one),
//! the coding agents that can use edits, accounts to post to, where edits go, and Start. Each part can change later
//! in Settings; `edits app welcome` shows it again.
use crate::app::icons::{icon, logo, mark, Icon};
use crate::app::runs::Settings;
use crate::app::theme::{self, brand, Brand, Prefs, BOLD, CARD, LIGHT, SMALL};
use crate::app::ui::{pill, primary, text};
use crate::app::workspace::{Section, Workspace};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// A part of the page: its icon, title and line, then what is done there.
fn part(i: Icon, title: &str, about: &str, body: impl IntoElement, b: &Brand) -> impl IntoElement {
    div().w_full().flex().gap(px(16.)).p(px(24.)).rounded(CARD).bg(b.card)
        .child(div().size(px(40.)).flex_none().flex().items_center().justify_center().rounded(SMALL).bg(b.accent.opacity(0.14)).child(icon(i, b.accent).size(px(20.))))
        .child(div().flex_1().min_w_0().flex().flex_col().gap(px(12.))
            .child(div().flex().flex_col().gap(px(4.)).child(text(title.to_string(), 17., BOLD, b.ink)).child(text(about.to_string(), 14., LIGHT, b.gray)))
            .child(body))
}

impl Workspace {
    fn welcome_ai(&self, s: &Settings, b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let here: Vec<_> = s.providers.iter().filter(|p| p.kind == "subscription" && p.ready).collect();
        if here.is_empty() {
            return div().flex().flex_col().gap(px(8.))
                .child(text("No AI agent on this computer yet. Add one, or use an API key.", 14., FontWeight::NORMAL, b.ink))
                .child(primary("welcome-add-ai", "Add provider", b).on_click(cx.listener(|this, _, _, cx| {
                    this.open_settings(Section::Ai, cx);
                    this.ai_adding = true;
                    cx.notify();
                })))
                .into_any_element();
        }
        div().flex().flex_col().children(here.iter().enumerate().map(|(i, p)| {
            let cmd = vec!["ai".to_string(), p.name.clone()];
            div().py(px(8.)).flex().items_center().gap(px(12.)).when(i > 0, |d| d.border_t_1().border_color(b.hair))
                .child(div().size(px(32.)).flex_none().flex().items_center().justify_center().rounded(SMALL).bg(b.page)
                    .children(logo(&p.name, b.ink, b.page).map(|l| l.size(px(18.)))))
                .child(div().flex_1().min_w_0().flex().flex_col().gap(px(2.))
                    .child(text(p.label.clone(), 14., BOLD, b.ink))
                    .child(text(p.about.clone(), 13., LIGHT, b.gray).truncate()))
                .child(if p.in_use {
                    div().px(px(12.)).h(px(28.)).flex().items_center().rounded(px(14.)).bg(b.accent.opacity(0.16)).child(text("Selected", 12., BOLD, b.accent)).into_any_element()
                } else {
                    pill(("welcome-use", i), None, "Use", false, b).on_click(cx.listener(move |this, _, window, cx| this.change(cmd.clone(), window, cx))).into_any_element()
                })
        })).into_any_element()
    }

    pub fn welcome(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let body: Vec<AnyElement> = match &self.settings {
            None => vec![text("Looking at this computer...", 14., FontWeight::NORMAL, b.gray).into_any_element()],
            Some(Err(e)) => vec![text(e.clone(), 14., FontWeight::NORMAL, b.red_text).into_any_element()],
            Some(Ok(s)) => {
                let out = s.out.clone();
                let sites = div().flex().flex_wrap().gap(px(8.)).children(s.sites.iter().enumerate().map(|(i, site)| {
                    let cmd = vec!["connect".to_string(), site.clone()];
                    let label = match site.as_str() { "tiktok" => "TikTok".to_string(), "youtube" => "YouTube".into(), "instagram" => "Instagram".into(), o => o.to_string() };
                    pill(("welcome-connect", i), Some(Icon::Send), format!("Connect {label}"), s.accounts.iter().any(|a| &a.platform == site), &b)
                        .on_click(cx.listener(move |this, _, window, cx| this.change(cmd.clone(), window, cx)))
                }));
                vec![
                    part(Icon::Sparkles, "Your AI", "It watches the video and picks the moments worth a clip. These are on this computer already.", self.welcome_ai(s, &b, cx), &b).into_any_element(),
                    part(Icon::Terminal, "Your coding agents", "Claude Code, Codex, Cursor and the rest can make edits for you when you ask them.",
                        pill("welcome-agents", Some(Icon::Plus), "Connect them", false, &b).on_click(cx.listener(|this, _, window, cx| {
                            this.change(vec!["mcp".into(), "install".into()], window, cx);
                            this.change(vec!["skills".into(), "install".into()], window, cx);
                        })), &b).into_any_element(),
                    part(Icon::Share, "Where your edits go up", "Optional: sign in once and edits can post for you. Your password stays with the site.", sites, &b).into_any_element(),
                    part(Icon::Folder, "Where your edits are kept", &s.out.display().to_string(),
                        pill("welcome-folder", Some(Icon::Folder), "Open the folder", false, &b).on_click(move |_, _, cx| cx.open_with_system(&out)), &b).into_any_element(),
                ]
            }
        };
        div().id("welcome").size_full().overflow_y_scroll().bg(b.page)
            .child(div().w_full().max_w(px(640.)).mx_auto().px(px(24.)).py(px(48.)).flex().flex_col().items_center().gap(px(24.))
                .child(mark(b.ink).size(px(56.)))
                .child(div().flex().flex_col().items_center().gap(px(8.))
                    .child(text("Welcome to EditsMaker", 32., BOLD, b.ink))
                    .child(text("Link in. Edits out. A few things first; each can change later in Settings.", 15., LIGHT, b.gray).text_center()))
                .children(body)
                .child(div().pt(px(8.)).child(primary("welcome-start", "Start", &b).px(px(32.)).h(px(40.)).rounded(px(20.)).on_click(cx.listener(|this, _, window, cx| {
                    theme::save(&Prefs { onboarded: true, ..theme::prefs() }, window, cx);
                    this.welcome = false;
                    this.input.update(cx, |s, cx| s.focus(window, cx));
                    cx.notify();
                })))))
    }
}
