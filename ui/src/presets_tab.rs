use crate::runner::Runner;
use crate::theme::{card_frame, cta_button, section_heading, toggle_chip, Palette, CUT_RED, OK_GREEN, PAPER, WARM_GRAY};
use edits::preset::{Curve, Preset};
use eframe::egui::{self, Color32, FontId, Pos2, Rect, RichText, ScrollArea, Sense, Stroke, TextEdit};
use std::collections::BTreeSet;

pub struct PresetsState {
    pub presets: Vec<Preset>,
    pub search: String,
    pub selected_tag: Option<String>,
    pub selected_index: usize,
    pub selected_slot: usize,
    pub json_buf: String,
    pub json_dirty: bool,
    pub feedback: String,
    pub feedback_err: bool,
    pub create_name: String,
    pub create_length: f64,
}

impl PresetsState {
    pub fn new() -> Self {
        let mut s = Self {
            presets: Vec::new(),
            search: String::new(),
            selected_tag: None,
            selected_index: 0,
            selected_slot: 0,
            json_buf: String::new(),
            json_dirty: false,
            feedback: String::new(),
            feedback_err: false,
            create_name: String::new(),
            create_length: 18.0,
        };
        s.reload();
        s
    }

    pub fn reload(&mut self) {
        self.presets = edits::presets::all();
        if self.selected_index >= self.presets.len() {
            self.selected_index = 0;
        }
        self.sync_json();
    }

    fn sync_json(&mut self) {
        if let Some(p) = self.presets.get(self.selected_index) {
            self.json_buf = edits::preset::pretty(p);
            self.json_dirty = false;
            self.selected_slot = 0;
        } else {
            self.json_buf.clear();
        }
    }

    pub fn all_tags(&self) -> Vec<String> {
        let mut set = BTreeSet::new();
        for p in &self.presets {
            for t in p.tags() {
                set.insert(t.to_string());
            }
        }
        set.into_iter().collect()
    }
}

