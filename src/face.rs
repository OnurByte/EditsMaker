//! Where are the faces? Lets a wide video be cropped to a tall one around the people instead of around the middle.
use crate::util::run_bytes;
use std::path::Path;

pub const MODEL: &[u8] = include_bytes!("../models/seeta_fd_frontal_v1.0.bin");
const W: u32 = 480;

/// The biggest face in each sampled frame of `[start, start+dur]` as (x, y, w, h), fractions of the frame.
pub fn boxes(video: &Path, start: f64, dur: f64, src_w: u32, src_h: u32) -> Vec<(f64, f64, f64, f64)> {
    boxes_at(video, start, dur, src_w, src_h, W, 24)
}

/// The same, looked for in frames `width` wide, faces from `min` pixels.
fn boxes_at(video: &Path, start: f64, dur: f64, src_w: u32, src_h: u32, width: u32, min: u32) -> Vec<(f64, f64, f64, f64)> {
    if src_w == 0 || src_h == 0 {
        return vec![];
    }
    let (w, h) = (width, (width * src_h / src_w / 2 * 2).max(2));
    let Ok(raw) = run_bytes("ffmpeg", &[
        "-v", "error", "-ss", &format!("{start:.3}"), "-t", &format!("{dur:.3}"), "-i", &video.to_string_lossy(),
        "-vf", &format!("fps=2,scale={w}:{h},format=gray"), "-f", "rawvideo", "-",
    ]) else { return vec![] };
    let Ok(model) = rustface::read_model(MODEL) else { return vec![] };
    let mut det = rustface::create_detector_with_model(model);
    det.set_min_face_size(min);
    det.set_score_thresh(2.0);
    det.set_pyramid_scale_factor(0.8);
    det.set_slide_window_step(4, 4);
    raw.chunks_exact((w * h) as usize).filter_map(|frame| {
        // the biggest face in the frame is the one being filmed
        let f = det.detect(&rustface::ImageData::new(frame, w, h)).into_iter().max_by_key(|f| f.bbox().width())?;
        let b = f.bbox();
        Some((b.x().max(0) as f64 / w as f64, b.y().max(0) as f64 / h as f64, b.width() as f64 / w as f64, b.height() as f64 / h as f64))
    }).collect()
}

/// Horizontal centre (0..1) of the faces in `[start, start+dur]`, or None when no face shows up.
pub fn center_x(video: &Path, start: f64, dur: f64, src_w: u32, src_h: u32) -> Option<f64> {
    let mut xs: Vec<f64> = boxes(video, start, dur, src_w, src_h).into_iter().map(|b| b.0 + b.2 / 2.0).collect();
    median(&mut xs)
}

/// A streamer's facecam in `[start, start+dur]`: the webcam's box (x, y, w, h, fractions of the frame), head and
/// shoulders around a small face that sits still near an edge. None when the face fills the frame (no facecam,
/// someone filmed) or there is none.
pub fn cam_box(video: &Path, start: f64, dur: f64, src_w: u32, src_h: u32) -> Option<(f64, f64, f64, f64)> {
    // twice the usual detail: a facecam's face is small
    cam_from(&boxes_at(video, start, dur.min(20.0), src_w, src_h, W * 2, 20))
}

pub fn cam_from(bs: &[(f64, f64, f64, f64)]) -> Option<(f64, f64, f64, f64)> {
    if bs.len() < 3 {
        return None;
    }
    let med = |f: fn(&(f64, f64, f64, f64)) -> f64| median(&mut bs.iter().map(f).collect::<Vec<_>>()).unwrap_or(0.0);
    let (w, h) = (med(|b| b.2), med(|b| b.3));
    let (cx, cy) = (med(|b| b.0 + b.2 / 2.0), med(|b| b.1 + b.3 / 2.0));
    let edge = !(0.3..=0.7).contains(&cx) || !(0.3..=0.7).contains(&cy);
    // it stays put: a facecam does not wander, a face in the game does
    let steady = bs.iter().filter(|b| (b.0 + b.2 / 2.0 - cx).abs() < 0.05 && (b.1 + b.3 / 2.0 - cy).abs() < 0.05).count() * 3 >= bs.len() * 2;
    if w > 0.2 || !edge || !steady {
        return None;
    }
    let (bw, bh) = ((w * 3.4).min(1.0), (h * 2.8).min(1.0));
    Some(((cx - bw / 2.0).clamp(0.0, 1.0 - bw), (cy + h * 0.2 - bh / 2.0).clamp(0.0, 1.0 - bh), bw, bh))
}

pub fn median(v: &mut [f64]) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(f64::total_cmp);
    Some(v[v.len() / 2])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_of_faces() {
        assert_eq!(median(&mut []), None);
        assert_eq!(median(&mut [0.9, 0.2, 0.3]), Some(0.3));
    }

    #[test]
    fn a_facecam_is_a_small_still_face_at_the_edge() {
        // a 0.05-wide face in the top right, steady
        let cam = vec![(0.85, 0.08, 0.05, 0.09); 6];
        let (x, y, w, h) = cam_from(&cam).unwrap();
        assert!(x > 0.6 && x + w <= 1.0 && y >= 0.0 && w > 0.15 && h > 0.2, "{x} {y} {w} {h}");
        // a talking head fills the frame; a face that wanders is in the game
        assert_eq!(cam_from(&[(0.35, 0.2, 0.3, 0.5); 6]), None);
        assert_eq!(cam_from(&[(0.05, 0.1, 0.05, 0.09), (0.6, 0.7, 0.05, 0.09), (0.9, 0.1, 0.05, 0.09), (0.1, 0.8, 0.05, 0.09)]), None);
        assert_eq!(cam_from(&cam[..2]), None);
    }

    #[test]
    fn model_loads() {
        assert!(rustface::read_model(MODEL).is_ok());
    }
}
