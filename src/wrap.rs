//! What goes around and over a finished edit: words on screen drawn as text (emoji and all), the spoken line as
//! captions, math that flashes in on the cuts, one-frame subliminal lines, and an intro clip before it and an
//! outro card after it.
use crate::svg::{self, Sticker};
use crate::util::run;
use std::path::{Path, PathBuf};

/// `TEXT@FROM-TO`: words on screen from FROM to TO seconds of the edit.
pub fn parse_caption(s: &str) -> Result<(String, f64, f64), String> {
    let bad = || format!("--caption {s}: use TEXT@FROM-TO, like \"sayın barış özbay 🗿@0-3.5\"");
    let (text, range) = s.rsplit_once('@').ok_or_else(bad)?;
    let (a, b) = range.split_once('-').ok_or_else(bad)?;
    let (a, b): (f64, f64) = (a.trim().parse().map_err(|_| bad())?, b.trim().parse().map_err(|_| bad())?);
    if text.trim().is_empty() || b <= a {
        return Err(bad());
    }
    Ok((text.trim().to_string(), a, b))
}

/// `SECONDS:TEXT`: a line shown for two frames on black.
pub fn parse_flash(s: &str) -> Result<(f64, String), String> {
    let bad = || format!("--flash-text {s}: use SECONDS:TEXT, like \"5.2:dışarısı bir simülasyon\"");
    let (t, text) = s.split_once(':').ok_or_else(bad)?;
    let t: f64 = t.trim().parse().map_err(|_| bad())?;
    if text.trim().is_empty() {
        return Err(bad());
    }
    Ok((t, text.trim().to_string()))
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Words wrapped to lines of at most `max` characters.
fn wrap(text: &str, max: usize) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    for w in text.split_whitespace() {
        match lines.last_mut() {
            Some(l) if l.chars().count() + 1 + w.chars().count() <= max => {
                l.push(' ');
                l.push_str(w);
            }
            _ => lines.push(w.to_string()),
        }
    }
    lines
}

/// Heavy white words with a black outline, `w` wide, centred: the TikTok caption look. Emoji come out in colour.
pub fn text_svg(text: &str, w: u32, size: f64) -> String {
    let max = ((w as f64 * 0.9) / (size * 0.56)).floor().max(6.0) as usize;
    let lines = wrap(text, max);
    let lh = size * 1.18;
    let h = (lines.len() as f64 * lh + size * 0.5).ceil();
    let spans: String = lines.iter().enumerate()
        .map(|(i, l)| format!(r#"<tspan x="{:.0}" y="{:.0}">{}</tspan>"#, w as f64 / 2.0, size * 1.05 + i as f64 * lh, esc(l))).collect();
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h:.0}" viewBox="0 0 {w} {h:.0}"><text text-anchor="middle" font-family="Noto Sans, Adwaita Sans, DejaVu Sans, Liberation Sans, Arial, sans-serif" font-weight="900" font-size="{size:.0}" fill="white" stroke="black" stroke-width="{:.0}" stroke-linejoin="round" paint-order="stroke">{spans}</text></svg>"#,
        size * 0.15
    )
}

/// Text drawn as stickers (written next to `clip`, removed by the caller): `y` is where the middle of the words
/// sits, as a fraction of the frame height.
pub fn text_stickers(clip: &Path, items: &[(String, f64, f64)], y: f64, size_frac: f64, tag: &str) -> Result<Vec<(Sticker, PathBuf)>, String> {
    let (w, h) = crate::fetch::dims(clip);
    items.iter().enumerate().map(|(i, (text, a, b))| {
        let file = clip.with_extension(format!("{tag}{i}.svg"));
        std::fs::write(&file, text_svg(text, w, h as f64 * size_frac)).map_err(|e| e.to_string())?;
        Ok((Sticker { src: file.to_string_lossy().into_owned(), from: *a, to: *b, x: 0.5, y, size: 1.0, color: "white".into(), anim: String::new() }, file))
    }).collect()
}

