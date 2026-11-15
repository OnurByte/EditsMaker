//! Reverse-engineers a finished edit into a preset: where it cuts, where it flashes, and how the picture moves in
//! each shot (zoom, slide, turn, shake), measured from the frames. The approach follows rea-video (Apache-2.0):
//! cuts from histogram and pixel change against a local adaptive threshold, with flashes that recover within two
//! frames kept apart from cuts, and camera motion fitted between frames as a similarity transform with outliers
//! dropped. The result is an ordinary preset: replay it over other footage, retime it to a song, change it as JSON.
use crate::preset::{Curve, Ease, Flash, Preset, Slot};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};

/// Analysis width; the height follows the frame.
const W: usize = 128;
/// Every value this module decides on, in one place.
const BLOCK: usize = 16;
const SEARCH: i32 = 6;
/// A cut is this many times the local typical change, and at least `CUT_MIN`.
const CUT_K: f64 = 3.0;
const CUT_MIN: f64 = 0.12;
/// A flash: brightness jumps by this much and is back within two frames.
const FLASH_JUMP: f64 = 0.12;

pub struct Frames {
    pub w: usize,
    pub h: usize,
    pub fps: f64,
    pub gray: Vec<Vec<u8>>,
}

/// The edit's frames, small and grey, at its own frame rate (capped at 60).
pub fn frames(video: &Path) -> Result<Frames, String> {
    let (vw, vh) = crate::fetch::dims(video);
    if vw == 0 || vh == 0 {
        return Err(format!("{}: no video", video.display()));
    }
    let h = ((W as f64 * vh as f64 / vw as f64 / 2.0).round() as usize * 2).max(2);
    let fps = probe_fps(video).unwrap_or(30.0).clamp(10.0, 60.0);
    let mut child = Command::new("ffmpeg")
        .args(["-v", "error", "-i", &video.to_string_lossy(), "-an", "-vf", &format!("fps={fps},scale={W}:{h},format=gray"), "-f", "rawvideo", "-"])
        .stdout(Stdio::piped()).spawn().map_err(|e| format!("ffmpeg: {e}"))?;
    let mut raw = vec![];
    child.stdout.take().ok_or("ffmpeg gave no output")?.read_to_end(&mut raw).map_err(|e| e.to_string())?;
    let _ = child.wait();
    let gray: Vec<Vec<u8>> = raw.chunks_exact(W * h).map(|c| c.to_vec()).collect();
    if gray.len() < 3 {
        return Err(format!("{}: too short to learn from", video.display()));
    }
    Ok(Frames { w: W, h, fps, gray })
}

fn probe_fps(video: &Path) -> Option<f64> {
    let out = crate::util::run(None, "ffprobe", &["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=avg_frame_rate", "-of", "csv=p=0", &video.to_string_lossy()]).ok()?;
    let (a, b) = out.trim().split_once('/')?;
    let (a, b): (f64, f64) = (a.parse().ok()?, b.parse().ok()?);
    (b > 0.0).then_some(a / b)
}

fn luma(f: &[u8]) -> f64 {
    f.iter().map(|&v| v as f64).sum::<f64>() / f.len() as f64 / 255.0
}

/// How different two frames are: half grey-histogram distance, half mean pixel difference (0..1 each).
pub fn change(a: &[u8], b: &[u8]) -> f64 {
    let hist = |f: &[u8]| {
        let mut h = [0f64; 32];
        f.iter().for_each(|&v| h[v as usize / 8] += 1.0);
        h.map(|x| x / f.len() as f64)
    };
    let (ha, hb) = (hist(a), hist(b));
    let dh = ha.iter().zip(&hb).map(|(x, y)| (x - y).abs()).sum::<f64>() / 2.0;
    let dp = a.iter().zip(b).map(|(&x, &y)| (x as f64 - y as f64).abs()).sum::<f64>() / a.len() as f64 / 255.0;
    0.5 * dh + dp
}

fn median(v: &mut [f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.total_cmp(b));
    v[v.len() / 2]
}

