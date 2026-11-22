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

/// How far apart two clips' starts can be and still be the same part of the song (seconds).
const NEAR: f64 = 2.5;

/// Offsets that agree within NEAR: the middle of the biggest group, when at least two clips found it.
pub fn agree(offsets: &[f64]) -> Option<f64> {
    let mut best: Vec<f64> = vec![];
    for &o in offsets {
        let near: Vec<f64> = offsets.iter().copied().filter(|x| (x - o).abs() <= NEAR).collect();
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
    // where each Short was heard, not the answer: a better way to read them needs no new questions to Shazam
    // (hook-heard.json; the older files remembered answers, some from searches that could not see Shorts)
    let file = cache.join("music").join("hook-heard.json");
    let mut known: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    if let Some(v) = known.get(&key) {
        let heard: Vec<f64> = v.as_array().into_iter().flatten().filter_map(|x| x.as_f64()).collect();
        return agree(&heard);
    }
    let heard = search(&key, cache)?;
    known.insert(key, serde_json::Value::from(heard.clone()));
    let _ = std::fs::write(&file, serde_json::to_string_pretty(&known).unwrap_or_default());
    agree(&heard)
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

/// Where in the song each Short heard as it starts; None when too few were heard to be sure (no Shorts, Shazam
/// refusing), so the song is searched again next time.
fn search(key: &str, cache: &Path) -> Option<Vec<f64>> {
    let Ok(out) = yt(&["--no-warnings", "--flat-playlist", "--print", "%(id)s\t%(duration)s\t%(webpage_url,url)s", "--", &format!("ytsearch15:{key} #shorts")]) else {
        eprintln!("edits: no hook for \"{key}\": the YouTube search failed; the most replayed or loudest part is used");
        return None;
    };
    // Shorts only: a full-length upload starts at the song's start and says nothing about the hook
    let found = out.lines().count();
    let shorts: Vec<(String, String)> = out.lines().filter_map(short).take(6).collect();
    if shorts.len() < 2 {
        eprintln!("edits: no hook for \"{key}\": {} Shorts among {found} search results; the most replayed or loudest part is used", shorts.len());
        return None;
    }
    let dir = cache.join("music").join("hook");
    std::fs::create_dir_all(&dir).ok()?;
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
    let mut refused = 0;
    for (i, (id, f)) in files.into_iter().enumerate() {
        let got = f.map_err(|e| format!("download failed: {}", e.lines().last().unwrap_or_default().split(". ").next().unwrap_or_default())).and_then(|f| {
            if i > 0 {
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
            // one question per clip, from its middle (a Short is one stretch of the song); answered with where the
            // clip itself starts
            let at = (crate::fetch::duration(&f).unwrap_or(0.0) / 2.0 - 6.0).max(0.0);
            let mut heard = crate::shazam::recognize(&f, Some(at));
            // too many questions (HTTP 429): Shazam answers again after a pause
            for wait in [8, 20] {
                if !heard.as_ref().is_err_and(|e| e.contains("429")) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_secs(wait));
                heard = crate::shazam::recognize(&f, Some(at));
            }
            let _ = std::fs::remove_file(&f);
            refused = if heard.as_ref().is_err_and(|e| e.contains("429")) { refused + 1 } else { 0 };
            place(key, heard)
        });
        match got {
            Ok(o) => offsets.push(o),
            Err(e) => why.push(format!("{id}: {e}")),
        }
        if refused >= 2 {
            why.push("Shazam keeps refusing; the rest are asked next time".into());
            break;
        }
    }
    // the clips not asked about after a refusal are still there
    let _ = std::fs::remove_dir_all(&dir);
    let at: Vec<String> = offsets.iter().map(|o| crate::util::mmss(*o)).collect();
    match agree(&offsets) {
        Some(h) => eprintln!("edits: the part people use starts at {} ({} of {} Shorts agree: {})", crate::util::mmss(h), offsets.iter().filter(|o| (*o - h).abs() <= NEAR).count(), offsets.len(), at.join(", ")),
        None => eprintln!("edits: no hook found in Shorts for \"{key}\" ({} of {} heard as this song{}{}); the most replayed or loudest part is used",
            offsets.len(), shorts.len(), if at.is_empty() { "" } else { ", starting at " }, at.join(", ")),
    }
    if !why.is_empty() {
        eprintln!("edits: Shorts left out: {}", why.join("; "));
    }
    (offsets.len() >= 2).then_some(offsets)
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
