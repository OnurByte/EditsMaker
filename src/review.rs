//! `edits review clip.mp4`: the clip in the app's review screen. The person picks a moment and a spot in the frame
//! and says what to change; the notes are saved beside the clip as NAME.notes.json and printed for the agent, which
//! changes exactly that. `--frames` gives the app what it shows: frames over time, the cuts, what the clip was made with.
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// Frames a second the review screen steps through.
const FPS: f64 = 2.0;

pub fn notes_path(clip: &Path) -> PathBuf {
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

/// What the review screen shows, as JSON: the clip's frames (FPS a second, small jpgs in a folder of their own), its
/// cuts and length, and the preset, song and look it was made with.
fn sheet(clip: &Path) -> Result<Value, String> {
    let total = crate::fetch::duration(clip)?;
    let dir = std::env::temp_dir().join(format!("edits-review-{:08x}", crate::util::crc32(clip.to_string_lossy().as_bytes())));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    crate::util::run(None, "ffmpeg", &["-v", "error", "-y", "-i", &clip.to_string_lossy(), "-vf", &format!("fps={FPS},scale=320:-2"), "-q:v", "4",
        &dir.join("%05d.jpg").to_string_lossy()])?;
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).collect();
    files.sort();
    let frames: Vec<Value> = files.iter().enumerate().map(|(i, f)| json!({"t": i as f64 / FPS, "file": f})).collect();
    let side: Value = std::fs::read_to_string(clip.with_extension("json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or(Value::Null);
    let about: Vec<String> = ["preset", "song", "look"].iter().filter_map(|k| side[*k].as_str().filter(|s| !s.is_empty()).map(|s| format!("{k} {s}"))).collect();
    Ok(json!({"clip": clip, "name": clip.file_name().unwrap_or_default().to_string_lossy(), "total": total, "cuts": crate::check::cuts(clip),
        "about": about, "frames": frames, "notes": notes_path(clip)}))
}

/// `edits review <clip.mp4> [--notes | --frames]`
pub fn cmd(args: &[String]) -> Result<(), String> {
    let clip = std::path::absolute(args.first().ok_or("usage: edits review <clip.mp4> [--notes]")?).map_err(|e| e.to_string())?;
    if !clip.is_file() {
        return Err(format!("no clip at {}", clip.display()));
    }
    let flag = |f: &str| args.iter().any(|a| a == f);
    if flag("--frames") {
        println!("{}", sheet(&clip)?);
        return Ok(());
    }
    let read = || -> Result<Value, String> {
        let saved = std::fs::read_to_string(notes_path(&clip)).map_err(|_| format!("no notes saved for {} yet", clip.display()))?;
        serde_json::from_str(&saved).map_err(|e| e.to_string())
    };
    if flag("--notes") {
        print!("notes on {}:\n{}", clip.display(), describe(&read()?, &crate::check::cuts(&clip)));
        return Ok(());
    }
    // the review screen of the app beside this edits; it saves the notes and closes
    let app = std::env::current_exe().map_err(|e| e.to_string())?.with_file_name(format!("edits-app{}", std::env::consts::EXE_SUFFIX));
    let before = std::fs::metadata(notes_path(&clip)).and_then(|m| m.modified()).ok();
    eprintln!("edits: reviewing {} in the app; waiting for the notes", clip.display());
    let status = std::process::Command::new(&app).arg("--review").arg(&clip).status()
        .map_err(|e| format!("{}: {e} (the app comes with the release download: edits update)", app.display()))?;
    let after = std::fs::metadata(notes_path(&clip)).and_then(|m| m.modified()).ok();
    if !status.success() || after.is_none() || after == before {
        return Err("the review was closed without notes".into());
    }
    print!("notes on {}:\n{}saved: {}\n", clip.display(), describe(&read()?, &crate::check::cuts(&clip)), notes_path(&clip).display());
    Ok(())
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
}
