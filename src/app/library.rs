//! The library: every clip edits made, newest first, as cards (its contact sheet when there is one), each opened in
//! the editor (review), played in the system's player, find in its folder or share with the system's share sheet. The sidebar's Library opens it.
use crate::app::icons::{icon, illustration, Icon};
use crate::app::runs;
use crate::app::theme::{brand, Brand, BOLD, HEADER, LIGHT};
use crate::app::ui::{icon_button, text};
use crate::app::workspace::Workspace;
use gpui_kit::component::input::Input;
use gpui_kit::*;
use std::path::PathBuf;

/// A clip in the library: the file, what it was made from, when.
pub struct Item {
    pub clip: PathBuf,
    pub subject: String,
    pub age: String,
}

/// The run's contact sheet beside a clip (NAME-sheet.jpg), its picture on the card.
pub fn thumb(clip: &std::path::Path) -> Option<PathBuf> {
    let stem = clip.file_stem()?.to_string_lossy();
    Some(clip.with_file_name(format!("{stem}-sheet.jpg"))).filter(|p| p.is_file())
}

impl Workspace {
    fn library_items(&self, q: &str) -> Vec<Item> {
        self.past.iter().flat_map(|r| runs::clips(&r.dir).into_iter().map(move |c| Item { clip: c, subject: r.subject.clone(), age: runs::ago(r.made) }))
            .filter(|i| crate::app::sidebar::matches(q, &i.subject, &i.clip.file_name().unwrap_or_default().to_string_lossy()))
            .take(400).collect()
    }

    fn clip_card(&self, i: usize, it: Item, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let (play, show, review, share) = (it.clip.clone(), it.clip.clone(), it.clip.clone(), it.clip.clone());
        let pic = thumb(&it.clip).and_then(|t| crate::app::icons::picture(&t.display().to_string(), || Some((ImageFormat::Jpeg, std::fs::read(&t).ok()?))));
        let name = it.clip.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        let hover = b.hair;
        div().id(("clip", i)).w(px(208.)).flex().flex_col().gap(px(8.)).p(px(8.)).rounded(px(14.)).bg(b.card).cursor_pointer().hover(move |s| s.bg(hover))
            // opened in the editor, as t3code opens a thread: its frames, its shots, notes for the next cut
            .on_click(cx.listener(move |this, _, window, cx| this.open_review(review.clone(), window, cx)))
            .child(div().w_full().h(px(148.)).rounded(px(10.)).overflow_hidden().bg(b.hair).flex().items_center().justify_center()
                .child(match pic {
                    Some(img) => img.size_full().object_fit(ObjectFit::Cover).into_any_element(),
                    None => icon(Icon::Film, b.gray).size(px(28.)).into_any_element(),
                }))
            .child(div().px(px(4.)).flex().flex_col().gap(px(2.))
                .child(text(name, 14., BOLD, b.ink).truncate())
                .child(text(format!("{} · {}", it.subject, it.age), 12., FontWeight::NORMAL, b.gray).truncate()))
            .child(div().flex().gap(px(2.))
                .child(icon_button(("lib-play", i), Icon::Play, b).tooltip(|w, cx| gpui_kit::component::tooltip::Tooltip::new("Open in your player").build(w, cx))
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        cx.open_with_system(&play);
                    }))
                .child(icon_button(("lib-show", i), Icon::Folder, b).tooltip(|w, cx| gpui_kit::component::tooltip::Tooltip::new("Show in folder").build(w, cx))
                    .on_click(move |_, _, cx| {
                        cx.stop_propagation();
                        cx.reveal_path(&show);
                    }))
                .child(icon_button(("lib-share", i), Icon::Share, b).tooltip(|w, cx| gpui_kit::component::tooltip::Tooltip::new("Share").build(w, cx))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.share_file(share.clone(), window, cx);
                    }))))
    }

    pub fn library(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let q = self.library_search.read(cx).value().to_string();
        let items = self.library_items(&q);
        let n = items.len();
        let body: AnyElement = if items.is_empty() {
            div().pt(px(48.)).flex().flex_col().items_center().gap(px(12.))
                .child(illustration(if q.trim().is_empty() { "library" } else { "search" }).w(px(200.)).h(px(150.)))
                .child(text(if q.trim().is_empty() { "Nothing here yet" } else { "Nothing like that" }, 17., BOLD, b.ink))
                .child(text(if q.trim().is_empty() { "Every edit you make lands here, ready to watch, review and share." } else { "Try another word: what it was made from, or the clip's name." }, 14., LIGHT, b.gray))
                .into_any_element()
        } else {
            div().flex().flex_wrap().gap(px(16.)).children(items.into_iter().enumerate().map(|(i, it)| self.clip_card(i, it, &b, cx))).into_any_element()
        };
        let out = runs::out_dir();
        div().size_full().flex().flex_col()
            .child(div().h(HEADER).flex_none().px(px(24.)).flex().items_center().gap(px(8.)).border_b_1().border_color(b.hair)
                .child(icon(Icon::Library, b.gray).size(px(16.)))
                .child(text("Library", 14., BOLD, b.ink))
                .child(text(if n == 1 { "1 clip".into() } else { format!("{n} clips") }, 13., FontWeight::NORMAL, b.gray).flex_1())
                .child(div().w(px(260.)).child(Input::new(&self.library_search).prefix(icon(Icon::Search, b.gray).size(px(14.))).suffix(crate::app::sidebar::shortcut(&b)).cleanable(true)))
                .child(icon_button("lib-folder", Icon::Folder, &b).on_click(move |_, _, cx| cx.open_with_system(&out))))
            .child(div().id("library").flex_1().min_h_0().overflow_y_scroll()
                .child(div().w_full().max_w(px(1120.)).mx_auto().p(px(24.)).flex().flex_col().gap(px(16.)).child(body)))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_clip_wears_its_contact_sheet() {
        let d = std::env::temp_dir().join(format!("edits-lib-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("sad-01.mp4"), b"").unwrap();
        assert!(super::thumb(&d.join("sad-01.mp4")).is_none());
        std::fs::write(d.join("sad-01-sheet.jpg"), b"").unwrap();
        assert_eq!(super::thumb(&d.join("sad-01.mp4")), Some(d.join("sad-01-sheet.jpg")));
        let _ = std::fs::remove_dir_all(&d);
    }
}
