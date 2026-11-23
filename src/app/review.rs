//! Reviewing a clip: its frames in a strip, the one picked large; point at a spot in it (a 3 x 3 grid), say what to
//! change there, add a word on the whole edit, and the notes go beside the clip for the agent (`edits review`).
use crate::app::icons::{icon, Icon};
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
        let t = r.sheet.as_ref().and_then(|s| s.as_ref().ok()).and_then(|s| s.frames.get(r.at)).map_or(0.0, |f| f.t);
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

    fn frame_view(&self, b: &Brand, cx: &mut Context<Self>) -> AnyElement {
        let Some(r) = self.review.as_ref() else { return div().into_any_element() };
        let sheet = match &r.sheet {
            None => return text("Taking the clip's frames...", 14., FontWeight::NORMAL, b.gray).p(px(24.)).into_any_element(),
            Some(Err(e)) => return text(e.clone(), 14., FontWeight::NORMAL, b.red_text).p(px(24.)).into_any_element(),
            Some(Ok(s)) => s,
        };
        let Some(frame) = sheet.frames.get(r.at) else { return text("This clip has no frames", 14., FontWeight::NORMAL, b.gray).into_any_element() };
        // nine places to point at, over the frame
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
        let strip = sheet.frames.iter().enumerate().map(|(i, f)| {
            let on = i == r.at;
            let cut = i > 0 && shot(f.t, &sheet.cuts) != shot(sheet.frames[i - 1].t, &sheet.cuts);
            div().id(("frame", i)).flex_none().h(px(72.)).w(px(48.)).rounded(SMALL).overflow_hidden().cursor_pointer()
                .when(cut, |d| d.ml(px(8.)))
                .border_2().border_color(if on { b.ink } else { transparent_black() })
                .child(img(f.file.clone()).size_full().object_fit(ObjectFit::Cover))
                .on_click(cx.listener(move |this, _, _, cx| {
                    if let Some(r) = this.review.as_mut() {
                        r.at = i;
                    }
                    cx.notify();
                }))
        });
        div().flex_1().min_w_0().flex().flex_col().gap(px(16.)).p(px(24.))
            .child(div().flex_1().min_h_0().relative().rounded(SMALL).overflow_hidden().bg(b.card)
                .child(img(frame.file.clone()).size_full().object_fit(ObjectFit::Contain))
                .child(div().absolute().inset_0().flex().flex_wrap().children(cells)))
            .child(text(format!("{:.1}s of {:.1}s · shot {} of {}", frame.t, sheet.total, shot(frame.t, &sheet.cuts), shot(sheet.total, &sheet.cuts)), 14., FontWeight::NORMAL, b.gray))
            .child(div().id("strip").flex_none().overflow_x_scroll().flex().gap(px(4.)).children(strip))
            .into_any_element()
    }

    pub fn review_screen(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
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
                .child(self.frame_view(&b, cx))
                .child(div().w(px(320.)).flex_none().border_l_1().border_color(b.hair).flex().flex_col().gap(px(16.)).p(px(24.))
                    .child(text("Notes", 19., BOLD, b.ink))
                    .child(text("Pick a frame and a spot in it, then say what to change.", 13., LIGHT, b.gray))
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
