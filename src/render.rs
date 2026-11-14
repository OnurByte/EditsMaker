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


}
