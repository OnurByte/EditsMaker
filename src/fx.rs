//! Effects any edit can use, taken out of the presets and the editors' toolboxes: hits on the beat (shake, flash,
//! colour split, glitch, negative, blur, freeze frame), speed ramps, face tracking and blurring a face or a region.
use crate::face::median;
use crate::montage::Frame;
use crate::preset::{Curve, Ease};

pub const HITS: &[&str] = &["shake", "flash", "rgb", "glitch", "invert", "blur", "freeze", "zoom"];

/// What a shot gets at its first moments when it lands on a hard beat. Filters in `t` (the shot's own clock),
/// on a frame of `w` x `h`. "freeze" is handled before the shot is cut (see `freeze`).
pub fn hit(kinds: &[String], w: u32, h: u32) -> String {
    let mut f = String::new();
    for k in kinds {
        match k.as_str() {
            "shake" => f.push_str(&format!(
                ",crop=iw*0.92:ih*0.92:x='(iw-ow)/2+iw*0.035*sin(t*90)*max(0,1-t/0.35)':y='(ih-oh)/2+ih*0.03*cos(t*77)*max(0,1-t/0.35)',scale={w}:{h},setsar=1"
            )),
            "flash" => f.push_str(",eq=brightness='0.55*max(0,1-t/0.14)':eval=frame"),
            "rgb" => f.push_str(",rgbashift=rh=-12:bh=12:rv=4:bv=-4:enable='lt(t,0.25)'"),
            "glitch" => f.push_str(",noise=alls=45:allf=t:enable='lt(t,0.18)',rgbashift=gh=10:gv=-6:enable='between(t,0.04,0.16)'"),
            "invert" => f.push_str(",negate=enable='lt(t,0.07)'"),
            "blur" => f.push_str(",gblur=sigma=18:enable='lt(t,0.1)'"),
            "zoom" => f.push_str(&format!(",zoompan=z='if(lt(it,0.2),1.25-1.25*it,1)':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=1:s={w}x{h}:fps=30,setsar=1")),
            _ => {}
        }
    }
    f
}

/// The freeze frame ("buz"): the shot's first frame held for `secs`, then it plays. Goes right after the source.
pub fn freeze(secs: f64) -> String {
    format!(",tpad=start_mode=clone:start_duration={secs:.2}")
}

pub fn wants_freeze(kinds: &[String]) -> bool {
    kinds.iter().any(|k| k == "freeze")
}

/// A speed ramp as cut pieces: (source from, source length, output length) for `parts` stretches of a shot
/// lasting `len` seconds, each at `base` x the ramp's speed in its middle.
pub fn ramp_parts(ramp: &Curve, base: f64, len: f64, parts: usize) -> Vec<(f64, f64, f64)> {
    let d = len / parts as f64;
    let mut at = 0.0;
    (0..parts).map(|k| {
        let sp = (base * ramp.at((k as f64 + 0.5) / parts as f64)).clamp(0.1, 8.0);
        let p = (at, d * sp, d);
        at += d * sp;
        p
    }).collect()
}

/// A velocity ramp: slow in, rushing through the middle, slowing onto the next beat.
pub fn velocity() -> Curve {
    Curve { keys: vec![(0.0, 0.7, Ease::Linear), (0.35, 2.6, Ease::Out(2.0)), (0.75, 0.45, Ease::In(2.0)), (1.0, 0.8, Ease::Linear)] }
}

/// A face followed through a shot: where the face is (0..1 across) over the shot's output time, smoothed so the
/// frame glides instead of jumping. `frames` are a video's measured frames at `fps`; the shot plays source
/// `from`..`from + src_len`. None when no face is seen.
pub fn track(frames: &[Frame], fps: f64, from: f64, src_len: f64) -> Option<Curve> {
    let (a, b) = ((from * fps) as usize, (((from + src_len) * fps).ceil() as usize).min(frames.len()));
    if b <= a {
        return None;
    }
    let seen: Vec<(f64, f64)> = (a..b).filter_map(|i| Some(((i - a) as f64 / (b - a).max(1) as f64, frames[i].face?))).collect();
    if seen.len() < 2 {
        return None;
    }
    // a 1-D Kalman-style filter forwards, then backwards, so the camera leads no less than it lags
    let smooth = |xs: Vec<f64>| -> Vec<f64> {
        let (q, r) = (0.002, 0.02);
        let (mut x, mut p) = (xs[0], 1.0);
        xs.into_iter().map(|z| {
            p += q;
            let k = p / (p + r);
            x += k * (z - x);
            p *= 1.0 - k;
            x
        }).collect()
    };
    let fwd = smooth(seen.iter().map(|s| s.1).collect());
    let mut back = smooth(seen.iter().rev().map(|s| s.1).collect());
    back.reverse();
    let keys = seen.iter().zip(fwd.iter().zip(&back)).map(|((u, _), (f, b))| (*u, ((f + b) / 2.0).clamp(0.0, 1.0), Ease::Linear)).collect();
    Some(Curve { keys })
}