pub enum PresetsAction {
    None,
    UseInMake(String),
    CatalogChanged,
}

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    p: Palette,
    state: &mut PresetsState,
    runner: &mut Runner,
) -> PresetsAction {
    let mut action = PresetsAction::None;

    // Top action bar: Import, Learn, Create from video
    card_frame(p).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            section_heading(
                ui,
                p,
                "Preset Kütüphanesi",
                Some(&format!("{} preset · {} cut move", state.presets.len(), edits::presets::motions().len())),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Yenile").clicked() {
                    state.reload();
                }
                if ui.button("Videodan Öğren (learn)").clicked() && !runner.running {
                    if let Some(file) = rfd::FileDialog::new().add_filter("Video", &["mp4", "mov", "mkv"]).pick_file() {
                        runner.start(ctx, "preset learn", vec!["learn".into(), file.to_string_lossy().into_owned()], false);
                    }
                }
                if ui.button("Videodan Oluştur (create)").clicked() && !runner.running {
                    if let Some(file) = rfd::FileDialog::new().add_filter("Video", &["mp4", "mov", "mkv"]).pick_file() {
                        let mut args = vec!["preset".into(), "create".into(), file.to_string_lossy().into_owned()];
                        if !state.create_name.trim().is_empty() {
                            args.extend(["--name".into(), state.create_name.trim().to_string()]);
                        }
                        args.extend(["--length".into(), format!("{:.1}", state.create_length)]);
                        runner.start(ctx, "preset create", args, false);
                    }
                }
                if ui.button("İçe Aktar (xml/zip/prfpset)").clicked() && !runner.running {
                    if let Some(files) = rfd::FileDialog::new()
                        .add_filter("Preset", &["xml", "zip", "prfpset", "json"])
                        .pick_files()
                    {
                        let mut args = vec!["preset".into(), "import".into()];
                        args.extend(files.into_iter().map(|f| f.to_string_lossy().into_owned()));
                        runner.start(ctx, "preset import", args, false);
                    }
                }
            });
        });
    });

    ui.add_space(8.0);

    ui.columns(2, |cols| {
        // Left: Filter & Preset list
        card_frame(p).show(&mut cols[0], |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Ara:").color(p.muted));
                ui.add(TextEdit::singleline(&mut state.search).hint_text("spin, anime, velocity...").desired_width(200.0));
                if !state.search.is_empty() && ui.button("×").clicked() {
                    state.search.clear();
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                if toggle_chip(ui, p, state.selected_tag.is_none(), "Tümü").clicked() {
                    state.selected_tag = None;
                }
                for tag in state.all_tags() {
                    let on = state.selected_tag.as_deref() == Some(tag.as_str());
                    if toggle_chip(ui, p, on, &tag).clicked() {
                        state.selected_tag = if on { None } else { Some(tag) };
                    }
                }
            });
            ui.add_space(6.0);

            let q = state.search.to_lowercase();
            let mut next_pick: Option<usize> = None;

            ScrollArea::vertical().id_salt("preset_list_scroll").show(ui, |ui| {
                for (idx, preset) in state.presets.iter().enumerate() {
                    let tags = preset.tags();
                    if let Some(tag) = &state.selected_tag {
                        if !tags.contains(&tag.as_str()) {
                            continue;
                        }
                    }
                    if !q.is_empty()
                        && !preset.name.to_lowercase().contains(&q)
                        && !preset.about.to_lowercase().contains(&q)
                        && !tags.iter().any(|t| t.contains(&q))
                    {
                        continue;
                    }

                    let selected = idx == state.selected_index;
                    let fill = if selected {
                        if p.dark { Color32::from_rgb(0x24, 0x17, 0x18) } else { Color32::from_rgb(0xF6, 0xE1, 0xDF) }
                    } else {
                        p.input
                    };
                    let stroke = if selected { Stroke::new(1.5_f32, CUT_RED) } else { Stroke::new(1.0_f32, p.line) };

                    let resp = egui::Frame::new()
                        .fill(fill)
                        .stroke(stroke)
                        .corner_radius(6)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&preset.name).strong().color(if selected { CUT_RED } else { p.text }));
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{} shot · {:.1}s · {}x{}", preset.slots.len(), preset.total, preset.w, preset.h))
                                            .font(FontId::monospace(11.0))
                                            .color(p.muted),
                                    );
                                });
                            });
                            if !preset.about.is_empty() {
                                ui.label(RichText::new(&preset.about).size(12.0).color(p.muted));
                            }
                            ui.label(RichText::new(tags.join(" · ")).size(11.0).color(WARM_GRAY));
                        });
                    if resp.response.interact(Sense::click()).clicked() {
                        next_pick = Some(idx);
                    }
                    ui.add_space(4.0);
                }
            });

            if let Some(idx) = next_pick {
                state.selected_index = idx;
                state.sync_json();
                state.feedback.clear();
            }
        });

        // Right: Visual Timeline, Curve Plotter & JSON Editor
        ScrollArea::vertical().id_salt("preset_detail_scroll").show(&mut cols[1], |ui| {
            let Some(preset) = state.presets.get(state.selected_index).cloned() else {
                ui.label("Preset seçilmedi.");
                return;
            };

            card_frame(p).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&preset.name).font(FontId::proportional(18.0)).strong().color(p.text));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(cta_button("Make Sekmesinde Kullan")).clicked() {
                            action = PresetsAction::UseInMake(preset.name.clone());
                        }
                    });
                });

                if !preset.about.is_empty() {
                    ui.label(RichText::new(&preset.about).size(12.5).color(p.muted));
                }

                let song_label = preset
                    .song
                    .as_ref()
                    .map(|s| format!("{} (@{:.1}s)", s.title, s.from))
                    .unwrap_or_else(|| "Belirtilmemiş".into());
                let look_label = preset.look.as_deref().unwrap_or("Otomatik / Yok");
                ui.label(
                    RichText::new(format!(
                        "Boyut: {}x{}  |  Süre: {:.2}s  |  Look: {look_label}  |  Şarkı: {song_label}",
                        preset.w, preset.h, preset.total
                    ))
                    .font(FontId::monospace(11.5))
                    .color(p.muted),
                );

                ui.add_space(8.0);
                section_heading(ui, p, "Shot Zaman Çizelgesi", Some("Bir shot'a tıklayarak eğrilerini inceleyin"));
                draw_timeline(ui, p, &preset, &mut state.selected_slot);

                ui.add_space(8.0);
                if let Some(slot) = preset.slots.get(state.selected_slot) {
                    ui.label(
                        RichText::new(format!(
                            "Shot #{}: {:.2}s -> {:.2}s (hız {:.2}x{})",
                            state.selected_slot + 1,
                            slot.start,
                            slot.end,
                            slot.speed,
                            if slot.drop { ", DROP" } else { "" }
                        ))
                        .font(FontId::monospace(11.5))
                        .color(p.text),
                    );
                    draw_curves(ui, p, slot);
                }
            });

            ui.add_space(10.0);

            card_frame(p).show(ui, |ui| {
                ui.horizontal(|ui| {
                    section_heading(ui, p, "JSON Düzenleyici", Some("Kaydedilen preset built-in olanın üzerine yazar"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(cta_button("Kaydet")).clicked() {
                            match serde_json::from_str::<Preset>(&state.json_buf) {
                                Ok(parsed) if parsed.slots.is_empty() => {
                                    state.feedback = "En az 1 shot gerekli.".into();
                                    state.feedback_err = true;
                                }
                                Ok(parsed) => {
                                    let dir = edits::presets::user_dir();
                                    if let Err(e) = std::fs::create_dir_all(&dir) {
                                        state.feedback = format!("Klasör hatası: {e}");
                                        state.feedback_err = true;
                                    } else {
                                        let to = dir.join(format!("{}.json", edits::preset::slug(&parsed.name)));
                                        let pretty = edits::preset::pretty(&parsed);
                                        match std::fs::write(&to, &pretty) {
                                            Ok(()) => {
                                                state.feedback = format!("Kaydedildi: {}", to.display());
                                                state.feedback_err = false;
                                                state.json_dirty = false;
                                                state.reload();
                                                action = PresetsAction::CatalogChanged;
                                            }
                                            Err(e) => {
                                                state.feedback = format!("Yazma hatası: {e}");
                                                state.feedback_err = true;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    state.feedback = format!("Geçersiz JSON: {e}");
                                    state.feedback_err = true;
                                }
                            }
                        }
                        if state.json_dirty && ui.button("Geri Al").clicked() {
                            state.sync_json();
                        }
                    });
                });

                if !state.feedback.is_empty() {
                    ui.label(
                        RichText::new(&state.feedback)
                            .size(12.0)
                            .color(if state.feedback_err { CUT_RED } else { OK_GREEN }),
                    );
                }

                ui.add_space(4.0);
                ScrollArea::vertical().id_salt("preset_json_scroll").max_height(280.0).show(ui, |ui| {
                    let resp = ui.add(
                        TextEdit::multiline(&mut state.json_buf)
                            .font(FontId::monospace(11.5))
                            .desired_width(f32::INFINITY)
                            .desired_rows(14),
                    );
                    if resp.changed() {
                        state.json_dirty = true;
                    }
                });
            });
        });
    });

    action
}

