//! Sources: a pack's own program finding new things to make, run on its own like an Omarchy plugin. It gets its
//! settings as JSON on stdin and prints one JSON object a line, `{"id", "title", "link", "video", "image",
//! "summary"}`; a line with a video is made from the video, one without from its title, as RSS is (rss.rs).
//!
//! Alone means: its own data folder as home, a clean environment, two minutes, 4 MB of output; on Linux inside
//! bubblewrap (the pack's folder read-only, its data folder, the system, nothing of the person's home, the internet
//! only when the manifest says `"network": true`), on macOS inside sandbox-exec. Windows has no such box, so there a
//! source runs only with the clean environment. An added source runs only after `edits packs trust ID`.
use crate::packs::{self, Pack};
use crate::rss::Item;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const TIME: Duration = Duration::from_secs(120);
const MOST: usize = 4 << 20;

/// A source pack by its id, or the last part of it (io.github.you.trends: trends).
pub fn find(what: &str) -> Result<Pack, String> {
    let want = what.to_lowercase();
    packs::all().into_iter().find(|p| p.manifest.source.is_some() && (p.manifest.id == want || p.manifest.id.ends_with(&format!(".{want}"))))
        .ok_or(format!("no source pack \"{what}\": edits packs lists them (the ones that say source)"))
}

/// The items a source's output names.
pub fn parse(out: &str) -> Vec<Item> {
    out.lines().filter_map(|l| serde_json::from_str::<serde_json::Value>(l.trim()).ok()).filter_map(|v| {
        let s = |k: &str| v[k].as_str().map(str::trim).filter(|x| !x.is_empty()).map(String::from);
        let title = s("title")?;
        let link = s("link").unwrap_or_default();
        Some(Item { id: s("id").unwrap_or_else(|| if link.is_empty() { title.clone() } else { link.clone() }), title, link, summary: s("summary").unwrap_or_default(), image: s("image"), video: s("video") })
    }).collect()
}

/// The command that runs `prog` alone: bubblewrap on Linux, sandbox-exec on macOS, just itself on Windows.
fn boxed(prog: &Path, folder: &Path, data: &Path, network: bool) -> Result<Command, String> {
    let shell = prog.extension().is_some_and(|e| e == "sh");
    let run: Vec<String> = if cfg!(windows) && prog.extension().is_some_and(|e| e == "ps1") {
        vec!["powershell".into(), "-NoProfile".into(), "-ExecutionPolicy".into(), "Bypass".into(), "-File".into(), prog.display().to_string()]
    } else if shell {
        vec!["sh".into(), prog.display().to_string()]
    } else {
        vec![prog.display().to_string()]
    };
    let (f, d) = (folder.display().to_string(), data.display().to_string());
    let mut cmd = if cfg!(target_os = "linux") {
        let mut c = Command::new("bwrap");
        for sys in ["/usr", "/bin", "/sbin", "/lib", "/lib64", "/etc", "/opt", "/nix"] {
            c.args(["--ro-bind-try", sys, sys]);
        }
        c.args(["--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp", "--ro-bind", &f, &f, "--bind", &d, &d, "--chdir", &d, "--unshare-all"]);
        if network {
            c.arg("--share-net");
        }
        c.args(["--die-with-parent", "--new-session", "--"]).args(&run);
        c
    } else if cfg!(target_os = "macos") {
        // the last rule that matches wins: nothing of the home folder, writes only in its data folder
        let home = crate::util::home().display().to_string();
        let profile = format!("(version 1)(allow default){}(deny file-read* (subpath \"{home}\"))(allow file-read* (subpath \"{f}\") (subpath \"{d}\"))\
            (deny file-write*)(allow file-write* (subpath \"{d}\") (subpath \"/private/tmp\") (subpath \"/private/var/folders\") (literal \"/dev/null\"))",
            if network { "" } else { "(deny network*)" });
        let mut c = Command::new("sandbox-exec");
        c.args(["-p", &profile]).args(&run);
        c
    } else {
        let mut c = Command::new(&run[0]);
        c.args(&run[1..]);
        c
    };
    cmd.env_clear().env("HOME", data).env("PATH", std::env::var("PATH").unwrap_or_default()).env("LANG", "C.UTF-8").current_dir(data)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)]
    {
        cmd.env("USERPROFILE", data).env("SystemRoot", std::env::var("SystemRoot").unwrap_or_default());
    }
    Ok(cmd)
}

