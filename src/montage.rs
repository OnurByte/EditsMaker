//! Music edits, the kind people repost: a video's best shots cut onto the beats of a song.
use crate::face::median;
use crate::fetch::{dims, safe};
use crate::pick::Moment;
use crate::render::{ass_time, chunks, grade, shape, upper_tr, Opts};
use crate::transcribe::Word;
use crate::util::{run, run_bytes, yt};
use std::f64::consts::PI;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// How an edit moves. `sad`: long shots, slow motion, crossfades, black and white. Otherwise: fast cuts with a punch-in.
pub struct Look {
    pub sad: bool,
    pub length: f64,
    pub tag: String,
    /// a line shown over the whole edit, like "THE PROBLEM"
    pub text: String,
}

impl Look {
    /// seconds a shot should last; rounded to whole beats
    pub fn shot(&self) -> f64 {
        if self.sad { 2.4 } else { 1.1 }
    }
    pub fn speed(&self) -> f64 {
        if self.sad { 0.8 } else { 1.0 }
    }
}

// ---------------------------------------------------------------- the song

const SR: usize = 22050;
const HOP: usize = 256;
const N: usize = 1024;
const TIGHT: f64 = 100.0; // how hard the beat tracker holds the tempo

fn hop_s() -> f64 {
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
}

