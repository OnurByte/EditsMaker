//! Google Fonts by name: a caption style's `font` that this machine lacks is fetched once ("Bebas Neue",
//! "Montserrat Black") into the edits font folder, where captions (libass) and drawn text (resvg) find it.

use crate::util::{home, run};
use std::path::PathBuf;

const WEIGHTS: &[(&str, u32)] = &[("Thin", 100), ("ExtraLight", 200), ("Extra Light", 200), ("Light", 300), ("Regular", 400), ("Medium", 500),
    ("SemiBold", 600), ("Semi Bold", 600), ("Bold", 700), ("ExtraBold", 800), ("Extra Bold", 800), ("Black", 900)];

pub fn dir() -> PathBuf {
    home().join(".local/share/fonts/edits")
}

/// "Montserrat Black" -> ("Montserrat", 900); a name without a weight is the regular one.
fn split(font: &str) -> (&str, u32) {
    WEIGHTS.iter().find_map(|(w, n)| font.strip_suffix(w).and_then(|f| f.strip_suffix(' ')).map(|f| (f, *n))).unwrap_or((font, 400))
}

fn slug(font: &str) -> String {
    font.chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_lowercase()
}

/// On this machine already: fontconfig knows the family or full name, or edits fetched it before.
fn installed(font: &str) -> bool {
    let fc = run(None, "fc-list", &["--format", "%{family}\n%{fullname}\n"])
        .is_ok_and(|l| l.lines().flat_map(|x| x.split(',')).any(|n| n.trim().eq_ignore_ascii_case(font)));
    fc || dir().join(format!("{}.ttf", slug(font))).is_file()
}

/// The .ttf links in a Google Fonts css2 answer (curl gets truetype, browsers get woff2).
fn ttf_urls(css: &str) -> Vec<String> {
    regex::Regex::new(r"url\((https://fonts\.gstatic\.com/[^)]+\.ttf)\)").unwrap().captures_iter(css).map(|c| c[1].to_string()).collect()
}

/// Fetches `font` from Google Fonts unless it is here. Ok(false): already here.
pub fn add(font: &str) -> Result<bool, String> {
    let font = font.trim();
    if font.is_empty() || installed(font) {
        return Ok(false);
    }
    let (family, weight) = split(font);
    let w = if weight == 400 { String::new() } else { format!(":wght@{weight}") };
    let url = format!("https://fonts.googleapis.com/css2?family={}{w}", family.replace(' ', "+"));
    // asked Google once already and it had none (a system font like Adwaita Sans on another machine): ask no more
    let none = dir().join(format!("{}.none", slug(font)));
    if none.is_file() {
        return Ok(false);
    }
    let css = match run(None, "curl", &["-fsSL", "--max-time", "60", "-A", "curl/8", &url]) {
        Ok(css) => css,
        // Google answered 400: no such family. Offline is not remembered, so it is tried again later
        Err(e) if e.contains("returned error") => {
            let _ = std::fs::create_dir_all(dir()).and_then(|_| std::fs::write(&none, ""));
            return Err(format!("\"{font}\" is not on this machine or on Google Fonts"));
        }
        Err(e) => return Err(format!("\"{font}\" could not be fetched from Google Fonts: {e}")),
    };
    let ttf = ttf_urls(&css).into_iter().next().ok_or(format!("\"{font}\": Google Fonts gave no .ttf"))?;
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let to = dir().join(format!("{}.ttf", slug(font)));
    run(None, "curl", &["-fsSL", "--max-time", "120", "-o", &to.to_string_lossy(), &ttf])?;
    let _ = run(None, "fc-cache", &["-f", &dir().to_string_lossy()]);
    Ok(true)
}

/// `add`, saying what happened; a font that is nowhere leaves the renderer's fallback, so it only warns.
pub fn ensure(font: &str) {
    if cfg!(test) {
        return;
    }
    match add(font) {
        Ok(true) => eprintln!("edits: font \"{font}\" installed from Google Fonts"),
        Ok(false) => {}
        Err(e) => eprintln!("edits: {e}; a fallback font is used"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_css() {
        assert_eq!(split("Montserrat Black"), ("Montserrat", 900));
        assert_eq!(split("Open Sans Semi Bold"), ("Open Sans", 600));
        assert_eq!(split("Bebas Neue"), ("Bebas Neue", 400));
        assert_eq!(split("Blackletter"), ("Blackletter", 400));
        assert_eq!(slug("Montserrat Black"), "montserratblack");
        let css = "@font-face {\n  src: url(https://fonts.gstatic.com/s/bebasneue/v16/JTUS.ttf) format('truetype');\n}";
        assert_eq!(ttf_urls(css), ["https://fonts.gstatic.com/s/bebasneue/v16/JTUS.ttf"]);
    }
}
