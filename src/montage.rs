//! Music edits, the kind people repost: a video's best shots cut onto the beats of a song.
use crate::face::median;
use crate::fetch::{dims, safe};
use crate::pick::Moment;
use crate::lang::upper;
use crate::render::{ass_time, chunks, grade, shape, Opts};
use crate::transcribe::Word;
use crate::util::{run, run_bytes, yt};
use std::f64::consts::PI;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Clone)]
/// How an edit moves. `sad`: long shots, slow motion, crossfades, black and white. Otherwise: fast cuts with a punch-in.
pub struct Look {
    pub sad: bool,
    /// land every cut with a quick zoom punch (fast edits); off for sad edits and copied references
    pub punch: bool,
    pub length: f64,
    pub tag: String,
    /// a line shown over the whole edit, like "THE PROBLEM"
    pub text: String,
    /// footage speed on top of the look's own (--speed)
    pub pace: f64,
    /// effects on the hardest cuts (--fx)
    pub hits: Vec<String>,
    /// --loop: no fades, the song's bar before the start comes in under the end, so the edit plays round seamlessly
    pub looped: bool,
    /// --corecore: unrelated footage, desaturated and grainy, dips and long dissolves, glimpses between shots
    pub core: bool,
}

impl Look {
    /// seconds a shot should last; rounded to whole beats
    pub fn shot(&self) -> f64 {
        if self.sad { 2.4 } else if self.core { 1.6 } else { 1.1 }
    }
    pub fn speed(&self) -> f64 {
        (if self.sad { 0.8 } else if self.core { 0.9 } else { 1.0 }) * if self.pace > 0.0 { self.pace } else { 1.0 }
    }
}

// ---------------------------------------------------------------- the song

const SR: usize = 22050;
const HOP: usize = 256;
const N: usize = 1024;
const TIGHT: f64 = 100.0; // how hard the beat tracker holds the tempo

pub fn hop_s() -> f64 {
    HOP as f64 / SR as f64
}

pub struct Song {
    pub file: PathBuf,
    pub title: String,
    pub beats: Vec<f64>,
    /// how hard the music hits on each beat
    pub strength: Vec<f64>,
    /// loudness per analysis frame
    pub loud: Vec<f64>,
    /// YouTube "most replayed" per analysis frame, empty when the song has none
    pub heat: Vec<f64>,
    /// where the part people use on TikTok and Shorts starts (seconds), when they agree on one
    pub hook: Option<f64>,
}

impl Song {
    /// What a song's stretch is chosen by: where people replay it most, else where it is loudest.
    pub fn stretch(&self) -> &[f64] {
        if self.heat.is_empty() { &self.loud } else { &self.heat }
    }

    /// The hook counts as the most replayed part for 20 s, so drops inside it win too.
    pub fn set_hook(&mut self, at: f64) {
        self.hook = Some(at);
        if self.heat.is_empty() {
            self.heat = vec![0.0; self.loud.len()];
        }
        let (a, b) = ((at / hop_s()) as usize, (((at + 20.0) / hop_s()) as usize).min(self.heat.len()));
        self.heat.iter_mut().take(b).skip(a).for_each(|h| *h = h.max(1.0));
    }

    /// Starts of `n` stretches of `length` s, none overlapping: the hook first (from its beat, moved back as little as
    /// needed for the stretch to fit before the song ends), then the most replayed or loudest parts.
    pub fn starts(&self, length: f64, n: usize) -> Vec<f64> {
        let end = self.loud.len() as f64 * hop_s();
        let mut out: Vec<f64> = vec![];
        if let Some(h) = self.hook {
            let fit = (end - length).max(0.0);
            if let Some(b) = self.beats.iter().rev().find(|b| **b <= h.min(fit) + 0.15) {
                out.push(*b);
            }
        }
        for w in windows(&self.beats, self.stretch(), length, n + 1) {
            if out.len() >= n {
                break;
            }
            if out.iter().all(|o| (o - w).abs() >= length) {
                out.push(w);
            }
        }
        out.truncate(n.max(1));
        out
    }
}

/// Replay heat bins (start, end, value) spread over `n` analysis frames.
fn heat_frames(heat: &[(f64, f64, f64)], n: usize) -> Vec<f64> {
    if heat.is_empty() {
        return vec![];
    }
    (0..n).map(|i| {
        let t = i as f64 * hop_s();
        heat.iter().find(|h| t >= h.0 && t < h.1).map_or(0.0, |h| h.2)
    }).collect()
}

pub(crate) fn fft(re: &mut [f64], im: &mut [f64]) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = -2.0 * PI / len as f64;
        for i in (0..n).step_by(len) {
            for k in 0..len / 2 {
                let (c, s) = ((ang * k as f64).cos(), (ang * k as f64).sin());
                let (a, b) = (i + k, i + k + len / 2);
                let (tr, ti) = (re[b] * c - im[b] * s, re[b] * s + im[b] * c);
                re[b] = re[a] - tr;
                im[b] = im[a] - ti;
                re[a] += tr;
                im[a] += ti;
            }
        }
        len <<= 1;
    }
}

/// Keeps a flux's attacks and drops the slow swell, in units of its own spread.
fn attacks(env: &[f64]) -> Vec<f64> {
    let w = (0.4 / hop_s()) as usize;
    let mut out = vec![0.0; env.len()];
    for i in 0..env.len() {
        let (a, b) = (i.saturating_sub(w), (i + w + 1).min(env.len()));
        let mean = env[a..b].iter().sum::<f64>() / (b - a) as f64;
        out[i] = (env[i] - mean).max(0.0);
    }
    let sd = (out.iter().map(|v| v * v).sum::<f64>() / out.len().max(1) as f64).sqrt();
    if sd > 0.0 {
        out.iter_mut().for_each(|v| *v /= sd);
    }
    out
}

/// Spectral flux (where notes start, normalised), RMS per frame of `HOP` samples, and the flux under 150 Hz alone: kicks and 808s, where edits hit (hi-hats and cymbals
/// make the full band jump just as much).
pub fn onsets(x: &[f32]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let low = (150.0 * N as f64 / SR as f64).ceil() as usize;
    let win: Vec<f64> = (0..N).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f64 / N as f64).cos()).collect();
    let frames = if x.len() >= N { (x.len() - N) / HOP + 1 } else { 0 };
    let (mut env, mut rms, mut bass) = (Vec::with_capacity(frames), Vec::with_capacity(frames), Vec::with_capacity(frames));
    let mut prev = vec![0.0; N / 2];
    for f in 0..frames {
        let seg = &x[f * HOP..f * HOP + N];
        rms.push((seg.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / N as f64).sqrt());
        let mut re: Vec<f64> = seg.iter().zip(&win).map(|(a, w)| *a as f64 * w).collect();
        let mut im = vec![0.0; N];
        fft(&mut re, &mut im);
        let (mut flux, mut low_flux) = (0.0, 0.0);
        for k in 0..N / 2 {
            let m = (1.0 + 100.0 * re[k].hypot(im[k])).ln();
            let rise = (m - prev[k]).max(0.0);
            flux += rise;
            if k < low {
                low_flux += rise;
            }
            prev[k] = m;
        }
        env.push(if f == 0 { 0.0 } else { flux });
        bass.push(if f == 0 { 0.0 } else { low_flux });
    }
    (attacks(&env), rms, attacks(&bass))
}

/// Beat period in frames: the strongest self-similarity between 50 and 180 BPM, leaning towards ~110.
pub fn tempo(env: &[f64]) -> usize {
    let (lo, hi) = ((60.0 / 180.0 / hop_s()) as usize, (60.0 / 50.0 / hop_s()) as usize);
    let hi = hi.min(env.len().saturating_sub(1));
    let score = |l: usize| {
        let ac = env.iter().zip(&env[l..]).map(|(a, b)| a * b).sum::<f64>() / (env.len() - l) as f64;
        let bpm = 60.0 / (l as f64 * hop_s());
        ac * (-0.5 * (bpm / 110.0).log2().powi(2)).exp()
    };
    (lo..=hi).max_by(|a, b| score(*a).total_cmp(&score(*b))).unwrap_or((0.5 / hop_s()) as usize).max(1)
}

/// Beat frames by dynamic programming (Ellis 2007): on onsets, roughly `lag` apart, free to drift with the band.
pub fn track(env: &[f64], lag: usize) -> Vec<usize> {
    let n = env.len();
    if n == 0 || lag == 0 {
        return vec![];
    }
    let mut score = env.to_vec();
    let mut back = vec![usize::MAX; n];
    for t in (lag / 2 + 1)..n {
        let (lo, hi) = (t.saturating_sub(2 * lag), t - (lag / 2).max(1));
        let mut best = (f64::MIN, usize::MAX);
        for (j, sp) in score[lo..=hi].iter().enumerate() {
            let p = lo + j;
            let s = sp - TIGHT * ((t - p) as f64 / lag as f64).ln().powi(2);
            if s > best.0 {
                best = (s, p);
            }
        }
        if best.1 != usize::MAX {
            score[t] = env[t] + best.0;
            back[t] = best.1;
        }
    }
    let mut t = (n.saturating_sub(lag)..n).max_by(|a, b| score[*a].total_cmp(&score[*b])).unwrap_or(n - 1);
    let mut beats = vec![t];
    while back[t] != usize::MAX {
        t = back[t];
        beats.push(t);
    }
    beats.reverse();
    beats
}

/// The strongest attack within `r` frames of `f` (ties: the nearest, then the earlier).
fn peak_near(env: &[f64], f: usize, r: usize) -> usize {
    let (lo, hi) = (f.saturating_sub(r), (f + r).min(env.len().saturating_sub(1)));
    (lo..=hi).min_by(|a, b| env[*b].total_cmp(&env[*a]).then(a.abs_diff(f).cmp(&b.abs_diff(f))).then(a.cmp(b))).unwrap_or(f)
}

/// Tracked beats tidied the way beatscope does it: a doubled beat dropped (the weaker of two closer than 55% of a beat),
/// a single missed beat put back (a gap of 1.65-2.35 beats), then each nudged onto its attack (within 70 ms or 15% of
/// a beat, never breaking order or tempo; after the repair, so a gap no longer blocks its neighbours). Cuts then land on the hit itself, not a few frames off it.
pub fn refine(beats: &[usize], env: &[f64], lag: usize) -> Vec<usize> {
    if beats.is_empty() || lag == 0 || env.is_empty() {
        return beats.to_vec();
    }
    let p = lag as f64;
    let sane = |gap: usize| (0.67..=1.5).contains(&(gap as f64 / p));
    let radius = |k: f64| ((0.07 / hop_s()).min(k * p).round() as usize).max(1);
    let support = |f: usize| env[f.saturating_sub(2)..(f + 3).min(env.len())].iter().sum::<f64>();
    let mut kept: Vec<usize> = vec![];
    for &f in beats {
        match kept.last_mut() {
            Some(l) if ((f - *l) as f64) < 0.55 * p => {
                if support(f) > support(*l) {
                    *l = f;
                }
            }
            _ => kept.push(f),
        }
    }
    let mut out: Vec<usize> = vec![];
    for f in kept {
        if let Some(&l) = out.last() {
            let gap = (f - l) as f64 / p;
            if gap > 1.65 && gap < 2.35 {
                let t = peak_near(env, l + lag, radius(0.10));
                if l < t && t < f && sane(t - l) && sane(f - t) {
                    out.push(t);
                }
            }
        }
        out.push(f);
    }
    let mut snapped: Vec<usize> = vec![];
    for (i, &f) in out.iter().enumerate() {
        let c = peak_near(env, f, radius(0.15));
        let next = out.get(i + 1).copied().unwrap_or(env.len());
        let ok = c == f || (snapped.last().is_none_or(|&l| l < c && sane(c - l)) && c < next && (i + 1 == out.len() || sane(next - c)));
        snapped.push(if ok { c } else { f });
    }
    snapped
}

