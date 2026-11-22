//! Long videos (a stream's VOD, a podcast): where the clips are, found the way clippers find them. Every 30 s
//! window is scored by how loud it gets against the whole video, how many reactions are said in it ("no way",
//! "oha"), how much is said, and how hard the chat and the viewers went; the best windows at least 90 s apart are
//! the moments. Flags (swearing, sexual talk, the story's ending, whisper's loops) warn and never change the score.
use crate::lang::Lang;
use crate::pick::Moment;
use crate::transcribe::{Line, Word};
use crate::util::{mmss, zscores};
use serde_json::Value;

pub const WINDOW: f64 = 30.0;
/// moments closer than this are one moment
pub const APART: f64 = 90.0;
/// (from, to) seconds
pub type Span = (f64, f64);
/// a stream's chat: (second, message)
pub type Chat = Vec<(f64, String)>;
/// a window where nobody speaks stays only when it is this loud (z): an explosion, a scream, a crash
const SILENT_LOUD: f64 = 1.5;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Window {
    pub start: f64,
    pub end: f64,
    pub score: f64,
    /// the loudest 5 s against the whole video (z)
    pub loud: f64,
    pub reactions: usize,
    pub words: usize,
    /// messages in the chat against the whole video (z), 0 without a chat
    pub chat: f64,
    /// chat messages laughing or hyped (KEKW, sjsj, 💀)
    pub hype: usize,
    /// where the loudest second is: the cut lands around it
    pub peak: f64,
    pub flags: Vec<&'static str>,
}

