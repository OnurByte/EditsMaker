//! Your asset library (~/.config/edits/library): the sound effects, music, overlays, transitions, green screens,
//! memes, backgrounds and images of editing packs, kept in the pack's own folders and found by name. A pack's fonts
//! are installed for the system (~/.local/share/fonts/edits) so a caption style can name them.
use crate::util::{home, run};
use std::path::{Path, PathBuf};

pub fn dir() -> PathBuf {
    home().join(".config/edits/library")
}

pub const KINDS: &[&str] = &["sfx", "music", "overlay", "transition", "greenscreen", "background", "meme", "video", "image", "font"];

/// What a file is, from its type and the folders it sits in. None: not media.
pub fn kind(rel: &str) -> Option<&'static str> {
    let l = rel.to_lowercase();
    let has = |w: &str| l.contains(w);
    Some(match l.rsplit('.').next()? {
        "ttf" | "otf" => "font",
        "wav" | "mp3" | "ogg" | "m4a" | "flac" | "aac" => if has("sfx") || has("sound effect") || has("whoosh") { "sfx" } else { "music" },
        ext @ ("mp4" | "mov" | "webm" | "m4v" | "mkv" | "gif" | "png" | "jpg" | "jpeg" | "jfif" | "webp") => {
            if has("green") { "greenscreen" } else if has("overlay") { "overlay" } else if has("transition") { "transition" }
            else if has("background") { "background" } else if has("meme") { "meme" }
            else if matches!(ext, "mp4" | "mov" | "webm" | "m4v" | "mkv" | "gif") { "video" } else { "image" }
        }
        _ => return None,
    })
}

/// Every file under a folder, sorted.
pub fn walk(root: &Path) -> Vec<PathBuf> {
    let mut files = vec![];
    let mut stack = vec![root.to_path_buf()];
    while let Some(p) = stack.pop() {
        if p.is_dir() {
            stack.extend(std::fs::read_dir(&p).into_iter().flatten().flatten().map(|e| e.path()));
        } else {
            files.push(p);
        }
    }
    files.sort();
    files
}

/// (path within the library, file) of everything in it.
fn files() -> Vec<(String, PathBuf)> {
    let root = dir();
    walk(&root).into_iter().filter_map(|f| Some((f.strip_prefix(&root).ok()?.to_string_lossy().into_owned(), f))).collect()
}

/// The closest of `names` to a query: every word of the query is in it; the shortest such name wins.
fn closest<'a>(names: impl Iterator<Item = &'a str>, query: &str) -> Option<&'a str> {
    let words: Vec<String> = query.to_lowercase().split_whitespace().map(String::from).collect();
    names.filter(|n| !words.is_empty() && words.iter().all(|w| n.to_lowercase().contains(w))).min_by_key(|n| (n.len(), n.to_string()))
}

/// A file as given, or the library's closest match of one of these kinds.
pub fn find(kinds: &[&str], spec: &str) -> Option<PathBuf> {
    if Path::new(spec).is_file() {
        return Some(PathBuf::from(spec));
    }
    let all: Vec<(String, PathBuf)> = files().into_iter().filter(|(rel, _)| kind(rel).is_some_and(|k| kinds.contains(&k))).collect();
    let hit = closest(all.iter().map(|(r, _)| r.as_str()), spec)?;
    all.iter().find(|(r, _)| r == hit).map(|(_, f)| f.clone())
}

/// `edits library [KIND] [WORDS]`: what is in the library.
pub fn list(args: &[String]) {
    let (kinds, words): (Vec<&str>, Vec<&str>) = args.iter().map(String::as_str).partition(|a| KINDS.contains(a));
    let words: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();
    let all: Vec<(&str, String, PathBuf)> = files().into_iter().filter_map(|(rel, f)| Some((kind(&rel)?, rel, f))).collect();
    if args.is_empty() {
        for k in KINDS {
            let n = all.iter().filter(|x| x.0 == *k).count();
            if n > 0 {
                println!("{n:>5}  {k}");
            }
        }
        println!("\n{}\nedits library KIND [WORDS]  lists them; add a pack: edits library add <zip|folder>", dir().display());
        return;
    }
    for (k, _, f) in all.iter().filter(|(k, rel, _)| (kinds.is_empty() || kinds.contains(k)) && words.iter().all(|w| rel.to_lowercase().contains(w))) {
        println!("{k}\t{}", f.display());
    }
}