fn fft(re: &mut [f64], im: &mut [f64]) {
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

/// Spectral flux (where notes start, normalised) and RMS per frame of `HOP` samples.
pub fn onset(x: &[f32]) -> (Vec<f64>, Vec<f64>) {
    let win: Vec<f64> = (0..N).map(|i| 0.5 - 0.5 * (2.0 * PI * i as f64 / N as f64).cos()).collect();
    let frames = if x.len() >= N { (x.len() - N) / HOP + 1 } else { 0 };
    let (mut env, mut rms) = (Vec::with_capacity(frames), Vec::with_capacity(frames));
    let mut prev = vec![0.0; N / 2];
    for f in 0..frames {
        let seg = &x[f * HOP..f * HOP + N];
        rms.push((seg.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / N as f64).sqrt());
        let mut re: Vec<f64> = seg.iter().zip(&win).map(|(a, w)| *a as f64 * w).collect();
        let mut im = vec![0.0; N];
        fft(&mut re, &mut im);
        let mut flux = 0.0;
        for k in 0..N / 2 {
            let m = (1.0 + 100.0 * re[k].hypot(im[k])).ln();
            flux += (m - prev[k]).max(0.0);
            prev[k] = m;
        }
        env.push(if f == 0 { 0.0 } else { flux });
    }
    // keep the attacks, drop the slow swell
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
    (out, rms)
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

/// Downloads (or takes) the song and finds its beats.
pub fn song(src: &str, cache: &Path) -> Result<Song, String> {
    let (file, title) = if Path::new(src).is_file() {
        (PathBuf::from(src), Path::new(src).file_stem().unwrap_or_default().to_string_lossy().into_owned())
    } else {
        let dir = cache.join("music");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let out = yt(&[
            "--no-warnings", "--no-progress", "--no-playlist", "-f", "ba[ext=m4a]/ba",
            "--print", "after_move:%(title)s\t%(filepath)s", "-o", &dir.join("%(id)s.%(ext)s").to_string_lossy(), "--", src,
        ])?;
        let line = out.lines().rfind(|l| l.contains('\t')).ok_or("yt-dlp gave no song file")?;
        let (t, f) = line.split_once('\t').unwrap_or_default();
        (PathBuf::from(f.trim()), t.trim().to_string())
    };
    let raw = run_bytes("ffmpeg", &["-v", "error", "-i", &file.to_string_lossy(), "-ac", "1", "-ar", &SR.to_string(), "-f", "f32le", "-"])?;
    let x: Vec<f32> = raw.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect();
    let (env, loud) = onset(&x);
    let frames = track(&env, tempo(&env));
    if frames.len() < 4 {
        return Err(format!("could not find a beat in {title}"));
    }
    let strength = frames.iter().map(|&f| env[f.saturating_sub(2)..(f + 3).min(env.len())].iter().cloned().fold(0.0, f64::max)).collect();
    let beats = frames.into_iter().map(|f| f as f64 * hop_s()).collect();
    Ok(Song { file, title, beats, strength, loud })
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
pub fn cuts(beats: &[f64], start: f64, length: f64, shot: f64) -> Vec<f64> {
    let inside: Vec<f64> = beats.iter().filter(|b| **b >= start - 1e-6 && **b <= start + length + 1e-6).map(|b| b - start).collect();
    if inside.len() < 2 {
        return vec![0.0, length];
    }
    let mut gaps: Vec<f64> = inside.windows(2).map(|w| w[1] - w[0]).collect();
    let period = median(&mut gaps).unwrap_or(0.5);
    let k = ((shot / period).round() as usize).max(1);
    let mut out: Vec<f64> = inside.iter().step_by(k).copied().collect();
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
    pub score: f64,
    pub focus: Option<f64>,
}

#[derive(Clone, Copy, Default)]
pub struct Frame {
    /// mean change from the previous frame, 0..1
    pub diff: f64,
    pub luma: f64,
    pub face: Option<f64>,
    pub checked: bool,
}

const FPS: f64 = 4.0;
const AW: u32 = 320;

/// Streams gray frames out of ffmpeg and measures each one.
pub fn frames(video: &Path) -> Result<Vec<Frame>, String> {
    let (sw, sh) = dims(video);
    if sw == 0 || sh == 0 {
        return Err(format!("{}: no video stream", video.display()));
    }
    let h = (AW * sh / sw / 2 * 2).max(2);
    let mut child = Command::new("ffmpeg")
        .args(["-v", "error", "-i", &video.to_string_lossy(), "-an", "-vf", &format!("fps={FPS},scale={AW}:{h},format=gray"), "-f", "rawvideo", "-"])
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
    let mut fs: Vec<Frame> = vec![];
    while out.read_exact(&mut cur).is_ok() {
        let mut f = Frame { luma: cur.iter().step_by(7).map(|v| *v as f64).sum::<f64>() / (size / 7).max(1) as f64, ..Default::default() };
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
                let score = 2.0 * faces + (motion * 25.0).min(1.0) - if luma < 22.0 { 2.0 } else { 0.0 } - if luma > 235.0 { 1.0 } else { 0.0 };
                out.push(Shot { video, s: a as f64 / FPS, e: e as f64 / FPS, score, focus: median(&mut xs) });
            }
            a = e;
        }
    }
    out
}

/// One shot per piece, each at least `need` seconds of source: the best unused ones, the very best first
/// as the hook, the rest in story order. Returns (shot, source start) per piece.
pub fn assign(shots: &[Shot], used: &mut [bool], needs: &[f64]) -> Vec<(usize, f64)> {
    let need = needs.iter().cloned().fold(0.0, f64::max);
    let mut order: Vec<usize> = (0..shots.len()).filter(|&i| shots[i].e - shots[i].s >= need).collect();
    order.sort_by(|a, b| shots[*b].score.total_cmp(&shots[*a].score));
    if order.iter().filter(|&&i| !used[i]).count() < needs.len() {
        used.iter_mut().for_each(|u| *u = false); // footage ran out: reuse rather than make a short edit
    }
    // pieces of the same scene back to back read as one long shot: keep picks 12 s apart
    let mut pick: Vec<usize> = vec![];
    for i in order.into_iter().filter(|&i| !used[i]) {
        if pick.len() == needs.len() {
            break;
        }
        if pick.iter().all(|&p| shots[p].video != shots[i].video || (shots[p].s - shots[i].s).abs() >= 12.0) {
            pick.push(i);
        }
    }
    pick.iter().for_each(|&i| used[i] = true);
    if pick.len() > 1 {
        pick[1..].sort_by(|a, b| (shots[*a].video, shots[*a].s).partial_cmp(&(shots[*b].video, shots[*b].s)).unwrap());
    }
    pick.iter().zip(needs).map(|(&i, n)| (i, (shots[i].s + shots[i].e - n) / 2.0)).collect()
}

// ---------------------------------------------------------------- the edit

/// How the picture changes at a cut: an ffmpeg xfade transition and its length. A one-frame fade is a hard cut.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Join {
    pub kind: &'static str,
    pub dur: f64,
}

const CUT: Join = Join { kind: "fade", dur: 0.034 };

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
            (false, true, _) => Join { kind: "fadewhite", dur: 0.3 },
            (false, _, true) => Join { kind: "zoomin", dur: 0.2 },
            (false, _, _) => CUT,
            (true, true, _) => Join { kind: "fadeblack", dur: 0.7 },
            (true, _, true) => Join { kind: "dissolve", dur: 0.6 },
            (true, _, _) => Join { kind: "fade", dur: 0.45 },
        };
    }
    out
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
}

