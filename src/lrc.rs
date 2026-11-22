//! A song's real words, timed line by line, from LRCLIB (free, no key): whisper mishears sung words, the record's
//! lyrics do not. The lines are set on the song's own clock where whisper's words agree with them.
use crate::transcribe::Word;
use crate::util::run;
use std::path::Path;

/// "[01:02.50] words" lines as (seconds, words); several time tags on one line give it at each; tags like [ar:] and
/// empty lines are left out.
pub fn parse(lrc: &str) -> Vec<(f64, String)> {
    let mut out = vec![];
    for line in lrc.lines() {
        let mut rest = line.trim();
        let mut times = vec![];
        while let Some(r) = rest.strip_prefix('[') {
            let Some((tag, after)) = r.split_once(']') else { break };
            if let Some((m, s)) = tag.split_once(':') {
                if let (Ok(m), Ok(s)) = (m.parse::<f64>(), s.parse::<f64>()) {
                    times.push(m * 60.0 + s);
                }
            }
            rest = after.trim_start();
        }
        if !rest.is_empty() {
            out.extend(times.into_iter().map(|t| (t, rest.to_string())));
        }
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

fn norm(w: &str) -> String {
    w.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// Heard as the same word: equal, or the same first three letters (whisper drops endings).
fn like(a: &str, b: &str) -> bool {
    a == b || (a.chars().count() >= 3 && b.chars().count() >= 3 && a.chars().take(3).eq(b.chars().take(3)))
}

/// How far whisper's clock is from the lyrics' (seconds to add to a line's time): where a line's first words were
/// heard, the shift most lines agree on within 0.7 s. None when fewer than three lines agree.
pub fn offset(lines: &[(f64, String)], words: &[Word]) -> Option<f64> {
    let heard: Vec<String> = words.iter().map(|w| norm(&w.t)).collect();
    let mut shifts = vec![];
    for (t, text) in lines {
        let first: Vec<String> = text.split_whitespace().map(norm).filter(|w| !w.is_empty()).take(3).collect();
        if first.len() < 2 {
            continue;
        }
        for (i, w) in words.iter().enumerate() {
            let hits = first.iter().enumerate().filter(|(k, f)| heard.get(i + k).is_some_and(|h| like(h, f))).count();
            if hits >= 2 {
                shifts.push(w.s - t);
            }
        }
    }
    let mut best: Vec<f64> = vec![];
    for &o in &shifts {
        let near: Vec<f64> = shifts.iter().copied().filter(|x| (x - o).abs() <= 0.7).collect();
        if near.len() > best.len() {
            best = near;
        }
    }
    if best.len() < 3 {
        return None;
    }
    best.sort_by(f64::total_cmp);
    Some(best[best.len() / 2])
}

/// The lines as words on the song's clock: each line's words spread from its time, at most half a second each,
/// ending before the next line. A line's last word ends in "\n", so the words on screen break where its lines do.
pub fn words(lines: &[(f64, String)], shift: f64) -> Vec<Word> {
    let mut out = vec![];
    for (j, (t, text)) in lines.iter().enumerate() {
        let ws: Vec<&str> = text.split_whitespace().collect();
        let next = lines.get(j + 1).map_or(t + 5.0, |n| n.0);
        let d = ((next - t - 0.1) / ws.len().max(1) as f64).clamp(0.05, 0.5);
        out.extend(ws.iter().enumerate().map(|(k, w)| {
            let s = t + shift + k as f64 * d;
            Word { s, e: s + d * 0.9, t: if k + 1 == ws.len() { format!("{w}\n") } else { w.to_string() } }
        }));
    }
    out
}

/// The song's synced lyrics from LRCLIB as (seconds, line) and the record's length, cached in `dir`; None when it
/// has none (an empty answer is remembered, a failed request is not).
pub fn find(title: &str, dir: &Path) -> Option<(Vec<(f64, String)>, f64)> {
    let file = dir.join("lrclib.json");
    let saved = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
    let v = match saved {
        Some(v) => v,
        None => {
            let key = crate::hook::clean(title);
            let out = run(None, "curl", &["-fsS", "--max-time", "15", "-A", "EditsMaker (https://github.com/OnurByte/EditsMaker)", "-G",
                "--data-urlencode", &format!("q={key}"), "https://lrclib.net/api/search"]).ok()?;
            let all: Vec<serde_json::Value> = serde_json::from_str(&out).ok()?;
            let hit = all.into_iter().find(|x| x["syncedLyrics"].as_str().is_some_and(|l| !l.is_empty()) && crate::hook::same_song(&key, x["trackName"].as_str().unwrap_or_default()))
                .unwrap_or(serde_json::Value::Null);
            let _ = std::fs::write(&file, hit.to_string());
            hit
        }
    };
    let lines = parse(v["syncedLyrics"].as_str()?);
    (lines.len() >= 3).then(|| (lines, v["duration"].as_f64().unwrap_or(0.0)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(s: f64, t: &str) -> Word {
        Word { s, e: s + 0.3, t: t.into() }
    }

    #[test]
    fn reads_synced_lyrics() {
        let l = parse("[ar:Mabel Matiz]\n[00:12.50] Umrumdışı bu dertler\n[00:15.00]\n[01:02.00][02:10.25] nakarat burada\n");
        assert_eq!(l, vec![(12.5, "Umrumdışı bu dertler".into()), (62.0, "nakarat burada".into()), (130.25, "nakarat burada".into())]);
    }

    #[test]
    fn whisper_puts_the_lines_on_the_song_clock() {
        let lines = vec![(10.0, "Umrumdışı bu dertler".to_string()), (14.0, "beni benden aldı".into()), (18.0, "gece uzun sürer".into()), (22.0, "tek".into())];
        // whisper heard the song 3 s later, with its endings wrong
        let heard = vec![w(13.0, "umrumdış"), w(13.4, "bu"), w(13.7, "dertleri"), w(17.0, "beni"), w(17.3, "benden"), w(21.0, "gece"), w(21.4, "uzun"), w(30.0, "gece"), w(30.3, "uzun")];
        let off = offset(&lines, &heard).unwrap();
        assert!((off - 3.0).abs() < 0.01, "{off}");
        assert_eq!(offset(&lines, &heard[..5]), None);
        let ws = words(&lines, off);
        assert_eq!(ws.len(), 10);
        assert!((ws[0].s - 13.0).abs() < 1e-9 && ws[0].t == "Umrumdışı");
        // half a second a word at most, and never into the next line
        assert!((ws[1].s - 13.5).abs() < 1e-9);
        assert!(ws[2].e < 17.0);
    }
}
