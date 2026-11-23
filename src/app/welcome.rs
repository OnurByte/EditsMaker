//! The first time the app opens: a welcome in small steps, one thing at a time, each coming in with a little motion
//! and its own illustration: hello, the AI this computer has (pick one), the coding agents that can use edits,
//! accounts to post to, where edits go, and ready. Each can change later in Settings; `edits app welcome` shows it
//! again. Discord is not asked about here: it is simply on, and Settings turns it off.
use crate::app::icons::{icon, illustration, logo, Icon};
use crate::app::runs::Settings;
use crate::app::theme::{self, brand, Brand, Prefs, BOLD, LIGHT, SMALL};
use crate::app::ui::{pill, primary, text};
use crate::app::workspace::{Section, Workspace};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::time::Duration;

/// Each step: its illustration, its line, and what it says under it.
pub const STEPS: &[(&str, &str, &str)] = &[
    ("hello", "Let's make something people feel.", "EditsMaker turns the moments you love into edits that stop the scroll. A few small steps, and it is yours."),
    ("brain", "First, a mind to watch with you", "Your AI watches the video and finds the moments worth a clip. These are already on this computer."),
    ("agents", "Your agents can edit too", "Claude Code, Codex, Cursor and the rest can make edits for you, whenever you ask them."),
    ("accounts", "Where your edits go up", "Optional: sign in once and edits can post for you. Your password stays with the site."),
    ("folder", "A home for everything you make", "Every edit is kept here, and in the Library, ready to watch and share."),
    ("ready", "You're ready.", "Paste a link, name a topic, or drop a video. We'll take it from there."),
];

impl Workspace {
    fn welcome_ai(&self, s: &Settings, b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let here: Vec<_> = s.providers.iter().filter(|p| p.kind == "subscription" && p.ready).collect();
        if here.is_empty() {
            return div().flex().flex_col().items_center().gap(px(12.))
                .child(text("No AI agent on this computer yet. Add one, or use an API key.", 14., FontWeight::NORMAL, b.ink))
                .child(primary("welcome-add-ai", "Add provider", b).on_click(cx.listener(|this, _, _, cx| {
                    this.open_settings(Section::Ai, cx);
                    this.ai_adding = true;
                    cx.notify();
                })))
                .into_any_element();
        }
        div().w_full().flex().flex_col().rounded(px(16.)).bg(b.card).px(px(16.)).children(here.iter().enumerate().map(|(i, p)| {
            let cmd = vec!["ai".to_string(), p.name.clone()];
            let who = [p.plan.as_str(), p.account.as_str()].into_iter().filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ");
            div().py(px(12.)).flex().items_center().gap(px(12.)).when(i > 0, |d| d.border_t_1().border_color(b.hair))
                .child(div().size(px(36.)).flex_none().flex().items_center().justify_center().rounded(SMALL).bg(b.page)
                    .children(logo(&p.name, b.ink, b.page).map(|l| l.size(px(18.)))))
                .child(div().flex_1().min_w_0().flex().flex_col().gap(px(2.))
                    .child(text(p.label.clone(), 14., BOLD, b.ink))
                    .child(text(if who.is_empty() { p.about.clone() } else { who }, 13., LIGHT, b.gray).truncate()))
                .child(if p.in_use {
                    div().px(px(12.)).h(px(28.)).flex().items_center().gap(px(6.)).rounded(px(14.)).bg(b.accent.opacity(0.16))
                        .child(icon(Icon::Check, b.accent).size(px(14.))).child(text("Selected", 12., BOLD, b.accent)).into_any_element()
                } else {
                    pill(("welcome-use", i), None, "Use", false, b).on_click(cx.listener(move |this, _, window, cx| this.change(cmd.clone(), window, cx))).into_any_element()
                })
        })).into_any_element()
    }