/// Downloads (or takes) the song and finds its beats, and where the part people use starts. A TikTok, Reel or Short
/// given as the song becomes its record, whole and clean, from where the clip starts (a sped-up sound stays as it is).
pub fn song(src: &str, cache: &Path) -> Result<Song, String> {
    let mut s = beats_of(src, cache)?;
    if Path::new(src).is_file() {
        return Ok(s);
    }
    if (s.loud.len() as f64 * hop_s()) < 75.0 {
        if let Ok(Some(f)) = crate::shazam::recognize(&s.file, None) {
            let name = format!("{} - {}", f.artist, f.title);
            if f.skew < 0.02 && !f.title.is_empty() {
                match crate::fetch::find_song(&name).and_then(|u| beats_of(&u, cache)) {
                    Ok(mut whole) => {
                        eprintln!("edits: the sound is {name}: the record from {}, where the clip starts", crate::util::mmss(f.offset.max(0.0)));
                        whole.set_hook(f.offset.max(0.0));
                        return Ok(whole);
                    }
                    Err(e) => eprintln!("edits: the sound is {name}, but its record was not found ({e}); the clip's own sound is used"),
                }
            } else if f.skew >= 0.02 {
                eprintln!("edits: the sound is a sped-up {name}; kept as it is");
            }
        }
        return Ok(s);
    }
    if let Some(h) = crate::hook::find(&s.title, cache) {
        s.set_hook(h);
    }
    Ok(s)
}

fn beats_of(src: &str, cache: &Path) -> Result<Song, String> {
    let (file, title, heat) = if Path::new(src).is_file() {
        let title = Path::new(src).file_stem().unwrap_or_default().to_string_lossy().into_owned();
        // an edit (it has a picture) brings its own dialogue: keep only its music
        let file = if dims(Path::new(src)).0 > 0 { music_only(Path::new(src), cache)? } else { PathBuf::from(src) };
        (file, title, vec![])
    } else {
        let dir = cache.join("music");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let out = yt(&[
            "--no-warnings", "--no-progress", "--no-playlist", "-f", "ba[ext=m4a]/ba",
            "--print", "after_move:%(title)s\t%(filepath)s\t%(heatmap)j", "-o", &dir.join("%(id)s.%(ext)s").to_string_lossy(), "--", src,
        ])?;
        let line = out.lines().rfind(|l| l.contains('\t')).ok_or("yt-dlp gave no song file")?;
        let mut parts = line.splitn(3, '\t');
        let (t, f) = (parts.next().unwrap_or_default(), parts.next().unwrap_or_default());
        let heat: Vec<(f64, f64, f64)> = serde_json::from_str::<serde_json::Value>(parts.next().unwrap_or("null")).ok()
            .and_then(|v| v.as_array().map(|a| a.iter().filter_map(|h| Some((h["start_time"].as_f64()?, h["end_time"].as_f64()?, h["value"].as_f64()?))).collect()))
            .unwrap_or_default();
        (PathBuf::from(f.trim()), t.trim().to_string(), heat)
    };
    let raw = run_bytes("ffmpeg", &["-v", "error", "-i", &file.to_string_lossy(), "-ac", "1", "-ar", &SR.to_string(), "-f", "f32le", "-"])?;
    let x: Vec<f32> = raw.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect();
    let (env, loud, bass) = onsets(&x);
    let lag = tempo(&env);
    let frames = refine(&track(&env, lag), &env, lag);
    if frames.len() < 4 {
        return Err(format!("could not find a beat in {title}"));
    }
    // how hard each beat hits: half its attack, half its kick (so the fx land on the bass, not on a cymbal)
    let peak = |e: &[f64], f: usize| e[f.saturating_sub(2)..(f + 3).min(e.len())].iter().cloned().fold(0.0, f64::max);
    let strength = frames.iter().map(|&f| 0.5 * peak(&env, f) + 0.5 * peak(&bass, f)).collect();
    let beats = frames.into_iter().map(|f| f as f64 * hop_s()).collect();
    let heat = heat_frames(&heat, loud.len());
    Ok(Song { file, title, beats, strength, loud, heat, hook: None })
}

/// Strength of the beat nearest to `t` seconds.
pub fn hit_at(beats: &[f64], strength: &[f64], t: f64) -> f64 {
    if beats.is_empty() {
        return 0.0;
    }
    let i = beats.partition_point(|b| *b < t);
    [i.saturating_sub(1), i.min(beats.len().saturating_sub(1))].iter()
        .min_by(|a, b| (beats[**a] - t).abs().total_cmp(&(beats[**b] - t).abs()))
        .and_then(|&k| strength.get(k).copied()).unwrap_or(0.0)
}

/// The beat where the music should hit as the voice ends: the one after which it gets loudest relative to before,
/// with `before` s of song ahead of it (under the voice) and `after` s behind it (the montage). With a replay
/// `heat` (per analysis frame, empty for none) the drop people come back for wins over a louder one they skip.
pub fn drop_at(beats: &[f64], strength: &[f64], loud: &[f64], heat: &[f64], before: f64, after: f64) -> Option<f64> {
    let end = loud.len() as f64 * hop_s();
    let mean = |a: f64, b: f64| {
        let (i, j) = ((a / hop_s()).max(0.0) as usize, ((b / hop_s()) as usize).min(loud.len()));
        if j > i { loud[i..j].iter().sum::<f64>() / (j - i) as f64 } else { 0.0 }
    };
    beats.iter().enumerate().filter(|(_, b)| **b >= before && **b + after <= end).map(|(i, &b)| {
        let lift = mean(b, b + after.min(6.0)) / (mean(b - before.min(6.0), b) + 1e-3);
        let hot = heat.get((b / hop_s()) as usize).copied().unwrap_or(0.0);
        (lift * (1.0 + hot) + 0.1 * strength.get(i).copied().unwrap_or(0.0), b)
    }).max_by(|a, b| a.0.total_cmp(&b.0)).map(|x| x.1)
}

/// The typical gap between beats.
pub fn beat_period(beats: &[f64]) -> f64 {
    let mut gaps: Vec<f64> = beats.windows(2).map(|w| w[1] - w[0]).collect();
    median(&mut gaps).unwrap_or(0.5)
}

/// Starts of the `n` loudest stretches of `length` seconds, each on a beat, none overlapping.
pub fn windows(beats: &[f64], loud: &[f64], length: f64, n: usize) -> Vec<f64> {
    let end = loud.len() as f64 * hop_s();
    let mean = |s: f64| {
        let (a, b) = ((s / hop_s()) as usize, (((s + length) / hop_s()) as usize).min(loud.len()));
        loud[a..b.max(a + 1).min(loud.len())].iter().sum::<f64>() / (b.saturating_sub(a)).max(1) as f64
    };
    let mut cands: Vec<(f64, f64)> = beats.iter().filter(|b| **b + length <= end).map(|b| (mean(*b), *b)).collect();
    cands.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut out: Vec<f64> = vec![];
    for (_, s) in cands {
        if out.len() == n {
            break;
        }
        if out.iter().all(|o| (o - s).abs() >= length) {
            out.push(s);
        }
    }
    if out.is_empty() {
        out.push(beats.first().copied().unwrap_or(0.0));
    }
    out
}

/// Cut points from 0 to the end of the edit, relative to `start`: every k-th beat, k chosen so shots last ~`shot` s.
/// With the song's loudness (`loud`, per analysis frame; empty for none) the pace follows the music as
/// BeatSync-Engine does: twice as fast where it is much louder than the edit's average, half as fast where it breathes.
pub fn cuts(beats: &[f64], loud: &[f64], start: f64, length: f64, shot: f64) -> Vec<f64> {
    let inside: Vec<f64> = beats.iter().filter(|b| **b >= start - 1e-6 && **b <= start + length + 1e-6).map(|b| b - start).collect();
    if inside.len() < 2 {
        return vec![0.0, length];
    }
    let mut gaps: Vec<f64> = inside.windows(2).map(|w| w[1] - w[0]).collect();
    let period = median(&mut gaps).unwrap_or(0.5);
    let k = ((shot / period).round() as usize).max(1);
    let level = |a: f64, b: f64| {
        let (i, j) = (((start + a) / hop_s()) as usize, (((start + b) / hop_s()) as usize).min(loud.len()));
        if j > i { loud[i..j].iter().sum::<f64>() / (j - i) as f64 } else { 0.0 }
    };
    let whole = level(0.0, length);
    let mut out: Vec<f64> = vec![];
    let mut i = 0;
    while i < inside.len() {
        out.push(inside[i]);
        let ahead = inside[(i + k).min(inside.len() - 1)];
        let e = if whole > 0.0 { level(inside[i], ahead.max(inside[i] + period)) / whole } else { 1.0 };
        i += if e > 1.3 { (k / 2).max(1) } else if e < 0.7 { k * 2 } else { k };
    }
    if out[0] > 1e-6 {
        out.insert(0, 0.0);
    }
    if out.len() < 2 {
        out.push(length);
    }
    out
}

// ---------------------------------------------------------------- the footage

#[derive(Clone, Debug)]
pub struct Shot {
    pub video: usize,
    pub s: f64,
    pub e: f64,
    /// where the scene this piece belongs to ends: a long piece of an edit can run on into it
    pub scene_end: f64,
    pub score: f64,
    pub focus: Option<f64>,
}

#[derive(Clone, Copy, Default, serde::Serialize, serde::Deserialize)]
pub struct Frame {
    /// mean change from the previous frame, 0..1
    pub diff: f64,
    pub luma: f64,
    pub face: Option<f64>,
    pub checked: bool,
    /// how far from gray the picture is: drawings and photos score high, text slides low
    pub color: f64,
    /// how much is drawn: near zero for an empty background, a blank slide, a flat wall
    pub detail: f64,
}

const FPS: f64 = 4.0;
const AW: u32 = 320;

/// Measures every frame once per video: the result is kept next to it (scanning a long video takes minutes).
pub fn frames(video: &Path) -> Result<Vec<Frame>, String> {
    let keep = video.with_extension("frames.json");
    let fresh = |p: &Path| p.metadata().and_then(|m| m.modified()).ok();
    if fresh(&keep) >= fresh(video) && fresh(&keep).is_some() {
        if let Some(fs) = std::fs::read_to_string(&keep).ok().and_then(|s| serde_json::from_str::<Vec<Frame>>(&s).ok()) {
            return Ok(fs);
        }
    }
    let fs = scan(video)?;
    // a file the user gave may sit somewhere read-only: then it is simply scanned again next time
    let _ = std::fs::write(&keep, serde_json::to_string(&fs).unwrap_or_default());
    Ok(fs)
}

/// Streams frames out of ffmpeg and measures each one.
fn scan(video: &Path) -> Result<Vec<Frame>, String> {
    let (sw, sh) = dims(video);
    if sw == 0 || sh == 0 {
        return Err(format!("{}: no video stream", video.display()));
    }
    let h = (AW * sh / sw / 2 * 2).max(2);
    let mut child = Command::new("ffmpeg")
        .args(["-v", "error", "-i", &video.to_string_lossy(), "-an", "-vf", &format!("fps={FPS},scale={AW}:{h},format=yuv420p"), "-f", "rawvideo", "-"])
        .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().map_err(|e| format!("ffmpeg: {e}"))?;
    let mut out = child.stdout.take().ok_or("no ffmpeg output")?;
    let model = rustface::read_model(crate::face::MODEL).map_err(|e| e.to_string())?;
    let mut det = rustface::create_detector_with_model(model);
    det.set_min_face_size(20);
    det.set_score_thresh(2.0);
    det.set_pyramid_scale_factor(0.8);
    det.set_slide_window_step(4, 4);
    let size = (AW * h) as usize;
    let (mut cur, mut prev) = (vec![0u8; size], vec![0u8; size]);
    let mut chroma = vec![0u8; size / 2];
    let mut fs: Vec<Frame> = vec![];
    while out.read_exact(&mut cur).is_ok() && out.read_exact(&mut chroma).is_ok() {
        let mut f = Frame { luma: cur.iter().step_by(7).map(|v| *v as f64).sum::<f64>() / (size / 7).max(1) as f64, ..Default::default() };
        f.color = chroma.iter().step_by(5).map(|v| (*v as f64 - 128.0).abs()).sum::<f64>() / (chroma.len() / 5).max(1) as f64;
        f.detail = detail(&cur, AW as usize);
        if !fs.is_empty() {
            f.diff = cur.iter().zip(&prev).step_by(3).map(|(a, b)| a.abs_diff(*b) as f64).sum::<f64>() / ((size / 3).max(1) as f64 * 255.0);
        }
        // faces twice a second is enough to frame a shot
        if fs.len().is_multiple_of(2) {
            f.checked = true;
            f.face = det.detect(&rustface::ImageData::new(&cur, AW, h)).into_iter().max_by_key(|x| x.bbox().width())
                .map(|x| (x.bbox().x() as f64 + x.bbox().width() as f64 / 2.0) / AW as f64);
        }
        fs.push(f);
        std::mem::swap(&mut cur, &mut prev);
    }
    let _ = child.wait();
    Ok(fs)
}

