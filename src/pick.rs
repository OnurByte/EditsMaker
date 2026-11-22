//! Chooses the moments: viewer signals + transcript go to an LLM with the taste file; signals alone are the fallback.
use crate::signals::{peaks, Hit};
use crate::transcribe::{Line, Word};
use crate::util::{home, mmss};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Moment {
    pub start: f64,
    pub end: f64,
    #[serde(default)]
    pub punch: f64,
    /// how good, 0-10 from the model or the signal strength without one; ranks moments across videos
    #[serde(default)]
    pub score: f64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub why: String,
}

pub fn taste_path() -> PathBuf {
    home().join(".config/edits/taste.md")
}

/// The user's taste file; written from the built-in one on first use.
pub fn taste() -> String {
    let p = taste_path();
    if let Ok(t) = std::fs::read_to_string(&p) {
        return t;
    }
    let default = include_str!("taste.md");
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
        let _ = std::fs::write(&p, default);
    }
    default.to_string()
}

pub fn prompt(taste: &str, title: &str, lines: &[Line], curve: &[f64], hits: &[Hit], n: usize) -> String {
    use crate::lang::prompt as part;
    let mut p = part("moments", "head", &[("n", n.to_string()), ("taste", taste.to_string()), ("title", title.to_string())]);
    for l in lines {
        let (a, b) = (l.s as usize, (l.e.ceil() as usize).max(l.s as usize + 1));
        let heat = curve.iter().skip(a).take(b - a).cloned().fold(f64::MIN, f64::max);
        p.push_str(&format!("[{:.1}-{:.1}] {}", l.s, l.e, l.text));
        if heat > 1.0 {
            p.push_str(&part("moments", "heat", &[("heat", format!("{heat:.1}"))]));
        }
        for h in hits.iter().filter(|h| h.t >= l.s - 2.0 && h.t < l.e) {
            p.push_str(&part("moments", "comment", &[("at", mmss(h.t)), ("text", h.text.chars().take(90).collect())]));
        }
        p.push('\n');
    }
    p.push_str(&part("moments", "tail", &[("n", n.to_string())]));
    p
}

/// Pulls the JSON array out of whatever the model wrapped around it.
pub fn parse(reply: &str) -> Result<Vec<Moment>, String> {
    let (a, b) = (reply.find('[').ok_or("no JSON array in the model's reply")?, reply.rfind(']').ok_or("no JSON array in the model's reply")?);
    if b < a {
        return Err("no JSON array in the model's reply".into());
    }
    serde_json::from_str(&reply[a..=b]).map_err(|e| format!("model reply is not valid JSON: {e}"))
}

/// Makes moments safe to render: inside the video, 4-75 s long, snapped to line edges, no overlaps.
pub fn refine(mut moments: Vec<Moment>, lines: &[Line], dur: f64) -> Vec<Moment> {
    let mut out: Vec<Moment> = vec![];
    for m in moments.iter_mut() {
        if !(m.start.is_finite() && m.end.is_finite()) {
            continue;
        }
        // start on the line that contains the start, end on the line that contains the end
        if let Some(l) = lines.iter().rev().find(|l| l.s <= m.start + 0.3 && l.e > m.start) {
            m.start = l.s;
        }
        if let Some(l) = lines.iter().find(|l| l.e >= m.end - 0.3 && l.s < m.end) {
            m.end = l.e;
        }
        m.start = (m.start - 0.15).clamp(0.0, dur);
        m.end = (m.end + 0.5).min(dur).min(m.start + 75.0); // half a second of reaction after the last word
        if m.end - m.start < 4.0 || out.iter().any(|o| m.start < o.end && o.start < m.end) {
            continue;
        }
        if !(m.punch > m.start && m.punch < m.end) {
            m.punch = (m.end - 2.0).max(m.start);
        }
        out.push(m.clone());
    }
    out
}

/// No model: cut around the strongest viewer signals.
pub fn from_signals(curve: &[f64], n: usize) -> Vec<Moment> {
    peaks(curve, n, 30).into_iter().filter(|&p| curve[p] > 0.0).map(|p| Moment {
        start: p as f64 - 10.0,
        end: p as f64 + 6.0,
        punch: p as f64,
        score: curve[p],
        title: String::new(),
        why: format!("izleyici sinyali {:.1}", curve[p]),
    }).collect()
}

