//! Where are the faces? Lets a wide video be cropped to a tall one around the people instead of around the middle.
use crate::util::run_bytes;
use std::path::Path;

pub const MODEL: &[u8] = include_bytes!("../models/seeta_fd_frontal_v1.0.bin");
const W: u32 = 480;

/// Horizontal centre (0..1) of the faces in `[start, start+dur]`, or None when no face shows up.
/// shortcut: one fixed crop per clip (median face), no per-shot tracking; upgrade when two-person scenes ping-pong.
pub fn center_x(video: &Path, start: f64, dur: f64, src_w: u32, src_h: u32) -> Option<f64> {
    if src_w == 0 || src_h == 0 {
        return None;
    }
    let h = (W * src_h / src_w / 2 * 2).max(2);
    let raw = run_bytes("ffmpeg", &[
        "-v", "error", "-ss", &format!("{start:.3}"), "-t", &format!("{dur:.3}"), "-i", &video.to_string_lossy(),
        "-vf", &format!("fps=2,scale={W}:{h},format=gray"), "-f", "rawvideo", "-",
    ]).ok()?;
    let model = rustface::read_model(MODEL).ok()?;
    let mut det = rustface::create_detector_with_model(model);
    det.set_min_face_size(24);
    det.set_score_thresh(2.0);
    det.set_pyramid_scale_factor(0.8);
    det.set_slide_window_step(4, 4);
    let mut xs: Vec<f64> = vec![];
    for frame in raw.chunks_exact((W * h) as usize) {
        // the biggest face in the frame is the one being filmed
        if let Some(f) = det.detect(&rustface::ImageData::new(frame, W, h)).into_iter().max_by_key(|f| f.bbox().width()) {
            xs.push((f.bbox().x() as f64 + f.bbox().width() as f64 / 2.0) / W as f64);
        }
    }
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
