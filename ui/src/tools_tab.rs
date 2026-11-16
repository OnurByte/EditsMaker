use crate::runner::{open_in_os, Runner};
use crate::theme::{card_frame, cta_button, section_heading, Palette, CUT_RED, OK_GREEN, WARN_AMBER};
use eframe::egui::{self, ColorImage, FontId, RichText, ScrollArea, Stroke, TextEdit, TextureHandle, TextureOptions};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};

pub struct DoctorItem {
    pub name: String,
    pub ok: bool,
    pub required: bool,
    pub detail: String,
}

type CheckResult = Result<(String, Option<PathBuf>), String>;

pub struct ToolsState {
    // Check clip
    pub check_clip_path: String,
    pub check_running: bool,
    pub check_report: String,
    pub check_error: String,
    pub sheet_texture: Option<TextureHandle>,
    pub sheet_path: Option<PathBuf>,
    check_rx: Option<Receiver<CheckResult>>,

    // Doctor
    pub doctor_items: Vec<DoctorItem>,

    // Taste
    pub taste_buf: String,
    pub taste_status: String,

    // Style defaults
    pub studied_style: Option<edits::study::Style>,
}

impl ToolsState {
    pub fn new() -> Self {
        let mut s = Self {
            check_clip_path: String::new(),
            check_running: false,
            check_report: String::new(),
            check_error: String::new(),
            sheet_texture: None,
            sheet_path: None,
            check_rx: None,
            doctor_items: Vec::new(),
            taste_buf: edits::pick::taste(),
            taste_status: String::new(),
            studied_style: edits::study::load(),
        };
        s.refresh_doctor();
        s
    }

    pub fn refresh_doctor(&mut self) {
        let have_ff = edits::util::have("ffmpeg") && edits::util::have("ffprobe");
        let libass = have_ff
            && edits::util::run(None, "ffmpeg", &["-hide_banner", "-filters"]).is_ok_and(|f| f.contains(" ass "));
        let have_yt = edits::util::have("yt-dlp");
        let have_whisper = edits::util::have("whisper-cli");
        let model = edits::transcribe::model();
        let llm_cmd = edits::pick::llm_cmd();
        let llm_bin = llm_cmd.split_whitespace().next().unwrap_or("");
        let have_llm = !llm_bin.is_empty() && edits::util::have(llm_bin);

        self.doctor_items = vec![
            DoctorItem {
                name: "ffmpeg + ffprobe".into(),
                ok: have_ff,
                required: true,
                detail: if have_ff {
                    "Kurulu".into()
                } else {
                    "Eksik — 'Araçları Kur (setup)' ile otomatik indirebilirsiniz".into()
                },
            },
            DoctorItem {
                name: "ffmpeg libass (altyazı)".into(),
                ok: libass,
                required: true,
                detail: if libass { "Hazır".into() } else { "Tam ffmpeg derlemesi gerekir".into() },
            },
            DoctorItem {
                name: "yt-dlp (YouTube & 1700+ site)".into(),
                ok: have_yt,
                required: false,
                detail: if have_yt {
                    "Kurulu".into()
                } else {
                    "Eksik (yerel dosyalar çalışır, linkler için setup gerekir)".into()
                },
            },
            DoctorItem {
                name: "whisper-cli".into(),
                ok: have_whisper,
                required: false,
                detail: if have_whisper {
                    "Kurulu".into()
                } else {
                    "Opsiyonel — altyazısız videolarda konuşma dökümü için".into()
                },
            },
            DoctorItem {
                name: "Whisper modeli (ggml-*.bin)".into(),
                ok: model.is_some(),
                required: false,
                detail: model
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "~/.local/share/edits/models veya EDITS_WHISPER_MODEL".into()),
            },
            DoctorItem {
                name: format!("LLM komutu ({llm_cmd})"),
                ok: have_llm,
                required: false,
                detail: if have_llm {
                    "Hazır".into()
                } else {
                    "Opsiyonel — yoksa yalnızca izleyici sinyalleri kullanılır".into()
                },
            },
        ];
    }

    pub fn run_check(&mut self, ctx: &egui::Context, clip: PathBuf) {
        if self.check_running {
            return;
        }
        self.check_clip_path = clip.to_string_lossy().into_owned();
        self.check_running = true;
        self.check_report.clear();
        self.check_error.clear();
        self.sheet_texture = None;
        self.sheet_path = None;

        let ctx = ctx.clone();
        let (tx, rx) = mpsc::channel();
        self.check_rx = Some(rx);

        std::thread::spawn(move || {
            let res = edits::check::check(&clip).map(|report| {
                let sheet = report
                    .lines()
                    .find_map(|l| l.strip_prefix("sheet: "))
                    .map(|p| PathBuf::from(p.trim()));
                (report, sheet)
            });
            let _ = tx.send(res);
            ctx.request_repaint();
        });
    }

    pub fn poll(&mut self, ctx: &egui::Context) {
        let Some(rx) = self.check_rx.as_ref() else { return };
        if let Ok(res) = rx.try_recv() {
            self.check_running = false;
            match res {
                Ok((report, sheet)) => {
                    self.check_report = report;
                    if let Some(sp) = sheet {
                        self.sheet_texture = load_image_texture(ctx, &sp);
                        self.sheet_path = Some(sp);
                    }
                }
                Err(e) => {
                    self.check_error = e;
                }
            }
            self.check_rx = None;
        }
    }
}

fn load_image_texture(ctx: &egui::Context, path: &PathBuf) -> Option<TextureHandle> {
    let img = image::open(path).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color_img = ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(format!("sheet_{}", path.display()), color_img, TextureOptions::LINEAR))
}

