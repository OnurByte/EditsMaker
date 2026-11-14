//! Words with times. The site's own captions when it has them (instant), whisper otherwise.
use crate::util::{home, run};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct Word {
    pub s: f64,
    pub e: f64,
    pub t: String,
}

pub struct Line {
    pub s: f64,
    pub e: f64,
    pub text: String,
}

#[derive(Debug, Default, PartialEq)]
pub struct Transcript {
    pub words: Vec<Word>,
    /// seconds where laughter or applause was heard instead of words
    pub laughs: Vec<f64>,
    /// "captions" or "whisper"
    pub from: &'static str,
}

fn is_laugh(marker: &str) -> bool {
    let m = marker.to_lowercase();
    ["gül", "kahka", "laugh", "alkış", "applau"].iter().any(|k| m.contains(k))
}

/// $EDITS_WHISPER_MODEL, else the largest ggml model in the usual folders.
pub fn model() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("EDITS_WHISPER_MODEL") {
        return Some(PathBuf::from(p)).filter(|p| p.is_file());
    }
    let share = home().join(".local/share");
    let mut found: Vec<(u64, PathBuf)> = ["edits/models", "tehom/models", "whisper"].iter()
        .filter_map(|d| std::fs::read_dir(share.join(d)).ok())
        .flatten().flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("ggml-") && n.to_string_lossy().ends_with(".bin")))
        .filter_map(|p| Some((p.metadata().ok()?.len(), p)))
        .collect();
    found.sort();
    found.pop().map(|x| x.1)
}

/// Captions downloaded next to the audio if they are usable, else whisper (cached as a.json).
/// `spoken` is the language the site says the video is in: an auto-dubbed video carries one caption track per dub.
pub fn transcript(work: &Path, wav: &Path, lang: &str, spoken: &str) -> Result<Transcript, String> {
    let mut tracks: Vec<PathBuf> = std::fs::read_dir(work).into_iter().flatten().flatten().map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with("-orig.json3")).collect();
    tracks.sort();
    let subs = pick_track(&tracks, spoken, lang);
    if let Some(t) = subs.and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| parse_json3(&s)) {
        return Ok(t);
    }
    let json = wav.with_extension("json");
    if !json.is_file() {
        let model = model().ok_or("no whisper model found; run `edits doctor`")?;
        // measured on 6 cores: one thread per physical core and greedy decoding is ~2x faster than the defaults
        // with the same text; more threads or two runs at once are slower (memory-bound)
        let threads = (std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) / 2).clamp(2, 8).to_string();
        run(None, "whisper-cli", &[
            "-m", &model.to_string_lossy(), "-l", lang, "-ml", "1", "-sow", "-oj", "-np",
            "-t", &threads, "-bs", "1", "-bo", "1",
            "--prompt", "Sansürsüz, olduğu gibi yaz: lan, amk, aq, siktir, ulan, hassiktir, mal, gerizekalı.",
            "-of", &wav.with_extension("").to_string_lossy(), &wav.to_string_lossy(),
        ])?;
    }
    parse_whisper(&std::fs::read_to_string(&json).map_err(|e| format!("{}: {e}", json.display()))?)
}

/// The track in the spoken language, else in the language asked for, else the only one there is.
fn pick_track(tracks: &[PathBuf], spoken: &str, lang: &str) -> Option<PathBuf> {
    let named = |l: &str| tracks.iter().find(|p| !l.is_empty() && p.to_string_lossy().ends_with(&format!(".{l}-orig.json3")));
    named(spoken).or_else(|| named(lang)).or(if tracks.len() == 1 { tracks.first() } else { None }).cloned()
}

pub fn parse_whisper(json: &str) -> Result<Transcript, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("whisper json: {e}"))?;
    let mut t = Transcript { from: "whisper", ..Default::default() };
    for w in v["transcription"].as_array().ok_or("whisper json has no transcription")? {
        let (Some(text), Some(s), Some(e)) = (w["text"].as_str().map(str::trim), w["offsets"]["from"].as_f64(), w["offsets"]["to"].as_f64()) else { continue };
        // whisper emits markers like [MÜZİK] or (gülüşmeler) as "words"
        if text.starts_with(['[', '(']) {
            if is_laugh(text) {
                t.laughs.push(s / 1000.0);
            }
        } else if !text.is_empty() {
            t.words.push(Word { s: s / 1000.0, e: e / 1000.0, t: text.to_string() });
        }
    }
    Ok(t)
}

/// YouTube json3 captions. None when they are empty or have profanity masked as "[ __ ]":
/// a censored punchline is worthless here, so whisper does that video instead.
pub fn parse_json3(json: &str) -> Option<Transcript> {
    let v: Value = serde_json::from_str(json).ok()?;
    let mut t = Transcript { from: "captions", ..Default::default() };
    for ev in v["events"].as_array()? {
        let (Some(start), Some(segs)) = (ev["tStartMs"].as_f64(), ev["segs"].as_array()) else { continue };
        let end = (start + ev["dDurationMs"].as_f64().unwrap_or(2000.0)) / 1000.0;
        for seg in segs {
            let text = seg["utf8"].as_str().unwrap_or("").trim();
            let s = (start + seg["tOffsetMs"].as_f64().unwrap_or(0.0)) / 1000.0;
            if text.contains("__") {
                return None;
            }
            if text.starts_with('[') {
                if is_laugh(text) {
                    t.laughs.push(s);
                }
            } else if !text.is_empty() {
                // the track gives starts only: a word ends where the next begins (fixed below), at most 1.2 s on
                t.words.push(Word { s, e: end.clamp(s + 0.1, s + 1.2), t: text.to_string() });
            }
        }
    }
    for i in 0..t.words.len().saturating_sub(1) {
        let next = t.words[i + 1].s;
        if next > t.words[i].s {
            t.words[i].e = t.words[i].e.min(next);
        }
    }
    (!t.words.is_empty()).then_some(t)
}

