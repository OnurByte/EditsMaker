//! Chooses the moments: viewer signals + transcript go to an LLM with the taste file; signals alone are the fallback.
use crate::signals::{peaks, Hit};
use crate::transcribe::Line;
use crate::util::{home, mmss, run_stdin};
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
    let mut p = format!(
        "Bir videodan en komik {n} anı seçeceksin. Önce zevk dosyasını oku, ona harfiyen uy.\n\n{taste}\n\n\
         # Video: {title}\n\n# Döküm\nHer satır: [başlangıç-bitiş saniye] metin. 🔥 = izleyici ilgisi (yüksek = çok tekrar izlenmiş).\n\n"
    );
    for l in lines {
        let (a, b) = (l.s as usize, (l.e.ceil() as usize).max(l.s as usize + 1));
        let heat = curve.iter().skip(a).take(b - a).cloned().fold(f64::MIN, f64::max);
        p.push_str(&format!("[{:.1}-{:.1}] {}", l.s, l.e, l.text));
        if heat > 1.0 {
            p.push_str(&format!("  🔥{heat:.1}"));
        }
        for h in hits.iter().filter(|h| h.t >= l.s - 2.0 && h.t < l.e) {
            p.push_str(&format!("  yorum({}): \"{}\"", mmss(h.t), h.text.chars().take(90).collect::<String>()));
        }
        p.push('\n');
    }
    p.push_str(&format!(
        "\n# Cevap\nSadece JSON dizisi yaz, başka hiçbir şey yazma. En iyi an başta, en fazla {n} an, anlar çakışmasın:\n\
         [{{\"start\": saniye, \"end\": saniye, \"punch\": punchline'ın başladığı saniye, \"score\": 0-10 ne kadar komik, \"title\": \"kısa başlık\", \"why\": \"neden komik, tek cümle\"}}]\n"
    ));
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

pub fn llm_cmd() -> String {
    std::env::var("EDITS_LLM").unwrap_or_else(|_| "claude -p".into())
}

pub fn pick(title: &str, lines: &[Line], curve: &[f64], hits: &[Hit], n: usize, dur: f64, use_llm: bool) -> Vec<Moment> {
    if use_llm && !lines.is_empty() {
        match run_stdin(&llm_cmd(), &prompt(&taste(), title, lines, curve, hits, n)).and_then(|r| parse(&r)) {
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
        assert!(p.contains("ZEVK") && p.contains("[48.0-52.0] buda boynu bükük kalmasın  🔥2.5  yorum(0:49): \"0:49 gariban :D\""), "{p}");
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
