//! Learns the look of edits you like: `edits study <folder>` measures them and saves the defaults.
use crate::face::median;
use crate::fetch::{dims, duration};
use crate::util::{home, run};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Style {
    pub aspect: String,
    pub grade: String,
    pub lufs: f64,
}

pub struct Stat {
    pub w: u32,
    pub h: u32,
    pub dur: f64,
    pub cuts: usize,
    pub sat: f64,
    pub lufs: f64,
}

pub const ASPECTS: &[(&str, u32, u32)] = &[("9:16", 1080, 1920), ("1:1", 1080, 1080), ("4:3", 1440, 1080), ("16:9", 1920, 1080)];

pub fn path() -> PathBuf {
    home().join(".config/edits/style.json")
}

pub fn load() -> Option<Style> {
    serde_json::from_str(&std::fs::read_to_string(path()).ok()?).ok()
}

fn nearest_aspect(w: u32, h: u32) -> &'static str {
    let r = (w.max(1) as f64 / h.max(1) as f64).ln();
    ASPECTS.iter().min_by(|a, b| (r - (a.1 as f64 / a.2 as f64).ln()).abs().total_cmp(&(r - (b.1 as f64 / b.2 as f64).ln()).abs())).unwrap().0
}

/// Most common shape, typical saturation -> grade, typical loudness.
pub fn summarize(stats: &[Stat]) -> Option<Style> {
    let aspect = ASPECTS.iter().map(|a| a.0).max_by_key(|a| stats.iter().filter(|s| nearest_aspect(s.w, s.h) == *a).count())?;
    let sat = median(&mut stats.iter().map(|s| s.sat).collect::<Vec<_>>())?;
    let lufs = median(&mut stats.iter().map(|s| s.lufs).filter(|l| l.is_finite() && *l > -40.0).collect::<Vec<_>>()).unwrap_or(-14.0);
    // signalstats SATAVG: under ~4 is black and white, the desaturated teal look measures 5-20, a normal camera picture 60 and up
    let grade = if sat < 4.5 { "bw" } else if sat < 45.0 { "dark" } else { "none" };
    Some(Style { aspect: aspect.into(), grade: grade.into(), lufs: (lufs * 10.0).round() / 10.0 })
}

fn values(out: &str, key: &str) -> Vec<f64> {
    out.lines().filter_map(|l| l.strip_prefix(key)).filter_map(|v| v.trim().parse().ok()).collect()
}

fn measure(f: &Path) -> Result<Stat, String> {
    let p = f.to_string_lossy();
    let (w, h) = dims(f);
    let sat = run(None, "ffmpeg", &["-v", "error", "-i", p.as_ref(), "-an", "-vf", "fps=2,scale=160:-2,signalstats,metadata=print:key=lavfi.signalstats.SATAVG:file=-", "-f", "null", "-"])?;
    let cuts = run(None, "ffmpeg", &["-v", "error", "-i", p.as_ref(), "-an", "-vf", "select='gt(scene,0.3)',metadata=print:key=lavfi.scene_score:file=-", "-f", "null", "-"])?;
    let loud = run(None, "ffmpeg", &["-v", "error", "-i", p.as_ref(), "-vn", "-af", "ebur128=metadata=1,ametadata=print:key=lavfi.r128.I:file=-", "-f", "null", "-"]).unwrap_or_default();
    let sats = values(&sat, "lavfi.signalstats.SATAVG=");
    Ok(Stat {
        w, h, dur: duration(f)?,
        cuts: values(&cuts, "lavfi.scene_score=").len(),
        sat: if sats.is_empty() { 0.0 } else { sats.iter().sum::<f64>() / sats.len() as f64 },
        lufs: values(&loud, "lavfi.r128.I=").last().copied().unwrap_or(f64::NAN),
    })
}

pub fn study(dir: &Path) -> Result<(), String> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?
        .flatten().map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| ["mp4", "mov", "mkv", "webm"].contains(&e.to_string_lossy().to_lowercase().as_str())))
        .collect();
    files.sort();
    let mut stats = vec![];
    for f in &files {
        match measure(f) {
            Ok(s) => {
                println!("{:<28} {:>4}x{:<4} {:>5.1}s  shot {:>4.1}s  sat {:>5.1}  {:>6.1} LUFS  -> {}",
                    f.file_name().unwrap_or_default().to_string_lossy(), s.w, s.h, s.dur, s.dur / (s.cuts + 1) as f64, s.sat, s.lufs, nearest_aspect(s.w, s.h));
                stats.push(s);
            }
            Err(e) => eprintln!("edits: skipped {}: {e}", f.display()),
        }
    }
    let style = summarize(&stats).ok_or("no videos to study in that folder")?;
    if let Some(d) = path().parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(path(), serde_json::to_string_pretty(&style).unwrap()).map_err(|e| e.to_string())?;
    println!("\nnew defaults: --aspect {} --grade {} at {} LUFS (saved to {})", style.aspect, style.grade, style.lufs, path().display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shapes() {
        assert_eq!(nearest_aspect(1080, 1920), "9:16");
        assert_eq!(nearest_aspect(586, 576), "1:1");
        assert_eq!(nearest_aspect(768, 576), "4:3");
        assert_eq!(nearest_aspect(2880, 2160), "4:3");
        assert_eq!(nearest_aspect(1920, 1080), "16:9");
    }

    #[test]
    fn summary() {
        let s = |w, h, sat, lufs| Stat { w, h, dur: 10.0, cuts: 3, sat, lufs };
        let st = summarize(&[s(720, 720, 30.0, -9.0), s(576, 576, 6.0, -12.0), s(1080, 1920, 35.0, f64::NAN), s(960, 720, 70.0, -11.0)]).unwrap();
        assert_eq!(st, Style { aspect: "1:1".into(), grade: "dark".into(), lufs: -11.0 });
        assert!(summarize(&[]).is_none());
    }
}
