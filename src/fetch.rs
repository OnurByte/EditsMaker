//! Downloads as little as possible: audio, captions and viewer data first; video only for the moments that get cut.
use crate::util::{run, yt};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone)]
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

/// Frames a second of a video's picture (30 when ffprobe cannot tell).
pub fn fps(video: &Path) -> f64 {
    let v = video.to_string_lossy();
    let out = run(None, "ffprobe", &["-v", "error", "-select_streams", "V:0", "-show_entries", "stream=r_frame_rate", "-of", "csv=p=0", v.as_ref()]).unwrap_or_default();
    let (a, b) = out.trim().split_once('/').unwrap_or((out.trim(), "1"));
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(a), Ok(b)) if a > 0.0 && b > 0.0 => a / b,
        _ => 30.0,
    }
}

/// Frame size (0, 0 if ffprobe cannot tell).
pub fn dims(video: &Path) -> (u32, u32) {
    let v = video.to_string_lossy();
    // V, not v: a real picture, never an mp3's cover art (that made every library song look like an edit with dialogue)
    let out = run(None, "ffprobe", &["-v", "error", "-select_streams", "V:0", "-show_entries", "stream=width,height", "-of", "csv=p=0", v.as_ref()]).unwrap_or_default();
    let mut it = out.trim().split(',').filter_map(|x| x.parse().ok());
    (it.next().unwrap_or(0), it.next().unwrap_or(0))
}

fn wav(input: &Path, out: &Path) -> Result<(), String> {
    run(None, "ffmpeg", &["-v", "error", "-y", "-i", &input.to_string_lossy(), "-vn", "-ar", "16000", "-ac", "1", &out.to_string_lossy()]).map(|_| ())
        .map_err(|e| if e.contains("does not contain any stream") { format!("{} has no sound", input.display()) } else { e })
}

/// A bare channel link lists tabs, not videos; point it at the uploads.
/// "@name" alone is a YouTube channel, never a topic to search for.
fn handle_url(input: &str) -> String {
    let h = input.trim();
    let bare = h.len() > 1 && h.starts_with('@') && h[1..].chars().all(|c| c.is_alphanumeric() || "._-".contains(c));
    if bare { format!("https://www.youtube.com/{h}/videos") } else { input.to_string() }
}

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
    let input = &handle_url(input);
    let is_url = input.starts_with("http://") || input.starts_with("https://");
    let target = if is_url { channel_videos(input) } else { format!("ytsearch{videos}:{input}") };
    // "--" so a topic starting with "-" can never be read as a yt-dlp option
    let out = yt(&["--no-warnings", "--flat-playlist", "--playlist-end", &videos.to_string(), "--print", "%(id)s\t%(duration)s\t%(webpage_url,url)s\t%(channel,uploader)s", "--", &target])?;
    // say whose videos these are, so a wrong channel shows in the log instead of in the edit
    if let Some(ch) = out.lines().find_map(|l| l.split('\t').nth(3)).map(str::trim).filter(|c| !c.is_empty() && *c != "NA") {
        eprintln!("edits: videos from \"{ch}\" ({target})");
    }
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
            (w.iter().filter(|x| tw.contains(x)).count(), std::cmp::Reverse(tw.len()))
        };
        score(a).cmp(&score(b))
    }).map(|x| x.0)
}

/// "Artist - Title" rows of a kworb.net TikTok chart page, in chart order.
pub fn parse_chart(html: &str) -> Vec<String> {
    let row = regex::Regex::new(r"(?s)<tr>(.*?)</tr>").unwrap();
    let cell = regex::Regex::new(r"(?s)<td[^>]*>(.*?)</td>").unwrap();
    let tag = regex::Regex::new(r"<[^>]+>").unwrap();
    let unescape = |t: &str| t.replace("&amp;", "&").replace("&#39;", "'").replace("&quot;", "\"").replace("&lt;", "<").replace("&gt;", ">");
    row.captures_iter(html).filter_map(|r| {
        cell.captures_iter(&r[1]).map(|c| unescape(tag.replace_all(&c[1], "").trim())).filter(|t| t.contains(" - ")).max_by_key(|t| t.len())
    }).collect()
}

/// The kworb.net page of a chart: TikTok's (`trend`) or Spotify's daily one, for a two-letter country (Spotify: also "global").
pub fn chart_url(kind: &str, cc: &str) -> String {
    match kind {
        "spotify" => format!("https://kworb.net/spotify/country/{cc}_daily.html"),
        _ => format!("https://kworb.net/charts/tiktok/{cc}.html"),
    }
}

