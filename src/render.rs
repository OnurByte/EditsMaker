use crate::pick::Moment;
use crate::transcribe::Word;
use crate::util::run;
use std::path::Path;

#[derive(Clone)]
pub struct Opts {
    pub w: u32,
    pub h: u32,
    pub grade: String,
    pub captions: bool,
    /// "fill" crops around the faces, "fit" keeps the whole frame over a blurred copy
    pub layout: String,
    /// loudness target in LUFS
    pub lufs: f64,
}

const RED_BGR: &str = "2A33E5"; // design/tokens.css --cut-red, as ASS wants it (BBGGRR)

pub fn ass_time(t: f64) -> String {
    let cs = (t.max(0.0) * 100.0).round() as u64;
    format!("{}:{:02}:{:02}.{:02}", cs / 360000, cs / 6000 % 60, cs / 100 % 60, cs % 100)
}

pub fn upper_tr(s: &str) -> String {
    s.replace('i', "İ").to_uppercase()
}

/// Caption chunks of at most 3 words / 22 characters: (start, end, text, is_punchline), times relative to the clip.
pub fn chunks(words: &[Word], m: &Moment) -> Vec<(f64, f64, String, bool)> {
    let mut out: Vec<(f64, f64, String, bool)> = vec![];
    let mut n = 0;
    let mut last_end = 0.0;
    for w in words.iter().filter(|w| w.s >= m.start - 0.05 && w.e <= m.end + 0.05) {
        let (s, e) = ((w.s - m.start).max(0.0), w.e - m.start);
        let text: String = w.t.chars().filter(|c| !matches!(c, '{' | '}' | '\\')).collect(); // ASS control characters
        let join = out.last().is_some_and(|c| n < 3 && c.2.chars().count() + text.chars().count() < 22 && s - last_end < 0.4 && !c.2.ends_with(['.', '?', '!']));
        if join {
            let c = out.last_mut().unwrap();
            c.1 = e;
            c.2.push(' ');
            c.2.push_str(&text);
            n += 1;
        } else {
            out.push((s, e, text, w.s >= m.punch - 0.05 && w.s < m.punch + 3.0)); // only the punchline itself is red
            n = 1;
        }
        last_end = e;
    }
    // a caption stays up until the next one, but never hangs over a long silence
    for i in 0..out.len() {
        let next = out.get(i + 1).map(|c| c.0).unwrap_or(f64::MAX);
        out[i].1 = next.min(out[i].1 + 0.6).max(out[i].0 + 0.15);
    }
    out
}

