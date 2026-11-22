//! `edits review clip.mp4`: the clip on a page in the browser, served from this machine. The user points at a moment,
//! a shot or a spot in the frame and says what to change; the notes come back here, are saved beside the clip as
//! NAME.notes.json and printed for the agent, which changes exactly that.
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const PAGE: &str = include_str!("review.html");
/// How long a review page waits for its notes.
const WAIT: Duration = Duration::from_secs(20 * 60);

fn notes_path(clip: &Path) -> PathBuf {
    clip.with_extension("notes.json")
}

/// The notes in words an agent acts on: when, which shot, where in the frame, what.
pub fn describe(notes: &Value, cuts: &[f64]) -> String {
    let mut starts = vec![0.0];
    starts.extend(cuts.iter().copied().filter(|c| *c > 0.05));
    let mut out = String::new();
    for n in notes["notes"].as_array().into_iter().flatten() {
        let t = n["t"].as_f64().unwrap_or(0.0);
        let k = starts.iter().rposition(|s| *s <= t + 0.01).unwrap_or(0);
        let end = starts.get(k + 1).map_or(String::from("end"), |e| format!("{e:.1}s"));
        let at = match (n["x"].as_f64(), n["y"].as_f64()) {
            (Some(x), Some(y)) => format!(", at {:.0}% across {:.0}% down", x * 100.0, y * 100.0),
            _ => String::new(),
        };
        out.push_str(&format!("- {t:.1}s, shot {} ({:.1}s-{end}){at}: {}\n", k + 1, starts[k], n["text"].as_str().unwrap_or("").trim()));
    }
    if let Some(g) = notes["general"].as_str().filter(|g| !g.trim().is_empty()) {
        out.push_str(&format!("- the whole edit: {}\n", g.trim()));
    }
    out
}

/// Opens a link in the default browser; false when there is none to open it.
fn open(url: &str) -> bool {
    let r = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(url).status()
    } else if cfg!(windows) {
        std::process::Command::new("cmd").args(["/C", "start", "", url]).status()
    } else {
        std::process::Command::new("xdg-open").arg(url).status()
    };
    r.is_ok_and(|s| s.success())
}

