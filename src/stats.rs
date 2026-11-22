//! What worked, measured on the posted videos themselves. Every finished edit goes into a render log with the sound
//! of it (a loudness curve, ten points a second); `edits stats` lists a channel's posts with their views, likes and
//! comments, recognises which of them are edits made here by that sound, and says which presets, songs and looks
//! people watched. Talking clips that did best and worst become examples in the taste file.
use crate::util::{home, run_bytes, yt};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn log_path() -> PathBuf {
    home().join(".config/edits/renders.jsonl")
}

fn rated_path() -> PathBuf {
    home().join(".config/edits/rated.txt")
}

/// Loudness in dB, ten points a second, of any audio or video file.
pub fn envelope(media: &Path) -> Result<Vec<f64>, String> {
    let raw = run_bytes("ffmpeg", &["-v", "error", "-i", &media.to_string_lossy(), "-vn", "-ac", "1", "-ar", "2000", "-f", "f32le", "-"])?;
    let x: Vec<f32> = raw.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect();
    Ok(x.chunks(200).filter(|c| c.len() == 200).map(|c| {
        let rms = (c.iter().map(|v| (*v as f64).powi(2)).sum::<f64>() / 200.0).sqrt();
        (20.0 * rms.max(1e-4).log10()).round()
    }).collect())
}

/// A finished edit into the render log: what it was made with and its sound. Never fails the render.
pub fn record(clip: &Path, side: &Value) {
    let env = match envelope(clip) {
        Ok(e) => e,
        Err(e) => return eprintln!("edits: not in the render log ({e})"),
    };
    let (day, time) = crate::util::now().unwrap_or(("undated".into(), "00.00".into()));
    let mut line = json!({ "made": format!("{day} {time}"), "clip": clip, "env": env });
    for k in ["kind", "preset", "song", "look", "made_from", "seconds", "text"] {
        if !side[k].is_null() {
            line[k] = side[k].clone();
        }
    }
    if let Some(v) = side["voice"]["text"].as_str() {
        line["text"] = json!(v);
    }
    use std::io::Write;
    let _ = std::fs::OpenOptions::new().create(true).append(true).open(log_path()).and_then(|mut f| writeln!(f, "{line}"));
}

/// How alike two loudness curves are, -1..1: the best correlation with one sliding up to 1.5 s against the other
/// (a platform trims or delays a little), over at least 70% of the shorter one.
pub fn alike(a: &[f64], b: &[f64]) -> f64 {
    let short = a.len().min(b.len());
    if short < 20 {
        return -1.0;
    }
    let mut best = -1.0f64;
    for lag in -15i64..=15 {
        let pairs: Vec<(f64, f64)> = a.iter().enumerate().filter_map(|(i, x)| Some((*x, *b.get(usize::try_from(i as i64 + lag).ok()?)?))).collect();
        if pairs.len() * 10 < short * 7 {
            continue;
        }
        let n = pairs.len() as f64;
        let (ma, mb) = (pairs.iter().map(|p| p.0).sum::<f64>() / n, pairs.iter().map(|p| p.1).sum::<f64>() / n);
        let (mut cov, mut va, mut vb) = (0.0, 0.0, 0.0);
        for (x, y) in &pairs {
            cov += (x - ma) * (y - mb);
            va += (x - ma).powi(2);
            vb += (y - mb).powi(2);
        }
        if va > 0.0 && vb > 0.0 {
            best = best.max(cov / (va * vb).sqrt());
        }
    }
    best
}

#[derive(Clone, Debug, Default)]
pub struct Post {
    pub id: String,
    pub url: String,
    pub title: String,
    pub seconds: f64,
    pub views: u64,
    pub likes: u64,
    pub comments: u64,
    /// "2026-11-21", "" when the site does not say
    pub day: String,
}

/// A post from yt-dlp's info JSON.
pub fn post(v: &Value) -> Post {
    let n = |k: &str| v[k].as_u64().unwrap_or(0);
    let d = v["upload_date"].as_str().unwrap_or("");
    Post {
        id: v["id"].as_str().unwrap_or("").into(),
        url: v["webpage_url"].as_str().or(v["url"].as_str()).unwrap_or("").into(),
        title: v["title"].as_str().unwrap_or("").replace('\n', " "),
        seconds: v["duration"].as_f64().unwrap_or(0.0),
        views: n("view_count"),
        likes: n("like_count"),
        comments: n("comment_count"),
        day: if d.len() == 8 { format!("{}-{}-{}", &d[..4], &d[4..6], &d[6..]) } else { String::new() },
    }
}