/// Sentences as (first word, last word) indexes: a sentence ends on . ? ! … or a pause over 1.2 s.
pub fn sentences(words: &[Word]) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut first = 0;
    for i in 0..words.len() {
        let end = words[i].t.ends_with(['.', '?', '!', '…']) || words.get(i + 1).is_none_or(|n| n.s - words[i].e > 1.2);
        if end {
            out.push((first, i));
            first = i + 1;
        }
    }
    out
}

/// Runs of whole sentences lasting `min`..`max` seconds, best first, none overlapping: a line that starts and
/// finishes, to put over an edit. Ranked by viewer interest; a closing ! or ? counts a little.
pub fn passages(words: &[Word], curve: &[f64], min: f64, max: f64) -> Vec<Moment> {
    let sent = sentences(words);
    let mut cands: Vec<Moment> = vec![];
    for i in 0..sent.len() {
        for j in i..sent.len() {
            let (s, e) = (words[sent[i].0].s, words[sent[j].1].e);
            if e - s > max {
                break;
            }
            if e - s >= min {
                let (a, b) = (s as usize, (e.ceil() as usize).min(curve.len()));
                let interest = if b > a { curve[a..b].iter().sum::<f64>() / (b - a) as f64 } else { 0.0 };
                let strong = if words[sent[j].1].t.ends_with(['!', '?']) { 0.3 } else { 0.0 };
                cands.push(Moment { start: (s - 0.05).max(0.0), end: e + 0.25, punch: words[sent[j].0].s, score: interest + strong, title: String::new(), why: String::new() });
            }
        }
    }
    cands.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut out: Vec<Moment> = vec![];
    for m in cands {
        if out.iter().all(|o| m.end <= o.start || m.start >= o.end) {
            out.push(m);
        }
    }
    out
}

/// The `n` passages that work best over a music edit. The model judges which lines stand on their own and carry
/// feeling; without one, the ranking stays.
pub fn choose_passages(cands: &[(String, Moment, String)], n: usize, use_llm: bool) -> Vec<usize> {
    let fallback: Vec<usize> = (0..cands.len().min(n)).collect();
    if !use_llm || cands.len() <= n {
        return fallback;
    }
    use crate::lang::prompt as part;
    let mut p = part("passages", "head", &[("n", n.to_string())]);
    for (i, (title, m, text)) in cands.iter().enumerate() {
        p.push_str(&part("passages", "item", &[("i", i.to_string()), ("secs", format!("{:.0}", m.end - m.start)), ("title", title.clone()), ("text", text.clone())]));
    }
    p.push_str(&part("passages", "tail", &[("n", n.to_string())]));
    let picked = crate::ai::ask(&p).ok().and_then(|r| {
        let (a, b) = (r.find('[')?, r.rfind(']')?);
        serde_json::from_str::<Vec<usize>>(r.get(a..=b)?).ok()
    });
    match picked {
        Some(v) => {
            let mut seen = vec![];
            v.into_iter().filter(|&i| i < cands.len()).for_each(|i| if !seen.contains(&i) { seen.push(i) });
            if seen.is_empty() { fallback } else { seen.into_iter().take(n).collect() }
        }
        None => {
            eprintln!("edits: the model did not choose; using viewer signals");
            fallback
        }
    }
}

