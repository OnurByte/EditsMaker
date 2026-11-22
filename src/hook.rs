//! The part of a song people use. A TikTok sound or a Short carries one stretch of a song, the one everyone knows
//! (often not the loudest one); Shazam says where each clip sits in the song, and where several agree is the hook.
use crate::util::yt;
use std::path::Path;

/// A title as people search it: what is in brackets dropped ("(Official Video)", "[Lyrics]"), "official",
/// "lyrics" and the like left out, lower case.
pub fn clean(title: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in title.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth = (depth - 1).max(0),
            // "İ" lowercases to i plus a combining dot, which would split the word
            'İ' if depth == 0 => out.push('i'),
            _ if depth == 0 => out.extend(c.to_lowercase()),
            _ => {}
        }
    }
    let noise = ["official", "video", "audio", "lyrics", "lyric", "music", "clip", "klip", "hd", "4k", "mv", "visualizer", "prod"];
    out.split(|c: char| !c.is_alphanumeric() && c != '\'').filter(|w| !w.is_empty() && !noise.contains(w)).collect::<Vec<_>>().join(" ")
}

/// Offsets that agree within 2.5 s: the middle of the biggest group, when at least two clips found it.
pub fn agree(offsets: &[f64]) -> Option<f64> {
    let mut best: Vec<f64> = vec![];
    for &o in offsets {
        let near: Vec<f64> = offsets.iter().copied().filter(|x| (x - o).abs() <= 2.5).collect();
        if near.len() > best.len() {
            best = near;
        }
    }
    if best.len() < 2 {
        return None;
    }
    best.sort_by(f64::total_cmp);
    Some(best[best.len() / 2])
}

/// Shazam's song is the one searched for: most of its title's words are in the search.
pub fn same_song(want: &str, title: &str) -> bool {
    let w = clean(want);
    let have: Vec<&str> = w.split(' ').collect();
    let t = clean(title);
    let words: Vec<&str> = t.split(' ').filter(|x| !x.is_empty()).collect();
    !words.is_empty() && words.iter().filter(|x| have.contains(x)).count() * 2 >= words.len()
}

/// Where in `title`'s song the clips people post start (seconds), from YouTube Shorts that use it, each heard by
/// Shazam; None when fewer than two agree. Remembered per song in the cache.
pub fn find(title: &str, cache: &Path) -> Option<f64> {
    let key = clean(title);
    // one word finds anything
    if key.split(' ').count() < 2 {
        return None;
    }
    let file = cache.join("music").join("hooks.json");
    let mut known: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if let Some(v) = known.get(&key) {
        return v.as_f64();
    }
    let found = search(&key, cache);
    known.insert(key, found.map_or(serde_json::Value::Null, serde_json::Value::from));
    let _ = std::fs::write(&file, serde_json::to_string_pretty(&known).unwrap_or_default());
    found
}

fn search(key: &str, cache: &Path) -> Option<f64> {
    let out = yt(&["--no-warnings", "--flat-playlist", "--print", "%(id)s\t%(duration)s\t%(webpage_url,url)s", "--", &format!("ytsearch15:{key} #shorts")]).ok()?;
    // Shorts only: a full-length upload starts at the song's start and says nothing about the hook
    let shorts: Vec<(String, String)> = out.lines().filter_map(|l| {
        let mut f = l.split('\t');
        let (id, dur, url) = (f.next()?.trim(), f.next()?.trim().parse::<f64>().ok()?, f.next()?.trim());
        ((5.0..=65.0).contains(&dur) && url.starts_with("http")).then(|| (id.to_string(), url.to_string()))
    }).take(6).collect();
    if shorts.len() < 2 {
        return None;
    }
    let dir = cache.join("music").join("hook");
    std::fs::create_dir_all(&dir).ok()?;
    let offsets: Vec<f64> = std::thread::scope(|s| {
        let jobs: Vec<_> = shorts.iter().map(|(id, url)| {
            let dir = &dir;
            s.spawn(move || {
                let got = yt(&["--no-warnings", "--no-progress", "--no-playlist", "-f", "ba[ext=m4a]/ba", "--print", "after_move:filepath",
                    "-o", &dir.join(format!("{}.%(ext)s", crate::fetch::safe(id))).to_string_lossy(), "--", url]).ok()?;
                let f = std::path::PathBuf::from(got.lines().last()?.trim());
                let heard = crate::shazam::recognize(&f, None);
                let _ = std::fs::remove_file(&f);
                let found = heard.ok()??;
                same_song(key, &found.title).then_some(found.offset.max(0.0))
            })
        }).collect();
        jobs.into_iter().filter_map(|j| j.join().ok().flatten()).collect()
    });
    let hook = agree(&offsets);
    match hook {
        Some(h) => eprintln!("edits: the part people use starts at {} ({} of {} Shorts agree)", crate::util::mmss(h), offsets.iter().filter(|o| (*o - h).abs() <= 2.5).count(), offsets.len()),
        None => eprintln!("edits: no hook found in Shorts for \"{key}\" ({} heard); the most replayed or loudest part is used", offsets.len()),
    }
    hook
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_as_people_search_them() {
        assert_eq!(clean("Ravi İncigöz - Maşallah (Official Video)"), "ravi incigöz maşallah");
        assert_eq!(clean("Mabel Matiz - Umrumdışı [Lyrics] 4K"), "mabel matiz umrumdışı");
        assert_eq!(clean("Don't Stop (Remix)"), "don't stop");
    }

    #[test]
    fn the_hook_is_where_clips_agree() {
        assert_eq!(agree(&[42.1, 61.0, 43.0, 41.8]), Some(42.1));
        assert_eq!(agree(&[10.0, 50.0, 90.0]), None);
        assert_eq!(agree(&[30.0]), None);
        assert_eq!(agree(&[]), None);
    }

    #[test]
    fn shazam_names_the_searched_song() {
        assert!(same_song("Ravi İncigöz - Maşallah", "Maşallah"));
        assert!(same_song("mabel matiz umrumdışı", "Umrumdışı (Akustik)"));
        assert!(!same_song("Ravi İncigöz - Maşallah", "Bambaşka Bir Şarkı"));
        assert!(!same_song("Ravi İncigöz - Maşallah", ""));
    }
}
