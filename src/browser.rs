//! A Chromium-family browser with a profile of its own, driven over the DevTools protocol: the person signs in to a
//! platform in that window, and the platform's own logged-in page makes the upload requests. A minimal WebSocket
//! client over a local TCP socket, so no library is needed.
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// Chromium-family browsers on PATH, by their command names.
const APP_BROWSERS: &[&str] = &["chromium", "chromium-browser", "google-chrome-stable", "google-chrome", "brave-browser", "brave", "helium-browser", "microsoft-edge-stable", "microsoft-edge", "vivaldi-stable"];

/// The first Chromium-family browser installed here.
pub fn find() -> Option<PathBuf> {
    let mut c: Vec<PathBuf> = ["Google Chrome", "Chromium", "Microsoft Edge", "Brave Browser"].iter()
        .map(|a| PathBuf::from(format!("/Applications/{a}.app/Contents/MacOS/{a}"))).collect();
    for root in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"].iter().filter_map(std::env::var_os) {
        for t in [r"Google\Chrome\Application\chrome.exe", r"Microsoft\Edge\Application\msedge.exe", r"BraveSoftware\Brave-Browser\Application\brave.exe"] {
            c.push(Path::new(&root).join(t));
        }
    }
    if let Some(p) = std::env::var_os("PATH") {
        for d in std::env::split_paths(&p) {
            c.extend(APP_BROWSERS.iter().map(|b| d.join(b)));
        }
    }
    c.into_iter().find(|p| p.is_file())
}

pub struct Browser {
    child: Child,
    port: u16,
    page: Ws,
}

