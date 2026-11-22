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

static GPU: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

/// What `--encoder` and `edits encoder` take: auto (the default), a GPU's encoder, or cpu.
pub const ENCODERS: &[&str] = &["auto", "nvenc", "videotoolbox", "cpu"];

/// What renders when nothing is forced: Apple's VideoToolbox on a Mac, an NVIDIA GPU elsewhere, each only when a
/// test encode works (the encoders are listed on machines that cannot run them), else the CPU. A GPU renders
/// several times faster.
pub fn auto_gpu() -> &'static str {
    let hw = if cfg!(target_os = "macos") { "videotoolbox" } else { "nvenc" };
    if works(&format!("h264_{hw}")) { hw } else { "cpu" }
}

fn pick(name: &str) -> &'static str {
    match name {
        "nvenc" => "nvenc",
        "videotoolbox" => "videotoolbox",
        "cpu" => "cpu",
        _ => auto_gpu(),
    }
}

/// The video encoder for this run (from --encoder, else EDITS_ENCODER, else `edits encoder`).
pub fn set_encoder(name: &str) -> Result<(), String> {
    if !ENCODERS.contains(&name) {
        return Err(format!("--encoder must be one of {}", ENCODERS.join(", ")));
    }
    let _ = GPU.set(pick(name));
    Ok(())
}

/// What renders this run: "nvenc", "videotoolbox" or "cpu".
pub fn gpu() -> &'static str {
    GPU.get_or_init(|| pick(&encoder_choice()))
}

fn encoder_file() -> PathBuf {
    home().join(".config/edits/encoder")
}

/// The encoder asked for: EDITS_ENCODER, else the one `edits encoder` saved, else auto.
pub fn encoder_choice() -> String {
    std::env::var("EDITS_ENCODER").ok()
        .or_else(|| std::fs::read_to_string(encoder_file()).ok())
        .map(|s| s.trim().to_string()).filter(|s| ENCODERS.contains(&s.as_str()))
        .unwrap_or_else(|| "auto".into())
}

pub fn describe(g: &str) -> &'static str {
    match g {
        "nvenc" => "NVIDIA GPU (h264_nvenc)",
        "videotoolbox" => "Apple VideoToolbox (h264_videotoolbox)",
        _ => "CPU (libx264)",
    }
}

/// `edits encoder [auto|nvenc|videotoolbox|cpu]`: what renders the video (GPU or CPU), saved for every run, or what it is now.
pub fn encoder_cmd(want: Option<&str>) -> Result<(), String> {
    if let Some(w) = want {
        if !ENCODERS.contains(&w) {
            return Err(format!("edits encoder: {w}? use auto (the GPU when it works), nvenc (NVIDIA), videotoolbox (Apple) or cpu"));
        }
        let f = encoder_file();
        std::fs::create_dir_all(f.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
        std::fs::write(&f, w).map_err(|e| e.to_string())?;
    }
    let (choice, here) = (encoder_choice(), auto_gpu());
    println!("edits encoder: {choice} -> renders on the {}; hardware encoder that works here: {}", describe(pick(&choice)),
        if here == "cpu" { "none" } else { describe(here) });
    if !matches!(choice.as_str(), "auto" | "cpu") && choice != here {
        println!("edits: the {choice} test encode fails here, renders will fail; edits encoder auto falls back to the CPU");
    }
    Ok(())
}

/// A tiny test encode: ffmpeg lists hardware encoders on machines that cannot run them.
fn works(encoder: &str) -> bool {
    run(None, "ffmpeg", &["-v", "error", "-f", "lavfi", "-i", "color=s=256x256:d=0.1", "-c:v", encoder, "-f", "null", "-"]).is_ok()
}

/// A bitrate standing for an x264-style quality `crf` on a vertical 1080p frame, for VideoToolbox (its constant
/// quality mode exists only on Apple Silicon): 12 Mb/s at crf 18, halved every 6 steps as x264 roughly does.
pub fn crf_bitrate(crf: f64) -> String {
    format!("{:.0}k", 12000.0 * 2f64.powf((18.0 - crf) / 6.0))
}

static FAST: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// --preview: encoders trade a little size for speed.
pub fn set_fast() {
    FAST.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// H.264 arguments at an x264-style quality `crf` (lower = better), on the GPU when --encoder says so.
pub fn h264(crf: u32) -> Vec<String> {
    let fast = FAST.load(std::sync::atomic::Ordering::Relaxed);
    let a: &[&str] = match gpu() {
        "nvenc" => &["-c:v", "h264_nvenc", "-preset", if fast { "p1" } else { "p5" }, "-tune", "hq", "-rc", "vbr", "-b:v", "0", "-cq"],
        "videotoolbox" => &["-c:v", "h264_videotoolbox", "-profile:v", "high", "-b:v"],
        _ => &["-c:v", "libx264", "-preset", if fast { "veryfast" } else { "medium" }, "-crf"],
    };
    let level = if gpu() == "videotoolbox" { crf_bitrate(crf as f64) } else { crf.to_string() };
    a.iter().map(|s| s.to_string()).chain([level]).collect()
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

/// CRC-32 (IEEE), as zip and Shazam's signatures use it.
pub fn crc32(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
        }
    }
    !c
}

pub fn mmss(t: f64) -> String {
    let s = t.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

/// A request through curl, given as curl config lines (`url`, `header`, `data-binary`, `request`, ...) on stdin, so
/// tokens and keys never show on a command line. Returns the HTTP status and the body.
pub fn curl(opts: &[(&str, &str)]) -> Result<(u16, String), String> {
    let q = |v: &str| v.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t");
    let mut cfg: String = opts.iter().map(|(k, v)| if v.is_empty() { format!("{k}\n") } else { format!("{k} = \"{}\"\n", q(v)) }).collect();
    // a caller's own max-time wins (curl takes the last one given)
    if !opts.iter().any(|(k, _)| *k == "max-time") {
        cfg.push_str("max-time = 900\n");
    }
    cfg.push_str("silent\nshow-error\nlocation\nwrite-out = \"\\n%{http_code}\"\n");
    let out = run_stdin("curl -K -", &cfg)?;
    let (body, code) = out.rsplit_once('\n').unwrap_or(("", &out));
    Ok((code.trim().parse().unwrap_or(0), body.to_string()))
}

/// curl's answer as JSON, an error unless it is a 2xx.
pub fn curl_json(opts: &[(&str, &str)]) -> Result<serde_json::Value, String> {
    let (code, body) = curl(opts)?;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap_or(serde_json::Value::Null);
    if !(200..300).contains(&code) {
        let why = [&v["error"]["message"], &v["message"], &v["error"]].iter().find_map(|x| x.as_str().map(str::to_string))
            .unwrap_or_else(|| body.chars().take(200).collect());
        return Err(format!("HTTP {code}: {why}"));
    }
    Ok(v)
}

/// A file only this user can read (keys, tokens).
pub fn write_private(path: &Path, text: &str) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    o.open(path).and_then(|mut f| f.write_all(text.as_bytes())).map_err(|e| format!("{}: {e}", path.display()))
}

/// Opens a web address in the person's own browser.
pub fn open_url(url: &str) -> Result<(), String> {
    let (cmd, args): (&str, Vec<&str>) = if cfg!(target_os = "macos") { ("open", vec![url]) } else if cfg!(windows) { ("cmd", vec!["/C", "start", "", url]) } else { ("xdg-open", vec![url]) };
    std::process::Command::new(cmd).args(args).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn().map(drop).map_err(|e| format!("{cmd}: {e}"))
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