/// Below this a frame counts as empty (see `detail`).
const EMPTY: f64 = 2.5;

/// Mean brightness step between neighbouring pixels (sampled): how much is drawn on a gray frame of `w` columns.
pub fn detail(y: &[u8], w: usize) -> f64 {
    if w < 3 || y.len() < w * 3 {
        return 0.0;
    }
    let mut sum = 0u64;
    let mut n = 0u64;
    for r in (1..y.len() / w - 1).step_by(3) {
        for c in (1..w - 1).step_by(3) {
            let i = r * w + c;
            sum += y[i].abs_diff(y[i + 1]) as u64 + y[i].abs_diff(y[i + w]) as u64;
            n += 1;
        }
    }
    sum as f64 / n.max(1) as f64
}

/// Frames -> shots of at most 3 s, scored: faces first, then movement; black and blown-out frames lose.
pub fn segment(fs: &[Frame], video: usize) -> Vec<Shot> {
    let n = fs.len();
    let skip = n * 3 / 100; // intros and end screens
    let mut bounds = vec![0];
    bounds.extend((1..n).filter(|&i| fs[i].diff > 0.12));
    bounds.push(n);
    let (chunk, min) = ((3.0 * FPS) as usize, (0.75 * FPS) as usize);
    let mut out = vec![];
    for w in bounds.windows(2) {
        let (mut a, b) = (w[0], w[1]);
        while b - a >= min {
            let e = if b - a < chunk + min { b } else { a + chunk };
            if a >= skip && e + skip <= n {
                let f = &fs[a..e];
                let motion = f.iter().skip(1).map(|x| x.diff).sum::<f64>() / (f.len() - 1).max(1) as f64;
                let luma = f.iter().map(|x| x.luma).sum::<f64>() / f.len() as f64;
                let mut xs: Vec<f64> = f.iter().filter_map(|x| x.face).collect();
                let checked = f.iter().filter(|x| x.checked).count().max(1);
                let faces = xs.len() as f64 / checked as f64;
                let color = f.iter().map(|x| x.color).sum::<f64>() / f.len() as f64;
                let drawn = f.iter().map(|x| x.detail).sum::<f64>() / f.len() as f64;
                // an empty frame (background, blank slide) is never worth a beat
                let empty = if drawn < EMPTY { 3.0 } else { 0.0 };
                let score = 2.0 * faces + (motion * 25.0).min(1.0) + (color / 12.0).min(1.5) + (drawn / 12.0).min(0.8) - empty
                    - if luma < 22.0 { 2.0 } else { 0.0 } - if luma > 235.0 { 1.0 } else { 0.0 };
                out.push(Shot { video, s: a as f64 / FPS, e: e as f64 / FPS, scene_end: (b.min(n - skip)) as f64 / FPS, score, focus: median(&mut xs) });
            }
            a = e;
        }
    }
    out
}

/// One shot per piece, each at least `need` seconds of source: the best unused ones, the very best first
/// as the hook, the rest in story order. Returns (shot, source start) per piece.
/// `prefer` (video, from, to): footage shown while the voice was said, so picture and words belong together.
pub fn assign(shots: &[Shot], used: &mut [bool], needs: &[f64], prefer: Option<(usize, f64, f64)>) -> Vec<(usize, f64)> {
    // long enough for most slots: one long slot must not rule out every short scene (the long ones are matched below)
    let mut sorted = needs.to_vec();
    sorted.sort_by(|a, b| a.total_cmp(b));
    let need = sorted.get(sorted.len() * 3 / 4).copied().unwrap_or(0.0);
    let near = |i: usize| prefer.is_some_and(|(v, a, b)| shots[i].video == v && shots[i].e > a && shots[i].s < b);
    let score = |i: usize| shots[i].score + if near(i) { 3.0 } else { 0.0 };
    let mut order: Vec<usize> = (0..shots.len()).filter(|&i| shots[i].scene_end.max(shots[i].e) - shots[i].s >= need).collect();
    order.sort_by(|a, b| score(*b).total_cmp(&score(*a)));
    if order.iter().filter(|&&i| !used[i]).count() < needs.len() {
        used.iter_mut().for_each(|u| *u = false); // footage ran out: reuse rather than make a short edit
    }
    // pieces of the same scene back to back read as one long shot: keep picks 12 s apart
    // and spread over every video: each pick from a video costs the next one from it a point
    let free: Vec<usize> = order.into_iter().filter(|&i| !used[i]).collect();
    // with little footage (one video) the gap gives way rather than leave the edit unmade
    let mut pick: Vec<usize> = vec![];
    for gap in [12.0, 6.0, 2.0, 0.0] {
        while pick.len() < needs.len() {
            let from = |v: usize| pick.iter().filter(|&&p| shots[p].video == v).count() as f64;
            let best = free.iter().copied()
                .filter(|i| !pick.contains(i) && pick.iter().all(|&p| shots[p].video != shots[*i].video || (shots[p].s - shots[*i].s).abs() >= gap))
                .max_by(|a, b| (score(*a) - from(shots[*a].video)).total_cmp(&(score(*b) - from(shots[*b].video))));
            match best {
                Some(i) => pick.push(i),
                None => break,
            }
        }
    }
    // still short (one short video, a long preset): the best shots again, each time from another part of them
    let base = pick.clone();
    for k in 0.. {
        if pick.len() >= needs.len() || base.is_empty() {
            break;
        }
        pick.push(base[k % base.len()]);
    }
    pick.iter().for_each(|&i| used[i] = true);
    if pick.len() > 1 {
        // after the hook, videos take turns, each in its own story order
        let hook = pick[0];
        let mut queues: Vec<Vec<usize>> = vec![];
        let mut rest: Vec<usize> = pick[1..].to_vec();
        rest.sort_by(|a, b| shots[*a].video.cmp(&shots[*b].video).then(shots[*a].s.total_cmp(&shots[*b].s)));
        for i in rest {
            match queues.iter_mut().find(|q| shots[q[0]].video == shots[i].video) {
                Some(q) => q.push(i),
                None => queues.push(vec![i]),
            }
        }
        // the hook's own video waits its turn
        queues.sort_by_key(|q| shots[q[0]].video == shots[hook].video);
        pick = vec![hook];
        let mut k = 0;
        while queues.iter().any(|q| !q.is_empty()) {
            let n = queues.len();
            if !queues[k % n].is_empty() {
                let i = queues[k % n].remove(0);
                // never the same video twice running when another can go
                if pick.last().is_some_and(|&l| shots[l].video == shots[i].video) && queues.iter().any(|q| !q.is_empty() && shots[q[0]].video != shots[i].video) {
                    queues[k % n].insert(0, i);
                } else {
                    pick.push(i);
                }
            }
            k += 1;
        }
    }
    // centred in its piece when it fits, else from the piece's start on into the rest of the scene
    // a slot longer than its shot swaps with one whose shot is long enough, when that one fits the other way too
    let avail = |i: usize| shots[i].scene_end.max(shots[i].e) - shots[i].s;
    for j in 0..pick.len().min(needs.len()) {
        if avail(pick[j]) < needs[j] {
            if let Some(k) = (0..pick.len().min(needs.len())).find(|&k| k != j && avail(pick[k]) >= needs[j] && avail(pick[j]) >= needs[k]) {
                pick.swap(j, k);
            }
        }
    }
    let mut seen: Vec<usize> = vec![];
    pick.iter().zip(needs).map(|(&i, n)| {
        let again = seen.iter().filter(|&&x| x == i).count();
        seen.push(i);
        let room = (shots[i].scene_end.max(shots[i].e) - shots[i].s - n).max(0.0);
        let from = if again > 0 && room > 0.0 {
            shots[i].s + (again as f64 * n) % room // a shot used again starts elsewhere in its scene
        } else if shots[i].e - shots[i].s >= *n {
            (shots[i].s + shots[i].e - n) / 2.0
        } else {
            shots[i].s
        };
        (i, from)
    }).collect()
}

// ---------------------------------------------------------------- the edit

/// How the picture changes at a cut: an ffmpeg xfade transition and its length. A one-frame fade is a hard cut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Join {
    pub kind: &'static str,
    pub dur: f64,
    /// a camera move around this cut, by index into `presets::motions()`
    pub motion: Option<usize>,
}

/// Every transition ffmpeg's xfade draws (Premiere's video transitions).
pub const XFADES: &[&str] = &["fade", "fadeblack", "fadewhite", "fadegrays", "dissolve", "pixelize", "radial", "circleopen", "circleclose", "circlecrop",
    "rectcrop", "distance", "wipeleft", "wiperight", "wipeup", "wipedown", "wipetl", "wipetr", "wipebl", "wipebr", "slideleft", "slideright", "slideup",
    "slidedown", "smoothleft", "smoothright", "smoothup", "smoothdown", "horzopen", "horzclose", "vertopen", "vertclose", "diagtl", "diagtr", "diagbl",
    "diagbr", "hlslice", "hrslice", "vuslice", "vdslice", "hblur", "squeezeh", "squeezev", "zoomin", "coverleft", "coverright", "coverup", "coverdown",
    "revealleft", "revealright", "revealup", "revealdown"];

pub const CUT: Join = Join { kind: "fade", dur: 0.034, motion: None };

/// One join per cut point, from how hard the music hits there (`hits`, same length as `cuts`).
/// As in the reference edits: hard cuts on the beat, a white flash or a zoom on the biggest hits;
/// sad edits dissolve, and dip to black on the biggest.
pub fn joins(hits: &[f64], sad: bool) -> Vec<Join> {
    let n = hits.len();
    let mut out = vec![CUT; n];
    if n < 3 {
        return out;
    }
    let mut inner: Vec<usize> = (1..n - 1).collect();
    inner.sort_by(|a, b| hits[*b].total_cmp(&hits[*a]));
    let (strong, medium) = ((inner.len() as f64 * 0.15).ceil() as usize, (inner.len() as f64 * 0.35).ceil() as usize);
    for (rank, &i) in inner.iter().enumerate() {
        out[i] = match (sad, rank < strong, rank < medium) {
            (false, true, _) => Join { kind: "fadewhite", dur: 0.3, motion: None },
            (false, _, true) => Join { kind: "zoomin", dur: 0.2, motion: None },
            (false, _, _) => CUT,
            (true, true, _) => Join { kind: "fadeblack", dur: 0.7, motion: None },
            (true, _, true) => Join { kind: "dissolve", dur: 0.6, motion: None },
            (true, _, _) => Join { kind: "fade", dur: 0.45, motion: None },
        };
    }
    out
}

/// --corecore: glimpses (a flicker of three shots) every fourth shot that is long enough, the first 0.12 s of it
/// split into two quick pieces of other footage.
pub fn glimpses(cuts: &[f64]) -> Vec<f64> {
    let mut out = vec![];
    for (i, w) in cuts.windows(2).enumerate() {
        out.push(w[0]);
        if i % 4 == 2 && w[1] - w[0] >= 1.0 {
            out.extend([w[0] + 0.12, w[0] + 0.24]);
        }
    }
    out.extend(cuts.last());
    out
}

/// --corecore joins: hard cuts into and out of glimpses, otherwise dips to black and long dissolves between pieces
/// long enough to carry them, cuts between.
pub fn joins_core(cuts: &[f64]) -> Vec<Join> {
    let n = cuts.len();
    let len = |i: usize| if i + 1 < n { cuts[i + 1] - cuts[i] } else { 0.0 };
    (0..n).map(|i| {
        if i == 0 || i + 1 >= n || len(i - 1) < 1.2 || len(i) < 1.2 {
            CUT
        } else if i % 5 == 0 {
            Join { kind: "dissolve", dur: 0.8, motion: None }
        } else if i % 3 == 0 {
            Join { kind: "fadeblack", dur: 0.5, motion: None }
        } else {
            CUT
        }
    }).collect()
}

/// Puts Premiere camera moves on the cuts the music hits: the hardest hits (white flashes) get the biggest moves,
/// the next ones (zooms) a move instead of the zoom. `k` varies the choice between edits.
pub fn add_moves(joins: &mut [Join], k: usize) {
    let all = crate::presets::motions();
    if all.is_empty() {
        return;
    }
    let mut order: Vec<usize> = (0..all.len()).collect();
    order.sort_by(|a, b| all[*b].strength().total_cmp(&all[*a].strength()));
    let (big, small) = order.split_at(order.len().div_ceil(2));
    let n = joins.len();
    for (i, j) in joins.iter_mut().enumerate().skip(1).take(n.saturating_sub(2)) {
        let pool = match j.kind {
            "fadewhite" => big,
            "zoomin" => small,
            _ => continue,
        };
        if pool.is_empty() {
            continue;
        }
        j.motion = Some(pool[(k * 7 + i * 3) % pool.len()]);
        if j.kind == "zoomin" {
            *j = Join { motion: j.motion, ..CUT };
        }
    }
}