/// 1234 as 1.2k, 1234567 as 1.2M.
pub fn short(n: u64) -> String {
    match n {
        0..=999 => n.to_string(),
        1_000..=999_999 => format!("{:.1}k", n as f64 / 1e3),
        _ => format!("{:.1}M", n as f64 / 1e6),
    }
}

fn median(mut v: Vec<u64>) -> u64 {
    v.sort();
    v.get(v.len() / 2).copied().unwrap_or(0)
}

/// Where a channel's or profile's posts are listed: "@name" is a YouTube channel, whose Shorts are the edits.
pub fn target(input: &str) -> String {
    let t = input.trim().trim_end_matches('/');
    if t.starts_with('@') && !t.contains('/') {
        return format!("https://www.youtube.com/{t}/shorts");
    }
    let channel = t.contains("youtube.com/@") || t.contains("youtube.com/channel/") || t.contains("youtube.com/c/");
    if channel && !["/shorts", "/videos", "/streams"].iter().any(|x| t.ends_with(x)) { format!("{t}/shorts") } else { t.to_string() }
}

/// A logged render that could be this post: as long (the site rounds to whole seconds) and made before it went up.
pub fn could_be(r: &Value, p: &Post) -> bool {
    let len = r["seconds"].as_f64().unwrap_or(-9.0);
    let made = r["made"].as_str().unwrap_or("").get(..10).unwrap_or("");
    (len - p.seconds).abs() <= 1.5 && (p.day.is_empty() || made.is_empty() || made <= p.day.as_str())
}

/// A taste-file example from a clip people liked or skipped (also what `edits rate` writes).
pub fn note(text: &str, liked: bool) -> Result<(), String> {
    let mark = crate::lang::prompt("notes", if liked { "liked" } else { "disliked" }, &[]);
    let mut taste = crate::pick::taste();
    taste.push_str(&format!("- {mark}: \"{text}\"\n"));
    std::fs::write(crate::pick::taste_path(), taste).map_err(|e| e.to_string())
}

/// `edits stats <@channel|link> [N]`: the latest N posts (default 30), matched to the edits that made them.
pub fn cmd(args: &[String]) -> Result<(), String> {
    let input = args.first().ok_or("usage: edits stats <@channel | channel or profile link> [N]")?;
    let n: usize = args.get(1).map(|x| x.parse().ok().filter(|n| (1..=200).contains(n)).ok_or("N must be 1-200")).transpose()?.unwrap_or(30);
    let mut t = target(input);
    let list = |t: &str| yt(&["--no-warnings", "--flat-playlist", "--playlist-end", &n.to_string(), "--print", "%(webpage_url,url)s", "--", t]);
    let listed = match list(&t) {
        // a channel that posts no Shorts: its videos
        Err(e) if e.contains("shorts tab") && t.ends_with("/shorts") => {
            t = format!("{}/videos", t.trim_end_matches("/shorts"));
            list(&t)?
        }
        r => r?,
    };
    let urls: Vec<String> = listed.lines().map(str::trim).filter(|l| l.starts_with("http")).map(String::from).collect();
    if urls.is_empty() {
        return Err(format!("no posts found at {t}"));
    }
    eprintln!("edits: reading {} posts from {t}", urls.len());
    let mut posts: Vec<Post> = vec![];
    for batch in urls.chunks(4) {
        posts.extend(std::thread::scope(|s| {
            let jobs: Vec<_> = batch.iter().map(|u| s.spawn(move || yt(&["--no-warnings", "--skip-download", "--no-playlist", "-j", "--", u]).ok().and_then(|j| serde_json::from_str::<Value>(&j).ok()).map(|v| post(&v)))).collect();
            jobs.into_iter().filter_map(|j| j.join().ok().flatten()).collect::<Vec<_>>()
        }));
    }
    let renders: Vec<Value> = std::fs::read_to_string(log_path()).unwrap_or_default().lines().filter_map(|l| serde_json::from_str(l).ok()).collect();
    let cache = home().join(".cache/edits/stats");
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    // only posts that could be one of the renders get their sound fetched (a few hundred KB each, kept as a curve)
    let mut matched: Vec<(Post, Option<Value>)> = vec![];
    for batch in posts.chunks(4) {
        matched.extend(std::thread::scope(|s| {
            let jobs: Vec<_> = batch.iter().map(|p| {
                let (renders, cache) = (&renders, &cache);
                s.spawn(move || {
                    let cands: Vec<&Value> = renders.iter().filter(|r| could_be(r, p)).collect();
                    if cands.is_empty() {
                        return (p.clone(), None);
                    }
                    let Some(env) = post_sound(p, cache) else { return (p.clone(), None) };
                    let best = cands.into_iter().map(|r| {
                        let e: Vec<f64> = r["env"].as_array().into_iter().flatten().filter_map(Value::as_f64).collect();
                        (alike(&env, &e), r)
                    }).max_by(|a, b| a.0.total_cmp(&b.0));
                    (p.clone(), best.filter(|b| b.0 >= 0.7).map(|b| b.1.clone()))
                })
            }).collect();
            jobs.into_iter().filter_map(|j| j.join().ok()).collect::<Vec<_>>()
        }));
    }
    print!("{}", report(&matched));
    learn(&matched)
}

