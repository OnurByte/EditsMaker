//! Lucide icons, the ones t3code draws with: gpui-kit's own set and a few more embedded here, one colour each
//! (BRAND.md § 6). And the EditsMaker mark.
use gpui_kit::assets::IconName;
use gpui_kit::component::Icon as Glyph;
use gpui_kit::{img, Hsla, Image, ImageFormat, Img, Rgba, Styled};
use std::sync::{Arc, LazyLock};

gpui_kit::assets::icon_assets!(pub Extra, [Trash, Rss, Film, Package, Download, Share2, Zap, Store, SquarePen, SlidersHorizontal, KeyRound, Brain, Library, PanelLeft,
    Upload, FileDown, GitBranch, ExternalLink, Gauge, Box, ChevronDown, LockOpen, Lock, Check, RotateCcw, ChevronRight, Pause, Volume2, VolumeX]);

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Send,
    Store,
    Workflow,
    Package,
    Folder,
    Play,
    Trash,
    Download,
    Eye,
    Plus,
    Attach,
    /// AI: a brain, not sparkles
    Brain,
    User,
    Terminal,
    Info,
    Search,
    Done,
    Failed,
    Running,
    Film,
    Settings,
    Rss,
    Share,
    Key,
    Palette,
    Sliders,
    Github,
    Library,
    PanelLeft,
    Import,
    Export,
    Git,
    External,
    Usage,
    Cube,
    ChevronDown,
    Lock,
    Unlock,
    Check,
    Reset,
    ChevronRight,
    Pause,
    Volume,
    Mute,
}

fn name(i: Icon) -> IconName {
    match i {
        Icon::Send => IconName::ArrowUp,
        Icon::Store => IconName::Store,
        Icon::Workflow => IconName::Zap,
        Icon::Package => IconName::Package,
        Icon::Folder => IconName::FolderOpen,
        Icon::Play => IconName::Play,
        Icon::Trash => IconName::Trash,
        Icon::Download => IconName::Download,
        Icon::Eye => IconName::Eye,
        Icon::Plus => IconName::SquarePen,
        Icon::Attach => IconName::Plus,
        Icon::Brain => IconName::Brain,
        Icon::User => IconName::User,
        Icon::Terminal => IconName::SquareTerminal,
        Icon::Info => IconName::Info,
        Icon::Search => IconName::Search,
        Icon::Done => IconName::CircleCheck,
        Icon::Failed => IconName::CircleX,
        Icon::Running => IconName::LoaderCircle,
        Icon::Film => IconName::Film,
        Icon::Settings => IconName::Settings,
        Icon::Rss => IconName::Rss,
        Icon::Share => IconName::Share2,
        Icon::Key => IconName::KeyRound,
        Icon::Palette => IconName::Palette,
        Icon::Sliders => IconName::SlidersHorizontal,
        Icon::Github => IconName::Github,
        Icon::Library => IconName::Library,
        Icon::PanelLeft => IconName::PanelLeft,
        Icon::Import => IconName::Upload,
        Icon::Export => IconName::FileDown,
        Icon::Git => IconName::GitBranch,
        Icon::External => IconName::ExternalLink,
        Icon::Usage => IconName::Gauge,
        Icon::Cube => IconName::Box,
        Icon::ChevronDown => IconName::ChevronDown,
        Icon::Lock => IconName::Lock,
        Icon::Unlock => IconName::LockOpen,
        Icon::Check => IconName::Check,
        Icon::Reset => IconName::RotateCcw,
        Icon::ChevronRight => IconName::ChevronRight,
        Icon::Pause => IconName::Pause,
        Icon::Volume => IconName::Volume2,
        Icon::Mute => IconName::VolumeX,
    }
}

/// An icon in one colour.
pub fn icon(i: Icon, color: Hsla) -> Glyph {
    Glyph::new(name(i)).text_color(color)
}

fn css(c: Hsla) -> String {
    let c = Rgba::from(c);
    format!("#{:02X}{:02X}{:02X}", (c.r * 255.).round() as u8, (c.g * 255.).round() as u8, (c.b * 255.).round() as u8)
}

/// The mark, its first half in `ink`, its cut half red.
pub fn mark(ink: Hsla) -> Img {
    type Drawn = std::sync::Mutex<Vec<(String, Arc<Image>)>>;
    static DRAWN: LazyLock<Drawn> = LazyLock::new(Default::default);
    let key = css(ink);
    let mut d = DRAWN.lock().unwrap();
    let image = match d.iter().find(|x| x.0 == key) {
        Some(x) => x.1.clone(),
        None => {
            let i = Arc::new(Image::from_bytes(ImageFormat::Svg, include_str!("../../assets/mark.svg").replace("#0B0B0C", &key).into_bytes()));
            d.push((key, i.clone()));
            i
        }
    };
    img(image)
}

