use crate::runner::{open_in_os, Runner};
use crate::theme::{card_frame, cta_button, section_heading, toggle_chip, Palette, CUT_RED};
use eframe::egui::{self, FontId, RichText, ScrollArea, Stroke, TextEdit};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

pub struct LibraryState {
    // Trends
    pub country: String,
    pub trends: Vec<String>,
    pub trends_loading: bool,
    pub trends_error: String,
    trends_rx: Option<Receiver<Result<Vec<String>, String>>>,

    // Asset library
    pub items: Vec<(&'static str, String, PathBuf)>,
    pub kind_filter: Option<String>,
    pub search: String,
}

impl LibraryState {
    pub fn new() -> Self {
        let mut s = Self {
            country: edits::lang::system().1,
            trends: Vec::new(),
            trends_loading: false,
            trends_error: String::new(),
            trends_rx: None,
            items: Vec::new(),
            kind_filter: None,
            search: String::new(),
        };
        s.reload_library();
        s
    }

    pub fn reload_library(&mut self) {
        self.items = edits::library::inventory();
    }

    pub fn poll(&mut self) {
        let Some(rx) = self.trends_rx.as_ref() else { return };
        if let Ok(res) = rx.try_recv() {
            self.trends_loading = false;
            match res {
                Ok(list) => {
                    self.trends = list;
                    self.trends_error.clear();
                }
                Err(e) => {
                    self.trends_error = e;
                }
            }
            self.trends_rx = None;
        }
    }