/// What is trending in a country right now (two-letter code), from kworb.net's public charts: TikTok's sounds, or
/// with `kind` "spotify" the most streamed songs of the day.
pub fn trends(kind: &str, country: &str) -> Result<Vec<String>, String> {
    let cc: String = if country.eq_ignore_ascii_case("global") { "global".into() } else { country.chars().filter(|c| c.is_ascii_alphabetic()).take(2).collect::<String>().to_lowercase() };
    let name = if kind == "spotify" { "Spotify" } else { "TikTok" };
    let html = run(None, "curl", &["-fsSL", "--max-time", "20", "-A", "Mozilla/5.0", &chart_url(kind, &cc)])
        .map_err(|e| format!("could not read the {name} chart for \"{cc}\": {e}"))?;
    let list = parse_chart(&html);
    if list.is_empty() {
        return Err(format!("the {name} chart for \"{cc}\" had no songs"));
    }
    Ok(list)
}

/// The chart to use: --lang's country when it differs from the system's language (tr -> tr, en -> us), else the system's.
pub fn country() -> String {
    let (lang, cc) = crate::lang::system();
    match crate::lang::code() {
        c if c == lang => cc,
        "en" => "us".into(),
        c => c.into(),
    }
}

/// `--music trend`, `trend:3` or `trend:us:3`: the song at that place on TikTok's chart, found on YouTube;
/// `spotify`, `spotify:3`, `spotify:us:3` the same on Spotify's daily chart.
/// A file or link is used as it is; any other text is a song title, found on YouTube.
pub fn music(spec: &str) -> Result<String, String> {
    let kind = if spec.starts_with("spotify") { "spotify" } else { "trend" };
    let Some(rest) = spec.strip_prefix(kind).filter(|r| r.is_empty() || r.starts_with(':')) else {
        // a file or a link as it is; any other text is a song title to look up
        let is_link = spec.starts_with("http://") || spec.starts_with("https://");
        if is_link {
            return Ok(spec.to_string());
        }
        // absolute: ffmpeg runs in the work folder, where a relative path points nowhere
        if Path::new(spec).exists() {
            return std::path::absolute(spec).map(|p| p.to_string_lossy().into_owned()).map_err(|e| e.to_string());
        }
        // a song of yours by name before a search
        return match crate::library::find(&["music"], spec) {
            Some(f) => {
                eprintln!("edits: music from your library: {}", f.display());
                Ok(f.to_string_lossy().into_owned())
            }
            None => find_song(spec),
        };
    };
    let parts: Vec<&str> = rest.split(':').filter(|p| !p.is_empty()).collect();
    let home = country();
    let (cc, n) = match parts[..] {
        [] => (home.as_str(), 1),
        [x] => x.parse().map_or((x, 1), |n| (home.as_str(), n)),
        [c, x, ..] => (c, x.parse().unwrap_or(1)),
    };
    let list = trends(kind, cc)?;
    let song = list.get(n.max(1) - 1).ok_or(format!("the chart has {} songs", list.len()))?;
    eprintln!("edits: {} #{n} in {cc}: {song}", if kind == "spotify" { "Spotify" } else { "trending" });
    find_song(song)
}

/// A song by name: the YouTube result whose title matches it best (the first result alone varies between calls).
pub fn find_song(title: &str) -> Result<String, String> {
    let out = yt(&["--no-warnings", "--flat-playlist", "--print", "%(title)s\t%(duration)s\t%(webpage_url,url)s", "--", &format!("ytsearch8:{title}")])?;
    let found = song_results(&out);
    let titles: Vec<String> = found.iter().map(|f| f.0.clone()).collect();
    best_title(title, &titles).map(|i| found[i].1.clone()).ok_or(format!("no song found for \"{title}\""))
}

