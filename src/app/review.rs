//! The editor: the clip playing (player.rs) with its shots marked on the timeline, its frames in a strip; trim it by
//! hand (set in and out, Cut: `edits cut`), or point at a spot (a 3 x 3 grid), say what to change there, add a word on
//! the whole edit, and the notes go beside the clip for the agent (`edits review`).
use crate::app::icons::{icon, Icon};
use crate::app::player::clock;
use crate::app::runs::{self, Note};
use crate::app::theme::{brand, Brand, BOLD, HEADER, LIGHT, SMALL};
use crate::app::ui::{card, pill, primary, text};
use crate::app::workspace::{Route, Workspace};
use gpui_kit::component::input::Input;
use gpui_kit::component::notification::Notification;
use gpui_kit::component::WindowExt;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

/// Which shot a moment is in (from 1), counting the cuts before it.
pub fn shot(t: f64, cuts: &[f64]) -> usize {
    1 + cuts.iter().filter(|c| **c > 0.05 && **c <= t + 0.01).count()
}

impl Workspace {
    pub fn add_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(r) = self.review.as_mut() else { return };
        let text = r.note.read(cx).value().trim().to_string();
        let t = match &r.player {
            Some(p) => p.position(),
            None => r.sheet.as_ref().and_then(|s| s.as_ref().ok()).and_then(|s| s.frames.get(r.at)).map_or(0.0, |f| f.t),
        };
        if text.is_empty() {
            return;
        }
        r.notes.push(Note { t, at: r.spot.take(), text });
        r.note.update(cx, |s, cx| s.set_value("", window, cx));
        cx.notify();
    }

    fn send_notes(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(r) = self.review.as_ref() else { return };
        let Some(Ok(sheet)) = r.sheet.as_ref() else { return };
        let general = r.general.read(cx).value().trim().to_string();
        match runs::save_notes(sheet, &r.notes, &general) {
            Ok(()) if self.review_only => cx.quit(),
            Ok(()) => {
                window.push_notification(Notification::success(format!("{} notes on {}", r.notes.len(), sheet.name)).title("Saved"), cx);
                self.review = None;
                self.go(Route::New, cx);
            }
            Err(e) => window.push_notification(Notification::error(e.to_string()), cx),
        }
    }

    /// The player's controls: play, the time, the timeline (its shots, the trim, the playhead), sound, trim by hand.
    fn controls(&self, cuts: &[f64], b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let Some(p) = self.review.as_ref().and_then(|r| r.player.as_ref()) else {
            return text("Opening the player...", 13., FontWeight::NORMAL, b.gray).into_any_element();
        };
        let (total, now) = (p.total.max(0.01), p.position());
        const STEPS: usize = 240;
        let (mark_in, mark_out) = (p.mark_in, p.mark_out);
        let (hover, accent, ink, hair) = (b.accent.opacity(0.5), b.accent, b.ink, b.hair);
        // the timeline as many thin steps, each one a place to seek to
        let steps = (0..STEPS).map(|i| {
            let (a, z) = (total * i as f64 / STEPS as f64, total * (i + 1) as f64 / STEPS as f64);
            let played = a < now;
            let trimmed = mark_in.is_some_and(|x| a >= x) && mark_out.is_none_or(|x| a < x) || mark_out.is_some_and(|x| a < x) && mark_in.is_none();
            let cut = cuts.iter().any(|c| *c > 0.05 && *c >= a && *c < z);
            div().id(("seek", i)).relative().flex_1().h_full().flex().items_center().cursor_pointer().hover(move |s| s.bg(hover))
                .child(div().w_full().h(px(6.)).bg(if played { accent } else { hair }).when(trimmed, |d| d.h(px(14.)).opacity(0.85)))
                .when(cut, |d| d.child(div().absolute().w(px(2.)).h(px(18.)).bg(ink)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(p) = this.review.as_mut().and_then(|r| r.player.as_mut()) {
                        p.seek(a);
                    }
                    cx.notify();
                }))
        });
        let playing = p.playing();
        let set = |id: &'static str, label: &'static str, f: fn(&mut crate::app::player::Player), b: &Brand, cx: &mut Context<Self>| {
            pill(id, None, label, false, b).on_click(cx.listener(move |this, _, _, cx| {
                if let Some(p) = this.review.as_mut().and_then(|r| r.player.as_mut()) {
                    f(p);
                }
                cx.notify();
            }))
        };
        let span = match (mark_in, mark_out) {
            (Some(a), Some(z)) if z > a + 0.05 => Some((a, z)),
            _ => None,
        };
        div().flex().flex_col().gap(px(8.))
            .child(div().id("timeline").h(px(24.)).flex().items_center().children(steps))
            .child(div().flex().items_center().gap(px(8.))
                .child(crate::app::ui::icon_button("play", if playing { Icon::Pause } else { Icon::Play }, b)
                    .tooltip(|w, cx| gpui_kit::component::tooltip::Tooltip::new("Play or pause").build(w, cx))
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(p) = this.review.as_mut().and_then(|r| r.player.as_mut()) {
                            p.toggle();
                        }
                        cx.notify();
                    })))
                .child(text(format!("{} / {} · shot {} of {}", clock(now), clock(p.total), shot(now, cuts), shot(p.total, cuts)), 13., FontWeight::NORMAL, b.gray).font_family(crate::app::theme::MONO))
                .child(crate::app::ui::icon_button("mute", if p.muted { Icon::Mute } else { Icon::Volume }, b)
                    .on_click(cx.listener(|this, _, _, cx| {
                        if let Some(p) = this.review.as_mut().and_then(|r| r.player.as_mut()) {
                            let m = !p.muted;
                            p.set_muted(m);
                        }
                        cx.notify();
                    })))
                .child(div().flex_1())
                .child(set("mark-in", "Set in", |p| p.mark_in = Some(p.position()), b, cx))
                .child(set("mark-out", "Set out", |p| p.mark_out = Some(p.position()), b, cx))
                .when(mark_in.is_some() || mark_out.is_some(), |d| d.child(set("mark-clear", "Clear", |p| (p.mark_in, p.mark_out) = (None, None), b, cx)))
                .children(span.map(|(a, z)| {
                    let clip = p.clip.display().to_string();
                    primary("cut", format!("Cut {}-{}", clock(a), clock(z)), b).on_click(cx.listener(move |this, _, window, cx| {
                        this.launch(vec!["cut".into(), clip.clone(), format!("{a:.2}-{z:.2}")], window, cx);
                    }))
                })))
            .into_any_element()
    }

    fn frame_view(&self, pic: Option<std::sync::Arc<RenderImage>>, b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let Some(r) = self.review.as_ref() else { return div().into_any_element() };
        let sheet = match &r.sheet {
            Some(Ok(s)) => Some(s),
            Some(Err(e)) if pic.is_none() => return text(e.clone(), 14., FontWeight::NORMAL, b.red_text).p(px(24.)).into_any_element(),
            _ => None,
        };
        let cuts: Vec<f64> = sheet.map(|s| s.cuts.clone()).unwrap_or_default();
        // nine places to point at, over the picture
        let cells = (0..9usize).map(|k| {
            let (x, y) = ((k % 3) as f64 / 3.0 + 1.0 / 6.0, (k / 3) as f64 / 3.0 + 1.0 / 6.0);
            let on = r.spot == Some((x, y));
            let ink = b.ink;
            div().id(("spot", k)).w(relative(1. / 3.)).h(relative(1. / 3.)).flex().items_center().justify_center().cursor_pointer()
                .hover(move |s| s.bg(ink.opacity(0.08)))
                .when(on, |d| d.child(div().size(px(24.)).rounded_full().border_2().border_color(ink)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(r) = this.review.as_mut() {
                        r.spot = if r.spot == Some((x, y)) { None } else { Some((x, y)) };
                    }
                    cx.notify();
                }))
        });
        let picture: AnyElement = match (&pic, sheet.and_then(|s| s.frames.get(r.at))) {
            (Some(img_), _) => img(ImageSource::Render(img_.clone())).size_full().object_fit(ObjectFit::Contain).into_any_element(),
            (None, Some(f)) => img(f.file.clone()).size_full().object_fit(ObjectFit::Contain).into_any_element(),
            (None, None) => text("Opening the clip...", 14., FontWeight::NORMAL, b.gray).p(px(24.)).into_any_element(),
        };
        let now = r.player.as_ref().map(|p| p.position());
        let strip = sheet.map(|sheet| sheet.frames.iter().enumerate().map(|(i, f)| {
            // the frame the playhead is in
            let on = match now {
                Some(t) => t >= f.t && sheet.frames.get(i + 1).is_none_or(|n| t < n.t),
                None => i == r.at,
            };
            let cut = i > 0 && shot(f.t, &sheet.cuts) != shot(sheet.frames[i - 1].t, &sheet.cuts);
            let t = f.t;
            div().id(("frame", i)).flex_none().h(px(72.)).w(px(48.)).rounded(SMALL).overflow_hidden().cursor_pointer()
                .when(cut, |d| d.ml(px(8.)))
                .border_2().border_color(if on { b.accent } else { transparent_black() })
                .child(img(f.file.clone()).size_full().object_fit(ObjectFit::Cover))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(r) = this.review.as_mut() {
                        r.at = i;
                        if let Some(p) = r.player.as_mut() {
                            p.seek(t);
                        }
                    }
                    cx.notify();
                }))
                .into_any_element()
        }).collect::<Vec<_>>()).unwrap_or_default();
        div().flex_1().min_w_0().flex().flex_col().gap(px(16.)).p(px(24.))
            .child(div().flex_1().min_h_0().relative().rounded(SMALL).overflow_hidden().bg(black())
                .child(picture)
                .child(div().absolute().inset_0().flex().flex_wrap().children(cells)))
            .child(self.controls(&cuts, b, cx))
            .child(div().id("strip").flex_none().overflow_x_scroll().flex().gap(px(4.)).children(strip))
            .into_any_element()
    }

    pub fn review_screen(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        // the newest frame; while it plays, drawn again each frame
        let player = self.review.as_mut().and_then(|r| r.player.as_mut());
        let pic = player.and_then(|p| {
            if p.playing() {
                window.request_animation_frame();
            }
            p.frame(window)
        });
        let Some(r) = self.review.as_ref() else { return div().into_any_element() };
        let (name, about) = match &r.sheet {
            Some(Ok(s)) => (s.name.clone(), s.about.join(" · ")),
            _ => (r.clip.file_name().unwrap_or_default().to_string_lossy().into_owned(), String::new()),
        };
        let cuts: Vec<f64> = r.sheet.as_ref().and_then(|s| s.as_ref().ok()).map(|s| s.cuts.clone()).unwrap_or_default();
        let notes = r.notes.iter().enumerate().map(|(i, n)| {
            card(&b).gap(px(4.))
                .child(div().flex().items_center().gap(px(8.))
                    .child(text(format!("{:.1}s · shot {}{}", n.t, shot(n.t, &cuts), if n.at.is_some() { " · a spot" } else { "" }), 12., FontWeight::NORMAL, b.gray).flex_1())
                    .child(div().id(("drop", i)).cursor_pointer().child(icon(Icon::Trash, b.gray).size(px(16.)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            if let Some(r) = this.review.as_mut() {
                                r.notes.remove(i);
                            }
                            cx.notify();
                        }))))
                .child(text(n.text.clone(), 14., LIGHT, b.ink))
                .into_any_element()
        }).collect::<Vec<_>>();
        div().size_full().flex().flex_col()
            .child(div().h(HEADER).flex_none().px(px(24.)).flex().items_center().gap(px(8.)).border_b_1().border_color(b.hair)
                .child(icon(Icon::Eye, b.gray).size(px(16.)))
                .child(text(name, 14., BOLD, b.ink).flex_none())
                .child(text(about, 13., FontWeight::NORMAL, b.gray).flex_1().min_w_0().truncate())
                .child(primary("send-notes", "Send notes", &b).on_click(cx.listener(|this, _, window, cx| this.send_notes(window, cx)))))
            .child(div().flex_1().min_h_0().flex()
                .child(self.frame_view(pic, &b, cx))
                .child(div().w(px(320.)).flex_none().border_l_1().border_color(b.hair).flex().flex_col().gap(px(16.)).p(px(24.))
                    .child(text("Notes", 19., BOLD, b.ink))
                    .child(text("Play it, stop where something is off, pick a spot in the picture, then say what to change. Or trim it yourself: Set in, Set out, Cut.", 13., LIGHT, b.gray))
                    .child(card(&b).p(px(8.)).child(Input::new(&r.note).appearance(false)))
                    .child(pill("add-note", Some(Icon::Plus), "Add note", false, &b).on_click(cx.listener(|this, _, window, cx| this.add_note(window, cx))))
                    .child(div().id("notes").flex_1().min_h_0().overflow_y_scroll().flex().flex_col().gap(px(8.)).children(notes))
                    .child(text("The whole edit", 13., BOLD, b.ink))
                    .child(card(&b).p(px(8.)).child(Input::new(&r.general).appearance(false)))))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shots_count_from_one() {
        let cuts = [1.5, 2.9, 4.1];
        assert_eq!([0.0, 1.0, 1.5, 3.2, 9.0].map(|t| super::shot(t, &cuts)), [1, 1, 2, 3, 4]);
    }
}