/// Subtitles for an edit as ASS: the spoken words and the `--text` line.
/// Fast edits: one word at a time, upper case, popping in (like the "EDUCATION" reference).
/// Sad edits: two or three words, as spoken, fading. The text line: heavy condensed capitals, outlined.
pub fn captions(words: &[Word], m: Option<&Moment>, text: &str, sad: bool, total: f64, o: &Opts) -> String {
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
        s.push_str(&format!("Dialogue: 1,{},{},T,,0,0,0,,{{\\fad(250,400)}}{}\n", ass_time(0.3), ass_time(total - 0.8), upper_tr(&clean(text.trim()))));
    }
    if let Some(m) = m {
        if sad {
            for (a, b, t, _) in chunks(words, m) {
                s.push_str(&format!("Dialogue: 0,{},{},W,,0,0,0,,{{\\fad(140,140)}}{}\n", ass_time(a), ass_time(b), clean(&t)));
            }
        } else {
            let ws: Vec<&Word> = words.iter().filter(|x| x.s >= m.start - 0.05 && x.e <= m.end + 0.05).collect();
            for (i, x) in ws.iter().enumerate() {
                let a = x.s - m.start;
                let b = ws.get(i + 1).map_or(x.e - m.start + 0.4, |n| (n.s - m.start).min(x.e - m.start + 0.6)).max(a + 0.12);
                let t = clean(x.t.trim_matches(|c: char| ",.;:!?\"'".contains(c)));
                s.push_str(&format!("Dialogue: 0,{},{},W,,0,0,0,,{{\\fscx125\\fscy125\\t(0,90,\\fscx100\\fscy100)}}{}\n", ass_time(a), ass_time(b), upper_tr(&t)));
            }
        }
    }
    s
}

#[allow(clippy::too_many_arguments)]
pub fn filter_graph(o: &Opts, look: &Look, focus: &[f64], spans: &[(f64, f64)], joins: &[Join], total: f64, voice: Option<&Voice>, subs: bool) -> String {
    let (w, h, speed) = (o.w, o.h, look.speed());
    let mut g = String::new();
    for (i, (f, (_, len))) in focus.iter().zip(spans).enumerate() {
        // every shot drifts in; fast edits also land each cut with a quick punch and a little motion smear
        let (zoom, smear) = if look.sad {
            ("min(1+0.0012*on,1.12)", "")
        } else {
            ("if(lt(on,5),1.24-0.048*on,min(1+0.0016*(on-5),1.15))", ",tmix=frames=2")
        };
        let tone = if look.sad {
            ",hue=s=0,eq=contrast=1.22:brightness=-0.05,vignette=PI/4.2,noise=alls=5:allf=t".to_string()
        } else {
            format!("{},eq=contrast=1.08,vignette=PI/5", grade(&o.grade))
        };
        g.push_str(&format!(
            "[{i}:v]setpts=(PTS-STARTPTS)/{speed},fps=30,{},zoompan=z='{zoom}':x='iw/2-(iw/zoom/2)':y='ih/2-(ih/zoom/2)':d=1:s={w}x{h}:fps=30{smear}{tone},trim=duration={len:.3},setpts=PTS-STARTPTS,settb=AVTB[v{i}];",
            shape(&Opts { layout: "fill".into(), ..o.clone() }, *f)
        ));
    }
    let mut last = "v0".to_string();
    for (i, (s, _)) in spans.iter().enumerate().skip(1) {
        let j = joins[i];
        g.push_str(&format!("[{last}][v{i}]xfade=transition={}:duration={:.3}:offset={s:.3}[x{i}];", j.kind, j.dur));
        last = format!("x{i}");
    }
    g.push_str(&format!("[{last}]fade=t=in:d=0.4,fade=t=out:st={:.3}:d=1.2", (total - 1.2).max(0.0)));
    if subs {
        g.push_str(",ass=cap.ass");
    }
    if !look.tag.is_empty() {
        g.push_str(&format!(",drawtext=text='{}':font=Adwaita Sans:fontsize=h*0.024:fontcolor=white@0.7:x=(w-tw)/2:y=h*0.93", look.tag));
    }
    let m = spans.len();
    g.push_str(&format!(",format=yuv420p[vout];[{m}:a]afade=t=in:d=0.3,afade=t=out:st={:.3}:d=1.5", (total - 1.5).max(0.0)));
    if let Some(v) = voice {
        // the music ducks whenever the voice speaks, and comes back up in the gaps and after the last word
        g.push_str(&format!(
            "[mus];[{}:a]aresample=48000,afade=t=in:d=0.05,afade=t=out:st={:.3}:d=0.3,asplit[vo][sc];\
             [mus]aresample=48000[mr];[mr][sc]sidechaincompress=threshold=0.012:ratio=12:attack=15:release=450[duck];\
             [duck][vo]amix=inputs=2:normalize=0:duration=first",
            m + 1, (v.len - 0.3).max(0.0)
        ));
    }
    g.push_str(&format!(",loudnorm=I={:.1}:TP=-1.5[aout]", o.lufs));
    g
}

