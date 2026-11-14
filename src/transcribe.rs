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

pub fn wav(video: &Path, out: &Path) -> Result<(), String> {
    if out.is_file() {
        return Ok(());
    }
    run(None, "ffmpeg", &["-v", "error", "-y", "-i", &video.to_string_lossy(), "-vn", "-ar", "16000", "-ac", "1", &out.to_string_lossy()]).map(|_| ())
}

/// Word-level transcript, cached next to the wav. The prompt keeps whisper from softening profanity.
pub fn words(wav: &Path, lang: &str) -> Result<Vec<Word>, String> {
    let base = wav.with_extension("");
    let json = wav.with_extension("json");
    if !json.is_file() {
        let model = model().ok_or("no whisper model found; run `edits doctor`")?;
        run(None, "whisper-cli", &[
            "-m", &model.to_string_lossy(), "-l", lang, "-ml", "1", "-sow", "-oj", "-np",
            "--prompt", "Sansürsüz, olduğu gibi yaz: lan, amk, aq, siktir, ulan, hassiktir, mal, gerizekalı.",
            "-of", &base.to_string_lossy(), &wav.to_string_lossy(),
        ])?;
    }
    let text = std::fs::read_to_string(&json).map_err(|e| format!("{}: {e}", json.display()))?;
    parse(&text)
}

pub fn parse(json: &str) -> Result<Vec<Word>, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("whisper json: {e}"))?;
    Ok(v["transcription"].as_array().ok_or("whisper json has no transcription")?.iter().filter_map(|w| {
        let t = w["text"].as_str()?.trim();
        // whisper emits markers like [MÜZİK] or (gülüşmeler) as "words"
        if t.is_empty() || t.starts_with('[') || t.starts_with('(') {
            return None;
        }
        Some(Word { s: w["offsets"]["from"].as_f64()? / 1000.0, e: w["offsets"]["to"].as_f64()? / 1000.0, t: t.to_string() })
    }).collect())
}

/// Seconds where whisper heard laughter or applause instead of words: "(Gülüşmeler)", "[Kahkaha]", "[LAUGHTER]".
pub fn laughs(wav: &Path) -> Vec<f64> {
    let v: Value = std::fs::read_to_string(wav.with_extension("json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    v["transcription"].as_array().map(|a| a.iter().filter_map(|w| {
        let t = w["text"].as_str()?.to_lowercase();
        let marker = t.trim().starts_with(['[', '(']) && ["gül", "kahka", "laugh", "alkış", "applau"].iter().any(|k| t.contains(k));
        marker.then(|| w["offsets"]["from"].as_f64()).flatten().map(|ms| ms / 1000.0)
    }).collect()).unwrap_or_default()
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

    #[test]
    fn parses_whisper_words_and_drops_markers() {
        let j = r#"{"transcription":[
            {"offsets":{"from":0,"to":500},"text":" Gençlere"},
            {"offsets":{"from":500,"to":900},"text":" [MÜZİK]"},
            {"offsets":{"from":900,"to":1400},"text":" lan."},
            {"offsets":{"from":1400,"to":1400},"text":""}]}"#;
        let w = parse(j).unwrap();
        assert_eq!(w, vec![Word { s: 0.0, e: 0.5, t: "Gençlere".into() }, Word { s: 0.9, e: 1.4, t: "lan.".into() }]);
        assert!(parse("{}").is_err());
    }

    #[test]
    fn lines_break_on_pause_and_punctuation() {
        let w = |s: f64, t: &str| Word { s, e: s + 0.3, t: t.into() };
        let l = lines(&[w(0.0, "bir"), w(0.4, "kapanın."), w(0.8, "bakalım"), w(3.0, "sonra")]);
        assert_eq!(l.iter().map(|x| x.text.as_str()).collect::<Vec<_>>(), vec!["bir kapanın.", "bakalım", "sonra"]);
        assert_eq!((l[0].s, l[0].e), (0.0, 0.7));
    }
}