/// Lower case words, every emoji its own word, punctuation gone.
pub fn norm(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_alphanumeric() || c == '\'' {
            out.extend(c.to_lowercase());
        } else if !c.is_ascii() && !c.is_whitespace() {
            out.push(' ');
            out.push(c);
            out.push(' ');
        } else {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// How often the phrases are said in `text` (already normalised): a one-word phrase counts as a whole word only.
pub fn count(text: &str, phrases: &[String]) -> usize {
    let words: Vec<&str> = text.split_whitespace().collect();
    phrases.iter().map(|p| norm(p)).filter(|p| !p.is_empty()).map(|p| {
        let want: Vec<&str> = p.split(' ').collect();
        words.windows(want.len()).filter(|w| *w == want.as_slice()).count()
    }).sum()
}

/// Whisper invents the same line over and over under music. A line said more than twice in a row keeps two, and a
/// line of four words or more heard 30 times or more is dropped everywhere. Returns the lines kept, the stretches
/// whose words are dropped, and the stretches of those loops (flagged).
pub fn unloop(lines: &[Line]) -> (Vec<Line>, Vec<Span>, Vec<Span>) {
    let keys: Vec<String> = lines.iter().map(|l| norm(&l.text)).collect();
    let mut seen: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for k in &keys {
        *seen.entry(k.as_str()).or_default() += 1;
    }
    let (mut kept, mut dropped, mut loops) = (vec![], vec![], vec![]);
    let mut run = 0;
    for (i, l) in lines.iter().enumerate() {
        run = if i > 0 && keys[i] == keys[i - 1] { run + 1 } else { 1 };
        let looped = keys[i].split(' ').count() >= 4 && seen[keys[i].as_str()] >= 30;
        if looped {
            loops.push((l.s, l.e));
        }
        if looped || run > 2 {
            dropped.push((l.s, l.e));
        } else {
            kept.push(Line { s: l.s, e: l.e, text: l.text.clone() });
        }
    }
    (kept, dropped, loops)
}

/// The words outside the dropped stretches.
pub fn keep_words(words: Vec<Word>, dropped: &[(f64, f64)]) -> Vec<Word> {
    words.into_iter().filter(|w| !dropped.iter().any(|(a, b)| w.s >= *a - 0.01 && w.s < *b)).collect()
}

/// A stream's chat replay (YouTube's live_chat JSON lines, Twitch's rechat JSON) as (second, message).
pub fn parse_chat(s: &str) -> Vec<(f64, String)> {
    fn texts(v: &Value, out: &mut Vec<String>) {
        match v {
            Value::Object(m) => {
                for (k, x) in m {
                    match (k.as_str(), x) {
                        ("text" | "body", Value::String(t)) => out.push(t.clone()),
                        // who wrote it is not what was said
                        ("authorName" | "author" | "commenter", _) => {}
                        _ => texts(x, out),
                    }
                }
            }
            Value::Array(a) => a.iter().for_each(|x| texts(x, out)),
            _ => {}
        }
    }
    fn walk(v: &Value, out: &mut Vec<(f64, String)>) {
        match v {
            Value::Object(m) => {
                let num = |x: &Value| x.as_f64().or_else(|| x.as_str().and_then(|s| s.parse().ok()));
                let at = m.get("videoOffsetTimeMsec").and_then(num).map(|ms| ms / 1000.0).or_else(|| m.get("content_offset_seconds").and_then(num));
                match at {
                    Some(t) => {
                        let mut said = vec![];
                        texts(v, &mut said);
                        let msg = said.join("").trim().to_string();
                        if !msg.is_empty() {
                            out.push((t, msg));
                        }
                    }
                    None => m.values().for_each(|x| walk(x, out)),
                }
            }
            Value::Array(a) => a.iter().for_each(|x| walk(x, out)),
            _ => {}
        }
    }
    let mut out = vec![];
    match serde_json::from_str::<Value>(s) {
        Ok(v) => walk(&v, &mut out),
        Err(_) => s.lines().filter_map(|l| serde_json::from_str::<Value>(l).ok()).for_each(|v| walk(&v, &mut out)),
    }
    out.sort_by(|a, b| a.0.total_cmp(&b.0));
    out
}

/// Every 30 s window of a `dur`-second video, scored. `loud` is RMS dB per half second, `viewers` one value per
/// second (replay heat and timestamped comments, already z-like), `loops` whisper's loops.
pub fn windows(dur: f64, loud: &[f64], words: &[Word], chat: &[(f64, String)], viewers: &[f64], loops: &[(f64, f64)], l: &Lang) -> Vec<Window> {
    let n = (dur / WINDOW).ceil().max(1.0) as usize;
    let sec: Vec<f64> = loud.chunks(2).map(|c| c.iter().sum::<f64>() / c.len() as f64).collect();
    let mut peaks = vec![];
    let ls: Vec<f64> = (0..n).map(|k| {
        let (a, b) = ((k as f64 * WINDOW) as usize, (((k + 1) as f64 * WINDOW) as usize).min(sec.len()));
        let mut v: Vec<(usize, f64)> = (a.min(b)..b).map(|i| (i, sec[i])).collect();
        v.sort_by(|x, y| y.1.total_cmp(&x.1));
        peaks.push(v.first().map_or(a as f64 + WINDOW / 2.0, |x| x.0 as f64 + 0.5));
        let top: Vec<f64> = v.iter().take(5).map(|x| x.1).collect();
        if top.is_empty() { -90.0 } else { top.iter().sum::<f64>() / top.len() as f64 }
    }).collect();
    let lz = if loud.is_empty() { vec![0.0; n] } else { zscores(&ls) };
    let mut text = vec![String::new(); n];
    let mut wc = vec![0usize; n];
    for w in words {
        let k = ((w.s / WINDOW) as usize).min(n - 1);
        text[k].push(' ');
        text[k].push_str(&w.t);
        wc[k] += 1;
    }
    let (mut msgs, mut hype) = (vec![0.0; n], vec![0usize; n]);
    for (t, m) in chat {
        let k = ((t / WINDOW) as usize).min(n - 1);
        msgs[k] += 1.0;
        if count(&norm(m), &l.hype) > 0 {
            hype[k] += 1;
        }
    }
    let cz = if chat.is_empty() { vec![0.0; n] } else { zscores(&msgs) };
    let hz = if chat.is_empty() { vec![0.0; n] } else { zscores(&hype.iter().map(|&x| x as f64).collect::<Vec<_>>()) };
    (0..n).map(|k| {
        let (a, b) = (k as f64 * WINDOW, ((k + 1) as f64 * WINDOW).min(dur));
        let t = norm(&text[k]);
        let reactions = count(&t, &l.reactions);
        let seen = viewers.iter().skip(a as usize).take(WINDOW as usize).cloned().fold(0.0, f64::max);
        let mut flags = vec![];
        for (name, list) in [("profanity", &l.profanity), ("sexual", &l.sexual), ("spoiler", &l.spoiler)] {
            if count(&t, list) > 0 {
                flags.push(name);
            }
        }
        if loops.iter().any(|(s, e)| *s < b && *e > a) {
            flags.push("loop");
        }
        let score = lz[k] + 0.35 * reactions.min(12) as f64 + 0.01 * wc[k].min(150) as f64 + 0.6 * cz[k].max(0.0) + 0.6 * hz[k].max(0.0) + 0.5 * seen.max(0.0);
        Window { start: a, end: b, score, loud: lz[k], reactions, words: wc[k], chat: cz[k], hype: hype[k], peak: peaks[k], flags }
    }).collect()
}

/// The `n` best windows at least APART from each other (greedy: the best, then the best not near it, ...). A window
/// where nobody speaks stays only when it is loud enough to be something.
pub fn best(ws: &[Window], n: usize) -> Vec<Window> {
    let mut order: Vec<&Window> = ws.iter().filter(|w| w.words > 0 || w.loud >= SILENT_LOUD).collect();
    order.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut out: Vec<Window> = vec![];
    for w in order {
        if out.len() == n {
            break;
        }
        if out.iter().all(|o| (o.start - w.start).abs() >= APART) {
            out.push(w.clone());
        }
    }
    out
}

/// A window as a moment: 6 s before it for the setup, 8 s after for the reaction.
pub fn moment(w: &Window) -> Moment {
    let mut why = format!("loud {:+.1}, {} reactions, {} words", w.loud, w.reactions, w.words);
    if w.chat != 0.0 || w.hype > 0 {
        why.push_str(&format!(", chat {:+.1}, {} hyped", w.chat, w.hype));
    }
    if !w.flags.is_empty() {
        why.push_str(&format!(" [{}]", w.flags.join(", ")));
    }
    Moment { start: (w.start - 6.0).max(0.0), end: w.end + 8.0, punch: w.peak, score: w.score, title: String::new(), why }
}

/// Lines that match any of `patterns` (case aside): the quiet finds and one-line punchlines a loudness score
/// misses, looked up by what was said (a clip people already posted). Each is a moment from 8 s before to 12 s after.
pub fn find(lines: &[Line], patterns: &[String]) -> Result<Vec<Moment>, String> {
    let res: Vec<regex::Regex> = patterns.iter().map(|p| regex::RegexBuilder::new(p).case_insensitive(true).build().map_err(|e| format!("--find {p}: {e}"))).collect::<Result<_, _>>()?;
    let mut out: Vec<Moment> = vec![];
    for l in lines {
        if let Some(r) = res.iter().find(|r| r.is_match(&l.text)) {
            if out.iter().all(|m| (m.start - (l.s - 8.0)).abs() >= WINDOW) {
                out.push(Moment { start: (l.s - 8.0).max(0.0), end: l.e + 12.0, punch: l.s, score: 0.0, title: String::new(), why: format!("said \"{}\" (--find {})", l.text.chars().take(80).collect::<String>(), r.as_str()) });
            }
        }
    }
    Ok(out)
}

/// The candidates as a prompt for the model to judge by the clipping rubric: what was said in each, its signals
/// and flags, the chat at its peak.
pub fn rubric_prompt(title: &str, cands: &[Moment], lines: &[Line], chat: &[(f64, String)], n: usize) -> String {
    use crate::lang::prompt as part;
    let mut p = part("vod", "head", &[("n", n.to_string()), ("title", title.to_string())]);
    for (i, m) in cands.iter().enumerate() {
        let said: Vec<String> = lines.iter().filter(|l| l.e > m.start && l.s < m.end).map(|l| format!("[{:.0}] {}", l.s, l.text)).collect();
        let talk: String = said.join("\n").chars().take(1500).collect();
        let chatted: Vec<&str> = chat.iter().filter(|(t, _)| *t >= m.start && *t < m.end).map(|(_, s)| s.as_str()).take(12).collect();
        p.push_str(&part("vod", "candidate", &[("id", i.to_string()), ("start", format!("{:.0}", m.start)), ("end", format!("{:.0}", m.end)),
            ("at", mmss(m.start)), ("signals", m.why.clone()), ("said", talk), ("chat", chatted.join(" | "))]));
    }
    p.push_str(&part("vod", "tail", &[("n", n.to_string())]));
    p
}

#[derive(serde::Deserialize)]
struct Judged {
    id: usize,
    #[serde(default)]
    start: Option<f64>,
    #[serde(default)]
    end: Option<f64>,
    #[serde(default)]
    score: f64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    why: String,
    #[serde(default)]
    post: String,
}

/// The model's verdicts on the candidates: the ones it kept (12 of 21 or more), best first, with its tighter cut
/// when it gave one inside the candidate, each with its post caption.
pub fn judged(reply: &str, cands: &[Moment]) -> Result<Vec<(Moment, String)>, String> {
    let (a, b) = (reply.find('[').ok_or("no JSON array in the model's reply")?, reply.rfind(']').ok_or("no JSON array in the model's reply")?);
    let v: Vec<Judged> = serde_json::from_str(reply.get(a..=b).ok_or("no JSON array in the model's reply")?).map_err(|e| format!("model reply is not valid JSON: {e}"))?;
    let mut out: Vec<(Moment, String)> = v.into_iter().filter(|j| j.score >= 12.0).filter_map(|j| {
        let c = cands.get(j.id)?;
        let s = j.start.filter(|s| *s >= c.start - 5.0 && *s < c.end).unwrap_or(c.start);
        let e = j.end.filter(|e| *e > s + 5.0 && *e <= c.end + 10.0 && *e - s <= 90.0).unwrap_or(c.end);
        Some((Moment { start: s, end: e, punch: c.punch.clamp(s, e), score: j.score, title: j.title, why: format!("{} ({}/21; {})", j.why, j.score, c.why) }, j.post))
    }).collect();
    out.sort_by(|a, b| b.0.score.total_cmp(&a.0.score));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(s: f64, t: &str) -> Line {
        Line { s, e: s + 2.0, text: t.into() }
    }

    fn lang() -> Lang {
        Lang { reactions: vec!["no way".into(), "oha".into(), "chat".into()], hype: vec!["kekw".into(), "💀".into()], profanity: vec!["amk".into()], spoiler: vec!["final mission".into()], ..Default::default() }
    }

    #[test]
    fn words_and_phrases_are_counted_whole() {
        assert_eq!(norm("NO WAY!! Oha, chat... 💀💀"), "no way oha chat 💀 💀");
        let r = lang().reactions;
        assert_eq!(count(&norm("no way, no way chat oha chatting"), &r), 4);
        assert_eq!(count(&norm("💀💀💀 lol"), &["💀".to_string()]), 3);
        assert_eq!(count("", &r), 0);
    }

    #[test]
    fn whisper_loops_are_dropped_and_flagged() {
        let mut ls: Vec<Line> = (0..40).map(|i| line(i as f64 * 100.0, "Thank you for watching the video")).collect();
        ls.extend([line(5000.0, "aynı"), line(5002.0, "aynı"), line(5004.0, "aynı"), line(5006.0, "başka")]);
        let (kept, dropped, loops) = unloop(&ls);
        assert_eq!(kept.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["aynı", "aynı", "başka"]);
        assert_eq!((loops.len(), dropped.len()), (40, 41));
        let ws = keep_words(vec![Word { s: 100.5, e: 101.0, t: "Thank".into() }, Word { s: 5006.5, e: 5007.0, t: "başka".into() }], &dropped);
        assert_eq!(ws.len(), 1);
    }

    #[test]
    fn reads_both_chats() {
        let yt = r#"{"replayChatItemAction":{"actions":[{"addChatItemAction":{"item":{"liveChatTextMessageRenderer":{"message":{"runs":[{"text":"KEKW "},{"text":"no way"}]},"authorName":{"simpleText":"x"}}}}}],"videoOffsetTimeMsec":"61500"}}
{"replayChatItemAction":{"actions":[],"videoOffsetTimeMsec":"70000"}}"#;
        assert_eq!(parse_chat(yt), vec![(61.5, "KEKW no way".to_string())]);
        let tw = r#"{"comments":[{"content_offset_seconds":12,"commenter":{"display_name":"y"},"message":{"body":"oha"}}]}"#;
        assert_eq!(parse_chat(tw), vec![(12.0, "oha".to_string())]);
    }

    #[test]
    fn the_loud_reacting_window_wins() {
        // 10 windows of quiet talk; window 4 is loud with reactions and a hyped chat, window 7 loud and silent
        let mut loud = vec![-30.0; 600];
        loud[4 * 60..4 * 60 + 10].iter_mut().for_each(|x| *x = -5.0);
        loud[7 * 60..7 * 60 + 10].iter_mut().for_each(|x| *x = -4.0);
        let mut words: Vec<Word> = (0..10).filter(|k| *k != 7).map(|k| Word { s: k as f64 * 30.0 + 5.0, e: k as f64 * 30.0 + 5.5, t: "yani".into() }).collect();
        words.extend(["oha", "no", "way", "amk"].iter().enumerate().map(|(i, t)| Word { s: 125.0 + i as f64, e: 125.5 + i as f64, t: t.to_string() }));
        let chat: Vec<(f64, String)> = (0..20).map(|i| (121.0 + i as f64 * 0.5, "KEKW".to_string())).chain([(10.0, "hi".to_string())]).collect();
        let ws = windows(300.0, &loud, &words, &chat, &[], &[(200.0, 210.0)], &lang());
        assert_eq!(ws.len(), 10);
        let top = best(&ws, 3);
        assert_eq!(top[0].start, 120.0);
        assert_eq!((top[0].reactions, top[0].flags.clone()), (2, vec!["profanity"]));
        assert!(top[0].chat > 2.0 && top[0].hype == 20);
        assert!((top[0].peak - 120.5).abs() < 1.0, "{}", top[0].peak);
        // loud and wordless is kept for being loud; the window 90 s near the best is not
        assert!(top.iter().any(|w| w.start == 210.0) && top.iter().all(|w| w.start != 150.0));
        assert_eq!(ws[6].flags, vec!["loop"]);
        let m = moment(&top[0]);
        assert_eq!((m.start, m.end), (114.0, 158.0));
    }

    #[test]
    fn found_by_what_was_said() {
        let ls = vec![line(10.0, "ilk"), line(400.0, "Bu araba UÇUYOR abi"), line(405.0, "uçuyor gerçekten")];
        let f = find(&ls, &["uçuyor".into()]).unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].start, f[0].end), (392.0, 414.0));
        assert!(find(&ls, &["(".into()]).is_err());
    }

    #[test]
    fn the_model_keeps_twelve_or_more() {
        let c = vec![Moment { start: 100.0, end: 150.0, punch: 120.0, score: 3.0, title: String::new(), why: "loud".into() }; 2];
        let r = "ok [{\"id\":1,\"start\":105,\"end\":140,\"score\":15,\"title\":\"t\",\"why\":\"w\",\"post\":\"p #a\"},{\"id\":0,\"score\":9},{\"id\":7,\"score\":20}]";
        let j = judged(r, &c).unwrap();
        assert_eq!(j.len(), 1);
        assert_eq!((j[0].0.start, j[0].0.end, j[0].0.title.as_str(), j[0].1.as_str()), (105.0, 140.0, "t", "p #a"));
    }
}
