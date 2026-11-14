//! Downloads as little as possible: audio, captions and viewer data first; video only for the moments that get cut.
use crate::util::{run, yt};
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct Source {
    /// a file the user gave; None for something online
    pub local: Option<PathBuf>,
    pub url: String,
    pub work: PathBuf,
    pub wav: PathBuf,
    /// the sound at full quality, for putting the voice into an edit (the 16 kHz wav when it is gone)
    pub audio: PathBuf,
    pub id: String,
    pub title: String,
    pub duration: f64,
    /// language the site says is spoken ("" when unknown)
    pub language: String,
    /// YouTube "most replayed": (start, end, value 0..1)
    pub heat: Vec<(f64, f64, f64)>,
    /// (text, likes)
    pub comments: Vec<(String, u64)>,
}

const VIDEO: &str = "bv*[height<=1080][ext=mp4]+ba[ext=m4a]/b[height<=1080]/b";

pub fn safe(id: &str) -> String {
    id.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).take(60).collect()
}

pub fn duration(video: &Path) -> Result<f64, String> {
    let v = video.to_string_lossy();
    let out = run(None, "ffprobe", &["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", v.as_ref()])?;
    out.trim().parse().map_err(|_| format!("ffprobe gave no duration for {v}"))
}

/// Frame size (0, 0 if ffprobe cannot tell).
pub fn dims(video: &Path) -> (u32, u32) {
    let v = video.to_string_lossy();
    let out = run(None, "ffprobe", &["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0", v.as_ref()]).unwrap_or_default();
    let mut it = out.trim().split(',').filter_map(|x| x.parse().ok());
    (it.next().unwrap_or(0), it.next().unwrap_or(0))
}

fn wav(input: &Path, out: &Path) -> Result<(), String> {
    run(None, "ffmpeg", &["-v", "error", "-y", "-i", &input.to_string_lossy(), "-vn", "-ar", "16000", "-ac", "1", &out.to_string_lossy()]).map(|_| ())
}

/// A bare channel link lists tabs, not videos; point it at the uploads.
fn channel_videos(url: &str) -> String {
    let u = url.trim_end_matches('/');
    let is_channel = u.contains("youtube.com/@") || u.contains("youtube.com/channel/") || u.contains("youtube.com/c/");
    let has_tab = ["/videos", "/shorts", "/streams", "/playlists"].iter().any(|t| u.ends_with(t));
    if is_channel && !has_tab { format!("{u}/videos") } else { url.to_string() }
}

/// One line of `yt-dlp --flat-playlist --print "%(id)s\t%(duration)s\t%(webpage_url,url)s"` -> (id, url),
/// dropping anything over `max_secs` when the length is known.
fn listed(line: &str, max_secs: Option<f64>) -> Option<(String, String)> {
    let mut f = line.split('\t');
    let (id, dur, url) = (f.next()?.trim(), f.next()?.trim(), f.next()?.trim());
    if id.is_empty() || !url.starts_with("http") {
        return None;
    }
    if let (Some(max), Ok(d)) = (max_secs, dur.parse::<f64>()) {
        if d > max {
            return None;
        }
    }
    Some((id.to_string(), url.to_string()))
}

/// What to work on: (id, url) pairs for a link, a channel, a playlist or a topic search. Downloads nothing.
pub fn resolve(input: &str, videos: usize) -> Result<Vec<(String, String)>, String> {
    let is_url = input.starts_with("http://") || input.starts_with("https://");
    let target = if is_url { channel_videos(input) } else { format!("ytsearch{videos}:{input}") };
    // "--" so a topic starting with "-" can never be read as a yt-dlp option
    let out = yt(&["--no-warnings", "--flat-playlist", "--playlist-end", &videos.to_string(), "--print", "%(id)s\t%(duration)s\t%(webpage_url,url)s", "--", &target])?;
    // a topic search must not pull a 3-hour stream
    let found: Vec<_> = out.lines().filter_map(|l| listed(l, (!is_url).then_some(1800.0))).collect();
    if found.is_empty() {
        return Err(format!("yt-dlp found nothing for \"{input}\""));
    }
    Ok(found)
}

