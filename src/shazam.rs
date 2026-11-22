//! Which song is this: Shazam's audio fingerprint made here (ported from shazamio-core, itself from SongRec) and
//! sent to Shazam's public recognition endpoint. 16 kHz mono in, "Artist - Title" and where in the song out.
use crate::util::{run, run_bytes};
use std::collections::BTreeMap;
use std::path::Path;

const SR: u32 = 16000;
const FRAME: usize = 2048;
const HOP: usize = 128;
const BINS: usize = FRAME / 2 + 1;
const HISTORY: usize = 256;
const PEAK_LAG: usize = 45;
const REFERENCE_LAG: usize = 48;
const FREQ_NEIGHBOURS: [usize; 4] = [2, 3, 6, 9];
const TIME_NEIGHBOURS: [usize; 7] = [4, 7, 14, 21, 28, 35, 42];
const SPREAD_BACK: [usize; 3] = [1, 3, 6];
const MIN_PEAK: f32 = 1.0 / 64.0;

/// (frame, magnitude, refined bin) per band 0..4 (250-520, 520-1450, 1450-3500, 3500-5500 Hz).
pub type Peaks = BTreeMap<u8, Vec<(u32, u16, u16)>>;

#[derive(Clone)]
struct Frame {
    power: Vec<f32>,
    spread: Vec<f32>,
}

fn band(hz: u32) -> Option<u8> {
    match hz {
        250..=519 => Some(0),
        520..=1449 => Some(1),
        1450..=3499 => Some(2),
        3500..=5500 => Some(3),
        _ => None,
    }
}

fn log_mag(p: f32) -> f32 {
    p.ln().max(1.0 / 64.0) * 1477.3 + 6144.0
}

/// The spectral peaks Shazam matches on, from 16 kHz samples.
pub fn peaks(samples: &[i16]) -> Peaks {
    let window: Vec<f64> = (1..=FRAME).map(|k| (std::f64::consts::PI * k as f64 / (FRAME + 1) as f64).sin().powi(2)).collect();
    let silence = Frame { power: vec![0.0; BINS], spread: vec![0.0; BINS] };
    let mut hist = vec![silence; HISTORY];
    let mut count = 0usize;
    let slot = |count: usize, lag: usize| (count + HISTORY - 1 - lag) % HISTORY;
    let mut buf = [0i16; FRAME];
    let mut out = Peaks::new();
    for hop in samples.as_chunks::<HOP>().0 {
        buf.copy_within(HOP.., 0);
        buf[FRAME - HOP..].copy_from_slice(hop);
        let mut re: Vec<f64> = buf.iter().zip(&window).map(|(s, w)| *s as f64 * w).collect();
        let mut im = vec![0.0; FRAME];
        crate::montage::fft(&mut re, &mut im);
        let power: Vec<f32> = (0..BINS).map(|k| ((re[k] * re[k] + im[k] * im[k]) / 131072.0).max(1e-10) as f32).collect();
        let mut spread = power.clone();
        for (b, w) in power.windows(3).enumerate() {
            spread[b] = w[0].max(w[1]).max(w[2]);
        }
        count += 1;
        for lag in SPREAD_BACK {
            let e = &mut hist[slot(count, lag)].spread;
            e.iter_mut().zip(&spread).for_each(|(a, b)| *a = a.max(*b));
        }
        hist[slot(count, 0)] = Frame { power, spread };
        if count <= PEAK_LAG {
            continue;
        }
        let frame = (count - 1 - PEAK_LAG) as u32;
        let cand = &hist[slot(count, PEAK_LAG)].power;
        let refr = &hist[slot(count, REFERENCE_LAG)].spread;
        let near: Vec<&Vec<f32>> = TIME_NEIGHBOURS.iter().flat_map(|d| [REFERENCE_LAG - d, REFERENCE_LAG + d]).map(|l| &hist[slot(count, l)].spread).collect();
        for bin in 10..=1014 {
            let p = cand[bin];
            let below = bin - 1;
            if p < MIN_PEAK || p < refr[below] {
                continue;
            }
            if !FREQ_NEIGHBOURS.iter().all(|d| p > refr[below - d] && p > refr[below + d]) || !near.iter().all(|f| p > f[below]) {
                continue;
            }
            let m = log_mag(p);
            let (lo, hi) = (log_mag(cand[bin - 1]), log_mag(cand[bin + 1]));
            let offset = (hi - lo) * 32.0 / (m * 2.0 - lo - hi);
            let refined = (bin as i32 * 64 + offset as i32) as u16;
            if let Some(b) = band(refined as u32 * 125 / 1024) {
                out.entry(b).or_default().push((frame, m as u16, refined));
            }
        }
    }
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
        }
    }
    !c
}

