use crate::runner::{format_args_preview, open_in_os, Runner};
use crate::theme::{card_frame, cta_button, section_heading, toggle_chip, Palette, CUT_RED, OK_GREEN, PAPER};
use eframe::egui::{self, FontId, RichText, ScrollArea, Stroke, TextEdit};
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EditMode {
    Preset,
    Music,
    Talking,
    Reference,
}

impl EditMode {
    pub fn label(self) -> &'static str {
        match self {
            EditMode::Preset => "Preset",
            EditMode::Music => "Müzik Edit",
            EditMode::Talking => "Konuşma / Klip",
            EditMode::Reference => "Referans (--like)",
        }
    }
}

pub struct MakeState {
    pub mode: EditMode,
    pub input: String,

    // Preset / Music / Talking / Reference specifics
    pub preset: String,
    pub preset_filter: String,
    pub intensity_override: bool,
    pub intensity: f64,

    pub music: String,
    pub like_video: String,
    pub sad: bool,
    pub voice: bool,
    pub length_override: bool,
    pub length: f64,
    pub transition: String,

    // Talking options
    pub caption_style: String,
    pub bed: String,
    pub no_jumpcuts: bool,
    pub no_sfx: bool,
    pub no_hook: bool,
    pub no_captions: bool,
    pub whoosh: String,
    pub boom: String,

    // Look & Frame
    pub aspect: String,
    pub custom_size: String,
    pub fit: bool,
    pub look: String,
    pub lut_path: String,
    pub fix_levels: bool,

    // Motion & Effects
    pub speed: f64,
    pub velocity: bool,
    pub fx_hits: BTreeSet<String>,
    pub blur_faces: bool,
    pub blur_box: String,

    // Words, Overlays & Memes
    pub lines_count: usize,
    pub line_texts: String,
    pub say_specs: String,
    pub say_text: bool,
    pub shot_specs: String,
    pub captions_at: String,
    pub flash_texts: String,
    pub svgs: String,
    pub anim: String,
    pub overlays: String,
    pub pips: String,
    pub math: bool,
    pub intro: String,
    pub intro_bed: String,
    pub intro_caption: String,
    pub outro: String,
    pub tag: String,
    pub text_banner: String,

    // Finish & Output
    pub n: usize,
    pub videos: usize,
    pub use_videos: usize,
    pub lang: String,
    pub out_dir: String,
    pub join: bool,
    pub no_llm: bool,
    pub curves: String,
    pub pop_color: String,
    pub bass: f64,
    pub pitch: f64,
    pub reverb: bool,
    pub stabilize: bool,
    pub denoise: bool,
    pub smooth: bool,
    pub fps: String,
    pub codec: String,
    pub quality: String,
    pub encoder: String,
    pub export_formats: BTreeSet<String>,

    // UI caches
    pub preset_names: Vec<String>,
    pub look_options: Vec<(String, String)>,
    pub caption_names: Vec<String>,
    pub status_toast: String,
}

impl MakeState {
    pub fn new() -> Self {
        let preset_names: Vec<String> = edits::presets::all().into_iter().map(|p| p.name).collect();
        let default_preset = preset_names
            .iter()
            .find(|n| *n == "spin-slide-shake")
            .cloned()
            .or_else(|| preset_names.first().cloned())
            .unwrap_or_default();

        let mut look_options: Vec<(String, String)> = edits::looks::LOOKS
            .iter()
            .map(|(n, about, _)| ((*n).to_string(), (*about).to_string()))
            .collect();
        for g in edits::grades::all() {
            look_options.push((g.name, g.about));
        }

        let caption_names: Vec<String> = edits::talk::styles().into_iter().map(|s| s.name).collect();
        let studied = edits::study::load();
        let default_aspect = studied.as_ref().map(|s| s.aspect.clone()).unwrap_or_else(|| "9:16".into());
        let default_look = studied
            .as_ref()
            .filter(|s| look_options.iter().any(|(n, _)| n == &s.grade))
            .map(|s| s.grade.clone())
            .unwrap_or_else(|| "bw".into());

        Self {
            mode: EditMode::Preset,
            input: String::new(),
            preset: default_preset,
            preset_filter: String::new(),
            intensity_override: false,
            intensity: 1.0,
            music: String::new(),
            like_video: String::new(),
            sad: false,
            voice: false,
            length_override: false,
            length: 18.0,
            transition: String::new(),
            caption_style: caption_names
                .iter()
                .find(|n| *n == "hormozi")
                .cloned()
                .unwrap_or_else(|| "hormozi".into()),
            bed: String::new(),
            no_jumpcuts: false,
            no_sfx: false,
            no_hook: false,
            no_captions: false,
            whoosh: String::new(),
            boom: String::new(),
            aspect: default_aspect,
            custom_size: String::new(),
            fit: false,
            look: default_look,
            lut_path: String::new(),
            fix_levels: false,
            speed: 1.0,
            velocity: false,
            fx_hits: BTreeSet::new(),
            blur_faces: false,
            blur_box: String::new(),
            lines_count: 0,
            line_texts: String::new(),
            say_specs: String::new(),
            say_text: false,
            shot_specs: String::new(),
            captions_at: String::new(),
            flash_texts: String::new(),
            svgs: String::new(),
            anim: String::new(),
            overlays: String::new(),
            pips: String::new(),
            math: false,
            intro: String::new(),
            intro_bed: String::new(),
            intro_caption: String::new(),
            outro: String::new(),
            tag: String::new(),
            text_banner: String::new(),
            n: 3,
            videos: 5,
            use_videos: 4,
            lang: edits::lang::system().0,
            out_dir: String::new(),
            join: false,
            no_llm: false,
            curves: String::new(),
            pop_color: String::new(),
            bass: 0.0,
            pitch: 0.0,
            reverb: false,
            stabilize: false,
            denoise: false,
            smooth: false,
            fps: String::new(),
            codec: "h264".into(),
            quality: "high".into(),
            encoder: "auto".into(),
            export_formats: BTreeSet::new(),
            preset_names,
            look_options,
            caption_names,
            status_toast: String::new(),
        }
    }

