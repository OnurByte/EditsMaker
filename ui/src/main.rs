#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod library_tab;
mod looks_tab;
mod make;
mod presets_tab;
mod runner;
mod theme;
mod tools_tab;

use eframe::egui::{self, FontId, Margin, RichText, Stroke};
use library_tab::{LibraryAction, LibraryState};
use looks_tab::{LooksAction, LooksState};
use make::{EditMode, MakeAction, MakeState};
use presets_tab::{PresetsAction, PresetsState};
use runner::Runner;
use theme::{apply_theme, toggle_chip, Assets, Palette, ThemeMode, CUT_RED};
use tools_tab::ToolsState;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Make,
    Presets,
    Looks,
    Library,
    Tools,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Tab::Make => "Edit Oluştur",
            Tab::Presets => "Presetler",
            Tab::Looks => "Görünümler",
            Tab::Library => "Trendler & Kütüphane",
            Tab::Tools => "Doktor & Kontrol",
        }
    }
}

struct EditsApp {
    tab: Tab,
    theme: ThemeMode,
    assets: Assets,
    make: MakeState,
    presets: PresetsState,
    looks: LooksState,
    library: LibraryState,
    tools: ToolsState,
    runner: Runner,
    screenshot_out: Option<std::path::PathBuf>,
    screenshot_sent: bool,
    frames: u32,
}

impl EditsApp {
    fn new(
        cc: &eframe::CreationContext<'_>,
        initial_tab: Tab,
        initial_theme: ThemeMode,
        screenshot_out: Option<std::path::PathBuf>,
    ) -> Self {
        edits::tools::add_to_path();
        apply_theme(&cc.egui_ctx, initial_theme);
        let assets = Assets::load(&cc.egui_ctx);

        Self {
            tab: initial_tab,
            theme: initial_theme,
            assets,
            make: MakeState::new(),
            presets: PresetsState::new(),
            looks: LooksState::new(),
            library: LibraryState::new(),
            tools: ToolsState::new(),
            runner: Runner::default(),
            screenshot_out,
            screenshot_sent: false,
            frames: 0,
        }
    }
}

