//! ffmpeg, ffprobe and yt-dlp, fetched without asking when the machine lacks them: the installers download them
//! with edits, and a run that finds one missing fetches it first. A static build goes into ~/.config/edits/bin,
//! checked against the publisher's SHA-256 list, and that folder is on PATH after the user's own: a machine that
//! already has ffmpeg or yt-dlp keeps using its own. `edits setup` does it by hand.
use crate::util::{have, home, run};
use std::path::{Path, PathBuf};

pub fn dir() -> PathBuf {
    home().join(".config/edits/bin")
}

fn exe(name: &str) -> String {
    if cfg!(windows) { format!("{name}.exe") } else { name.to_string() }
}

/// edits' own tools come after the user's: an installed ffmpeg or yt-dlp wins.
pub fn add_to_path() {
    let mut paths: Vec<PathBuf> = std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default();
    if !paths.contains(&dir()) {
        paths.push(dir());
        if let Ok(p) = std::env::join_paths(paths) {
            // set once at start, before any thread runs
            std::env::set_var("PATH", p);
        }
    }
}

/// Downloads `url` to `to`; curl is on Linux, macOS and Windows 10+.
fn get(url: &str, to: &Path) -> Result<(), String> {
    run(None, "curl", &["-fsSL", "--retry", "2", "--max-time", "900", "-o", &to.to_string_lossy(), url]).map(|_| ())
}

/// The hash a checksum list gives for `name` ("HASH  name" or "HASH *name" per line).
fn listed(list: &str, name: &str) -> Option<String> {
    list.lines().find_map(|l| {
        let (h, f) = l.split_once(char::is_whitespace)?;
        (f.trim().trim_start_matches('*') == name).then(|| h.trim().to_lowercase())
    })
}

fn check(file: &Path, want: Option<String>) -> Result<(), String> {
    let got = crate::update::sha256(file)?;
    if want.as_deref() == Some(got.as_str()) { Ok(()) } else { Err(format!("checksum mismatch for {}, nothing installed", file.display())) }
}

fn install(from: &Path, name: &str) -> Result<(), String> {
    std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
    let to = dir().join(exe(name));
    std::fs::copy(from, &to).map_err(|e| format!("{}: {e}", to.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&to, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn yt_dlp(tmp: &Path) -> Result<(), String> {
    let asset = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "yt-dlp_linux",
        ("linux", "aarch64") => "yt-dlp_linux_aarch64",
        ("macos", _) => "yt-dlp_macos",
        ("windows", _) => "yt-dlp.exe",
        _ => return Err("no yt-dlp build for this machine: install it with pip or your package manager".into()),
    };
    let base = "https://github.com/yt-dlp/yt-dlp/releases/latest/download";
    let (file, sums) = (tmp.join(asset), tmp.join("SHA2-256SUMS"));
    get(&format!("{base}/{asset}"), &file)?;
    get(&format!("{base}/SHA2-256SUMS"), &sums)?;
    check(&file, listed(&std::fs::read_to_string(&sums).unwrap_or_default(), asset))?;
    install(&file, "yt-dlp")
}

/// Linux and Windows: BtbN's static GPL builds (with libass, for captions). macOS: Martin Riedl's release builds.
fn ffmpeg(tmp: &Path) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        return ffmpeg_macos(tmp);
    }
    let (asset, folder) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => ("ffmpeg-master-latest-linux64-gpl.tar.xz", "ffmpeg-master-latest-linux64-gpl"),
        ("linux", "aarch64") => ("ffmpeg-master-latest-linuxarm64-gpl.tar.xz", "ffmpeg-master-latest-linuxarm64-gpl"),
        ("windows", "x86_64") => ("ffmpeg-master-latest-win64-gpl.zip", "ffmpeg-master-latest-win64-gpl"),
        _ => return Err("no ffmpeg build for this machine: install ffmpeg with your package manager".into()),
    };
    let base = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest";
    let (file, sums) = (tmp.join(asset), tmp.join("checksums.sha256"));
    get(&format!("{base}/{asset}"), &file)?;
    get(&format!("{base}/checksums.sha256"), &sums)?;
    check(&file, listed(&std::fs::read_to_string(&sums).unwrap_or_default(), asset))?;
    // tar reads .tar.xz on Linux and .zip on Windows 10+ (bsdtar)
    run(None, "tar", &["-xf", &file.to_string_lossy(), "-C", &tmp.to_string_lossy()])?;
    for name in ["ffmpeg", "ffprobe"] {
        install(&tmp.join(folder).join("bin").join(exe(name)), name)?;
    }
    Ok(())
}

