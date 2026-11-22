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
    let mut out: Vec<f64> = vec![];
    let mut seen = f64::NEG_INFINITY;
    for t in log.lines().filter_map(|l| l.split("pts_time:").nth(1)?.split_whitespace().next()?.parse::<f64>().ok()) {
        // a flash is a jump into white and back out, a few frames apart: one cut, not two or three
        if t - seen > 0.12 {
            out.push(t);
        }
        seen = t;
    }
    out
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
    contact_sheet_at(video, 0.0, total, sheet, cols, rows, width)
}

/// The same over `total` seconds from `from` on.
pub fn contact_sheet_at(video: &Path, from: f64, total: f64, sheet: &Path, cols: u32, rows: u32, width: u32) -> Result<(), String> {
    let fps = (cols * rows) as f64 / total.max(1.0);
    let skip: &[&str] = if fps < 0.2 { &["-skip_frame", "nokey"] } else { &[] };
    let (v, s, ss, t) = (video.to_string_lossy(), sheet.to_string_lossy(), format!("{from:.3}"), format!("{total:.3}"));
    let filter = format!("fps={fps:.4},scale={width}:-2,tile={cols}x{rows}");
    log(&[skip, &["-ss", &ss, "-t", &t, "-i", &v, "-vf", &filter, "-frames:v", "1", "-y", &s]].concat())?;
    if sheet.exists() { Ok(()) } else { Err(format!("no contact sheet for {}", video.display())) }
}

/// "336", "336.5" or "5:36" as seconds.
pub fn secs(s: &str) -> Option<f64> {
    s.split(':').try_fold(0.0, |acc, p| p.trim().parse::<f64>().ok().map(|v| acc * 60.0 + v))
}

/// The footage behind a path or a video ID edits already fetched (~/.cache/edits/ID: hd.mp4, src.mp4 or v.mp4).
pub fn footage(v: &str) -> Result<PathBuf, String> {
    let p = crate::fetch::reference(v)?;
    if p.is_file() {
        return Ok(p);
    }
    let dir = crate::util::home().join(".cache/edits").join(v);
    ["hd.mp4", "src.mp4", "v.mp4"].iter().map(|f| dir.join(f)).find(|f| f.is_file()).ok_or(format!("no video at {v} and none fetched under that ID"))
}

/// `edits frames VIDEO|ID AT[+LEN]...`: each moment as a row of 6 frames over LEN seconds (default 3), stacked in
/// one jpg, to look at before picking `--shot ID@AT`. No text on the picture: the rows are listed in order.
pub fn frames(video: &str, at: &[String]) -> Result<String, String> {
    let v = footage(video)?;
    let rows: Vec<(f64, f64)> = at.iter().map(|a| {
        let (s, l) = a.split_once('+').unwrap_or((a, "3"));
        Some((secs(s)?, secs(l)?.max(0.2)))
    }).collect::<Option<_>>().filter(|r: &Vec<_>| !r.is_empty()).ok_or("times look like 336, 5:36 or 336+4 (from 336 s, 4 s long)")?;
    let out = crate::util::home().join(".cache/edits/frames").join(format!("{}-{}.jpg", v.parent().and_then(|d| d.file_name()).unwrap_or_default().to_string_lossy(), at.join("_").replace([':', '+', '.'], "-")));
    std::fs::create_dir_all(out.parent().unwrap()).map_err(|e| e.to_string())?;
    let vs = v.to_string_lossy();
    let mut args: Vec<String> = vec![];
    let mut graph = String::new();
    for (i, (s, l)) in rows.iter().enumerate() {
        args.extend(["-ss".into(), format!("{s}"), "-t".into(), format!("{l}"), "-i".into(), vs.to_string()]);
        graph += &format!("[{i}:v]fps={:.4},scale=200:-2,tile=6x1[r{i}];", 6.0 / l);
    }
    graph += &if rows.len() == 1 { "[r0]null".into() } else { format!("{}vstack=inputs={}", (0..rows.len()).map(|i| format!("[r{i}]")).collect::<String>(), rows.len()) };
    let o = out.to_string_lossy().to_string();
    args.extend(["-filter_complex".into(), graph, "-frames:v".into(), "1".into(), "-update".into(), "1".into(), o.clone()]);
    log(&args.iter().map(String::as_str).collect::<Vec<_>>())?;
    if !out.exists() {
        return Err(format!("no frames from {}", v.display()));
    }
    let list: Vec<String> = rows.iter().enumerate().map(|(i, (s, l))| format!("row {}: {s:.1}-{:.1}s", i + 1, s + l)).collect();
    Ok(format!("{o}\n{}", list.join("\n")))
}