/// Search lines (title, seconds, link) as (title, link), song-length ones only when there are any: a 20-minute
/// "1 hour loop" or a whole album shares the song's title but not its clock.
fn song_results(out: &str) -> Vec<(String, String)> {
    let all: Vec<(String, Option<f64>, String)> = out.lines().filter_map(|l| {
        let mut f = l.split('\t');
        let (t, d, u) = (f.next()?, f.next()?, f.next()?);
        Some((t.to_string(), d.trim().parse().ok(), u.trim().to_string()))
    }).collect();
    let song = |d: &Option<f64>| d.is_none_or(|d| d <= 600.0);
    let keep = all.iter().any(|x| song(&x.1));
    all.into_iter().filter(|x| !keep || song(&x.1)).map(|(t, _, u)| (t, u)).collect()
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

/// A finished edit given as a link (X, TikTok, Instagram, YouTube): the whole video, downloaded once into the
/// cache and named UPLOADER-ID.mp4. A file is returned as it is.
pub fn reference(v: &str) -> Result<PathBuf, String> {
    if Path::new(v).is_file() || !(v.starts_with("http://") || v.starts_with("https://")) {
        return Ok(PathBuf::from(v));
    }
    let dir = crate::util::home().join(".cache/edits/refs").join(safe(v.split('?').next().unwrap_or(v).trim_end_matches('/').rsplit("//").next().unwrap_or(v)));
    let found = |d: &Path| std::fs::read_dir(d).into_iter().flatten().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|e| e == "mp4"));
    if let Some(f) = found(&dir) {
        return Ok(f);
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    yt(&["--no-warnings", "--no-progress", "--no-playlist", "-f", VIDEO, "--merge-output-format", "mp4",
        "-o", &dir.join("%(uploader_id,uploader)s-%(id)s.%(ext)s").to_string_lossy(), "--", v])?;
    found(&dir).ok_or(format!("yt-dlp downloaded no video from {v}"))
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
        use crate::lang::prompt;
        let mut p = prompt("videos", "head", &[("n", keep.to_string())]);
        for (i, v) in previews.iter().enumerate() {
            p.push_str(&prompt("videos", "item", &[("i", i.to_string()), ("title", v.title.clone()), ("views", format!("{:.0}", v.views)), ("text", v.text.chars().take(500).collect())]));
        }
        p.push_str(&prompt("videos", "tail", &[("n", keep.to_string())]));
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
    fn a_song_is_song_length() {
        let out = "Umrumdışı - Mabel Matiz\t1214\thttps://y/a\nMabel Matiz - Umrumdışı\t221\thttps://y/b\nUmrumdışı (canlı)\tNA\thttps://y/c\n";
        assert_eq!(song_results(out), vec![("Mabel Matiz - Umrumdışı".into(), "https://y/b".into()), ("Umrumdışı (canlı)".into(), "https://y/c".into())]);
        // only long ones: better one of them than none
        assert_eq!(song_results("Album\t3000\thttps://y/d\n").len(), 1);
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
    fn reads_a_tiktok_chart() {
        let html = "<table><tr><th>Pos</th><th>Artist - Title</th></tr>\
            <tr><td>1</td><td>+2</td><td><a href='x'>Ravi İncigöz - Maşallah</a></td><td>9</td></tr>\
            <tr><td>2</td><td>=</td><td>Heijan &amp; Muti - Yansın Geceler</td></tr></table>";
        assert_eq!(parse_chart(html), vec!["Ravi İncigöz - Maşallah", "Heijan & Muti - Yansın Geceler"]);
        assert!(parse_chart("<html></html>").is_empty());
        assert_eq!(music("https://youtu.be/x").unwrap(), "https://youtu.be/x");
        // Spotify's daily chart: artist and title are two links in one cell
        let sp = r#"<tr><td>1</td><td class="text mp"><div><a href="../artist/1.html">Mabel Matiz</a> - <a href="../track/2.html">Umrumdışı</a></div></td><td>572,574</td></tr>"#;
        assert_eq!(parse_chart(sp), vec!["Mabel Matiz - Umrumdışı"]);
        assert_eq!(chart_url("spotify", "tr"), "https://kworb.net/spotify/country/tr_daily.html");
        assert_eq!(chart_url("trend", "us"), "https://kworb.net/charts/tiktok/us.html");
    }

    #[test]
    fn safe_names() {
        assert_eq!(safe("a/b c.mp4"), "a_b_c_mp4");
    }

    #[test]
    fn a_bare_handle_is_a_channel() {
        assert_eq!(super::handle_url("@avenoxai"), "https://www.youtube.com/@avenoxai/videos");
        assert_eq!(super::handle_url("@ topic words"), "@ topic words");
        assert_eq!(super::handle_url("sad edits"), "sad edits");
    }
}