/// Words of a title, for matching: lower case, letters and digits only.
fn words(t: &str) -> Vec<String> {
    t.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() > 1).map(String::from).collect()
}

/// Which of `titles` is the song called `want`: the most of its words, then the fewest extra ones.
pub fn best_title(want: &str, titles: &[String]) -> Option<usize> {
    let w = words(want);
    titles.iter().enumerate().max_by(|(_, a), (_, b)| {
        let score = |t: &String| {
            let tw = words(t);
            (w.iter().filter(|x| tw.contains(x)).count() as f64, -(tw.len() as f64))
        };
        score(a).partial_cmp(&score(b)).unwrap()
    }).map(|x| x.0)
}

/// A song by name: the YouTube result whose title matches it best (the first result alone varies between calls).
pub fn find_song(title: &str) -> Result<String, String> {
    let out = yt(&["--no-warnings", "--flat-playlist", "--print", "%(title)s\t%(webpage_url,url)s", "--", &format!("ytsearch8:{title}")])?;
    let found: Vec<(String, String)> = out.lines().filter_map(|l| l.split_once('\t')).map(|(t, u)| (t.to_string(), u.to_string())).collect();
    let titles: Vec<String> = found.iter().map(|f| f.0.clone()).collect();
    best_title(title, &titles).map(|i| found[i].1.clone()).ok_or(format!("no song found for \"{title}\""))
}

pub fn local(file: &Path, cache: &Path) -> Result<Source, String> {
    let id = safe(&file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "video".into()));
    let work = cache.join(&id);
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let w = work.join("a.wav");
    // always redone, and the old transcript dropped: another file may share this name
    let _ = std::fs::remove_file(w.with_extension("json"));
    wav(file, &w)?;
    Ok(Source { local: Some(file.to_path_buf()), url: String::new(), audio: file.to_path_buf(), wav: w, work, title: id.clone(), id, duration: duration(file)?, language: String::new(), heat: vec![], comments: vec![] })
}

/// Audio, info, comments and the site's own word-timed captions: a few MB instead of the whole video.
pub fn remote(id: &str, url: &str, cache: &Path) -> Result<Source, String> {
    let work = cache.join(safe(id));
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let (w, info_file) = (work.join("a.wav"), work.join("src.info.json"));
    if !(w.is_file() && info_file.is_file()) {
        let out = yt(&[
            "--no-warnings", "--no-progress", "--no-playlist", "-f", "ba[ext=m4a]/ba/b",
            "--write-info-json", "--write-comments", "--extractor-args", "youtube:max_comments=300,all,0,0;comment_sort=top",
            // "-orig" is the speech recognition track in the language actually spoken, with per-word times
            "--write-auto-subs", "--sub-langs", ".*-orig", "--sub-format", "json3",
            "--print", "after_move:filepath", "-o", &work.join("src.%(ext)s").to_string_lossy(), "--", url,
        ])?;
        let audio = out.lines().map(str::trim).rfind(|l| !l.is_empty()).ok_or("yt-dlp reported no file")?;
        wav(Path::new(audio), &w)?;
    }
    let audio = std::fs::read_dir(&work).into_iter().flatten().flatten().map(|e| e.path())
        .find(|p| p.file_stem().is_some_and(|s| s == "src") && p.extension().is_some_and(|e| ["m4a", "webm", "opus", "mp3", "mp4"].contains(&e.to_string_lossy().as_ref())))
        .unwrap_or_else(|| w.clone());
    let info: Value = std::fs::read_to_string(&info_file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    Ok(Source {
        local: None,
        url: url.to_string(),
        id: id.to_string(),
        title: info["title"].as_str().unwrap_or("").to_string(),
        language: info["language"].as_str().unwrap_or("").to_string(),
        duration: match info["duration"].as_f64() { Some(d) => d, None => duration(&w)? },
        heat: info["heatmap"].as_array().map(|a| a.iter().filter_map(|h| {
            Some((h["start_time"].as_f64()?, h["end_time"].as_f64()?, h["value"].as_f64()?))
        }).collect()).unwrap_or_default(),
        comments: info["comments"].as_array().map(|a| a.iter().filter_map(|c| {
            Some((c["text"].as_str()?.to_string(), c["like_count"].as_u64().unwrap_or(0)))
        }).collect()).unwrap_or_default(),
        wav: w,
        audio,
        work,
    })
}

/// What a video says and how it did, read without downloading it: its info and its captions (a few kB).
pub struct Preview {
    pub id: String,
    pub url: String,
    pub title: String,
    pub views: f64,
    pub likes: f64,
    pub duration: f64,
    pub text: String,
}

pub fn preview(id: &str, url: &str, cache: &Path, lang: &str) -> Result<Preview, String> {
    let work = cache.join(safe(id));
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    let info_file = work.join("src.info.json");
    if !info_file.is_file() {
        yt(&["--no-warnings", "--skip-download", "--no-playlist", "--write-info-json", "--write-auto-subs", "--sub-langs", ".*-orig", "--sub-format", "json3",
            "-o", &work.join("src.%(ext)s").to_string_lossy(), "--", url])?;
    }
    let info: Value = std::fs::read_to_string(&info_file).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let mut tracks: Vec<PathBuf> = std::fs::read_dir(&work).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.to_string_lossy().ends_with("-orig.json3")).collect();
    tracks.sort();
    let text = crate::transcribe::pick_track(&tracks, info["language"].as_str().unwrap_or(""), lang)
        .and_then(|p| std::fs::read_to_string(p).ok()).and_then(|j| crate::transcribe::parse_json3(&j))
        .map(|t| t.words.iter().map(|w| w.t.as_str()).collect::<Vec<_>>().join(" ")).unwrap_or_default();
    Ok(Preview {
        id: id.to_string(),
        url: url.to_string(),
        title: info["title"].as_str().unwrap_or("").to_string(),
        views: info["view_count"].as_f64().unwrap_or(0.0),
        likes: info["like_count"].as_f64().unwrap_or(0.0),
        duration: info["duration"].as_f64().unwrap_or(0.0),
        text,
    })
}

