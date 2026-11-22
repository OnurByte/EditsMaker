//! Automations, as in Tehom: follow a YouTube channel, a Kick channel or any RSS/Atom feed; every new video is made
//! into clips with the options given when it was followed, and the clips can be posted to connected accounts. A feed
//! pack (or `--titles`) follows articles instead: each new item is made from its headline (rss.rs), like n8n's
//! RSS in, video made, posted out.
//! `edits watch` does the checking (or `edits service install` keeps it running in the background).
use crate::util::{curl, home};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Follow {
    pub source: String,
    /// the edits options each new video is made with (--preset, --music, -n, ...)
    #[serde(default)]
    pub args: Vec<String>,
    /// accounts the clips go to (names, platforms or all); empty: made, not posted
    #[serde(default)]
    pub post: String,
    #[serde(default)]
    pub private: bool,
    /// the items are articles, not videos: each is made from its headline
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub titles: bool,
    /// minutes between checks
    pub every: u64,
    #[serde(default)]
    pub last: u64,
    #[serde(default)]
    pub seen: Vec<String>,
    #[serde(default)]
    pub failed: BTreeMap<String, u32>,
}

const TRIES: u32 = 3;
const PER_CHECK: usize = 3;

fn file() -> PathBuf {
    home().join(".config/edits/follows.json")
}
fn log_file() -> PathBuf {
    home().join(".config/edits/follow.log")
}
fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

