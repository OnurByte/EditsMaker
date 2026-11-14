//! Talking clips the way short-form editors cut them: pauses cut out (jump cuts), the frame punching in on
//! every other cut, captions word by word in a chosen style, a hook line on top, a whoosh on each cut and a boom
//! on the punchline, and an optional music bed that ducks under the voice.
use crate::pick::Moment;
use crate::render::{ass_time, grade, shape, upper_tr, Opts};
use crate::transcribe::Word;
use crate::util::run;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A caption look. Kept in presets/captions.json, so it can be changed like any preset.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CaptionStyle {
    pub name: String,
    #[serde(default)]
    pub about: String,
    pub font: String,
    /// text height as a fraction of the frame width
    pub size: f64,
    /// "#RRGGBB"
    pub color: String,
    /// the word being said
    pub highlight: String,
    /// the punchline
    #[serde(default = "red")]
    pub punch: String,
    pub outline: f64,
    #[serde(default = "black")]
    pub outline_color: String,
    #[serde(default)]
    pub shadow: f64,
    /// a solid box behind the text
    #[serde(default)]
    pub boxed: bool,
    #[serde(default)]
    pub upper: bool,
    /// most words on screen at once
    pub words: usize,
    /// distance from the bottom as a fraction of the frame height
    pub position: f64,
    /// the said word pops up in size
    #[serde(default)]
    pub pop: bool,
}

fn red() -> String {
    "#E5332A".into()
}

fn black() -> String {
    "#000000".into()
}

pub fn styles() -> Vec<CaptionStyle> {
    let text = crate::presets::file("captions.json").unwrap_or_default();
    serde_json::from_str(&text).unwrap_or_else(|e| {
        eprintln!("edits: captions.json is not valid: {e}");
        vec![]
    })
}

pub fn style(name: &str) -> Result<CaptionStyle, String> {
    let all = styles();
    all.iter().find(|s| s.name == name).cloned()
        .ok_or(format!("no caption style \"{name}\"; there are: {}", all.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", ")))
}

/// What a talking clip gets.
#[derive(Clone)]
pub struct Talk {
    pub captions: Option<CaptionStyle>,
    pub jumpcuts: bool,
    pub sfx: bool,
    /// the line shown on top for the first seconds
    pub hook: Option<String>,
    /// (music file, seconds into it)
    pub bed: Option<(PathBuf, f64)>,
}

/// "#RRGGBB" as ASS wants it: &H00BBGGRR&.
fn ass_color(hex: &str) -> String {
    let h = hex.trim_start_matches('#');
    if h.len() != 6 {
        return "&H00FFFFFF&".into();
    }
    format!("&H00{}{}{}&", &h[4..6], &h[2..4], &h[0..2]).to_uppercase()
}

/// The stretches to keep: speech, with every pause over `gap` seconds cut out (a little air left at each edge).
pub fn segments(words: &[Word], m: &Moment, gap: f64) -> Vec<(f64, f64)> {
    let ws: Vec<&Word> = words.iter().filter(|w| w.s >= m.start - 0.05 && w.e <= m.end + 0.05).collect();
    let (Some(first), Some(last)) = (ws.first(), ws.last()) else { return vec![(m.start, m.end)] };
    let pad = 0.08;
    let mut out = vec![];
    let mut a = (first.s - pad).max(m.start);
    for pair in ws.windows(2) {
        if pair[1].s - pair[0].e > gap {
            out.push((a, pair[0].e + pad));
            a = pair[1].s - pad;
        }
    }
    // the reaction after the last word stays
    out.push((a, (last.e + 0.4).min(m.end).max(a + 0.1)));
    // a sliver is not worth a cut: fold it into its neighbour
    let mut merged: Vec<(f64, f64)> = vec![];
    for s in out {
        match merged.last_mut() {
            Some(prev) if s.1 - s.0 < 0.3 || prev.1 - prev.0 < 0.3 => prev.1 = s.1,
            _ => merged.push(s),
        }
    }
    merged
}

/// Where source time `t` lands in the cut clip, if it was kept.
pub fn remap(t: f64, segs: &[(f64, f64)]) -> Option<f64> {
    let mut at = 0.0;
    for &(a, b) in segs {
        if t >= a - 1e-6 && t <= b + 1e-6 {
            return Some(at + (t - a).max(0.0));
        }
        at += b - a;
    }
    None
}

