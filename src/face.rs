//! Where are the faces? Lets a wide video be cropped to a tall one around the people instead of around the middle.
use crate::util::run_bytes;
use std::path::Path;

pub const MODEL: &[u8] = include_bytes!("../models/seeta_fd_frontal_v1.0.bin");
const W: u32 = 480;

/// The biggest face in each sampled frame of `[start, start+dur]` as (x, y, w, h), fractions of the frame.
pub fn boxes(video: &Path, start: f64, dur: f64, src_w: u32, src_h: u32) -> Vec<(f64, f64, f64, f64)> {
    if src_w == 0 || src_h == 0 {
        return vec![];
    }
    let h = (W * src_h / src_w / 2 * 2).max(2);
    let Ok(raw) = run_bytes("ffmpeg", &[
        "-v", "error", "-ss", &format!("{start:.3}"), "-t", &format!("{dur:.3}"), "-i", &video.to_string_lossy(),
        "-vf", &format!("fps=2,scale={W}:{h},format=gray"), "-f", "rawvideo", "-",
    ]) else { return vec![] };
    let Ok(model) = rustface::read_model(MODEL) else { return vec![] };
    let mut det = rustface::create_detector_with_model(model);
    det.set_min_face_size(24);
    det.set_score_thresh(2.0);
    det.set_pyramid_scale_factor(0.8);
    det.set_slide_window_step(4, 4);
    raw.chunks_exact((W * h) as usize).filter_map(|frame| {
        // the biggest face in the frame is the one being filmed
        let f = det.detect(&rustface::ImageData::new(frame, W, h)).into_iter().max_by_key(|f| f.bbox().width())?;
        let b = f.bbox();
        Some((b.x().max(0) as f64 / W as f64, b.y().max(0) as f64 / h as f64, b.width() as f64 / W as f64, b.height() as f64 / h as f64))
    }).collect()
}

/// Horizontal centre (0..1) of the faces in `[start, start+dur]`, or None when no face shows up.
pub fn center_x(video: &Path, start: f64, dur: f64, src_w: u32, src_h: u32) -> Option<f64> {
    let mut xs: Vec<f64> = boxes(video, start, dur, src_w, src_h).into_iter().map(|b| b.0 + b.2 / 2.0).collect();
    median(&mut xs)
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
    fn model_loads() {
        assert!(rustface::read_model(MODEL).is_ok());
    }
}