fn draw_timeline(ui: &mut egui::Ui, p: Palette, preset: &Preset, selected_slot: &mut usize) {
    let total = preset.total.max(0.1) as f32;
    let width = ui.available_width().max(120.0);
    let height = 38.0;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 4.0, p.input);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, p.line), egui::StrokeKind::Inside);

    for (i, s) in preset.slots.iter().enumerate() {
        let x0 = rect.left() + (s.start as f32 / total).clamp(0.0, 1.0) * rect.width();
        let x1 = rect.left() + (s.end as f32 / total).clamp(0.0, 1.0) * rect.width();
        let seg = Rect::from_min_max(Pos2::new(x0 + 1.0, rect.top() + 3.0), Pos2::new((x1 - 1.0).max(x0 + 3.0), rect.bottom() - 3.0));
        let fill = if i == *selected_slot {
            CUT_RED
        } else if s.drop {
            Color32::from_rgb(0x88, 0x29, 0x25)
        } else if p.dark {
            Color32::from_rgb(0x2B, 0x2B, 0x30)
        } else {
            Color32::from_rgb(0xD4, 0xCE, 0xC2)
        };
        painter.rect_filled(seg, 3.0, fill);

        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                if seg.contains(pos) {
                    *selected_slot = i;
                }
            }
        }
    }

    // Flash markers as thin vertical ticks at top
    for fl in &preset.flashes {
        let fx = rect.left() + (fl.start as f32 / total).clamp(0.0, 1.0) * rect.width();
        painter.line_segment([Pos2::new(fx, rect.top()), Pos2::new(fx, rect.top() + 7.0)], Stroke::new(2.0_f32, PAPER));
    }
}

fn draw_curves(ui: &mut egui::Ui, p: Palette, slot: &edits::preset::Slot) {
    let curves: Vec<(&str, &Curve, Color32)> = vec![
        ("zoom", &slot.zoom, CUT_RED),
        ("x", &slot.x, Color32::from_rgb(0x52, 0xB7, 0x88)),
        ("y", &slot.y, Color32::from_rgb(0x4E, 0xA8, 0xDE)),
        ("rot", &slot.rot, Color32::from_rgb(0xF4, 0xA2, 0x61)),
        ("shake", &slot.shake, Color32::from_rgb(0xE9, 0xC4, 0x6A)),
        ("blur", &slot.blur, Color32::from_rgb(0xB5, 0x83, 0x8D)),
        ("ramp", &slot.ramp, Color32::from_rgb(0xA8, 0xDA, 0xDC)),
    ];

    let active: Vec<(&str, &Curve, Color32)> = curves.into_iter().filter(|(_, c, _)| !c.is_empty()).collect();
    if active.is_empty() {
        ui.label(RichText::new("Bu shot'ta animasyon eğrisi yok (sabit çerçeve).").size(12.0).color(p.muted));
        return;
    }

    ui.horizontal_wrapped(|ui| {
        for (name, _, color) in &active {
            ui.label(RichText::new(format!("■ {name}")).size(11.5).color(*color));
        }
    });

    let width = ui.available_width().max(120.0);
    let height = 96.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, p.input);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, p.line), egui::StrokeKind::Inside);

    for (_, curve, color) in active {
        let min_v = curve.min();
        let max_v = curve.max();
        let span = (max_v - min_v).abs().max(0.01);
        let samples = 36usize;
        let pts: Vec<Pos2> = (0..=samples)
            .map(|i| {
                let u = i as f64 / samples as f64;
                let v = curve.at(u);
                let norm = ((v - min_v) / span) as f32;
                Pos2::new(
                    rect.left() + 6.0 + u as f32 * (rect.width() - 12.0),
                    rect.bottom() - 8.0 - norm * (rect.height() - 16.0),
                )
            })
            .collect();
        painter.add(egui::Shape::line(pts, Stroke::new(1.8_f32, color)));
    }
}
