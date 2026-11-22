//! --bg, --behind and --outline: who is in front, cut out frame by frame by Robust Video Matting (people, with soft
//! edges and hair), and the picture rebuilt around them: another background, big words behind them, a glowing outline.
//! The mask is worked out at about 512 px wide on the CPU through ONNX Runtime (both fetched on first use), then laid
//! over the full frame by ffmpeg.
use crate::util::run;
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone, Debug, PartialEq)]
pub enum Bg {
    Blur,
    /// an ffmpeg colour: a name or 0xRRGGBB
    Colour(String),
    /// a picture or a video, played round
    File(PathBuf),
}

#[derive(Clone, Debug, Default)]
pub struct Subject {
    pub bg: Option<Bg>,
    /// (words, from, to) drawn between the background and the person
    pub behind: Vec<(String, f64, f64)>,
    /// the outline's colour
    pub outline: Option<String>,
}

impl Subject {
    pub fn any(&self) -> bool {
        self.bg.is_some() || !self.behind.is_empty() || self.outline.is_some()
    }
}

const NAMES: &[&str] = &["white", "black", "red", "yellow", "green", "cyan", "blue", "magenta", "orange", "pink", "purple", "gold"];

/// A colour as ffmpeg takes it: a name, or #RRGGBB as 0xRRGGBB.
pub fn colour(s: &str) -> Option<String> {
    let t = s.trim().to_lowercase();
    if NAMES.contains(&t.as_str()) {
        return Some(t);
    }
    let hex = t.trim_start_matches('#');
    (hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit())).then(|| format!("0x{hex}"))
}

/// --bg: blur, a colour, or a picture or video (a file, or a background, image or video from the library).
pub fn parse_bg(s: &str) -> Result<Bg, String> {
    if s.trim().eq_ignore_ascii_case("blur") {
        return Ok(Bg::Blur);
    }
    if let Some(c) = colour(s) {
        return Ok(Bg::Colour(c));
    }
    crate::library::find(&["background", "image", "video", "meme"], s).map(Bg::File)
        .ok_or(format!("--bg {s}: give blur, a colour (white, #ff2a6d) or a picture or video (a file or a library background)"))
}

/// --behind TEXT[@FROM-TO]: words behind the person, the whole edit long without times.
pub fn parse_behind(s: &str) -> Result<(String, f64, f64), String> {
    if s.contains('@') {
        return crate::wrap::parse_caption(s).map_err(|e| e.replace("--caption", "--behind"));
    }
    let t = s.trim();
    if t.is_empty() {
        return Err("--behind needs words".into());
    }
    Ok((t.to_string(), 0.0, f64::MAX))
}