impl Browser {
    /// Opens a window on `url` with `profile` as its own data folder, connected to its first tab.
    pub fn open(profile: &Path, url: &str) -> Result<Browser, String> {
        let exe = find().ok_or("no Chrome, Chromium, Edge or Brave found; install one to connect this account")?;
        std::fs::create_dir_all(profile).map_err(|e| e.to_string())?;
        let marker = profile.join("DevToolsActivePort");
        let _ = std::fs::remove_file(&marker);
        let child = Command::new(exe)
            .arg(format!("--user-data-dir={}", profile.display()))
            // the debugging port makes navigator.webdriver true, which Google's sign-in refuses as "not secure"
            .args(["--remote-debugging-port=0", "--no-first-run", "--no-default-browser-check", "--disable-blink-features=AutomationControlled", url])
            .stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|e| format!("the browser did not start: {e}"))?;
        let mut b = Browser { child, port: 0, page: Ws(None) };
        for _ in 0..200 {
            if let Some(p) = std::fs::read_to_string(&marker).ok().and_then(|t| t.lines().next()?.trim().parse().ok()) {
                b.port = p;
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        if b.port == 0 {
            return Err("the browser did not start; close any window already using this account's profile and try again".into());
        }
        for _ in 0..100 {
            let tabs: Value = serde_json::from_str(&http_get(b.port, "/json/list").unwrap_or_default()).unwrap_or(Value::Null);
            if let Some(ws) = tabs.as_array().into_iter().flatten().find(|t| t["type"] == "page").and_then(|t| t["webSocketDebuggerUrl"].as_str()) {
                b.page = Ws::connect(b.port, &ws[ws.find("/devtools").unwrap_or(0)..])?;
                return Ok(b);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        Err("the browser opened no tab".into())
    }

    /// A DevTools call on the tab; events that arrive meanwhile are skipped.
    pub fn call(&mut self, method: &str, params: Value, wait: Duration) -> Result<Value, String> {
        self.page.call(method, params, wait)
    }

    /// A JavaScript expression's value (a promise is awaited); a thrown string comes back as the error.
    pub fn eval(&mut self, js: &str, wait: Duration) -> Result<Value, String> {
        let r = self.call("Runtime.evaluate", json!({"expression": js, "awaitPromise": true, "returnByValue": true}), wait)?;
        if let Some(e) = r.get("exceptionDetails") {
            let ex = &e["exception"];
            return Err(ex["value"].as_str().or(ex["description"].as_str()).unwrap_or("script error").to_string());
        }
        Ok(r["result"]["value"].clone())
    }

    /// Waits for the page to load and its scripts to start.
    pub fn settle(&mut self) -> Result<(), String> {
        for _ in 0..120 {
            if self.eval("document.readyState", Duration::from_secs(10)).is_ok_and(|s| s == "complete") {
                // shortcut: a fixed pause for the site's own scripts; upgrade if its requests go out unready
                std::thread::sleep(Duration::from_secs(4));
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        Err("the page did not finish loading".into())
    }

    pub fn goto(&mut self, url: &str) -> Result<(), String> {
        self.call("Page.navigate", json!({"url": url}), Duration::from_secs(30))?;
        std::thread::sleep(Duration::from_secs(1));
        self.settle()
    }

    /// Whether a cookie by that name is set for a domain containing `domain`.
    pub fn has_cookie(&mut self, name: &str, domain: &str) -> Result<bool, String> {
        let c = self.call("Network.getAllCookies", json!({}), Duration::from_secs(30))?;
        Ok(c["cookies"].as_array().into_iter().flatten()
            .any(|k| k["name"] == name && k["domain"].as_str().is_some_and(|d| d.contains(domain)) && k["value"].as_str().is_some_and(|v| !v.is_empty())))
    }

    /// Puts a local file into the page as `document.getElementById('__edits_file').files[0]`, so the page's own
    /// scripts can upload it.
    pub fn give_file(&mut self, file: &Path) -> Result<(), String> {
        let w = Duration::from_secs(30);
        self.eval("(() => { let i = document.getElementById('__edits_file'); if (!i) { i = document.createElement('input'); i.type = 'file'; i.id = '__edits_file'; i.style.display = 'none'; document.documentElement.appendChild(i); } return 1; })()", w)?;
        let root = self.call("DOM.getDocument", json!({"depth": 0}), w)?["root"]["nodeId"].clone();
        let node = self.call("DOM.querySelector", json!({"nodeId": root, "selector": "#__edits_file"}), w)?["nodeId"].clone();
        let abs = std::fs::canonicalize(file).map_err(|e| format!("{}: {e}", file.display()))?;
        self.call("DOM.setFileInputFiles", json!({"files": [abs.to_string_lossy()], "nodeId": node}), w).map(|_| ())
    }

    /// Closes the window so the profile (the sign-in) is written to disk.
    pub fn close(mut self) {
        if let Some(path) = http_get(self.port, "/json/version").ok().and_then(|t| serde_json::from_str::<Value>(&t).ok())
            .and_then(|v| v["webSocketDebuggerUrl"].as_str().map(|u| u[u.find("/devtools").unwrap_or(0)..].to_string())) {
            if let Ok(mut ws) = Ws::connect(self.port, &path) {
                let _ = ws.send(&json!({"id": 1, "method": "Browser.close"}).to_string());
            }
        }
        let end = Instant::now() + Duration::from_secs(10);
        while Instant::now() < end {
            if self.child.try_wait().is_ok_and(|s| s.is_some()) {
                return;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        let _ = self.child.kill();
    }
}

fn http_get(port: u16, path: &str) -> Result<String, String> {
    let mut s = TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())?;
    s.set_read_timeout(Some(Duration::from_secs(5))).ok();
    s.write_all(format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n").as_bytes()).map_err(|e| e.to_string())?;
    let mut out = String::new();
    s.read_to_string(&mut out).map_err(|e| e.to_string())?;
    Ok(out.split_once("\r\n\r\n").map_or("", |(_, b)| b).to_string())
}

/// A WebSocket to the browser: text frames out (masked, as a client must), text frames in.
struct Ws(Option<(TcpStream, u64)>);

impl Ws {
    fn connect(port: u16, path: &str) -> Result<Ws, String> {
        let mut s = TcpStream::connect(("127.0.0.1", port)).map_err(|e| e.to_string())?;
        s.write_all(format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: ZWRpdHNtYWtlcmVkaXRzMQ==\r\nSec-WebSocket-Version: 13\r\n\r\n").as_bytes()).map_err(|e| e.to_string())?;
        let mut head = vec![];
        let mut b = [0u8; 1];
        while !head.ends_with(b"\r\n\r\n") {
            s.read_exact(&mut b).map_err(|e| e.to_string())?;
            head.push(b[0]);
        }
        if !head.starts_with(b"HTTP/1.1 101") {
            return Err(format!("the browser refused the connection: {}", String::from_utf8_lossy(&head).lines().next().unwrap_or("")));
        }
        Ok(Ws(Some((s, 0))))
    }

    fn send(&mut self, text: &str) -> Result<(), String> {
        let (s, _) = self.0.as_mut().ok_or("not connected")?;
        s.write_all(&frame(text.as_bytes())).map_err(|e| e.to_string())
    }

    /// The next whole text message.
    fn recv(&mut self, until: Instant) -> Result<String, String> {
        let (s, _) = self.0.as_mut().ok_or("not connected")?;
        let mut msg = vec![];
        loop {
            s.set_read_timeout(Some(until.saturating_duration_since(Instant::now()).max(Duration::from_millis(1)))).ok();
            let mut h = [0u8; 2];
            s.read_exact(&mut h).map_err(|e| if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) { "timeout".to_string() } else { "the browser window was closed".to_string() })?;
            let mut len = (h[1] & 0x7f) as u64;
            if len >= 126 {
                let mut x = vec![0u8; if len == 126 { 2 } else { 8 }];
                s.read_exact(&mut x).map_err(|e| e.to_string())?;
                len = x.iter().fold(0, |a, &b| a << 8 | b as u64);
            }
            let mut p = vec![0u8; len as usize];
            s.read_exact(&mut p).map_err(|e| e.to_string())?;
            match h[0] & 0x0f {
                0x8 => return Err("the browser window was closed".into()),
                0x0 | 0x1 => msg.extend(p),
                _ => continue, // ping, pong, binary: CDP sends none it needs answered
            }
            if h[0] & 0x80 != 0 {
                return Ok(String::from_utf8_lossy(&msg).into_owned());
            }
        }
    }

    fn call(&mut self, method: &str, params: Value, wait: Duration) -> Result<Value, String> {
        let id = {
            let (_, n) = self.0.as_mut().ok_or("not connected")?;
            *n += 1;
            *n
        };
        self.send(&json!({"id": id, "method": method, "params": params}).to_string())?;
        let until = Instant::now() + wait;
        loop {
            let m = self.recv(until).map_err(|e| if e == "timeout" { format!("the browser did not answer {method} in time") } else { e })?;
            let Ok(v) = serde_json::from_str::<Value>(&m) else { continue };
            if v["id"] != id {
                continue; // an event
            }
            if let Some(e) = v.get("error") {
                return Err(e["message"].as_str().unwrap_or("browser error").to_string());
            }
            return Ok(v["result"].clone());
        }
    }
}

/// One final masked text frame.
fn frame(p: &[u8]) -> Vec<u8> {
    let mut f = vec![0x81u8];
    match p.len() {
        n if n < 126 => f.push(0x80 | n as u8),
        n if n < 65536 => {
            f.push(0x80 | 126);
            f.extend((n as u16).to_be_bytes());
        }
        n => {
            f.push(0x80 | 127);
            f.extend((n as u64).to_be_bytes());
        }
    }
    let key = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0x5a17_c3e1, |d| d.subsec_nanos()).to_be_bytes();
    f.extend(key);
    f.extend(p.iter().enumerate().map(|(i, b)| b ^ key[i % 4]));
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_masked_and_sized() {
        for n in [5usize, 300, 70000] {
            let f = frame(&vec![b'a'; n]);
            assert_eq!(f[0], 0x81);
            assert!(f[1] & 0x80 != 0, "client frames must be masked");
            let (len, at) = match f[1] & 0x7f {
                126 => (u16::from_be_bytes([f[2], f[3]]) as usize, 4),
                127 => (u64::from_be_bytes(f[2..10].try_into().unwrap()) as usize, 10),
                l => (l as usize, 2),
            };
            assert_eq!(len, n);
            let key = &f[at..at + 4];
            assert!(f[at + 4..].iter().enumerate().all(|(i, b)| b ^ key[i % 4] == b'a'));
        }
    }

    /// A stand-in DevTools tab: answers each call with its id after an event, the way a browser does.
    #[test]
    fn calls_skip_events() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let (mut head, mut b) = (vec![], [0u8; 1]);
            while !head.ends_with(b"\r\n\r\n") {
                s.read_exact(&mut b).unwrap();
                head.push(b[0]);
            }
            s.write_all(b"HTTP/1.1 101 Switching Protocols\r\n\r\n").unwrap();
            let mut h = [0u8; 2];
            s.read_exact(&mut h).unwrap();
            let mut p = vec![0u8; (h[1] & 0x7f) as usize + 4];
            s.read_exact(&mut p).unwrap();
            let text: Vec<u8> = p[4..].iter().enumerate().map(|(i, b)| b ^ p[i % 4]).collect();
            let id = serde_json::from_slice::<Value>(&text).unwrap()["id"].clone();
            for m in [json!({"method": "Page.loadEventFired"}), json!({"id": id, "result": {"ok": 1}})] {
                let t = m.to_string();
                s.write_all(&[0x81, t.len() as u8]).unwrap();
                s.write_all(t.as_bytes()).unwrap();
            }
        });
        let mut ws = Ws::connect(port, "/devtools/page/x").unwrap();
        assert_eq!(ws.call("Runtime.evaluate", json!({}), Duration::from_secs(5)).unwrap()["ok"], 1);
    }
}
