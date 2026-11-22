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

/// Words that mark another version of a song: its times differ from the record's.
const VERSIONS: &[&str] = &["sped", "speed", "slowed", "nightcore", "reverb", "remix", "acoustic", "akustik"];

/// Shazam's song is the one searched for: most of its title's words are in the search, and both are the same
/// version (a sped-up search is not answered by the original record, whose times differ).
pub fn same_song(want: &str, title: &str) -> bool {
    let w = clean(want);
    let have: Vec<&str> = w.split(' ').collect();
    let t = clean(title);
    let words: Vec<&str> = t.split(' ').filter(|x| !x.is_empty()).collect();
    let version = |ws: &[&str]| VERSIONS.iter().any(|v| ws.contains(v));
    !words.is_empty() && words.iter().filter(|x| have.contains(x)).count() * 2 >= words.len() && version(&have) == version(&words)
}

/// Where in `title`'s song the clips people post start (seconds), from YouTube Shorts that use it, each heard by
/// Shazam; None when fewer than two agree. Remembered per song in the cache.
pub fn find(title: &str, cache: &Path) -> Option<f64> {
    let key = clean(title);
    // one word finds anything
    if key.split(' ').count() < 2 {
        return None;
    }
    // hook-starts.json, not 0.26.0's hooks.json: that one remembered searches that could not see Shorts
    let file = cache.join("music").join("hook-starts.json");
    let mut known: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if let Some(v) = known.get(&key) {
        return v.as_f64();
    }
    let (found, sure) = search(&key, cache);
    // a search that heard too few clips (no Shorts, Shazam down) is tried again next time, not remembered as no hook
    if sure {
        known.insert(key, found.map_or(serde_json::Value::Null, serde_json::Value::from));
        let _ = std::fs::write(&file, serde_json::to_string_pretty(&known).unwrap_or_default());
    }
    found
}

/// A search line (id, length, link) that is a Short: a /shorts/ link (the search often gives no length for those),
/// or anything 5 to 65 s long.
pub fn short(line: &str) -> Option<(String, String)> {
    let mut f = line.split('\t');
    let (id, dur, url) = (f.next()?.trim(), f.next()?.trim(), f.next()?.trim());
    let fits = url.contains("/shorts/") || dur.parse::<f64>().is_ok_and(|d| (5.0..=65.0).contains(&d));
    (fits && url.starts_with("http") && !id.is_empty()).then(|| (id.to_string(), url.to_string()))
}

/// Why a Short's place was not used, or its place in the song.
fn place(key: &str, heard: Result<Option<crate::shazam::Found>, String>) -> Result<f64, String> {
    let f = heard?.ok_or("Shazam did not know it")?;
    if !same_song(key, &f.title) {
        return Err(format!("another song (\"{}\")", f.title));
    }
    // a sped-up clip's place is on the record's clock, not this song's
    if f.skew >= 0.02 {
        return Err(format!("sped up or pitched ({:.0}%)", f.skew * 100.0));
    }
    Ok(f.offset.max(0.0))
}

