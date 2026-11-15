//! Drawn graphics over the edit, the way editors point at things: an arrow at the face, a ring around the detail,
//! a scribbled underline, a "?" when it makes no sense. SVG stickers (built in, or your own .svg file) are drawn
//! with resvg and laid over the finished clip at a time and place, popping in and fading out.
use crate::util::run;
use regex::Regex;
use resvg::{tiny_skia, usvg};
use std::path::{Path, PathBuf};

/// Built-in stickers on a 100x100 box. `currentColor` takes the colour asked for.
pub const STICKERS: &[(&str, &str, &str)] = &[
    ("arrow", "a drawn arrow pointing down-right at something", r#"<path d="M12 14 C40 10 70 30 78 74" fill="none" stroke="currentColor" stroke-width="9" stroke-linecap="round"/><path d="M58 64 L79 80 L90 55" fill="none" stroke="currentColor" stroke-width="9" stroke-linecap="round" stroke-linejoin="round"/>"#),
    ("circle", "a hand-drawn ring around something", r#"<path d="M52 8 C80 8 95 28 92 52 C89 78 66 93 44 91 C18 89 5 70 8 46 C11 24 30 9 60 12" fill="none" stroke="currentColor" stroke-width="6" stroke-linecap="round"/>"#),
    ("underline", "a scribbled underline", r#"<path d="M6 60 C25 48 40 70 58 56 C72 46 84 62 95 50" fill="none" stroke="currentColor" stroke-width="8" stroke-linecap="round"/>"#),
    ("cross", "a big X: wrong, no", r#"<path d="M18 18 L82 82 M82 18 L18 82" stroke="currentColor" stroke-width="13" stroke-linecap="round"/>"#),
    ("check", "a tick: right, yes", r#"<path d="M14 54 L40 78 L88 22" fill="none" stroke="currentColor" stroke-width="13" stroke-linecap="round" stroke-linejoin="round"/>"#),
    ("burst", "a comic burst behind a punchline", r##"<path d="M50 4 L58 32 L84 14 L70 42 L97 50 L70 58 L84 86 L58 68 L50 96 L42 68 L16 86 L30 58 L3 50 L30 42 L16 14 L42 32 Z" fill="currentColor" stroke="#fff" stroke-width="3" stroke-linejoin="round"/>"##),
    ("question", "a question mark: what?", r#"<path d="M30 32 C30 12 70 8 72 30 C74 48 50 50 50 66" fill="none" stroke="currentColor" stroke-width="12" stroke-linecap="round"/><circle cx="50" cy="86" r="8" fill="currentColor"/>"#),
    ("exclaim", "an exclamation mark", r#"<path d="M50 10 L50 64" stroke="currentColor" stroke-width="15" stroke-linecap="round"/><circle cx="50" cy="86" r="9" fill="currentColor"/>"#),
    ("heart", "a heart", r#"<path d="M50 88 C10 60 4 38 14 24 C26 8 46 14 50 30 C54 14 74 8 86 24 C96 38 90 60 50 88 Z" fill="currentColor"/>"#),
    ("sparkle", "a four-point sparkle", r#"<path d="M50 4 C54 40 60 46 96 50 C60 54 54 60 50 96 C46 60 40 54 4 50 C40 46 46 40 50 4 Z" fill="currentColor"/>"#),
];

pub fn names() -> Vec<&'static str> {
    STICKERS.iter().map(|s| s.0).collect()
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sticker {
    /// a built-in name or an .svg file
    pub src: String,
    pub from: f64,
    pub to: f64,
    /// centre, fractions of the frame
    pub x: f64,
    pub y: f64,
    /// width as a fraction of the frame's width
    pub size: f64,
    pub color: String,
}

/// `NAME|FILE[@FROM[-TO]][:X,Y[,SIZE]][#COLOUR]`, e.g. `arrow@1.2-2.5:0.6,0.3,0.25#ffd400` or `circle:0.5,0.4`.
pub fn parse(s: &str) -> Result<Sticker, String> {
    let re = Regex::new(r"^([^@#]+?)(?:@([\d.]+)(?:-([\d.]+))?)?(?::([\d.]+),([\d.]+)(?:,([\d.]+))?)?(?:#(\w+))?$").unwrap();
    let c = re.captures(s.trim()).ok_or_else(|| format!("--svg {s}: use NAME[@FROM-TO][:X,Y,SIZE][#COLOUR]"))?;
    let f = |i: usize, d: f64| c.get(i).and_then(|m| m.as_str().parse().ok()).unwrap_or(d);
    let src = c[1].to_string();
    if !STICKERS.iter().any(|t| t.0 == src) && !Path::new(&src).is_file() {
        return Err(format!("--svg {src}: not a sticker ({}) or an .svg file", names().join(", ")));
    }
    let from = f(2, 0.0);
    let color = c.get(7).map(|m| m.as_str()).unwrap_or("E5332A");
    // a hex colour gets its '#' back; names (white, yellow) stay as they are
    let color = if color.len() == 6 && color.chars().all(|c| c.is_ascii_hexdigit()) { format!("#{color}") } else { color.to_string() };
    Ok(Sticker { src, from, to: f(3, from + 1.5).max(from + 0.2), x: f(4, 0.5).clamp(0.0, 1.0), y: f(5, 0.4).clamp(0.0, 1.0), size: f(6, 0.3).clamp(0.02, 1.0), color })
}

fn source(s: &Sticker) -> Result<String, String> {
    let body = match STICKERS.iter().find(|t| t.0 == s.src) {
        Some(t) => format!(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">{}</svg>"#, t.2),
        None => std::fs::read_to_string(&s.src).map_err(|e| format!("{}: {e}", s.src))?,
    };
    Ok(body.replace("currentColor", &s.color))
}

/// The sticker as a PNG `px` wide (height from its own shape).
pub fn rasterize(s: &Sticker, px: u32, png: &Path) -> Result<(u32, u32), String> {
    let mut opt = usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let tree = usvg::Tree::from_str(&source(s)?, &opt).map_err(|e| format!("{}: {e}", s.src))?;
    let size = tree.size();
    let k = px as f32 / size.width();
    let (w, h) = (px.max(1), ((size.height() * k).round() as u32).max(1));
    let mut pm = tiny_skia::Pixmap::new(w, h).ok_or("sticker too big")?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(k, k), &mut pm.as_mut());
    pm.save_png(png).map_err(|e| e.to_string())?;
    Ok((w, h))
}

/// The overlay graph for stickers already drawn as inputs 1..: each slides up a little and fades in, then fades out.
pub fn graph(st: &[Sticker], sizes: &[(u32, u32)], w: u32, h: u32) -> String {
    let mut g = String::new();
    let mut last = "0:v".to_string();
    for (i, (s, (sw, sh))) in st.iter().zip(sizes).enumerate() {
        let (a, b) = (s.from, s.to);
        let out = if i + 1 == st.len() { "v".to_string() } else { format!("o{i}") };
        let (x, y) = (s.x * w as f64 - *sw as f64 / 2.0, s.y * h as f64 - *sh as f64 / 2.0);
        g.push_str(&format!(
            "[{}:v]format=rgba,fade=t=in:st={a:.2}:d=0.12:alpha=1,fade=t=out:st={:.2}:d=0.15:alpha=1[s{i}];[{last}][s{i}]overlay=x={x:.0}:y='{y:.0}+{:.0}*max(0,1-(t-{a:.2})/0.18)':enable='between(t,{a:.2},{b:.2})':eof_action=pass[{out}];",
            i + 1, (b - 0.15).max(a), h as f64 * 0.03
        ));
        last = out;
    }
    g.trim_end_matches(';').to_string()
}

/// Lays the stickers over `clip` in place.
pub fn apply(clip: &Path, st: &[Sticker]) -> Result<PathBuf, String> {
    if st.is_empty() {
        return Ok(clip.to_path_buf());
    }
    let (w, h) = crate::fetch::dims(clip);
    let tmp = clip.with_extension("svg.mp4");
    let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into(), "-i".into(), clip.to_string_lossy().into()];
    let mut sizes = vec![];
    let mut pngs = vec![];
    for (i, s) in st.iter().enumerate() {
        let png = clip.with_extension(format!("sticker{i}.png"));
        sizes.push(rasterize(s, (s.size * w as f64).round() as u32, &png)?);
        args.extend(["-loop".into(), "1".into(), "-t".into(), format!("{:.2}", s.to + 0.1), "-i".into(), png.to_string_lossy().into()]);
        pngs.push(png);
    }
    args.extend(["-filter_complex".into(), graph(st, &sizes, w, h), "-map".into(), "[v]".into(), "-map".into(), "0:a?".into()]);
    args.extend(["-c:v", "libx264", "-preset", "medium", "-crf", "18", "-pix_fmt", "yuv420p", "-c:a", "copy", "-movflags", "+faststart"].iter().map(|s| s.to_string()));
    args.push(tmp.to_string_lossy().into());
    let res = run(None, "ffmpeg", &args);
    pngs.iter().for_each(|p| { let _ = std::fs::remove_file(p); });
    res?;
    std::fs::rename(&tmp, clip).map_err(|e| e.to_string())?;
    Ok(clip.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs() {
        let s = parse("arrow@1.2-2.5:0.6,0.3,0.25#ffd400").unwrap();
        assert_eq!((s.from, s.to, s.x, s.y, s.size, s.color.as_str()), (1.2, 2.5, 0.6, 0.3, 0.25, "#ffd400"));
        let d = parse("circle").unwrap();
        assert_eq!((d.from, d.to, d.x, d.color.as_str()), (0.0, 1.5, 0.5, "#E5332A"));
        assert_eq!(parse("burst@3#white").unwrap().color, "white");
        assert!(parse("nosuch").is_err() && parse("arrow@x").is_err());
    }

    #[test]
    fn every_sticker_draws() {
        let dir = std::env::temp_dir();
        for (n, _, _) in STICKERS {
            let p = dir.join(format!("edits-test-{n}.png"));
            let (w, h) = rasterize(&parse(n).unwrap(), 200, &p).unwrap();
            assert_eq!((w, h), (200, 200));
            // something got drawn
            let px = tiny_skia::Pixmap::load_png(&p).unwrap();
            assert!(px.pixels().iter().any(|c| c.alpha() > 200), "{n} is empty");
            let _ = std::fs::remove_file(p);
        }
    }

    #[test]
    fn overlay_graph() {
        let st = [parse("arrow@1-2").unwrap(), parse("cross@3:0.2,0.8").unwrap()];
        let g = graph(&st, &[(300, 300), (200, 200)], 1080, 1920);
        assert!(g.starts_with("[1:v]format=rgba,fade=t=in:st=1.00") && g.contains("[0:v][s0]overlay=x=390") && g.contains("[o0][s1]overlay=x=116") && g.ends_with("[v]"), "{g}");
    }
}
