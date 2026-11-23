//! The few shapes every screen is built from, in brand sizes: text, an icon button, a pill, a badge, a card.
use crate::app::icons::{icon, Icon};
use crate::app::theme::{Brand, CARD, SMALL};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

pub fn text(s: impl Into<SharedString>, size: f32, weight: FontWeight, color: Hsla) -> Div {
    div().text_size(px(size)).font_weight(weight).text_color(color).child(s.into())
}

/// A 32 px square button with a 16 px icon, the page header's and the composer's.
pub fn icon_button(id: impl Into<ElementId>, i: Icon, b: &Brand) -> Stateful<Div> {
    let hover = b.card;
    div().id(id).size(px(32.)).flex().items_center().justify_center().rounded(SMALL).cursor_pointer()
        .hover(move |s| s.bg(hover)).child(icon(i, b.gray).size(px(16.)))
}

/// A chip: half its height round, filled when on.
pub fn pill(id: impl Into<ElementId>, i: Option<Icon>, label: impl Into<SharedString>, on: bool, b: &Brand) -> Stateful<Div> {
    let (bg, fg, hover) = (if on { b.accent.opacity(0.16) } else { transparent_black() }, if on { b.ink } else { b.gray }, b.card);
    div().id(id).h(px(32.)).px(px(12.)).flex().items_center().gap(px(8.)).rounded(px(16.)).cursor_pointer().bg(bg)
        .when(!on, move |d| d.hover(move |s| s.bg(hover)))
        .children(i.map(|i| icon(i, if on { b.accent } else { fg }).size(px(16.))))
        .child(text(label, 13., FontWeight::NORMAL, fg))
}

/// The one strong button in view: white on the accent.
pub fn primary(id: impl Into<ElementId>, label: impl Into<SharedString>, b: &Brand) -> Stateful<Div> {
    div().id(id).h(px(32.)).px(px(16.)).flex().items_center().rounded(px(16.)).cursor_pointer().bg(b.accent)
        .child(text(label, 13., crate::app::theme::BOLD, white()))
}

/// A project's two letters on a small square, as t3code marks a thread's project.
pub fn badge(s: &str, b: &Brand) -> Div {
    div().size(px(16.)).flex_none().flex().items_center().justify_center().rounded(px(4.)).bg(b.hair)
        .child(text(crate::app::runs::initials(s), 8., crate::app::theme::BOLD, b.ink))
}

/// A section on its own card, one step off the page.
pub fn card(b: &Brand) -> Div {
    div().flex().flex_col().bg(b.card).rounded(CARD).p(px(16.))
}

/// A tag as a word on its own colour, never "#word": the same tag is always the same colour, so they are told
/// apart at a glance.
pub fn tag(label: &str, b: &Brand) -> Div {
    const COLOURS: &[u32] = &[0xE5332A, 0x3B6FF5, 0x2EA043, 0x8B5CF6, 0xF28C28, 0x0EA5A4, 0xDB2777, 0xCA8A04, 0x6366F1, 0x059669];
    let c: Hsla = rgb(COLOURS[crate::util::crc32(label.as_bytes()) as usize % COLOURS.len()]).into();
    let _ = b;
    div().h(px(24.)).px(px(10.)).flex().flex_none().items_center().rounded(px(12.)).bg(c.opacity(0.14))
        .child(text(label.to_string(), 12., crate::app::theme::BOLD, c))
}

/// What lies under a modal: the page frosted over (GPUI draws no backdrop blur, so the page's own colour veils it and
/// a dark tint pushes it back), taking every click and scroll so nothing behind moves. A click on it closes the modal.
pub fn backdrop(id: impl Into<SharedString>, b: &Brand) -> Stateful<Div> {
    div().id(ElementId::Name(id.into())).absolute().inset_0().occlude().bg(b.page.opacity(0.72))
        .child(div().size_full().bg(black().opacity(0.28)))
}

/// A modal sheet over the window: a frosted backdrop that closes it, and the card in the middle.
pub fn sheet(id: &'static str, width: f32, b: &Brand, close: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> (Stateful<Div>, Stateful<Div>) {
    let backdrop = backdrop(format!("{id}-backdrop"), b).on_click(close);
    let card = div().id(id).relative().occlude().shadow_2xl().w(px(width)).max_w(relative(0.94)).max_h(relative(0.9)).flex().flex_col().rounded(CARD).bg(b.card).overflow_y_scroll()
        .on_click(|_, _, cx| cx.stop_propagation());
    (backdrop, card)
}