/// Captions and the hook as ASS, on the cut clip's clock.
pub fn captions(words: &[Word], m: &Moment, segs: &[(f64, f64)], st: Option<&CaptionStyle>, hook: Option<&str>, w: u32, h: u32) -> String {
    let mut s = format!(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: {w}\nPlayResY: {h}\nWrapStyle: 0\nScaledBorderAndShadow: yes\n\n\
         [V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
         Style: H,Adwaita Sans Black,{},&H00111111,&H00111111,&H00FFFFFF,&H00FFFFFF,-1,0,0,0,100,100,0,0,3,{},0,8,80,80,{},1\n",
        (w as f64 * 0.052) as u32, (w as f64 * 0.018) as u32, (h as f64 * 0.11) as u32
    );
    if let Some(st) = st {
        s.push_str(&format!(
            "Style: C,{},{},{},{},{},{},-1,0,0,0,100,100,0,0,{},{},{},2,70,70,{},1\n",
            st.font, (w as f64 * st.size) as u32, ass_color(&st.color), ass_color(&st.color), ass_color(&st.outline_color),
            if st.boxed { ass_color(&st.outline_color) } else { "&H99000000&".into() }, if st.boxed { 3 } else { 1 },
            if st.boxed { (w as f64 * st.size * 0.25) as u32 } else { st.outline as u32 }, st.shadow, (h as f64 * st.position) as u32
        ));
    }
    s.push_str("\n[Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n");
    let clean = |t: &str| t.chars().filter(|c| !matches!(c, '{' | '}' | '\\')).collect::<String>();
    let total: f64 = segs.iter().map(|x| x.1 - x.0).sum();
    if let Some(hk) = hook.filter(|x| !x.trim().is_empty()) {
        s.push_str(&format!("Dialogue: 1,{},{},H,,0,0,0,,{{\\fad(120,200)}}{}\n", ass_time(0.0), ass_time(2.8f64.min(total)), clean(hk.trim())));
    }
    let Some(st) = st else { return s };
    // words on the cut clock, in chunks that break on punctuation, pauses and the style's word count
    let ws: Vec<(f64, f64, String, bool)> = words.iter().filter(|x| x.s >= m.start - 0.05 && x.e <= m.end + 0.05)
        .filter_map(|x| Some((remap(x.s, segs)?, remap(x.e, segs).unwrap_or(remap(x.s, segs)? + 0.3), clean(&x.t), x.s >= m.punch - 0.05)))
        .collect();
    let mut chunks: Vec<Vec<usize>> = vec![];
    for i in 0..ws.len() {
        let fresh = chunks.last().is_none_or(|c| c.len() >= st.words.max(1) || ws[c[c.len() - 1]].2.ends_with(['.', '?', '!', ',']) || ws[i].0 - ws[c[c.len() - 1]].1 > 0.5);
        if fresh { chunks.push(vec![i]) } else { chunks.last_mut().unwrap().push(i) }
    }
    let shown = |t: &str| if st.upper { upper_tr(t) } else { t.to_string() };
    for (ci, c) in chunks.iter().enumerate() {
        // a chunk leaves when the next one comes: two lines never stack
        let next = chunks.get(ci + 1).map_or(f64::MAX, |n| ws[n[0]].0);
        let end_of_chunk = (ws[c[c.len() - 1]].1 + 0.25).min(next);
        for (k, &i) in c.iter().enumerate() {
            let a = ws[i].0;
            let b = c.get(k + 1).map_or(end_of_chunk, |&j| ws[j].0).max(a + 0.08).min(next.max(a + 0.08));
            let mark = if ws[i].3 { &st.punch } else { &st.highlight };
            let line: Vec<String> = c.iter().map(|&j| {
                let t = shown(ws[j].2.trim_end_matches([',', '.']));
                if j == i {
                    let grow = if st.pop { "\\fscx125\\fscy125\\t(0,90,\\fscx108\\fscy108)" } else { "" };
                    format!("{{\\c{}{grow}}}{t}{{\\r}}", ass_color(mark))
                } else {
                    t
                }
            }).collect();
            s.push_str(&format!("Dialogue: 0,{},{},C,,0,0,0,,{}\n", ass_time(a), ass_time(b), line.join(" ")));
        }
    }
    s
}

/// The sound effects, made with ffmpeg the first time (nothing to license): (whoosh, boom).
pub fn sfx(cache: &Path) -> Result<(PathBuf, PathBuf), String> {
    let dir = cache.join("sfx");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let make = |name: &str, src: &str, af: &str| -> Result<PathBuf, String> {
        let out = dir.join(name);
        if !out.is_file() {
            run(None, "ffmpeg", &["-v", "error", "-y", "-f", "lavfi", "-i", src, "-af", af, "-ac", "2", "-ar", "48000", &out.to_string_lossy()])?;
        }
        Ok(out)
    };
    let whoosh = make("whoosh.wav", "anoisesrc=d=0.45:c=pink:a=0.7:r=48000", "highpass=f=500,lowpass=f=6000,afade=t=in:d=0.28:curve=exp,afade=t=out:st=0.28:d=0.17")?;
    let boom = make("boom.wav", "aevalsrc='0.9*sin(2*PI*(70-45*t)*t)*exp(-3.5*t)+0.25*(random(0)-0.5)*exp(-30*t)':d=1.1:s=48000", "lowpass=f=900,volume=1.6")?;
    Ok((whoosh, boom))
}

/// One finished talking clip. The moment starts `seek` seconds into `video`.
#[allow(clippy::too_many_arguments)]
pub fn render(video: &Path, seek: f64, m: &Moment, words: &[Word], o: &Opts, t: &Talk, focus: f64, cache: &Path, work: &Path, out: &Path) -> Result<(), String> {
    let abs = |p: &Path| std::path::absolute(p).map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string());
    let segs = if t.jumpcuts { segments(words, m, 0.35) } else { vec![(m.start, m.end)] };
    let total: f64 = segs.iter().map(|x| x.1 - x.0).sum();
    let punch = remap(m.punch, &segs).filter(|p| *p > 0.3);
    let subs = captions(words, m, &segs, t.captions.as_ref(), t.hook.as_deref(), o.w, o.h);
    std::fs::write(work.join("cap.ass"), &subs).map_err(|e| e.to_string())?;
    let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into(), "-i".into(), abs(video)?];
    let mut g = String::new();
    let mut at = 0.0;
    let mut cuts = vec![];
    for (i, (a, b)) in segs.iter().enumerate() {
        let (fa, fb) = (a - m.start + seek, b - m.start + seek);
        let len = b - a;
        // every other cut punches in; the punchline punches in harder
        let base = if i % 2 == 1 { 1.14 } else { 1.0 };
        let z = match punch.filter(|p| *p >= at && *p < at + len) {
            Some(p) => format!("if(gte(it,{:.3}),{:.2},{base})", p - at, base + 0.14),
            None => format!("{base}"),
        };
        g.push_str(&format!(
            "[0:v]trim=start={fa:.3}:end={fb:.3},setpts=PTS-STARTPTS,fps=30,{},zoompan=z='{z}':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=1:s={}x{}:fps=30,setsar=1[v{i}];\
             [0:a]atrim=start={fa:.3}:end={fb:.3},asetpts=PTS-STARTPTS,afade=t=in:d=0.02,afade=t=out:st={:.3}:d=0.03[a{i}];",
            shape(o, focus), o.w, o.h, (len - 0.03).max(0.0)
        ));
        if i > 0 {
            cuts.push(at);
        }
        at += len;
    }
    let n = segs.len();
    (0..n).for_each(|i| g.push_str(&format!("[v{i}][a{i}]")));
    g.push_str(&format!("concat=n={n}:v=1:a=1[vc][ac];[vc]settb=1/30,fps=30{},eq=contrast=1.06,vignette=PI/6,ass=cap.ass,fade=t=in:d=0.12,fade=t=out:st={:.3}:d=0.25,format=yuv420p[vout];",
        grade(&o.grade), (total - 0.25).max(0.0)));
    // the sound: voice, then the cut whooshes and the punchline boom, then the music bed ducked under the voice
    let mut mix = vec!["[voice]".to_string()];
    g.push_str("[ac]asplit=2[voice][key];");
    let mut input = 1;
    if t.sfx {
        let (whoosh, boom) = sfx(cache)?;
        if !cuts.is_empty() {
            args.extend(["-i".into(), abs(&whoosh)?]);
            g.push_str(&format!("[{input}:a]asplit={}{};", cuts.len(), (0..cuts.len()).map(|k| format!("[w{k}]")).collect::<String>()));
            for (k, c) in cuts.iter().enumerate() {
                g.push_str(&format!("[w{k}]adelay=delays={}:all=1,volume=0.3[wd{k}];", ((c - 0.2).max(0.0) * 1000.0) as u64));
                mix.push(format!("[wd{k}]"));
            }
            input += 1;
        }
        if let Some(p) = punch {
            args.extend(["-i".into(), abs(&boom)?]);
            g.push_str(&format!("[{input}:a]adelay=delays={}:all=1,volume=0.6[bm];", (p * 1000.0) as u64));
            mix.push("[bm]".into());
            input += 1;
        }
    }
    if let Some((bed, from)) = &t.bed {
        args.extend(["-ss".into(), format!("{from:.3}"), "-t".into(), format!("{total:.3}"), "-i".into(), abs(bed)?]);
        g.push_str(&format!("[{input}:a]aresample=48000,volume=0.22[bd];[bd][key]sidechaincompress=threshold=0.02:ratio=10:attack=20:release=500[duck];"));
        mix.push("[duck]".into());
    } else {
        g.push_str("[key]anullsink;");
    }
    g.push_str(&format!("{}amix=inputs={}:normalize=0:duration=first,loudnorm=I={:.1}:TP=-1.5[aout]", mix.join(""), mix.len(), o.lufs));
    args.extend(["-filter_complex".into(), g, "-map".into(), "[vout]".into(), "-map".into(), "[aout]".into(),
        "-t".into(), format!("{total:.3}"), "-fps_mode".into(), "cfr".into(), "-r".into(), "30".into()]);
    args.extend(["-c:v", "libx264", "-preset", "medium", "-crf", "20", "-maxrate", "8M", "-bufsize", "16M", "-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-movflags", "+faststart"].iter().map(|s| s.to_string()));
    args.push(abs(out)?);
    let _ = std::fs::write(work.join("ffmpeg.txt"), args.iter().map(|a| format!("'{}'", a.replace('\'', "'\\''"))).collect::<Vec<_>>().join(" "));
    run(Some(work), "ffmpeg", &args).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: f64, e: f64, t: &str) -> Word {
        Word { s, e, t: t.into() }
    }
    fn m() -> Moment {
        Moment { start: 10.0, end: 20.0, punch: 15.0, score: 0.0, title: String::new(), why: String::new() }
    }

    #[test]
    fn pauses_are_cut() {
        let words = [w(10.5, 11.0, "bir"), w(11.1, 11.6, "iki."), w(13.0, 13.5, "üç"), w(13.6, 14.0, "dört"), w(15.0, 15.5, "beş")];
        let s = segments(&words, &m(), 0.35);
        assert_eq!(s.len(), 3, "{s:?}");
        assert!((s[0].0 - 10.42).abs() < 1e-9 && (s[0].1 - 11.68).abs() < 1e-9);
        assert!((s[2].1 - 15.9).abs() < 1e-9); // reaction kept after the last word
        assert!((remap(13.0, &s).unwrap() - 1.34).abs() < 1e-9);
        assert_eq!(remap(12.0, &s), None);
        assert_eq!(segments(&[], &m(), 0.35), vec![(10.0, 20.0)]);
    }

    #[test]
    fn captions_highlight_the_said_word() {
        let st = CaptionStyle { name: "t".into(), about: String::new(), font: "X".into(), size: 0.08, color: "#FFFFFF".into(), highlight: "#FFE600".into(),
            punch: "#E5332A".into(), outline: 6.0, outline_color: "#000000".into(), shadow: 0.0, boxed: false, upper: true, words: 3, position: 0.2, pop: true };
        let words = [w(10.0, 10.4, "bunu"), w(10.4, 10.8, "biliyor"), w(10.8, 11.2, "musun?"), w(15.0, 15.5, "hayır")];
        let segs = vec![(10.0, 11.5), (14.9, 16.0)];
        let a = captions(&words, &m(), &segs, Some(&st), Some("Bunu kimse bilmiyor"), 1080, 1920);
        assert!(a.contains("Style: C,X,86,&H00FFFFFF&"), "{a}");
        assert!(a.contains(&format!("{{\\c{}\\fscx125\\fscy125\\t(0,90,\\fscx108\\fscy108)}}BİLİYOR{{\\r}}", ass_color("#FFE600"))), "{a}");
        assert!(a.contains(&format!("{{\\c{}", ass_color("#E5332A"))) && a.contains("HAYIR"), "{a}"); // punchline colour
        assert!(a.contains(",H,,0,0,0,,{\\fad(120,200)}Bunu kimse bilmiyor"), "{a}");
        assert_eq!(ass_color("#E5332A"), "&H002A33E5&");
        // no two caption lines on screen at once
        let mut spans: Vec<(String, String)> = a.lines().filter(|l| l.contains(",C,,")).map(|l| {
            let f: Vec<&str> = l.split(',').collect();
            (f[1].to_string(), f[2].to_string())
        }).collect();
        spans.sort();
        assert!(spans.windows(2).all(|p| p[0].1 <= p[1].0), "{spans:?}");
    }

    #[test]
    fn shipped_caption_styles_read() {
        let s = styles();
        assert!(s.len() >= 5 && s.iter().any(|x| x.name == "hormozi"), "{}", s.len());
    }
}
