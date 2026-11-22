//! Updates from GitHub Releases: `edits update` now, and once a day on its own unless turned off.
//! The same steps as install.sh: the asset for this machine, its SHA-256 from SHA256SUMS, then the binary swapped in place.
use crate::util::{home, run};
use std::path::{Path, PathBuf};

pub const REPO: &str = "OnurByte/EditsMaker";
pub const GITHUB: &str = "https://github.com/OnurByte";
pub const X: &str = "https://x.com/onurbinary";
const DAY: u64 = 24 * 3600;

fn dir() -> PathBuf {
    home().join(".config/edits")
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// "v0.7.0" or "0.7.0" -> (0, 7, 0)
fn parse(v: &str) -> Option<(u32, u32, u32)> {
    let mut it = v.trim().trim_start_matches('v').split('.').map(|p| p.parse::<u32>().ok());
    Some((it.next()??, it.next()??, it.next().flatten().unwrap_or(0)))
}

/// The newest release's tag.
pub fn latest() -> Result<String, String> {
    let body = run(None, "curl", &["-fsSL", "--max-time", "5", "-H", "Accept: application/vnd.github+json", &format!("https://api.github.com/repos/{REPO}/releases/latest")])?;
    let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    v["tag_name"].as_str().map(String::from).ok_or("no release found".into())
}

fn newer(tag: &str) -> bool {
    matches!((parse(tag), parse(env!("CARGO_PKG_VERSION"))), (Some(a), Some(b)) if a > b)
}

/// The release asset name for this machine.
fn asset() -> Option<String> {
    let t = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu",
        ("macos", "x86_64") => "x86_64-apple-darwin",
        ("macos", "aarch64") => "aarch64-apple-darwin",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc",
        _ => return None,
    };
    Some(if cfg!(windows) { format!("edits-{t}.zip") } else { format!("edits-{t}.tar.gz") })
}

pub fn sha256(f: &Path) -> Result<String, String> {
    let p = f.to_string_lossy().into_owned();
    let out = if cfg!(windows) {
        run(None, "certutil", &["-hashfile", &p, "SHA256"]).map(|o| o.lines().nth(1).unwrap_or("").replace(' ', ""))
    } else {
        run(None, "sha256sum", &[&p]).or_else(|_| run(None, "shasum", &["-a", "256", &p])).map(|o| o.split_whitespace().next().unwrap_or("").to_string())
    }?;
    Ok(out.trim().to_lowercase())
}