/// Without a model: well-watched, well-liked, not too long, with something said.
pub fn preview_score(p: &Preview) -> f64 {
    (1.0 + p.views).ln() + 3.0 * (p.likes / p.views.max(1.0) * 30.0).min(1.0) - if p.duration > 1500.0 { 1.0 } else { 0.0 }
        + if p.text.split_whitespace().count() > 200 { 0.5 } else { 0.0 }
}

/// Reads every video's captions first and keeps the `keep` worth downloading; the rest are never downloaded.
pub fn screen(found: Vec<(String, String)>, keep: usize, cache: &Path, lang: &str, use_llm: bool) -> Vec<(String, String)> {
    if found.len() <= keep {
        return found;
    }
    let previews: Vec<Preview> = std::thread::scope(|s| {
        let jobs: Vec<_> = found.iter().map(|(id, url)| s.spawn(move || preview(id, url, cache, lang))).collect();
        jobs.into_iter().filter_map(|j| j.join().ok()?.map_err(|e| eprintln!("edits: could not read a video: {e}")).ok()).collect()
    });
    let mut order: Vec<usize> = (0..previews.len()).collect();
    order.sort_by(|a, b| preview_score(&previews[*b]).total_cmp(&preview_score(&previews[*a])));
    if use_llm {
        let mut p = format!("Bu videolardan müzikli edit (sad edit / viral edit) yapmak için en iyi malzeme olan {keep} tanesini seç. \
            İyi malzeme: güçlü, akılda kalan, duygusu olan konuşmalar ve ilginç görüntüler. Sıkıcı, teknik, liste okuyan videoları seçme.\n\n");
        for (i, v) in previews.iter().enumerate() {
            p.push_str(&format!("[{i}] {} ({:.0} görüntülenme): {}\n", v.title, v.views, v.text.chars().take(500).collect::<String>()));
        }
        p.push_str(&format!("\nSadece JSON dizisi yaz, en iyisi başta, {keep} sayı: [2, 0]\n"));
        let picked = crate::util::run_stdin(&crate::pick::llm_cmd(), &p).ok().and_then(|r| {
            let (a, b) = (r.find('[')?, r.rfind(']')?);
            serde_json::from_str::<Vec<usize>>(r.get(a..=b)?).ok()
        });
        if let Some(v) = picked.filter(|v| v.iter().all(|i| *i < previews.len()) && !v.is_empty()) {
            let rest: Vec<usize> = order.iter().copied().filter(|i| !v.contains(i)).collect();
            order = v.into_iter().chain(rest).collect();
        }
    }
    for (rank, &i) in order.iter().enumerate() {
        let v = &previews[i];
        eprintln!("edits: {} {} ({:.0} views)", if rank < keep { "using  " } else { "skipping" }, v.title, v.views);
    }
    order.into_iter().take(keep).map(|i| (previews[i].id.clone(), previews[i].url.clone())).collect()
}