/// (start, length) on the output timeline for each piece between `cuts`; neighbours overlap by their join,
/// centred on the cut so the change lands on the beat.
pub fn pieces(cuts: &[f64], joins: &[Join]) -> Vec<(f64, f64)> {
    let m = cuts.len() - 1;
    (0..m).map(|i| {
        let s = if i == 0 { 0.0 } else { cuts[i] - joins[i].dur / 2.0 };
        let e = if i == m - 1 { cuts[m] } else { cuts[i + 1] + joins[i + 1].dur / 2.0 };
        (s, e - s)
    }).collect()
}

/// The person speaking over the edit: their sound from `start` for `len` s.
pub struct Voice {
    pub file: PathBuf,
    pub start: f64,
    pub len: f64,
    /// where in the edit the voice comes in: on a beat or a cut
    pub at: f64,
}

/// Subtitles for an edit as ASS: the spoken words and the `--text` line.
/// Fast edits: one word at a time, upper case, popping in (like the "EDUCATION" reference).
/// Sad edits: two or three words, as spoken, fading. The text line: heavy condensed capitals, outlined.
/// `offset` is where the voice starts in the edit.
#[allow(clippy::too_many_arguments)]
pub fn captions(words: &[Word], m: Option<&Moment>, offset: f64, text: &str, sad: bool, total: f64, o: &Opts) -> String {
    let (w, h) = (o.w, o.h);
    let (size, margin) = if sad { (w * 52 / 1000, h * 17 / 100) } else { (w * 85 / 1000, h * 20 / 100) };
    let mut s = format!(
        "[Script Info]\nScriptType: v4.00+\nPlayResX: {w}\nPlayResY: {h}\nWrapStyle: 0\nScaledBorderAndShadow: yes\n\n\
         [V4+ Styles]\nFormat: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
         Style: W,Adwaita Sans Black,{size},&H00FFFFFF,&H00FFFFFF,&H00000000,&H99000000,-1,0,0,0,100,100,0,0,1,{},4,2,60,60,{margin},1\n\
         Style: T,Nimbus Sans Narrow,{},&H00FFFFFF,&H00FFFFFF,&H00000000,&H00000000,-1,0,0,0,100,100,0,0,1,6,0,2,40,40,{},1\n\n\
         [Events]\nFormat: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n",
        if sad { 1 } else { 2 }, w * 11 / 100, if words.is_empty() { h * 40 / 100 } else { h * 34 / 100 }
    );
    let clean = |t: &str| t.chars().filter(|c| !matches!(c, '{' | '}' | '\\')).collect::<String>();
    if !text.trim().is_empty() {
        s.push_str(&format!("Dialogue: 1,{},{},T,,0,0,0,,{{\\fad(250,400)}}{}\n", ass_time(0.3), ass_time(total - 0.8), upper(&clean(text.trim()))));
    }
    if let Some(m) = m {
        if sad {
            for (a, b, t, _) in chunks(words, m) {
                s.push_str(&format!("Dialogue: 0,{},{},W,,0,0,0,,{{\\fad(140,140)}}{}\n", ass_time(a + offset), ass_time(b + offset), clean(&t)));
            }
        } else {
            let ws: Vec<&Word> = words.iter().filter(|x| x.s >= m.start - 0.05 && x.e <= m.end + 0.05).collect();
            for (i, x) in ws.iter().enumerate() {
                let a = x.s - m.start + offset;
                let b = ws.get(i + 1).map_or(x.e - m.start + 0.4, |n| (n.s - m.start).min(x.e - m.start + 0.6)).max(x.s - m.start + 0.12) + offset;
                let t = clean(x.t.trim_matches(|c: char| ",.;:!?\"'".contains(c)));
                s.push_str(&format!("Dialogue: 0,{},{},W,,0,0,0,,{{\\fscx125\\fscy125\\t(0,90,\\fscx100\\fscy100)}}{}\n", ass_time(a), ass_time(b), upper(&t)));
            }
        }
    }
    s
}

#[allow(clippy::too_many_arguments)]
/// `beats` are the music's beats on the edit's timeline.
/// `pre`: with --loop, seconds of song from before the start, its own last input, crossfaded in under the end.
pub fn filter_graph(o: &Opts, look: &Look, focus: &[f64], spans: &[(f64, f64)], joins: &[Join], beats: &[f64], total: f64, voice: Option<&Voice>, subs: bool, pre: Option<f64>) -> String {
    let (w, h, speed) = (o.w, o.h, look.speed());
    let mut g = String::new();
    for (i, (f, (_, len))) in focus.iter().zip(spans).enumerate() {
        // every shot drifts in; fast edits also land each cut with a quick punch and a little motion smear
        // every shot drifts in and kicks on each beat that falls inside it; the first kick is the cut itself
        let kick = if look.punch { 0.08 } else if look.sad { 0.025 } else if look.core { 0.015 } else { 0.05 };
        let (s0, _) = spans[i];
        let pulses: String = beats.iter().map(|b| b - s0).filter(|t| *t >= -0.01 && *t < *len).take(40)
            .map(|t| format!("+{kick}*max(0,1-(it-{:.3})*6)*gte(it,{:.3})", t.max(0.0), t.max(0.0))).collect();
        // Premiere moves around the cuts at either end of this piece, on the piece's own clock
        let moves = crate::presets::motions();
        let incoming = (i > 0).then(|| joins[i].motion.and_then(|k| moves.get(k))).flatten().map(|m| (m, format!("+{:.3}", m.cut - joins[i].dur / 2.0)));
        let outgoing = (i + 1 < spans.len()).then(|| joins[i + 1].motion.and_then(|k| moves.get(k))).flatten()
            .map(|m| (m, format!("-{:.3}", len - joins[i + 1].dur / 2.0 - m.cut)));
        let ends: Vec<_> = incoming.iter().chain(outgoing.iter()).collect();
        let term = |pick: &dyn Fn(&crate::preset::Motion) -> &crate::preset::Curve, var: &str, op: &str| -> String {
            ends.iter().filter(|(m, _)| !pick(m).keys.is_empty()).map(|(m, shift)| format!("{op}({})", pick(m).expr(&format!("({var}{shift})"), m.dur))).collect()
        };
        let zoom = format!("max(1,(min(1+0.0012*on,1.12){pulses}){})", term(&|m| &m.zoom, "it", "*"));
        let (mx, my, mr) = (term(&|m| &m.x, "t", "+"), term(&|m| &m.y, "t", "+"), term(&|m| &m.rot, "t", "+"));
        let mut smear = if look.punch { ",tmix=frames=2".to_string() } else { String::new() };
        if !(mx.is_empty() && my.is_empty() && mr.is_empty()) {
            let or0 = |e: &String| if e.is_empty() { "0".to_string() } else { format!("0{e}") };
            let reach: f64 = ends.iter().map(|(m, _)| crate::preset::reach(&[&m.x, &m.y])).sum();
            smear.push_str(&crate::preset::moves(w, h, &or0(&mx), &or0(&my), if mr.is_empty() { "" } else { &mr[1..] }, reach));
        }
        // ffmpeg's max takes two arguments: fold them
        let blur = ends.iter().filter(|(m, _)| !m.blur.keys.is_empty())
            .fold(String::new(), |acc, (m, shift)| {
                let e = m.blur.expr(&format!("(t{shift})"), m.dur);
                if acc.is_empty() { e } else { format!("max({acc},{e})") }
            });
        if !blur.is_empty() {
            smear.push_str(&format!(",gblur=sigma=12:enable='gt({blur},0.25)'"));
        }
        let mut tone = if look.sad {
            ",hue=s=0,eq=contrast=1.22:brightness=-0.05,vignette=PI/4.2,noise=alls=5:allf=t".to_string()
        } else if look.core {
            // found footage: colour mostly gone, crushed a little, heavy grain
            format!("{},hue=s=0.3,eq=contrast=1.12:brightness=-0.04:gamma=0.95,vignette=PI/4,noise=alls=16:allf=t", grade(&o.grade))
        } else {
            format!("{},eq=contrast=1.08,vignette=PI/5", grade(&o.grade))
        };
        // the hardest cuts land with the chosen hits (--fx)
        if i > 0 && joins[i].kind == "fadewhite" {
            tone.push_str(&crate::fx::hit(&look.hits, w, h));
        }
        // each piece runs on 0.2 s past its length (its last frame held): xfade ends the whole picture when the piece
        // before a transition stops even a frame short of the transition's end, and the spare is never shown
        g.push_str(&format!(
            "[{i}:v]setpts=(PTS-STARTPTS)/{speed},fps=30,{},zoompan=z='{zoom}':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=1:s={w}x{h}:fps=30{smear}{tone},setsar=1,trim=duration={len:.3},setpts=PTS-STARTPTS,tpad=stop_mode=clone:stop_duration=0.2,settb=AVTB[v{i}];",
            shape(&Opts { layout: "fill".into(), ..o.clone() }, *f)
        ));
    }
    let mut last = "v0".to_string();
    for (i, (s, _)) in spans.iter().enumerate().skip(1) {
        let j = joins[i];
        g.push_str(&format!("[{last}][v{i}]xfade=transition={}:duration={:.3}:offset={s:.3}[x{i}];", j.kind, j.dur));
        last = format!("x{i}");
    }
    // one clean 30 fps clock after the transitions, or the encoder drops frames it takes for duplicates
    g.push_str(&format!("[{last}]settb=1/30,fps=30{}", color_hits(spans, joins, voice.is_some() && !look.sad)));
    if !look.looped {
        g.push_str(&format!(",fade=t=in:d=0.4,fade=t=out:st={:.3}:d=1.2", (total - 1.2).max(0.0)));
    }
    if subs {
        g.push_str(",ass=cap.ass");
    }
    if !look.tag.is_empty() {
        g.push_str(&format!(",drawtext=text='{}':font=Adwaita Sans:fontsize=h*0.024:fontcolor=white@0.7:x=(w-tw)/2:y=h*0.93", look.tag));
    }
    let m = spans.len();
    g.push_str(",format=yuv420p[vout];");
    match (look.looped, pre) {
        (true, Some(d)) => g.push_str(&splice(m, m + 1 + usize::from(voice.is_some()), total, d)),
        (true, None) => g.push_str(&format!("[{m}:a]anull")),
        _ => g.push_str(&format!("[{m}:a]afade=t=in:d=0.3,afade=t=out:st={:.3}:d=1.5", (total - 1.5).max(0.0))),
    }
    if let Some(v) = voice {
        g.push_str(&duck(m + 1, v));
    }
    g.push_str(&format!(",loudnorm=I={:.1}:TP=-1.5[aout]", o.lufs));
    g
}

/// Continues a music chain with the voice (ffmpeg input `input`) laid in at `v.at`: the music ducks whenever the
/// voice speaks, and comes back up in the gaps and after the last word.
pub fn duck(input: usize, v: &Voice) -> String {
    format!(
        // a YouTube voice sits near -24 LUFS against a mastered song near -9: bring it up to the song before mixing
        "[mus];[{input}:a]loudnorm=I=-9:TP=-1.5:LRA=11,aresample=48000,afade=t=in:d=0.05,afade=t=out:st={:.3}:d=0.3,adelay=delays={}:all=1,asplit[vo][sc];\
         [sc]apad[scp];[mus]aresample=48000[mr];[mr][scp]sidechaincompress=threshold=0.012:ratio=12:attack=15:release=450[duck];\
         [duck][vo]amix=inputs=2:normalize=0:duration=first",
        (v.len - 0.3).max(0.0), (v.at * 1000.0).round() as u64
    )
}

/// A reference edit read as a recipe: where it cuts, how (flash, dip to black, cut), its colour and shape,
/// and its music with the dialogue taken out.
pub struct Template {
    pub music: PathBuf,
    pub cuts: Vec<f64>,
    pub joins: Vec<Join>,
    pub grade: String,
    pub aspect: &'static str,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct RefFrame {
    pub t: f64,
    pub scene: f64,
    pub y: f64,
    pub u: f64,
    pub v: f64,
    pub sat: f64,
}