    fn fetch_trends(&mut self, ctx: &egui::Context) {
        if self.trends_loading {
            return;
        }
        self.trends_loading = true;
        self.trends_error.clear();
        let cc = self.country.clone();
        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.trends_rx = Some(rx);
        std::thread::spawn(move || {
            let res = edits::fetch::trends(&cc);
            let _ = tx.send(res);
            ctx.request_repaint();
        });
    }
}

pub enum LibraryAction {
    None,
    UseMusic(String),
    UseBed(String),
    AddOverlay(String),
    UseInput(String),
}

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    p: Palette,
    state: &mut LibraryState,
    runner: &mut Runner,
) -> LibraryAction {
    state.poll();
    let mut action = LibraryAction::None;

    ui.columns(2, |cols| {
        // Left: TikTok Trends
        card_frame(p).show(&mut cols[0], |ui| {
            ui.horizontal(|ui| {
                section_heading(ui, p, "TikTok Trend Müzikleri", Some("trend:N olarak kullanılır"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add_enabled(!state.trends_loading, cta_button("Trendleri Getir")).clicked() {
                        state.fetch_trends(ctx);
                    }
                    ui.add(TextEdit::singleline(&mut state.country).desired_width(36.0));
                    ui.label(RichText::new("Ülke:").color(p.muted));
                });
            });

            if state.trends_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("kworb.net üzerinden çekiliyor...").color(p.muted));
                });
            }
            if !state.trends_error.is_empty() {
                ui.label(RichText::new(&state.trends_error).size(12.0).color(CUT_RED));
            }
            ui.add_space(6.0);

            ScrollArea::vertical().id_salt("trends_scroll").show(ui, |ui| {
                if state.trends.is_empty() && !state.trends_loading {
                    ui.label(
                        RichText::new("Ülke kodu (ör. tr, us, gb) girip 'Trendleri Getir'e basın.")
                            .size(12.5)
                            .color(p.muted),
                    );
                }
                for (idx, song) in state.trends.iter().take(30).enumerate() {
                    let rank = idx + 1;
                    let spec = if state.country.trim().eq_ignore_ascii_case(&edits::lang::system().1) {
                        format!("trend:{rank}")
                    } else {
                        format!("trend:{}:{rank}", state.country.trim().to_lowercase())
                    };
                    egui::Frame::new()
                        .fill(p.input)
                        .stroke(Stroke::new(1.0_f32, p.line))
                        .corner_radius(6)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("{rank:>2}.")).font(FontId::monospace(12.0)).color(CUT_RED));
                                ui.label(RichText::new(song).color(p.text));
                            });
                            ui.horizontal(|ui| {
                                if ui.button("Müzik Yap").clicked() {
                                    action = LibraryAction::UseMusic(spec.clone());
                                }
                                if ui.button("Arka Fon (--bed)").clicked() {
                                    action = LibraryAction::UseBed(spec);
                                }
                            });
                        });
                    ui.add_space(4.0);
                }
            });
        });

        // Right: Asset Library
        card_frame(p).show(&mut cols[1], |ui| {
            ui.horizontal_wrapped(|ui| {
                section_heading(
                    ui,
                    p,
                    "Varlık Kütüphanesi",
                    Some(&format!("{} dosya", state.items.len())),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Yenile").clicked() {
                        state.reload_library();
                    }
                    if ui.button("Klasörü Aç").clicked() {
                        let d = edits::library::dir();
                        let _ = std::fs::create_dir_all(&d);
                        open_in_os(&d);
                    }
                    if ui.button("Paket Ekle (.zip)").clicked() && !runner.running {
                        if let Some(file) = rfd::FileDialog::new().add_filter("Paket", &["zip"]).pick_file() {
                            runner.start(
                                ctx,
                                "library add",
                                vec!["library".into(), "add".into(), file.to_string_lossy().into_owned()],
                                false,
                            );
                        }
                    }
                    if ui.button("Klasör Ekle").clicked() && !runner.running {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            runner.start(
                                ctx,
                                "library add",
                                vec!["library".into(), "add".into(), dir.to_string_lossy().into_owned()],
                                false,
                            );
                        }
                    }
                });
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Ara:").color(p.muted));
                ui.add(TextEdit::singleline(&mut state.search).hint_text("whoosh, vhs, burn...").desired_width(180.0));
            });

            let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
            for (k, _, _) in &state.items {
                *counts.entry(*k).or_default() += 1;
            }
            ui.horizontal_wrapped(|ui| {
                if toggle_chip(ui, p, state.kind_filter.is_none(), "Tümü").clicked() {
                    state.kind_filter = None;
                }
                for (k, n) in counts {
                    let on = state.kind_filter.as_deref() == Some(k);
                    if toggle_chip(ui, p, on, &format!("{k} ({n})")).clicked() {
                        state.kind_filter = if on { None } else { Some(k.to_string()) };
                    }
                }
            });

            ui.add_space(6.0);
            let q = state.search.to_lowercase();

            ScrollArea::vertical().id_salt("library_items_scroll").show(ui, |ui| {
                if state.items.is_empty() {
                    ui.label(
                        RichText::new(format!(
                            "Kütüphane boş ({}). Paket eklemek için yukarıdaki butonları kullanın.",
                            edits::library::dir().display()
                        ))
                        .size(12.5)
                        .color(p.muted),
                    );
                }

                for (k, rel, full) in &state.items {
                    if let Some(kf) = &state.kind_filter {
                        if *k != kf {
                            continue;
                        }
                    }
                    if !q.is_empty() && !rel.to_lowercase().contains(&q) {
                        continue;
                    }
                    let stem = full.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    egui::Frame::new()
                        .fill(p.input)
                        .stroke(Stroke::new(1.0_f32, p.line))
                        .corner_radius(6)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(*k).font(FontId::monospace(11.0)).color(CUT_RED));
                                ui.label(RichText::new(rel).color(p.text));
                            });
                            ui.horizontal(|ui| {
                                match *k {
                                    "music" => {
                                        if ui.button("Müzik Yap").clicked() {
                                            action = LibraryAction::UseMusic(stem.clone());
                                        }
                                    }
                                    "overlay" | "transition" | "greenscreen" => {
                                        if ui.button("Overlay Ekle").clicked() {
                                            action = LibraryAction::AddOverlay(stem.clone());
                                        }
                                    }
                                    "meme" | "background" | "video"
                                        if ui.button("Kaynak Yap").clicked() =>
                                    {
                                        action = LibraryAction::UseInput(full.to_string_lossy().into_owned());
                                    }
                                    _ => {}
                                }
                                if ui.button("Aç").clicked() {
                                    open_in_os(full);
                                }
                            });
                        });
                    ui.add_space(4.0);
                }
            });
        });
    });

    action
}