/// Huge capitals as an SVG `w` wide: one line, or two when long.
pub fn behind_svg(text: &str, w: u32) -> String {
    let up = crate::lang::upper(text.trim());
    let words: Vec<&str> = up.split_whitespace().collect();
    let lines: Vec<String> = if up.chars().count() > 10 && words.len() > 1 {
        let half = up.chars().count() / 2;
        let mut first = String::new();
        let mut k = 0;
        while k < words.len() && (first.chars().count() < half || first.is_empty()) {
            first = if first.is_empty() { words[k].to_string() } else { format!("{first} {}", words[k]) };
            k += 1;
        }
        let rest = words[k..].join(" ");
        if rest.is_empty() { vec![first] } else { vec![first, rest] }
    } else {
        vec![up.clone()]
    };
    let longest = lines.iter().map(|l| l.chars().count()).max().unwrap_or(1).max(1) as f64;
    let size = (w as f64 * 0.94 / (longest * 0.6)).min(w as f64 * 0.36);
    let lh = size * 0.98;
    let h = (lh * lines.len() as f64 + size * 0.25).ceil();
    let spans: String = lines.iter().enumerate()
        .map(|(i, l)| format!(r#"<tspan x="{:.0}" y="{:.0}">{}</tspan>"#, w as f64 / 2.0, size * 0.9 + i as f64 * lh, crate::wrap::esc(l))).collect();
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h:.0}" viewBox="0 0 {w} {h:.0}"><text text-anchor="middle" font-family="Anton, Bebas Neue, Impact, Adwaita Sans, Noto Sans, sans-serif" font-weight="900" font-size="{size:.0}" letter-spacing="-1" fill="white">{spans}</text></svg>"#)
}

/// The size the mask is worked out at: about 512 px on the short side's scale, both sides a multiple of 16.
pub fn work_size(w: u32, h: u32) -> (u32, u32) {
    let k = (512.0 / w.max(1) as f64).min(1.0);
    let r16 = |x: f64| ((x / 16.0).round() as u32).max(1) * 16;
    (r16(w as f64 * k), r16(h as f64 * k))
}

/// The person in every frame of `clip` as a grey video (white = in front), at the work size.
fn mask(clip: &Path, out: &Path) -> Result<usize, String> {
    let lib = crate::tools::onnxruntime()?;
    let model = crate::ort::Model::load(&lib, &crate::tools::matting_model()?)?;
    let (w, h) = crate::fetch::dims(clip);
    let (mw, mh) = work_size(w, h);
    // the model looks at a smaller copy (about 384 px on the long side) and refines at full work size
    let mut ratio = [(384.0 / mw.max(mh) as f32).min(1.0)];
    let fps = format!("{:.3}", crate::fetch::fps(clip));
    let c = clip.to_string_lossy();
    let mut dec = Command::new("ffmpeg").args(["-v", "error", "-i", &c, "-vf", &format!("scale={mw}:{mh},format=rgb24"), "-f", "rawvideo", "-"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().map_err(|e| format!("ffmpeg: {e}"))?;
    let mut enc = Command::new("ffmpeg").args(["-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "gray", "-s", &format!("{mw}x{mh}"), "-r", &fps, "-i", "-", "-c:v", "ffv1", &out.to_string_lossy()])
        .stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e| format!("ffmpeg: {e}"))?;
    let n = (mw * mh) as usize;
    let (mut rgb, mut src, mut grey) = (vec![0u8; n * 3], vec![0f32; n * 3], vec![0u8; n]);
    let (mut z1, mut z2, mut z3, mut z4) = ([0f32], [0f32], [0f32], [0f32]);
    let zero = [1i64, 1, 1, 1];
    let ratio_t = model.tensor(&mut ratio, &[1])?;
    let zeros = [model.tensor(&mut z1, &zero)?, model.tensor(&mut z2, &zero)?, model.tensor(&mut z3, &zero)?, model.tensor(&mut z4, &zero)?];
    let mut input = BufReader::with_capacity(n * 3, dec.stdout.take().ok_or("ffmpeg gave no frames")?);
    let mut sink = enc.stdin.take().ok_or("ffmpeg took no mask")?;
    let mut frames = 0;
    // the model carries what it saw in four recurrent states from frame to frame: steady edges, no flicker
    let mut state: Option<Vec<crate::ort::Value>> = None;
    let res = (|| -> Result<(), String> {
        while input.read_exact(&mut rgb).is_ok() {
            for i in 0..n {
                for ch in 0..3 {
                    src[ch * n + i] = rgb[i * 3 + ch] as f32 / 255.0;
                }
            }
            let frame = model.tensor(&mut src, &[1, 3, mh as i64, mw as i64])?;
            let r: [&crate::ort::Value; 4] = match &state {
                Some(s) => [&s[1], &s[2], &s[3], &s[4]],
                None => [&zeros[0], &zeros[1], &zeros[2], &zeros[3]],
            };
            let outs = model.run(&[("src", &frame), ("r1i", r[0]), ("r2i", r[1]), ("r3i", r[2]), ("r4i", r[3]), ("downsample_ratio", &ratio_t)], &["pha", "r1o", "r2o", "r3o", "r4o"])?;
            let (pha, _) = outs[0].floats()?;
            if pha.len() != n {
                return Err(format!("the subject model gave {} values for a {mw}x{mh} frame", pha.len()));
            }
            pha.iter().zip(grey.iter_mut()).for_each(|(a, g)| *g = (a.clamp(0.0, 1.0) * 255.0).round() as u8);
            sink.write_all(&grey).map_err(|e| format!("ffmpeg stopped taking the mask: {e}"))?;
            drop(frame);
            state = Some(outs);
            frames += 1;
        }
        Ok(())
    })();
    drop(sink);
    let _ = dec.kill();
    let _ = dec.wait();
    let done = enc.wait_with_output().map_err(|e| e.to_string())?;
    res?;
    if !done.status.success() {
        return Err(format!("ffmpeg could not write the mask: {}", String::from_utf8_lossy(&done.stderr).trim()));
    }
    if frames == 0 {
        return Err(format!("{}: no frames", clip.display()));
    }
    Ok(frames)
}

/// The filter graph over input 0 (the clip), 1 (the mask), then the background file (when there is one), then one
/// picture per --behind line. `total` is the clip's length.
pub fn graph(s: &Subject, w: u32, h: u32, total: f64) -> String {
    let fg = s.bg.is_some() || !s.behind.is_empty();
    let uses = usize::from(fg) + 2 * usize::from(s.outline.is_some());
    let mut g = format!("[1:v]scale={w}:{h}:flags=bicubic,format=gray,setsar=1");
    let masks: Vec<String> = (0..uses).map(|i| format!("m{i}")).collect();
    match uses {
        0 => g.push_str("[m0];"),
        1 => g.push_str("[m0];"),
        n => g.push_str(&format!(",split={n}{};", masks.iter().map(|m| format!("[{m}]")).collect::<String>())),
    }
    let mut next = 2;
    let (mut cur, fg_src) = match (&s.bg, fg) {
        (Some(Bg::Blur), _) => {
            g.push_str("[0:v]split[vb][vf];[vb]gblur=sigma=40,eq=brightness=-0.06[base];");
            ("base".to_string(), Some("vf".to_string()))
        }
        (Some(Bg::Colour(c)), _) => {
            g.push_str(&format!("color=c={c}:s={w}x{h}:r=30:d={total:.3}[base];"));
            ("base".into(), Some("0:v".into()))
        }
        (Some(Bg::File(_)), _) => {
            g.push_str(&format!("[2:v]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},setsar=1,fps=30[base];"));
            next = 3;
            ("base".into(), Some("0:v".into()))
        }
        (None, true) => {
            g.push_str("[0:v]split[vb][vf];");
            ("vb".into(), Some("vf".into()))
        }
        (None, false) => ("0:v".into(), None),
    };
    for (i, (_, a, b)) in s.behind.iter().enumerate() {
        let (a, b) = (a.max(0.0), b.min(total));
        g.push_str(&format!("[{}:v]format=rgba,fade=t=in:st={a:.3}:d=0.25:alpha=1[tx{i}];[{cur}][tx{i}]overlay=x=(W-w)/2:y=(H-h)*0.42:enable='between(t,{a:.3},{b:.3})'[bt{i}];", next + i));
        cur = format!("bt{i}");
    }
    if let Some(f) = fg_src {
        g.push_str(&format!("[{f}][m0]alphamerge[cut];[{cur}][cut]overlay=format=auto[front];"));
        cur = "front".into();
    }
    if let Some(col) = &s.outline {
        let (a, b) = (&masks[uses - 2], &masks[uses - 1]);
        g.push_str(&format!(
            "[{a}]gblur=sigma=5,lut=c0='if(gt(val,10),255,0)'[big];[big][{b}]blend=all_mode=subtract,gblur=sigma=2.5[ring];[ring]split[ra][rb];[rb]gblur=sigma=14[rbb];\
             [ra][rbb]blend=all_mode=lighten[glow];color=c={col}:s={w}x{h}:r=30:d={total:.3},format=yuva420p[col];[col][glow]alphamerge[line];[{cur}][line]overlay=format=auto[lined];"
        ));
        cur = "lined".into();
    }
    g.push_str(&format!("[{cur}]format=yuv420p[v]"));
    g
}

/// The person cut out of `clip` and the picture rebuilt around them, in place.
pub fn apply(clip: &Path, s: &Subject) -> Result<(), String> {
    if !s.any() {
        return Ok(());
    }
    let clip = &std::path::absolute(clip).map_err(|e| e.to_string())?;
    let (w, h) = crate::fetch::dims(clip);
    let total = crate::fetch::duration(clip)?;
    let started = std::time::Instant::now();
    let mask_file = clip.with_extension("mask.mkv");
    let frames = mask(clip, &mask_file).inspect_err(|_| {
        let _ = std::fs::remove_file(&mask_file);
    })?;
    eprintln!("edits: the person cut out of {frames} frames in {:.0}s", started.elapsed().as_secs_f64());
    let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into(), "-i".into(), clip.to_string_lossy().into(), "-i".into(), mask_file.to_string_lossy().into()];
    if let Some(Bg::File(f)) = &s.bg {
        let still = crate::fetch::duration(f).map_or(true, |d| d < 0.2) || ["png", "jpg", "jpeg", "webp", "bmp"].iter().any(|e| f.extension().is_some_and(|x| x.eq_ignore_ascii_case(e)));
        args.extend(if still { ["-loop", "1"] } else { ["-stream_loop", "-1"] }.iter().map(|x| x.to_string()));
        args.extend(["-t".into(), format!("{total:.3}"), "-i".into(), std::path::absolute(f).map_err(|e| e.to_string())?.to_string_lossy().into()]);
    }
    let mut temp: Vec<PathBuf> = vec![mask_file.clone()];
    for (i, (text, _, _)) in s.behind.iter().enumerate() {
        let png = clip.with_extension(format!("behind{i}.png"));
        crate::wrap::png(&behind_svg(text, w), w, &png)?;
        args.extend(["-loop".into(), "1".into(), "-t".into(), format!("{total:.3}"), "-i".into(), png.to_string_lossy().into()]);
        temp.push(png);
    }
    let tmp = clip.with_extension("subject.mp4");
    args.extend(["-filter_complex".into(), graph(s, w, h, total), "-map".into(), "[v]".into(), "-map".into(), "0:a?".into(), "-t".into(), format!("{total:.3}")]);
    args.extend(crate::util::h264(18));
    args.extend(["-c:a", "copy", "-movflags", "+faststart"].iter().map(|x| x.to_string()));
    args.push(tmp.to_string_lossy().into());
    let r = run(None, "ffmpeg", &args);
    temp.iter().for_each(|p| { let _ = std::fs::remove_file(p); });
    r?;
    std::fs::rename(&tmp, clip).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_and_backgrounds() {
        assert_eq!(colour("White"), Some("white".into()));
        assert_eq!(colour("#FF2A6D"), Some("0xff2a6d".into()));
        assert_eq!(colour("ff2a6d"), Some("0xff2a6d".into()));
        assert_eq!(colour("notacolour"), None);
        assert_eq!(parse_bg("blur"), Ok(Bg::Blur));
        assert_eq!(parse_bg("#000000"), Ok(Bg::Colour("0x000000".into())));
        assert_eq!(parse_behind("RUN"), Ok(("RUN".into(), 0.0, f64::MAX)));
        assert_eq!(parse_behind("RUN@1-3.5"), Ok(("RUN".into(), 1.0, 3.5)));
        assert!(parse_behind("  ").is_err());
    }

    #[test]
    fn work_sizes_are_small_and_even() {
        assert_eq!(work_size(1080, 1920), (512, 912));
        assert_eq!(work_size(320, 568), (320, 576));
    }

    #[test]
    fn graphs() {
        let bg = Subject { bg: Some(Bg::Blur), ..Default::default() };
        let g = graph(&bg, 1080, 1920, 10.0);
        assert!(g.contains("[0:v]split[vb][vf];[vb]gblur=sigma=40") && g.contains("[vf][m0]alphamerge[cut];[base][cut]overlay") && g.ends_with("[front]format=yuv420p[v]"), "{g}");
        let all = Subject { bg: Some(Bg::File("x.png".into())), behind: vec![("RUN".into(), 1.0, 3.0)], outline: Some("white".into()) };
        let g = graph(&all, 1080, 1920, 10.0);
        assert!(g.contains(",split=3[m0][m1][m2];") && g.contains("[2:v]scale=1080:1920") && g.contains("[3:v]format=rgba,fade=t=in:st=1.000") && g.contains("[base][tx0]overlay"), "{g}");
        assert!(g.contains("[bt0][cut]overlay") && g.contains("[m1]gblur=sigma=5") && g.contains("[big][m2]blend=all_mode=subtract") && g.contains("[front][line]overlay") && g.ends_with("[lined]format=yuv420p[v]"), "{g}");
        // the whole edit long: clamped to the clip
        let words = Subject { behind: vec![("RUN".into(), 0.0, f64::MAX)], ..Default::default() };
        let g = graph(&words, 1080, 1920, 10.0);
        assert!(g.contains("[0:v]split[vb][vf];[2:v]format=rgba") && g.contains("between(t,0.000,10.000)") && g.contains("[bt0][cut]overlay"), "{g}");
        let line = Subject { outline: Some("0xff0000".into()), ..Default::default() };
        let g = graph(&line, 720, 1280, 5.0);
        assert!(g.contains(",split=2[m0][m1];[m0]gblur") && g.contains("[0:v][line]overlay"), "{g}");
    }

    #[test]
    fn words_behind_fit_the_frame() {
        let one = behind_svg("run", 1080);
        assert!(one.contains(">RUN</tspan>") && one.contains("font-size=\"389\""), "{one}");
        let two = behind_svg("never give up today", 1080);
        assert_eq!(two.matches("<tspan").count(), 2, "{two}");
    }
}
