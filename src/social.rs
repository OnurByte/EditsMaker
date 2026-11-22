//! Connected accounts and posting: Instagram Reels, TikTok and YouTube Shorts through the person's own signed-in
//! browser window (edits never sees a password; the platform's own page makes the requests), Bluesky with a handle
//! and app password and Facebook Reels with a Page token through their official APIs. The same platforms as Tehom.
//! Unofficial routes can break or get an account restricted; use one you can afford to lose.
use crate::browser::Browser;
use crate::util::{curl_json, home};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const PLATFORMS: &[&str] = &["instagram", "tiktok", "youtube", "bluesky", "facebook"];

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub name: String,
    pub platform: String,
    /// the platform's id for it: user id, channel id, DID, Page id
    pub id: String,
    pub user: String,
    /// Bluesky: the app password; Facebook: the Page token
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub secret: String,
}

fn dir() -> PathBuf {
    home().join(".config/edits/accounts")
}
fn file() -> PathBuf {
    home().join(".config/edits/accounts.json")
}

pub fn all() -> Vec<Account> {
    std::fs::read_to_string(file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save(list: &[Account]) -> Result<(), String> {
    // shortcut: app passwords and Page tokens sit in a file only this user can read, not the system keyring; move
    // them there if edits ever runs on shared machines
    crate::util::write_private(&file(), &serde_json::to_string_pretty(list).unwrap_or_default())
}

/// Accounts by name, or every account with that platform's name; all of them for "all".
pub fn pick(names: &str) -> Result<Vec<Account>, String> {
    let list = all();
    let mut out = vec![];
    for n in names.split(',').map(str::trim).filter(|n| !n.is_empty()) {
        let found: Vec<_> = list.iter().filter(|a| n == "all" || a.name == n || a.platform == n).cloned().collect();
        if found.is_empty() {
            return Err(format!("no connected account \"{n}\" (edits accounts lists them; edits connect PLATFORM adds one)"));
        }
        out.extend(found.into_iter().filter(|a| !out.contains(a)).collect::<Vec<_>>());
    }
    Ok(out)
}

/// (sign-in page, page to post from, cookie that says signed in, its domain, the page script)
fn site(platform: &str) -> Option<(&'static str, &'static str, &'static str, &'static str, &'static str)> {
    Some(match platform {
        "tiktok" => ("https://www.tiktok.com/login", "https://www.tiktok.com/tiktokstudio/upload", "sessionid", "tiktok.com", include_str!("post/tiktok.js")),
        "youtube" => ("https://studio.youtube.com", "https://studio.youtube.com", "SAPISID", "google.com", include_str!("post/youtube.js")),
        "instagram" => ("https://www.instagram.com/accounts/login/", "https://www.instagram.com/", "sessionid", "instagram.com", include_str!("post/instagram.js")),
        _ => return None,
    })
}

fn run_page(b: &mut Browser, js: &str, args: Value, wait: Duration) -> Result<Value, String> {
    b.eval(&format!("({})({args})", js.trim()), wait)
}

fn relogin(platform: &str, e: String) -> String {
    if e == "login" { format!("{platform} sign-in has expired; run edits connect {platform} again") } else { e }
}

/// `edits connect PLATFORM [HANDLE|PAGE_ID]`
pub fn connect(args: &[String]) -> Result<(), String> {
    let platform = args.first().map(String::as_str).unwrap_or("");
    let acc = match platform {
        "bluesky" => {
            let handle = args.get(1).ok_or("usage: edits connect bluesky HANDLE   (then paste an app password from bsky.app/settings/app-passwords)")?;
            let secret = read_secret("Bluesky app password (not your main password): ")?;
            let s = bsky_login(handle, &secret)?;
            Account { name: format!("bluesky-{}", s.handle), platform: platform.into(), id: s.did, user: s.handle, secret }
        }
        "facebook" => {
            let page = args.get(1).ok_or("usage: edits connect facebook PAGE_ID   (then paste a Page access token with pages_manage_posts)")?;
            let secret = read_secret("Facebook Page access token: ")?;
            let v = curl_json(&[("url", &format!("{GRAPH}/{}", page.trim())), ("data-urlencode", "fields=id,name"), ("data-urlencode", &format!("access_token={}", secret.trim())), ("get", "")])?;
            if v["id"].as_str() != Some(page.trim()) {
                return Err("the token does not belong to that Page".into());
            }
            let user = v["name"].as_str().unwrap_or(page).to_string();
            Account { name: format!("facebook-{}", crate::fetch::safe(&user)), platform: platform.into(), id: page.trim().into(), user, secret: secret.trim().into() }
        }
        p => {
            let (login, home_url, cookie, domain, js) = site(p).ok_or(format!("usage: edits connect {}", PLATFORMS.join("|")))?;
            let tmp = dir().join(format!("{p}-new"));
            let _ = std::fs::remove_dir_all(&tmp);
            eprintln!("edits: sign in to {p} in the window that opened (up to 10 minutes); edits never sees your password");
            let mut b = Browser::open(&tmp, login)?;
            let who = (|| {
                for _ in 0..600 {
                    if b.has_cookie(cookie, domain)? {
                        b.goto(home_url)?;
                        let mut last = String::new();
                        for _ in 0..5 {
                            match run_page(&mut b, js, json!({"do": "who"}), Duration::from_secs(60)) {
                                Ok(v) => return Ok(v),
                                Err(e) => last = e,
                            }
                            std::thread::sleep(Duration::from_secs(3));
                        }
                        return Err(relogin(p, last));
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
                Err(format!("{p} sign-in timed out; try again"))
            })();
            b.close();
            let who = who.map_err(|e| if e.contains("window was closed") { format!("the {p} window was closed before sign-in finished") } else { e })?;
            let user = who["name"].as_str().unwrap_or_default().to_string();
            let name = format!("{p}-{}", crate::fetch::safe(&user));
            let to = dir().join(&name);
            let _ = std::fs::remove_dir_all(&to);
            std::fs::rename(&tmp, &to).map_err(|e| e.to_string())?;
            Account { name, platform: p.into(), id: who["id"].as_str().unwrap_or_default().into(), user, secret: String::new() }
        }
    };
    let mut list = all();
    list.retain(|a| a.name != acc.name);
    println!("connected {} ({}) as {}", acc.platform, acc.user, acc.name);
    list.push(acc);
    save(&list)
}

/// `edits accounts`
pub fn list() {
    let l = all();
    if l.is_empty() {
        println!("no accounts yet: edits connect {}", PLATFORMS.join("|"));
    }
    l.iter().for_each(|a| println!("{:<28} {:<10} {}", a.name, a.platform, a.user));
}

/// `edits disconnect NAME`: forgets the account and deletes its browser profile (the sign-in).
pub fn disconnect(name: &str) -> Result<(), String> {
    let mut l = all();
    let n = l.len();
    l.retain(|a| a.name != name);
    if l.len() == n {
        return Err(format!("no account \"{name}\""));
    }
    let _ = std::fs::remove_dir_all(dir().join(name));
    save(&l)?;
    println!("disconnected {name}");
    Ok(())
}

/// A clip's caption: what edits wrote for it (its post line, title or source), else nothing.
pub fn caption_of(clip: &Path) -> String {
    let v: Value = std::fs::read_to_string(clip.with_extension("json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null);
    ["post", "title", "source"].iter().find_map(|k| v[k].as_str().filter(|s| !s.trim().is_empty()).map(str::to_string)).unwrap_or_default()
}

/// Posts a clip to one account; its web address back.
pub fn post(acc: &Account, clip: &Path, caption: &str, private: bool) -> Result<String, String> {
    if std::fs::metadata(clip).map_err(|e| format!("{}: {e}", clip.display()))?.len() == 0 {
        return Err("the clip is empty".into());
    }
    match acc.platform.as_str() {
        "bluesky" => bsky_post(acc, clip, caption),
        "facebook" => fb_post(acc, clip, caption),
        p => {
            let (_, home_url, _, _, js) = site(p).ok_or(format!("unknown platform {p}"))?;
            let mut b = Browser::open(&dir().join(&acc.name), home_url)?;
            let r = (|| {
                b.settle()?;
                b.give_file(clip)?;
                run_page(&mut b, js, json!({"do": "post", "caption": caption, "private": private, "expect": acc.id}), Duration::from_secs(1800))
            })();
            b.close();
            let v = r.map_err(|e| relogin(p, e))?;
            Ok(v.as_str().unwrap_or("posted").to_string())
        }
    }
}

/// `edits post <clip.mp4>... [--to NAME,...] [--caption TEXT] [--private]`
pub fn cmd(args: &[String]) -> Result<(), String> {
    let opt = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let skip: Vec<usize> = ["--to", "--caption"].iter().filter_map(|k| args.iter().position(|a| a == k)).flat_map(|i| [i, i + 1]).collect();
    let clips: Vec<&String> = args.iter().enumerate().filter(|(i, a)| !skip.contains(i) && !a.starts_with("--")).map(|(_, a)| a).collect();
    if clips.is_empty() {
        return Err("usage: edits post <clip.mp4>... [--to ACCOUNT,...|all] [--caption TEXT] [--private]".into());
    }
    let to = match opt("--to") {
        Some(t) => pick(&t)?,
        None => match all().as_slice() {
            [one] => vec![one.clone()],
            [] => return Err(format!("no account connected: edits connect {}", PLATFORMS.join("|"))),
            _ => return Err("which account? --to NAME (or a platform, or all); edits accounts lists them".into()),
        },
    };
    let mut failed = 0;
    for c in clips {
        let clip = Path::new(c);
        let caption = opt("--caption").unwrap_or_else(|| caption_of(clip));
        for a in &to {
            match post(a, clip, &caption, args.iter().any(|x| x == "--private")) {
                Ok(url) => println!("{} -> {}: {url}", clip.display(), a.name),
                Err(e) => {
                    failed += 1;
                    eprintln!("edits: {} -> {}: {e}", clip.display(), a.name);
                }
            }
        }
    }
    if failed > 0 { Err(format!("{failed} post(s) failed")) } else { Ok(()) }
}

/// A secret typed or pasted at the terminal without showing it, or piped in.
pub(crate) fn read_secret(prompt: &str) -> Result<String, String> {
    use std::io::IsTerminal;
    let tty = std::io::stdin().is_terminal();
    if tty {
        eprint!("{prompt}");
        #[cfg(unix)]
        let _ = std::process::Command::new("stty").arg("-echo").stdin(std::process::Stdio::inherit()).status();
    }
    let mut s = String::new();
    let r = std::io::stdin().read_line(&mut s);
    if tty {
        #[cfg(unix)]
        let _ = std::process::Command::new("stty").arg("echo").stdin(std::process::Stdio::inherit()).status();
        eprintln!();
    }
    r.map_err(|e| e.to_string())?;
    let s = s.trim().to_string();
    if s.is_empty() { Err("nothing was entered".into()) } else { Ok(s) }
}

// ---- Bluesky: the AT Protocol's documented XRPC API ----

struct Bsky {
    jwt: String,
    did: String,
    handle: String,
    pds: String,
}

fn bsky_login(handle: &str, password: &str) -> Result<Bsky, String> {
    let body = json!({"identifier": handle.trim().trim_start_matches('@'), "password": password}).to_string();
    let v = curl_json(&[("url", "https://bsky.social/xrpc/com.atproto.server.createSession"), ("header", "content-type: application/json"), ("data-binary", &body)])
        .map_err(|e| if e.starts_with("HTTP 401") || e.starts_with("HTTP 400") { "Bluesky refused the handle or app password".to_string() } else { e })?;
    let s = |k: &Value| k.as_str().unwrap_or_default().to_string();
    let pds = v["didDoc"]["service"].as_array().into_iter().flatten().find(|x| x["id"] == "#atproto_pds").map_or("https://bsky.social".into(), |x| s(&x["serviceEndpoint"]));
    let b = Bsky { jwt: s(&v["accessJwt"]), did: s(&v["did"]), handle: s(&v["handle"]), pds: pds.trim_end_matches('/').into() };
    if b.jwt.is_empty() || b.did.is_empty() {
        return Err("Bluesky's sign-in answer had no session".into());
    }
    Ok(b)
}

fn bsky_post(acc: &Account, clip: &Path, text: &str) -> Result<String, String> {
    let text: String = text.trim().chars().take(300).collect();
    let s = bsky_login(&acc.user, &acc.secret)?;
    if s.did != acc.id {
        return Err("this app password belongs to a different account; run edits connect bluesky again".into());
    }
    let bearer = |t: &str| format!("Authorization: Bearer {t}");
    let host = s.pds.split("://").nth(1).unwrap_or(&s.pds).to_string();
    let exp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) + 1800;
    let auth = curl_json(&[("url", &format!("{}/xrpc/com.atproto.server.getServiceAuth?aud=did:web:{host}&lxm=com.atproto.repo.uploadBlob&exp={exp}", s.pds)), ("header", &bearer(&s.jwt))])?;
    let job = curl_json(&[("url", &format!("https://video.bsky.app/xrpc/app.bsky.video.uploadVideo?did={}&name=clip.mp4", s.did)),
        ("header", &bearer(auth["token"].as_str().unwrap_or_default())), ("header", "content-type: video/mp4"), ("data-binary", &format!("@{}", clip.display()))])
        .map_err(|e| if e.starts_with("HTTP 409") { "this video was already uploaded to Bluesky".to_string() } else { e })?;
    let mut blob = job["blob"].clone();
    let id = job["jobId"].as_str().unwrap_or_default().to_string();
    for _ in 0..90 {
        if !blob.is_null() {
            break;
        }
        std::thread::sleep(Duration::from_secs(2));
        let st = curl_json(&[("url", &format!("https://video.bsky.app/xrpc/app.bsky.video.getJobStatus?jobId={id}"))])?;
        let st = &st["jobStatus"];
        if st["state"] == "JOB_STATE_FAILED" {
            return Err(st["error"].as_str().or(st["message"].as_str()).unwrap_or("Bluesky could not process the video").to_string());
        }
        blob = st["blob"].clone();
    }
    if blob.is_null() {
        return Err("Bluesky took too long to process the video".into());
    }
    let (w, h) = crate::fetch::dims(clip);
    let mut embed = json!({"$type": "app.bsky.embed.video", "video": blob});
    if w > 0 && h > 0 {
        embed["aspectRatio"] = json!({"width": w, "height": h});
    }
    let rec = json!({"repo": s.did, "collection": "app.bsky.feed.post", "record": {"$type": "app.bsky.feed.post", "text": text, "createdAt": rfc3339_now(), "embed": embed}}).to_string();
    let v = curl_json(&[("url", &format!("{}/xrpc/com.atproto.repo.createRecord", s.pds)), ("header", &bearer(&s.jwt)), ("header", "content-type: application/json"), ("data-binary", &rec)])?;
    Ok(format!("https://bsky.app/profile/{}/post/{}", s.handle, v["uri"].as_str().and_then(|u| u.rsplit('/').next()).unwrap_or_default()))
}

/// Now as RFC 3339 UTC (civil-from-days, Howard Hinnant).
fn rfc3339_now() -> String {
    rfc3339(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64)
}
fn rfc3339(secs: i64) -> String {
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let (era, doe) = (z.div_euclid(146_097), z.rem_euclid(146_097));
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let (d, m) = (doy - (153 * mp + 2) / 5 + 1, if mp < 10 { mp + 3 } else { mp - 9 });
    format!("{:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", yoe + era * 400 + i64::from(m <= 2), rest / 3600, rest % 3600 / 60, rest % 60)
}

// ---- Facebook Reels: the documented Graph API ----

const GRAPH: &str = "https://graph.facebook.com/v21.0";

fn fb_post(acc: &Account, clip: &Path, description: &str) -> Result<String, String> {
    let tok = format!("access_token={}", acc.secret);
    let url = format!("{GRAPH}/{}/video_reels", acc.id);
    let fb = |e: String| if e.contains("190") || e.starts_with("HTTP 401") { "Facebook refused the Page token; run edits connect facebook again".to_string() } else { e };
    let start = curl_json(&[("url", &url), ("data-urlencode", "upload_phase=start"), ("data-urlencode", &tok)]).map_err(fb)?;
    let (id, up) = (start["video_id"].as_str().unwrap_or_default(), start["upload_url"].as_str().unwrap_or_default());
    if id.is_empty() || up.is_empty() {
        return Err("Facebook did not start the upload".into());
    }
    let size = std::fs::metadata(clip).map(|m| m.len()).unwrap_or(0).to_string();
    curl_json(&[("url", up), ("header", &format!("Authorization: OAuth {}", acc.secret)), ("header", "offset: 0"), ("header", &format!("file_size: {size}")), ("data-binary", &format!("@{}", clip.display()))]).map_err(fb)?;
    let done = curl_json(&[("url", &url), ("data-urlencode", "upload_phase=finish"), ("data-urlencode", &format!("video_id={id}")), ("data-urlencode", "video_state=PUBLISHED"),
        ("data-urlencode", &format!("description={}", description.trim().chars().take(2200).collect::<String>())), ("data-urlencode", &tok)]).map_err(fb)?;
    if done["success"] != true {
        return Err("Facebook did not publish the Reel".into());
    }
    Ok(format!("https://www.facebook.com/reel/{id}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_and_sites() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_709_210_096), "2024-02-29T12:34:56Z");
        assert!(PLATFORMS.iter().all(|p| site(p).is_some() || ["bluesky", "facebook"].contains(p)));
        assert!(site("instagram").unwrap().4.trim_start().starts_with("//"), "page scripts are whole files");
    }

    #[test]
    fn page_scripts_are_one_async_function() {
        for p in ["tiktok", "youtube", "instagram"] {
            let js = site(p).unwrap().4;
            let code: String = js.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
            assert!(code.trim_start().starts_with("async (p) => {") && code.trim_end().ends_with('}'), "{p}");
        }
    }
}