/// One finished edit. `picks` are (video file, source start, focus) per piece, `joins` one per cut.
/// `work` is the folder ffmpeg runs in; `subs` is the ASS text written there as cap.ass.
#[allow(clippy::too_many_arguments)]
pub fn render(picks: &[(PathBuf, f64, f64)], cuts: &[f64], joins: &[Join], song: &Path, song_start: f64, voice: Option<&Voice>, subs: &str, look: &Look, o: &Opts, work: &Path, out: &Path) -> Result<(), String> {
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
    let focus: Vec<f64> = picks.iter().map(|p| p.2).collect();
    args.extend(["-filter_complex".into(), filter_graph(o, look, &focus, &spans, joins, total, voice, has_subs), "-map".into(), "[vout]".into(), "-map".into(), "[aout]".into()]);
    args.extend(["-t", &format!("{total:.3}"), "-c:v", "libx264", "-preset", "medium", "-crf", "22", "-maxrate", "6M", "-bufsize", "12M",
        "-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-movflags", "+faststart"].iter().map(|s| s.to_string()));
    args.push(abs(out)?);
    run(Some(work), "ffmpeg", &args).map(|_| ())
}

/// Only letters, digits and a few marks: the tag goes into an ffmpeg filter.
pub fn clean_tag(t: &str) -> String {
    t.chars().filter(|c| c.is_alphanumeric() || "@._- ".contains(*c)).take(40).collect()
}

pub fn video_file(id: &str, url: &str, cache: &Path) -> Result<PathBuf, String> {
    let work = cache.join(safe(id));
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    if let Some(v) = std::fs::read_dir(&work).into_iter().flatten().flatten().map(|e| e.path())
        .find(|p| p.file_stem().is_some_and(|s| s == "v") && p.extension().is_some_and(|e| e != "part")) {
        return Ok(v);
    }
    // picture only: the song replaces the sound
    let out = yt(&[
        "--no-warnings", "--no-progress", "--no-playlist", "-f", "bv*[height<=720][ext=mp4]/bv*[height<=720]/b[height<=720]",
        "--print", "after_move:filepath", "-o", &work.join("v.%(ext)s").to_string_lossy(), "--", url,
    ])?;
    out.lines().map(str::trim).rfind(|l| !l.is_empty()).map(PathBuf::from).ok_or("yt-dlp reported no file".into())
}

#[cfg(test)]
mod tests {
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
        let (env, rms) = onset(&x);
        assert_eq!(rms.len(), env.len());
        let lag = tempo(&env);
        assert!((lag as f64 * hop_s() - 0.5).abs() < 0.03, "{lag}");
        let beats: Vec<f64> = track(&env, lag).into_iter().map(|f| f as f64 * hop_s()).collect();
        assert!(beats.len() >= 20, "{beats:?}");
        // every beat sits on a click
        assert!(beats.iter().all(|b| (b * 2.0 - (b * 2.0).round()).abs() < 0.12), "{beats:?}");
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
        let c = cuts(&beats, 10.0, 6.0, 1.1);
        assert_eq!(c, vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(cuts(&[], 0.0, 6.0, 1.0), vec![0.0, 6.0]);
    }

