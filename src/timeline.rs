//! An edit as a timeline other editors open: FCPXML (Final Cut Pro, DaVinci Resolve), CMX 3600 EDL (Premiere Pro,
//! Resolve, Avid) and SRT captions, written beside the rendered clip. The cuts, sources, in points and music carry
//! over; the moves and effects stay baked into the rendered clip.
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

pub const FPS: f64 = 30.0;

#[derive(Clone, Debug, Default)]
pub struct Timeline {
    pub name: String,
    pub w: u32,
    pub h: u32,
    /// (source file, in point in it, length) in timeline order, end to end
    pub clips: Vec<(PathBuf, f64, f64)>,
    /// (song file, in point in it)
    pub music: Option<(PathBuf, f64)>,
    /// (start, end, text) on the timeline
    pub captions: Vec<(f64, f64, String)>,
    /// the spoken line alone, (start, end, text): drawn on the picture with --say-text
    pub said: Vec<(f64, f64, String)>,
}

fn frames(t: f64) -> u64 {
    (t.max(0.0) * FPS).round() as u64
}

/// Seconds as an FCPXML rational time on the 30 fps grid.
fn rt(t: f64) -> String {
    format!("{}/30s", frames(t))
}

/// Seconds as SMPTE timecode at 30 fps (non-drop).
pub fn tc(t: f64) -> String {
    let f = frames(t);
    format!("{:02}:{:02}:{:02}:{:02}", f / 108000, f / 1800 % 60, f / 30 % 60, f % 30)
}