pub fn check(clip: &Path) -> Result<String, String> {
    let c = clip.to_string_lossy();
    let probe = crate::util::run(None, "ffprobe", &["-v", "error", "-show_entries", "format=duration:stream=width,height", "-of", "csv=p=0", &c])?;
    let total: f64 = probe.lines().filter_map(|l| l.trim().parse().ok()).next().ok_or("could not read the length")?;
    let size = probe.lines().find(|l| l.contains(',')).unwrap_or("?").replace(',', "x");
    let loud = lufs(&log(&["-i", &c, "-af", "ebur128", "-f", "null", "-"])?);
    let cuts = cuts(clip);
    let sh = shots(&cuts, total);
    let sheet: PathBuf = clip.with_file_name(format!("{}-sheet.jpg", clip.file_stem().unwrap_or_default().to_string_lossy()));
    contact_sheet(clip, total, &sheet, 10, 2, 180)?;
    let longest = sh.iter().copied().fold(0.0, f64::max);
    let round = seam_apart(clip).map_or(String::new(), |d| format!("\nend to start: frames {:.0}% apart ({})", d * 100.0, if d < LOOPS { "plays round seamlessly" } else { "a jump when it plays again" }));
    Ok(format!(
        "{}: {total:.1}s {size}\nloudness: {} LUFS\ncuts: {} (shots average {:.1}s, longest {longest:.1}s){round}\nsheet: {}",
        clip.display(),
        loud.map_or("?".into(), |l| format!("{l:.1}")),
        cuts.len(),
        total / sh.len().max(1) as f64,
        sheet.display()
    ))
}

/// Where a clip cuts, seconds (the picture jumps).
pub fn cuts(clip: &Path) -> Vec<f64> {
    let c = clip.to_string_lossy();
    log(&["-i", &c, "-an", "-vf", "select='gt(scene,0.3)',showinfo", "-f", "null", "-"]).map(|l| cut_times(&l)).unwrap_or_default()
}

/// Below this, the last frame runs into the first without a visible jump.
const LOOPS: f64 = 0.12;

/// How far apart a clip's last and first frames are, 0 (the same) to 1: small, grey, compared pixel by pixel. None
/// when it starts from black (a fade in: not made to loop, and black next to a faded-out black end says nothing).
fn seam_apart(clip: &Path) -> Option<f64> {
    let c = clip.to_string_lossy();
    let frame = |at: &[&str]| -> Option<Vec<u8>> {
        let mut args: Vec<&str> = vec!["-v", "error"];
        args.extend_from_slice(at);
        args.extend_from_slice(&["-i", &c, "-frames:v", "1", "-vf", "scale=36:64,format=gray", "-f", "rawvideo", "-"]);
        crate::util::run_bytes("ffmpeg", &args).ok().filter(|b| b.len() >= 36 * 64)
    };
    // every frame of the last second, the last of them taken: a point just before the end can fall past the
    // picture when the sound runs a little longer
    let tail = frame_all(&c, &["-sseof", "-1"])?;
    let first = frame(&[])?;
    (!dark(&first)).then(|| apart(&tail[tail.len() - 36 * 64..], &first))
}

/// A frame that is all but black (mean under 4 %).
fn dark(f: &[u8]) -> bool {
    f.iter().map(|&x| x as f64).sum::<f64>() / (255.0 * f.len().max(1) as f64) < 0.04
}

/// Every frame from `at` on as 36x64 grey, back to back.
fn frame_all(c: &str, at: &[&str]) -> Option<Vec<u8>> {
    let mut args: Vec<&str> = vec!["-v", "error"];
    args.extend_from_slice(at);
    args.extend_from_slice(&["-i", c, "-vf", "scale=36:64,format=gray", "-f", "rawvideo", "-"]);
    crate::util::run_bytes("ffmpeg", &args).ok().filter(|b| b.len() >= 36 * 64 && b.len() % (36 * 64) == 0)
}

/// Mean difference of two grey frames, 0..1.
fn apart(a: &[u8], b: &[u8]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (*x as f64 - *y as f64).abs()).sum::<f64>() / (255.0 * a.len().max(1) as f64)
}

/// "12-20" or "1:05-1:12.5" as (from, to) seconds.
fn span(s: &str) -> Option<(f64, f64)> {
    let (a, b) = s.split_once('-')?;
    let (a, b) = (secs(a)?, secs(b)?);
    (b > a).then_some((a, b))
}