const TEX: &[(&str, &str)] = &[("alpha", "α"), ("beta", "β"), ("gamma", "γ"), ("delta", "δ"), ("epsilon", "ε"), ("varepsilon", "ε"), ("zeta", "ζ"),
    ("eta", "η"), ("theta", "θ"), ("iota", "ι"), ("kappa", "κ"), ("lambda", "λ"), ("mu", "μ"), ("nu", "ν"), ("xi", "ξ"), ("pi", "π"), ("rho", "ρ"),
    ("sigma", "σ"), ("tau", "τ"), ("upsilon", "υ"), ("phi", "φ"), ("varphi", "φ"), ("chi", "χ"), ("psi", "ψ"), ("omega", "ω"), ("Gamma", "Γ"),
    ("Delta", "Δ"), ("Theta", "Θ"), ("Lambda", "Λ"), ("Xi", "Ξ"), ("Pi", "Π"), ("Sigma", "Σ"), ("Phi", "Φ"), ("Psi", "Ψ"), ("Omega", "Ω"),
    ("int", "∫"), ("iint", "∬"), ("iiint", "∭"), ("oint", "∮"), ("sum", "Σ"), ("prod", "∏"), ("infty", "∞"), ("partial", "∂"), ("nabla", "∇"),
    ("cdot", "·"), ("times", "×"), ("div", "÷"), ("pm", "±"), ("mp", "∓"), ("leq", "≤"), ("le", "≤"), ("geq", "≥"), ("ge", "≥"), ("neq", "≠"),
    ("ne", "≠"), ("approx", "≈"), ("equiv", "≡"), ("sim", "∼"), ("propto", "∝"), ("to", "→"), ("rightarrow", "→"), ("leftarrow", "←"),
    ("Rightarrow", "⇒"), ("implies", "⇒"), ("iff", "⇔"), ("mapsto", "↦"), ("in", "∈"), ("notin", "∉"), ("subset", "⊂"), ("subseteq", "⊆"),
    ("cup", "∪"), ("cap", "∩"), ("emptyset", "∅"), ("varnothing", "∅"), ("forall", "∀"), ("exists", "∃"), ("neg", "¬"), ("land", "∧"),
    ("lor", "∨"), ("hbar", "ℏ"), ("ell", "ℓ"), ("circ", "∘"), ("degree", "°"), ("ldots", "…"), ("cdots", "⋯"), ("dots", "…"), ("langle", "⟨"),
    ("rangle", "⟩"), ("quad", "  "), ("qquad", "    ")];

/// Superscript and subscript forms; a script with a character missing here is written ^(..) / _(..).
const SUP: (&str, &str) = ("0123456789+-−=()abcdefghijklmnoprstuvwxyz", "⁰¹²³⁴⁵⁶⁷⁸⁹⁺⁻⁻⁼⁽⁾ᵃᵇᶜᵈᵉᶠᵍʰⁱʲᵏˡᵐⁿᵒᵖʳˢᵗᵘᵛʷˣʸᶻ");
const SUB: (&str, &str) = ("0123456789+-−=()aehijklmnoprstuvx", "₀₁₂₃₄₅₆₇₈₉₊₋₋₌₍₎ₐₑₕᵢⱼₖₗₘₙₒₚᵣₛₜᵤᵥₓ");

/// A LaTeX formula as Unicode text, since the math layer draws text, not TeX: Greek, operators, \frac, \sqrt,
/// \mathbb, ^ and _. shortcut: no stacked fractions, matrices or big operators with limits above and below;
/// a real TeX engine would break the download size.
pub fn tex(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    tex_until(&c, &mut 0, false)
}

fn tex_until(c: &[char], i: &mut usize, close: bool) -> String {
    let mut out = String::new();
    while *i < c.len() {
        let ch = c[*i];
        *i += 1;
        match ch {
            '}' if close => return out,
            '{' => out.push_str(&tex_until(c, i, true)),
            '^' | '_' => out.push_str(&script(&tex_arg(c, i), ch == '^')),
            '\\' => out.push_str(&tex_command(c, i)),
            _ => out.push(ch),
        }
    }
    out
}

