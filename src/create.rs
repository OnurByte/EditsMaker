//! Makes a preset from nothing but a video: the video sets the rhythm (its cuts, and extra cuts where it moves
//! most) and how hard each shot is hit; the moves themselves come from other presets, picked by what they do.
use crate::learn::{change, cuts_and_flashes, frames};
use crate::preset::{Curve, Flash, Preset, Slot};
use std::path::Path;

/// Longest shot kept whole, in seconds; longer ones are cut again where the picture changes most.
const LONGEST: f64 = 2.0;

fn range(c: &Curve) -> f64 {
    if c.is_empty() { 0.0 } else { c.max() - c.min() }
}

/// How much a shot does to the picture: zoom, slide, turn, shake, blur.
fn strength(s: &Slot) -> f64 {
    range(&s.zoom) + 3.0 * (range(&s.x) + range(&s.y)) + range(&s.rot) / 15.0 + if s.shake.is_empty() { 0.0 } else { 20.0 * s.shake.max() } + range(&s.blur)
}

/// Shot boundaries in frames (first is 0, last is the end), long shots cut again at the biggest change.
fn bounds(g: &[Vec<u8>], fps: f64) -> Vec<usize> {
    let d: Vec<f64> = (0..g.len()).map(|i| if i == 0 { 0.0 } else { change(&g[i - 1], &g[i]) }).collect();
    let mut cuts = cuts_and_flashes(g).0;
    cuts.push(g.len());
    let mut out = vec![];
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        out.push(a);
        let parts = ((b - a) as f64 / fps / LONGEST).ceil() as usize;
        for k in 1..parts.max(1) {
            let (ideal, r) = (a + k * (b - a) / parts, (b - a) / parts / 3);
            out.push((ideal.saturating_sub(r).max(a + 1)..(ideal + r).min(b - 1)).max_by(|&x, &y| d[x].total_cmp(&d[y])).unwrap_or(ideal));
        }
    }
    out.push(g.len());
    out.dedup();
    out
}

/// A slot for each shot (seconds, how hard the video moves there): the pool's strongest moves go to the hardest shots.
fn compose(shots: &[(f64, f64, f64)], pool: &[Slot]) -> Vec<Slot> {
    let mut pool: Vec<&Slot> = pool.iter().collect();
    pool.sort_by(|a, b| strength(a).total_cmp(&strength(b)));
    let mut order: Vec<usize> = (0..shots.len()).collect();
    order.sort_by(|&a, &b| shots[a].2.total_cmp(&shots[b].2));
    let rank: Vec<f64> = (0..shots.len()).map(|i| order.iter().position(|&o| o == i).unwrap_or(0) as f64 / (shots.len() - 1).max(1) as f64).collect();
    let mut last = usize::MAX;
    shots.iter().zip(rank).map(|(&(start, end, _), q)| {
        let mid = (q * (pool.len() - 1) as f64).round() as usize;
        let cost = |i: usize| (pool[i].end - pool[i].start - (end - start)).abs() + 0.3 * (i as f64 - mid as f64).abs();
        let near = mid.saturating_sub(2)..(mid + 3).min(pool.len());
        // among the neighbours, the one that plays for about as long, and not the one just used
        let pick = near.filter(|&i| i != last || pool.len() < 2)
            .min_by(|&a, &b| cost(a).total_cmp(&cost(b))).unwrap_or(mid);
        last = pick;
        Slot { start, end, ..pool[pick].clone() }
    }).collect()
}

/// The presets to draw from: the named ones, else the three whose shots run closest to the video's.
fn inspiration(like: &[String], name: &str, mean_shot: f64) -> Result<Vec<Preset>, String> {
    if !like.is_empty() {
        return like.iter().map(|n| crate::preset::load(n)).collect();
    }
    let mut all: Vec<Preset> = crate::presets::all().into_iter().filter(|p| p.name != name && !p.slots.is_empty()).collect();
    let gap = |p: &Preset| (p.total / p.slots.len() as f64 - mean_shot).abs();
    all.sort_by(|a, b| gap(a).total_cmp(&gap(b)));
    all.truncate(3);
    if all.is_empty() { Err("no presets to take inspiration from".into()) } else { Ok(all) }
}

