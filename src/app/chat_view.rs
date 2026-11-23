//! t3code's ChatView: the page header (WorkspacePageHeader) with the thread's project and title, the timeline in one
//! column (MessagesTimeline: your message on the right, then what edits did and what it made), the composer below.
//! With no thread, t3code's NoActiveThreadState: what to do, and the composer.
use crate::app::icons::{icon, Icon};
use crate::app::runs;
use crate::app::theme::{self, brand, Brand, BOLD, HEADER, LIGHT, MONO};
use crate::app::ui::{badge, card, icon_button, text};
use crate::app::workspace::{Route, Workspace};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;
use std::path::PathBuf;
use std::time::Duration;

fn took(d: Duration) -> String {
    let s = d.as_secs();
    if s < 60 { format!("{s}s") } else { format!("{}m {}s", s / 60, s % 60) }
}

impl Workspace {
    fn header(&self, subject: &str, title: String, folder: Option<PathBuf>, b: &Brand) -> impl IntoElement {
        div().h(HEADER).flex_none().px(px(24.)).flex().items_center().gap(px(8.)).border_b_1().border_color(b.hair)
            .child(badge(subject, b))
            .child(text(subject.to_string(), 14., FontWeight::NORMAL, b.gray).flex_none())
            .child(text("/", 14., LIGHT, b.gray))
            .child(text(title, 14., BOLD, b.ink).flex_1().min_w_0().truncate())
            .children(folder.map(|f| icon_button("folder", Icon::Folder, b).on_click(move |_, _, cx| cx.open_with_system(&f))))
    }

    /// The clips (and packs) a run made, each to open, review or find in its folder.
    fn made(&self, paths: Vec<PathBuf>, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let n = paths.len();
        let rows = paths.into_iter().enumerate().map(|(i, p)| {
            let (open, show, review) = (p.clone(), p.clone(), p.clone());
            let clip = p.extension().is_some_and(|e| e == "mp4");
            div().h(px(40.)).flex().items_center().gap(px(8.)).when(i > 0, |d| d.border_t_1().border_color(b.hair))
                .child(icon(if p.is_dir() { Icon::Package } else { Icon::Play }, b.gray).size(px(16.)))
                .child(text(p.file_name().unwrap_or_default().to_string_lossy().into_owned(), 14., FontWeight::NORMAL, b.ink).flex_1().min_w_0().truncate())
                .when(clip, |d| d.child(icon_button(("review", i), Icon::Eye, b).on_click(cx.listener(move |this, _, window, cx| this.open_review(review.clone(), window, cx)))))
                .child(icon_button(("open", i), Icon::Play, b).on_click(move |_, _, cx| cx.open_with_system(&open)))
                .child(icon_button(("show", i), Icon::Folder, b).on_click(move |_, _, cx| cx.reveal_path(&show)))
        });
        card(b).gap(px(8.))
            .child(text(match n { 0 => "Nothing made yet".into(), 1 => "1 thing made".into(), n => format!("{n} things made") }, 14., BOLD, b.ink))
            .children(rows)
    }

    fn timeline(&self, cx: &mut Context<Self>) -> (String, String, Option<PathBuf>, Vec<AnyElement>) {
        let b = brand(cx);
        match self.route {
            Route::Job(i) => {
                let job = &self.jobs[i];
                let lines = job.lines();
                let paths = runs::paths_in(&lines);
                let folder = paths.first().and_then(|p| p.parent().map(PathBuf::from));
                let (state, said) = match (job.finished(), job.took) {
                    (None, _) => (Icon::Running, format!("Working for {}", took(job.started.elapsed()))),
                    (Some(ok), t) => (if ok { Icon::Done } else { Icon::Failed }, format!("{} in {}", if ok { "Worked" } else { "Stopped" }, took(t.unwrap_or_default()))),
                };
                let shown = if self.show_log { &lines[..] } else { &lines[lines.len().saturating_sub(8)..] };
                let log = card(&b).gap(px(4.))
                    .children(shown.iter().map(|l| text(l.clone(), 12., FontWeight::NORMAL, b.gray).font_family(MONO).truncate()));
                let items = vec![
                    // your message, on the right as in t3code
                    div().flex().justify_end().child(div().max_w(relative(0.8)).bg(b.card).rounded(px(20.)).px(px(16.)).py(px(8.))
                        .child(text(job.title(), 15., LIGHT, b.ink))).into_any_element(),
                    div().id("worked").flex().items_center().gap(px(8.)).cursor_pointer()
                        .child(icon(state, b.gray).size(px(16.)))
                        .child(text(said, 14., FontWeight::NORMAL, b.gray))
                        .child(text(if self.show_log { "Hide" } else { "Show all" }, 14., FontWeight::NORMAL, b.gray).underline())
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.show_log = !this.show_log;
                            cx.notify();
                        })).into_any_element(),
                    log.into_any_element(),
                    self.made(paths, &b, cx).into_any_element(),
                ];
                ("This session".into(), job.title(), folder, items)
            }
            Route::Run(i) => {
                let run = &self.past[i];
                let items = vec![
                    text(format!("Made {}", run.when), 14., FontWeight::NORMAL, b.gray).into_any_element(),
                    self.made(runs::files(&run.dir), &b, cx).into_any_element(),
                ];
                (run.subject.clone(), run.when.clone(), Some(run.dir.clone()), items)
            }
            _ => Default::default(),
        }
    }

    pub fn chat_view(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let composer = div().flex_none().px(px(24.)).pb(px(24.)).child(self.composer(window, cx));
        if self.route == Route::New {
            return div().size_full().flex().flex_col()
                .child(div().flex_1().flex().flex_col().items_center().justify_center().gap(px(16.)).px(px(24.))
                    .child(crate::app::icons::mark(b.ink).size(px(48.)))
                    .child(text("Link in. Edits out.", 32., BOLD, b.ink))
                    .child(text("A YouTube video or channel, a topic, or a video file: drop one anywhere here.", 15., LIGHT, b.gray).max_w(px(420.)).text_center()))
                .child(composer);
        }
        let (subject, title, folder, items) = self.timeline(cx);
        div().size_full().flex().flex_col()
            .child(self.header(&subject, title, folder, &b))
            .child(div().id("timeline").flex_1().min_h_0().overflow_y_scroll()
                .child(div().w_full().max_w(theme::column()).mx_auto().px(px(24.)).py(px(24.)).flex().flex_col().gap(px(24.)).children(items)))
            .child(composer)
    }
}