/// Downloads `tag`, checks it and puts it where this program runs from.
fn install(tag: &str) -> Result<(), String> {
    let name = asset().ok_or("no prebuilt binary for this machine: build it from source")?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let tmp = std::env::temp_dir().join(format!("edits-update-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let res = (|| {
        let base = format!("https://github.com/{REPO}/releases/download/{tag}");
        let (file, sums) = (tmp.join(&name), tmp.join("SHA256SUMS"));
        run(None, "curl", &["-fsSL", "--max-time", "300", "-o", &file.to_string_lossy(), &format!("{base}/{name}")])?;
        run(None, "curl", &["-fsSL", "--max-time", "30", "-o", &sums.to_string_lossy(), &format!("{base}/SHA256SUMS")])?;
        let list = std::fs::read_to_string(&sums).map_err(|e| e.to_string())?;
        let want = list.lines().find_map(|l| l.split_once(char::is_whitespace).filter(|(_, f)| f.trim().trim_start_matches('*') == name).map(|(h, _)| h.to_lowercase()));
        if want.as_deref() != Some(sha256(&file)?.as_str()) {
            return Err("checksum mismatch, nothing changed".into());
        }
        // bsdtar opens the zip too (it is tar.exe on Windows 10 and later)
        run(None, "tar", &["-xf", &file.to_string_lossy(), "-C", &tmp.to_string_lossy()])?;
        let bin = tmp.join(if cfg!(windows) { "edits.exe" } else { "edits" });
        let next = exe.with_extension("new");
        std::fs::copy(&bin, &next).map_err(|e| format!("{}: {e}", next.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&next, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
        }
        // a running exe can be renamed but not replaced on Windows; on unix the rename alone is atomic
        let old = exe.with_extension("old");
        if cfg!(windows) {
            let _ = std::fs::remove_file(&old);
            std::fs::rename(&exe, &old).map_err(|e| e.to_string())?;
        }
        std::fs::rename(&next, &exe).map_err(|e| {
            // put the old one back, so a failed update never leaves no edits at all
            if cfg!(windows) {
                let _ = std::fs::rename(&old, &exe);
            }
            let _ = std::fs::remove_file(&next);
            e.to_string()
        })
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    if res.is_ok() {
        // the new edits brings its skills; a guard the user installed points at it (none is added unasked)
        let _ = run(None, &exe.to_string_lossy(), &["guard", "refresh"]);
        let _ = run(None, &exe.to_string_lossy(), &["skills", "install"]);
    }
    res
}

fn off_file() -> PathBuf {
    dir().join("auto-update-off")
}

/// `edits update [check|on|off]`
pub fn cmd(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("off") => {
            std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
            std::fs::write(off_file(), "").map_err(|e| e.to_string())?;
            println!("edits: automatic updates off (edits update on to turn them back on)");
            Ok(())
        }
        Some("on") => {
            let _ = std::fs::remove_file(off_file());
            println!("edits: automatic updates on (checked once a day)");
            Ok(())
        }
        Some("check") => {
            let tag = latest()?;
            println!("edits {}: latest is {tag}{}", env!("CARGO_PKG_VERSION"), if newer(&tag) { " (edits update installs it)" } else { ", up to date" });
            Ok(())
        }
        None => {
            let tag = latest()?;
            if !newer(&tag) {
                println!("edits {} is up to date", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            install(&tag)?;
            println!("edits: updated {} -> {tag}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some(x) => Err(format!("edits update: unknown '{x}' (use check, on or off)")),
    }
}

fn auto_on() -> bool {
    std::env::var("EDITS_AUTO_UPDATE").map_or(true, |v| !matches!(v.as_str(), "0" | "off" | "no" | "false")) && !off_file().exists()
}

/// Once a day: install a newer release before this run. Quiet when offline; never stops the run.
pub fn auto() {
    let exe = std::env::current_exe().unwrap_or_default();
    let _ = std::fs::remove_file(exe.with_extension("old"));
    // a cargo build is someone working on edits: never overwrite it
    if !auto_on() || exe.components().any(|c| c.as_os_str() == "target") {
        return;
    }
    let stamp = dir().join("update-check");
    let last = std::fs::read_to_string(&stamp).ok().and_then(|s| s.trim().parse::<u64>().ok()).unwrap_or(0);
    if now().saturating_sub(last) < DAY {
        return;
    }
    let _ = std::fs::create_dir_all(dir());
    let _ = std::fs::write(&stamp, now().to_string());
    crate::tools::update_yt_dlp();
    if let Ok(tag) = latest() {
        if newer(&tag) {
            eprintln!("edits: updating {} -> {tag} (turn off: edits update off)", env!("CARGO_PKG_VERSION"));
            match install(&tag) {
                Ok(()) => eprintln!("edits: updated to {tag}; it runs from the next command"),
                Err(e) => eprintln!("edits: update failed ({e}); this run uses {}", env!("CARGO_PKG_VERSION")),
            }
        }
    }
}

/// `edits info`
pub fn info() -> Result<(), String> {
    println!("EditsMaker {}", env!("CARGO_PKG_VERSION"));
    println!("  GitHub    {GITHUB}");
    println!("  X         {X}");
    println!("  project   https://github.com/{REPO}");
    println!("  issues    https://github.com/{REPO}/issues");
    let up = if auto_on() { "on, checked once a day (edits update off)" } else { "off (edits update on)" };
    println!("  updates   {up}");
    match latest() {
        Ok(t) if newer(&t) => println!("  latest    {t}: run edits update"),
        Ok(t) => println!("  latest    {t}, up to date"),
        Err(_) => println!("  latest    unknown (offline?)"),
    }
    println!("  program   {}", std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default());
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn versions() {
        assert_eq!(super::parse("v0.7.0"), Some((0, 7, 0)));
        assert_eq!(super::parse("1.2"), Some((1, 2, 0)));
        assert_eq!(super::parse("vx"), None);
        assert!(super::newer("v99.0.0"));
        assert!(!super::newer(concat!("v", env!("CARGO_PKG_VERSION"))));
        assert!(!super::newer("v0.0.1"));
    }
}