/// Words -> sentence-ish lines: break on a pause, on end punctuation, or after 14 words.
pub fn lines(words: &[Word]) -> Vec<Line> {
    let mut out: Vec<Line> = vec![];
    let mut count = 0;
    for (i, w) in words.iter().enumerate() {
        let fresh = i == 0 || count >= 14 || w.s - words[i - 1].e > 0.6 || words[i - 1].t.ends_with(['.', '?', '!']);
        if fresh {
            out.push(Line { s: w.s, e: w.e, text: w.t.clone() });
            count = 1;
        } else if let Some(l) = out.last_mut() {
            l.e = w.e;
            l.text.push(' ');
            l.text.push_str(&w.t);
            count += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: f64, e: f64, t: &str) -> Word {
        Word { s, e, t: t.into() }
    }

    #[test]
    fn whisper_words_and_laughter() {
        let j = r#"{"transcription":[
            {"offsets":{"from":0,"to":500},"text":" Gençlere"},
            {"offsets":{"from":500,"to":900},"text":" [MÜZİK]"},
            {"offsets":{"from":900,"to":1400},"text":" lan."},
            {"offsets":{"from":1400,"to":2000},"text":" (Gülüşmeler)"},
            {"offsets":{"from":2000,"to":2000},"text":""}]}"#;
        let t = parse_whisper(j).unwrap();
        assert_eq!(t.words, vec![w(0.0, 0.5, "Gençlere"), w(0.9, 1.4, "lan.")]);
        assert_eq!(t.laughs, vec![1.4]);
        assert!(parse_whisper("{}").is_err());
    }

    #[test]
    fn youtube_captions() {
        let j = r#"{"events":[
            {"tStartMs":0,"dDurationMs":900000,"id":1},
            {"tStartMs":2500,"dDurationMs":2900,"segs":[{"utf8":"güvenlik"},{"utf8":" reposuyla","tOffsetMs":300},{"utf8":" denedim.","tOffsetMs":800}]},
            {"tStartMs":3890,"segs":[{"utf8":"\n"}]},
            {"tStartMs":6000,"dDurationMs":1000,"segs":[{"utf8":"[Gülüşmeler]"}]},
            {"tStartMs":9000,"dDurationMs":500,"segs":[{"utf8":"Sonra"}]}]}"#;
        let t = parse_json3(j).unwrap();
        assert_eq!(t.words, vec![w(2.5, 2.8, "güvenlik"), w(2.8, 3.3, "reposuyla"), w(3.3, 4.5, "denedim."), w(9.0, 9.5, "Sonra")]);
        assert_eq!((t.laughs, t.from), (vec![6.0], "captions"));
        // masked profanity or nothing spoken -> not usable
        assert!(parse_json3(r#"{"events":[{"tStartMs":0,"segs":[{"utf8":"ne diyon"},{"utf8":" [ __ ]","tOffsetMs":400}]}]}"#).is_none());
        assert!(parse_json3(r#"{"events":[{"tStartMs":0,"segs":[{"utf8":"[Müzik]"}]}]}"#).is_none());
        assert!(parse_json3("not json").is_none());
    }

    #[test]
    fn dubbed_videos_use_the_spoken_track() {
        let t = |names: &[&str]| names.iter().map(PathBuf::from).collect::<Vec<_>>();
        let both = t(&["w/src.en-orig.json3", "w/src.tr-orig.json3"]);
        assert_eq!(pick_track(&both, "tr", "en"), Some(PathBuf::from("w/src.tr-orig.json3")));
        assert_eq!(pick_track(&both, "", "en"), Some(PathBuf::from("w/src.en-orig.json3")));
        assert_eq!(pick_track(&both, "", "de"), None); // no way to tell: whisper decides
        assert_eq!(pick_track(&t(&["w/src.ja-orig.json3"]), "", "tr"), Some(PathBuf::from("w/src.ja-orig.json3")));
        assert_eq!(pick_track(&[], "tr", "tr"), None);
    }

    #[test]
    fn lines_break_on_pause_and_punctuation() {
        let l = lines(&[w(0.0, 0.3, "bir"), w(0.4, 0.7, "kapanın."), w(0.8, 1.1, "bakalım"), w(3.0, 3.3, "sonra")]);
        assert_eq!(l.iter().map(|x| x.text.as_str()).collect::<Vec<_>>(), vec!["bir kapanın.", "bakalım", "sonra"]);
        assert_eq!((l[0].s, l[0].e), (0.0, 0.7));
    }
}
