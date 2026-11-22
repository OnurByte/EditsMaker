//! The brand (BRAND.md) as the window draws it: ink and paper, one warm grey, one red, cards one step off the page,
//! Helvetica in Light and Bold, every space a multiple of 8 px. BRAND.md points here: change a value here, then there.
use gpui_kit::component::Theme;
use gpui_kit::*;

#[derive(Clone, Copy)]
pub struct Brand {
    /// the window's background: GitHub's own page colour, which the brand's cards sit one step off
    pub page: Hsla,
    /// a card: the composer, a message, a pack, the row that is selected
    pub card: Hsla,
    /// hairlines between rows, never a frame
    pub hair: Hsla,
    /// text, and the mark's first half
    pub ink: Hsla,
    /// labels, small notes
    pub gray: Hsla,
    /// red text under 24 px
    pub red_text: Hsla,
}

fn hex(v: u32) -> Hsla {
    rgb(v).into()
}

pub fn brand(cx: &App) -> Brand {
    if Theme::global(cx).is_dark() {
        Brand { page: hex(0x0D1117), card: hex(0x161B22), hair: hex(0x30363D), ink: hex(0xF4F1EA), gray: hex(0x9A958A), red_text: hex(0xF25247) }
    } else {
        Brand { page: hex(0xFFFFFF), card: hex(0xF6F4EF), hair: hex(0xE4E0D7), ink: hex(0x0B0B0C), gray: hex(0x6F6A60), red_text: hex(0xC8261E) }
    }
}

/// Helvetica, or the stand-in each system has.
pub const FONT: &str = if cfg!(target_os = "macos") { "Helvetica Neue" } else if cfg!(windows) { "Arial" } else { "Liberation Sans" };
pub const MONO: &str = if cfg!(target_os = "macos") { "Menlo" } else if cfg!(windows) { "Consolas" } else { "DejaVu Sans Mono" };
/// describing
pub const LIGHT: FontWeight = FontWeight::LIGHT;
/// naming
pub const BOLD: FontWeight = FontWeight::BOLD;

/// 20 px on a card, 6 px on anything inside one
pub const CARD: Pixels = px(20.);
pub const SMALL: Pixels = px(6.);
/// t3code's geometry: the sidebar, the page header, the chat column
pub const SIDEBAR: Pixels = px(256.);
pub const HEADER: Pixels = px(52.);
pub const COLUMN: Pixels = px(736.);

fn pref_file() -> std::path::PathBuf {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".into());
    std::path::Path::new(&home).join(".config/edits/app-theme")
}

/// "system", "light" or "dark": what Settings > Appearance chose.
pub fn pref() -> String {
    std::fs::read_to_string(pref_file()).map(|s| s.trim().to_string()).ok().filter(|s| s == "light" || s == "dark").unwrap_or_else(|| "system".into())
}

pub fn set_pref(p: &str, window: &mut Window, cx: &mut App) {
    let f = pref_file();
    let _ = std::fs::create_dir_all(f.parent().unwrap_or(std::path::Path::new(".")));
    let _ = std::fs::write(f, p);
    apply(Some(window), cx);
    window.refresh();
}

/// Light or dark (the system's, unless Settings chose), in brand colours, also for what gpui-kit draws (the text
/// input, notifications).
pub fn apply(window: Option<&mut Window>, cx: &mut App) {
    match pref().as_str() {
        "light" => Theme::change(gpui_kit::component::ThemeMode::Light, window, cx),
        "dark" => Theme::change(gpui_kit::component::ThemeMode::Dark, window, cx),
        _ => Theme::sync_system_appearance(window, cx),
    }
    let b = brand(cx);
    let t = Theme::global_mut(cx);
    t.font_family = FONT.into();
    t.font_size = px(15.);
    let c = &mut t.colors;
    (c.background, c.foreground, c.border, c.input, c.ring, c.caret) = (b.page, b.ink, b.hair, b.hair, b.hair, b.ink);
    (c.muted_foreground, c.popover, c.popover_foreground, c.list_active, c.list_active_border) = (b.gray, b.card, b.ink, b.card, b.hair);
    (c.primary, c.primary_hover, c.primary_active, c.primary_foreground) = (b.ink, b.ink, b.ink, b.page);
    (c.scrollbar_thumb, c.scrollbar_thumb_hover, c.selection) = (b.hair, b.gray, b.gray.opacity(0.3));
}