    pub fn reload_catalogs(&mut self) {
        self.preset_names = edits::presets::all().into_iter().map(|p| p.name).collect();
        let mut looks: Vec<(String, String)> = edits::looks::LOOKS
            .iter()
            .map(|(n, about, _)| ((*n).to_string(), (*about).to_string()))
            .collect();
        for g in edits::grades::all() {
            looks.push((g.name, g.about));
        }
        self.look_options = looks;
    }

    pub fn build_args(&self) -> Result<Vec<String>, String> {
        let input = self.input.trim();
        if input.is_empty() {
            return Err("Lütfen bir YouTube linki, kanal, konu veya video dosyası girin.".into());
        }
        let mut args = vec![input.to_string()];

        match self.mode {
            EditMode::Preset => {
                if !self.preset.trim().is_empty() {
                    args.extend(["--preset".into(), self.preset.trim().to_string()]);
                }
                if self.intensity_override {
                    args.extend(["--intensity".into(), format!("{:.2}", self.intensity)]);
                }
                if !self.music.trim().is_empty() {
                    args.extend(["--music".into(), self.music.trim().to_string()]);
                }
                for s in split_lines(&self.shot_specs) {
                    args.extend(["--shot".into(), s]);
                }
            }
            EditMode::Music => {
                if self.music.trim().is_empty() {
                    return Err("Müzik edit modu için şarkı, dosya yolu veya trend:N girin.".into());
                }
                args.extend(["--music".into(), self.music.trim().to_string()]);
                if self.sad {
                    args.push("--sad".into());
                }
                if self.voice {
                    args.push("--voice".into());
                }
                if self.length_override {
                    args.extend(["--length".into(), format!("{:.1}", self.length)]);
                }
                if !self.transition.trim().is_empty() {
                    args.extend(["--transition".into(), self.transition.trim().to_string()]);
                }
            }
            EditMode::Talking => {
                args.extend(["--captions".into(), self.caption_style.clone()]);
                if !self.bed.trim().is_empty() {
                    args.extend(["--bed".into(), self.bed.trim().to_string()]);
                }
                if self.no_jumpcuts {
                    args.push("--no-jumpcuts".into());
                }
                if self.no_sfx {
                    args.push("--no-sfx".into());
                }
                if self.no_hook {
                    args.push("--no-hook".into());
                }
                if self.no_captions {
                    args.push("--no-captions".into());
                }
                if !self.whoosh.trim().is_empty() {
                    args.extend(["--whoosh".into(), self.whoosh.trim().to_string()]);
                }
                if !self.boom.trim().is_empty() {
                    args.extend(["--boom".into(), self.boom.trim().to_string()]);
                }
            }
            EditMode::Reference => {
                if self.like_video.trim().is_empty() {
                    return Err("Referans modu için örnek bir .mp4 edit dosyası seçin.".into());
                }
                args.extend(["--like".into(), self.like_video.trim().to_string()]);
                if self.voice {
                    args.push("--voice".into());
                }
            }
        }

        // Frame & Look
        if !self.custom_size.trim().is_empty() {
            args.extend(["--size".into(), self.custom_size.trim().to_string()]);
        } else if !self.aspect.is_empty() {
            args.extend(["--aspect".into(), self.aspect.clone()]);
        }
        if self.fit {
            args.push("--fit".into());
        }
        if !self.look.is_empty() {
            args.extend(["--look".into(), self.look.clone()]);
        }
        if !self.lut_path.trim().is_empty() {
            args.extend(["--lut".into(), self.lut_path.trim().to_string()]);
        }
        if self.fix_levels {
            args.push("--fix".into());
        }

        // Motion & FX
        if (self.speed - 1.0).abs() > 0.01 {
            args.extend(["--speed".into(), format!("{:.2}", self.speed)]);
        }
        if self.velocity {
            args.push("--velocity".into());
        }
        if !self.fx_hits.is_empty() {
            let list: Vec<String> = self.fx_hits.iter().cloned().collect();
            args.extend(["--fx".into(), list.join(",")]);
        }
        if self.blur_faces {
            args.push("--blur-faces".into());
        }
        if !self.blur_box.trim().is_empty() {
            args.extend(["--blur-box".into(), self.blur_box.trim().to_string()]);
        }

        // Words, Overlays & Memes
        if self.lines_count > 0 {
            args.extend(["--lines".into(), self.lines_count.to_string()]);
        }
        for line in split_lines(&self.line_texts) {
            args.extend(["--line".into(), line]);
        }
        for s in split_lines(&self.say_specs) {
            args.extend(["--say".into(), s]);
        }
        if self.say_text {
            args.push("--say-text".into());
        }
        for c in split_lines(&self.captions_at) {
            args.extend(["--caption".into(), c]);
        }
        for f in split_lines(&self.flash_texts) {
            args.extend(["--flash-text".into(), f]);
        }
        for sv in split_lines(&self.svgs) {
            args.extend(["--svg".into(), sv]);
        }
        if !self.anim.trim().is_empty() {
            args.extend(["--anim".into(), self.anim.trim().to_string()]);
        }
        for ov in split_lines(&self.overlays) {
            args.extend(["--overlay".into(), ov]);
        }
        for p in split_lines(&self.pips) {
            args.extend(["--pip".into(), p]);
        }
        if self.math {
            args.push("--math".into());
        }
        if !self.intro.trim().is_empty() {
            args.extend(["--intro".into(), self.intro.trim().to_string()]);
        }
        if !self.intro_bed.trim().is_empty() {
            args.extend(["--intro-bed".into(), self.intro_bed.trim().to_string()]);
        }
        if !self.intro_caption.trim().is_empty() {
            args.extend(["--intro-caption".into(), self.intro_caption.trim().to_string()]);
        }
        if !self.outro.trim().is_empty() {
            args.extend(["--outro".into(), self.outro.trim().to_string()]);
        }
        if !self.tag.trim().is_empty() {
            args.extend(["--tag".into(), self.tag.trim().to_string()]);
        }
        if !self.text_banner.trim().is_empty() {
            args.extend(["--text".into(), self.text_banner.trim().to_string()]);
        }

        // Finish & Output
        args.extend(["-n".into(), self.n.to_string()]);
        if self.videos != 5 {
            args.extend(["--videos".into(), self.videos.to_string()]);
        }
        if self.use_videos != 4 {
            args.extend(["--use".into(), self.use_videos.to_string()]);
        }
        if !self.lang.trim().is_empty() {
            args.extend(["--lang".into(), self.lang.trim().to_string()]);
        }
        if !self.out_dir.trim().is_empty() {
            args.extend(["-o".into(), self.out_dir.trim().to_string()]);
        }
        if self.join {
            args.push("--join".into());
        }
        if self.no_llm {
            args.push("--no-llm".into());
        }
        if !self.curves.trim().is_empty() {
            args.extend(["--curves".into(), self.curves.trim().to_string()]);
        }
        if !self.pop_color.trim().is_empty() {
            args.extend(["--pop".into(), self.pop_color.trim().to_string()]);
        }
        if self.bass.abs() > 0.05 {
            args.extend(["--bass".into(), format!("{:.1}", self.bass)]);
        }
        if self.pitch.abs() > 0.05 {
            args.extend(["--pitch".into(), format!("{:.1}", self.pitch)]);
        }
        if self.reverb {
            args.push("--reverb".into());
        }
        if self.stabilize {
            args.push("--stabilize".into());
        }
        if self.denoise {
            args.push("--denoise".into());
        }
        if self.smooth {
            args.push("--smooth".into());
        }
        if let Ok(fps) = self.fps.trim().parse::<u32>() {
            args.extend(["--fps".into(), fps.to_string()]);
        }
        if self.codec != "h264" {
            args.extend(["--codec".into(), self.codec.clone()]);
        }
        if self.quality != "high" {
            args.extend(["--quality".into(), self.quality.clone()]);
        }
        if self.encoder != "cpu" {
            args.extend(["--encoder".into(), self.encoder.clone()]);
        }
        if !self.export_formats.is_empty() {
            let list: Vec<String> = self.export_formats.iter().cloned().collect();
            args.extend(["--export".into(), list.join(",")]);
        }

        Ok(args)
    }

