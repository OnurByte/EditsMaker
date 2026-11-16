use crate::runner::Runner;
use crate::theme::{card_frame, section_heading, Assets, Palette, CUT_RED, INK, PAPER};
use eframe::egui::{self, Color32, FontId, RichText, ScrollArea, Stroke};

pub struct LooksState {
    pub search: String,
    pub caption_styles: Vec<edits::talk::CaptionStyle>,
    pub grades: Vec<edits::grades::Grade>,
    pub toast: String,
}

impl LooksState {
    pub fn new() -> Self {
        Self {
            search: String::new(),
            caption_styles: edits::talk::styles(),
            grades: edits::grades::all(),
            toast: String::new(),
        }
    }
}

pub enum LooksAction {
    None,
    UseLook(String),
    UseCaption(String),
    AddSticker(String),
    ToggleFx(String),
}

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    p: Palette,
    assets: &Assets,
    state: &mut LooksState,
    runner: &mut Runner,
) -> LooksAction {
    let mut action = LooksAction::None;

    card_frame(p).show(ui, |ui| {
        ui.horizontal(|ui| {
            section_heading(ui, p, "Renkler, Altyazılar ve Çıkartmalar", None);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::TextEdit::singleline(&mut state.search).hint_text("Filtrele...").desired_width(180.0));
            });
        });
        if !state.toast.is_empty() {
            ui.label(RichText::new(&state.toast).size(12.0).color(CUT_RED));
        }
    });

    ui.add_space(8.0);
    let q = state.search.to_lowercase();

    ScrollArea::vertical().id_salt("looks_scroll").show(ui, |ui| {
        // Colour Looks
        card_frame(p).show(ui, |ui| {
            section_heading(ui, p, "Renk Görünümleri (--look)", Some("Yerleşik filtreler + Alight CC paketleri"));
            ui.add_space(8.0);

            let builtin: Vec<(&str, &str)> = edits::looks::LOOKS
                .iter()
                .filter(|(n, about, _)| q.is_empty() || n.contains(&q) || about.to_lowercase().contains(&q))
                .map(|(n, about, _)| (*n, *about))
                .collect();

            let grades: Vec<&edits::grades::Grade> = state
                .grades
                .iter()
                .filter(|g| q.is_empty() || g.name.to_lowercase().contains(&q) || g.about.to_lowercase().contains(&q))
                .collect();

            let avail = ui.available_width().max(240.0);
            let cols = ((avail + 8.0) / 240.0).floor().max(1.0) as usize;

            enum LookCard<'a> {
                Builtin(&'a str, &'a str),
                Grade(&'a edits::grades::Grade),
            }
            let cards: Vec<LookCard<'_>> = builtin
                .into_iter()
                .map(|(n, a)| LookCard::Builtin(n, a))
                .chain(grades.into_iter().map(LookCard::Grade))
                .collect();

            for row in cards.chunks(cols) {
                ui.columns(cols, |row_cols| {
                    for (ci, item) in row.iter().enumerate() {
                        egui::Frame::new()
                            .fill(p.input)
                            .stroke(Stroke::new(1.0_f32, p.line))
                            .corner_radius(6)
                            .inner_margin(10.0)
                            .show(&mut row_cols[ci], |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_min_height(78.0);
                                match item {
                                    LookCard::Builtin(name, about) => {
                                        ui.horizontal(|ui| {
                                            let swatch = look_swatch(name);
                                            let (r, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                                            ui.painter().rect_filled(r, 3.0, swatch);
                                            ui.label(RichText::new(*name).strong().color(p.text));
                                        });
                                        ui.add(egui::Label::new(RichText::new(*about).size(11.5).color(p.muted)).wrap());
                                        ui.add_space(4.0);
                                        if ui.button("Seç").clicked() {
                                            action = LooksAction::UseLook((*name).to_string());
                                        }
                                    }
                                    LookCard::Grade(g) => {
                                        ui.label(RichText::new(&g.name).strong().color(p.text));
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(format!("{} · {} katman", g.about, g.layers.len()))
                                                    .size(11.5)
                                                    .color(p.muted),
                                            )
                                            .wrap(),
                                        );
                                        ui.add_space(4.0);
                                        ui.horizontal(|ui| {
                                            if ui.button("Seç").clicked() {
                                                action = LooksAction::UseLook(g.name.clone());
                                            }
                                            if ui.button(".cube").clicked() && !runner.running {
                                                if let Some(to) = rfd::FileDialog::new()
                                                    .set_file_name(format!("{}.cube", g.name))
                                                    .save_file()
                                                {
                                                    runner.start(
                                                        ctx,
                                                        "cube",
                                                        vec!["cube".into(), g.name.clone(), to.to_string_lossy().into_owned()],
                                                        false,
                                                    );
                                                    state.toast = format!("LUT dışa aktarılıyor: {}", to.display());
                                                }
                                            }
                                        });
                                    }
                                }
                            });
                    }
                });
                ui.add_space(8.0);
            }
        });

        ui.add_space(10.0);

        // Caption Styles
        card_frame(p).show(ui, |ui| {
            section_heading(ui, p, "Altyazı Stilleri (--captions)", None);
            ui.add_space(8.0);

            let styles: Vec<&edits::talk::CaptionStyle> = state
                .caption_styles
                .iter()
                .filter(|st| q.is_empty() || st.name.to_lowercase().contains(&q) || st.about.to_lowercase().contains(&q))
                .collect();

            let avail = ui.available_width().max(240.0);
            let cols = ((avail + 8.0) / 260.0).floor().max(1.0) as usize;

            for row in styles.chunks(cols) {
                ui.columns(cols, |row_cols| {
                    for (ci, st) in row.iter().enumerate() {
                        egui::Frame::new()
                            .fill(p.input)
                            .stroke(Stroke::new(1.0_f32, p.line))
                            .corner_radius(6)
                            .inner_margin(10.0)
                            .show(&mut row_cols[ci], |ui| {
                                ui.set_width(ui.available_width());
                                ui.set_min_height(98.0);
                                ui.label(RichText::new(&st.name).strong().color(p.text));
                                ui.add(egui::Label::new(RichText::new(&st.about).size(11.5).color(p.muted)).wrap());
                                ui.add_space(4.0);
                                egui::Frame::new()
                                    .fill(Color32::from_rgb(0x18, 0x18, 0x1B))
                                    .corner_radius(4)
                                    .inner_margin(8.0)
                                    .show(ui, |ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            let base = parse_hex(&st.color).unwrap_or(PAPER);
                                            let hi = parse_hex(&st.highlight).unwrap_or(CUT_RED);
                                            let sample = if st.upper { "LINK IN." } else { "link in." };
                                            let punch = if st.upper { "EDITS OUT" } else { "edits out" };
                                            ui.label(RichText::new(sample).strong().color(base));
                                            ui.label(RichText::new(punch).strong().color(hi));
                                        });
                                    });
                                ui.add_space(4.0);
                                if ui.button("Stili Kullan").clicked() {
                                    action = LooksAction::UseCaption(st.name.clone());
                                }
                            });
                    }
                });
                ui.add_space(8.0);
            }
        });

        ui.add_space(10.0);

        // SVG Stickers & Beat FX
        ui.columns(2, |cols| {
            card_frame(p).show(&mut cols[0], |ui| {
                section_heading(ui, p, "Çizim Çıkartmaları (--svg)", Some("resvg ile vektörel"));
                ui.add_space(8.0);
                let stickers: Vec<(&str, &str)> = edits::svg::STICKERS
                    .iter()
                    .filter(|(n, about, _)| q.is_empty() || n.contains(&q) || about.to_lowercase().contains(&q))
                    .map(|(n, about, _)| (*n, *about))
                    .collect();

                let avail = ui.available_width().max(180.0);
                let scols = ((avail + 8.0) / 170.0).floor().max(1.0) as usize;

                for row in stickers.chunks(scols) {
                    ui.columns(scols, |row_cols| {
                        for (ci, (name, about)) in row.iter().enumerate() {
                            egui::Frame::new()
                                .fill(p.input)
                                .stroke(Stroke::new(1.0_f32, p.line))
                                .corner_radius(6)
                                .inner_margin(8.0)
                                .show(&mut row_cols[ci], |ui| {
                                    ui.set_width(ui.available_width());
                                    ui.set_min_height(88.0);
                                    ui.horizontal(|ui| {
                                        if let Some(tex) = assets.stickers.get(*name) {
                                            ui.image((tex.id(), egui::vec2(24.0, 24.0)));
                                        }
                                        ui.label(RichText::new(*name).strong().color(p.text));
                                    });
                                    ui.add(egui::Label::new(RichText::new(*about).size(11.0).color(p.muted)).wrap());
                                    if ui.button("Ekle").clicked() {
                                        action = LooksAction::AddSticker(format!("{name}@0.5-2.0:0.5,0.4,0.28#E5332A~pop"));
                                    }
                                });
                        }
                    });
                    ui.add_space(8.0);
                }
            });

            card_frame(p).show(&mut cols[1], |ui| {
                section_heading(ui, p, "Vuruş Efektleri (--fx)", None);
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for hit in edits::fx::HITS {
                        if ui.button(RichText::new(*hit).font(FontId::monospace(12.5))).clicked() {
                            action = LooksAction::ToggleFx((*hit).to_string());
                        }
                    }
                });
                ui.add_space(8.0);
                ui.label(RichText::new("Giriş Animasyonları (--anim):").size(12.0).color(p.muted));
                ui.label(RichText::new(edits::svg::ANIMS.join(" · ")).font(FontId::monospace(12.0)).color(p.text));
            });
        });
    });

    action
}

