//! Kubai Icons (MIT, icons/LICENSE), their duotone drawings in one brand colour: the main tone in the colour, the
//! light tone in the same colour faded. No gradients or shadows (BRAND.md § 6). And the EditsMaker mark.
use gpui_kit::{img, Hsla, Image, ImageFormat, Img, Rgba};
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Icon {
    Send,
    Store,
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
    Workflow,
    Mark,
}

fn source(i: Icon) -> &'static str {
    macro_rules! file {
        ($f:literal) => {
            include_str!(concat!("../icons/", $f, ".svg"))
        };
    }
    match i {
        Icon::Send => file!("send"),
        Icon::Store => file!("store"),
        Icon::Package => file!("package"),
        Icon::Folder => file!("folder"),
        Icon::Play => file!("play"),
        Icon::Trash => file!("trash"),
        Icon::Download => file!("download"),
        Icon::Eye => file!("eye"),
        Icon::Plus => file!("circle-plus"),
        Icon::Done => file!("circle-check"),
        Icon::Failed => file!("circle-x"),
        Icon::Running => file!("loader"),
        Icon::Film => file!("film"),
        Icon::Settings => file!("settings"),
        Icon::Workflow => file!("workflow"),
        Icon::Mark => include_str!("../../../assets/mark.svg"),
    }
}

fn css(c: Hsla) -> String {
    let c = Rgba::from(c);
    format!("#{:02X}{:02X}{:02X}", (c.r * 255.).round() as u8, (c.g * 255.).round() as u8, (c.b * 255.).round() as u8)
}

/// The SVG in `color`: kubai's main tone (#5A76F2) becomes it, its light tone (#BDC8FA) the same faded; the mark's
/// ink half becomes it and its red half stays red.
pub fn recolor(svg: &str, color: Hsla) -> String {
    let c = css(color);
    svg.replace("#5A76F2", &c).replace("fill=\"#BDC8FA\"", &format!("fill=\"{c}\" fill-opacity=\"0.4\""))
        .replace("stroke=\"#BDC8FA\"", &format!("stroke=\"{c}\" stroke-opacity=\"0.4\"")).replace("#0B0B0C", &c)
}

type Drawn = Mutex<HashMap<(Icon, String), Arc<Image>>>;
static CACHE: LazyLock<Drawn> = LazyLock::new(Default::default);

pub fn icon(i: Icon, color: Hsla) -> Img {
    let key = (i, css(color));
    let image = CACHE.lock().unwrap().entry(key).or_insert_with(|| Arc::new(Image::from_bytes(ImageFormat::Svg, recolor(source(i), color).into_bytes()))).clone();
    img(image)
}

/// A picture of a pack (its preview SVG, or its own file), drawn once.
pub fn picture(key: &str, bytes: impl FnOnce() -> Option<(ImageFormat, Vec<u8>)>) -> Option<Img> {
    type Pictures = Mutex<HashMap<String, Option<Arc<Image>>>>;
    static PICTURES: LazyLock<Pictures> = LazyLock::new(Default::default);
    let image = PICTURES.lock().unwrap().entry(key.to_string()).or_insert_with(|| bytes().map(|(f, b)| Arc::new(Image::from_bytes(f, b)))).clone();
    image.map(img)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_take_one_brand_colour() {
        let ink: Hsla = gpui_kit::rgb(0xF4F1EA).into();
        let s = recolor(source(Icon::Store), ink);
        assert!(s.contains("#F4F1EA") && !s.contains("#5A76F2") && !s.contains("#BDC8FA"), "{s}");
        let m = recolor(source(Icon::Mark), ink);
        assert!(m.contains("#F4F1EA") && m.contains("#E5332A"));
        for i in [Icon::Send, Icon::Store, Icon::Play, Icon::Eye, Icon::Running] {
            assert!(!recolor(source(i), ink).contains("#5A76F2"));
        }
    }
}