    pub fn command_preview(&self) -> String {
        match self.build_args() {
            Ok(args) => format!("edits {}", format_args_preview(&args)),
            Err(_) => "edits <kaynak> ...".into(),
        }
    }
}

fn split_lines(s: &str) -> Vec<String> {
    s.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect()
}

pub enum MakeAction {
    None,
    CheckClip(std::path::PathBuf),
}

pub fn show(ui: &mut egui::Ui, ctx: &egui::Context, p: Palette, state: &mut MakeState, runner: &mut Runner) -> MakeAction {
    let mut action = MakeAction::None;

    // Drag & drop video files directly onto the Make view
    let dropped: Vec<std::path::PathBuf> = ctx.input(|i| {
        i.raw
            .dropped_files
            .iter()
            .filter_map(|f| f.path.clone())
            .collect()
    });
    if let Some(first) = dropped.first() {
        state.input = first.to_string_lossy().into_owned();
    }

    let avail_w = ui.available_width();
    let two_col = avail_w >= 880.0;

    if two_col {
        ui.columns(2, |cols| {
            let left_w = cols[0].available_width();
            let _ = left_w;
            ScrollArea::vertical().id_salt("make_left_scroll").show(&mut cols[0], |ui| {
                show_form(ui, p, state);
            });
            show_output_panel(ctx, &mut cols[1], p, state, runner, &mut action);
        });
    } else {
        ScrollArea::vertical().id_salt("make_single_scroll").show(ui, |ui| {
            show_form(ui, p, state);
            ui.add_space(12.0);
            show_output_panel(ctx, ui, p, state, runner, &mut action);
        });
    }

    action
}