pub fn all() -> Vec<Follow> {
    std::fs::read_to_string(file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}
fn save(l: &[Follow]) -> Result<(), String> {
    std::fs::create_dir_all(file().parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(file(), serde_json::to_string_pretty(l).unwrap_or_default()).map_err(|e| e.to_string())
}
/// Saves one follow's state, keeping any follow added or removed meanwhile.
fn update(f: &Follow) -> Result<(), String> {
    let mut l = all();
    if let Some(x) = l.iter_mut().find(|x| x.source == f.source) {
        *x = f.clone();
        save(&l)?;
    }
    Ok(())
}

fn log(msg: &str) {
    let (d, t) = crate::util::now().unwrap_or_default();
    let line = format!("{d} {t}  {msg}");
    eprintln!("edits: {msg}");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(log_file()) {
        let _ = writeln!(f, "{line}");
    }
}

/// A video found at a source: (id, link, title).
type Found = (String, String, String);

/// The source's latest videos, newest first.
pub fn latest(source: &str) -> Result<Vec<Found>, String> {
    let s = source.trim_end_matches('/');
    if let Some(slug) = s.strip_prefix("https://kick.com/").or(s.strip_prefix("https://www.kick.com/")).filter(|x| !x.contains('/')) {
        let (code, body) = curl(&[("url", &format!("https://kick.com/api/v2/channels/{slug}/videos")), ("header", "accept: application/json"), ("user-agent", "Mozilla/5.0")])?;
        let v: Value = serde_json::from_str(&body).map_err(|_| format!("Kick did not list {slug}'s videos (HTTP {code}); it may be blocking automated requests"))?;
        return Ok(kick(&v));
    }
    let youtube = ["youtube.com", "youtu.be"].iter().any(|h| s.contains(h));
    if !youtube {
        let (code, body) = curl(&[("url", s), ("user-agent", "Mozilla/5.0")])?;
        if (200..300).contains(&code) && body.trim_start().starts_with('<') {
            if let Some(items) = feed(&body) {
                return Ok(items);
            }
        }
    }
    // a YouTube channel, a playlist or anything else yt-dlp can list
    Ok(crate::fetch::resolve(s, 10)?.into_iter().map(|(id, url)| (id, url, String::new())).collect())
}

fn kick(v: &Value) -> Vec<Found> {
    v.as_array().into_iter().flatten().filter(|i| i["is_live"] != true).filter_map(|i| {
        let u = i["video"]["uuid"].as_str().filter(|u| u.len() == 36)?;
        let t = i["session_title"].as_str().or(i["title"].as_str()).unwrap_or("Kick VOD");
        Some((format!("kick:{u}"), format!("https://kick.com/video/{u}"), t.to_string()))
    }).collect()
}

/// An RSS or Atom feed's items: the video file when the item carries one, else its link.
fn feed(xml: &str) -> Option<Vec<Found>> {
    let doc = roxmltree::Document::parse(xml).ok()?;
    let root = doc.root_element();
    if !matches!(root.tag_name().name(), "rss" | "feed" | "RDF") {
        return None;
    }
    let text = |n: roxmltree::Node, tag: &str| n.children().find(|c| c.tag_name().name() == tag).and_then(|c| c.text()).map(|t| t.trim().to_string());
    Some(root.descendants().filter(|n| matches!(n.tag_name().name(), "item" | "entry")).filter_map(|n| {
        let media = n.descendants().find(|c| matches!(c.tag_name().name(), "enclosure" | "content")
            && c.attribute("type").is_some_and(|t| t.starts_with("video/"))).and_then(|c| c.attribute("url"));
        let link = n.children().filter(|c| c.tag_name().name() == "link")
            .find_map(|c| c.attribute("href").filter(|_| c.attribute("rel").is_none_or(|r| r == "alternate")).map(str::to_string).or_else(|| c.text().map(|t| t.trim().to_string())));
        let url = media.map(str::to_string).or(link)?;
        let id = text(n, "videoId").or_else(|| text(n, "guid")).or_else(|| text(n, "id")).unwrap_or_else(|| url.clone());
        Some((id, url, text(n, "title").unwrap_or_default()))
    }).collect())
}

/// `edits follow <channel|feed|kick link> [--every MIN] [--post ACCOUNTS] [--private] [edit options...]`
pub fn add(args: &[String]) -> Result<(), String> {
    let given = args.first().filter(|a| !a.starts_with('-')).ok_or("usage: edits follow <YouTube channel | Kick channel | RSS/Atom feed | feed pack> [--feed NAME] [--titles] [--every MINUTES] [--post ACCOUNT,...|all] [--private] [edit options: --preset, --music, -n ...]")?;
    let mut f = Follow { source: given.clone(), every: 60, ..Default::default() };
    // a feed pack: one of its feeds, its items made from their headlines, in the pack's look
    if !given.contains("://") {
        let (feeds, preset) = crate::rss::resolve(given)?;
        let want = args.iter().position(|a| a == "--feed").and_then(|i| args.get(i + 1)).map(|w| w.to_lowercase());
        let feed = feeds.iter().find(|x| want.as_ref().is_none_or(|w| x.name.to_lowercase().contains(w))).ok_or("no feed like that in the pack")?;
        (f.source, f.titles) = (feed.url.clone(), true);
        if let Some(p) = preset.filter(|_| !args.iter().any(|a| a == "--preset")) {
            f.args.extend(["--preset".into(), p]);
        }
    }
    let source = &f.source.clone();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--every" => {
                f.every = args.get(i + 1).and_then(|v| v.parse().ok()).filter(|m| (5..=1440).contains(m)).ok_or("--every takes 5-1440 minutes")?;
                i += 1;
            }
            "--post" => {
                f.post = args.get(i + 1).cloned().ok_or("--post takes account names (edits accounts)")?;
                crate::social::pick(&f.post)?;
                i += 1;
            }
            "--private" => f.private = true,
            "--titles" => f.titles = true,
            "--feed" => i += 1,
            a => f.args.push(a.to_string()),
        }
        i += 1;
    }
    // the options must make sense now, not at 3 a.m. on the first new video
    crate::parse(&[vec!["x".to_string()], f.args.clone()].concat(), None)?;
    let found = latest(source)?;
    if found.is_empty() {
        return Err(format!("{source} lists nothing now"));
    }
    // what is already there counts as seen: only videos that come out from now on are made
    f.seen = found.iter().map(|x| x.0.clone()).collect();
    let mut l = all();
    l.retain(|x| x.source != f.source);
    println!("following {source}: {} videos there now; new ones become clips every {} min check{}", found.len(), f.every,
        if f.post.is_empty() { String::new() } else { format!(", posted to {}", f.post) });
    l.push(f);
    save(&l)?;
    println!("run `edits watch` (or `edits service install` to keep it running)");
    Ok(())
}

/// `edits follows`: what is followed, and the latest events.
pub fn list() {
    let l = all();
    if l.is_empty() {
        println!("nothing followed yet: edits follow <channel|feed> [options]");
    }
    for (i, f) in l.iter().enumerate() {
        let ago = if f.last == 0 { "never checked".into() } else { format!("checked {} min ago", (now().saturating_sub(f.last)) / 60) };
        println!("{}. {}  every {} min, {ago}{}{}", i + 1, f.source, f.every,
            if f.post.is_empty() { String::new() } else { format!(", posts to {}", f.post) },
            if f.args.is_empty() { String::new() } else { format!("\n   {}", f.args.join(" ")) });
    }
    if let Ok(t) = std::fs::read_to_string(log_file()) {
        let lines: Vec<&str> = t.lines().collect();
        if !lines.is_empty() {
            println!("\nlatest:");
            lines[lines.len().saturating_sub(10)..].iter().for_each(|l| println!("  {l}"));
        }
    }
}

