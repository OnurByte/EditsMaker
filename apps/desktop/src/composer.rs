//! t3code's ChatComposer: one card under the timeline. On top, the files attached (dropped or picked, each made on
//! its own) and the list a picker opened; then the prompt, growing with what is typed (Enter sends, Shift+Enter is a
//! new line); under it what to make and with what on the left, a file and send on the right.
use crate::icons::{icon, Icon};
use crate::theme::{brand, Brand, CARD, COLUMN, SMALL};
use crate::ui::{icon_button, pill, text};
use crate::workspace::{Mode, Picker, Workspace};
use gpui_kit::component::input::Textarea;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

impl Workspace {
    /// The open picker's choices: presets (from the packs that carry one) or RSS packs.
    pub fn picker_list(&self, which: Picker, b: &Brand, cx: &mut Context<Self>) -> impl IntoElement {
        let packs = self.market.packs.as_ref().and_then(|p| p.as_ref().ok()).cloned().unwrap_or_default();
        let mut rows: Vec<(String, String, Option<String>)> = match which {
            Picker::Preset => packs.iter().filter_map(|p| Some((p.preset.clone()?, p.description.clone(), p.preset.clone()))).collect(),
            Picker::Source | Picker::FlowSource => packs.iter().filter(|p| p.is("feed")).map(|p| (p.name.clone(), p.description.clone(), Some(p.short()))).collect(),
        };
        if which == Picker::Preset {
            rows.insert(0, ("No preset".into(), "edits picks the cuts and the moves for the footage".into(), None));
        }
        let list = rows.into_iter().enumerate().map(|(i, (name, about, value))| {
            let hover = b.hair;
            div().id(("choice", i)).px(px(8.)).py(px(8.)).rounded(SMALL).cursor_pointer().hover(move |s| s.bg(hover)).flex().flex_col().gap(px(2.))
                .child(text(name, 14., FontWeight::NORMAL, b.ink).truncate())
                .child(text(about, 12., FontWeight::NORMAL, b.gray).truncate())
                .on_click(cx.listener(move |this, _, _, cx| {
                    match which {
                        Picker::Preset => this.preset = value.clone(),
                        Picker::Source => this.source = value.clone(),
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
        let mode = |id: &'static str, i: Icon, label: &'static str, m: Mode, cx: &mut Context<Self>| {
            pill(id, Some(i), label, self.mode == m, &b).on_click(cx.listener(move |this, _, _, cx| {
                this.mode = m;
                this.picker = None;
                cx.notify();
            }))
        };
        let files = self.files.iter().enumerate().map(|(i, f)| {
            div().h(px(32.)).pl(px(12.)).pr(px(4.)).flex().items_center().gap(px(8.)).rounded(px(16.)).bg(b.hair)
                .child(icon(Icon::Play, b.ink).size(px(16.)))
                .child(text(f.file_name().unwrap_or_default().to_string_lossy().into_owned(), 13., FontWeight::NORMAL, b.ink).max_w(px(240.)).truncate())
                .child(div().id(("unattach", i)).size(px(24.)).flex().items_center().justify_center().rounded_full().cursor_pointer()
                    .child(text("×", 14., FontWeight::NORMAL, b.gray))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.files.remove(i);
                        cx.notify();
                    })))
        }).collect::<Vec<_>>();
        let preset = self.picker_pill("preset", Icon::Package, self.preset.clone().unwrap_or_else(|| "No preset".into()), Picker::Preset, &b, cx);
        let source = self.picker_pill("source", Icon::Workflow, self.source.clone().unwrap_or_else(|| "Pick a source".into()), Picker::Source, &b, cx);
        // how many edits each run makes: 1, 3, 5
        let count = pill("count", None, format!("{}x", self.count), false, &b).on_click(cx.listener(|this, _, _, cx| {
            this.count = match this.count { 1 => 3, 3 => 5, _ => 1 };
            cx.notify();
        }));
        let picker = self.picker.filter(|p| *p != Picker::FlowSource).map(|p| self.picker_list(p, &b, cx));
        div().w_full().max_w(COLUMN).mx_auto().flex().flex_col().gap(px(8.)).bg(b.card).rounded(CARD).p(px(16.))
            .when(!files.is_empty(), |d| d.child(div().flex().flex_wrap().gap(px(8.)).children(files)))
            .children(picker)
            .child(Textarea::new(&self.input).appearance(false))
            .child(div().flex().items_center().gap(px(8.)).flex_wrap()
                .child(mode("mode-edit", Icon::Film, "Edits", Mode::Edit, cx))
                .child(mode("mode-pack", Icon::Package, "Pack", Mode::Pack, cx))
                .child(mode("mode-rss", Icon::Workflow, "RSS", Mode::Rss, cx))
                .child(div().w(px(1.)).h(px(16.)).bg(b.hair))
                .when(self.mode == Mode::Rss, |d| d.child(source))
                .when(self.mode != Mode::Pack, |d| d.child(preset).child(count))
                .child(div().flex_1())
                .child(icon_button("pick", Icon::Folder, &b).on_click(cx.listener(|this, _, window, cx| this.pick(window, cx))))
                .child(div().id("send").size(px(32.)).flex().items_center().justify_center().rounded_full().cursor_pointer().bg(b.ink)
                    .child(icon(Icon::Send, b.page).size(px(16.)))
                    .on_click(cx.listener(|this, _, window, cx| this.start(window, cx)))))
    }
}
