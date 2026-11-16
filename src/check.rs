//! `edits check clip.mp4`: what a render turned out as, measured: length, loudness, cut rhythm, and a contact sheet to look at.
use std::path::{Path, PathBuf};
use std::process::Command;

/// ffmpeg writes its measurements to stderr.
fn log(args: &[&str]) -> Result<String, String> {
    let out = Command::new("ffmpeg").args(["-hide_banner", "-nostats", "-v", "info", "-y"]).args(args).output().map_err(|e| format!("ffmpeg: {e}"))?;
    Ok(String::from_utf8_lossy(&out.stderr).into_owned())
}

/// Integrated loudness (LUFS) from the ebur128 summary at the end of the log.
fn lufs(log: &str) -> Option<f64> {
    let summary = log.rsplit("Summary:").next()?;
    summary.lines().find_map(|l| l.trim().strip_prefix("I:")).and_then(|v| v.split_whitespace().next()?.parse().ok())
}

/// Times of the cuts that `select=gt(scene,..),showinfo` printed.
fn cut_times(log: &str) -> Vec<f64> {
    log.lines().filter_map(|l| l.split("pts_time:").nth(1)?.split_whitespace().next()?.parse().ok()).collect()
}

/// Lengths of the shots between cuts, first to last.
fn shots(cuts: &[f64], total: f64) -> Vec<f64> {
    let mut at = vec![0.0];
    at.extend(cuts.iter().copied().filter(|&c| c > 0.0 && c < total));
    at.push(total);
    at.windows(2).map(|w| w[1] - w[0]).collect()
}

/// `cols`x`rows` frames spread evenly over `total` seconds, tiled into one jpg.
/// Frames more than 5 s apart come from keyframes alone: a 38-minute video's sheet takes 1 s instead of 54, and the
/// frames are still all different (keyframes come every few seconds).
pub fn contact_sheet(video: &Path, total: f64, sheet: &Path, cols: u32, rows: u32, width: u32) -> Result<(), String> {
    let fps = (cols * rows) as f64 / total.max(1.0);
    let skip: &[&str] = if fps < 0.2 { &["-skip_frame", "nokey"] } else { &[] };
    let (v, s) = (video.to_string_lossy(), sheet.to_string_lossy());
    let filter = format!("fps={fps:.4},scale={width}:-2,tile={cols}x{rows}");
    log(&[skip, &["-i", &v, "-vf", &filter, "-frames:v", "1", &s]].concat())?;
    if sheet.exists() { Ok(()) } else { Err(format!("no contact sheet for {}", video.display())) }
}

pub fn check(clip: &Path) -> Result<String, String> {
    let c = clip.to_string_lossy();
    let probe = crate::util::run(None, "ffprobe", &["-v", "error", "-show_entries", "format=duration:stream=width,height", "-of", "csv=p=0", &c])?;
    let total: f64 = probe.lines().filter_map(|l| l.trim().parse().ok()).next().ok_or("could not read the length")?;
    let size = probe.lines().find(|l| l.contains(',')).unwrap_or("?").replace(',', "x");
    let loud = lufs(&log(&["-i", &c, "-af", "ebur128", "-f", "null", "-"])?);
    let cuts = cut_times(&log(&["-i", &c, "-an", "-vf", "select='gt(scene,0.3)',showinfo", "-f", "null", "-"])?);
    let sh = shots(&cuts, total);
    let sheet: PathBuf = clip.with_file_name(format!("{}-sheet.jpg", clip.file_stem().unwrap_or_default().to_string_lossy()));
    contact_sheet(clip, total, &sheet, 10, 2, 180)?;
    let longest = sh.iter().copied().fold(0.0, f64::max);
    Ok(format!(
        "{}: {total:.1}s {size}\nloudness: {} LUFS\ncuts: {} (shots average {:.1}s, longest {longest:.1}s)\nsheet: {}",
        clip.display(),
        loud.map_or("?".into(), |l| format!("{l:.1}")),
        cuts.len(),
        total / sh.len().max(1) as f64,
        sheet.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ffmpeg_logs() {
        let l = "[Parsed_ebur128_0 @ 0x1] t: 1 M: -20 I: -30.0 LUFS\n[Parsed_ebur128_0 @ 0x1] Summary:\n\n  Integrated loudness:\n    I:         -16.0 LUFS\n    Threshold: -26.0 LUFS\n";
        assert_eq!(lufs(l), Some(-16.0));
        assert_eq!(lufs("nothing"), None);
        let c = "[Parsed_showinfo_1 @ 0x1] n: 0 pts: 1 pts_time:1.5 pos: 1\n[Parsed_showinfo_1 @ 0x1] n: 1 pts: 2 pts_time:4 pos: 2\n";
        assert_eq!(cut_times(c), [1.5, 4.0]);
        assert_eq!(shots(&[1.5, 4.0], 10.0), [1.5, 2.5, 6.0]);
        assert_eq!(shots(&[], 10.0), [10.0]);
    }
}