/// `edits unfollow N|SOURCE`
pub fn remove(what: &str) -> Result<(), String> {
    let mut l = all();
    let i = what.parse::<usize>().ok().and_then(|n| n.checked_sub(1)).filter(|&i| i < l.len())
        .or_else(|| l.iter().position(|f| f.source == what)).ok_or(format!("not followed: {what} (edits follows lists them)"))?;
    let f = l.remove(i);
    save(&l)?;
    println!("unfollowed {}", f.source);
    Ok(())
}

/// `edits watch [--once]`: checks every follow when it is due, makes the new videos into clips and posts them.
pub fn watch(args: &[String]) -> Result<(), String> {
    // one watcher at a time, or a clip would be posted twice; the port is the lock and the OS frees it on exit
    let _lock = std::net::TcpListener::bind(("127.0.0.1", 47613)).map_err(|_| "edits watch is already running (the background service?)".to_string())?;
    let once = args.iter().any(|a| a == "--once");
    loop {
        for mut f in all().into_iter().filter(|f| once || now().saturating_sub(f.last) >= f.every * 60) {
            if let Err(e) = check(&mut f) {
                log(&format!("{}: {e}", f.source));
            }
            f.last = now();
            update(&f)?;
        }
        if once {
            return Ok(());
        }
        std::thread::sleep(Duration::from_secs(60));
    }
}

fn check(f: &mut Follow) -> Result<(), String> {
    let found = latest(&f.source)?;
    // oldest first, a few per check so one busy channel cannot hold up the rest
    let new: Vec<Found> = found.into_iter().filter(|x| !f.seen.contains(&x.0)).collect::<Vec<_>>().into_iter().rev().take(PER_CHECK).collect();
    for (id, url, title) in new {
        log(&format!("{}: new video {}", f.source, if title.is_empty() { &url } else { &title }));
        let made = if f.titles { make_from(&[crate::rss::args_for(&title), f.args.clone()].concat()) } else { make(&url, &f.args) };
        match made {
            Ok(clips) => {
                f.seen.push(id.clone());
                f.failed.remove(&id);
                log(&format!("{}: {} clip(s) made", f.source, clips.len()));
                if !f.post.is_empty() {
                    let to = crate::social::pick(&f.post)?;
                    for c in &clips {
                        let cap = Some(crate::social::caption_of(c)).filter(|c| !c.is_empty()).unwrap_or_else(|| title.clone());
                        for a in &to {
                            match crate::social::post(a, c, &cap, f.private) {
                                Ok(u) => log(&format!("posted to {}: {u}", a.name)),
                                Err(e) => log(&format!("posting {} to {} failed: {e}", c.display(), a.name)),
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let n = f.failed.entry(id.clone()).or_insert(0);
                *n += 1;
                if *n >= TRIES {
                    f.seen.push(id.clone());
                    f.failed.remove(&id);
                    log(&format!("{}: gave up on {url} after {TRIES} tries: {e}", f.source));
                } else {
                    log(&format!("{}: {url} failed ({n}/{TRIES}), tried again next check: {e}", f.source));
                }
            }
        }
        update(f)?;
    }
    Ok(())
}

/// Runs edits itself on the video, the way a person would, and returns the clips it made.
fn make(url: &str, args: &[String]) -> Result<Vec<PathBuf>, String> {
    make_from(&[vec![url.to_string()], args.to_vec()].concat())
}

/// Runs `edits ARGS` and returns the clips it made.
pub fn make_from(args: &[String]) -> Result<Vec<PathBuf>, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let out = std::process::Command::new(exe).args(args).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::inherit())
        .output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("edits failed on it (its output above says why)".into());
    }
    let clips: Vec<PathBuf> = String::from_utf8_lossy(&out.stdout).lines()
        .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok()?["clip"].as_str().map(PathBuf::from)).collect();
    if clips.is_empty() { Err("it made no clips".into()) } else { Ok(clips) }
}

