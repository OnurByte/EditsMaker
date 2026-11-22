//! Lucide icons, the ones t3code draws with: gpui-kit's own set and a few more embedded here, one colour each
//! (BRAND.md § 6). And the EditsMaker mark.
use gpui_kit::assets::IconName;
use gpui_kit::component::Icon as Glyph;
use gpui_kit::{img, Hsla, Image, ImageFormat, Img, Rgba, Styled};
use std::sync::{Arc, LazyLock};

gpui_kit::assets::icon_assets!(pub Extra, [Trash, Rss, Film, Package, Download, Share2, Zap, Store, SquarePen, SlidersHorizontal, KeyRound]);

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
    Done,
    Failed,
    Running,
    Film,
    Settings,
    Rss,
    Share,
    Key,
    Sun,
    Moon,
    Palette,
    Sliders,
    Github,
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
        Icon::Done => IconName::CircleCheck,
        Icon::Failed => IconName::CircleX,
        Icon::Running => IconName::LoaderCircle,
        Icon::Film => IconName::Film,
        Icon::Settings => IconName::Settings,
        Icon::Rss => IconName::Rss,
        Icon::Share => IconName::Share2,
        Icon::Key => IconName::KeyRound,
        Icon::Sun => IconName::Sun,
        Icon::Moon => IconName::Moon,
        Icon::Palette => IconName::Palette,
        Icon::Sliders => IconName::SlidersHorizontal,
        Icon::Github => IconName::Github,
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
        let mut s = src.to_string();
        for dark in ["#000000", "#000", "currentColor", "black", "#9046FF"] {
            s = s.replace(&format!("\"{dark}\""), &format!("\"{c}\""));
        }
        for light in ["#ffffff", "#FFFFFF", "#fff", "#FFF", "white"] {
            s = s.replace(&format!("\"{light}\""), &format!("\"{p}\""));
        }
        // a logo with no colour of its own takes it on its root
        if !s.contains(&c) {
            s = s.replacen("<svg", &format!("<svg fill=\"{c}\""), 1);
        }
        Some((ImageFormat::Svg, s.into_bytes()))
    })
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
    fn the_mark_takes_the_ink_and_keeps_its_red() {
        let s = include_str!("../../assets/mark.svg");
        assert!(s.contains("#0B0B0C") && s.contains("#E5332A"), "the mark's colours are what mark() swaps");
    }
}
