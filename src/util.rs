use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Runs a program and returns its stdout; a non-zero exit becomes an error carrying the end of stderr.
pub fn run<S: AsRef<OsStr>>(dir: Option<&Path>, cmd: &str, args: &[S]) -> Result<String, String> {
    let mut c = Command::new(cmd);
    c.args(args).stdin(Stdio::null());
    if let Some(d) = dir {
        c.current_dir(d);
    }
    let out = c.output().map_err(|e| format!("{cmd}: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail: Vec<&str> = err.lines().rev().take(6).collect();
        return Err(format!("{cmd} failed: {}", tail.into_iter().rev().collect::<Vec<_>>().join(" | ")));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Unpacks a zip into `to`: unzip where it is installed, else tar, which is bsdtar on Windows 10+ and macOS and
/// opens zips too.
pub fn unzip(zip: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    let (z, d) = (zip.to_string_lossy(), to.to_string_lossy());
    run(None, "unzip", &["-q", "-o", &z, "-d", &d]).or_else(|e| run(None, "tar", &["-xf", &z, "-C", &d]).map_err(|t| format!("{e}; {t}"))).map(|_| ())
}

/// The local date and time as ("2026-11-16", "21.04"): `date` on unix, PowerShell's Get-Date on Windows, where
/// `date` is a cmd built-in and not a program.
pub fn now() -> Option<(String, String)> {
    let out = if cfg!(windows) {
        run(None, "powershell", &["-NoProfile", "-Command", "Get-Date -Format 'yyyy-MM-dd HH.mm'"])
    } else {
        run(None, "date", &["+%F %H.%M"])
    };
    let s = out.ok()?;
    let (d, t) = s.trim().split_once(' ')?;
    Some((d.to_string(), t.to_string()))
}

static NVENC: std::sync::OnceLock<bool> = std::sync::OnceLock::new();

/// The video encoder for this run (from --encoder, else EDITS_ENCODER): cpu (default), nvenc, or auto (nvenc when it works).
pub fn set_encoder(name: &str) -> Result<(), String> {
    let on = match name {
        "cpu" => false,
        "nvenc" => true,
        "auto" => nvenc_works(),
        _ => return Err("--encoder must be cpu, nvenc or auto".into()),
    };
    let _ = NVENC.set(on);
    Ok(())
}

pub fn nvenc() -> bool {
    *NVENC.get_or_init(|| match std::env::var("EDITS_ENCODER").as_deref() {
        Ok("nvenc") => true,
        Ok("auto") => nvenc_works(),
        _ => false,
    })
}

/// A tiny test encode: ffmpeg can list h264_nvenc without a usable NVIDIA GPU or driver.
fn nvenc_works() -> bool {
    run(None, "ffmpeg", &["-v", "error", "-f", "lavfi", "-i", "color=s=256x256:d=0.1", "-c:v", "h264_nvenc", "-f", "null", "-"]).is_ok()
}

/// H.264 arguments at an x264-style quality `crf` (lower = better), on the GPU when --encoder says so.
pub fn h264(crf: u32) -> Vec<String> {
    if nvenc() {
        ["-c:v", "h264_nvenc", "-preset", "p5", "-tune", "hq", "-rc", "vbr", "-cq"].iter().map(|s| s.to_string()).chain([crf.to_string(), "-b:v".into(), "0".into()]).collect()
    } else {
        ["-c:v", "libx264", "-preset", "medium", "-crf"].iter().map(|s| s.to_string()).chain([crf.to_string()]).collect()
    }
}

/// Like `run` without a working directory, for binary stdout (raw video frames).
pub fn run_bytes<S: AsRef<OsStr>>(cmd: &str, args: &[S]) -> Result<Vec<u8>, String> {
    let out = Command::new(cmd).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().map_err(|e| format!("{cmd}: {e}"))?;
    if !out.status.success() {
        return Err(format!("{cmd} failed"));
    }
    Ok(out.stdout)
}

/// Runs a shell command line with `input` on stdin (the LLM command is user-configured, so it goes through sh).
pub fn run_stdin(cmdline: &str, input: &str) -> Result<String, String> {
    let (shell, flag) = if cfg!(windows) { ("cmd", "/C") } else { ("sh", "-c") };
    let mut child = Command::new(shell)
        .args([flag, cmdline])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{cmdline}: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let text = input.to_string();
    // written from a thread: a large prompt would otherwise deadlock against a full stdout pipe
    let writer = std::thread::spawn(move || stdin.write_all(text.as_bytes()));
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let _ = writer.join();
    if !out.status.success() {
        // some CLIs (claude: "Not logged in") report errors on stdout
        let (err, std) = (String::from_utf8_lossy(&out.stderr), String::from_utf8_lossy(&out.stdout));
        return Err(format!("{cmdline} failed: {}", if err.trim().is_empty() { std.trim() } else { err.trim() }));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// yt-dlp, tried again when YouTube refuses for a moment (403/429 when several downloads start at once). When it
/// still fails the way an out-of-date yt-dlp does (YouTube changed its player, the bot check, the JS challenge), edits'
/// own copy is updated and the run tried once more with the fixes yt-dlp's tracker gives: the newest challenge solver
/// (--remote-components ejs:github), a JS runtime it can find, and the browser's cookies for the bot check.
pub fn yt<S: AsRef<OsStr>>(args: &[S]) -> Result<String, String> {
    let mut wait = 3;
    loop {
        match run(None, "yt-dlp", args) {
            Err(e) if wait <= 12 && ["403", "429", "timed out", "Connection reset"].iter().any(|k| e.contains(k)) => {
                std::thread::sleep(std::time::Duration::from_secs(wait));
                wait *= 2;
            }
            Err(e) if stale(&e) => {
                eprintln!("edits: yt-dlp was refused, updating it and trying again with its latest fixes");
                crate::tools::update_yt_dlp();
                let mut again: Vec<std::ffi::OsString> = heal_args(&e).into_iter().map(Into::into).collect();
                again.extend(args.iter().map(|a| a.as_ref().to_os_string()));
                // an older yt-dlp without these options keeps the first, clearer error
                return run(None, "yt-dlp", &again).map_err(|e2| if e2.contains("no such option") { e } else { e2 });
            }
            r => return r,
        }
    }
}

/// Errors that an update or yt-dlp's extra options fix, as reported on its issue tracker.
fn stale(e: &str) -> bool {
    ["403", "not a bot", "n challenge", "JavaScript runtime", "Requested format is not available", "Precondition", "nsig", "player"]
        .iter().any(|k| e.contains(k))
}

fn heal_args(e: &str) -> Vec<String> {
    let mut a = vec!["--remote-components".to_string(), "ejs:github".into()];
    // deno is yt-dlp's default runtime; node or bun work when named
    if !have("deno") {
        if let Some(r) = ["node", "bun"].into_iter().find(|r| have(r)) {
            a.extend(["--js-runtimes".into(), r.into()]);
        }
    }
    if e.contains("not a bot") {
        if let Some(b) = browser() {
            eprintln!("edits: YouTube asks to sign in; using {b}'s YouTube cookies for this download");
            a.extend(["--cookies-from-browser".into(), b.into()]);
        }
    }
    a
}

/// The first browser with a profile on this machine, as yt-dlp names it. Firefox first: Chrome's cookies are
/// locked on Windows (app-bound encryption).
fn browser() -> Option<&'static str> {
    let h = home();
    let local = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default();
    let roaming = std::env::var("APPDATA").map(PathBuf::from).unwrap_or_default();
    let mac = h.join("Library/Application Support");
    [("firefox", vec![h.join(".mozilla/firefox"), mac.join("Firefox"), roaming.join("Mozilla/Firefox")]),
     ("chrome", vec![h.join(".config/google-chrome"), mac.join("Google/Chrome"), local.join("Google/Chrome")]),
     ("brave", vec![h.join(".config/BraveSoftware"), mac.join("BraveSoftware"), local.join("BraveSoftware")]),
     ("chromium", vec![h.join(".config/chromium"), mac.join("Chromium")]),
     ("edge", vec![h.join(".config/microsoft-edge"), mac.join("Microsoft Edge"), local.join("Microsoft/Edge")]),
     ("safari", vec![h.join("Library/Cookies")])]
        .into_iter().find(|(_, dirs)| dirs.iter().any(|d| !d.as_os_str().is_empty() && d.is_dir())).map(|(b, _)| b)
}

pub fn have(cmd: &str) -> bool {
    Command::new(cmd).arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok()
        || Command::new(cmd).arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok()
}

pub fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_else(|_| ".".into()))
}

/// Mean and standard deviation; a flat series gets sd 1 so z-scores stay 0 instead of NaN.
pub fn zscores(v: &[f64]) -> Vec<f64> {
    if v.is_empty() {
        return vec![];
    }
    let mean = v.iter().sum::<f64>() / v.len() as f64;
    let var = v.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / v.len() as f64;
    let sd = if var.sqrt() < 1e-9 { 1.0 } else { var.sqrt() };
    v.iter().map(|x| (x - mean) / sd).collect()
}

pub fn mmss(t: f64) -> String {
    let s = t.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_refused_youtube_gets_the_latest_solver() {
        assert!(super::stale("yt-dlp failed: ERROR: Sign in to confirm you're not a bot") && !super::stale("Video unavailable"));
        assert_eq!(super::heal_args("HTTP Error 403")[..2], ["--remote-components", "ejs:github"]);
    }

    #[test]
    fn now_is_a_day_and_a_time() {
        let (d, t) = super::now().expect("date works here");
        assert!(d.len() == 10 && d.as_bytes()[4] == b'-' && d.as_bytes()[7] == b'-', "{d}");
        assert!(t.len() == 5 && t.as_bytes()[2] == b'.', "{t}");
    }
}
