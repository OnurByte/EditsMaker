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

/// yt-dlp, tried again when YouTube refuses for a moment (403/429 when several downloads start at once).
pub fn yt<S: AsRef<OsStr>>(args: &[S]) -> Result<String, String> {
    let mut wait = 3;
    loop {
        match run(None, "yt-dlp", args) {
            Err(e) if wait <= 12 && ["403", "429", "timed out", "Connection reset"].iter().any(|k| e.contains(k)) => {
                std::thread::sleep(std::time::Duration::from_secs(wait));
                wait *= 2;
            }
            r => return r,
        }
    }
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
