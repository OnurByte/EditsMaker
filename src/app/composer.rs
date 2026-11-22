//! t3code's ChatComposer: one card under the timeline. On top, the files attached (dropped or picked, each made on
//! its own) and the style list when it is open; then the prompt, growing with what is typed (Enter sends, Shift+Enter
//! is a new line); under it a file on the left, the style (as t3code's model picker) and send on the right.
use crate::app::icons::{icon, Icon};
use crate::app::theme::{self, brand, Brand, CARD, SMALL};
use crate::app::ui::{icon_button, pill, text};
use crate::app::workspace::{Picker, Workspace};
use gpui_kit::component::input::Textarea;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

impl Workspace {
    /// The open picker's choices: styles (the packs' presets) or topics (for a flow).
    pub fn picker_list(&self, which: Picker, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let packs = self.market.packs.as_ref().and_then(|p| p.as_ref().ok()).cloned().unwrap_or_default();
        let mut rows: Vec<(String, String, Option<String>)> = match which {
            Picker::Preset => packs.iter().filter_map(|p| Some((p.preset.clone()?, p.description.clone(), p.preset.clone()))).collect(),
            Picker::FlowSource => packs.iter().filter(|p| p.is("feed")).map(|p| (p.name.clone(), p.description.clone(), Some(p.short()))).collect(),
        };
        if which == Picker::Preset {
            rows.insert(0, ("Auto".into(), "edits picks the cuts and the moves for the footage".into(), None));
        }
        let list = rows.into_iter().enumerate().map(|(i, (name, about, value))| {
            let hover = b.hair;
            div().id(("choice", i)).px(px(8.)).py(px(8.)).rounded(SMALL).cursor_pointer().hover(move |s| s.bg(hover)).flex().flex_col().gap(px(2.))
                .child(text(name, 14., FontWeight::NORMAL, b.ink).truncate())
                .child(text(about, 12., FontWeight::NORMAL, b.gray).truncate())
                .on_click(cx.listener(move |this, _, _, cx| {
                    match which {
                        Picker::Preset => this.preset = value.clone(),
                        Picker::FlowSource => this.flow_source = value.clone(),
                    }
                    this.picker = None;
                    cx.notify();
                }))
        });
        div().id("picker").max_h(px(240.)).overflow_y_scroll().flex().flex_col().gap(px(2.)).children(list)
    }

    /// A pill that opens a picker, saying what is chosen.
    pub fn picker_pill(&self, id: &'static str, i: Icon, label: String, which: Picker, b: &Brand, cx: &mut Context<Self>) -> Stateful<Div> {
        pill(id, Some(i), label, self.picker == Some(which), b).on_click(cx.listener(move |this, _, _, cx| {
            this.picker = if this.picker == Some(which) { None } else { Some(which) };
            cx.notify();
        }))
    }

    pub fn composer(&self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let b = brand(cx);
        let files = self.files.iter().enumerate().map(|(i, f)| {
            div().h(px(32.)).pl(px(12.)).pr(px(4.)).flex().items_center().gap(px(8.)).rounded(px(16.)).bg(b.hair)
                .child(icon(Icon::Film, b.ink).size(px(16.)))
                .child(text(f.file_name().unwrap_or_default().to_string_lossy().into_owned(), 13., FontWeight::NORMAL, b.ink).max_w(px(240.)).truncate())
                .child(div().id(("unattach", i)).size(px(24.)).flex().items_center().justify_center().rounded_full().cursor_pointer()
                    .child(text("×", 14., FontWeight::NORMAL, b.gray))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.files.remove(i);
                        cx.notify();
                    })))
        }).collect::<Vec<_>>();
        let style = self.picker_pill("style", Icon::Sliders, self.preset.clone().unwrap_or_else(|| "Auto".into()), Picker::Preset, &b, cx);
        let picker = self.picker.filter(|p| *p == Picker::Preset).map(|p| self.picker_list(p, &b, cx));
        div().w_full().max_w(theme::column()).mx_auto().flex().flex_col().gap(px(8.)).bg(b.card).rounded(CARD).p(px(16.))
            .when(!files.is_empty(), |d| d.child(div().flex().flex_wrap().gap(px(8.)).children(files)))
            .children(picker)
            .child(Textarea::new(&self.input).appearance(false))
            .child(div().flex().items_center().gap(px(8.))
                .child(icon_button("pick", Icon::Attach, &b).on_click(cx.listener(|this, _, window, cx| this.pick(window, cx))))
                .child(div().flex_1())
                .child(style)
                .child(div().id("send").size(px(32.)).flex().items_center().justify_center().rounded_full().cursor_pointer().bg(b.accent)
                    .child(icon(Icon::Send, white()).size(px(16.)))
                    .on_click(cx.listener(|this, _, window, cx| this.start(window, cx)))))
    }
}