/// Copies a pack's media into the library and installs its fonts. Returns the pack's folder (unzipped if it was a
/// zip) for the preset importer.
pub fn add(src: &Path) -> Result<PathBuf, String> {
    let unzip = |zip: &Path, to: &Path| -> Result<(), String> {
        let _ = std::fs::remove_dir_all(to);
        run(None, "unzip", &["-q", "-o", &zip.to_string_lossy(), "-d", &to.to_string_lossy()]).map(|_| ())
    };
    let is_zip = |p: &Path| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip"));
    let tmp = std::env::temp_dir().join(format!("edits-pack-{}", std::process::id()));
    // a zip keeps its own top folder; a folder is filed under its name
    let (root, base) = if is_zip(src) {
        unzip(src, &tmp)?;
        (tmp.clone(), tmp.clone())
    } else if src.is_dir() {
        (src.to_path_buf(), std::path::absolute(src).ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_default())
    } else {
        return Err(format!("{}: not a zip or a folder", src.display()));
    };
    let fonts = home().join(".local/share/fonts/edits");
    let mut counts = std::collections::BTreeMap::<&str, usize>::new();
    let mut put = |f: &Path, rel: &str| -> Result<(), String> {
        let Some(k) = kind(rel) else { return Ok(()) };
        let to = if k == "font" { fonts.join(f.file_name().unwrap_or_default()) } else { dir().join(rel) };
        // already there with the same size: an earlier import
        if std::fs::metadata(&to).map(|m| m.len()).ok() != std::fs::metadata(f).map(|m| m.len()).ok() {
            std::fs::create_dir_all(to.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
            std::fs::copy(f, &to).map_err(|e| format!("{}: {e}", to.display()))?;
        }
        *counts.entry(k).or_default() += 1;
        Ok(())
    };
    for (i, f) in walk(&root).iter().enumerate() {
        let rel = std::path::absolute(f).ok().and_then(|a| a.strip_prefix(&base).ok().map(|r| r.to_string_lossy().into_owned())).unwrap_or_default();
        if is_zip(f) {
            // fonts come zipped inside packs
            let inner = tmp.with_extension(format!("inner{i}"));
            if unzip(f, &inner).is_ok() {
                for g in walk(&inner).iter().filter(|g| kind(&g.to_string_lossy()) == Some("font")) {
                    put(g, &g.to_string_lossy())?;
                }
            }
            let _ = std::fs::remove_dir_all(&inner);
        } else {
            put(f, &rel)?;
        }
    }
    if counts.contains_key("font") {
        let _ = run(None, "fc-cache", &["-f", &fonts.to_string_lossy()]);
    }
    println!("{}: {}", src.display(), if counts.is_empty() { "no media".into() } else { counts.iter().map(|(k, n)| format!("{n} {k}")).collect::<Vec<_>>().join(", ") });
    Ok(root)
}

/// A library clip laid over the finished edit: `NAME[@FROM[-TO]][:OPACITY]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Overlay {
    pub file: PathBuf,
    pub from: f64,
    pub to: Option<f64>,
    pub opacity: f64,
}

pub fn parse(spec: &str) -> Result<Overlay, String> {
    let bad = || format!("--overlay {spec}: use NAME[@FROM[-TO]][:OPACITY], like \"vhs texture:0.5\" or \"burn 3@4.2\"");
    let (rest, opacity) = match spec.rsplit_once(':') {
        Some((r, o)) if o.parse::<f64>().is_ok() => (r, o.parse::<f64>().unwrap().clamp(0.0, 1.0)),
        _ => (spec, 1.0),
    };
    let (name, from, to) = match rest.rsplit_once('@') {
        Some((n, t)) => {
            let (a, b) = t.split_once('-').map_or((t, None), |(a, b)| (a, Some(b)));
            (n, a.parse::<f64>().map_err(|_| bad())?, b.map(|b| b.parse::<f64>().map_err(|_| bad())).transpose()?)
        }
        None => (rest, 0.0, None),
    };
    let file = find(&["overlay", "transition", "greenscreen", "background", "meme", "video", "image"], name)
        .ok_or(format!("--overlay {name}: no such file or library clip (edits library overlay)"))?;
    Ok(Overlay { file, from, to, opacity })
}