impl eframe::App for EditsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frames = self.frames.saturating_add(1);
        if let Some(path) = self.screenshot_out.clone() {
            let shot = ctx.input(|i| {
                i.events.iter().chain(i.raw.events.iter()).find_map(|ev| match ev {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            });
            if let Some(img) = shot {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = image::save_buffer(
                    &path,
                    img.as_raw(),
                    img.width() as u32,
                    img.height() as u32,
                    image::ColorType::Rgba8,
                );
                self.screenshot_out = None;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            } else if !self.screenshot_sent && self.frames >= 4 {
                self.screenshot_sent = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            ctx.request_repaint();
        }
        if let Some(done_label) = self.runner.poll() {
            match done_label.as_str() {
                "preset import" | "preset learn" | "preset create" => {
                    self.presets.reload();
                    self.make.reload_catalogs();
                }
                "library add" => {
                    self.library.reload_library();
                    self.presets.reload();
                    self.make.reload_catalogs();
                }
                "setup" => {
                    edits::tools::add_to_path();
                    self.tools.refresh_doctor();
                }
                "study" => {
                    self.tools.studied_style = edits::study::load();
                }
                _ => {}
            }
        }
        let p = Palette::from_mode(self.theme);

        // Top editorial header
        egui::TopBottomPanel::top("edits_header")
            .frame(
                egui::Frame::new()
                    .fill(p.bg)
                    .stroke(Stroke::new(1.0_f32, p.line))
                    .inner_margin(Margin::symmetric(16, 10)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let mark = if p.dark {
                        self.assets.mark_dark.as_ref()
                    } else {
                        self.assets.mark_light.as_ref()
                    };
                    if let Some(tex) = mark {
                        ui.image((tex.id(), egui::vec2(28.0, 28.0)));
                    }
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.label(
                            RichText::new("EditsMaker")
                                .font(FontId::proportional(19.0))
                                .strong()
                                .color(p.text),
                        );
                        ui.label(
                            RichText::new(format!("v{} · Link in. Edits out.", env!("CARGO_PKG_VERSION")))
                                .font(FontId::proportional(11.0))
                                .color(p.muted),
                        );
                    });

                    ui.add_space(16.0);

                    ui.horizontal_wrapped(|ui| {
                        for tab in [Tab::Make, Tab::Presets, Tab::Looks, Tab::Library, Tab::Tools] {
                            if toggle_chip(ui, p, self.tab == tab, tab.label()).clicked() {
                                self.tab = tab;
                            }
                        }
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let toggle_label = if self.theme == ThemeMode::Ink { "Paper" } else { "Ink" };
                        if ui.button(toggle_label).clicked() {
                            self.theme = if self.theme == ThemeMode::Ink {
                                ThemeMode::Paper
                            } else {
                                ThemeMode::Ink
                            };
                            apply_theme(ctx, self.theme);
                        }
                        if self.runner.running {
                            ui.spinner();
                            ui.label(RichText::new(&self.runner.active_label).size(12.0).color(CUT_RED));
                        }
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.bg).inner_margin(Margin::same(14)))
            .show(ctx, |ui| match self.tab {
                Tab::Make => {
                    if let MakeAction::CheckClip(path) = make::show(ui, ctx, p, &mut self.make, &mut self.runner) {
                        self.tab = Tab::Tools;
                        self.tools.run_check(ctx, path);
                    }
                }
                Tab::Presets => match presets_tab::show(ui, ctx, p, &mut self.presets, &mut self.runner) {
                    PresetsAction::UseInMake(name) => {
                        self.make.mode = EditMode::Preset;
                        self.make.preset = name;
                        self.tab = Tab::Make;
                    }
                    PresetsAction::CatalogChanged => {
                        self.make.reload_catalogs();
                    }
                    PresetsAction::None => {}
                },
                Tab::Looks => match looks_tab::show(ui, ctx, p, &self.assets, &mut self.looks, &mut self.runner) {
                    LooksAction::UseLook(look) => {
                        self.make.look = look;
                        self.tab = Tab::Make;
                    }
                    LooksAction::UseCaption(style) => {
                        self.make.mode = EditMode::Talking;
                        self.make.caption_style = style;
                        self.tab = Tab::Make;
                    }
                    LooksAction::AddSticker(spec) => {
                        if !self.make.svgs.trim().is_empty() && !self.make.svgs.ends_with('\n') {
                            self.make.svgs.push('\n');
                        }
                        self.make.svgs.push_str(&spec);
                        self.tab = Tab::Make;
                    }
                    LooksAction::ToggleFx(hit) => {
                        if !self.make.fx_hits.insert(hit.clone()) {
                            self.make.fx_hits.remove(&hit);
                        }
                        self.tab = Tab::Make;
                    }
                    LooksAction::None => {}
                },
                Tab::Library => match library_tab::show(ui, ctx, p, &mut self.library, &mut self.runner) {
                    LibraryAction::UseMusic(song) => {
                        self.make.music = song;
                        if self.make.mode == EditMode::Talking || self.make.mode == EditMode::Reference {
                            self.make.mode = EditMode::Music;
                        }
                        self.tab = Tab::Make;
                    }
                    LibraryAction::UseBed(song) => {
                        self.make.mode = EditMode::Talking;
                        self.make.bed = song;
                        self.tab = Tab::Make;
                    }
                    LibraryAction::AddOverlay(spec) => {
                        if !self.make.overlays.trim().is_empty() && !self.make.overlays.ends_with('\n') {
                            self.make.overlays.push('\n');
                        }
                        self.make.overlays.push_str(&spec);
                        self.tab = Tab::Make;
                    }
                    LibraryAction::UseInput(path) => {
                        self.make.input = path;
                        self.tab = Tab::Make;
                    }
                    LibraryAction::None => {}
                },
                Tab::Tools => {
                    tools_tab::show(ui, ctx, p, &mut self.tools, &mut self.runner);
                }
            });
    }
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let opt = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).map(String::as_str);
    let initial_tab = match opt("--tab") {
        Some("presets") => Tab::Presets,
        Some("looks") => Tab::Looks,
        Some("library") => Tab::Library,
        Some("tools") => Tab::Tools,
        _ => Tab::Make,
    };
    let initial_theme = match opt("--theme") {
        Some("paper") => ThemeMode::Paper,
        _ => ThemeMode::Ink,
    };
    let screenshot_out = opt("--screenshot").map(std::path::PathBuf::from);

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1240.0, 820.0])
        .with_min_inner_size([860.0, 600.0])
        .with_title("EditsMaker");
    if let Some(icon) = theme::window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "EditsMaker",
        options,
        Box::new(move |cc| Ok(Box::new(EditsApp::new(cc, initial_tab, initial_theme, screenshot_out)))),
    )
}
