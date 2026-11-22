//! The brand (BRAND.md) as the window draws it: ink and paper, one warm grey, one red, cards one step off the page,
//! Helvetica in Light and Bold, every space a multiple of 8 px. BRAND.md points here: change a value here, then there.
//! Appearance can also follow Omarchy: its current theme's colours, live, when this computer runs it.
use gpui_kit::component::{Theme, ThemeMode};
use gpui_kit::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    /// t3code's accent: what is selected, on, focused, and the button that does the thing
    pub accent: Hsla,
}

/// The accents to pick from, the brand's red first.
pub const ACCENTS: &[(&str, &str)] = &[("Red", "#E5332A"), ("Blue", "#3B6FF5"), ("Green", "#2EA043"), ("Violet", "#8B5CF6"), ("Orange", "#F28C28")];

pub fn hex_of(s: &str) -> Hsla {
    parse(s).unwrap_or_else(|| hex(0xE5332A))
}

fn hex(v: u32) -> Hsla {
    rgb(v).into()
}

/// "#89b4fa" as a colour.
fn parse(s: &str) -> Option<Hsla> {
    u32::from_str_radix(s.trim().trim_start_matches('#'), 16).ok().filter(|_| s.trim().len() == 7).map(hex)
}

/// What Settings > Appearance chose, after t3code's appearance settings.
#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Prefs {
    /// system, light, dark or omarchy
    pub mode: String,
    /// the chat column: narrow, normal, wide
    pub width: String,
    pub font_size: f32,
    #[serde(default = "red")]
    pub accent: String,
    /// the welcome page was seen
    #[serde(default)]
    pub onboarded: bool,
}

fn red() -> String {
    "#E5332A".into()
}

impl Default for Prefs {
    fn default() -> Self {
        Prefs { mode: "system".into(), width: "normal".into(), font_size: 15., accent: red(), onboarded: false }
    }
}

fn prefs_file() -> PathBuf {
    crate::util::home().join(".config/edits/app.json")
}

pub fn prefs() -> Prefs {
    std::fs::read_to_string(prefs_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

pub fn save(p: &Prefs, window: &mut Window, cx: &mut App) {
    let f = prefs_file();
    let _ = std::fs::create_dir_all(f.parent().unwrap_or(std::path::Path::new(".")));
    let _ = std::fs::write(f, serde_json::to_string_pretty(p).unwrap_or_default());
    apply(Some(window), cx);
    window.refresh();
}

/// The chat column's width.
pub fn column() -> Pixels {
    match prefs().width.as_str() {
        "narrow" => px(640.),
        "wide" => px(960.),
        _ => px(736.),
    }
}

/// Omarchy's current theme folder, when this computer runs Omarchy.
pub fn omarchy_dir() -> Option<PathBuf> {
    Some(crate::util::home().join(".local/state/omarchy/current/theme")).filter(|d| d.join("colors.toml").is_file())
}

/// Changes each time Omarchy switches its theme: the app looks for it to follow along.
pub fn omarchy_generation() -> String {
    std::fs::read_to_string(crate::util::home().join(".local/state/omarchy/current/theme.generation")).unwrap_or_default()
}

/// Omarchy's theme as brand colours: its background is the page, its lighter one the cards, its text the ink.
fn omarchy() -> Option<(Brand, bool)> {
    let t = std::fs::read_to_string(omarchy_dir()?.join("colors.toml")).ok()?;
    let get = |k: &str| t.lines().find_map(|l| l.split_once('=').filter(|(a, _)| a.trim() == k).map(|(_, v)| v.trim().trim_matches('"').to_string()));
    let c = |k: &str| get(k).and_then(|v| parse(&v));
    let dark = get("mode").is_none_or(|m| m != "light");
    Some((Brand { page: c("background")?, card: c("lighter_background").or(c("dark_background"))?, hair: c("selection").or(c("muted"))?,
        ink: c("foreground")?, gray: c("muted").or(c("dark_foreground"))?, red_text: c("red")?, accent: c("accent").or(c("blue"))? }, dark))
}

pub fn brand(cx: &App) -> Brand {
    let p = prefs();
    if p.mode == "omarchy" {
        if let Some((b, _)) = omarchy() {
            return b;
        }
    }
    let accent = hex_of(&p.accent);
    if Theme::global(cx).is_dark() {
        Brand { page: hex(0x0D1117), card: hex(0x161B22), hair: hex(0x30363D), ink: hex(0xF4F1EA), gray: hex(0x9A958A), red_text: hex(0xF25247), accent }
    } else {
        Brand { page: hex(0xFFFFFF), card: hex(0xF6F4EF), hair: hex(0xE4E0D7), ink: hex(0x0B0B0C), gray: hex(0x6F6A60), red_text: hex(0xC8261E), accent }
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
/// t3code's geometry: the sidebar, the page header
pub const SIDEBAR: Pixels = px(256.);
pub const HEADER: Pixels = px(52.);

/// The chosen look, in brand colours (or Omarchy's), also for what gpui-kit draws (the text input, notifications).
pub fn apply(window: Option<&mut Window>, cx: &mut App) {
    let p = prefs();
    match (p.mode.as_str(), omarchy()) {
        ("light", _) => Theme::change(ThemeMode::Light, window, cx),
        ("dark", _) => Theme::change(ThemeMode::Dark, window, cx),
        ("omarchy", Some((_, dark))) => Theme::change(if dark { ThemeMode::Dark } else { ThemeMode::Light }, window, cx),
        _ => Theme::sync_system_appearance(window, cx),
    }
    let b = brand(cx);
    let t = Theme::global_mut(cx);
    t.font_family = FONT.into();
    t.font_size = px(p.font_size);
    let c = &mut t.colors;
    (c.background, c.foreground, c.border, c.input, c.ring, c.caret) = (b.page, b.ink, b.hair, b.hair, b.accent, b.accent);
    (c.muted_foreground, c.popover, c.popover_foreground, c.list_active, c.list_active_border) = (b.gray, b.card, b.ink, b.accent.opacity(0.14), b.accent);
    (c.primary, c.primary_hover, c.primary_active, c.primary_foreground) = (b.accent, b.accent.opacity(0.9), b.accent.opacity(0.8), white());
    (c.scrollbar_thumb, c.scrollbar_thumb_hover, c.selection) = (b.hair, b.gray, b.accent.opacity(0.3));
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn omarchy_colours_read() {
        assert!(parse("#89b4fa").is_some() && parse("\"#1e1e2e\"".trim_matches('"')).is_some());
        assert!(parse("89b4").is_none() && parse("blue").is_none());
    }
}