pub fn create(video: &Path, name: &str, like: &[String], max_len: f64) -> Result<Preset, String> {
    let f = frames(video)?;
    let g = &f.gray[..f.gray.len().min((max_len * f.fps) as usize)];
    let cuts = bounds(g, f.fps);
    let d: Vec<f64> = (0..g.len()).map(|i| if i == 0 { 0.0 } else { change(&g[i - 1], &g[i]) }).collect();
    let t = |i: usize| (i as f64 / f.fps * 1000.0).round() / 1000.0;
    let shots: Vec<(f64, f64, f64)> = cuts.windows(2).map(|w| (t(w[0]), t(w[1]), d[w[0]..w[1]].iter().sum::<f64>() / (w[1] - w[0]) as f64)).collect();
    let seeds = inspiration(like, name, t(g.len()) / shots.len() as f64)?;
    let pool: Vec<Slot> = seeds.iter().flat_map(|p| p.slots.clone()).collect();
    let slots = compose(&shots, &pool);
    // flashes as often as the inspirations use them, on the hardest cuts, in the first one's style
    let rate = seeds.iter().map(|p| p.flashes.len() as f64 / p.slots.len() as f64).sum::<f64>() / seeds.len() as f64;
    let model = seeds.iter().flat_map(|p| p.flashes.first()).next();
    let mut hard: Vec<usize> = (1..shots.len()).collect();
    hard.sort_by(|&a, &b| shots[b].2.total_cmp(&shots[a].2));
    hard.truncate((rate * shots.len() as f64).round() as usize);
    hard.sort();
    let flashes: Vec<Flash> = model.into_iter().flat_map(|m| hard.iter().map(|&i| Flash { start: shots[i].0, end: shots[i].0 + (m.end - m.start), ..m.clone() })).collect();
    let (vw, vh) = crate::fetch::dims(video);
    let mut p = seeds[0].clone();
    p.name = name.into();
    p.about = format!("made from {}: {} shots, moves from {}", video.file_name().unwrap_or_default().to_string_lossy(), slots.len(), seeds.iter().map(|s| s.name.as_str()).collect::<Vec<_>>().join(", "));
    (p.w, p.h, p.total, p.song, p.slots, p.flashes) = (vw, vh, shots.last().map_or(0.0, |s| s.1), None, slots, flashes);
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(zoom: f64) -> Slot {
        serde_json::from_value(serde_json::json!({"start": 0.0, "end": 1.0, "zoom": [[0.0, 1.0, "linear"], [1.0, zoom, "linear"]]})).unwrap()
    }

    #[test]
    fn hardest_shots_get_strongest_moves() {
        let pool = [slot(1.1), slot(1.5), slot(1.9)];
        let out = compose(&[(0.0, 1.0, 0.5), (1.0, 2.0, 0.1), (2.0, 3.0, 0.9)], &pool);
        let z: Vec<f64> = out.iter().map(|s| s.zoom.max()).collect();
        assert_eq!(z, vec![1.5, 1.1, 1.9]);
        assert_eq!((out[2].start, out[2].end), (2.0, 3.0));
    }

    #[test]
    fn long_shots_are_cut_again() {
        let g: Vec<Vec<u8>> = (0..60).map(|i| vec![if i >= 22 { 22 } else { 20 }; 64]).collect();
        let b = bounds(&g, 10.0); // 6 s, no cut found: split in three, one boundary at the small step
        assert!(b.len() >= 4 && b[0] == 0 && *b.last().unwrap() == 60 && b.contains(&22), "{b:?}");
    }
}