/// How a clip goes over the picture: (input options, filter chain for it, joining filter).
/// Green screens are keyed, clips with transparency laid on top, everything else screened (black disappears).
/// `alpha`: the file has transparency. `len`: its length, None for a still.
#[allow(clippy::too_many_arguments)]
fn layer(o: &Overlay, i: usize, base: &str, out: &str, w: u32, h: u32, alpha: bool, len: Option<f64>) -> (Vec<String>, String) {
    let rel = o.file.to_string_lossy().to_lowercase();
    let k = kind(&rel).unwrap_or("video");
    // textures and backgrounds run under the whole edit; a transition, a meme or a green screen plays once
    let looped = len.is_some() && matches!(k, "overlay" | "background") && o.to.is_none() && o.from == 0.0;
    let end = o.to.or(if looped { None } else { len.map(|l| o.from + l) });
    let when = match end {
        Some(b) => format!(":enable='between(t,{:.3},{b:.3})'", o.from),
        None if o.from > 0.0 => format!(":enable='gte(t,{:.3})'", o.from),
        None => String::new(),
    };
    let mut input: Vec<String> = vec![];
    if len.is_none() {
        input.extend(["-loop".into(), "1".into()]);
    } else if looped {
        input.extend(["-stream_loop".into(), "-1".into()]);
    }
    if rel.ends_with(".webm") && alpha {
        input.extend(["-c:v".into(), "libvpx".into()]); // ffmpeg's own VP8 decoder drops the alpha
    }
    input.extend(["-i".into(), o.file.to_string_lossy().into_owned()]);
    let n = i + 1;
    let shift = format!("setpts=PTS+{:.3}/TB", o.from);
    let g = if k == "greenscreen" || alpha {
        let key = if k == "greenscreen" && !alpha { "chromakey=0x00FF00:0.18:0.08," } else { "" };
        format!("[{n}:v]{key}scale={w}:{h}:force_original_aspect_ratio=decrease,format=rgba,colorchannelmixer=aa={:.2},{shift}[l{i}];[{base}][l{i}]overlay=(W-w)/2:(H-h)/2:eof_action=pass{when}[{out}];", o.opacity)
    } else {
        format!("[{n}:v]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},setsar=1,fps=30,format=gbrp,{shift}[l{i}];[{base}]format=gbrp[b{i}];[b{i}][l{i}]blend=all_mode=screen:all_opacity={:.2}:eof_action=pass{when}[{out}];", o.opacity)
    };
    (input, g)
}

