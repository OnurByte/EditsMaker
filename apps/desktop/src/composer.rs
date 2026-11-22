//! t3code's ChatComposer: one card under the timeline, the prompt on top and a row under it: what to make on the
//! left, a file and send on the right.
use crate::icons::{icon, Icon};
use crate::theme::{brand, CARD, COLUMN};
use crate::ui::{icon_button, pill};
use crate::workspace::{Mode, Workspace};
use gpui_kit::component::input::Input;
use gpui_kit::*;

impl Workspace {
    pub fn composer(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let mode = |id: &'static str, i: Icon, label: &'static str, m: Mode, cx: &mut Context<Self>| {
            pill(id, Some(i), label, self.mode == m, &b).on_click(cx.listener(move |this, _, _, cx| {
                this.mode = m;
                cx.notify();
            }))
        };
        div().w_full().max_w(COLUMN).mx_auto().flex().flex_col().gap(px(8.)).bg(b.card).rounded(CARD).p(px(16.))
            .child(Input::new(&self.input).appearance(false))
            .child(div().flex().items_center().gap(px(8.))
                .child(mode("mode-edit", Icon::Film, "Edits", Mode::Edit, cx))
                .child(mode("mode-pack", Icon::Package, "Pack", Mode::Pack, cx))
                .child(div().flex_1())
                .child(icon_button("pick", Icon::Folder, &b).on_click(cx.listener(|this, _, window, cx| this.pick(window, cx))))
                .child(div().id("send").size(px(32.)).flex().items_center().justify_center().rounded_full().cursor_pointer().bg(b.ink)
                    .child(icon(Icon::Send, b.page).size(px(16.)))
                    .on_click(cx.listener(|this, _, window, cx| this.start(window, cx)))))
    }
}