fn tex_command(c: &[char], i: &mut usize) -> String {
    let start = *i;
    while *i < c.len() && c[*i].is_ascii_alphabetic() {
        *i += 1;
    }
    let name: String = c[start..*i].iter().collect();
    if name.is_empty() {
        // \, \; \! spacing, \{ \} braces
        let Some(&ch) = c.get(*i) else { return String::new() };
        *i += 1;
        return match ch { ',' | ';' | ':' | ' ' => " ".into(), '!' => String::new(), _ => ch.to_string() };
    }
    let paren = |a: String| if a.chars().count() > 1 && !a.starts_with('(') { format!("({a})") } else { a };
    match name.as_str() {
        "frac" | "dfrac" | "tfrac" => {
            let (a, b) = (tex_arg(c, i), tex_arg(c, i));
            format!("{}/{}", paren(a), paren(b))
        }
        "sqrt" => format!("√{}", paren(tex_arg(c, i))),
        "mathbb" => tex_arg(c, i).chars().map(|x| match x { 'R' => 'ℝ', 'N' => 'ℕ', 'Z' => 'ℤ', 'Q' => 'ℚ', 'C' => 'ℂ', 'P' => 'ℙ', o => o }).collect(),
        "text" | "mathrm" | "mathbf" | "mathit" | "operatorname" | "boldsymbol" => tex_arg(c, i),
        "left" | "right" | "displaystyle" | "limits" => String::new(),
        _ => TEX.iter().find(|(n, _)| *n == name).map_or(name.clone(), |(_, u)| u.to_string()),
    }
}

/// The next argument: a {group}, a \command or one character.
fn tex_arg(c: &[char], i: &mut usize) -> String {
    while c.get(*i) == Some(&' ') {
        *i += 1;
    }
    match c.get(*i) {
        Some('{') => {
            *i += 1;
            tex_until(c, i, true)
        }
        Some('\\') => {
            *i += 1;
            tex_command(c, i)
        }
        Some(&ch) => {
            *i += 1;
            ch.to_string()
        }
        None => String::new(),
    }
}

fn script(a: &str, up: bool) -> String {
    let (from, to) = if up { SUP } else { SUB };
    let to: Vec<char> = to.chars().collect();
    // a script inside a script (e^{-x^2}) is already raised: it stays as it is
    let mapped: Option<String> = a.chars().map(|x| from.chars().position(|f| f == x).map(|k| to[k]).or(to.contains(&x).then_some(x))).collect();
    match mapped {
        Some(m) if !m.is_empty() => m,
        _ if a.chars().count() == 1 => format!("{}{a}", if up { '^' } else { '_' }),
        _ => format!("{}({a})", if up { '^' } else { '_' }),
    }
}