    #[test]
    fn crossfaded_pieces_meet_on_the_beat() {
        let j = |d| Join { kind: "fade", dur: d };
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
        let f = |diff: f64, luma: f64, face: Option<f64>| Frame { diff, luma, face, checked: true };
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
    fn hook_first_then_story_order() {
        let sh = |s: f64, score: f64| Shot { video: 0, s, e: s + 3.0, score, focus: None };
        let shots = vec![sh(0.0, 1.0), sh(15.0, 5.0), sh(30.0, 2.0), sh(45.0, 3.0), Shot { e: 61.0, ..sh(60.0, 9.0) }];
        let mut used = vec![false; shots.len()];
        let p = assign(&shots, &mut used, &[2.0, 2.0, 2.0]);
        // the 1 s shot is too short; best is shot 1, then 3 and 2 in time order
        assert_eq!(p, vec![(1, 15.5), (2, 30.5), (3, 45.5)]);
        // only one long-enough shot left unused: start over with the best rather than make a short edit
        let q = assign(&shots, &mut used, &[2.0, 2.0]);
        assert_eq!(q.iter().map(|x| x.0).collect::<Vec<_>>(), vec![1, 3]);
        // neighbouring pieces of one scene are not used together
        let near = vec![sh(0.0, 9.0), sh(3.0, 8.0), sh(10.0, 7.0), sh(20.0, 1.0)];
        let r = assign(&near, &mut [false; 4], &[2.0, 2.0]);
        assert_eq!(r.iter().map(|x| x.0).collect::<Vec<_>>(), vec![0, 3]);
    }

    #[test]
    fn graph_shapes() {
        let o = Opts { w: 1080, h: 1080, grade: "dark".into(), captions: false, layout: "fit".into(), lufs: -12.0 };
        let sad = Look { sad: true, length: 6.0, tag: "@me".into(), text: String::new() };
        let cuts = [0.0, 2.0, 4.0];
        let js = joins(&[1.0, 5.0, 1.0], true);
        let g = filter_graph(&o, &sad, &[0.5, 0.4], &pieces(&cuts, &js), &js, 4.0, None, false);
        assert!(g.contains("[0:v]setpts=(PTS-STARTPTS)/0.8,fps=30,crop=") && g.contains("[v0][v1]xfade=transition=fadeblack:duration=0.700:offset=1.650[x1];[x1]fade=t=in"), "{g}");
        assert!(g.contains("drawtext=text='@me'") && g.contains("[2:a]afade") && g.contains("loudnorm=I=-12.0") && !g.contains("ass="), "{g}");
        let hype = Look { sad: false, length: 6.0, tag: String::new(), text: String::new() };
        let js = joins(&[0.0, 1.0, 0.0], false);
        let v = Voice { file: "a.m4a".into(), start: 30.0, len: 1.5 };
        let g = filter_graph(&o, &hype, &[0.5, 0.5], &pieces(&[0.0, 1.0, 2.0], &js), &js, 2.0, Some(&v), true);
        assert!(g.contains("tmix=frames=2") && g.contains("xfade=transition=fadewhite") && g.contains(",ass=cap.ass") && !g.contains("drawtext"), "{g}");
        assert!(g.contains("[3:a]aresample=48000,afade=t=in:d=0.05,afade=t=out:st=1.200:d=0.3,asplit[vo][sc]") && g.contains("[duck][vo]amix=inputs=2:normalize=0:duration=first,loudnorm=I=-12.0:TP=-1.5[aout]"), "{g}");
        assert_eq!(clean_tag("@x'; rm -rf"), "@x rm -rf");
    }

    #[test]
    fn caption_styles() {
        let o = Opts { w: 1080, h: 1080, grade: "dark".into(), captions: true, layout: "fill".into(), lufs: -12.0 };
        let w = |s: f64, t: &str| Word { s, e: s + 0.3, t: t.into() };
        let words = [w(10.0, "bunu"), w(10.4, "bilmiyordum."), w(11.0, "ama"), w(13.0, "sonra")];
        let m = Moment { start: 10.0, end: 12.0, punch: 11.0, score: 0.0, title: String::new(), why: String::new() };
        let fast = captions(&words, Some(&m), "the {problem}", false, 8.0, &o);
        assert!(fast.contains(",T,,0,0,0,,{\\fad(250,400)}THE PROBLEM\n"), "{fast}");
        assert!(fast.contains("Dialogue: 0,0:00:00.40,0:00:01.00,W,,0,0,0,,{\\fscx125\\fscy125\\t(0,90,\\fscx100\\fscy100)}BİLMİYORDUM\n"), "{fast}");
        assert!(!fast.contains("SONRA"), "{fast}"); // after the line ends
        let sad = captions(&words, Some(&m), "", true, 8.0, &o);
        assert!(sad.contains("{\\fad(140,140)}bunu bilmiyordum.") && !sad.contains(",T,,"), "{sad}");
        assert!(!captions(&[], None, "", true, 8.0, &o).contains("Dialogue:"));
    }
}