/// A crop across the frame following `x` (an expression in `t`, 0..1 across) to `w` x `h`.
pub fn follow(w: u32, h: u32, x: &str) -> String {
    // lanczos: the crop is usually blown up, and bicubic softens it further
    format!("crop=w='min(iw,ih*{w}/{h})':h='min(ih,iw*{h}/{w})':x='max(0,min(iw-ow,iw*({x})-ow/2))',scale={w}:{h}:flags=lanczos,setsar=1")
}

/// Blurs a box of the source frame (fractions x, y, w, h), before any crop. `p` keeps labels unique.
pub fn blur_box(p: &str, b: (f64, f64, f64, f64)) -> String {
    let (x, y, w, h) = (b.0.clamp(0.0, 1.0), b.1.clamp(0.0, 1.0), b.2.clamp(0.01, 1.0), b.3.clamp(0.01, 1.0));
    format!(",split[{p}u][{p}v];[{p}v]crop=iw*{w:.3}:ih*{h:.3}:iw*{x:.3}:ih*{y:.3},boxblur=24:3[{p}w];[{p}u][{p}w]overlay=main_w*{x:.3}:main_h*{y:.3}")
}

/// "x,y,w,h" as fractions of the frame.
pub fn parse_box(s: &str) -> Option<(f64, f64, f64, f64)> {
    let v: Vec<f64> = s.split(',').filter_map(|x| x.trim().parse().ok()).collect();
    match v[..] {
        [x, y, w, h] if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) && w > 0.0 && h > 0.0 => Some((x, y, w, h)),
        _ => None,
    }
}

/// The faces of a shot as one box to blur (the union of where they were, padded), or None.
pub fn face_box(boxes: &mut [(f64, f64, f64, f64)]) -> Option<(f64, f64, f64, f64)> {
    let mut xs: Vec<f64> = boxes.iter().map(|b| b.0).collect();
    let mut ys: Vec<f64> = boxes.iter().map(|b| b.1).collect();
    let mut ws: Vec<f64> = boxes.iter().map(|b| b.2).collect();
    let mut hs: Vec<f64> = boxes.iter().map(|b| b.3).collect();
    let (x, y, w, h) = (median(&mut xs)?, median(&mut ys)?, median(&mut ws)?, median(&mut hs)?);
    let pad = 0.35;
    Some(((x - w * pad).max(0.0), (y - h * pad).max(0.0), (w * (1.0 + 2.0 * pad)).min(1.0), (h * (1.0 + 2.0 * pad)).min(1.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hits_are_filters() {
        let h = hit(&["shake".into(), "flash".into(), "nope".into()], 1080, 1920);
        assert!(h.starts_with(",crop=iw*0.92") && h.contains("scale=1080:1920,setsar=1,eq=brightness=") && !h.contains("nope"), "{h}");
        assert!(wants_freeze(&["freeze".into()]) && freeze(0.35) == ",tpad=start_mode=clone:start_duration=0.35");
    }

    #[test]
    fn ramps_cover_the_shot() {
        let p = ramp_parts(&velocity(), 1.0, 2.0, 4);
        assert_eq!(p.len(), 4);
        assert!(p.iter().all(|x| (x.2 - 0.5).abs() < 1e-9));
        // fast in the middle, so the middle parts eat more source
        assert!(p[1].1 > p[0].1 && p[1].1 > p[3].1, "{p:?}");
        // pieces follow on from each other in the source
        assert!((p[1].0 - p[0].1).abs() < 1e-9);
    }

    #[test]
    fn tracking_glides() {
        let f = |face: Option<f64>| Frame { face, ..Default::default() };
        let mut frames: Vec<Frame> = (0..40).map(|i| f(if i < 20 { Some(0.3) } else { Some(0.7) })).collect();
        frames[10] = f(None);
        let c = track(&frames, 4.0, 0.0, 10.0).unwrap();
        assert!(c.at(0.0) < 0.4 && c.at(1.0) > 0.6, "{:?}", c);
        // no jump: neighbouring keys stay close
        assert!(c.keys.windows(2).all(|w| (w[0].1 - w[1].1).abs() < 0.15), "{:?}", c);
        assert!(track(&frames, 4.0, 50.0, 2.0).is_none());
        assert!(track(&[f(None); 10], 4.0, 0.0, 2.0).is_none());
    }

    #[test]
    fn boxes() {
        assert_eq!(parse_box("0.1,0.2,0.3,0.4"), Some((0.1, 0.2, 0.3, 0.4)));
        assert_eq!(parse_box("1,2"), None);
        let b = face_box(&mut [(0.4, 0.2, 0.2, 0.3), (0.42, 0.22, 0.2, 0.3)]).unwrap();
        assert!(b.0 < 0.4 && b.2 > 0.2);
        assert!(blur_box("q", b).contains("[qu][qw]overlay=main_w*"));
        assert!(face_box(&mut []).is_none());
    }
}