/// Cut points and joins of a reference from its frames: hard cuts where the picture jumps, white flashes where
/// it peaks near white, dips where it falls to black. Events closer than 0.3 s merge, a flash or a dip winning.
pub fn structure(fs: &[RefFrame], dur: f64) -> (Vec<f64>, Vec<Join>) {
    let mut ys: Vec<f64> = fs.iter().map(|f| f.y).collect();
    let med = median(&mut ys).unwrap_or(100.0);
    let mut events: Vec<(f64, Join)> = vec![];
    for (i, f) in fs.iter().enumerate() {
        let (p, n) = (fs.get(i.wrapping_sub(1)).map_or(f.y, |x| x.y), fs.get(i + 1).map_or(f.y, |x| x.y));
        let kind = if f.y > 200.0 && f.y - med > 60.0 && f.y >= p && f.y >= n {
            Some(Join { kind: "fadewhite", dur: 0.3, motion: None })
        } else if f.y < 16.0 && med > 40.0 && f.y <= p && f.y <= n && f.t > 0.5 && f.t < dur - 0.8 {
            Some(Join { kind: "fadeblack", dur: 0.5, motion: None })
        } else if f.scene > 0.3 {
            Some(CUT)
        } else {
            None
        };
        if let Some(j) = kind {
            match events.last_mut() {
                Some(last) if f.t - last.0 < 0.3 => {
                    if last.1 == CUT && j != CUT {
                        *last = (f.t, j);
                    }
                }
                _ => events.push((f.t, j)),
            }
        }
    }
    let mut cuts = vec![0.0];
    let mut joins = vec![CUT];
    for (t, j) in events {
        if t > 0.3 && t < dur - 0.3 {
            cuts.push(t);
            joins.push(j);
        }
    }
    cuts.push(dur);
    joins.push(CUT);
    (cuts, joins)
}

/// Its look: black and white, teal, darkened or as shot.
pub fn tone(fs: &[RefFrame]) -> &'static str {
    let n = fs.len().max(1) as f64;
    let (sat, u, v) = fs.iter().fold((0.0, 0.0, 0.0), |a, f| (a.0 + f.sat / n, a.1 + f.u / n, a.2 + f.v / n));
    if sat < 4.5 { "bw" } else if u > 129.0 && v < 127.0 { "teal" } else if sat < 45.0 { "dark" } else { "none" }
}

/// The music of an edit without its dialogue: speech sits in the middle of the stereo field, music around it,
/// so the sides are kept when they carry enough. A mono or music-only file comes back as it is.
/// The side of a stereo mix (left minus right) in both channels: the centred voice cancels, the wide music stays,
/// and it survives the mono mix the beat analysis reads (opposite signs in the two channels would sum to nothing).
const SIDE: &str = "pan=stereo|c0=c0-c1|c1=c0-c1";

pub fn music_only(edit: &Path, cache: &Path) -> Result<PathBuf, String> {
    let e = edit.to_string_lossy();
    let level = |af: &str| {
        let o = Command::new("ffmpeg").args(["-hide_banner", "-i", e.as_ref(), "-vn", "-af", &format!("{af}volumedetect"), "-f", "null", "-"]).output().ok()?;
        String::from_utf8_lossy(&o.stderr).lines().find_map(|l| l.split("mean_volume:").nth(1)?.trim().trim_end_matches(" dB").parse::<f64>().ok())
    };
    let (full, side) = (level(""), level("pan=mono|c0=0.5*c0-0.5*c1,"));
    if !matches!((full, side), (Some(f), Some(s)) if s > f - 24.0 && s > -45.0) {
        return Ok(edit.to_path_buf());
    }
    let dir = cache.join("music");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // -side.wav, not the old -music.wav: those were made with c1=c1-c0, which a mono mix cancels to silence
    let out = dir.join(format!("{}-side.wav", safe(&edit.file_stem().unwrap_or_default().to_string_lossy())));
    if !out.is_file() {
        run(None, "ffmpeg", &["-v", "error", "-y", "-i", e.as_ref(), "-vn", "-af", SIDE, "-ar", "48000", &out.to_string_lossy()])?;
    }
    Ok(out)
}

pub fn template(reference: &Path, cache: &Path) -> Result<Template, String> {
    let r = reference.to_string_lossy();
    let out = run(None, "ffmpeg", &["-v", "error", "-i", r.as_ref(), "-an", "-vf", "scale=160:-2,select='gte(scene,0)',signalstats,metadata=print:file=-", "-f", "null", "-"])?;
    let mut fs: Vec<RefFrame> = vec![];
    for l in out.lines() {
        if let Some(t) = l.split("pts_time:").nth(1) {
            fs.push(RefFrame { t: t.trim().parse().unwrap_or(0.0), ..Default::default() });
        } else if let (Some(f), Some((k, v))) = (fs.last_mut(), l.split_once('=')) {
            let v: f64 = v.trim().parse().unwrap_or(0.0);
            match k {
                "lavfi.scene_score" => f.scene = v,
                "lavfi.signalstats.YAVG" => f.y = v,
                "lavfi.signalstats.UAVG" => f.u = v,
                "lavfi.signalstats.VAVG" => f.v = v,
                "lavfi.signalstats.SATAVG" => f.sat = v,
                _ => {}
            }
        }
    }
    if fs.is_empty() {
        return Err(format!("{r}: no video frames"));
    }
    let dur = crate::fetch::duration(reference)?;
    let (cuts, joins) = structure(&fs, dur);
    let music = music_only(reference, cache)?;
    let (w, h) = dims(reference);
    Ok(Template { music, cuts, joins, grade: tone(&fs).into(), aspect: crate::study::nearest_aspect(w, h) })
}

/// Colour that changes with the music: on each of the hardest hits the picture drops to black and white for a
/// moment, every other one flipping to negative for two frames first. With `story`, everything before the first
/// hard hit (the voice) is black and white, so the colour arrives with the drop.
pub fn color_hits(spans: &[(f64, f64)], joins: &[Join], story: bool) -> String {
    let hits: Vec<f64> = (1..spans.len()).filter(|&i| joins[i].kind == "fadewhite").map(|i| spans[i].0 + joins[i].dur / 2.0).collect();
    let mut f = String::new();
    if story {
        if let Some(first) = hits.first() {
            f.push_str(&format!(",hue=s=0:enable='lt(t,{first:.3})'"));
        }
    }
    for (k, t) in hits.iter().enumerate().skip(usize::from(story)) {
        if k % 2 == 1 {
            f.push_str(&format!(",negate=enable='between(t,{t:.3},{:.3})'", t + 0.07));
        }
        f.push_str(&format!(",hue=s=0:enable='between(t,{:.3},{:.3})'", t + 0.07, t + 0.3));
    }
    f
}

// ---------------------------------------------------------------- lyrics

/// The song's sung words with their times, cached beside the song: the record's lyrics from LRCLIB when it has them,
/// else what whisper hears (any language; instrumental marks like "[Music]" and "♪" left out). Empty when neither has any.
pub fn lyrics(song: &Song, cache: &Path) -> Vec<Word> {
    let dir = cache.join("music").join(format!("lyrics-{}", safe(&song.file.file_stem().unwrap_or_default().to_string_lossy())));
    let wav = dir.join("a.wav");
    let made = std::fs::create_dir_all(&dir).map_err(|e| e.to_string())
        .and_then(|_| if wav.is_file() { Ok(String::new()) } else { run(None, "ffmpeg", &["-v", "error", "-y", "-i", &song.file.to_string_lossy(), "-vn", "-ar", "16000", "-ac", "1", &wav.to_string_lossy()]) })
        .and_then(|_| crate::transcribe::transcript(&dir, &wav, "auto", ""));
    let heard: Vec<Word> = match made {
        Ok(t) => t.words.into_iter().filter(|w| !w.t.contains(['♪', '[', ']', '(', ')', '*']) && !w.t.trim().is_empty()).collect(),
        Err(e) => {
            eprintln!("edits: whisper did not hear the lyrics of {} ({e})", song.title);
            vec![]
        }
    };
    // the record's own words where LRCLIB has them, set on this file's clock by whisper's, or as they are when the
    // file is the record (same length); else whisper's words
    let Some((lines, len)) = crate::lrc::find(&song.title, &dir) else { return heard };
    let same_length = crate::fetch::duration(&song.file).is_ok_and(|d| len > 0.0 && (d - len).abs() <= 2.0);
    match crate::lrc::offset(&lines, &heard).or(same_length.then_some(0.0)) {
        Some(shift) => {
            eprintln!("edits: lyrics of {} from LRCLIB ({} lines)", song.title, lines.len());
            crate::lrc::words(&lines, shift)
        }
        None => {
            eprintln!("edits: LRCLIB's lyrics of {} did not line up with this recording; whisper's words are used", song.title);
            heard
        }
    }
}

/// Sung words from `start` for `total` s as lines on the edit's clock: a line breaks at a pause, a comma, the end of
/// a line of the lyrics (a word ending in "\n") or four words.
pub fn sung(words: &[Word], start: f64, total: f64) -> Vec<(f64, f64, String)> {
    let inside: Vec<&Word> = words.iter().filter(|w| w.s >= start && w.s < start + total - 0.2).collect();
    let mut out: Vec<(f64, f64, String)> = vec![];
    let mut run: Vec<&Word> = vec![];
    for (i, w) in inside.iter().enumerate() {
        run.push(w);
        let next = inside.get(i + 1);
        // four words a line, five when the fifth ends the lyrics' line (no word left alone)
        let full = run.len() >= 5 || (run.len() == 4 && !next.is_some_and(|n| n.t.ends_with('\n')));
        if next.is_none_or(|n| n.s - w.e > 0.6) || full || w.t.ends_with([',', '.', '?', '!', '\n']) {
            let end = next.map_or(w.e + 0.4, |n| (w.e + 0.4).min(n.s - 0.02)).max(w.e).min(start + total);
            out.push((run[0].s - start, end - start, run.iter().map(|w| w.t.trim()).collect::<Vec<_>>().join(" ")));
            run.clear();
        }
    }
    out
}

/// --lyrics: a cut that is not on a sung word moves to the nearest beat within 0.75 s that is, when it keeps 0.35 s
/// from its neighbours, so the words land on the cuts. `cuts`, `beats` and `onsets` share the edit's clock.
pub fn on_words(cuts: &[f64], beats: &[f64], onsets: &[f64]) -> Vec<f64> {
    let sung = |t: f64| onsets.iter().any(|o| (o - t).abs() <= 0.09);
    let mut out = cuts.to_vec();
    for i in 1..out.len().saturating_sub(1) {
        let c = out[i];
        if sung(c) {
            continue;
        }
        let (prev, next) = (out[i - 1], out[i + 1]);
        if let Some(b) = beats.iter().copied().filter(|b| (b - c).abs() > 1e-6 && (b - c).abs() <= 0.75 && sung(*b) && b - prev >= 0.35 && next - b >= 0.35)
            .min_by(|x, y| (x - c).abs().total_cmp(&(y - c).abs())) {
            out[i] = b;
        }
    }
    out
}

// ---------------------------------------------------------------- loops

/// --loop: how much song from before the edit's start comes in under its end (one bar, at most a third of the edit),
/// so the last second leads into the first as the song itself does; None when the edit starts at the song's start.
pub fn preroll(beats: &[f64], song_start: f64, total: f64) -> Option<f64> {
    let d = (4.0 * beat_period(beats)).min(song_start).min(total / 3.0);
    (d >= 0.5).then_some(d)
}

/// The song (input `song`, `total` s from the start) with its `d` s before the start (input `pre`) crossfaded in under
/// its end: played round, the end runs into the start as the record does. A chain left open, like the fades it replaces.
pub fn splice(song: usize, pre: usize, total: f64, d: f64) -> String {
    format!("[{song}:a]atrim=0:{:.3},asetpts=PTS-STARTPTS[s0];[{pre}:a]asetpts=PTS-STARTPTS[s1];[s0][s1]acrossfade=d=0.15:c1=tri:c2=tri", total - d + 0.15)
}

/// --loop: an edit's length cut back to whole bars (four beats of `period`), so the song's rhythm carries on round
/// the loop; at least one bar. `cuts` keep their order and end on the new length.
pub fn whole_bars(cuts: &[f64], period: f64) -> Vec<f64> {
    let total = *cuts.last().unwrap_or(&0.0);
    let bar = 4.0 * period;
    if bar <= 0.0 || total < bar {
        return cuts.to_vec();
    }
    let end = (total / bar + 1e-6).floor() * bar;
    let mut out: Vec<f64> = cuts.iter().copied().filter(|c| *c < end - 0.3).collect();
    out.push(end);
    out
}