fn base64(data: &[u8]) -> String {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            s.push(if i <= c.len() { A[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    s
}

/// The binary signature Shazam reads, for `samples` 16 kHz samples.
pub fn encode(peaks: &Peaks, samples: u32) -> Vec<u8> {
    let mut body = vec![];
    for (b, list) in peaks {
        let mut enc = vec![];
        let mut prev = 0;
        for &(frame, mag, bin) in list {
            let mut delta = frame - prev;
            if delta >= 255 {
                enc.push(255);
                enc.extend(frame.to_le_bytes());
                delta = 0;
            }
            enc.push(delta as u8);
            enc.extend(mag.to_le_bytes());
            enc.extend(bin.to_le_bytes());
            prev = frame;
        }
        body.extend((0x6003_0040u32 + *b as u32).to_le_bytes());
        body.extend((enc.len() as u32).to_le_bytes());
        body.extend(enc);
        body.resize(body.len().next_multiple_of(4), 0);
    }
    let len = (body.len() + 8) as u32;
    // 3 = 16 kHz; the count is padded by 240 ms as the app does
    let words = [0xcafe_2580, 0, len, 0x9411_9c00, 0, 0, 0, 3 << 27, 0, 0, samples + SR * 24 / 100, 0x007c_0000, 0x4000_0000, len];
    let mut sig: Vec<u8> = words.iter().flat_map(|w: &u32| w.to_le_bytes()).collect();
    sig.extend(body);
    let crc = crc32(&sig[8..]);
    sig[4..8].copy_from_slice(&crc.to_le_bytes());
    sig
}

fn uuid() -> String {
    use std::hash::{BuildHasher, Hasher};
    let r = |salt: u64| {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(salt);
        h.finish()
    };
    let (a, b) = (r(1), r(2));
    format!("{:08X}-{:04X}-4{:03X}-{:04X}-{:012X}", a >> 32, (a >> 16) & 0xffff, a & 0xfff, 0x8000 | (b >> 48) & 0x3fff, b & 0xffff_ffff_ffff)
}

pub struct Found {
    pub artist: String,
    pub title: String,
    /// where the listened stretch starts inside the song, seconds
    pub offset: f64,
    pub url: String,
}

/// Shazam's answer: the song, or None when nothing matched.
pub fn parse(json: &str) -> Result<Option<Found>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("Shazam answered something else: {e}"))?;
    let Some(t) = v.get("track") else { return Ok(None) };
    let s = |k: &str| t[k].as_str().unwrap_or_default().to_string();
    Ok(Some(Found { artist: s("subtitle"), title: s("title"), offset: v["matches"][0]["offset"].as_f64().unwrap_or(0.0), url: s("url") }))
}

fn ask(samples: &[i16]) -> Result<Option<Found>, String> {
    let sig = encode(&peaks(samples), samples.len() as u32);
    let ms = (samples.len() as f32 / SR as f32 * 1000.0) as u64;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis() as u64;
    let body = serde_json::json!({"timezone": "Europe/Paris", "signature": {"uri": format!("data:audio/vnd.shazam.sig;base64,{}", base64(&sig)), "samplems": ms},
        "timestamp": now, "context": {}, "geolocation": {}});
    let url = format!("https://amp.shazam.com/discovery/v5/en/US/iphone/-/tag/{}/{}?sync=true&webv3=true&sampling=true&connected=&shazamapiversion=v3&sharehub=true&hubv5minorversion=v5.1&hidelb=true&video=v3", uuid(), uuid());
    // only an app user agent reaches the API; a browser one gets the website
    let out = run(None, "curl", &["-fsS", "--compressed", "--max-time", "20", "-X", "POST", "-H", "Content-Type: application/json", "-H", "X-Shazam-Platform: IPHONE",
        "-H", "X-Shazam-AppVersion: 14.1.0", "-A", "Dalvik/2.1.0 (Linux; U; Android 6.0.1; SM-G920F Build/MMB29K)", "--data-binary", &body.to_string(), &url])
        .map_err(|e| format!("could not reach Shazam: {e}"))?;
    parse(&out)
}

/// The song playing in `file` (any audio or video): 12 s from `at`, else from the middle, a quarter and three quarters in.
pub fn recognize(file: &Path, at: Option<f64>) -> Result<Option<Found>, String> {
    let dur = crate::fetch::duration(file).unwrap_or(0.0);
    let tries: Vec<f64> = match at {
        Some(a) => vec![a],
        None => [0.5, 0.25, 0.75].iter().map(|k| (dur * k - 6.0).max(0.0)).collect(),
    };
    for (i, s) in tries.iter().enumerate() {
        if i > 0 && dur < 14.0 {
            break; // a short clip was heard whole the first time
        }
        let raw = run_bytes("ffmpeg", &["-v", "error", "-ss", &format!("{s:.2}"), "-t", "12", "-i", &file.to_string_lossy(), "-vn", "-ac", "1", "-ar", "16000", "-f", "s16le", "-"])?;
        let pcm: Vec<i16> = raw.as_chunks::<2>().0.iter().map(|b| i16::from_le_bytes(*b)).collect();
        if pcm.len() < SR as usize * 3 {
            continue;
        }
        if let Some(mut f) = ask(&pcm)? {
            // the answer is where this stretch sits in the song; report where the file's own start would be
            f.offset -= s;
            return Ok(Some(f));
        }
    }
    Ok(None)
}

/// `edits song <file|link|ID> [AT]`: the song playing there, ready as a --music title.
pub fn cmd(args: &[String]) -> Result<(), String> {
    let file = crate::check::footage(&args[0])?;
    let at = args.get(1).map(|a| crate::check::secs(a).ok_or(format!("not a time: {a}"))).transpose()?;
    let f = recognize(&file, at)?.ok_or("Shazam did not know this song (try another time: edits song FILE 0:30)")?;
    let name = format!("{} - {}", f.artist, f.title);
    println!("{name}");
    if f.offset.abs() > 0.5 {
        println!("the clip starts {} into the song", crate::util::mmss(f.offset));
    }
    if !f.url.is_empty() {
        println!("{}", f.url);
    }
    println!("\nuse it: --music \"{name}\"");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_checksum_and_base64() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
    }

    #[test]
    fn header_matches_shazamio_core() {
        let sig = encode(&Peaks::new(), 128000);
        let w = |o: usize| u32::from_le_bytes(sig[o..o + 4].try_into().unwrap());
        assert_eq!((w(0), w(12), w(28), w(40), w(44)), (0xcafe2580, 0x94119c00, 3 << 27, 128000 + 3840, (15 << 19) + 0x40000));
        assert_eq!(w(8), 8);
        assert_eq!(w(4), crc32(&sig[8..]));
        // a late peak is escaped with its absolute frame
        let mut p = Peaks::new();
        p.insert(0, vec![(0, 7, 9), (300, 7, 9)]);
        let sig = encode(&p, 1);
        assert_eq!(&sig[64..69], &[0, 7, 0, 9, 0]);
        assert_eq!(&sig[69..74], &[255, 44, 1, 0, 0]);
    }

    #[test]
    fn a_tone_peaks_at_its_frequency_in_its_band() {
        // 1 kHz with a slow tremolo so peaks rise above their recent past
        let s: Vec<i16> = (0..SR as usize * 4).map(|i| {
            let t = i as f64 / SR as f64;
            (8000.0 * (0.6 + 0.4 * (2.0 * std::f64::consts::PI * 1.5 * t).sin()) * (2.0 * std::f64::consts::PI * 1000.0 * t).sin()) as i16
        }).collect();
        let p = peaks(&s);
        let found = p.get(&1).expect("a peak in the 520-1450 Hz band");
        assert!(found.iter().all(|&(_, _, bin)| ((bin as u32 * 125 / 1024) as i32 - 1000).abs() < 20), "{found:?}");
    }

    #[test]
    fn matches_shazamio_core_byte_for_byte() {
        // the same synthetic chord through shazamio-core 2.1.0's own fingerprint_samples gave tests/data/shazam-golden.uri
        let sr = 16000usize;
        let s: Vec<i16> = (0..sr * 6).map(|i| {
            let t = i as f64 / sr as f64;
            let env = 0.35 + 0.65 * (-6.0 * (t % 0.5)).exp();
            let f = |hz: f64| (2.0 * std::f64::consts::PI * hz * t).sin();
            (6000.0 * env * (f(440.0) + 0.6 * f(660.0) + 0.4 * f(990.0) + 0.3 * f(450.0 + 240.0 * t))) as i16
        }).collect();
        let got = format!("data:audio/vnd.shazam.sig;base64,{}", base64(&encode(&peaks(&s), s.len() as u32)));
        assert_eq!(got, include_str!("../tests/data/shazam-golden.uri").trim());
    }

    #[test]
    fn reads_an_answer() {
        let f = parse(r#"{"matches":[{"offset":42.5}],"track":{"title":"Umrumdışı","subtitle":"Mabel Matiz","url":"https://www.shazam.com/track/1"}}"#).unwrap().unwrap();
        assert_eq!((f.artist.as_str(), f.title.as_str(), f.offset), ("Mabel Matiz", "Umrumdışı", 42.5));
        assert!(parse(r#"{"matches":[]}"#).unwrap().is_none());
        assert!(uuid().len() == 36 && uuid() != uuid());
    }
}