fn show_form(ui: &mut egui::Ui, p: Palette, state: &mut MakeState) {
    // 1. Source & Mode
    card_frame(p).show(ui, |ui| {
        section_heading(ui, p, "1. Kaynak ve Mod", Some("Link, kanal, konu veya dosya sürükle-bırak"));
        ui.add_space(6.0);

        ui.horizontal_wrapped(|ui| {
            for mode in [EditMode::Preset, EditMode::Music, EditMode::Talking, EditMode::Reference] {
                if toggle_chip(ui, p, state.mode == mode, mode.label()).clicked() {
                    state.mode = mode;
                }
            }
        });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Kaynak:").color(p.muted));
            let resp = ui.add(
                TextEdit::singleline(&mut state.input)
                    .hint_text("https://youtube.com/@kanal  veya  konu  veya  C:\\video.mp4")
                    .desired_width(ui.available_width() - 108.0),
            );
            let _ = resp;
            if ui.button("Dosya Seç").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Video", &["mp4", "mov", "mkv", "webm", "avi", "m4v"])
                    .pick_file()
                {
                    state.input = file.to_string_lossy().into_owned();
                }
            }
        });

        ui.add_space(6.0);
        match state.mode {
            EditMode::Preset => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Preset:").color(p.muted));
                    egui::ComboBox::from_id_salt("preset_combo")
                        .selected_text(&state.preset)
                        .width(220.0)
                        .show_ui(ui, |ui| {
                            ui.add(TextEdit::singleline(&mut state.preset_filter).hint_text("Ara..."));
                            let q = state.preset_filter.to_lowercase();
                            for name in &state.preset_names {
                                if q.is_empty() || name.to_lowercase().contains(&q) {
                                    ui.selectable_value(&mut state.preset, name.clone(), name);
                                }
                            }
                        });
                    ui.checkbox(&mut state.intensity_override, "Şiddet");
                    if state.intensity_override {
                        ui.add(egui::Slider::new(&mut state.intensity, 0.2..=2.5).step_by(0.05));
                    }
                });
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Müzik (ops.):").color(p.muted));
                    ui.add(
                        TextEdit::singleline(&mut state.music)
                            .hint_text("Boş = preset şarkısı, ya da trend:3 / link / .mp3")
                            .desired_width(ui.available_width() - 100.0),
                    );
                    if ui.button("Ses Seç").clicked() {
                        if let Some(f) = rfd::FileDialog::new()
                            .add_filter("Ses/Video", &["mp3", "wav", "m4a", "flac", "ogg", "mp4"])
                            .pick_file()
                        {
                            state.music = f.to_string_lossy().into_owned();
                        }
                    }
                });
            }
            EditMode::Music => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Şarkı:").color(p.muted));
                    ui.add(
                        TextEdit::singleline(&mut state.music)
                            .hint_text("trend:3, YouTube linki, şarkı adı veya dosya yolu")
                            .desired_width(ui.available_width() - 100.0),
                    );
                    if ui.button("Dosya").clicked() {
                        if let Some(f) = rfd::FileDialog::new()
                            .add_filter("Ses/Video", &["mp3", "wav", "m4a", "flac", "ogg", "mp4"])
                            .pick_file()
                        {
                            state.music = f.to_string_lossy().into_owned();
                        }
                    }
                });
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut state.sad, "Sad Edit (--sad)");
                    ui.checkbox(&mut state.voice, "Konuşma Üzerine (--voice)");
                    ui.checkbox(&mut state.length_override, "Süre");
                    if state.length_override {
                        ui.add(egui::Slider::new(&mut state.length, 6.0..=90.0).suffix(" s"));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Geçiş (--transition):").color(p.muted));
                    egui::ComboBox::from_id_salt("xfade_combo")
                        .selected_text(if state.transition.is_empty() { "Otomatik" } else { &state.transition })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.transition, String::new(), "Otomatik");
                            for x in edits::montage::XFADES {
                                ui.selectable_value(&mut state.transition, (*x).to_string(), *x);
                            }
                        });
                });
            }
            EditMode::Talking => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Altyazı Stili:").color(p.muted));
                    egui::ComboBox::from_id_salt("cap_combo")
                        .selected_text(&state.caption_style)
                        .show_ui(ui, |ui| {
                            for name in &state.caption_names {
                                ui.selectable_value(&mut state.caption_style, name.clone(), name);
                            }
                        });
                    ui.label(RichText::new("Arka Fon:").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.bed).hint_text("trend:7 veya şarkı").desired_width(140.0));
                });
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut state.no_jumpcuts, "Jumpcut Yok");
                    ui.checkbox(&mut state.no_sfx, "SFX Yok");
                    ui.checkbox(&mut state.no_hook, "Hook Yok");
                    ui.checkbox(&mut state.no_captions, "Altyazı Yakma");
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Whoosh:").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.whoosh).hint_text("kütüphane sfx adı").desired_width(120.0));
                    ui.label(RichText::new("Boom:").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.boom).hint_text("kütüphane sfx adı").desired_width(120.0));
                });
            }
            EditMode::Reference => {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Referans Edit:").color(p.muted));
                    ui.add(
                        TextEdit::singleline(&mut state.like_video)
                            .hint_text("Kopyalanacak .mp4 edit")
                            .desired_width(ui.available_width() - 100.0),
                    );
                    if ui.button("Video Seç").clicked() {
                        if let Some(f) = rfd::FileDialog::new().add_filter("Video", &["mp4", "mov", "mkv"]).pick_file() {
                            state.like_video = f.to_string_lossy().into_owned();
                        }
                    }
                });
                ui.checkbox(&mut state.voice, "Konuşma Üzerine (--voice)");
            }
        }
    });

    ui.add_space(10.0);

    // 2. Frame, Colour Look & Effects
    card_frame(p).show(ui, |ui| {
        section_heading(ui, p, "2. Kadraj, Renk ve Efektler", Some("Aspect, LUT ve vuruş efektleri"));
        ui.add_space(6.0);

        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Oran:").color(p.muted));
            for (asp, _, _) in edits::study::ASPECTS {
                if toggle_chip(ui, p, state.custom_size.is_empty() && state.aspect == *asp, asp).clicked() {
                    state.aspect = (*asp).to_string();
                    state.custom_size.clear();
                }
            }
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Özel WxH:").color(p.muted));
            ui.add(TextEdit::singleline(&mut state.custom_size).hint_text("1080x1350").desired_width(90.0));
            ui.checkbox(&mut state.fit, "Sığdır (--fit, kırpma)");
            ui.checkbox(&mut state.fix_levels, "Işık Düzelt (--fix)");
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Renk (--look):").color(p.muted));
            let about = state
                .look_options
                .iter()
                .find(|(n, _)| n == &state.look)
                .map(|(_, a)| a.as_str())
                .unwrap_or("");
            egui::ComboBox::from_id_salt("look_combo")
                .selected_text(&state.look)
                .width(170.0)
                .show_ui(ui, |ui| {
                    for (name, desc) in &state.look_options {
                        ui.selectable_value(&mut state.look, name.clone(), format!("{name} — {desc}"));
                    }
                });
            ui.label(RichText::new(about).size(11.5).color(p.muted));
        });

        ui.horizontal(|ui| {
            ui.label(RichText::new(".cube LUT:").color(p.muted));
            ui.add(TextEdit::singleline(&mut state.lut_path).hint_text("Opsiyonel .cube dosyası").desired_width(210.0));
            if ui.button("LUT Seç").clicked() {
                if let Some(f) = rfd::FileDialog::new().add_filter("LUT", &["cube"]).pick_file() {
                    state.lut_path = f.to_string_lossy().into_owned();
                }
            }
            if !state.lut_path.is_empty() && ui.button("Temizle").clicked() {
                state.lut_path.clear();
            }
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Hız:").color(p.muted));
            ui.add(egui::Slider::new(&mut state.speed, 0.25..=3.0).step_by(0.05).suffix("x"));
            ui.checkbox(&mut state.velocity, "Velocity Ramp");
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("Beat FX:").color(p.muted));
            for hit in edits::fx::HITS {
                let on = state.fx_hits.contains(*hit);
                if toggle_chip(ui, p, on, hit).clicked() {
                    if on {
                        state.fx_hits.remove(*hit);
                    } else {
                        state.fx_hits.insert((*hit).to_string());
                    }
                }
            }
        });
    });

    ui.add_space(10.0);

    // 3. Words, Overlays & Memes (Collapsible)
    card_frame(p).show(ui, |ui| {
        egui::CollapsingHeader::new(RichText::new("3. Yazılar, Çıkartmalar, Overlay & Meme").strong().color(p.text))
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Ana Başlık (--text):").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.text_banner).hint_text("THE PROBLEM").desired_width(180.0));
                    ui.label(RichText::new("İmza (--tag):").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.tag).hint_text("@hesap").desired_width(120.0));
                });
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Otomatik Alıntı (--lines):").color(p.muted));
                    ui.add(egui::DragValue::new(&mut state.lines_count).range(0..=12));
                    ui.checkbox(&mut state.math, "Math Flash (--math)");
                    ui.checkbox(&mut state.say_text, "Söylenen Alt Yazı (--say-text)");
                });
                ui.label(RichText::new("Özel Satırlar (--line, her satır bir replik):").size(12.0).color(p.muted));
                ui.add(TextEdit::multiline(&mut state.line_texts).desired_rows(2).desired_width(f32::INFINITY));

                ui.label(RichText::new("--say (VIDEO@FROM-TO) / --shot (VIDEO@SEC):").size(12.0).color(p.muted));
                ui.columns(2, |cols| {
                    cols[0].add(TextEdit::multiline(&mut state.say_specs).hint_text("ID@12.4-15.1").desired_rows(2).desired_width(f32::INFINITY));
                    cols[1].add(TextEdit::multiline(&mut state.shot_specs).hint_text("ID@4.0:0.5").desired_rows(2).desired_width(f32::INFINITY));
                });

                ui.label(RichText::new("Üst Yazılar (--caption TEXT@FROM-TO) & Subliminal (--flash-text SEC:TEXT):").size(12.0).color(p.muted));
                ui.columns(2, |cols| {
                    cols[0].add(TextEdit::multiline(&mut state.captions_at).hint_text("bu adam 🗿@0-2.5").desired_rows(2).desired_width(f32::INFINITY));
                    cols[1].add(TextEdit::multiline(&mut state.flash_texts).hint_text("5.2:dışarısı simülasyon").desired_rows(2).desired_width(f32::INFINITY));
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Animasyon (--anim):").color(p.muted));
                    egui::ComboBox::from_id_salt("anim_combo")
                        .selected_text(if state.anim.is_empty() { "rise (varsayılan)" } else { &state.anim })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.anim, String::new(), "rise (varsayılan)");
                            for a in edits::svg::ANIMS {
                                ui.selectable_value(&mut state.anim, (*a).to_string(), *a);
                            }
                        });
                });

                ui.label(RichText::new("SVG (--svg), Overlay (--overlay) & PiP (--pip):").size(12.0).color(p.muted));
                ui.columns(3, |cols| {
                    cols[0].add(TextEdit::multiline(&mut state.svgs).hint_text("arrow@0.5-2.5:0.6,0.3,0.3~pop").desired_rows(2).desired_width(f32::INFINITY));
                    cols[1].add(TextEdit::multiline(&mut state.overlays).hint_text("vhs texture:0.5").desired_rows(2).desired_width(f32::INFINITY));
                    cols[2].add(TextEdit::multiline(&mut state.pips).hint_text("cam.mp4@2-8:0.75,0.2,0.35").desired_rows(2).desired_width(f32::INFINITY));
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Intro:").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.intro).hint_text("ID@1-4 veya dosya").desired_width(120.0));
                    ui.label(RichText::new("Intro Bed:").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.intro_bed).desired_width(90.0));
                    ui.label(RichText::new("Intro Yazı:").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.intro_caption).desired_width(100.0));
                    ui.label(RichText::new("Outro:").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.outro).hint_text("edits").desired_width(70.0));
                });
            });
    });

    ui.add_space(10.0);

    // 4. Finish, Audio & Export
    card_frame(p).show(ui, |ui| {
        egui::CollapsingHeader::new(RichText::new("4. Çıktı, Son Rötuş & Export").strong().color(p.text))
            .default_open(false)
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Klip Sayısı (-n):").color(p.muted));
                    ui.add(egui::DragValue::new(&mut state.n).range(1..=20));
                    ui.label(RichText::new("Taranacak Video:").color(p.muted));
                    ui.add(egui::DragValue::new(&mut state.videos).range(1..=40));
                    ui.label(RichText::new("İndirilecek:").color(p.muted));
                    ui.add(egui::DragValue::new(&mut state.use_videos).range(1..=20));
                    ui.label(RichText::new("Dil:").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.lang).desired_width(36.0));
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Çıktı Klasörü (-o):").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.out_dir).hint_text("Boş = ~/Videos/edits/...").desired_width(220.0));
                    if ui.button("Klasör Seç").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            state.out_dir = dir.to_string_lossy().into_owned();
                        }
                    }
                    ui.checkbox(&mut state.join, "Birleştir (--join)");
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Codec:").color(p.muted));
                    egui::ComboBox::from_id_salt("codec_combo")
                        .selected_text(&state.codec)
                        .show_ui(ui, |ui| {
                            for c in edits::finish::CODECS {
                                ui.selectable_value(&mut state.codec, (*c).to_string(), *c);
                            }
                        });
                    ui.label(RichText::new("Kalite:").color(p.muted));
                    egui::ComboBox::from_id_salt("qual_combo")
                        .selected_text(&state.quality)
                        .show_ui(ui, |ui| {
                            for q in edits::finish::QUALITIES {
                                ui.selectable_value(&mut state.quality, (*q).to_string(), *q);
                            }
                        });
                    ui.label(RichText::new("Encoder:").color(p.muted));
                    egui::ComboBox::from_id_salt("enc_combo")
                        .selected_text(&state.encoder)
                        .show_ui(ui, |ui| {
                            for e in ["auto", "cpu", "nvenc"] {
                                ui.selectable_value(&mut state.encoder, e.to_string(), e);
                            }
                        });
                });

                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut state.stabilize, "Stabilize");
                    ui.checkbox(&mut state.denoise, "Denoise");
                    ui.checkbox(&mut state.smooth, "Smooth 60fps");
                    ui.checkbox(&mut state.reverb, "Reverb");
                    ui.checkbox(&mut state.blur_faces, "Yüz Bulanıklaştır");
                    ui.checkbox(&mut state.no_llm, "LLM Kullanma");
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Curves:").color(p.muted));
                    egui::ComboBox::from_id_salt("curves_combo")
                        .selected_text(if state.curves.is_empty() { "Yok" } else { &state.curves })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.curves, String::new(), "Yok");
                            for cv in edits::fx::CURVES {
                                ui.selectable_value(&mut state.curves, (*cv).to_string(), *cv);
                            }
                        });
                    ui.label(RichText::new("Colour Pop:").color(p.muted));
                    egui::ComboBox::from_id_salt("pop_combo")
                        .selected_text(if state.pop_color.is_empty() { "Yok" } else { &state.pop_color })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.pop_color, String::new(), "Yok");
                            for (name, _) in edits::fx::POP {
                                ui.selectable_value(&mut state.pop_color, (*name).to_string(), *name);
                            }
                        });
                    ui.label(RichText::new("FPS:").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.fps).hint_text("30/60").desired_width(50.0));
                    ui.label(RichText::new("Blur Box:").color(p.muted));
                    ui.add(TextEdit::singleline(&mut state.blur_box).hint_text("x,y,w,h").desired_width(90.0));
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Bass dB:").color(p.muted));
                    ui.add(egui::Slider::new(&mut state.bass, -12.0..=15.0).step_by(0.5));
                    ui.label(RichText::new("Pitch:").color(p.muted));
                    ui.add(egui::Slider::new(&mut state.pitch, -12.0..=12.0).step_by(0.5));
                });

                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Timeline Export:").color(p.muted));
                    for fmt in ["fcpxml", "edl", "srt", "vtt"] {
                        let on = state.export_formats.contains(fmt);
                        if toggle_chip(ui, p, on, fmt).clicked() {
                            if on {
                                state.export_formats.remove(fmt);
                            } else {
                                state.export_formats.insert(fmt.to_string());
                            }
                        }
                    }
                });
            });
    });
}