/// The post's loudness curve: fetched once, kept in the cache by its id.
fn post_sound(p: &Post, cache: &Path) -> Option<Vec<f64>> {
    let kept = cache.join(format!("{}.json", crate::fetch::safe(&p.id)));
    if let Some(e) = std::fs::read_to_string(&kept).ok().and_then(|s| serde_json::from_str(&s).ok()) {
        return Some(e);
    }
    let got = yt(&["--no-warnings", "--no-progress", "--no-playlist", "-f", "ba/b", "--print", "after_move:filepath",
        "-o", &cache.join(format!("{}.%(ext)s", crate::fetch::safe(&p.id))).to_string_lossy(), "--", &p.url]).ok()?;
    let f = PathBuf::from(got.lines().last()?.trim());
    let env = envelope(&f);
    let _ = std::fs::remove_file(&f);
    let env = env.ok()?;
    let _ = std::fs::write(&kept, serde_json::to_string(&env).unwrap_or_default());
    Some(env)
}

/// What an edit was made with, as one label per kind of choice.
fn label(r: &Value, key: &str) -> Option<String> {
    match key {
        "preset" => r["preset"].as_str().map(String::from).or_else(|| r["kind"].as_str().map(|k| format!("({k})"))),
        k => r[k].as_str().filter(|s| !s.is_empty()).map(String::from),
    }
}

/// The table of posts, then each preset, song and look with how its posts did, best first.
pub fn report(matched: &[(Post, Option<Value>)]) -> String {
    let mut s = String::new();
    let made = matched.iter().filter(|m| m.1.is_some()).count();
    s.push_str(&format!("{} posts, {made} recognised as edits made here\n\n{:>7} {:>6} {:>5}  {:<10}  post / made with\n", matched.len(), "views", "likes", "comm.", "posted"));
    for (p, r) in matched {
        let with = r.as_ref().map(|r| ["preset", "song", "look"].iter().filter_map(|k| Some(format!("{k} {}", label(r, k)?))).collect::<Vec<_>>().join(" · ")).unwrap_or_default();
        s.push_str(&format!("{:>7} {:>6} {:>5}  {:<10}  {}\n", short(p.views), short(p.likes), short(p.comments), p.day, p.title.chars().take(60).collect::<String>()));
        if !with.is_empty() {
            s.push_str(&format!("{:>33}{with}\n", "↳ "));
        }
    }
    for key in ["preset", "song", "look"] {
        let mut groups: Vec<(String, Vec<&Post>)> = vec![];
        for (p, r) in matched {
            if let Some(l) = r.as_ref().and_then(|r| label(r, key)) {
                match groups.iter_mut().find(|g| g.0 == l) {
                    Some(g) => g.1.push(p),
                    None => groups.push((l, vec![p])),
                }
            }
        }
        if groups.is_empty() {
            continue;
        }
        let mut rows: Vec<(u64, String)> = groups.iter().map(|(l, ps)| {
            let views = median(ps.iter().map(|p| p.views).collect());
            let likes: u64 = ps.iter().map(|p| p.likes).sum();
            let seen: u64 = ps.iter().map(|p| p.views).sum::<u64>().max(1);
            (views, format!("  {l:<28} {} post{}, median {} views, {:.1}% liked", ps.len(), if ps.len() == 1 { "" } else { "s" }, short(views), likes as f64 * 100.0 / seen as f64))
        }).collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.0));
        s.push_str(&format!("\nby {key}:\n"));
        rows.iter().for_each(|r| s.push_str(&format!("{}\n", r.1)));
    }
    if made == 0 {
        s.push_str("\nnone of these posts is an edit in the render log (edits logs every edit it makes from this version on)\n");
    }
    s
}

