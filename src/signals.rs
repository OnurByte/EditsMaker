//! Where did real viewers react? Humour is read from people first, from the model second.
use crate::util::{run, zscores};
use regex::Regex;
use std::path::Path;

pub struct Hit {
    pub t: f64,
    pub weight: f64,
    pub text: String,
}

/// Comments that carry a timestamp ("1:25 efsane 😂"). Likes and laughter raise the weight.
pub fn comment_hits(comments: &[(String, u64)], dur: f64) -> Vec<Hit> {
    let re = Regex::new(r"(?:\b(\d{1,2}):)?\b(\d{1,2}):(\d{2})\b").unwrap();
    let mut hits = vec![];
    for (text, likes) in comments {
        let low = text.to_lowercase();
        let laugh = if crate::lang::get().laugh.iter().any(|w| low.contains(w.as_str())) { 2.0 } else { 1.0 };
        for c in re.captures_iter(text) {
            let n = |i: usize| c.get(i).and_then(|m| m.as_str().parse::<f64>().ok()).unwrap_or(0.0);
            let t = n(1) * 3600.0 + n(2) * 60.0 + n(3);
            if t < dur {
                hits.push(Hit { t, weight: laugh * (1.0 + (*likes as f64).ln_1p()), text: text.replace('\n', " ") });
            }
        }
    }
    hits
}

/// RMS level in dB for every half second of a 16 kHz mono wav.
pub fn loudness(wav: &Path) -> Result<Vec<f64>, String> {
    let w = wav.to_string_lossy();
    let af = "asetnsamples=8000,astats=metadata=1:reset=1,ametadata=print:key=lavfi.astats.Overall.RMS_level:file=-";
    let out = run(None, "ffmpeg", &["-v", "error", "-i", w.as_ref(), "-af", af, "-f", "null", "-"])?;
    Ok(out.lines().filter_map(|l| l.strip_prefix("lavfi.astats.Overall.RMS_level=")).map(|v| v.trim().parse().unwrap_or(-90.0f64).max(-90.0)).collect())
}

/// One score per second: replay heat + timestamped comments + loudness spikes.
pub fn curve(dur: f64, heat: &[(f64, f64, f64)], hits: &[Hit], loud: &[f64]) -> Vec<f64> {
    let n = dur.ceil().max(1.0) as usize;
    let mut h = vec![0.0; n];
    let mean = if heat.is_empty() { 0.0 } else { heat.iter().map(|x| x.2).sum::<f64>() / heat.len() as f64 };
    for &(s, e, v) in heat {
        // everyone watches the first seconds, so that part of the heatmap says nothing
        let v = if s < dur * 0.03 { mean } else { v };
        for cell in h.iter_mut().take((e.ceil() as usize).min(n)).skip(s as usize) {
            *cell = v;
        }
    }
    let mut c = vec![0.0; n];
    for hit in hits {
        // people stamp the start of the bit; the laugh lands a few seconds later
        for cell in c.iter_mut().take(((hit.t + 7.0) as usize).min(n)).skip((hit.t - 2.0).max(0.0) as usize) {
            *cell += hit.weight;
        }
    }
    let cmax = c.iter().cloned().fold(0.0, f64::max);
    let lz = zscores(loud);
    let hz = if heat.is_empty() { h } else { zscores(&h) };
    (0..n).map(|i| {
        let l = lz.iter().skip(i * 2).take(2).cloned().fold(0.0, f64::max);
        hz[i] + if cmax > 0.0 { 3.0 * c[i] / cmax } else { 0.0 } + 0.5 * l
    }).collect()
}

/// The `n` highest seconds that are at least `gap` seconds apart, best first.
pub fn peaks(curve: &[f64], n: usize, gap: usize) -> Vec<usize> {
    let mut order: Vec<usize> = (0..curve.len()).collect();
    order.sort_by(|a, b| curve[*b].total_cmp(&curve[*a]));
    let mut picked: Vec<usize> = vec![];
    for i in order {
        if picked.len() == n {
            break;
        }
        if picked.iter().all(|p| p.abs_diff(i) >= gap) {
            picked.push(i);
        }
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps_in_comments() {
        let c = vec![
            ("0:49 Buda boynu bükük kalmasın :D".to_string(), 788),
            ("1:25 metin abinin çıkışı efsane ya 😂😂".to_string(), 320),
            ("1:02:03 uzun video".to_string(), 0),
            ("saat 12:30 gibi izledim, 9:59".to_string(), 1), // both out of range
            ("zaman damgası yok".to_string(), 99),
        ];
        let h = comment_hits(&c, 114.0);
        assert_eq!(h.iter().map(|x| x.t).collect::<Vec<_>>(), vec![49.0, 85.0]);
        // laughter doubles the weight
        let no_laugh = comment_hits(&[("1:25 a".into(), 320)], 114.0)[0].weight;
        assert!((h[1].weight - 2.0 * no_laugh).abs() < 1e-9);
    }

    #[test]
    fn curve_peaks_where_people_reacted() {
        let heat: Vec<(f64, f64, f64)> = (0..10).map(|i| (i as f64 * 10.0, i as f64 * 10.0 + 10.0, if i == 6 { 1.0 } else if i == 0 { 0.9 } else { 0.2 })).collect();
        let hits = vec![Hit { t: 30.0, weight: 5.0, text: String::new() }];
        let cv = curve(100.0, &heat, &hits, &[]);
        let p = peaks(&cv, 2, 20);
        assert!(p.iter().any(|&x| (60..70).contains(&x)), "{p:?}");
        assert!(p.iter().any(|&x| (28..37).contains(&x)), "{p:?}");
        // the always-hot intro is ignored
        assert!(!p.iter().any(|&x| x < 10));
    }

    #[test]
    fn peaks_keep_their_distance() {
        let cv = vec![0.0, 9.0, 8.0, 7.0, 0.0, 0.0, 6.0];
        assert_eq!(peaks(&cv, 2, 3), vec![1, 6]);
        assert!(peaks(&[], 3, 5).is_empty());
    }
}