pub fn ass(words: &[Word], m: &Moment, o: &Opts) -> String {
    let size = o.w * 7 / 100;
    let margin = if o.h > o.w { o.h * 28 / 100 } else { o.h * 10 / 100 }; // above the app UI on vertical video
    let mut s = format!(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: {}\nPlayResY: {}\nWrapStyle: 0\nScaledBorderAndShadow: yes\n\n\
         [V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
         Style: C,Helvetica,{size},&H00FFFFFF,&H00FFFFFF,&H00000000,&H80000000,-1,0,0,0,100,100,0,0,1,5,2,2,60,60,{margin},1\n\n\
         [Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        o.w, o.h
    );
    for (a, b, text, punch) in chunks(words, m) {
        let (fx, text) = if punch {
            (format!("{{\\c&H{RED_BGR}&\\fscx95\\fscy95\\t(0,90,\\fscx118\\fscy118)}}"), upper_tr(&text))
        } else {
            ("{\\fscx88\\fscy88\\t(0,80,\\fscx100\\fscy100)}".to_string(), text)
        };
        s.push_str(&format!("Dialogue: 0,{},{},C,,0,0,0,,{fx}{text}\n", ass_time(a), ass_time(b)));
    }
    s
}

/// Any frame to `o.w` x `o.h`: a crop centred on `focus` (0..1 across), or with layout "fit" the whole frame
/// over a blurred copy of itself.
pub fn shape(o: &Opts, focus: f64) -> String {
    let (w, h) = (o.w, o.h);
    if o.layout == "fit" {
        format!("split[a][b];[a]scale={w}:{h}:force_original_aspect_ratio=increase,crop={w}:{h},boxblur=24:2,eq=brightness=-0.12[bg];\
                 [b]scale={w}:{h}:force_original_aspect_ratio=decrease[fg];[bg][fg]overlay=(W-w)/2:(H-h)/2,setsar=1")
    } else {
        format!("crop=w='min(iw,ih*{w}/{h})':h='min(ih,iw*{h}/{w})':x='max(0,min(iw-ow,iw*{focus:.3}-ow/2))',scale={w}:{h},setsar=1")
    }
}

/// A colour grade by name (see `looks`), or the LUT it names when it starts with "lut3d".
pub fn grade(name: &str) -> String {
    if name.starts_with(",lut3d") { name.to_string() } else { crate::looks::chain(name).to_string() }
}

/// The video filter chain: reshape to the target, punch in on the punchline, grade, captions, fades.
/// `focus` is the horizontal centre of the faces (0..1), 0.5 when unknown.
pub fn filters(m: &Moment, o: &Opts, ass_file: Option<&str>, focus: f64) -> String {
    let (w, h, dur, punch) = (o.w, o.h, m.end - m.start, m.punch - m.start);
    let mut f = format!("fps=30,{}", shape(o, focus));
    f.push_str(&format!(",zoompan=z='if(gte(in_time,{punch:.2}),1.12,1)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=1:s={w}x{h}:fps=30"));
    f.push_str(&grade(&o.grade));
    if let Some(a) = ass_file {
        f.push_str(&format!(",ass={a}"));
    }
    f.push_str(&format!(",fade=t=in:d=0.12,fade=t=out:st={:.2}:d=0.25,format=yuv420p", (dur - 0.25).max(0.0)));
    f
}

/// Cuts one moment into a finished clip. The moment starts `seek` seconds into `video`; `work` holds the subtitle file.
#[allow(clippy::too_many_arguments)]
pub fn render(video: &Path, seek: f64, m: &Moment, words: &[Word], o: &Opts, focus: f64, work: &Path, out: &Path) -> Result<(), String> {
    let dur = m.end - m.start;
    let ass_file = if o.captions && words.iter().any(|w| w.s >= m.start && w.e <= m.end) {
        std::fs::write(work.join("cap.ass"), ass(words, m, o)).map_err(|e| e.to_string())?;
        Some("cap.ass") // relative on purpose: ffmpeg runs in `work`, so the path needs no filter escaping
    } else {
        None
    };
    let abs = |p: &Path| std::path::absolute(p).map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string());
    let af = format!("afade=t=in:d=0.05,afade=t=out:st={:.2}:d=0.25,loudnorm=I={:.1}:TP=-1.5:LRA=11", (dur - 0.25).max(0.0), o.lufs);
    run(Some(work), "ffmpeg", &[
        "-v", "error", "-y", "-ss", &format!("{seek:.3}"), "-t", &format!("{dur:.3}"), "-i", &abs(video)?,
        "-map", "0:v:0", "-map", "0:a:0?", "-vf", &filters(m, o, ass_file, focus), "-af", &af,
        "-c:v", "libx264", "-preset", "medium", "-crf", "21", "-maxrate", "6M", "-bufsize", "12M",
        "-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-movflags", "+faststart", &abs(out)?,
    ]).map(|_| ())
}

/// Joins finished clips (same size, fps and codecs) without re-encoding.
pub fn join(clips: &[std::path::PathBuf], work: &Path, out: &Path) -> Result<(), String> {
    let mut list = String::new();
    for c in clips {
        let p = std::path::absolute(c).map_err(|e| e.to_string())?;
        list.push_str(&format!("file '{}'\n", p.to_string_lossy().replace('\'', "'\\''")));
    }
    std::fs::write(work.join("join.txt"), list).map_err(|e| e.to_string())?;
    let o = std::path::absolute(out).map_err(|e| e.to_string())?;
    run(Some(work), "ffmpeg", &["-v", "error", "-y", "-f", "concat", "-safe", "0", "-i", "join.txt", "-c", "copy", "-movflags", "+faststart", &o.to_string_lossy()]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: f64, e: f64, t: &str) -> Word {
        Word { s, e, t: t.into() }
    }
    fn o() -> Opts {
        Opts { w: 1080, h: 1920, grade: "bw".into(), captions: true, layout: "fill".into(), lufs: -14.0 }
    }
    fn m() -> Moment {
        Moment { start: 10.0, end: 20.0, punch: 14.0, score: 0.0, title: String::new(), why: String::new() }
    }

    #[test]
    fn ass_times() {
        assert_eq!(ass_time(0.0), "0:00:00.00");
        assert_eq!(ass_time(75.256), "0:01:15.26");
        assert_eq!(ass_time(3661.5), "1:01:01.50");
    }

    #[test]
    fn chunks_group_break_and_mark_the_punchline() {
        let words = [
            w(9.0, 9.5, "önce"), // before the clip
            w(10.0, 10.3, "bir"), w(10.3, 10.7, "kapanın"), w(10.7, 11.0, "bakalım"), w(11.0, 11.3, "siz"),
            w(14.0, 14.4, "siktir{\\b1}"), w(14.4, 14.8, "git."), w(14.8, 15.0, "he"), w(18.0, 18.3, "neyse"),
        ];
        let c = chunks(&words, &m());
        assert_eq!(c.iter().map(|x| x.2.as_str()).collect::<Vec<_>>(), vec!["bir kapanın bakalım", "siz", "siktirb1 git.", "he", "neyse"]);
        assert_eq!(c.iter().map(|x| x.3).collect::<Vec<_>>(), vec![false, false, true, true, false]);
        assert_eq!((c[0].0, c[0].1), (0.0, 1.0)); // held until the next chunk starts
        assert!((c[1].1 - 1.9).abs() < 1e-9); // not held across the 2.7 s silence
    }

    #[test]
    fn punchline_is_red_and_turkish_uppercase() {
        let o = o();
        let a = ass(&[w(14.0, 14.5, "siktir"), w(14.5, 15.0, "git")], &m(), &o);
        assert!(a.contains("PlayResX: 1080") && a.contains("Style: C,Helvetica,75,"), "{a}");
        assert!(a.contains("Dialogue: 0,0:00:04.00,0:00:05.60,C,,0,0,0,,{\\c&H2A33E5&") && a.ends_with("SİKTİR GİT\n"), "{a}");
    }

    #[test]
    fn filter_chain() {
        let f = filters(&m(), &o(), Some("cap.ass"), 0.7);
        assert!(f.starts_with("fps=30,crop=w='min(iw,ih*1080/1920)':h='min(ih,iw*1920/1080)':x='max(0,min(iw-ow,iw*0.700-ow/2))',scale=1080:1920"), "{f}");
        assert!(f.contains("gte(in_time,4.00)") && f.contains(",hue=s=0,") && f.contains(",ass=cap.ass,fade=t=in") && f.contains("st=9.75"), "{f}");
        let fit = filters(&m(), &Opts { grade: "none".into(), layout: "fit".into(), ..o() }, None, 0.5);
        assert!(!fit.contains("ass=") && fit.contains("boxblur") && fit.contains("[bg][fg]overlay"), "{fit}");
    }
}
