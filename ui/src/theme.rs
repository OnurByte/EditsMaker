use edits::resvg::{self, tiny_skia, usvg};
use eframe::egui::{self, Color32, ColorImage, CornerRadius, FontId, Margin, Stroke, TextureHandle, TextureOptions, Visuals};
use std::collections::HashMap;

/// EditsMaker design tokens (from `design/tokens.css`).
pub const INK: Color32 = Color32::from_rgb(0x0B, 0x0B, 0x0C);
pub const INK_ELEVATED: Color32 = Color32::from_rgb(0x13, 0x13, 0x16);
pub const INK_INPUT: Color32 = Color32::from_rgb(0x19, 0x19, 0x1D);
pub const PAPER: Color32 = Color32::from_rgb(0xF4, 0xF1, 0xEA);
pub const PAPER_ELEVATED: Color32 = Color32::from_rgb(0xEB, 0xE6, 0xDC);
pub const PAPER_INPUT: Color32 = Color32::from_rgb(0xFF, 0xFD, 0xF9);
pub const CUT_RED: Color32 = Color32::from_rgb(0xE5, 0x33, 0x2A);
pub const WARM_GRAY: Color32 = Color32::from_rgb(0x8C, 0x87, 0x7C);
pub const LINE_DARK: Color32 = Color32::from_rgb(0x36, 0x36, 0x3B);
pub const LINE_LIGHT: Color32 = Color32::from_rgb(0xCE, 0xC8, 0xBC);
pub const OK_GREEN: Color32 = Color32::from_rgb(0x3E, 0xB4, 0x71);
pub const WARN_AMBER: Color32 = Color32::from_rgb(0xE0, 0x9F, 0x3E);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Ink,
    Paper,
}

#[derive(Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: Color32,
    pub panel: Color32,
    pub input: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub line: Color32,
}

impl Palette {
    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Ink => Self {
                dark: true,
                bg: INK,
                panel: INK_ELEVATED,
                input: INK_INPUT,
                text: PAPER,
                muted: WARM_GRAY,
                line: LINE_DARK,
            },
            ThemeMode::Paper => Self {
                dark: false,
                bg: PAPER,
                panel: PAPER_ELEVATED,
                input: PAPER_INPUT,
                text: INK,
                muted: WARM_GRAY,
                line: LINE_LIGHT,
            },
        }
    }
}

pub fn apply_theme(ctx: &egui::Context, mode: ThemeMode) {
    let p = Palette::from_mode(mode);
    let mut visuals = if p.dark { Visuals::dark() } else { Visuals::light() };

    visuals.panel_fill = p.bg;
    visuals.window_fill = p.panel;
    visuals.extreme_bg_color = p.input;
    visuals.faint_bg_color = p.panel;
    visuals.code_bg_color = p.input;
    visuals.override_text_color = Some(p.text);

    visuals.selection.bg_fill = CUT_RED;
    visuals.selection.stroke = Stroke::new(1.0_f32, PAPER);

    visuals.widgets.noninteractive.bg_fill = p.panel;
    visuals.widgets.noninteractive.weak_bg_fill = p.panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.line);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(6);

    visuals.widgets.inactive.bg_fill = p.input;
    visuals.widgets.inactive.weak_bg_fill = p.panel;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.line);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, p.text);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(6);

    visuals.widgets.hovered.bg_fill = if p.dark {
        Color32::from_rgb(0x23, 0x23, 0x28)
    } else {
        Color32::from_rgb(0xE2, 0xDC, 0xD0)
    };
    visuals.widgets.hovered.weak_bg_fill = visuals.widgets.hovered.bg_fill;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, WARM_GRAY);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.2_f32, p.text);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(6);

    visuals.widgets.active.bg_fill = CUT_RED;
    visuals.widgets.active.weak_bg_fill = CUT_RED;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, CUT_RED);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, PAPER);
    visuals.widgets.active.corner_radius = CornerRadius::same(6);

    visuals.window_stroke = Stroke::new(1.0_f32, p.line);
    visuals.window_corner_radius = CornerRadius::same(8);

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 6.0);
    style.spacing.window_margin = Margin::same(12);
    style.spacing.indent = 16.0;
    ctx.set_style(style);
}

