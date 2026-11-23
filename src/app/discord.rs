//! Discord Rich Presence: "Playing EditsMaker", what is being made under it, our icon, and a Download button to the
//! README's Install. On by default (Settings, General turns it off); never asked about in the welcome. It speaks
//! Discord's local IPC (a socket, or a named pipe on Windows) itself: a frame is op and length (u32, little-endian)
//! then JSON. Without Discord running it waits and tries again.
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The Discord application "EditsMaker" (discord.com/developers: its name is what "Playing" shows); EDITS_DISCORD_APP
/// overrides it. Empty: no presence until it is set.
const APP_ID: &str = "1574266727988985886";
const ICON: &str = "https://raw.githubusercontent.com/OnurByte/EditsMaker/main/assets/icon.png";
const DOWNLOAD: &str = "https://github.com/OnurByte/EditsMaker#install";

/// What the app is doing now, as the presence's second line.
static DOING: Mutex<String> = Mutex::new(String::new());

pub fn doing(what: String) {
    *DOING.lock().unwrap() = what.chars().take(120).collect();
}

/// The SET_ACTIVITY frame's body.
pub fn activity(doing: &str, since: u64, nonce: u64) -> Value {
    let mut a = json!({
        "details": if doing.is_empty() { "Making edits" } else { doing },
        "state": "The AI video harness.",
        "timestamps": {"start": since},
        "assets": {"large_image": ICON, "large_text": "EditsMaker"},
        "buttons": [{"label": "Download EditsMaker", "url": DOWNLOAD}],
    });
    // Discord refuses a field under 2 characters
    if doing.chars().count() == 1 {
        a["details"] = json!("Making edits");
    }
    json!({"cmd": "SET_ACTIVITY", "args": {"pid": std::process::id(), "activity": a}, "nonce": nonce.to_string()})
}

pub fn frame(op: u32, body: &Value) -> Vec<u8> {
    let b = body.to_string().into_bytes();
    let mut v = Vec::with_capacity(8 + b.len());
    v.extend(op.to_le_bytes());
    v.extend((b.len() as u32).to_le_bytes());
    v.extend(b);
    v
}

trait Pipe: Read + Write + Send {}
impl<T: Read + Write + Send> Pipe for T {}

fn connect() -> Option<Box<dyn Pipe>> {
    for i in 0..10 {
        #[cfg(unix)]
        {
            let dirs = ["XDG_RUNTIME_DIR", "TMPDIR", "TMP", "TEMP"].iter().filter_map(|k| std::env::var(k).ok()).chain(["/tmp".to_string()]);
            for d in dirs {
                // Flatpak and Snap Discord put theirs one folder down
                for sub in ["", "app/com.discordapp.Discord/", "snap.discord/"] {
                    if let Ok(s) = std::os::unix::net::UnixStream::connect(format!("{d}/{sub}discord-ipc-{i}")) {
                        return Some(Box::new(s));
                    }
                }
            }
        }
        #[cfg(windows)]
        if let Ok(f) = std::fs::OpenOptions::new().read(true).write(true).open(format!(r"\\.\pipe\discord-ipc-{i}")) {
            return Some(Box::new(f));
        }
    }
    None
}

fn read_frame(p: &mut dyn Pipe) -> std::io::Result<Value> {
    let mut head = [0u8; 8];
    p.read_exact(&mut head)?;
    let n = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
    let mut body = vec![0u8; n.min(1 << 20)];
    p.read_exact(&mut body)?;
    Ok(serde_json::from_slice(&body).unwrap_or(Value::Null))
}

/// Keeps the presence up to date while the app runs, if it is on.
pub fn start() {
    static STARTED: OnceLock<()> = OnceLock::new();
    if STARTED.set(()).is_err() {
        return;
    }
    let since = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let app = std::env::var("EDITS_DISCORD_APP").unwrap_or_else(|_| APP_ID.into());
    if app.trim().is_empty() {
        return;
    }
    std::thread::spawn(move || loop {
        if !crate::app::theme::prefs().discord {
            std::thread::sleep(Duration::from_secs(5));
            continue;
        }
        let Some(mut p) = connect() else {
            std::thread::sleep(Duration::from_secs(20));
            continue;
        };
        if p.write_all(&frame(0, &json!({"v": 1, "client_id": app}))).is_err() || read_frame(p.as_mut()).is_err() {
            std::thread::sleep(Duration::from_secs(20));
            continue;
        }
        let (mut shown, mut nonce) = (None::<String>, 0u64);
        loop {
            let on = crate::app::theme::prefs().discord;
            let now = DOING.lock().unwrap().clone();
            if !on {
                // turned off: clear it and let go of Discord
                let _ = p.write_all(&frame(1, &json!({"cmd": "SET_ACTIVITY", "args": {"pid": std::process::id()}, "nonce": "off"})));
                break;
            }
            if shown.as_deref() != Some(now.as_str()) {
                nonce += 1;
                if p.write_all(&frame(1, &activity(&now, since, nonce))).is_err() || read_frame(p.as_mut()).is_err() {
                    break;
                }
                shown = Some(now);
            }
            std::thread::sleep(Duration::from_secs(2));
        }
        std::thread::sleep(Duration::from_secs(5));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_is_op_length_then_json() {
        let f = frame(1, &json!({"a": 1}));
        assert_eq!(&f[..4], &1u32.to_le_bytes());
        assert_eq!(u32::from_le_bytes([f[4], f[5], f[6], f[7]]) as usize, f.len() - 8);
        let a = activity("Editing Madara", 7, 3);
        assert_eq!(a["args"]["activity"]["buttons"][0]["url"], DOWNLOAD);
        assert_eq!(a["args"]["activity"]["details"], "Editing Madara");
        assert_eq!(activity("x", 7, 1)["args"]["activity"]["details"], "Making edits");
    }
}