/// The last keyframe at or before `t` (a stream copy can only start on one); `t` itself when none is listed.
fn keyframe_before(keys: &[f64], t: f64) -> f64 {
    keys.iter().copied().filter(|k| *k <= t + 1e-3).reduce(f64::max).map_or(t, |k| k.min(t))
}

/// `edits cut VIDEO A-B...`: each stretch copied out without re-encoding, LosslessCut's way: no quality lost and done in
/// a moment, the start moved back to the keyframe at or before A (where a copy can start), the end kept at B.
pub fn cut(video: &str, spans: &[String], out: &Path) -> Result<Vec<PathBuf>, String> {
    let src = footage(video)?;
    let ext = src.extension().map(|e| e.to_string_lossy().into_owned()).unwrap_or_else(|| "mp4".into());
    let stem = src.file_stem().unwrap_or_default().to_string_lossy().into_owned();
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut made = vec![];
    for sp in spans {
        let (a, b) = span(sp).ok_or(format!("not a stretch: {sp} (like 12-20 or 1:05-1:12)"))?;
        let probe = crate::util::run(None, "ffprobe", &["-v", "error", "-select_streams", "v:0", "-skip_frame", "nokey", "-show_entries", "frame=pts_time", "-of", "csv=p=0",
            "-read_intervals", &format!("{:.3}%{:.3}", (a - 12.0).max(0.0), a + 0.05), &src.to_string_lossy()]).unwrap_or_default();
        let keys: Vec<f64> = probe.lines().filter_map(|l| l.trim().trim_end_matches(',').parse().ok()).collect();
        let k = keyframe_before(&keys, a);
        let to = out.join(format!("{stem} {}-{}.{ext}", crate::util::mmss(a).replace(':', "."), crate::util::mmss(b).replace(':', ".")));
        crate::util::run(None, "ffmpeg", &["-v", "error", "-y", "-ss", &format!("{k:.3}"), "-i", &src.to_string_lossy(), "-t", &format!("{:.3}", b - k),
            "-map", "0:v?", "-map", "0:a?", "-c", "copy", "-avoid_negative_ts", "make_zero", "-map_metadata", "0", &to.to_string_lossy()])?;
        if k < a - 0.05 {
            eprintln!("edits: {sp} starts at {k:.2}s, the keyframe before {a:.2}s (a lossless cut starts on one)");
        }
        made.push(to);
    }
    Ok(made)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_apart() {
        assert_eq!(apart(&[10, 20, 30], &[10, 20, 30]), 0.0);
        assert!((apart(&[0, 0], &[255, 0]) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn a_fade_from_black_is_no_loop() {
        assert!(dark(&[0, 3, 5, 2]));
        assert!(!dark(&[0, 120, 200, 40]));
    }

    #[test]
    fn reads_ffmpeg_logs() {
        let l = "[Parsed_ebur128_0 @ 0x1] t: 1 M: -20 I: -30.0 LUFS\n[Parsed_ebur128_0 @ 0x1] Summary:\n\n  Integrated loudness:\n    I:         -16.0 LUFS\n    Threshold: -26.0 LUFS\n";
        assert_eq!(lufs(l), Some(-16.0));
        assert_eq!(lufs("nothing"), None);
        let c = "[Parsed_showinfo_1 @ 0x1] n: 0 pts: 1 pts_time:1.5 pos: 1\n[Parsed_showinfo_1 @ 0x1] n: 1 pts: 2 pts_time:4 pos: 2\n";
        assert_eq!(cut_times(c), [1.5, 4.0]);
        assert_eq!(cut_times("pts_time:5.3 x\npts_time:5.37 x\npts_time:5.43 x\npts_time:7 x"), [5.3, 7.0]);
        assert_eq!(shots(&[1.5, 4.0], 10.0), [1.5, 2.5, 6.0]);
        assert_eq!(shots(&[], 10.0), [10.0]);
        assert_eq!((secs("336.5"), secs("5:36"), secs("1:00:00"), secs("x")), (Some(336.5), Some(336.0), Some(3600.0), None));
    }

    #[test]
    fn lossless_spans_start_on_a_keyframe() {
        assert_eq!(super::span("12-20"), Some((12.0, 20.0)));
        assert_eq!(super::span("1:05-1:12.5"), Some((65.0, 72.5)));
        assert_eq!(super::span("20-12"), None);
        let keys = [0.0, 2.0, 4.0, 6.0];
        assert_eq!(super::keyframe_before(&keys, 5.3), 4.0);
        assert_eq!(super::keyframe_before(&keys, 4.0), 4.0);
        assert_eq!(super::keyframe_before(&[], 5.3), 5.3);
    }
}