pub fn show(ui: &mut egui::Ui, ctx: &egui::Context, p: Palette, state: &mut ToolsState, runner: &mut Runner) {
    state.poll(ctx);

    ScrollArea::vertical().id_salt("tools_scroll").show(ui, |ui| {
        ui.columns(2, |cols| {
            // Left: System Doctor & Study defaults
            card_frame(p).show(&mut cols[0], |ui| {
                ui.horizontal(|ui| {
                    section_heading(ui, p, "Sistem Doktoru (edits doctor)", None);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Yenile").clicked() {
                            edits::tools::add_to_path();
                            state.refresh_doctor();
                        }
                        if ui.add_enabled(!runner.running, cta_button("Araçları Kur (setup)")).clicked() {
                            runner.start(ctx, "setup", vec!["setup".into()], false);
                        }
                    });
                });
                ui.add_space(6.0);

                for item in &state.doctor_items {
                    egui::Frame::new()
                        .fill(p.input)
                        .stroke(Stroke::new(1.0_f32, p.line))
                        .corner_radius(6)
                        .inner_margin(8.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let (badge, color) = if item.ok {
                                    ("OK", OK_GREEN)
                                } else if item.required {
                                    ("EKSİK", CUT_RED)
                                } else {
                                    ("UYARI", WARN_AMBER)
                                };
                                ui.label(RichText::new(format!("[{badge}]")).font(FontId::monospace(11.5)).strong().color(color));
                                ui.label(RichText::new(&item.name).strong().color(p.text));
                            });
                            ui.label(RichText::new(&item.detail).size(11.5).color(p.muted));
                        });
                    ui.add_space(4.0);
                }

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    section_heading(ui, p, "Stil Varsayılanları (edits study)", None);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(!runner.running, egui::Button::new("Klasör İncele (study)")).clicked() {
                            if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                                runner.start(ctx, "study", vec!["study".into(), dir.to_string_lossy().into_owned()], false);
                            }
                        }
                    });
                });
                if let Some(st) = &state.studied_style {
                    ui.label(
                        RichText::new(format!(
                            "aspect: {}  |  grade: {}  |  LUFS: {:.1}  |  sfx: {}  |  hook: {}",
                            st.aspect, st.grade, st.lufs, st.sfx, st.hook
                        ))
                        .font(FontId::monospace(11.5))
                        .color(p.text),
                    );
                } else {
                    ui.label(RichText::new("Henüz study çalıştırılmadı (varsayılan: 9:16, bw, -14 LUFS).").size(12.0).color(p.muted));
                }
            });

            // Right: Taste editor
            card_frame(p).show(&mut cols[1], |ui| {
                ui.horizontal(|ui| {
                    section_heading(ui, p, "Zevk Profili (taste.md)", Some("An seçimi için AI yönergesi"));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(cta_button("Kaydet")).clicked() {
                            let path = edits::pick::taste_path();
                            if let Some(dir) = path.parent() {
                                let _ = std::fs::create_dir_all(dir);
                            }
                            match std::fs::write(&path, &state.taste_buf) {
                                Ok(()) => state.taste_status = format!("Kaydedildi: {}", path.display()),
                                Err(e) => state.taste_status = format!("Hata: {e}"),
                            }
                        }
                    });
                });
                if !state.taste_status.is_empty() {
                    ui.label(RichText::new(&state.taste_status).size(11.5).color(OK_GREEN));
                }
                ui.add_space(4.0);
                ScrollArea::vertical().id_salt("taste_scroll").max_height(310.0).show(ui, |ui| {
                    ui.add(
                        TextEdit::multiline(&mut state.taste_buf)
                            .font(FontId::monospace(11.5))
                            .desired_width(f32::INFINITY)
                            .desired_rows(16),
                    );
                });
            });
        });

        ui.add_space(10.0);

        // Clip Inspector (edits check)
        card_frame(p).show(ui, |ui| {
            section_heading(ui, p, "Klip Kalite Kontrolü (edits check)", Some("Süre, LUFS, kesim ritmi ve contact sheet"));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add(
                    TextEdit::singleline(&mut state.check_clip_path)
                        .hint_text("İncelenecek .mp4 klip yolu...")
                        .desired_width(ui.available_width() - 210.0),
                );
                if ui.button("Klip Seç").clicked() {
                    if let Some(f) = rfd::FileDialog::new().add_filter("Video", &["mp4", "mov", "mkv"]).pick_file() {
                        state.run_check(ctx, f);
                    }
                }
                if ui.add_enabled(!state.check_running && !state.check_clip_path.trim().is_empty(), cta_button("Ölç")).clicked() {
                    state.run_check(ctx, PathBuf::from(state.check_clip_path.trim()));
                }
            });

            if state.check_running {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new("ffprobe ve contact sheet oluşturuluyor...").color(p.muted));
                });
            }
            if !state.check_error.is_empty() {
                ui.label(RichText::new(&state.check_error).color(CUT_RED));
            }
            if !state.check_report.is_empty() {
                ui.add_space(6.0);
                egui::Frame::new()
                    .fill(p.input)
                    .stroke(Stroke::new(1.0_f32, p.line))
                    .corner_radius(6)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        ui.label(RichText::new(&state.check_report).font(FontId::monospace(12.0)).color(p.text));
                    });
            }
            if let Some(tex) = &state.sheet_texture {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Contact Sheet (10x2 kare):").strong().color(p.text));
                    if let Some(sp) = &state.sheet_path {
                        if ui.button("Görseli Aç").clicked() {
                            open_in_os(sp);
                        }
                    }
                });
                let max_w = ui.available_width().max(200.0);
                let sz = tex.size_vec2();
                let scale = (max_w / sz.x.max(1.0)).min(1.0);
                ui.image((tex.id(), sz * scale));
            }
        });
    });
}