/// `edits review <clip.mp4> [--notes]`
pub fn cmd(args: &[String]) -> Result<(), String> {
    let clip = std::path::absolute(args.first().ok_or("usage: edits review <clip.mp4> [--notes]")?).map_err(|e| e.to_string())?;
    if !clip.is_file() {
        return Err(format!("no clip at {}", clip.display()));
    }
    let cuts = crate::check::cuts(&clip);
    if args.iter().any(|a| a == "--notes") {
        let saved = std::fs::read_to_string(notes_path(&clip)).map_err(|_| format!("no notes saved for {} yet", clip.display()))?;
        let v: Value = serde_json::from_str(&saved).map_err(|e| e.to_string())?;
        print!("notes on {}:\n{}", clip.display(), describe(&v, &cuts));
        return Ok(());
    }
    let total = crate::fetch::duration(&clip)?;
    let side: Value = std::fs::read_to_string(clip.with_extension("json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let about: Vec<String> = [("preset", "preset"), ("song", "song"), ("look", "look")].iter()
        .filter_map(|(k, l)| side[*k].as_str().filter(|s| !s.is_empty()).map(|s| format!("{l} {s}"))).collect();
    let cfg = json!({ "name": clip.file_name().unwrap_or_default().to_string_lossy(), "total": total, "cuts": cuts, "about": about });
    // the config goes into a script: "</" must not end it early
    let page = PAGE.replace("/*CONFIG*/", &format!("const CFG = {};", cfg.to_string().replace("</", "<\\/")));
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let url = format!("http://127.0.0.1:{}/", listener.local_addr().map_err(|e| e.to_string())?.port());
    listener.set_nonblocking(true).map_err(|e| e.to_string())?;
    if !open(&url) {
        eprintln!("edits: open {url} in a browser");
    }
    eprintln!("edits: review page at {url}; waiting for notes (up to {} minutes)", WAIT.as_secs() / 60);
    let got: Arc<Mutex<Option<Value>>> = Arc::new(Mutex::new(None));
    let (page, clip_ref) = (Arc::new(page), Arc::new(clip.clone()));
    let start = Instant::now();
    while got.lock().map(|g| g.is_none()).unwrap_or(false) {
        if start.elapsed() > WAIT {
            return Err(format!("no notes came from the review page in {} minutes", WAIT.as_secs() / 60));
        }
        match listener.accept() {
            Ok((stream, _)) => {
                let (page, clip, got) = (page.clone(), clip_ref.clone(), got.clone());
                std::thread::spawn(move || {
                    let _ = serve(stream, &page, &clip, &got);
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(60)),
            Err(e) => return Err(e.to_string()),
        }
    }
    let v = got.lock().map_err(|e| e.to_string())?.clone().unwrap_or(Value::Null);
    print!("notes on {}:\n{}saved: {}\n", clip.display(), describe(&v, &cuts), notes_path(&clip).display());
    Ok(())
}

/// One request: the page, the video (with byte ranges, so it seeks) or the notes.
fn serve(stream: TcpStream, page: &str, clip: &Path, got: &Mutex<Option<Value>>) -> std::io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    let mut r = BufReader::new(stream.try_clone()?);
    let mut first = String::new();
    r.read_line(&mut first)?;
    let (mut range, mut len) = (None, 0usize);
    loop {
        let mut h = String::new();
        if r.read_line(&mut h)? == 0 || h.trim().is_empty() {
            break;
        }
        let (k, v) = h.split_once(':').unwrap_or(("", ""));
        match k.trim().to_ascii_lowercase().as_str() {
            "range" => range = parse_range(v.trim()),
            "content-length" => len = v.trim().parse().unwrap_or(0).min(1 << 20),
            _ => {}
        }
    }
    let mut w = stream;
    let mut parts = first.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    match (method, path) {
        ("GET", "/") => reply(&mut w, "200 OK", "text/html; charset=utf-8", page.as_bytes()),
        ("GET", "/video") => video(&mut w, clip, range),
        ("POST", "/notes") => {
            let mut body = vec![0; len];
            r.read_exact(&mut body)?;
            match serde_json::from_slice::<Value>(&body) {
                Ok(v) if v["notes"].is_array() => {
                    let saved = json!({ "clip": clip, "notes": v["notes"], "general": v["general"] });
                    std::fs::write(notes_path(clip), serde_json::to_string_pretty(&saved).unwrap_or_default())?;
                    reply(&mut w, "200 OK", "application/json", b"{\"ok\":true}")?;
                    if let Ok(mut g) = got.lock() {
                        *g = Some(saved);
                    }
                    Ok(())
                }
                _ => reply(&mut w, "400 Bad Request", "text/plain", b"notes must be JSON with a notes list"),
            }
        }
        _ => reply(&mut w, "404 Not Found", "text/plain", b"not here"),
    }
}

fn reply(w: &mut TcpStream, status: &str, kind: &str, body: &[u8]) -> std::io::Result<()> {
    write!(w, "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n", body.len())?;
    w.write_all(body)
}

/// "bytes=A-B", "bytes=A-": (A, B or None).
pub fn parse_range(v: &str) -> Option<(u64, Option<u64>)> {
    let (a, b) = v.strip_prefix("bytes=")?.split(',').next()?.split_once('-')?;
    Some((a.trim().parse().ok()?, b.trim().parse().ok()))
}

fn video(w: &mut TcpStream, clip: &Path, range: Option<(u64, Option<u64>)>) -> std::io::Result<()> {
    let mut f = std::fs::File::open(clip)?;
    let size = f.metadata()?.len();
    let (a, b) = match range {
        Some((a, b)) if a < size => (a, b.unwrap_or(size - 1).min(size - 1)),
        Some(_) => return write!(w, "HTTP/1.1 416 Range Not Satisfiable\r\nContent-Range: bytes */{size}\r\nConnection: close\r\n\r\n"),
        None => (0, size.saturating_sub(1)),
    };
    let n = b + 1 - a;
    let status = if range.is_some() { format!("206 Partial Content\r\nContent-Range: bytes {a}-{b}/{size}") } else { "200 OK".into() };
    write!(w, "HTTP/1.1 {status}\r\nContent-Type: video/mp4\r\nAccept-Ranges: bytes\r\nContent-Length: {n}\r\nConnection: close\r\n\r\n")?;
    f.seek(SeekFrom::Start(a))?;
    std::io::copy(&mut f.take(n), w).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_in_words() {
        let n = json!({ "notes": [{ "t": 3.2, "x": 0.62, "y": 0.3, "text": " too dark " }, { "t": 0.4, "x": null, "y": null, "text": "start later" }], "general": "song too quiet" });
        let d = describe(&n, &[1.5, 2.9, 4.1]);
        assert_eq!(d, "- 3.2s, shot 3 (2.9s-4.1s), at 62% across 30% down: too dark\n- 0.4s, shot 1 (0.0s-1.5s): start later\n- the whole edit: song too quiet\n");
        assert_eq!(describe(&json!({ "notes": [{ "t": 9.0, "text": "x" }] }), &[1.5]), "- 9.0s, shot 2 (1.5s-end): x\n");
    }

    #[test]
    fn byte_ranges() {
        assert_eq!(parse_range("bytes=0-"), Some((0, None)));
        assert_eq!(parse_range("bytes=100-199"), Some((100, Some(199))));
        assert_eq!(parse_range("items=1-2"), None);
    }

    #[test]
    fn the_page_takes_its_config() {
        assert!(PAGE.contains("/*CONFIG*/") && PAGE.contains("fetch('/notes'") && PAGE.contains("src=\"/video\""));
    }
}