/// Frame numbers where a new shot starts, and frames that flash (bright for a moment, then back).
pub fn cuts_and_flashes(g: &[Vec<u8>]) -> (Vec<usize>, Vec<usize>) {
    let n = g.len();
    let d: Vec<f64> = (0..n).map(|i| if i == 0 { 0.0 } else { change(&g[i - 1], &g[i]) }).collect();
    let l: Vec<f64> = g.iter().map(|f| luma(f)).collect();
    let mut flashes = vec![];
    let mut cuts = vec![0];
    let mut i = 1;
    while i < n {
        let back = (i + 2).min(n - 1);
        let flash = l[i] - l[i - 1] > FLASH_JUMP && (1..=2).any(|k| i + k < n && (l[i + k] - l[i - 1]).abs() < FLASH_JUMP / 2.0);
        if flash {
            flashes.push(i);
            // a flash can hide a cut: compare what came before with what came after
            if change(&g[i - 1], &g[back]) > CUT_MIN * 1.5 && back - cuts[cuts.len() - 1] >= 3 {
                cuts.push(back);
            }
            i = back + 1;
            continue;
        }
        let mut local: Vec<f64> = d[i.saturating_sub(15)..(i + 16).min(n)].to_vec();
        let typical = median(&mut local);
        if d[i] > CUT_MIN.max(CUT_K * typical) && i - cuts[cuts.len() - 1] >= 3 {
            cuts.push(i);
        }
        i += 1;
    }
    (cuts, flashes)
}

/// The block that `a` at (bx, by) moved to in `b`, to a fraction of a pixel, and how well it matched.
fn match_block(a: &[u8], b: &[u8], w: usize, h: usize, bx: usize, by: usize) -> Option<(f64, f64)> {
    let sad = |dx: i32, dy: i32| -> Option<u32> {
        let mut s = 0u32;
        for y in 0..BLOCK {
            let yy = by as i32 + y as i32 + dy;
            if yy < 0 || yy >= h as i32 {
                return None;
            }
            for x in 0..BLOCK {
                let xx = bx as i32 + x as i32 + dx;
                if xx < 0 || xx >= w as i32 {
                    return None;
                }
                s += (a[(by + y) * w + bx + x] as i32 - b[yy as usize * w + xx as usize] as i32).unsigned_abs();
            }
        }
        Some(s)
    };
    let mut best = (u32::MAX, 0, 0);
    for dy in -SEARCH..=SEARCH {
        for dx in -SEARCH..=SEARCH {
            if let Some(s) = sad(dx, dy) {
                if s < best.0 {
                    best = (s, dx, dy);
                }
            }
        }
    }
    let (s0, dx, dy) = best;
    if s0 == u32::MAX || s0 as f64 / (BLOCK * BLOCK) as f64 > 18.0 {
        return None;
    }
    // a parabola through the neighbours puts the minimum between pixels
    let sub = |m: Option<u32>, p: Option<u32>| match (m, p) {
        (Some(m), Some(p)) => {
            let den = m as f64 + p as f64 - 2.0 * s0 as f64;
            if den > 0.0 { ((m as f64 - p as f64) / (2.0 * den)).clamp(-0.5, 0.5) } else { 0.0 }
        }
        _ => 0.0,
    };
    Some((dx as f64 + sub(sad(dx - 1, dy), sad(dx + 1, dy)), dy as f64 + sub(sad(dx, dy - 1), sad(dx, dy + 1))))
}

/// The camera move between two frames as (scale, turn in radians, shift x, shift y) in pixels: a similarity
/// transform fitted to the textured blocks' motion, refitted without the blocks it explains worst.
pub fn motion(a: &[u8], b: &[u8], w: usize, h: usize) -> (f64, f64, f64, f64) {
    let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
    let mut pts: Vec<(f64, f64, f64, f64)> = vec![];
    for by in (0..h.saturating_sub(BLOCK)).step_by(BLOCK) {
        for bx in (0..w.saturating_sub(BLOCK)).step_by(BLOCK) {
            let blk: Vec<f64> = (0..BLOCK).flat_map(|y| (0..BLOCK).map(move |x| (y, x))).map(|(y, x)| a[(by + y) * w + bx + x] as f64).collect();
            let mean = blk.iter().sum::<f64>() / blk.len() as f64;
            let var = blk.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / blk.len() as f64;
            if var < 60.0 {
                continue; // flat blocks match anywhere
            }
            if let Some((dx, dy)) = match_block(a, b, w, h, bx, by) {
                pts.push((bx as f64 + BLOCK as f64 / 2.0 - cx, by as f64 + BLOCK as f64 / 2.0 - cy, dx, dy));
            }
        }
    }
    let mut keep = pts.clone();
    let mut fit = (0.0, 0.0, 0.0, 0.0);
    for _ in 0..3 {
        if keep.len() < 4 {
            return (1.0, 0.0, 0.0, 0.0);
        }
        fit = solve(&keep);
        let err = |p: &(f64, f64, f64, f64)| {
            let (a, b, tx, ty) = fit;
            ((a * p.0 - b * p.1 + tx - p.2).powi(2) + (b * p.0 + a * p.1 + ty - p.3).powi(2)).sqrt()
        };
        let mut e: Vec<f64> = keep.iter().map(err).collect();
        let cut = (median(&mut e) * 2.5).max(0.3);
        keep.retain(|p| err(p) <= cut);
    }
    let (a, b, tx, ty) = fit;
    (((1.0 + a).powi(2) + b * b).sqrt(), b.atan2(1.0 + a), tx, ty)
}