/// Formulas and sine waves, drawn twice the frame size so they can drift. Sarcastic ones in Turkish. `own`
/// formulas (already Unicode, see `tex`) replace the built-in ones.
fn math_svg(w: u32, h: u32, lang: &str, own: &[String]) -> String {
    let mut f: Vec<&str> = vec!["e^(iπ) + 1 = 0", "∫ f(x) dx", "Σ 1/n² = π²/6", "∇ · E = ρ/ε₀", "sin²x + cos²x = 1", "ζ(s) = Σ 1/nˢ", "x → ∞",
        "π ≈ 3.14159", "ℏ = h/2π", "∞ − ∞ = ?", "y = ax² + bx + c", "det(A) = 0", "∂u/∂t = α∇²u", "lim x→0 sin x / x = 1", "∮ F · ds", "√2 ∉ ℚ"];
    if lang == "tr" {
        f.extend(["f(dışarı) = 0", "Σ sorumluluk = ∅", "∫ ev dt = huzur", "P(çıkmak) ≈ 0.0001", "ev² = mc²", "∂(hayat)/∂t = 0",
            "∀ gün : uyu()", "dışarı ∉ ℝ", "log(iş) → −∞", "Σ fatura = 0", "Δ motivasyon < 0"]);
    }
    if !own.is_empty() {
        // repeated to the built-in count, so the layer is as full as ever
        f = own.iter().map(String::as_str).cycle().take(own.len().max(16)).collect();
    }
    let (cw, ch) = (w * 2, h * 2);
    // a fixed scatter, so every render of the same edit looks the same
    let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
    let mut rnd = |m: f64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % 10_000) as f64 / 10_000.0 * m
    };
    let mut s = format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{cw}" height="{ch}" viewBox="0 0 {cw} {ch}">"#);
    for t in &f {
        let size = h as f64 * (0.032 + rnd(0.03));
        let (x, y, r) = (rnd(cw as f64 * 0.85), size + rnd(ch as f64 - size * 1.5), rnd(40.0) - 20.0);
        s.push_str(&format!(r#"<text x="{x:.0}" y="{y:.0}" transform="rotate({r:.1} {x:.0} {y:.0})" font-family="Noto Serif, DejaVu Serif, serif" font-style="italic" font-size="{size:.0}" fill="white">{}</text>"#, esc(t)));
    }
    for k in 0..6 {
        let base = ch as f64 * (0.1 + 0.16 * k as f64);
        let amp = h as f64 * (0.06 + 0.02 * (k % 3) as f64);
        let per = w as f64 * (0.05 + 0.025 * k as f64);
        let pts: String = (0..=cw / 12).map(|i| {
            let x = (i * 12) as f64;
            format!("{x:.0},{:.0} ", base + amp * (x / per + k as f64).sin())
        }).collect();
        s.push_str(&format!(r#"<polyline points="{pts}" fill="none" stroke="white" stroke-width="{:.1}"/>"#, h as f64 * 0.0028));
    }
    s.push_str("</svg>");
    s
}

/// An svg string as a PNG `w` wide.
fn png(svg_src: &str, w: u32, file: &Path) -> Result<(), String> {
    let s = file.with_extension("svg");
    std::fs::write(&s, svg_src).map_err(|e| e.to_string())?;
    let r = svg::rasterize(&Sticker { src: s.to_string_lossy().into_owned(), from: 0.0, to: 1.0, x: 0.5, y: 0.5, size: 1.0, color: "white".into(), anim: String::new() }, w, file);
    let _ = std::fs::remove_file(&s);
    r.map(|_| ())
}

/// When the math shows: short bursts on each cut, between cuts and on a steady pulse; invisible otherwise.
pub fn bursts(cuts: &[f64], total: f64) -> Vec<f64> {
    let mut t: Vec<f64> = cuts.to_vec();
    t.extend(cuts.windows(2).map(|w| (w[0] + w[1]) / 2.0));
    t.extend((0..(total / 1.37) as usize).map(|i| 0.9 + i as f64 * 1.37));
    t.retain(|x| *x >= 0.0 && *x < total - 0.2);
    t.sort_by(f64::total_cmp);
    // at least 0.6 s apart, else fast presets (a cut every 0.4 s) keep the math on screen
    let mut kept: Vec<f64> = vec![];
    for x in t {
        if kept.last().is_none_or(|l| x - l >= 0.6) {
            kept.push(x);
        }
    }
    kept
}

/// The math layer and the words (an ASS script: the spoken line, subliminal flashes) over `clip`, in place.
pub fn math(clip: &Path, cuts: &[f64], math: bool, formulas: &[String], words: &str, lang: &str) -> Result<(), String> {
    let words = if words.contains("Dialogue:") { words } else { "" };
    if !math && words.is_empty() {
        return Ok(());
    }
    let clip = &std::path::absolute(clip).map_err(|e| e.to_string())?;
    let (w, h) = crate::fetch::dims(clip);
    let total = crate::fetch::duration(clip)?;
    let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into(), "-i".into(), clip.to_string_lossy().into()];
    let mut temp: Vec<PathBuf> = vec![];
    let mut g = String::new();
    let mut last = "0:v".to_string();
    if math {
        let tile = clip.with_extension("math.png");
        png(&math_svg(w, h, lang, formulas), w * 2, &tile)?;
        args.extend(["-loop".into(), "1".into(), "-t".into(), format!("{total:.3}"), "-i".into(), tile.to_string_lossy().into()]);
        temp.push(tile);
        let k: String = bursts(cuts, total).iter().map(|t| format!("between(t,{t:.2},{:.2})", t + 0.17)).collect::<Vec<_>>().join("+");
        // the screened math is built in (fast); a per-pixel expression with every burst in it took minutes
        g.push_str(&format!(
            "[1:v]fps=30,crop={w}:{h}:'t*28':'t*17',format=gray[fm];[0:a]showwaves=s={w}x{}:mode=cline:rate=30:colors=white:scale=sqrt,pad={w}:{h}:0:{},format=gray[sw];\
             [fm][sw]blend=all_mode=lighten:shortest=1,format=gbrp[ml];[0:v]split[b0][b1];[b0]format=gbrp[bg];[bg][ml]blend=all_mode=screen:all_opacity=0.85:shortest=1,format=yuv420p[scr];\
             [b1][scr]overlay=0:0:shortest=1:enable='{k}'[m0];",
            h * 3 / 10, h * 35 / 100
        ));
        last = "m0".into();
    }
    if !words.is_empty() {
        let ass = clip.with_extension("words.ass");
        std::fs::write(&ass, words).map_err(|e| e.to_string())?;
        // ffmpeg runs beside the clip, so the filter names the file alone and needs no path escaping
        g.push_str(&format!("[{last}]ass='{}'[w0];", ass.file_name().unwrap().to_string_lossy().replace('\'', "")));
        temp.push(ass);
        last = "w0".into();
    }
    g.push_str(&format!("[{last}]format=yuv420p[v]"));
    let tmp = clip.with_extension("math.mp4");
    args.extend(["-filter_complex".into(), g, "-map".into(), "[v]".into(), "-map".into(), "0:a?".into(), "-t".into(), format!("{total:.3}")]);
    args.extend(crate::util::h264(18));
    args.extend(["-c:a", "copy", "-movflags", "+faststart"].iter().map(|s| s.to_string()));
    args.push(tmp.to_string_lossy().into());
    let r = run(clip.parent(), "ffmpeg", &args);
    temp.iter().for_each(|p| { let _ = std::fs::remove_file(p); });
    r?;
    std::fs::rename(&tmp, clip).map_err(|e| e.to_string())
}

/// `VIDEOID@FROM-TO[/PICTURE]` (its sound from FROM to TO, its picture from PICTURE, else FROM), or a file.
#[derive(Debug, Clone, PartialEq)]
pub enum Intro {
    Moment { id: String, from: f64, to: f64, picture: Option<f64> },
    File(PathBuf),
}

pub fn parse_intro(s: &str) -> Result<Intro, String> {
    if Path::new(s).is_file() {
        return Ok(Intro::File(PathBuf::from(s)));
    }
    let bad = || format!("--intro {s}: use VIDEOID@FROM-TO[/PICTURE_FROM] or a video file");
    let (id, range) = s.rsplit_once('@').ok_or_else(bad)?;
    let (range, picture) = match range.split_once('/') {
        Some((r, p)) => (r, Some(p.trim().parse::<f64>().map_err(|_| bad())?)),
        None => (range, None),
    };
    let (a, b) = range.split_once('-').ok_or_else(bad)?;
    let (from, to): (f64, f64) = (a.trim().parse().map_err(|_| bad())?, b.trim().parse().map_err(|_| bad())?);
    if id.trim().is_empty() || to <= from || to - from > 20.0 {
        return Err(bad());
    }
    Ok(Intro::Moment { id: id.trim().to_string(), from, to, picture })
}

/// The settings for what goes before and after the edit.
#[derive(Debug, Clone, Default)]
pub struct Around {
    pub intro: Option<Intro>,
    /// a song under the intro (library name, file or link), quiet, cut with a record scratch at the end
    pub intro_bed: Option<String>,
    /// words over the intro
    pub intro_caption: Option<String>,
    /// "edits" for the EditsMaker card, else a video file or library clip
    pub outro: Option<String>,
}

/// The EditsMaker card, `secs` long, as a PNG.
fn outro_svg(w: u32, h: u32, lang: &str) -> String {
    let (top, sub, foot) = if lang == "tr" {
        ("bu edit tek komutla yapıldı", "link at, edit al.", "ücretsiz  ·  4 MB  ·  AI agent ile çalışır")
    } else {
        ("this edit took one command", "link in, edits out.", "free  ·  4 MB  ·  works with AI agents")
    };
    let u = h as f64 / 1080.0;
    let t = |y: f64, size: f64, weight: u32, fill: &str, s: &str| format!(r#"<text x="{:.0}" y="{:.0}" text-anchor="middle" font-family="Noto Sans, Adwaita Sans, DejaVu Sans, Liberation Sans, Arial, sans-serif" font-weight="{weight}" font-size="{:.0}" fill="{fill}">{}</text>"#, w as f64 / 2.0, h as f64 / 2.0 + y * u, size * u, esc(s));
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><rect width="{w}" height="{h}" fill="black"/>{}{}{}{}{}</svg>"#,
        t(-170.0, 44.0, 700, "#aaaaaa", top), t(10.0, 150.0, 900, "white", "EditsMaker"), t(110.0, 54.0, 700, "white", sub),
        t(200.0, 40.0, 700, "#aaaaaa", "github.com/OnurByte/EditsMaker"), t(260.0, 32.0, 700, "#888888", foot)
    )
}

/// Video and sound inputs for the intro and its filter (output [iv][ia]), and its length.
#[allow(clippy::too_many_arguments)]
fn intro_part(it: &Intro, around: &Around, w: u32, h: u32, lufs: f64, cache: &Path, n0: usize, args: &mut Vec<String>) -> Result<(String, f64), String> {
    match it {
        Intro::File(f) => {
            let len = crate::fetch::duration(f)?;
            args.extend(["-i".into(), f.to_string_lossy().into()]);
            Ok((format!("[{n0}:v]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},fps=30,setsar=1,format=yuv420p[iv];[{n0}:a]aresample=48000,aformat=channel_layouts=stereo[ia];"), len))
        }
        Intro::Moment { id, from, to, picture } => {
            let len = to - from;
            let pic = picture.unwrap_or(*from);
            // only the stretch the intro needs, never the whole (often hour-long) video
            let (lo, hi) = (pic.min(*from), pic.max(*from) + len);
            if hi - lo > 60.0 {
                return Err("--intro: keep the picture within a minute of the words".into());
            }
            let work = cache.join(crate::fetch::safe(id));
            std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
            let file = work.join(format!("intro-{lo:.2}-{hi:.2}.mp4"));
            if !file.is_file() {
                crate::util::yt(&["--no-warnings", "--no-progress", "--no-playlist", "-f", "bv*[height<=1080]+ba/b[height<=1080]/b", "--download-sections", &format!("*{lo:.2}-{hi:.2}"),
                    "--force-keyframes-at-cuts", "--merge-output-format", "mp4", "-o", &file.to_string_lossy(), "--", &format!("https://www.youtube.com/watch?v={id}")])?;
            }
            let f = file.to_string_lossy().into_owned();
            args.extend(["-ss".into(), format!("{:.3}", pic - lo), "-t".into(), format!("{len:.3}"), "-i".into(), f.clone()]);
            args.extend(["-ss".into(), format!("{:.3}", from - lo), "-t".into(), format!("{len:.3}"), "-i".into(), f]);
            let mut g = format!(
                "[{n0}:v]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},fps=30,setsar=1,format=yuv420p[iv];\
                 [{}:a]aresample=48000,aformat=channel_layouts=stereo,loudnorm=I={lufs:.1}:TP=-1.5,afade=t=out:st={:.3}:d=0.1,adelay=150|150[vo];",
                n0 + 1, (len - 0.25).max(0.0)
            );
            let mut mix = vec!["[vo]".to_string()];
            let mut n = n0 + 2;
            if let Some(bed) = &around.intro_bed {
                let song = crate::library::find(&["music"], bed).or_else(|| Some(PathBuf::from(bed)).filter(|p| p.is_file()))
                    .ok_or(format!("--intro-bed {bed}: no such file or library song"))?;
                args.extend(["-i".into(), song.to_string_lossy().into()]);
                g.push_str(&format!("[{n}:a]aresample=48000,aformat=channel_layouts=stereo,atrim=0:{:.3},volume=0.32,afade=t=out:st={:.3}:d=0.15,apad[bed];", len - 0.45, (len - 0.6).max(0.0)));
                mix.push("[bed]".into());
                n += 1;
                // the record scratch that stops the bed
                if let Some(sc) = crate::library::find(&["sfx"], "scratch") {
                    args.extend(["-i".into(), sc.to_string_lossy().into()]);
                    g.push_str(&format!("[{n}:a]aresample=48000,aformat=channel_layouts=stereo,atrim=0:0.5,volume=0.9,adelay={0}|{0}[sc];", ((len - 0.5) * 1000.0) as u64));
                    mix.push("[sc]".into());
                }
            }
            g.push_str(&format!("{}amix=inputs={}:normalize=0:duration=longest,apad,atrim=0:{len:.3}[ia];", mix.join(""), mix.len()));
            Ok((g, len))
        }
    }
}

/// The intro before and the outro after `clip`, in place. Returns how long the intro is (captions move by it).
pub fn around(clip: &Path, a: &Around, lang: &str, lufs: f64, cache: &Path) -> Result<f64, String> {
    if a.intro.is_none() && a.outro.is_none() {
        return Ok(0.0);
    }
    let (w, h) = crate::fetch::dims(clip);
    let total = crate::fetch::duration(clip)?;
    let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into()];
    let mut g = String::new();
    let mut parts: Vec<String> = vec![];
    let mut temp: Vec<PathBuf> = vec![];
    let mut n = 0;
    let mut intro_len = 0.0;
    if let Some(it) = &a.intro {
        // the intro is made on its own first, so its words can be drawn over it
        let mut ia: Vec<String> = vec!["-v".into(), "error".into(), "-y".into()];
        let (ig, len) = intro_part(it, a, w, h, lufs, cache, 0, &mut ia)?;
        let file = clip.with_extension("intro.mp4");
        ia.extend(["-filter_complex".into(), ig.trim_end_matches(';').to_string(), "-map".into(), "[iv]".into(), "-map".into(), "[ia]".into(), "-t".into(), format!("{len:.3}")]);
        ia.extend(crate::util::h264(18));
        ia.extend(["-c:a", "aac", "-b:a", "192k", "-ar", "48000"].iter().map(|s| s.to_string()));
        ia.push(file.to_string_lossy().into());
        run(None, "ffmpeg", &ia)?;
        if let Some(words) = &a.intro_caption {
            let st = text_stickers(&file, &[(words.clone(), 0.15, len)], 0.11, 0.09, "icap")?;
            let r = svg::apply(&file, &st.iter().map(|s| s.0.clone()).collect::<Vec<_>>());
            st.iter().for_each(|s| { let _ = std::fs::remove_file(&s.1); });
            r?;
        }
        args.extend(["-i".into(), file.to_string_lossy().into()]);
        g.push_str("[0:v]fps=30,setsar=1,format=yuv420p[v0];[0:a]aresample=48000,aformat=channel_layouts=stereo[a0];");
        parts.push("[v0][a0]".into());
        temp.push(file);
        intro_len = len;
        n = 1;
    }
    args.extend(["-i".into(), clip.to_string_lossy().into()]);
    g.push_str(&format!("[{n}:v]fps=30,setsar=1,format=yuv420p[vm];[{n}:a]aresample=48000,aformat=channel_layouts=stereo{}[am];",
        if a.outro.is_some() { format!(",afade=t=out:st={:.3}:d=0.25", (total - 0.25).max(0.0)) } else { String::new() }));
    parts.push("[vm][am]".into());
    n += 1;
    match a.outro.as_deref() {
        Some("edits") => {
            let card = clip.with_extension("outro.png");
            png(&outro_svg(w, h, lang), w, &card)?;
            args.extend(["-loop".into(), "1".into(), "-t".into(), "3.2".into(), "-i".into(), card.to_string_lossy().into()]);
            g.push_str(&format!("[{n}:v]fps=30,setsar=1,fade=t=in:d=0.25,fade=t=out:st=2.9:d=0.3,format=yuv420p[vo];"));
            temp.push(card);
            // a bass hit under the card when the library has one, else quiet
            match crate::library::find(&["sfx"], "bass drop") {
                Some(b) => {
                    args.extend(["-i".into(), b.to_string_lossy().into()]);
                    g.push_str(&format!("[{}:a]aresample=48000,aformat=channel_layouts=stereo,apad,atrim=0:3.2,volume=0.8,afade=t=out:st=2.5:d=0.7[ao];", n + 1));
                }
                None => g.push_str("anullsrc=r=48000:cl=stereo,atrim=0:3.2[ao];"),
            }
            parts.push("[vo][ao]".into());
        }
        Some(o) => {
            let f = crate::library::find(&["video", "meme", "background", "overlay", "transition"], o).or_else(|| Some(PathBuf::from(o)).filter(|p| p.is_file()))
                .ok_or(format!("--outro {o}: \"edits\", a video file or a library clip"))?;
            args.extend(["-i".into(), f.to_string_lossy().into()]);
            g.push_str(&format!("[{n}:v]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},fps=30,setsar=1,format=yuv420p[vo];[{n}:a]aresample=48000,aformat=channel_layouts=stereo[ao];"));
            parts.push("[vo][ao]".into());
        }
        None => {}
    }
    g.push_str(&format!("{}concat=n={}:v=1:a=1[v][a]", parts.join(""), parts.len()));
    let tmp = clip.with_extension("around.mp4");
    args.extend(["-filter_complex".into(), g, "-map".into(), "[v]".into(), "-map".into(), "[a]".into()]);
    args.extend(crate::util::h264(18));
    args.extend(["-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-movflags", "+faststart"].iter().map(|s| s.to_string()));
    args.push(tmp.to_string_lossy().into());
    let r = run(clip.parent(), "ffmpeg", &args);
    temp.iter().for_each(|p| { let _ = std::fs::remove_file(p); });
    r?;
    std::fs::rename(&tmp, clip).map_err(|e| e.to_string())?;
    Ok(intro_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specs() {
        assert_eq!(parse_caption("sayın barış özbay 🗿@4.2-7.8").unwrap(), ("sayın barış özbay 🗿".into(), 4.2, 7.8));
        assert!(parse_caption("no time").is_err() && parse_caption("x@3-2").is_err());
        assert_eq!(parse_flash("5.2:duş = opsiyonel").unwrap(), (5.2, "duş = opsiyonel".into()));
        assert!(parse_flash("x:y").is_err());
        assert_eq!(parse_intro("FqsAALrRhj4@3351.55-3354.85/3344.4").unwrap(),
            Intro::Moment { id: "FqsAALrRhj4".into(), from: 3351.55, to: 3354.85, picture: Some(3344.4) });
        assert!(parse_intro("abc@5-40").is_err(), "an intro longer than 20 s is a mistake");
    }

    #[test]
    fn wrapping_and_bursts() {
        assert_eq!(wrap("Ya hiçbir şey umurumda değil", 14), vec!["Ya hiçbir şey", "umurumda değil"]);
        let b = bursts(&[2.0, 4.0], 6.0);
        assert!(b.contains(&2.0) && b.contains(&3.0), "{b:?}");
        assert!(b.windows(2).all(|w| w[1] - w[0] >= 0.6), "{b:?}");
        let fast: Vec<f64> = (1..30).map(|i| i as f64 * 0.43).collect();
        assert!(bursts(&fast, 13.0).len() <= 22, "a cut every 0.43 s still leaves dark gaps");
        assert!(b.iter().all(|t| *t < 5.8));
    }

    #[test]
    fn latex_as_unicode() {
        assert_eq!(tex(r"e^{i\pi} + 1 = 0"), "e^(iπ) + 1 = 0");
        assert_eq!(tex(r"\int_0^\infty e^{-x^2}dx"), "∫₀^∞ e⁻ˣ²dx");
        assert_eq!(tex(r"\frac{a+b}{2} \leq \sqrt{ab}"), "(a+b)/2 ≤ √(ab)");
        assert_eq!(tex(r"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}"), "Σₙ₌₁^∞ 1/(n²) = (π²)/6");
        assert_eq!(tex(r"\sqrt{2} \notin \mathbb{Q}"), "√2 ∉ ℚ");
        assert_eq!(tex(r"\lim_{x \to 0} \frac{\sin x}{x}"), "lim_(x → 0) (sin x)/x");
        assert_eq!(tex(r"\left( \hbar \right)"), "( ℏ )");
    }

    #[test]
    fn text_draws_with_emoji() {
        let dir = std::env::temp_dir().join("edits-wrap-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.png");
        png(&text_svg("avenox 🤡", 1080, 96.0), 1080, &p).unwrap();
        let px = resvg::tiny_skia::Pixmap::load_png(&p).unwrap();
        let mut db = resvg::usvg::fontdb::Database::new();
        db.load_system_fonts();
        let has = |name: &str| db.faces().any(|f| f.families.iter().any(|(n, _)| n.contains(name)));
        if ["Noto Sans", "Adwaita Sans", "DejaVu Sans", "Liberation Sans", "Arial"].iter().any(|n| has(n)) {
            assert!(px.pixels().iter().any(|c| c.alpha() > 200), "the words are drawn");
        }
        // the clown's red nose: colour emoji really drawn, not a blank box. shortcut: only Noto Color Emoji (Linux)
        // draws; Apple's sbix emoji come out blank, upgrade if macOS users need emoji captions
        if has("Noto Color Emoji") {
            assert!(px.pixels().iter().any(|c| c.red() > 180 && c.green() < 90 && c.blue() < 90 && c.alpha() > 200));
        }
        png(&math_svg(540, 540, "tr", &[tex(r"\int_0^\infty e^{-x^2}dx")]), 1080, &p).unwrap();
        png(&outro_svg(540, 540, "en"), 540, &p).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }
}