fn show_output_panel(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    p: Palette,
    state: &mut MakeState,
    runner: &mut Runner,
    action: &mut MakeAction,
) {
    // Command preview & Run bar
    card_frame(p).show(ui, |ui| {
        section_heading(ui, p, "Komut ve Çalıştırma", None);
        ui.add_space(6.0);

        let cmd = state.command_preview();
        egui::Frame::new()
            .fill(p.input)
            .stroke(Stroke::new(1.0_f32, p.line))
            .corner_radius(6)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(RichText::new(&cmd).font(FontId::monospace(11.5)).color(p.text)).wrap(),
                );
            });

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if runner.running {
                if ui.add_sized([150.0, 34.0], cta_button("Durdur")).clicked() {
                    runner.cancel();
                }
            } else if ui.add_sized([160.0, 34.0], cta_button("Edit Oluştur")).clicked() {
                match state.build_args() {
                    Ok(args) => {
                        state.status_toast.clear();
                        runner.start(ctx, "edits", args, true);
                    }
                    Err(e) => {
                        state.status_toast = e;
                    }
                }
            }

            if ui.button("Komutu Kopyala").clicked() {
                ctx.copy_text(cmd);
                state.status_toast = "Komut panoya kopyalandı.".into();
            }

            if runner.running {
                ui.spinner();
            }
            ui.label(RichText::new(&runner.status).size(12.0).color(p.muted));
        });

        if !state.status_toast.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new(&state.status_toast).size(12.0).color(CUT_RED));
        }
    });

    ui.add_space(10.0);

    // Finished clips cards
    if !runner.clips.is_empty() {
        card_frame(p).show(ui, |ui| {
            section_heading(ui, p, "Üretilen Klipler", Some(&format!("{} adet", runner.clips.len())));
            ui.add_space(6.0);
            for clip in &runner.clips {
                egui::Frame::new()
                    .fill(p.input)
                    .stroke(Stroke::new(1.0_f32, p.line))
                    .corner_radius(6)
                    .inner_margin(10.0)
                    .show(ui, |ui| {
                        let name = clip
                            .clip_path
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| clip.clip_path.display().to_string());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(name).strong().color(OK_GREEN));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Kalite Kontrol (check)").clicked() {
                                    *action = MakeAction::CheckClip(clip.clip_path.clone());
                                }
                                if ui.button("Klasörü Aç").clicked() {
                                    if let Some(dir) = clip.clip_path.parent() {
                                        open_in_os(dir);
                                    }
                                }
                                if ui.button("Oynat").clicked() {
                                    open_in_os(&clip.clip_path);
                                }
                            });
                        });
                        if !clip.summary.is_empty() {
                            ui.label(RichText::new(&clip.summary).size(12.0).color(p.muted));
                        }
                        egui::CollapsingHeader::new(RichText::new("JSON").size(11.0).color(p.muted))
                            .id_salt(&clip.clip_path)
                            .show(ui, |ui| {
                                ui.label(RichText::new(&clip.raw_json).font(FontId::monospace(11.0)).color(p.text));
                            });
                    });
                ui.add_space(4.0);
            }
        });
        ui.add_space(10.0);
    }

    // Live streaming console
    card_frame(p).show(ui, |ui| {
        ui.horizontal(|ui| {
            section_heading(ui, p, "Canlı Log", None);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Temizle").clicked() {
                    runner.logs.clear();
                }
            });
        });
        ui.add_space(6.0);

        let h = ui.available_height().max(240.0);
        egui::Frame::new()
            .fill(p.input)
            .stroke(Stroke::new(1.0_f32, p.line))
            .corner_radius(6)
            .inner_margin(8.0)
            .show(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt("runner_logs_scroll")
                    .max_height(h - 16.0)
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        if runner.logs.is_empty() {
                            ui.label(
                                RichText::new("Henüz çıktı yok. Yukarıdan kaynak girip 'Edit Oluştur'a basın.")
                                    .font(FontId::monospace(12.0))
                                    .color(p.muted),
                            );
                        }
                        for line in &runner.logs {
                            let color = if line.text.contains("failed") || line.text.contains("error") || line.text.starts_with("edits: could not") {
                                CUT_RED
                            } else if line.is_err {
                                p.muted
                            } else if line.text.trim_start().starts_with('{') {
                                PAPER
                            } else {
                                p.text
                            };
                            ui.label(RichText::new(&line.text).font(FontId::monospace(11.5)).color(color));
                        }
                    });
            });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_mode_builds_expected_args() {
        let mut s = MakeState::new();
        assert!(s.build_args().is_err());

        s.input = "https://youtube.com/@demo".into();
        s.preset = "spin-slide-shake".into();
        s.music = "trend:3".into();
        s.look = "tealorange".into();
        s.aspect = "4:5".into();
        s.fx_hits.insert("shake".into());
        s.fx_hits.insert("rgb".into());
        s.velocity = true;
        s.svgs = "arrow@0.5-2.0\ncircle@3-4".into();

        let args = s.build_args().unwrap();
        assert_eq!(args[0], "https://youtube.com/@demo");
        assert!(args.windows(2).any(|w| w == ["--preset", "spin-slide-shake"]));
        assert!(args.windows(2).any(|w| w == ["--music", "trend:3"]));
        assert!(args.windows(2).any(|w| w == ["--look", "tealorange"]));
        assert!(args.windows(2).any(|w| w == ["--aspect", "4:5"]));
        assert!(args.contains(&"--velocity".to_string()));
        assert_eq!(args.iter().filter(|a| *a == "--svg").count(), 2);
    }

    #[test]
    fn music_and_reference_modes_validate_required_fields() {
        let mut s = MakeState::new();
        s.input = "clip.mp4".into();
        s.mode = EditMode::Music;
        assert!(s.build_args().is_err());

        s.music = "trend:1".into();
        s.sad = true;
        let args = s.build_args().unwrap();
        assert!(args.contains(&"--sad".to_string()));

        s.mode = EditMode::Reference;
        assert!(s.build_args().is_err());
        s.like_video = "ref.mp4".into();
        let args = s.build_args().unwrap();
        assert!(args.windows(2).any(|w| w == ["--like", "ref.mp4"]));
    }
}