fn url(p: &Path) -> String {
    let abs = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let s = abs.to_string_lossy().replace('\\', "/");
    let enc: String = s.chars().map(|c| match c {
        ' ' => "%20".to_string(),
        '&' => "%26".to_string(),
        '#' => "%23".to_string(),
        '?' => "%3F".to_string(),
        c => c.to_string(),
    }).collect();
    if enc.starts_with('/') { format!("file://{enc}") } else { format!("file:///{enc}") }
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

impl Timeline {
    pub fn total(&self) -> f64 {
        self.clips.iter().map(|c| c.2).sum()
    }

    pub fn fcpxml(&self) -> String {
        let total = self.total();
        let mut assets: Vec<PathBuf> = vec![];
        for (f, _, _) in &self.clips {
            if !assets.contains(f) {
                assets.push(f.clone());
            }
        }
        let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE fcpxml>\n<fcpxml version=\"1.9\">\n  <resources>\n");
        let _ = writeln!(s, "    <format id=\"r0\" frameDuration=\"1/30s\" width=\"{}\" height=\"{}\"/>", self.w, self.h);
        let src_len = |f: &Path| crate::fetch::duration(f).unwrap_or(3600.0);
        for (i, f) in assets.iter().enumerate() {
            let _ = writeln!(s, "    <asset id=\"a{i}\" name=\"{}\" src=\"{}\" start=\"0s\" duration=\"{}\" hasVideo=\"1\" hasAudio=\"1\" format=\"r0\"/>",
                esc(&f.file_name().unwrap_or_default().to_string_lossy()), esc(&url(f)), rt(src_len(f)));
        }
        if let Some((m, _)) = &self.music {
            let _ = writeln!(s, "    <asset id=\"music\" name=\"{}\" src=\"{}\" start=\"0s\" duration=\"{}\" hasAudio=\"1\"/>",
                esc(&m.file_name().unwrap_or_default().to_string_lossy()), esc(&url(m)), rt(src_len(m)));
        }
        let _ = write!(s, "  </resources>\n  <library>\n    <event name=\"EditsMaker\">\n      <project name=\"{}\">\n        <sequence format=\"r0\" duration=\"{}\" tcStart=\"0s\" tcFormat=\"NDF\">\n          <spine>\n",
            esc(&self.name), rt(total));
        let mut at = 0.0;
        for (k, (f, inp, len)) in self.clips.iter().enumerate() {
            let a = assets.iter().position(|x| x == f).unwrap_or(0);
            let _ = write!(s, "            <asset-clip ref=\"a{a}\" name=\"shot {}\" offset=\"{}\" start=\"{}\" duration=\"{}\" format=\"r0\"",
                k + 1, rt(at), rt(*inp), rt(*len));
            // the song rides on the first clip, as a connected clip under the whole edit
            match (&self.music, k) {
                (Some((_, from)), 0) => {
                    let _ = write!(s, ">\n              <asset-clip ref=\"music\" lane=\"-1\" name=\"music\" offset=\"{}\" start=\"{}\" duration=\"{}\"/>\n            </asset-clip>\n",
                        rt(*inp), rt(*from), rt(total));
                }
                _ => s.push_str("/>\n"),
            }
            at += len;
        }
        s.push_str("          </spine>\n        </sequence>\n      </project>\n    </event>\n  </library>\n</fcpxml>\n");
        s
    }

    /// CMX 3600: one event per clip, each source named by its file so editors can relink it.
    pub fn edl(&self) -> String {
        let mut s = format!("TITLE: {}\nFCM: NON-DROP FRAME\n\n", self.name.chars().take(60).collect::<String>());
        let mut at = 0.0;
        for (k, (f, inp, len)) in self.clips.iter().enumerate() {
            let reel = format!("AX{:03}", k + 1);
            let _ = writeln!(s, "{:03}  {reel:<8} AA/V  C        {} {} {} {}", k + 1, tc(*inp), tc(inp + len), tc(at), tc(at + len));
            let _ = writeln!(s, "* FROM CLIP NAME: {}\n* SOURCE FILE: {}\n", f.file_name().unwrap_or_default().to_string_lossy(), f.to_string_lossy());
            at += len;
        }
        if let Some((m, from)) = &self.music {
            let n = self.clips.len() + 1;
            let _ = writeln!(s, "{n:03}  AX{n:03}    A     C        {} {} {} {}", tc(*from), tc(from + at), tc(0.0), tc(at));
            let _ = writeln!(s, "* FROM CLIP NAME: {}\n* SOURCE FILE: {}\n", m.file_name().unwrap_or_default().to_string_lossy(), m.to_string_lossy());
        }
        s
    }

    pub fn srt(&self) -> String {
        self.cues(',').iter().enumerate().map(|(i, c)| format!("{}\n{c}", i + 1)).collect()
    }

    /// WebVTT: the same cues with dots in the times and a header.
    pub fn vtt(&self) -> String {
        format!("WEBVTT\n\n{}", self.cues('.').concat())
    }

    fn cues(&self, dot: char) -> Vec<String> {
        let t = |x: f64| {
            let ms = (x.max(0.0) * 1000.0).round() as u64;
            format!("{:02}:{:02}:{:02}{dot}{:03}", ms / 3_600_000, ms / 60_000 % 60, ms / 1000 % 60, ms % 1000)
        };
        self.captions.iter().map(|(a, b, text)| format!("{} --> {}\n{}\n\n", t(*a), t(*b), text)).collect()
    }

    /// Writes the asked-for kinds ("fcpxml", "edl", "srt", "vtt") beside `out` (the rendered clip).
    pub fn write(&self, out: &Path, kinds: &[String]) -> Result<Vec<PathBuf>, String> {
        let mut written = vec![];
        for k in kinds {
            let (ext, text) = match k.as_str() {
                "fcpxml" => ("fcpxml", self.fcpxml()),
                "edl" => ("edl", self.edl()),
                "srt" if !self.captions.is_empty() => ("srt", self.srt()),
                "srt" => continue,
                "vtt" if !self.captions.is_empty() => ("vtt", self.vtt()),
                "vtt" => continue,
                other => return Err(format!("--export: {other} is not fcpxml, edl, srt or vtt")),
            };
            let p = out.with_extension(ext);
            std::fs::write(&p, text).map_err(|e| format!("{}: {e}", p.display()))?;
            written.push(p);
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tl() -> Timeline {
        Timeline {
            name: "test & co".into(), w: 1080, h: 1920,
            clips: vec![("/v/a b.mp4".into(), 10.0, 1.5), ("/v/c.mp4".into(), 3.0, 2.0)],
            music: Some(("/m/song.wav".into(), 30.0)),
            captions: vec![(0.0, 1.2, "Merhaba dünya".into())],
            said: vec![],
        }
    }

    #[test]
    fn timecodes() {
        assert_eq!(tc(0.0), "00:00:00:00");
        assert_eq!(tc(61.5), "00:01:01:15");
        assert_eq!(tc(3600.0), "01:00:00:00");
        assert_eq!(rt(1.5), "45/30s");
    }

    #[test]
    fn fcpxml_has_the_cuts_and_the_song() {
        let x = tl().fcpxml();
        assert!(x.contains("<format id=\"r0\" frameDuration=\"1/30s\" width=\"1080\" height=\"1920\"/>"), "{x}");
        // Windows makes it file:///D:/v/..., so only check the encoded tail
        assert!(x.contains("/v/a%20b.mp4\"") && x.contains("src=\"file:///"), "{x}");
        assert!(x.contains("<asset-clip ref=\"a0\" name=\"shot 1\" offset=\"0/30s\" start=\"300/30s\" duration=\"45/30s\" format=\"r0\">"), "{x}");
        assert!(x.contains("<asset-clip ref=\"music\" lane=\"-1\" name=\"music\" offset=\"300/30s\" start=\"900/30s\" duration=\"105/30s\"/>"), "{x}");
        assert!(x.contains("<asset-clip ref=\"a1\" name=\"shot 2\" offset=\"45/30s\" start=\"90/30s\" duration=\"60/30s\" format=\"r0\"/>"), "{x}");
        assert!(x.contains("project name=\"test &amp; co\""));
        // it parses as XML
        roxmltree::Document::parse(&x.replace("<!DOCTYPE fcpxml>\n", "")).unwrap();
    }

    #[test]
    fn edl_and_srt() {
        let e = tl().edl();
        assert!(e.contains("001  AX001    AA/V  C        00:00:10:00 00:00:11:15 00:00:00:00 00:00:01:15"), "{e}");
        assert!(e.contains("* FROM CLIP NAME: a b.mp4") && e.contains("003  AX003    A     C        00:00:30:00 00:00:33:15 00:00:00:00 00:00:03:15"), "{e}");
        assert_eq!(tl().srt(), "1\n00:00:00,000 --> 00:00:01,200\nMerhaba dünya\n\n");
        assert_eq!(tl().vtt(), "WEBVTT\n\n00:00:00.000 --> 00:00:01.200\nMerhaba dünya\n\n");
        assert!(tl().write(Path::new("/tmp/x.mp4"), &["mov".into()]).is_err());
    }
}