/// Least squares for motion = a*p + b*perp(p) + t over (x, y, dx, dy) points.
fn solve(p: &[(f64, f64, f64, f64)]) -> (f64, f64, f64, f64) {
    let n = p.len() as f64;
    let (mx, my) = (p.iter().map(|q| q.0).sum::<f64>() / n, p.iter().map(|q| q.1).sum::<f64>() / n);
    let (mdx, mdy) = (p.iter().map(|q| q.2).sum::<f64>() / n, p.iter().map(|q| q.3).sum::<f64>() / n);
    let (mut sa, mut sb, mut ss) = (0.0, 0.0, 0.0);
    for q in p {
        let (x, y, u, v) = (q.0 - mx, q.1 - my, q.2 - mdx, q.3 - mdy);
        sa += x * u + y * v;
        sb += x * v - y * u;
        ss += x * x + y * y;
    }
    let (a, b) = if ss > 1e-9 { (sa / ss, sb / ss) } else { (0.0, 0.0) };
    (a, b, mdx - (a * mx - b * my), mdy - (b * mx + a * my))
}

fn smooth(v: &[f64], r: usize) -> Vec<f64> {
    (0..v.len()).map(|i| {
        let s = &v[i.saturating_sub(r)..(i + r + 1).min(v.len())];
        s.iter().sum::<f64>() / s.len() as f64
    }).collect()
}

/// `n` evenly spread keys of a per-frame track, or an empty curve when it hardly moves.
fn keys(track: &[f64], rest: f64, still: f64) -> Curve {
    if track.iter().all(|v| (v - rest).abs() < still) {
        return Curve::default();
    }
    let n = 6.min(track.len());
    let at = |k: usize| k * (track.len() - 1) / (n - 1).max(1);
    Curve { keys: (0..n).map(|k| (at(k) as f64 / (track.len() - 1).max(1) as f64, (track[at(k)] * 1000.0).round() / 1000.0, Ease::Linear)).collect() }
}

/// One shot's moves from its frames: zoom from 1, slide and turn from 0, and how much it shakes.
fn shot(g: &[Vec<u8>], w: usize, h: usize, start: f64, end: f64) -> Slot {
    let (mut z, mut x, mut y, mut r) = (vec![1.0], vec![0.0], vec![0.0], vec![0.0]);
    for p in g.windows(2) {
        let (s, th, tx, ty) = motion(&p[0], &p[1], w, h);
        z.push(z[z.len() - 1] * s.clamp(0.8, 1.25));
        x.push(x[x.len() - 1] + tx / w as f64);
        y.push(y[y.len() - 1] + ty / h as f64);
        r.push(r[r.len() - 1] + th.to_degrees());
    }
    // shake is what is left of the slide after smoothing it; its speed from how often it turns back
    let (sx, sy) = (smooth(&x, 2), smooth(&y, 2));
    let jitter: Vec<f64> = x.iter().zip(&sx).zip(y.iter().zip(&sy)).map(|((a, b), (c, d))| ((a - b).powi(2) + (c - d).powi(2)).sqrt()).collect();
    let amp = (jitter.iter().map(|j| j * j).sum::<f64>() / jitter.len() as f64).sqrt();
    let turns = x.iter().zip(&sx).map(|(a, b)| a - b).collect::<Vec<_>>().windows(2).filter(|p| p[0].signum() != p[1].signum()).count();
    let hz = (turns as f64 / 2.0 / ((end - start).max(0.1))).clamp(1.0, 16.0);
    let mut slot: Slot = serde_json::from_value(serde_json::json!({ "start": (start * 1000.0).round() / 1000.0, "end": (end * 1000.0).round() / 1000.0 })).expect("a bare slot parses");
    slot.zoom = keys(&z, 1.0, 0.02);
    slot.x = keys(&sx, 0.0, 0.01);
    slot.y = keys(&sy, 0.0, 0.01);
    slot.rot = keys(&r, 0.0, 0.5);
    if amp > 0.004 {
        slot.shake = Curve { keys: vec![(0.0, (amp * 1000.0).round() / 1000.0, Ease::Linear)] };
        slot.shake_hz = (hz * 10.0).round() / 10.0;
    }
    slot
}