/// --loop: the last piece shows the footage that ran just before the first one, so the end runs into the start.
/// `first` is (start, seconds it uses) of the first piece inside its shot (from its start to where its scene ends,
/// not the 3 s piece's end: a first piece longer than its shot runs on into the scene); `need` what the last
/// piece uses. Returns (first's start, last's start), moving the first later inside its shot when `movable` (not when
/// it is tied to a voice line) and there is no room before it. None when the shot has no room either way.
pub fn seam(first: (f64, f64), shot: (f64, f64), need: f64, movable: bool) -> Option<(f64, f64)> {
    let ((from, uses), (s, e)) = (first, shot);
    let from = if from - need >= s - 0.01 { from } else if movable { s + need } else { return None };
    (from + uses <= e + 0.05).then_some((from, from - need))
}

/// --loop when the first shot has no room before it: the best shot with room for both the first piece and the
/// last one before it, among `shots[..usable]`.
pub fn roomy(shots: &[Shot], usable: usize, need: f64) -> Option<usize> {
    shots.iter().take(usable).enumerate().filter(|(_, x)| x.scene_end.max(x.e) - x.s >= need + 0.05).max_by(|a, b| a.1.score.total_cmp(&b.1.score)).map(|x| x.0)
}

/// One finished edit. `picks` are (video file, source start, focus) per piece, `joins` one per cut.
/// `work` is the folder ffmpeg runs in; `subs` is the ASS text written there as cap.ass.
#[allow(clippy::too_many_arguments)]
pub fn render(picks: &[(PathBuf, f64, f64)], cuts: &[f64], joins: &[Join], beats: &[f64], song: &Path, song_start: f64, voice: Option<&Voice>, subs: &str, look: &Look, o: &Opts, work: &Path, out: &Path) -> Result<(), String> {
    let abs = |p: &Path| std::path::absolute(p).map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string());
    let spans = pieces(cuts, joins);
    let total = *cuts.last().unwrap();
    let has_subs = subs.contains("Dialogue:");
    if has_subs {
        std::fs::write(work.join("cap.ass"), subs).map_err(|e| e.to_string())?;
    }
    let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into()];
    for ((file, start, _), (_, len)) in picks.iter().zip(&spans) {
        args.extend(["-ss".into(), format!("{start:.3}"), "-t".into(), format!("{:.3}", len * look.speed() + 0.3), "-i".into(), abs(file)?]);
    }
    args.extend(["-ss".into(), format!("{song_start:.3}"), "-t".into(), format!("{total:.3}"), "-i".into(), abs(song)?]);
    if let Some(v) = voice {
        args.extend(["-ss".into(), format!("{:.3}", v.start), "-t".into(), format!("{:.3}", v.len), "-i".into(), abs(&v.file)?]);
    }
    let pre = if look.looped { preroll(beats, song_start, total) } else { None };
    if let Some(d) = pre {
        args.extend(["-ss".into(), format!("{:.3}", song_start - d), "-t".into(), format!("{d:.3}"), "-i".into(), abs(song)?]);
    }
    let focus: Vec<f64> = picks.iter().map(|p| p.2).collect();
    args.extend(["-filter_complex".into(), filter_graph(o, look, &focus, &spans, joins, beats, total, voice, has_subs, pre), "-map".into(), "[vout]".into(), "-map".into(), "[aout]".into()]);
    args.extend(["-t".into(), format!("{total:.3}"), "-fps_mode".into(), "cfr".into(), "-r".into(), "30".into()]);
    args.extend(crate::util::h264(22));
    args.extend(["-maxrate", "6M", "-bufsize", "12M",
        "-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-movflags", "+faststart"].iter().map(|s| s.to_string()));
    args.push(abs(out)?);
    // kept next to the subtitles so a bad edit can be re-run by hand
    let _ = std::fs::write(work.join("ffmpeg.txt"), args.iter().map(|a| format!("'{}'", a.replace('\'', "'\\''"))).collect::<Vec<_>>().join(" "));
    run(Some(work), "ffmpeg", &args).map(|_| ())
}

/// Only letters, digits and a few marks: the tag goes into an ffmpeg filter.
pub fn clean_tag(t: &str) -> String {
    t.chars().filter(|c| c.is_alphanumeric() || "@._- ".contains(*c)).take(40).collect()
}