/// `edits service install|remove`: `edits watch` kept running in the background, started at login.
pub fn service(args: &[String]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?.to_string_lossy().into_owned();
    let install = match args.first().map(String::as_str) {
        Some("install") => true,
        Some("remove") => false,
        _ => return Err("usage: edits service install|remove".into()),
    };
    let run = |c: &str, a: &[&str]| crate::util::run(None, c, a).map(|_| ());
    if cfg!(target_os = "macos") {
        let plist = home().join("Library/LaunchAgents/com.onurbyte.edits.plist");
        let _ = run("launchctl", &["unload", &plist.to_string_lossy()]);
        if !install {
            let _ = std::fs::remove_file(&plist);
        } else {
            std::fs::create_dir_all(plist.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(&plist, format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>Label</key><string>com.onurbyte.edits</string><key>ProgramArguments</key><array><string>{exe}</string><string>watch</string></array><key>RunAtLoad</key><true/><key>KeepAlive</key><true/><key>StandardErrorPath</key><string>{}</string></dict></plist>\n", log_file().with_extension("err").display())).map_err(|e| e.to_string())?;
            run("launchctl", &["load", "-w", &plist.to_string_lossy()])?;
        }
    } else if cfg!(windows) {
        if install {
            run("schtasks", &["/Create", "/F", "/SC", "ONLOGON", "/TN", "edits-watch", "/TR", &format!("\"{exe}\" watch")])?;
            run("schtasks", &["/Run", "/TN", "edits-watch"])?;
        } else {
            let _ = run("schtasks", &["/End", "/TN", "edits-watch"]);
            run("schtasks", &["/Delete", "/F", "/TN", "edits-watch"])?;
        }
    } else {
        let unit = home().join(".config/systemd/user/edits.service");
        if install {
            std::fs::create_dir_all(unit.parent().unwrap()).map_err(|e| e.to_string())?;
            std::fs::write(&unit, format!("[Unit]\nDescription=edits: follow channels and post their clips\nAfter=network-online.target\n\n[Service]\nExecStart={exe} watch\nRestart=on-failure\nRestartSec=60\n\n[Install]\nWantedBy=default.target\n")).map_err(|e| e.to_string())?;
            run("systemctl", &["--user", "daemon-reload"])?;
            run("systemctl", &["--user", "enable", "--now", "edits.service"])?;
        } else {
            let _ = run("systemctl", &["--user", "disable", "--now", "edits.service"]);
            let _ = std::fs::remove_file(&unit);
            let _ = run("systemctl", &["--user", "daemon-reload"]);
        }
    }
    println!("{}", if install { "edits watch now runs in the background and starts at login (edits follows shows what it did)" } else { "background service removed" });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feeds_give_video_links() {
        let rss = r#"<rss><channel><item><title>One</title><guid>a1</guid><link>https://x.com/1</link><enclosure url="https://x.com/1.mp4" type="video/mp4"/></item>
            <item><title>Two</title><link>https://x.com/2</link></item></channel></rss>"#;
        assert_eq!(feed(rss).unwrap(), vec![("a1".into(), "https://x.com/1.mp4".into(), "One".into()), ("https://x.com/2".into(), "https://x.com/2".into(), "Two".into())]);
        let atom = r#"<feed xmlns="http://www.w3.org/2005/Atom" xmlns:yt="http://www.youtube.com/xml/schemas/2015"><entry><id>yt:video:Q</id><yt:videoId>Q</yt:videoId><title>Yt</title><link rel="alternate" href="https://www.youtube.com/watch?v=Q"/></entry></feed>"#;
        assert_eq!(feed(atom).unwrap(), vec![("Q".into(), "https://www.youtube.com/watch?v=Q".into(), "Yt".into())]);
        assert!(feed("<html><body/></html>").is_none());
    }

    #[test]
    fn kick_skips_live_and_bad_ids() {
        let v: Value = serde_json::json!([
            {"is_live": true, "video": {"uuid": "11111111-1111-1111-1111-111111111111"}},
            {"session_title": "Stream", "video": {"uuid": "22222222-2222-2222-2222-222222222222"}},
            {"video": {"uuid": "nope"}}]);
        assert_eq!(kick(&v), vec![("kick:22222222-2222-2222-2222-222222222222".into(), "https://kick.com/video/22222222-2222-2222-2222-222222222222".into(), "Stream".into())]);
    }
}
