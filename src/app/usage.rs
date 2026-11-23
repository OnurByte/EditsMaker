//! Settings, Usage: what edits made (runs, clips, the space they take, the last two weeks day by day) and what the AI
//! was asked (each call `edits` noted in ~/.config/edits/usage.jsonl: which provider, how long, how much text).
use crate::app::runs::{self, Run};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Default)]
pub struct Usage {
    pub runs: usize,
    pub clips: usize,
    pub bytes: u64,
    /// runs made on each of the last 14 days, oldest first
    pub days: Vec<usize>,
    /// per provider: calls, answered, seconds, characters in and out
    pub ai: Vec<(String, Ai)>,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Ai {
    pub calls: usize,
    pub ok: usize,
    pub secs: f64,
    pub chars: u64,
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// The AI calls in a usage log, by provider, busiest first.
pub fn ai_of(log: &str) -> Vec<(String, Ai)> {
    let mut by: BTreeMap<String, Ai> = BTreeMap::new();
    for v in log.lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok()) {
        let a = by.entry(v["provider"].as_str().unwrap_or("?").to_string()).or_default();
        a.calls += 1;
        a.ok += v["ok"].as_bool().unwrap_or(false) as usize;
        a.secs += v["secs"].as_f64().unwrap_or(0.);
        a.chars += v["in"].as_u64().unwrap_or(0) + v["out"].as_u64().unwrap_or(0);
    }
    let mut v: Vec<(String, Ai)> = by.into_iter().collect();
    v.sort_by_key(|a| std::cmp::Reverse(a.1.calls));
    v
}

/// Runs a day over the last `n` days, oldest first.
pub fn per_day(made: &[SystemTime], n: usize, now: u64) -> Vec<usize> {
    let mut days = vec![0; n];
    for t in made {
        let age = now.saturating_sub(t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())) / 86400;
        if (age as usize) < n {
            days[n - 1 - age as usize] += 1;
        }
    }
    days
}

pub fn read(past: &[Run]) -> Usage {
    let (mut clips, mut bytes) = (0, 0);
    for r in past {
        let files = runs::files(&r.dir);
        clips += files.iter().filter(|f| f.extension().is_some_and(|e| e == "mp4")).count();
        bytes += files.iter().filter_map(|f| std::fs::metadata(f).ok()).map(|m| m.len()).sum::<u64>();
    }
    let log = std::fs::read_to_string(crate::ai::usage_file()).unwrap_or_default();
    Usage { runs: past.len(), clips, bytes, days: per_day(&past.iter().map(|r| r.made).collect::<Vec<_>>(), 14, now()), ai: ai_of(&log) }
}

/// 1.4 GB, 320 MB.
pub fn size(b: u64) -> String {
    match b {
        0..1_000_000 => format!("{} KB", b / 1000),
        1_000_000..1_000_000_000 => format!("{} MB", b / 1_000_000),
        _ => format!("{:.1} GB", b as f64 / 1e9),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calls_are_counted_by_provider() {
        let log = "{\"provider\":\"codex\",\"secs\":2.5,\"in\":100,\"out\":20,\"ok\":true}\n{\"provider\":\"claude-code\",\"secs\":1,\"in\":1,\"out\":1,\"ok\":false}\nnot json\n{\"provider\":\"codex\",\"secs\":0.5,\"in\":0,\"out\":0,\"ok\":true}";
        let v = ai_of(log);
        assert_eq!(v[0].0, "codex");
        assert_eq!(v[0].1, Ai { calls: 2, ok: 2, secs: 3.0, chars: 120 });
        assert_eq!(v[1].1.ok, 0);
        let day = 86400;
        let t = |d: u64| UNIX_EPOCH + std::time::Duration::from_secs(100 * day - d * day);
        assert_eq!(per_day(&[t(0), t(0), t(2), t(30)], 3, 100 * day), [1, 0, 2]);
        assert_eq!(size(1_500_000_000), "1.5 GB");
    }
}