/// A provider's logo as t3code draws it (assets/providers, from t3code and Simple Icons), in one colour.
pub fn logo(name: &str, color: Hsla, page: Hsla) -> Option<Img> {
    macro_rules! svg {
        ($f:literal) => {
            include_str!(concat!("../../assets/providers/", $f, ".svg"))
        };
    }
    let src = match name {
        "claude-code" => svg!("claude"),
        "codex" | "openai" => svg!("openai"),
        "anthropic" => svg!("anthropic"),
        "cursor" => svg!("cursor"),
        "grok" => svg!("grok"),
        "fx" => svg!("fx"),
        "opencode" => svg!("opencode"),
        "antigravity" => svg!("antigravity"),
        "kiro" => svg!("kiro"),
        "muse" => svg!("muse"),
        "pi" => svg!("pi"),
        "gemini-cli" | "gemini" => svg!("gemini"),
        "acp" => svg!("acp"),
        "openrouter" => svg!("openrouter"),
        "ollama" => svg!("ollama"),
        _ => return None,
    };
    let (c, p) = (css(color), css(page));
    let key = format!("{name}{c}{p}");
    picture(&key, || {
        Some((ImageFormat::Svg, recolor(src, &c, &p).into_bytes()))
    })
}

/// A logo's dark parts in `c`, its light ones in `p`. Through placeholders: an ink that is itself white (Omarchy's
/// dark themes) must not be turned into the page by the second pass.
fn recolor(src: &str, c: &str, p: &str) -> String {
    let mut s = src.to_string();
    for dark in ["#000000", "#000", "currentColor", "black", "#9046FF"] {
        s = s.replace(&format!("\"{dark}\""), "\"@INK@\"");
    }
    for light in ["#ffffff", "#FFFFFF", "#fff", "#FFF", "white"] {
        s = s.replace(&format!("\"{light}\""), "\"@PAGE@\"");
    }
    // a logo with no colour of its own takes it on its root
    if !s.contains("@INK@") {
        s = s.replacen("<svg", "<svg fill=\"@INK@\"", 1);
    }
    s.replace("@INK@", c).replace("@PAGE@", p)
}

/// A micro illustration (assets/illustrations): flat, pastel, outlined, one for each kind of thing and each step of
/// the welcome.
pub fn illustration(name: &str) -> Img {
    macro_rules! art {
        ($($f:literal),*) => {
            match name { $($f => include_str!(concat!("../../assets/illustrations/", $f, ".svg")),)* _ => include_str!("../../assets/illustrations/styles.svg") }
        };
    }
    let src = art!("styles", "topics", "automations", "skills", "scene", "library", "search", "community", "hello", "brain", "agents", "accounts", "folder", "ready");
    // on a dark page the pale backdrop would turn muddy: it is drawn whole, as a sticker
    let dark = ILLUSTRATIONS_DARK.load(std::sync::atomic::Ordering::Relaxed);
    picture(&format!("art:{name}:{dark}"), || {
        let mut s = src.to_string();
        if dark {
            for o in [".35", ".4", ".45", ".5", ".55"] {
                s = s.replace(&format!("fill-opacity=\"{o}\" stroke=\"none\""), "fill-opacity=\".92\" stroke=\"none\"");
            }
        }
        Some((ImageFormat::Svg, s.into_bytes()))
    }).expect("an illustration always draws")
}

/// Whether the page is dark, for the illustrations; theme::apply sets it.
pub static ILLUSTRATIONS_DARK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(test)]
mod art_tests {
    #[test]
    fn every_illustration_has_a_backdrop_to_swap() {
        for f in std::fs::read_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/illustrations")).unwrap().flatten() {
            let s = std::fs::read_to_string(f.path()).unwrap();
            assert!(s.starts_with("<svg") && s.contains("stroke=\"none\"/>"), "{:?}", f.path());
        }
    }
}

/// A pack's cover (its preview SVG, or its own file), read once.
pub fn picture(key: &str, bytes: impl FnOnce() -> Option<(ImageFormat, Vec<u8>)>) -> Option<Img> {
    type Pictures = std::sync::Mutex<std::collections::HashMap<String, Option<Arc<Image>>>>;
    static PICTURES: LazyLock<Pictures> = LazyLock::new(Default::default);
    let image = PICTURES.lock().unwrap().entry(key.to_string()).or_insert_with(|| bytes().map(|(f, b)| Arc::new(Image::from_bytes(f, b)))).clone();
    image.map(img)
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_logo_takes_one_colour() {
        for f in ["claude", "openai", "anthropic", "cursor", "grok", "fx", "opencode", "antigravity", "kiro", "muse", "pi", "gemini", "acp", "openrouter", "ollama"] {
            let s = std::fs::read_to_string(format!("{}/assets/providers/{f}.svg", env!("CARGO_MANIFEST_DIR"))).unwrap();
            assert!(s.starts_with("<svg") && !s.contains("ANTIGRAVITY_ICON"), "{f}");
        }
    }

    #[test]
    fn a_white_ink_stays_ink() {
        // Omarchy's vantablack: white ink on a black page; the logo must not turn into the page
        let s = super::recolor(r##"<svg fill="#000"><path/></svg>"##, "#FFFFFF", "#0A0A0A");
        assert!(s.contains("fill=\"#FFFFFF\"") && !s.contains("#0A0A0A"), "{s}");
        let s = super::recolor("<svg><path/></svg>", "#FFFFFF", "#0A0A0A");
        assert!(s.starts_with("<svg fill=\"#FFFFFF\""), "{s}");
    }

    #[test]
    fn the_mark_takes_the_ink_and_keeps_its_red() {
        let s = include_str!("../../assets/mark.svg");
        assert!(s.contains("#0B0B0C") && s.contains("#E5332A"), "the mark's colours are what mark() swaps");
    }
}