fn ffmpeg_macos(tmp: &Path) -> Result<(), String> {
    let arch = if std::env::consts::ARCH == "aarch64" { "arm64" } else { "amd64" };
    let site = "https://ffmpeg.martin-riedl.de";
    let page = tmp.join("index.html");
    get(site, &page)?;
    let html = std::fs::read_to_string(&page).unwrap_or_default();
    // the newest release build (a version number, not a nightly "N-..."), listed newest first
    let re = regex::Regex::new(&format!(r#"/download/macos/{arch}/\d+_\d+\.\d+(?:\.\d+)?/"#)).unwrap();
    let dir_url = re.find(&html).map(|m| m.as_str().to_string()).ok_or("no macOS ffmpeg build found: brew install ffmpeg")?;
    for name in ["ffmpeg", "ffprobe"] {
        let (zip, sum) = (tmp.join(format!("{name}.zip")), tmp.join(format!("{name}.zip.sha256")));
        get(&format!("{site}{dir_url}{name}.zip"), &zip)?;
        get(&format!("{site}{dir_url}{name}.zip.sha256"), &sum)?;
        let want = std::fs::read_to_string(&sum).unwrap_or_default().split_whitespace().next().map(str::to_lowercase);
        check(&zip, want)?;
        run(None, "tar", &["-xf", &zip.to_string_lossy(), "-C", &tmp.to_string_lossy()])?;
        install(&tmp.join(name), name)?;
    }
    Ok(())
}

/// DeepFilterNet's `deep-filter` (speech kept, noise and music under it removed), fetched on first use. Its releases
/// publish no checksum list, so the hashes of the v0.5.6 builds are pinned here.
pub fn deep_filter() -> Result<String, String> {
    if have("deep-filter") {
        return Ok("deep-filter".into());
    }
    let own = dir().join(exe("deep-filter"));
    if own.is_file() {
        return Ok(own.to_string_lossy().into_owned());
    }
    let (target, sha) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => ("x86_64-unknown-linux-musl", "70775e251eee44c0f2451a1e833326cf8bcbbe304d3e7cd12851e6fce72ef7da"),
        ("linux", "aarch64") => ("aarch64-unknown-linux-gnu", "14e02a1c0028f3ca0bdf83b62b3336e56ba0556894ef295a95e8573f06557166"),
        ("macos", "aarch64") => ("aarch64-apple-darwin", "4601e7f4e4c03e59a4c5b5000216ef3add3e808799cfccd95e14e83ea4611081"),
        ("macos", "x86_64") => ("x86_64-apple-darwin", "d3be84003acb7c23e738ad7f70a158ec779a8d233a82e7fa3e717d112eb5b50f"),
        ("windows", "x86_64") => ("x86_64-pc-windows-msvc.exe", "75e11fa16445f560cb6b021521ddb89e89270d13b83089705d98776f58fd7915"),
        _ => return Err("no DeepFilterNet build for this machine".into()),
    };
    eprintln!("edits: downloading DeepFilterNet (once, ~35 MB)...");
    let tmp = std::env::temp_dir().join(format!("edits-df-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let file = tmp.join(exe("deep-filter"));
    let res = get(&format!("https://github.com/Rikorose/DeepFilterNet/releases/download/v0.5.6/deep-filter-0.5.6-{target}"), &file)
        .and_then(|_| check(&file, Some(sha.into())))
        .and_then(|_| install(&file, "deep-filter"));
    let _ = std::fs::remove_dir_all(&tmp);
    res.map(|_| own.to_string_lossy().into_owned())
}

const ORT: &str = "1.31.0";

/// ONNX Runtime's library (the CPU build, ~8 MB), fetched on first use from the builds sherpa-onnx uses
/// (github.com/csukuangfj/onnxruntime-libs), which publish no checksum list: the hashes are pinned here.
pub fn onnxruntime() -> Result<PathBuf, String> {
    let (asset, sha, lib) = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => (format!("onnxruntime-linux-x64-glibc2_17-MinSizeRel-{ORT}.zip"), "2f1bd0a4da08bc7e2bae84f8cb973327c6984c2e2dfe945cf7f8c8e400820503", "libonnxruntime.so"),
        ("linux", "aarch64") => (format!("onnxruntime-linux-aarch64-glibc2_17-MinSizeRel-{ORT}.zip"), "0769ed211d2aa5f86ff839b7e9fd81c19bb0880628882be7854182001567e90d", "libonnxruntime.so"),
        ("macos", "aarch64") => (format!("onnxruntime-osx-arm64-{ORT}.zip"), "60a558189937147849ffc90240f57444198d3f57d7c7cf290e07b2dabd5d90a0", "libonnxruntime.dylib"),
        ("macos", "x86_64") => (format!("onnxruntime-osx-x86_64-{ORT}.zip"), "759b64ee4124b347c41b17e7f89e029b42ef6d5ca881f8d008832daba21cc4d2", "libonnxruntime.dylib"),
        // MT: the C runtime built in, nothing else to install
        ("windows", "x86_64") => (format!("onnxruntime-win-x64-MT-MinSizeRel-{ORT}.tar.bz2"), "03f36b5bb1cf2cef278573d8be26067ae3183851063a1d531a2bc3a41d195f6e", "onnxruntime.dll"),
        _ => return Err("no ONNX Runtime build for this machine".into()),
    };
    let home_dir = dir().join(format!("onnxruntime-{ORT}"));
    let found = |d: &Path| crate::library::walk(d).into_iter().find(|p| p.file_name().is_some_and(|n| n == lib));
    if let Some(f) = found(&home_dir) {
        return Ok(f);
    }
    eprintln!("edits: downloading ONNX Runtime (once, ~8 MB)...");
    let tmp = std::env::temp_dir().join(format!("edits-ort-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let file = tmp.join(&asset);
    let res = get(&format!("https://github.com/csukuangfj/onnxruntime-libs/releases/download/v{ORT}/{asset}"), &file)
        .and_then(|_| check(&file, Some(sha.into())))
        // tar opens .zip and .tar.bz2 on Linux, macOS and Windows 10+ (bsdtar); unzip when tar cannot
        .and_then(|_| {
            std::fs::create_dir_all(&home_dir).map_err(|e| e.to_string())?;
            run(None, "tar", &["-xf", &file.to_string_lossy(), "-C", &home_dir.to_string_lossy()]).map(|_| ())
                .or_else(|e| if asset.ends_with(".zip") { crate::util::unzip(&file, &home_dir) } else { Err(e) })
        });
    let _ = std::fs::remove_dir_all(&tmp);
    res?;
    found(&home_dir).ok_or(format!("{lib} was not in the ONNX Runtime download"))
}

/// Robust Video Matting (MobileNetV3, 14 MB): who is in front, frame by frame, with soft edges and hair.
pub fn matting_model() -> Result<PathBuf, String> {
    let file = home().join(".local/share/edits/models/rvm_mobilenetv3_fp32.onnx");
    if file.is_file() {
        return Ok(file);
    }
    eprintln!("edits: downloading the subject model (once, 14 MB)...");
    std::fs::create_dir_all(file.parent().unwrap()).map_err(|e| e.to_string())?;
    let part = file.with_extension("part");
    let res = get("https://github.com/PeterL1n/RobustVideoMatting/releases/download/v1.0.0/rvm_mobilenetv3_fp32.onnx", &part)
        .and_then(|_| check(&part, Some("88d4531297118f595bf2fd60f6f566aec2e559393802d1f436c380f0cbbd2828".into())))
        .and_then(|_| std::fs::rename(&part, &file).map_err(|e| e.to_string()));
    if res.is_err() {
        let _ = std::fs::remove_file(&part);
    }
    res.map(|_| file)
}

/// Downloads whichever of ffmpeg/ffprobe and yt-dlp this machine does not have; one it has is left alone.
pub fn ensure() -> Result<(), String> {
    let need_ff = !have("ffmpeg") || !have("ffprobe");
    let need_yt = !have("yt-dlp");
    if !need_ff && !need_yt {
        return Ok(());
    }
    let tmp = std::env::temp_dir().join(format!("edits-tools-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let res = (|| {
        if need_yt {
            eprintln!("edits: downloading yt-dlp (~40 MB)...");
            yt_dlp(&tmp)?;
        }
        if need_ff {
            eprintln!("edits: downloading ffmpeg (once, a minute or two)...");
            ffmpeg(&tmp)?;
        }
        Ok(())
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    res
}

/// Keeps edits' own yt-dlp current (YouTube changes break old ones); only the copy edits installed.
pub fn update_yt_dlp() {
    let own = dir().join(exe("yt-dlp"));
    if own.exists() {
        let _ = run(None, &own.to_string_lossy(), &["-U"]);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn checksum_lists_are_read_both_ways() {
        let list = "aaa  yt-dlp_linux\nBBB *ffmpeg-master-latest-win64-gpl.zip\n";
        assert_eq!(super::listed(list, "yt-dlp_linux").as_deref(), Some("aaa"));
        assert_eq!(super::listed(list, "ffmpeg-master-latest-win64-gpl.zip").as_deref(), Some("bbb"));
        assert_eq!(super::listed(list, "yt-dlp"), None, "a prefix is not a match");
    }
}