fn parse_hex(s: &str) -> Option<Color32> {
    let h = s.trim_start_matches('#');
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some(Color32::from_rgb(r, g, b))
}

fn look_swatch(name: &str) -> Color32 {
    match name {
        "bw" => Color32::from_gray(160),
        "noir" => INK,
        "dark" => Color32::from_gray(75),
        "teal" => Color32::from_rgb(0x2A, 0x9D, 0x8F),
        "tealorange" => Color32::from_rgb(0xE7, 0x6F, 0x51),
        "warm" => Color32::from_rgb(0xE9, 0xC4, 0x6A),
        "cold" => Color32::from_rgb(0x45, 0x7B, 0x9D),
        "faded" => Color32::from_rgb(0xB8, 0xB8, 0xAA),
        "vintage" => Color32::from_rgb(0xCB, 0x99, 0x7E),
        "sepia" => Color32::from_rgb(0x9C, 0x66, 0x44),
        "bleach" => Color32::from_rgb(0x8D, 0x99, 0xAE),
        "neon" => Color32::from_rgb(0xC7, 0x7D, 0xFF),
        "matrix" => Color32::from_rgb(0x2D, 0x6A, 0x4F),
        "sunset" => CUT_RED,
        "pop" => Color32::from_rgb(0xFF, 0xBE, 0x0B),
        _ => PAPER,
    }
}