    /// What a step asks for, under its words.
    fn welcome_body(&self, step: usize, b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let s = match &self.settings {
            Some(Ok(s)) => s,
            Some(Err(e)) => return text(e.clone(), 14., FontWeight::NORMAL, b.red_text).into_any_element(),
            None if step == 0 || step == 5 => return div().into_any_element(),
            None => return text("Looking at this computer...", 14., FontWeight::NORMAL, b.gray).into_any_element(),
        };
        match step {
            1 => self.welcome_ai(s, b, cx),
            2 => pill("welcome-agents", Some(Icon::Terminal), if self.changing.as_deref().is_some_and(|c| c.starts_with("mcp") || c.starts_with("skills")) { "Connecting..." } else { "Connect them" }, false, b)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.change(vec!["mcp".into(), "install".into()], window, cx);
                    this.change(vec!["skills".into(), "install".into()], window, cx);
                })).into_any_element(),
            3 => div().flex().flex_wrap().justify_center().gap(px(8.)).children(s.sites.iter().enumerate().map(|(i, site)| {
                let cmd = vec!["connect".to_string(), site.clone()];
                let label = match site.as_str() { "tiktok" => "TikTok".to_string(), "youtube" => "YouTube".into(), "instagram" => "Instagram".into(), o => o.to_string() };
                let on = s.accounts.iter().any(|a| &a.platform == site);
                pill(("welcome-connect", i), Some(if on { Icon::Check } else { Icon::Send }), if on { format!("{label} connected") } else { format!("Connect {label}") }, on, b)
                    .on_click(cx.listener(move |this, _, window, cx| this.change(cmd.clone(), window, cx)))
            })).into_any_element(),
            4 => {
                let out = s.out.clone();
                div().flex().flex_col().items_center().gap(px(12.))
                    .child(text(s.out.display().to_string(), 13., FontWeight::NORMAL, b.gray).font_family(theme::MONO))
                    .child(pill("welcome-folder", Some(Icon::Folder), "Open the folder", false, b).on_click(move |_, _, cx| cx.open_with_system(&out)))
                    .into_any_element()
            }
            _ => div().into_any_element(),
        }
    }

    fn finish_welcome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        theme::save(&Prefs { onboarded: true, ..theme::prefs() }, window, cx);
        self.welcome = false;
        self.welcome_step = 0;
        self.input.update(cx, |s, cx| s.focus(window, cx));
        cx.notify();
    }

    pub fn welcome(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let step = self.welcome_step.min(STEPS.len() - 1);
        let (art, title, line) = STEPS[step];
        let last = step == STEPS.len() - 1;
        // the steps so far, filling in as they are passed
        let progress = div().flex().gap(px(6.)).children((0..STEPS.len()).map(|i| {
            let bar = div().h(px(4.)).w(px(40.)).rounded(px(2.)).bg(if i <= step { b.accent } else { b.hair });
            if i == step {
                bar.with_animation(("progress", step), Animation::new(Duration::from_millis(500)).with_easing(ease_out_quint()), |d, t| d.opacity(0.3 + 0.7 * t)).into_any_element()
            } else {
                bar.into_any_element()
            }
        }));
        // the illustration comes up and in, then breathes
        let picture = div().child(div().child(illustration(art).w(px(280.)).h(px(210.)))
                .with_animation(("float", step), Animation::new(Duration::from_millis(3200)).repeat().with_easing(bounce(ease_in_out)), |d, t| d.mt(px(-6. * t)).mb(px(6. * t))))
            .with_animation(("art", step), Animation::new(Duration::from_millis(700)).with_easing(ease_out_quint()), |d, t| d.opacity(t).mt(px(32. * (1. - t))));
        let words = div().flex().flex_col().items_center().gap(px(12.))
            .child(text(title, 32., BOLD, b.ink).text_center().max_w(px(600.)))
            .child(text(line, 16., LIGHT, b.gray).text_center().max_w(px(480.)))
            .with_animation(("words", step), Animation::new(Duration::from_millis(900)).with_easing(ease_out_quint()), |d, t| {
                // a beat after the picture
                let t = ((t - 0.2) / 0.8).clamp(0., 1.);
                d.opacity(t).mt(px(16. * (1. - t)))
            });
        let body = div().w_full().flex().justify_center().child(self.welcome_body(step, &b, cx))
            .with_animation(("body", step), Animation::new(Duration::from_millis(1100)).with_easing(ease_out_quint()), |d, t| d.opacity(((t - 0.4) / 0.6).clamp(0., 1.)));
        let back = pill("welcome-back", None, "Back", false, &b).when(step == 0, |d| d.invisible()).on_click(cx.listener(|this, _, _, cx| {
            this.welcome_step = this.welcome_step.saturating_sub(1);
            cx.notify();
        }));
        let next = primary("welcome-next", if step == 0 { "Let's begin" } else if last { "Start making" } else { "Continue" }, &b).h(px(40.)).px(px(28.)).rounded(px(20.))
            .on_click(cx.listener(move |this, _, window, cx| {
                if last {
                    this.finish_welcome(window, cx);
                } else {
                    this.welcome_step += 1;
                    cx.notify();
                }
            }));
        let skip = pill("welcome-skip", None, if (2..=3).contains(&step) { "Not now" } else { "Skip all" }, false, &b).when(last, |d| d.invisible())
            .on_click(cx.listener(move |this, _, window, cx| {
                if (2..=3).contains(&this.welcome_step) {
                    this.welcome_step += 1;
                    cx.notify();
                } else {
                    this.finish_welcome(window, cx);
                }
            }));
        div().id("welcome").size_full().overflow_y_scroll().bg(b.page).flex().flex_col()
            .child(div().pt(px(32.)).flex().justify_center().child(progress))
            .child(div().flex_1().w_full().max_w(px(640.)).mx_auto().px(px(24.)).py(px(24.)).flex().flex_col().items_center().justify_center().gap(px(24.))
                .child(picture)
                .child(words)
                .child(body))
            .child(div().w_full().max_w(px(640.)).mx_auto().px(px(24.)).pb(px(40.)).flex().items_center().gap(px(8.))
                .child(back).child(div().flex_1()).child(skip).child(next))
    }
}