pub fn pick(title: &str, lines: &[Line], curve: &[f64], hits: &[Hit], n: usize, dur: f64, use_llm: bool) -> Vec<Moment> {
    if use_llm && !lines.is_empty() {
        match crate::ai::ask(&prompt(&taste(), title, lines, curve, hits, n)).and_then(|r| parse(&r)) {
            Ok(m) => {
                let m = refine(m, lines, dur);
                if !m.is_empty() {
                    return m.into_iter().take(n).collect();
                }
                eprintln!("edits: the model picked nothing usable, falling back to viewer signals");
            }
            Err(e) => eprintln!("edits: {e}; falling back to viewer signals"),
        }
    }
    refine(from_signals(curve, n), lines, dur)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(s: f64, e: f64) -> Line {
        Line { s, e, text: "x".into() }
    }

    #[test]
    fn parses_a_chatty_reply() {
        let r = "Tabii! İşte anlar:\n```json\n[{\"start\": 12.5, \"end\": 30, \"punch\": 27, \"title\": \"lan\", \"why\": \"küfür\"}, {\"start\": 50, \"end\": 60}]\n```\nUmarım beğenirsin.";
        let m = parse(r).unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!((m[0].start, m[0].punch, m[0].title.as_str()), (12.5, 27.0, "lan"));
        assert_eq!(m[1].punch, 0.0);
        assert!(parse("hiç an yok").is_err());
        assert!(parse("] [").is_err());
    }

    #[test]
    fn refine_snaps_clamps_and_drops() {
        let lines = vec![line(10.0, 14.0), line(14.5, 20.0), line(40.0, 44.0)];
        let m = |start, end| Moment { start, end, punch: 0.0, score: 0.0, title: String::new(), why: String::new() };
        let out = refine(vec![m(12.0, 18.0), m(15.0, 19.0), m(41.0, 42.0), m(f64::NAN, 5.0), m(95.0, 300.0)], &lines, 100.0);
        // first: snapped to whole lines; second: overlaps the first; third: grows to its whole line;
        // fourth: not a number; fifth: no line there, clamped to the end of the video
        assert_eq!(out.len(), 3, "{out:?}");
        assert_eq!((out[0].start, out[0].end), (9.85, 20.5));
        assert_eq!(out[0].punch, 18.5);
        assert_eq!((out[1].start, out[1].end), (39.85, 44.5));
        assert_eq!((out[2].start, out[2].end), (94.85, 100.0));
        assert!(refine(vec![m(41.0, 42.0)], &[], 100.0).is_empty()); // one second, nothing to snap to
    }

    #[test]
    fn prompt_carries_signals() {
        let lines = vec![Line { s: 48.0, e: 52.0, text: "buda boynu bükük kalmasın".into() }];
        let hits = vec![Hit { t: 49.0, weight: 1.0, text: "0:49 gariban :D".into() }];
        let curve = vec![0.0; 49].into_iter().chain([2.5, 2.5, 0.0]).collect::<Vec<_>>();
        let p = prompt("ZEVK", "başlık", &lines, &curve, &hits, 3);
        assert!(p.contains("ZEVK") && p.contains("[48.0-52.0] buda boynu bükük kalmasın  🔥2.5  comment(0:49): \"0:49 gariban :D\""), "{p}");
    }

    #[test]
    fn passages_are_whole_sentences() {
        let w = |s: f64, t: &str| Word { s, e: s + 0.4, t: t.into() };
        let words = [
            w(0.0, "yani"), w(0.5, "aslında"), w(1.0, "bu."), // sentence 0: 0-1.4
            w(2.0, "Her"), w(2.5, "şey"), w(3.0, "bitti!"), // sentence 1: 2-3.4
            w(6.0, "sonra"), w(6.5, "dedim"), // pause after -> sentence 2: 6-6.9
            w(9.0, "neyse."),
        ];
        assert_eq!(sentences(&words), vec![(0, 2), (3, 5), (6, 7), (8, 8)]);
        let mut curve = vec![0.0; 10];
        curve[2] = 5.0;
        curve[3] = 5.0;
        let p = passages(&words, &curve, 1.0, 4.0);
        // the best one is the exclaimed sentence where viewers reacted; it starts and ends on sentence edges
        assert_eq!((p[0].start, p[0].end), (1.95, 3.65));
        assert!(p.iter().all(|m| m.end - m.start <= 4.0 + 0.3), "{p:?}");
        assert!(p.windows(2).all(|x| x[0].score >= x[1].score));
        assert!(passages(&words, &curve, 20.0, 30.0).is_empty());
    }

    #[test]
    fn choosing_without_a_model_keeps_the_ranking() {
        let m = Moment { start: 0.0, end: 5.0, punch: 0.0, score: 0.0, title: String::new(), why: String::new() };
        let c: Vec<(String, Moment, String)> = (0..5).map(|i| (format!("v{i}"), m.clone(), "x".into())).collect();
        assert_eq!(choose_passages(&c, 2, false), vec![0, 1]);
        assert_eq!(choose_passages(&c[..1], 2, true), vec![0]);
    }

    #[test]
    fn signal_fallback_needs_a_real_peak() {
        assert!(from_signals(&[0.0; 50], 3).is_empty());
        let mut c = vec![0.0; 100];
        c[40] = 3.0;
        let m = from_signals(&c, 3);
        assert_eq!((m.len(), m[0].start, m[0].end, m[0].punch), (1, 30.0, 46.0, 40.0));
    }
}