/// Runs a source pack's program, alone, and reads what it found.
pub fn run(p: &Pack) -> Result<Vec<Item>, String> {
    let m = &p.manifest;
    let src = m.source.as_ref().ok_or(format!("{} is not a source", m.id))?;
    if !packs::trusted(p) {
        return Err(format!("{0} runs a program: read it in {1}, then edits packs trust {0}", m.id, packs::dir().join(&m.id).display()));
    }
    if packs::origin(&m.id) == packs::Origin::BuiltIn {
        return Err(format!("{}: built-in packs carry no program", m.id));
    }
    let folder = packs::dir().join(&m.id);
    let file = if cfg!(windows) { src.windows.clone().ok_or(format!("{} has no Windows program", m.id))? } else { src.run.clone() };
    let prog = folder.join(&file);
    if cfg!(target_os = "linux") && which("bwrap").is_none() {
        return Err("sources run inside bubblewrap here: install bubblewrap (bwrap)".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&prog, std::fs::Permissions::from_mode(0o755));
    }
    let data = crate::util::home().join(".local/share/edits/pack-data").join(&m.id);
    std::fs::create_dir_all(&data).map_err(|e| e.to_string())?;
    let mut child = boxed(&prog, &folder, &data, src.network)?.spawn().map_err(|e| format!("{}: {e}", m.id))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(serde_json::json!({"settings": packs::settings(m)}).to_string().as_bytes());
    }
    let mut out = child.stdout.take().ok_or("no output")?;
    let reader = std::thread::spawn(move || {
        let mut buf = vec![];
        let _ = (&mut out).take(MOST as u64).read_to_end(&mut buf);
        buf
    });
    let start = Instant::now();
    let status = loop {
        if let Some(s) = child.try_wait().map_err(|e| e.to_string())? {
            break s;
        }
        if start.elapsed() > TIME {
            let _ = child.kill();
            return Err(format!("{} ran over {} s and was stopped", m.id, TIME.as_secs()));
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    let mut err = String::new();
    if let Some(mut e) = child.stderr.take() {
        let _ = (&mut e).take(4096).read_to_string(&mut err);
    }
    let buf = reader.join().unwrap_or_default();
    if !status.success() {
        return Err(format!("{} failed: {}", m.id, err.lines().last().unwrap_or("no reason given")));
    }
    Ok(parse(&String::from_utf8_lossy(&buf)))
}

fn which(name: &str) -> Option<PathBuf> {
    std::env::split_paths(&std::env::var_os("PATH")?).map(|d| d.join(name)).find(|p| p.is_file())
}

/// `edits source <pack> [--list|--json] [-n N] [edit options]`
pub fn cmd(args: &[String]) -> Result<(), String> {
    let what = args.first().filter(|a| !a.starts_with('-')).ok_or("usage: edits source <pack> [--list|--json] [-n N] [edit options]")?;
    let p = find(what)?;
    let items = run(&p)?;
    let (mut list, mut json, mut n, mut rest) = (false, false, 1usize, vec![]);
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--list" => list = true,
            "--json" => (list, json) = (true, true),
            "-n" => {
                n = args.get(i + 1).and_then(|v| v.parse().ok()).filter(|n| (1..=20).contains(n)).ok_or("-n takes 1-20")?;
                i += 1;
            }
            a => rest.push(a.to_string()),
        }
        i += 1;
    }
    if list {
        for it in &items {
            if json {
                println!("{}", serde_json::json!({"id": it.id, "title": it.title, "link": it.link, "video": it.video, "image": it.image, "summary": it.summary}));
            } else {
                println!("{}\n  {}", it.title, it.video.as_deref().unwrap_or(&it.link));
            }
        }
        return Ok(());
    }
    let mut made = vec![];
    for it in items.iter().take(n) {
        eprintln!("edits: {}", it.title);
        made.extend(crate::follow::make_from(&[crate::rss::args_of(it), rest.clone()].concat())?);
    }
    made.iter().for_each(|c| println!("{}", c.display()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_speaks_json_lines() {
        let v = parse("{\"title\": \"A clip\", \"video\": \"https://youtu.be/x\"}\nnoise\n{\"title\": \" \"}\n{\"id\": \"7\", \"title\": \"News\", \"link\": \"https://x/7\", \"image\": \"https://x/7.jpg\"}\n");
        assert_eq!(v.len(), 2);
        assert_eq!((v[0].id.as_str(), v[0].video.as_deref()), ("A clip", Some("https://youtu.be/x")));
        assert_eq!((v[1].id.as_str(), v[1].link.as_str(), v[1].image.as_deref(), v[1].video.as_deref()), ("7", "https://x/7", Some("https://x/7.jpg"), None));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_source_sees_only_its_own_folders() {
        if which("bwrap").is_none() {
            return;
        }
        let t = std::env::temp_dir().join(format!("edits-source-{}", std::process::id()));
        let (folder, data) = (t.join("pack"), t.join("data"));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(t.join("secret"), "x").unwrap();
        std::fs::write(folder.join("source.sh"), format!("cat {}/secret >/dev/null 2>&1 && echo leak; cat; echo > ok; ls", t.display())).unwrap();
        let mut c = boxed(&folder.join("source.sh"), &folder, &data, false).unwrap();
        let Ok(mut child) = c.spawn() else { return };
        child.stdin.take().unwrap().write_all(b"{\"settings\":{}}").unwrap();
        let out = child.wait_with_output().unwrap();
        let s = String::from_utf8_lossy(&out.stdout);
        // bubblewrap may be refused where user namespaces are off: then there is nothing to check here
        if out.status.success() {
            assert!(!s.contains("leak") && s.contains("{\"settings\":{}}") && s.contains("ok"), "{s}");
            assert!(data.join("ok").is_file(), "it writes in its data folder");
        }
        let _ = std::fs::remove_dir_all(&t);
    }
}