pub fn video_file(id: &str, url: &str, cache: &Path) -> Result<PathBuf, String> {
    let work = cache.join(safe(id));
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let cached = std::fs::read_dir(&work).into_iter().flatten().flatten().map(|e| e.path())
        .find(|p| p.file_stem().is_some_and(|s| s == "hd") && p.extension().is_some_and(|e| e != "part"));
    let v = match cached {
        Some(v) => v,
        None => {
            // picture only: the song replaces the sound. 1080p, not 720: a 9:16 crop of a 720p video is 405 pixels
            // wide and came out 2.7x upscaled before any zoom, the main reason edits looked soft
            let out = yt(&[
                "--no-warnings", "--no-progress", "--no-playlist", "-f", "bv*[height<=1080][ext=mp4]/bv*[height<=1080]/b[height<=1080]/b",
                "--print", "after_move:filepath", "-o", &work.join("hd.%(ext)s").to_string_lossy(), "--", url,
            ])?;
            out.lines().map(str::trim).rfind(|l| !l.is_empty()).map(PathBuf::from).ok_or("yt-dlp reported no file")?
        }
    };
    // a contact sheet of the footage, so whoever picks moments can look at it without running ffmpeg
    let sheet = work.join("sheet.jpg");
    if sheet.exists() || crate::fetch::duration(&v).and_then(|d| crate::check::contact_sheet(&v, d, &sheet, 8, 4, 160)).is_ok() {
        eprintln!("edits: footage {id}: sheet {}", sheet.display());
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_most_replayed_stretch_wins_over_the_loudest() {
        let n = (100.0 / super::hop_s()) as usize;
        let heat = super::heat_frames(&[(0.0, 50.0, 0.1), (60.0, 70.0, 1.0), (70.0, 100.0, 0.2)], n);
        let beats: Vec<f64> = (0..100).map(|b| b as f64).collect();
        assert_eq!(super::windows(&beats, &heat, 10.0, 1), vec![60.0]);
        assert!(super::heat_frames(&[], n).is_empty(), "no heatmap: the loudness decides");
    }

    #[test]
    fn the_side_mix_survives_a_mono_mix() {
        let side = super::SIDE.trim_start_matches("pan=stereo|");
        let (c0, c1) = side.split_once('|').unwrap();
        assert_eq!(c0.trim_start_matches("c0="), c1.trim_start_matches("c1="), "both channels carry the same side, so mono keeps it");
    }

    use super::*;

    #[test]
    fn fft_finds_a_tone() {
        let n = 64;
        let mut re: Vec<f64> = (0..n).map(|i| (2.0 * PI * 5.0 * i as f64 / n as f64).cos()).collect();
        let mut im = vec![0.0; n];
        fft(&mut re, &mut im);
        let mag: Vec<f64> = re.iter().zip(&im).map(|(a, b)| a.hypot(*b)).collect();
        assert!((mag[5] - 32.0).abs() < 1e-9 && (mag[59] - 32.0).abs() < 1e-9);
        assert!(mag.iter().enumerate().all(|(k, m)| k == 5 || k == 59 || *m < 1e-9));
    }

    #[test]
    fn finds_the_beat_of_clicks() {
        // a click every 0.5 s (120 BPM) for 12 s
        let mut x = vec![0f32; SR * 12];
        for b in 0..24 {
            for i in 0..200 {
                x[b * SR / 2 + i] = ((i as f32) * 0.9).sin() * (1.0 - i as f32 / 200.0);
            }
        }
        let (env, rms, _) = onsets(&x);
        assert_eq!(rms.len(), env.len());
        let lag = tempo(&env);
        assert!((lag as f64 * hop_s() - 0.5).abs() < 0.03, "{lag}");
        let beats: Vec<f64> = track(&env, lag).into_iter().map(|f| f as f64 * hop_s()).collect();
        assert!(beats.len() >= 20, "{beats:?}");
        // every beat sits on a click
        assert!(beats.iter().all(|b| (b * 2.0 - (b * 2.0).round()).abs() < 0.12), "{beats:?}");
    }

    #[test]
    fn beats_snap_onto_their_attacks() {
        // attacks every 20 frames; the tracker is 2 frames late, doubled one beat and missed another
        let mut env = vec![0.0; 200];
        (0..10).for_each(|i| env[i * 20 + 5] = 1.0);
        let got = refine(&[7, 27, 30, 47, 87, 107], &env, 20);
        assert_eq!(got, vec![5, 25, 45, 65, 85, 105]);
        assert_eq!(refine(&[], &env, 20), Vec::<usize>::new());
    }

    #[test]
    fn corecore_flickers_and_dips() {
        let c = glimpses(&[0.0, 1.6, 3.2, 4.8, 6.4, 6.9]);
        let want = [0.0, 1.6, 3.2, 3.32, 3.44, 4.8, 6.4, 6.9];
        assert!(c.len() == want.len() && c.iter().zip(want).all(|(a, b)| (a - b).abs() < 1e-9), "{c:?}");
        let j = joins_core(&[0.0, 1.6, 3.2, 4.8, 6.4, 8.0, 8.12, 8.24, 9.6, 11.2]);
        assert_eq!(j[3].kind, "fadeblack");
        assert!(j[5..8].iter().all(|x| *x == CUT) && j[0] == CUT && j[9] == CUT, "{j:?}");
    }

    #[test]
    fn lyrics_on_the_edit() {
        let w = |s: f64, e: f64, t: &str| Word { s, e, t: t.into() };
        let words = vec![w(9.0, 9.4, "early"), w(10.2, 10.5, "I"), w(10.5, 10.9, "keep"), w(10.9, 11.3, "running,"), w(11.4, 11.8, "every"), w(11.8, 12.1, "night"), w(14.0, 14.6, "away")];
        let s = sung(&words, 10.0, 5.0);
        assert_eq!(s.len(), 3, "{s:?}");
        assert_eq!(s[0].2, "I keep running,");
        assert!((s[0].0 - 0.2).abs() < 1e-9 && (s[1].1 - 2.5).abs() < 1e-9 && s[2].2 == "away" && (s[2].1 - 5.0).abs() < 1e-9, "{s:?}");
        // the lyrics' own lines: "çıkar bir de sen koy" then "Anlamıyorsun beni", not run together
        let lines = vec![(1.0, "çıkar bir de sen koy".to_string()), (3.0, "Anlamıyorsun beni,".into()), (6.0, "son".into())];
        let s = sung(&crate::lrc::words(&lines, 0.0), 0.0, 6.0);
        assert_eq!(s.iter().map(|x| x.2.as_str()).collect::<Vec<_>>(), ["çıkar bir de sen koy", "Anlamıyorsun beni,"]);
        let long = vec![(1.0, "You like a pretty boy with a pretty voice".to_string()), (6.0, "x".into())];
        let s = sung(&crate::lrc::words(&long, 0.0), 0.0, 6.0);
        assert_eq!(s.iter().map(|x| x.2.as_str()).collect::<Vec<_>>(), ["You like a pretty", "boy with a pretty voice"]);
        let five = vec![(1.0, "You like a pretty boy".to_string()), (4.0, "With a pretty voice".into())];
        let s = sung(&crate::lrc::words(&five, 0.0), 0.0, 6.0);
        assert_eq!(s.iter().map(|x| x.2.as_str()).collect::<Vec<_>>(), ["You like a pretty boy", "With a pretty voice"]);
        // cut at 2.0 with no word: the beat at 1.5 has one; the cut at 3.0 already sits on a word
        let beats = [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];
        assert_eq!(on_words(&[0.0, 2.0, 3.0, 4.0], &beats, &[1.52, 3.0]), vec![0.0, 1.5, 3.0, 4.0]);
        // too close to its neighbour: stays
        assert_eq!(on_words(&[0.0, 1.8, 2.0, 4.0], &beats, &[1.5]), vec![0.0, 1.5, 2.0, 4.0]);
        assert_eq!(on_words(&[0.0, 1.6, 1.8, 4.0], &beats, &[1.5]), vec![0.0, 1.6, 1.8, 4.0]);
    }

    #[test]
    fn a_roomy_shot_for_the_loop() {
        let sh = |s: f64, e: f64, score: f64| Shot { video: 0, s, e, scene_end: e, score, focus: None };
        let shots = vec![sh(0.0, 1.0, 9.0), sh(5.0, 9.0, 2.0), sh(10.0, 14.0, 3.0), sh(20.0, 30.0, 8.0)];
        assert_eq!(roomy(&shots, 3, 3.0), Some(2));
        assert_eq!(roomy(&shots, 4, 3.0), Some(3));
        assert_eq!(roomy(&shots, 4, 20.0), None);
        // a 3 s piece of a 12 s scene has room for the scene's rest
        let long = vec![Shot { scene_end: 12.0, ..sh(0.0, 3.0, 1.0) }];
        assert_eq!(roomy(&long, 1, 6.0), Some(0));
    }

    #[test]
    fn loops_play_round() {
        let beats: Vec<f64> = (0..40).map(|i| i as f64 * 0.5).collect();
        // one bar of 2 s before a start 10 s in; a start at 0.3 s has no song before it
        assert_eq!(preroll(&beats, 10.0, 18.0), Some(2.0));
        assert_eq!(preroll(&beats, 0.3, 18.0), None);
        assert_eq!(preroll(&beats, 10.0, 3.0), Some(1.0));
        let sp = splice(6, 8, 16.0, 2.0);
        assert!(sp.starts_with("[6:a]atrim=0:14.150") && sp.contains("[8:a]asetpts") && sp.ends_with("acrossfade=d=0.15:c1=tri:c2=tri"), "{sp}");
        // 17.3 s of cuts on a 0.5 s beat: 16 s, whole bars of 2 s
        assert_eq!(whole_bars(&[0.0, 5.0, 15.8, 17.3], 0.5), vec![0.0, 5.0, 16.0]);
        assert_eq!(whole_bars(&[0.0, 1.0], 0.5), vec![0.0, 1.0]);
        // the first piece at 12 s in a 10-20 s shot, 1.5 s before it for the last piece
        assert_eq!(seam((12.0, 1.0), (10.0, 20.0), 1.5, false), Some((12.0, 10.5)));
        // no room before it: moved later in its shot, unless a voice line holds it
        assert_eq!(seam((10.2, 1.0), (10.0, 20.0), 1.5, true), Some((11.5, 10.0)));
        assert_eq!(seam((10.2, 1.0), (10.0, 20.0), 1.5, false), None);
        assert_eq!(seam((10.2, 1.0), (10.0, 12.0), 1.5, true), None);
        let look = Look { sad: false, punch: true, length: 6.0, tag: String::new(), text: String::new(), pace: 1.0, hits: vec![], looped: true, core: false };
        let o = Opts { w: 1080, h: 1920, grade: "none".into(), captions: true, layout: "fill".into(), lufs: -14.0, cam: None };
        let js = vec![CUT, CUT];
        let g = filter_graph(&o, &look, &[0.5, 0.5], &pieces(&[0.0, 1.0, 2.0], &js), &js, &[], 2.0, None, false, Some(0.5));
        assert!(!g.contains("fade=t=") && !g.contains("afade") && g.contains("[2:a]atrim=0:1.650") && g.contains("[3:a]asetpts"), "{g}");
    }

    #[test]
    fn the_hook_comes_first() {
        let beats: Vec<f64> = (0..120).map(|i| i as f64 * 0.5).collect();
        let mut loud = vec![0.1; (60.0 / hop_s()) as usize];
        let n = loud.len();
        loud[(40.0 / hop_s()) as usize..n].iter_mut().for_each(|v| *v = 1.0);
        let mut s = Song { file: PathBuf::new(), title: String::new(), beats, strength: vec![], loud, heat: vec![], hook: None };
        assert!(s.starts(10.0, 1)[0] >= 40.0, "{:?}", s.starts(10.0, 1));
        // TikTok uses 12.3 s in: the beat at it, then the loudest part
        s.set_hook(12.3);
        let st = s.starts(10.0, 2);
        assert_eq!(st[0], 12.0);
        assert!(st[1] >= 22.0, "{st:?}");
        assert!(s.heat[(15.0 / hop_s()) as usize] >= 1.0 && s.heat[(50.0 / hop_s()) as usize] == 0.0);
        // a hook near the end moves back so the stretch still fits
        s.set_hook(57.0);
        assert_eq!(s.starts(10.0, 1), vec![50.0]);
    }

    #[test]
    fn loudest_windows_and_cuts() {
        let beats: Vec<f64> = (0..60).map(|i| i as f64 * 0.5).collect();
        let mut loud = vec![0.1; (30.0 / hop_s()) as usize];
        let quiet = loud.len();
        loud[(20.0 / hop_s()) as usize..quiet].iter_mut().for_each(|v| *v = 1.0);
        let w = windows(&beats, &loud, 6.0, 2);
        assert_eq!(w.len(), 2);
        assert!(w[0] >= 20.0 && (w[0] - w[1]).abs() >= 6.0, "{w:?}");
        // shots of ~1.1 s on a 0.5 s beat -> every 2nd beat
        let c = cuts(&beats, &[], 10.0, 6.0, 1.1);
        assert_eq!(c, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(cuts(&[], &[], 0.0, 6.0, 1.0), vec![0.0, 6.0]);
        // 16-22 s quiet, 22-28 s loud: long holds, then cuts on every beat
        let c = cuts(&beats, &loud, 16.0, 12.0, 1.1);
        let before: Vec<f64> = c.iter().filter(|t| **t < 4.0).copied().collect();
        let after: Vec<f64> = c.iter().filter(|t| **t > 7.0).copied().collect();
        assert!(before.windows(2).all(|w| w[1] - w[0] >= 2.0), "{c:?}");
        assert!(after.windows(2).all(|w| w[1] - w[0] <= 0.5 + 1e-9), "{c:?}");
    }

    #[test]
    fn crossfaded_pieces_meet_on_the_beat() {
        let j = |d| Join { kind: "fade", dur: d, motion: None };
        let p = pieces(&[0.0, 2.0, 4.0, 6.0], &[CUT, j(0.4), j(0.4), CUT]);
        let want = [(0.0, 2.2), (1.8, 2.4), (3.8, 2.2)];
        assert!(p.iter().zip(want).all(|(a, b)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9), "{p:?}");
    }

    #[test]
    fn joins_follow_the_music() {
        // the hardest hit gets the flash, the next ones a zoom, the rest hard cuts; ends are never joins
        let hits = [9.0, 1.0, 5.0, 2.0, 0.5, 3.0, 0.1, 9.0];
        let fast = joins(&hits, false);
        assert_eq!(fast[2].kind, "fadewhite");
        assert_eq!(fast.iter().filter(|j| j.kind == "zoomin").count(), 2);
        assert_eq!((fast[0], fast[7], fast[4]), (CUT, CUT, CUT));
        let sad = joins(&hits, true);
        assert_eq!(sad[2].kind, "fadeblack");
        assert!(sad[1..7].iter().all(|j| j.dur >= 0.45));
        assert_eq!(joins(&[1.0, 2.0], false), vec![CUT, CUT]);
        assert_eq!(hit_at(&[0.0, 0.5, 1.0], &[1.0, 2.0, 3.0], 0.6), 2.0);
        assert_eq!(hit_at(&[], &[], 0.6), 0.0);
    }

    #[test]
    fn shots_from_frames() {
        let f = |diff: f64, luma: f64, face: Option<f64>| Frame { diff, luma, face, checked: true, color: 0.0, detail: 20.0 };
        let mut fs = vec![f(0.0, 120.0, None); 100];
        fs.extend(vec![f(0.03, 120.0, Some(0.3)); 40]); // a face, moving
        fs[100].diff = 0.5; // hard cut
        fs.extend(vec![f(0.0, 5.0, None); 60]); // black
        fs[140].diff = 0.5;
        let s = segment(&fs, 7);
        assert!(s.iter().all(|x| x.e - x.s <= 3.0 + 1.0 && x.e - x.s >= 0.75), "{s:?}");
        let best = s.iter().max_by(|a, b| a.score.total_cmp(&b.score)).unwrap();
        assert!(best.video == 7 && (25.0..35.0).contains(&best.s) && best.focus == Some(0.3), "{best:?}");
        assert!(s.iter().filter(|x| x.s >= 35.0).all(|x| x.score < 0.0)); // the black stretch
    }

    #[test]
    fn empty_frames_are_found() {
        let flat = vec![200u8; 320 * 180];
        let mut drawn = flat.clone();
        for (i, p) in drawn.iter_mut().enumerate() {
            if (i % 320) % 7 < 2 || (i / 320) % 9 < 2 {
                *p = 30;
            }
        }
        assert!(detail(&flat, 320) < 0.01);
        assert!(detail(&drawn, 320) > EMPTY, "{}", detail(&drawn, 320));
        let f = |detail: f64| Frame { diff: 0.02, luma: 150.0, face: None, checked: true, color: 10.0, detail };
        let mut fs = vec![f(0.5); 40];
        fs.extend(vec![f(20.0); 40]);
        fs[40].diff = 0.5;
        let s = segment(&fs, 0);
        let (blank, full): (Vec<_>, Vec<_>) = s.iter().partition(|x| x.s < 10.0);
        assert!(blank.iter().all(|b| full.iter().all(|f| f.score > b.score + 2.0)), "{s:?}");
    }

    #[test]
    fn picks_spread_over_videos() {
        let sh = |video: usize, s: f64, score: f64| Shot { video, s, e: s + 3.0, scene_end: s + 3.0, score, focus: None };
        // video 0 has all the best shots, video 1 slightly worse ones
        let shots: Vec<Shot> = (0..6).map(|k| sh(0, k as f64 * 20.0, 3.0)).chain((0..6).map(|k| sh(1, k as f64 * 20.0, 2.5))).collect();
        let p = assign(&shots, &mut [false; 12], &[2.0; 4], None);
        let from0 = p.iter().filter(|(i, _)| shots[*i].video == 0).count();
        assert_eq!(from0, 2, "{p:?}");
        // and they alternate after the hook
        let order: Vec<usize> = p.iter().map(|(i, _)| shots[*i].video).collect();
        assert!(order.windows(2).skip(1).all(|w| w[0] != w[1]), "{order:?}");
    }

    #[test]
    fn one_long_slot_does_not_rule_out_short_scenes() {
        // nine 3 s scenes and one 8 s one; nine 1 s slots and one 7 s slot
        let mut shots: Vec<Shot> = (0..9).map(|k| Shot { video: 0, s: k as f64 * 20.0, e: k as f64 * 20.0 + 3.0, scene_end: k as f64 * 20.0 + 3.0, score: 2.0, focus: None }).collect();
        shots.push(Shot { video: 0, s: 300.0, e: 308.0, scene_end: 308.0, score: 1.0, focus: None });
        let mut needs = vec![1.0; 10];
        needs[4] = 7.0;
        let p = assign(&shots, &mut [false; 10], &needs, None);
        assert_eq!(p.len(), 10);
        assert_eq!(p[4].0, 9, "the long slot gets the long scene: {p:?}");
    }

    #[test]
    fn few_shots_are_used_again_from_elsewhere() {
        let shots: Vec<Shot> = (0..3).map(|k| Shot { video: 0, s: k as f64 * 20.0, e: k as f64 * 20.0 + 10.0, scene_end: k as f64 * 20.0 + 10.0, score: 1.0, focus: None }).collect();
        let p = assign(&shots, &mut [false; 3], &[2.0; 6], None);
        assert_eq!(p.len(), 6);
        // no two picks start at the same second
        let mut starts: Vec<i64> = p.iter().map(|x| (x.1 * 10.0) as i64).collect();
        starts.sort();
        starts.dedup();
        assert_eq!(starts.len(), 6, "{p:?}");
    }

    #[test]
    fn one_short_video_still_fills_the_edit() {
        // 10 shots 3 s apart in one 30 s video: 12 s apart would allow 3; the gap gives way
        let shots: Vec<Shot> = (0..10).map(|k| Shot { video: 0, s: k as f64 * 3.0, e: k as f64 * 3.0 + 3.0, scene_end: k as f64 * 3.0 + 3.0, score: 1.0, focus: None }).collect();
        assert_eq!(assign(&shots, &mut [false; 10], &[1.0; 9], None).len(), 9);
    }

    #[test]
    fn hook_first_then_story_order() {
        let sh = |s: f64, score: f64| Shot { video: 0, s, e: s + 3.0, scene_end: s + 3.0, score, focus: None };
        let shots = vec![sh(0.0, 1.0), sh(15.0, 5.0), sh(30.0, 2.0), sh(45.0, 3.0), Shot { e: 61.0, scene_end: 61.0, ..sh(60.0, 9.0) }];
        let mut used = vec![false; shots.len()];
        let p = assign(&shots, &mut used, &[2.0, 2.0, 2.0], None);
        // the 1 s shot is too short; best is shot 1, then 3 and 2 in time order
        assert_eq!(p, vec![(1, 15.5), (2, 30.5), (3, 45.5)]);
        // only one long-enough shot left unused: start over with the best rather than make a short edit
        let q = assign(&shots, &mut used, &[2.0, 2.0], None);
        assert_eq!(q.iter().map(|x| x.0).collect::<Vec<_>>(), vec![1, 3]);
        // neighbouring pieces of one scene are not used together
        let near = vec![sh(0.0, 9.0), sh(3.0, 8.0), sh(10.0, 7.0), sh(20.0, 1.0)];
        let r = assign(&near, &mut [false; 4], &[2.0, 2.0], None);
        // footage from when the words were said wins over better-looking footage
        // a 6 s piece fits a 3 s chunk whose scene runs on
        let long = vec![Shot { scene_end: 9.0, ..sh(0.0, 1.0) }, sh(20.0, 5.0)];
        assert_eq!(assign(&long, &mut [false; 2], &[6.0], None), vec![(0, 0.0)]);
        let pair = vec![sh(0.0, 2.5), sh(20.0, 0.5)];
        let r2 = assign(&pair, &mut [false; 2], &[2.0], Some((0, 19.0, 25.0)));
        assert_eq!(r2[0].0, 1);
        assert_eq!(r.iter().map(|x| x.0).collect::<Vec<_>>(), vec![0, 3]);
    }

    #[test]
    fn the_drop_is_where_it_gets_loud() {
        let beats: Vec<f64> = (0..40).map(|i| i as f64 * 0.5).collect();
        let loud: Vec<f64> = (0..(20.0 / hop_s()) as usize).map(|f| if f as f64 * hop_s() >= 12.0 { 1.0 } else { 0.2 }).collect();
        assert_eq!(drop_at(&beats, &vec![0.0; 40], &loud, &[], 5.0, 4.0), Some(12.0));
        // no room before it: nothing
        assert_eq!(drop_at(&beats, &[], &loud, &[], 19.0, 4.0), None);
    }

    #[test]
    fn the_drop_people_replay_beats_a_louder_one() {
        let beats: Vec<f64> = (0..40).map(|i| i as f64 * 0.5).collect();
        let n = (20.0 / hop_s()) as usize;
        let t = |f: usize| f as f64 * hop_s();
        let loud: Vec<f64> = (0..n).map(|f| if (8.0..10.5).contains(&t(f)) || t(f) >= 14.0 { 1.0 } else { 0.2 }).collect();
        assert_eq!(drop_at(&beats, &vec![0.0; 40], &loud, &[], 5.0, 4.0), Some(8.0), "loudness alone: the first drop");
        let heat: Vec<f64> = (0..n).map(|f| if (13.0..14.25).contains(&t(f)) { 1.0 } else { 0.1 }).collect();
        assert_eq!(drop_at(&beats, &vec![0.0; 40], &loud, &heat, 5.0, 4.0), Some(14.0), "the replayed one");
    }

    #[test]
    fn reads_a_reference() {
        let f = |t: f64, scene: f64, y: f64| RefFrame { t, scene, y, u: 135.0, v: 120.0, sat: 20.0 };
        let mut fs: Vec<RefFrame> = (0..300).map(|i| f(i as f64 / 30.0, 0.01, 100.0)).collect();
        fs[60].scene = 0.6; // cut at 2.0
        for (k, y) in [(119, 150.0), (120, 240.0), (121, 180.0)] {
            fs[k].y = y; // white flash at 4.0, with the cut it hides
        }
        fs[121].scene = 0.5;
        fs[180].y = 5.0; // dip to black at 6.0
        let (cuts, joins) = structure(&fs, 10.0);
        assert_eq!(cuts, vec![0.0, 2.0, 4.0, 6.0, 10.0]);
        assert_eq!(joins.iter().map(|j| j.kind).collect::<Vec<_>>(), vec!["fade", "fade", "fadewhite", "fadeblack", "fade"]);
        assert_eq!(tone(&fs), "teal");
        assert_eq!(tone(&[RefFrame { sat: 1.0, ..Default::default() }]), "bw");
    }

    #[test]
    fn colour_follows_the_hits() {
        let flash = Join { kind: "fadewhite", dur: 0.3, motion: None };
        let spans = [(0.0, 5.15), (4.85, 2.3), (6.85, 2.3), (8.85, 1.0)];
        let joins = [CUT, flash, flash, flash];
        let story = color_hits(&spans, &joins, true);
        assert!(story.starts_with(",hue=s=0:enable='lt(t,5.000)'") && story.contains(",negate=enable='between(t,7.000,7.070)'"), "{story}");
        let plain = color_hits(&spans, &joins, false);
        assert!(plain.starts_with(",hue=s=0:enable='between(t,5.070,5.300)'"), "{plain}");
        assert_eq!(color_hits(&spans, &[CUT; 4], false), "");
    }

    #[test]
    fn the_picture_runs_to_the_end() {
        let Ok(_) = std::process::Command::new("ffmpeg").arg("-version").output() else { return };
        // a real music edit's cuts whose picture stopped at 3.07 s (92 of 501 frames): the piece before a transition
        // ended a frame short of it
        let cuts = [0.0, 1.033, 2.009, 3.088, 4.656, 5.155, 5.666, 7.221, 8.278, 9.288, 10.344, 11.355, 12.411, 13.444, 14.454, 15.523, 16.718];
        let mut js = vec![CUT; cuts.len() - 1];
        for i in [6, 9, 12] {
            js[i] = Join { kind: "fadewhite", dur: 0.3, motion: None };
        }
        let o = Opts { w: 64, h: 64, grade: "dark".into(), captions: false, layout: "fill".into(), lufs: -12.0, cam: None };
        let look = Look { sad: false, punch: true, length: 16.0, tag: String::new(), text: String::new(), pace: 1.0, hits: vec![], looped: false, core: false };
        let spans = pieces(&cuts, &js);
        let g = filter_graph(&o, &look, &vec![0.5; spans.len()], &spans, &js, &[], 16.718, None, false, None);
        let mut args: Vec<String> = vec!["-v".into(), "error".into()];
        for _ in &spans {
            args.extend(["-f", "lavfi", "-i", "testsrc=s=96x96:d=3:r=30"].map(String::from));
        }
        args.extend(["-f", "lavfi", "-i", "anullsrc=d=17", "-filter_complex", &g, "-map", "[vout]", "-map", "[aout]", "-t", "16.718", "-fps_mode", "cfr", "-r", "30", "-f", "framecrc", "-"].map(String::from));
        let out = std::process::Command::new("ffmpeg").args(&args).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let frames = String::from_utf8_lossy(&out.stdout).lines().filter(|l| l.starts_with("0,")).count();
        assert!(frames >= 500, "{frames} frames of 501");
    }

    #[test]
    fn graph_shapes() {
        let o = Opts { w: 1080, h: 1080, grade: "dark".into(), captions: false, layout: "fit".into(), lufs: -12.0, cam: None };
        let sad = Look { sad: true, punch: false, length: 6.0, tag: "@me".into(), text: String::new(), pace: 1.0, hits: vec![], looped: false, core: false };
        let cuts = [0.0, 2.0, 4.0];
        let js = joins(&[1.0, 5.0, 1.0], true);
        let g = filter_graph(&o, &sad, &[0.5, 0.4], &pieces(&cuts, &js), &js, &[], 4.0, None, false, None);
        assert!(g.contains("[0:v]setpts=(PTS-STARTPTS)/0.8,fps=30,crop=") && g.contains("[v0][v1]xfade=transition=fadeblack:duration=0.700:offset=1.650[x1];[x1]settb=1/30,fps=30,fade=t=in"), "{g}");
        assert!(g.contains("drawtext=text='@me'") && g.contains("[2:a]afade") && g.contains("loudnorm=I=-12.0") && !g.contains("ass="), "{g}");
        let hype = Look { sad: false, punch: true, length: 6.0, tag: String::new(), text: String::new(), pace: 1.0, hits: vec![], looped: false, core: false };
        let js = joins(&[0.0, 1.0, 0.0], false);
        let v = Voice { file: "a.m4a".into(), start: 30.0, len: 1.5, at: 0.5 };
        let g = filter_graph(&o, &hype, &[0.5, 0.5], &pieces(&[0.0, 1.0, 2.0], &js), &js, &[0.0, 0.5, 1.0, 1.5], 2.0, Some(&v), true, None);
        // beats kick the picture: piece 1 (from 0.85 s) kicks at 0.15 s and 0.65 s into itself
        assert!(g.contains("[1:v]") && g.contains("+0.08*max(0,1-(it-0.150)*6)*gte(it,0.150)+0.08*max(0,1-(it-0.650)*6)"), "{g}");
        assert!(g.contains("tmix=frames=2") && g.contains("xfade=transition=fadewhite") && g.contains(",ass=cap.ass") && !g.contains("drawtext"), "{g}");
        assert!(g.contains("[3:a]loudnorm=I=-9:TP=-1.5:LRA=11,aresample=48000,afade=t=in:d=0.05,afade=t=out:st=1.200:d=0.3,adelay=delays=500:all=1,asplit[vo][sc];") && g.contains("[sc]apad[scp]") && g.contains("[duck][vo]amix=inputs=2:normalize=0:duration=first,loudnorm=I=-12.0:TP=-1.5[aout]"), "{g}");
        assert_eq!(clean_tag("@x'; rm -rf"), "@x rm -rf");
    }

    #[test]
    fn caption_styles() {
        let o = Opts { w: 1080, h: 1080, grade: "dark".into(), captions: true, layout: "fill".into(), lufs: -12.0, cam: None };
        let w = |s: f64, t: &str| Word { s, e: s + 0.3, t: t.into() };
        let words = [w(10.0, "bunu"), w(10.4, "bilmiyordum."), w(11.0, "ama"), w(13.0, "sonra")];
        let m = Moment { start: 10.0, end: 12.0, punch: 11.0, score: 0.0, title: String::new(), why: String::new() };
        let fast = captions(&words, Some(&m), 0.0, "the {problem}", false, 8.0, &o);
        assert!(fast.contains(",T,,0,0,0,,{\\fad(250,400)}THE PROBLEM\n"), "{fast}");
        assert!(fast.contains("Dialogue: 0,0:00:00.40,0:00:01.00,W,,0,0,0,,{\\fscx125\\fscy125\\t(0,90,\\fscx100\\fscy100)}BİLMİYORDUM\n"), "{fast}");
        assert!(!fast.contains("SONRA"), "{fast}"); // after the line ends
        let sad = captions(&words, Some(&m), 1.0, "", true, 8.0, &o);
        assert!(sad.contains("Dialogue: 0,0:00:01.00,") && sad.contains("{\\fad(140,140)}bunu bilmiyordum.") && !sad.contains(",T,,"), "{sad}");
        assert!(!captions(&[], None, 0.0, "", true, 8.0, &o).contains("Dialogue:"));
    }

    #[test]
    fn kicks_outweigh_hi_hats_in_the_bass_band() {
        // a 55 Hz kick every second, a hi-hat (a fast sign-flipping click train) half a second after each
        let n = SR * 4;
        let mut x = vec![0.0f32; n];
        for b in 0..4 {
            let k = b * SR + SR / 10;
            for i in 0..SR / 10 {
                x[k + i] += (0.8 * (-(i as f64) / 600.0).exp() * (2.0 * PI * 55.0 * i as f64 / SR as f64).sin()) as f32;
            }
            let h = k + SR / 2;
            for i in 0..SR / 50 {
                x[h + i] += (0.8 * (-(i as f64) / 60.0).exp() * if i % 2 == 0 { 1.0 } else { -1.0 }) as f32;
            }
        }
        let (env, _, bass) = onsets(&x);
        let at = |sec: f64, e: &[f64]| { let f = (sec / hop_s()) as usize; e[f.saturating_sub(3)..(f + 4).min(e.len())].iter().cloned().fold(0.0, f64::max) };
        // the full band hears both about as loud; the bass band clearly favours the kick
        let full = at(1.1, &env) / at(1.6, &env).max(1e-3);
        let low = at(1.1, &bass) / at(1.6, &bass).max(1e-3);
        assert!(low > 1.5 * full && low > 1.5, "kick/hat: bass band {low:.2}, full band {full:.2}");
    }
}