pub struct Assets {
    pub mark_light: Option<TextureHandle>,
    pub mark_dark: Option<TextureHandle>,
    pub stickers: HashMap<String, TextureHandle>,
}

impl Assets {
    pub fn load(ctx: &egui::Context) -> Self {
        let mark_svg = include_str!("../../assets/mark.svg");
        let mark_dark_svg = mark_svg.replace("#0B0B0C", "#F4F1EA");

        let mark_light = rasterize_svg(ctx, "edits_mark_light", mark_svg, 64);
        let mark_dark = rasterize_svg(ctx, "edits_mark_dark", &mark_dark_svg, 64);

        let mut stickers = HashMap::new();
        for (name, _, body) in edits::svg::STICKERS {
            let full = format!(
                r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">{}</svg>"#,
                body.replace("currentColor", "#E5332A")
            );
            if let Some(tex) = rasterize_svg(ctx, &format!("sticker_{name}"), &full, 72) {
                stickers.insert((*name).to_string(), tex);
            }
        }

        Self {
            mark_light,
            mark_dark,
            stickers,
        }
    }
}

pub fn window_icon() -> Option<egui::IconData> {
    let svg_xml = include_str!("../../assets/icon.svg");
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg_xml, &opt).ok()?;
    let sz = tree.size();
    let size_px = 64u32;
    let scale = size_px as f32 / sz.width().max(1.0);
    let mut pixmap = tiny_skia::Pixmap::new(size_px, size_px)?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    Some(egui::IconData {
        rgba: pixmap.take(),
        width: size_px,
        height: size_px,
    })
}

pub fn rasterize_svg(ctx: &egui::Context, name: &str, svg_xml: &str, size_px: u32) -> Option<TextureHandle> {
    let opt = usvg::Options::default();
    let tree = usvg::Tree::from_str(svg_xml, &opt).ok()?;
    let sz = tree.size();
    let scale = size_px as f32 / sz.width().max(1.0);
    let w = size_px.max(1);
    let h = ((sz.height() * scale).round() as u32).max(1);
    let mut pixmap = tiny_skia::Pixmap::new(w, h)?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    let img = ColorImage::from_rgba_unmultiplied([w as usize, h as usize], pixmap.data());
    Some(ctx.load_texture(name, img, TextureOptions::LINEAR))
}

pub fn card_frame(p: Palette) -> egui::Frame {
    egui::Frame::new()
        .fill(p.panel)
        .stroke(Stroke::new(1.0_f32, p.line))
        .corner_radius(CornerRadius::same(8))
        .inner_margin(Margin::same(14))
}

pub fn section_heading(ui: &mut egui::Ui, p: Palette, title: &str, subtitle: Option<&str>) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).font(FontId::proportional(15.0)).strong().color(p.text));
        if let Some(sub) = subtitle {
            ui.label(egui::RichText::new(sub).font(FontId::proportional(12.0)).color(p.muted));
        }
    });
}

pub fn cta_button<'a>(text: &'a str) -> egui::Button<'a> {
    egui::Button::new(egui::RichText::new(text).color(PAPER).strong())
        .fill(CUT_RED)
        .stroke(Stroke::new(1.0_f32, CUT_RED))
        .corner_radius(CornerRadius::same(6))
}

pub fn toggle_chip(ui: &mut egui::Ui, p: Palette, selected: bool, label: &str) -> egui::Response {
    let (fill, text_color, stroke) = if selected {
        (CUT_RED, PAPER, Stroke::new(1.0_f32, CUT_RED))
    } else {
        (p.input, p.text, Stroke::new(1.0_f32, p.line))
    };
    ui.add(
        egui::Button::new(egui::RichText::new(label).size(12.5).color(text_color))
            .fill(fill)
            .stroke(stroke)
            .corner_radius(CornerRadius::same(14)),
    )
}