/// Lays the overlays over `clip` in place.
pub fn apply(clip: &Path, ov: &[Overlay]) -> Result<(), String> {
    if ov.is_empty() {
        return Ok(());
    }
    let (w, h) = crate::fetch::dims(clip);
    let total = crate::fetch::duration(clip)?;
    let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into(), "-i".into(), clip.to_string_lossy().into()];
    let mut g = String::new();
    for (i, o) in ov.iter().enumerate() {
        let f = o.file.to_string_lossy();
        let probe = run(None, "ffprobe", &["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=pix_fmt,nb_frames:stream_tags=alpha_mode", "-of", "default=nw=1", f.as_ref()]).unwrap_or_default();
        let get = |key: &str| probe.lines().find_map(|l| l.strip_prefix(key)).unwrap_or("").to_string();
        let alpha = get("TAG:alpha_mode=") == "1" || ["rgba", "bgra", "argb", "abgr", "yuva", "gbrap", "ya", "pal8"].iter().any(|p| get("pix_fmt=").starts_with(p));
        let still = matches!(kind(&f.to_lowercase()), Some("image")) || [".png", ".jpg", ".jpeg", ".jfif", ".webp"].iter().any(|e| f.to_lowercase().ends_with(e));
        let len = if still { None } else { Some(crate::fetch::duration(&o.file)?) };
        let base = if i == 0 { "0:v".to_string() } else { format!("x{}", i - 1) };
        let (input, chain) = layer(o, i, &base, &format!("x{i}"), w, h, alpha, len);
        args.extend(input);
        g.push_str(&chain);
    }
    g.push_str(&format!("[x{}]format=yuv420p[v]", ov.len() - 1));
    let tmp = clip.with_extension("overlay.mp4");
    args.extend(["-filter_complex".into(), g, "-map".into(), "[v]".into(), "-map".into(), "0:a?".into(), "-t".into(), format!("{total:.3}")]);
    args.extend(crate::util::h264(18));
    args.extend(["-pix_fmt", "yuv420p", "-c:a", "copy", "-movflags", "+faststart"].iter().map(|s| s.to_string()));
    args.push(tmp.to_string_lossy().into());
    run(None, "ffmpeg", &args)?;
    std::fs::rename(&tmp, clip).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_from_folders() {
        assert_eq!(kind("Pack/Sound/SFX(Sound Effects)/Whooshes/Main Swoosh.wav"), Some("sfx"));
        assert_eq!(kind("Pack/Sound/Phonk+Music/x.mp3"), Some("music"));
        assert_eq!(kind("Pack/Video/Transitions/raining_green_screen_.mp4"), Some("greenscreen"));
        assert_eq!(kind("Pack/Video/Transitions/Burn 3.mp4"), Some("transition"));
        assert_eq!(kind("Pack/Video/Overlays/Dirt Texture.png"), Some("overlay"));
        assert_eq!(kind("Pack/Image/People/a.JPG"), Some("image"));
        assert_eq!(kind("Pack/Fonts/riffic.ttf"), Some("font"));
        assert_eq!(kind("Pack/Presets/a.prfpset.xml"), None);
    }

    #[test]
    fn closest_name() {
        let names = ["p/Whooshes/Main Swoosh.wav", "p/Whooshes/fast woosh 2.mp3", "p/Swoosh/Swoosh 01 - 01.wav"];
        assert_eq!(closest(names.into_iter(), "main swoosh"), Some(names[0]));
        assert_eq!(closest(names.into_iter(), "FAST 2"), Some(names[1]));
        assert_eq!(closest(names.into_iter(), "swoosh"), Some(names[0]));
        assert_eq!(closest(names.into_iter(), "boom"), None);
        assert_eq!(closest(names.into_iter(), " "), None);
    }

    #[test]
    fn overlay_graphs() {
        let o = |f: &str, from: f64, to: Option<f64>| Overlay { file: PathBuf::from(f), from, to, opacity: 0.5 };
        // a texture loops under the whole edit, screened
        let (i, g) = layer(&o("Overlays/VHS Texture.mp4", 0.0, None), 0, "0:v", "x0", 1080, 1920, false, Some(20.0));
        assert_eq!(i, ["-stream_loop", "-1", "-i", "Overlays/VHS Texture.mp4"]);
        assert!(g.contains("blend=all_mode=screen:all_opacity=0.50:eof_action=pass[x0]") && !g.contains("enable"), "{g}");
        // a transition plays once where it is put
        let (i, g) = layer(&o("Transitions/Burn 3.mp4", 4.0, None), 1, "x0", "x1", 1080, 1920, false, Some(0.5));
        assert_eq!(i, ["-i", "Transitions/Burn 3.mp4"]);
        assert!(g.contains("setpts=PTS+4.000/TB[l1];[x0]format=gbrp[b1]") && g.contains("enable='between(t,4.000,4.500)'"), "{g}");
        // green is keyed out; a webm with alpha needs libvpx; a still is looped
        let (_, g) = layer(&o("Green Screens/a.mp4", 0.0, None), 0, "0:v", "x0", 1080, 1920, false, Some(3.0));
        assert!(g.contains("chromakey=0x00FF00") && g.contains("overlay=(W-w)/2:(H-h)/2:eof_action=pass"), "{g}");
        let (i, g) = layer(&o("Transitions/wipe.webm", 0.0, None), 0, "0:v", "x0", 1080, 1920, true, Some(2.0));
        assert!(i.contains(&"libvpx".to_string()) && !g.contains("chromakey"), "{g}");
        let (i, _) = layer(&o("Overlays/Dirt Texture.png", 0.0, Some(2.0)), 0, "0:v", "x0", 1080, 1920, true, None);
        assert_eq!(&i[..2], ["-loop", "1"]);
    }

    #[test]
    fn overlay_specs() {
        let f = std::env::temp_dir().join("edits-overlay-spec.mp4");
        std::fs::write(&f, "").unwrap();
        let s = f.to_string_lossy();
        assert_eq!(parse(&format!("{s}@1.5-3:0.4")).unwrap(), Overlay { file: f.clone(), from: 1.5, to: Some(3.0), opacity: 0.4 });
        assert_eq!(parse(&s).unwrap(), Overlay { file: f.clone(), from: 0.0, to: None, opacity: 1.0 });
        assert!(parse(&format!("{s}@x")).is_err());
        let _ = std::fs::remove_file(f);
    }
}