/// The picture for one moment: (file, where the moment starts inside it). Online, only that stretch is downloaded.
pub fn clip(src: &Source, start: f64, end: f64) -> Result<(PathBuf, f64), String> {
    if let Some(file) = &src.local {
        return Ok((file.clone(), start));
    }
    let out = src.work.join(format!("sec-{start:.2}-{end:.2}.mp4"));
    if !out.is_file() {
        yt(&[
            "--no-warnings", "--no-progress", "--no-playlist", "-f", VIDEO, "--merge-output-format", "mp4",
            // re-encoded at the cut so the clip starts exactly at `start` and captions stay in sync
            "--download-sections", &format!("*{start:.2}-{end:.2}"), "--force-keyframes-at-cuts",
            "-o", &out.with_extension("%(ext)s").to_string_lossy(), "--", &src.url,
        ])?;
    }
    if !out.is_file() {
        return Err(format!("yt-dlp did not produce {}", out.display()));
    }
    Ok((out, 0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_links_point_at_uploads() {
        assert_eq!(channel_videos("https://www.youtube.com/@Avenoxai"), "https://www.youtube.com/@Avenoxai/videos");
        assert_eq!(channel_videos("https://www.youtube.com/@Avenoxai/"), "https://www.youtube.com/@Avenoxai/videos");
        assert_eq!(channel_videos("https://www.youtube.com/@Avenoxai/shorts"), "https://www.youtube.com/@Avenoxai/shorts");
        assert_eq!(channel_videos("https://www.youtube.com/watch?v=abc"), "https://www.youtube.com/watch?v=abc");
    }

    #[test]
    fn listing_lines() {
        let url = "https://www.youtube.com/watch?v=abc";
        assert_eq!(listed(&format!("abc\t472.0\t{url}"), Some(1800.0)), Some(("abc".into(), url.into())));
        assert_eq!(listed(&format!("abc\t4000\t{url}"), Some(1800.0)), None); // too long for a topic search
        assert!(listed(&format!("abc\t4000\t{url}"), None).is_some()); // a link you gave is never filtered
        assert!(listed(&format!("abc\tNA\t{url}"), Some(1800.0)).is_some()); // unknown length is kept
        assert_eq!(listed("abc\t10\tNA", None), None);
        assert_eq!(listed("garbage", None), None);
    }

    #[test]
    fn matches_the_song_title() {
        let t = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let titles = t(&["Raya (Slowed)", "NUEKI, Mc Menor Do Alvorada - MONTAGEM ROYAL (SLOWED)", "MONTAGEM ROYAL (slowed + reverb)"]);
        assert_eq!(best_title("Montagem Royal - Slowed & Reverb", &titles), Some(2));
        assert_eq!(best_title("x", &[]), None);
    }

    #[test]
    fn preview_ranking() {
        let p = |views: f64, likes: f64, duration: f64| Preview { id: String::new(), url: String::new(), title: String::new(), views, likes, duration, text: String::new() };
        assert!(preview_score(&p(50000.0, 3000.0, 500.0)) > preview_score(&p(50000.0, 100.0, 500.0)));
        assert!(preview_score(&p(50000.0, 3000.0, 500.0)) > preview_score(&p(50000.0, 3000.0, 3000.0)));
        assert!(preview_score(&p(0.0, 0.0, 0.0)).is_finite());
        assert_eq!(screen(vec![("a".into(), "u".into())], 3, Path::new("/nonexistent"), "tr", false).len(), 1);
    }

    #[test]
    fn safe_names() {
        assert_eq!(safe("a/b c.mp4"), "a_b_c_mp4");
    }
}