/// The hook, and whether the answer is sure enough to remember (at least two clips heard as this song).
fn search(key: &str, cache: &Path) -> (Option<f64>, bool) {
    let Ok(out) = yt(&["--no-warnings", "--flat-playlist", "--print", "%(id)s\t%(duration)s\t%(webpage_url,url)s", "--", &format!("ytsearch15:{key} #shorts")]) else {
        eprintln!("edits: no hook for \"{key}\": the YouTube search failed; the most replayed or loudest part is used");
        return (None, false);
    };
    // Shorts only: a full-length upload starts at the song's start and says nothing about the hook
    let found = out.lines().count();
    let shorts: Vec<(String, String)> = out.lines().filter_map(short).take(6).collect();
    if shorts.len() < 2 {
        eprintln!("edits: no hook for \"{key}\": {} Shorts among {found} search results; the most replayed or loudest part is used", shorts.len());
        return (None, false);
    }
    let dir = cache.join("music").join("hook");
    if std::fs::create_dir_all(&dir).is_err() {
        return (None, false);
    }
    // downloads side by side; Shazam one clip at a time, since asking six at once gets every answer refused
    let files: Vec<(String, Result<std::path::PathBuf, String>)> = std::thread::scope(|s| {
        let jobs: Vec<_> = shorts.iter().map(|(id, url)| {
            let dir = &dir;
            s.spawn(move || {
                yt(&["--no-warnings", "--no-progress", "--no-playlist", "-f", "ba[ext=m4a]/ba", "--print", "after_move:filepath",
                    "-o", &dir.join(format!("{}.%(ext)s", crate::fetch::safe(id))).to_string_lossy(), "--", url])
                    .and_then(|got| got.lines().last().map(|l| std::path::PathBuf::from(l.trim())).ok_or_else(|| "no file".to_string()))
            })
        }).collect();
        shorts.iter().zip(jobs).map(|((id, _), j)| (id.clone(), j.join().unwrap_or_else(|_| Err("download crashed".into())))).collect()
    });
    let mut offsets = vec![];
    let mut why = vec![];
    for (i, (id, f)) in files.into_iter().enumerate() {
        let got = f.map_err(|e| format!("download failed: {}", e.lines().last().unwrap_or_default())).and_then(|f| {
            if i > 0 {
                std::thread::sleep(std::time::Duration::from_millis(700));
            }
            let mut heard = crate::shazam::recognize(&f, None);
            if heard.is_err() {
                // refused once: wait and ask again before giving up on the clip
                std::thread::sleep(std::time::Duration::from_secs(3));
                heard = crate::shazam::recognize(&f, None);
            }
            let _ = std::fs::remove_file(&f);
            place(key, heard)
        });
        match got {
            Ok(o) => offsets.push(o),
            Err(e) => why.push(format!("{id}: {e}")),
        }
    }
    let hook = agree(&offsets);
    match hook {
        Some(h) => eprintln!("edits: the part people use starts at {} ({} of {} Shorts agree)", crate::util::mmss(h), offsets.iter().filter(|o| (*o - h).abs() <= 2.5).count(), offsets.len()),
        None => eprintln!("edits: no hook found in Shorts for \"{key}\" ({} of {} heard as this song); the most replayed or loudest part is used", offsets.len(), shorts.len()),
    }
    if hook.is_none() && !why.is_empty() {
        eprintln!("edits: Shorts left out: {}", why.join("; "));
    }
    (hook, offsets.len() >= 2)
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
        // a sped-up song is not answered by its original record
        assert!(!same_song("Cupid Twin Ver Sped Up", "Cupid (Twin Ver.)"));
        assert!(same_song("Cupid Twin Ver Sped Up", "Cupid (Twin Ver.) [Sped Up]".replace(['[', ']'], "").as_str()));
    }

    #[test]
    fn a_short_is_placed_only_when_it_is_this_song_as_released() {
        let f = |title: &str, skew: f64| Ok(Some(crate::shazam::Found { artist: String::new(), title: title.into(), offset: 42.0, url: String::new(), skew }));
        assert_eq!(place("mabel matiz umrumdışı", f("Umrumdışı", 0.001)), Ok(42.0));
        assert!(place("mabel matiz umrumdışı", f("Umrumdışı", 0.08)).unwrap_err().starts_with("sped up"));
        assert!(place("mabel matiz umrumdışı", f("Başka Şarkı", 0.0)).unwrap_err().starts_with("another song"));
        assert_eq!(place("mabel matiz umrumdışı", Ok(None)), Err("Shazam did not know it".into()));
        assert_eq!(place("x y", Err("could not reach Shazam".into())), Err("could not reach Shazam".into()));
    }

    #[test]
    fn shorts_in_search_results() {
        assert_eq!(short("abc\tNA\thttps://www.youtube.com/shorts/abc"), Some(("abc".into(), "https://www.youtube.com/shorts/abc".into())));
        assert!(short("abc\t31\thttps://www.youtube.com/watch?v=abc").is_some());
        assert!(short("abc\t213\thttps://www.youtube.com/watch?v=abc").is_none());
        assert!(short("abc\tNA\thttps://www.youtube.com/watch?v=abc").is_none());
    }
}