/// The whole edit as a preset named `name`.
pub fn learn(video: &Path, name: &str) -> Result<Preset, String> {
    let f = frames(video)?;
    let (cuts, flashes) = cuts_and_flashes(&f.gray);
    let n = f.gray.len();
    let t = |i: usize| i as f64 / f.fps;
    let slots: Vec<Slot> = cuts.iter().enumerate().map(|(k, &a)| {
        let b = cuts.get(k + 1).copied().unwrap_or(n);
        shot(&f.gray[a..b], f.w, f.h, t(a), t(b))
    }).collect();
    let flash_list: Vec<Flash> = flashes.iter().map(|&i| Flash {
        start: (t(i) * 1000.0).round() / 1000.0, end: (t(i + 2) * 1000.0).round() / 1000.0, color: "ffffff".into(),
        opacity: Curve { keys: vec![(0.0, 0.9, Ease::Linear), (1.0, 0.0, Ease::Linear)] }, blink: false,
    }).collect();
    let (vw, vh) = crate::fetch::dims(video);
    let moving = slots.iter().filter(|s| s.zoom.is_moving() || s.x.is_moving() || s.y.is_moving() || s.rot.is_moving()).count();
    let mut p: Preset = serde_json::from_value(serde_json::json!({ "name": name, "w": vw, "h": vh, "total": t(n), "shots": [] })).map_err(|e| e.to_string())?;
    p.about = format!("learned from {}: {} shots ({moving} moving), {} flashes", video.file_name().unwrap_or_default().to_string_lossy(), slots.len(), flash_list.len());
    p.fit_beats = true;
    p.slots = slots;
    p.flashes = flash_list;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A textured frame, its content shifted by (dx, dy) and scaled by `s` about the centre.
    fn frame(w: usize, h: usize, dx: f64, dy: f64, s: f64, seed: u32) -> Vec<u8> {
        let tex = |x: f64, y: f64| (((x * 0.37 + seed as f64).sin() * (y * 0.29).cos() + (x * 0.11 - y * 0.07).sin()) * 60.0 + 128.0) as u8;
        let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
        (0..h).flat_map(|y| (0..w).map(move |x| (x, y))).map(|(x, y)| tex((x as f64 - cx - dx) / s + cx, (y as f64 - cy - dy) / s + cy)).collect()
    }

    #[test]
    fn camera_moves_are_measured() {
        let (w, h) = (128, 224);
        let (s, _, tx, ty) = motion(&frame(w, h, 0.0, 0.0, 1.0, 1), &frame(w, h, 2.0, -1.0, 1.0, 1), w, h);
        assert!((tx - 2.0).abs() < 0.3 && (ty + 1.0).abs() < 0.3 && (s - 1.0).abs() < 0.01, "{s} {tx} {ty}");
        let (s, _, tx, _) = motion(&frame(w, h, 0.0, 0.0, 1.0, 1), &frame(w, h, 0.0, 0.0, 1.04, 1), w, h);
        assert!((s - 1.04).abs() < 0.01 && tx.abs() < 0.3, "{s} {tx}");
    }

    #[test]
    fn cuts_and_flashes_apart() {
        let (w, h) = (64, 112);
        let mut g: Vec<Vec<u8>> = (0..10).map(|_| frame(w, h, 0.0, 0.0, 1.0, 1)).collect();
        g.extend((0..10).map(|_| frame(w, h, 0.0, 0.0, 1.0, 7).iter().map(|v| v / 3).collect::<Vec<u8>>()));
        // a white flash in the second shot that is gone two frames later
        g[14] = vec![250; w * h];
        g.extend((0..10).map(|_| frame(w, h, 0.0, 0.0, 1.0, 13)));
        let (cuts, flashes) = cuts_and_flashes(&g);
        assert_eq!(cuts, vec![0, 10, 20], "{flashes:?}");
        assert_eq!(flashes, vec![14]);
    }
}