/// Talking clips people watched most and least become liked and disliked examples in the taste file, once each.
fn learn(matched: &[(Post, Option<Value>)]) -> Result<(), String> {
    let mut clips: Vec<(&Post, &Value)> = matched.iter().filter_map(|(p, r)| Some((p, r.as_ref()?))).filter(|(_, r)| r["kind"] == "clip" && r["text"].as_str().is_some_and(|t| !t.trim().is_empty())).collect();
    if clips.len() < 4 {
        return Ok(());
    }
    clips.sort_by_key(|c| std::cmp::Reverse(c.0.views));
    let q = (clips.len() / 4).max(1);
    let done = std::fs::read_to_string(rated_path()).unwrap_or_default();
    let mut noted = 0;
    for (i, (_, r)) in clips.iter().enumerate() {
        let liked = i < q;
        if !liked && i < clips.len() - q {
            continue;
        }
        let clip = r["clip"].as_str().unwrap_or("");
        if done.lines().any(|l| l == clip) {
            continue;
        }
        note(r["text"].as_str().unwrap_or("").trim(), liked)?;
        use std::io::Write;
        let _ = std::fs::OpenOptions::new().create(true).append(true).open(rated_path()).and_then(|mut f| writeln!(f, "{clip}"));
        noted += 1;
    }
    if noted > 0 {
        println!("\nedits: {noted} clip(s) noted in {} as liked or disliked, by their views", crate::pick::taste_path().display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_same_sound_is_recognised() {
        let a: Vec<f64> = (0..180).map(|i| -30.0 + 10.0 * ((i as f64) * 0.37).sin() + if i % 17 == 0 { 8.0 } else { 0.0 }).collect();
        // the platform re-encoded it a little quieter and 0.4 s late
        let mut b: Vec<f64> = vec![-60.0; 4];
        b.extend(a.iter().map(|x| x - 3.0 + if (*x as i64) % 3 == 0 { 0.5 } else { 0.0 }));
        assert!(alike(&a, &b) > 0.9, "{}", alike(&a, &b));
        let other: Vec<f64> = (0..180).map(|i| -30.0 + 10.0 * ((i as f64) * 0.11 + 2.0).cos()).collect();
        assert!(alike(&a, &other) < 0.5, "{}", alike(&a, &other));
        assert_eq!(alike(&a[..10], &b), -1.0);
    }

    #[test]
    fn posts_and_candidates() {
        let p = post(&json!({"id": "x1", "webpage_url": "https://t/x1", "title": "a\nb", "duration": 18.0, "view_count": 12345, "like_count": 900, "upload_date": "20261121"}));
        assert_eq!((p.title.as_str(), p.views, p.day.as_str(), p.comments), ("a b", 12345, "2026-11-21", 0));
        assert!(could_be(&json!({"seconds": 18.03, "made": "2026-11-20 21.04"}), &p));
        assert!(!could_be(&json!({"seconds": 18.03, "made": "2026-11-22 09.00"}), &p));
        assert!(!could_be(&json!({"seconds": 24.0, "made": "2026-11-20 21.04"}), &p));
        assert_eq!(short(950), "950");
        assert_eq!(short(12_345), "12.3k");
        assert_eq!(short(2_500_000), "2.5M");
        assert_eq!(target("@avenoxai"), "https://www.youtube.com/@avenoxai/shorts");
        assert_eq!(target("https://www.youtube.com/@avenoxai/"), "https://www.youtube.com/@avenoxai/shorts");
        assert_eq!(target("https://www.tiktok.com/@someone"), "https://www.tiktok.com/@someone");
    }

    #[test]
    fn a_report_ranks_what_worked() {
        let p = |v: u64, t: &str| Post { views: v, likes: v / 10, title: t.into(), day: "2026-11-21".into(), ..Default::default() };
        let r = |preset: &str| Some(json!({"preset": preset, "song": "X - Y", "look": "bw"}));
        let m = vec![(p(500, "a"), r("calm")), (p(20_000, "b"), r("rush")), (p(30_000, "c"), r("rush")), (p(10, "d"), None)];
        let s = report(&m);
        assert!(s.starts_with("4 posts, 3 recognised"), "{s}");
        let rush = s.find("  rush").unwrap();
        assert!(rush < s.find("  calm").unwrap() && s.contains("2 posts, median 30.0k views, 10.0% liked"), "{s}");
    }
}
