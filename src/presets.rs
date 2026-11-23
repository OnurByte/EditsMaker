//! The presets: the built-in packs' (packs/*/preset.json), the added packs', and your own in ~/.config/edits/presets
//! (a saved preset with the same name replaces the others). data/ holds the files beside them: `moves.json` the cut
//! moves, captions, grades, languages, prompts.
use crate::preset::{Motion, Preset};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// (file name, contents) of every built-in file: the data files, then each pack's preset as FOLDER.json.
fn built_in_files() -> Vec<(String, String)> {
    let data = crate::packs::DATA_FILES.iter().map(|(n, t)| (n.to_string(), t.to_string()));
    // a built-in pack that is turned off brings no preset
    let off = crate::packs::disabled();
    let packs = crate::packs::BUILT_IN_PACKS.iter().filter(|(d, ..)| !off.contains(&format!("edits.{d}")));
    // a family's presets come one by one, each under its pack's folder
    data.chain(packs.flat_map(|(d, _, p, _, _, _, family, ..)| p.iter().map(move |p| (format!("{d}.json"), p.to_string()))
        .chain(family.iter().enumerate().map(move |(i, t)| (format!("{d}-{i}.json"), t.to_string()))))).collect()
}

pub fn user_dir() -> PathBuf {
    crate::util::home().join(".config/edits/presets")
}

/// (file name, contents) of every preset file: built in first, then the added packs', then saved ones over them.
fn files() -> Vec<(String, String)> {
    let mut all = built_in_files();
    for p in crate::packs::all().into_iter().filter(|p| !p.manifest.id.starts_with("edits.")) {
        all.extend(p.all_presets().into_iter().enumerate().map(|(i, t)| (format!("{}/{i}.json", p.manifest.id), t.to_string())));
    }
    let mut saved: Vec<PathBuf> = std::fs::read_dir(user_dir()).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect()).unwrap_or_default();
    saved.sort();
    for p in saved {
        if let Ok(text) = std::fs::read_to_string(&p) {
            let name = p.file_name().unwrap_or_default().to_string_lossy().into_owned();
            all.retain(|(n, _)| *n != name);
            all.push((name, text));
        }
    }
    all
}

/// A short code for a preset as it is now. A change made `--if` that code is refused once the preset has changed
/// since (another agent, another edit), instead of quietly overwriting it.
pub fn revision(p: &Preset) -> String {
    format!("{:08x}", crate::util::crc32(crate::preset::pretty(p).as_bytes()))
}

/// The preset as a pack brings it (built in or added), under your saved one.
fn built_in(name: &str) -> Option<Preset> {
    let added = crate::packs::all().into_iter().filter(|p| !p.manifest.id.starts_with("edits.")).flat_map(|p| p.all_presets().into_iter().map(String::from).collect::<Vec<_>>());
    built_in_files().into_iter().map(|f| f.1).chain(added).filter_map(|t| serde_json::from_str::<Preset>(&t).ok()).rfind(|b| b.name == name)
}

/// The preset named exactly `name` as it is now: the one saved in `dir`, else the built-in one.
fn current(dir: &Path, name: &str) -> Option<Preset> {
    let saved = dir.join(format!("{}.json", crate::preset::slug(name)));
    std::fs::read_to_string(saved).ok().and_then(|t| serde_json::from_str(&t).ok()).or_else(|| built_in(name))
}

/// Earlier versions of a saved preset, oldest first: .history/SLUG/<milliseconds>.json in your preset folder.
fn history(dir: &Path, slug: &str) -> (PathBuf, Vec<PathBuf>) {
    let dir = dir.join(".history").join(slug);
    let mut v: Vec<PathBuf> = std::fs::read_dir(&dir).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "json")).collect()).unwrap_or_default();
    v.sort_by_key(|p| p.file_stem().and_then(|s| s.to_str()?.parse::<u128>().ok()).unwrap_or(0));
    (dir, v)
}

/// Saves `p` as your preset; the version it replaces is kept (the last 20) for `undo`. With `expect`, a revision
/// read earlier, it refuses when the preset has changed since. Returns the file and the new revision.
pub fn save(p: &Preset, expect: Option<&str>) -> Result<(PathBuf, String), String> {
    save_in(&user_dir(), p, expect)
}

fn save_in(dir: &Path, p: &Preset, expect: Option<&str>) -> Result<(PathBuf, String), String> {
    if p.slots.is_empty() {
        return Err("a preset needs at least one shot".into());
    }
    if let (Some(want), Some(now)) = (expect, current(dir, &p.name).map(|c| revision(&c))) {
        if want != now {
            return Err(format!("\"{}\" changed since revision {want} was read (it is {now} now): read it again and redo the change", p.name));
        }
    }
    let slug = crate::preset::slug(&p.name);
    let to = dir.join(format!("{slug}.json"));
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    if to.is_file() {
        let (hist, old) = history(dir, &slug);
        std::fs::create_dir_all(&hist).map_err(|e| e.to_string())?;
        // one more than the newest, so two saves in the same millisecond keep both
        let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
            .max(old.last().and_then(|f| f.file_stem()?.to_str()?.parse::<u128>().ok()).map_or(0, |n| n + 1));
        std::fs::copy(&to, hist.join(format!("{ms}.json"))).map_err(|e| e.to_string())?;
        for f in old.iter().take((old.len() + 1).saturating_sub(20)) {
            let _ = std::fs::remove_file(f);
        }
    }
    std::fs::write(&to, crate::preset::pretty(p)).map_err(|e| e.to_string())?;
    Ok((to, revision(p)))
}

/// Takes your preset back one step: the version it replaced, or the built-in one when your first save was over it.
/// The version undone is not kept.
pub fn undo(name: &str) -> Result<String, String> {
    undo_in(&user_dir(), &crate::preset::load(name)?.name)
}

fn undo_in(dir: &Path, name: &str) -> Result<String, String> {
    let slug = crate::preset::slug(name);
    let to = dir.join(format!("{slug}.json"));
    let (_, old) = history(dir, &slug);
    if let Some(last) = old.last() {
        std::fs::rename(last, &to).map_err(|e| e.to_string())?;
        return Ok(format!("\"{name}\" is back to the version before ({} older one(s) left)", old.len() - 1));
    }
    if to.is_file() && built_in(name).is_some() {
        std::fs::remove_file(&to).map_err(|e| e.to_string())?;
        return Ok(format!("\"{name}\" is back to the built-in one"));
    }
    Err(format!("nothing to undo for \"{name}\""))
}

/// Files beside the presets that are not presets.
const DATA: &[&str] = &["moves.json", "captions.json", "grades.json", "lang.json", "prompts.json"];

pub fn all() -> Vec<Preset> {
    let mut out: Vec<Preset> = vec![];
    for (file, text) in files().into_iter().filter(|(n, _)| !DATA.contains(&n.as_str())) {
        match serde_json::from_str::<Preset>(&text) {
            Ok(p) => {
                out.retain(|o| o.name != p.name);
                out.push(p);
            }
            Err(e) => eprintln!("edits: preset {file} is not valid: {e}"),
        }
    }
    out.sort_by_key(|p| p.name.to_lowercase());
    out
}

/// A data file by name (saved one first, else built in): moves.json, captions.json.
pub fn file(name: &str) -> Option<String> {
    files().into_iter().rfind(|(n, _)| n == name).map(|x| x.1)
}

/// The cut moves, read once.
pub fn motions() -> &'static [Motion] {
    static MOVES: OnceLock<Vec<Motion>> = OnceLock::new();
    MOVES.get_or_init(|| {
        files().into_iter().rfind(|(n, _)| n == "moves.json")
            .and_then(|(_, t)| serde_json::from_str(&t).map_err(|e| eprintln!("edits: moves.json is not valid: {e}")).ok())
            .unwrap_or_default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_keep_history_undo_walks_back_and_stale_changes_are_refused() {
        let dir = std::env::temp_dir().join(format!("edits-undo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        // a built-in preset, changed twice
        let base = built_in(&all_built_in_name()).unwrap();
        let first = revision(&base);
        let mut a = base.clone();
        a.total += 1.0;
        let (_, ra) = save_in(&dir, &a, Some(&first)).unwrap();
        let mut b = a.clone();
        b.total += 1.0;
        // someone still holding the built-in revision is refused: the preset moved on
        assert!(save_in(&dir, &b, Some(&first)).unwrap_err().contains(&ra));
        save_in(&dir, &b, Some(&ra)).unwrap();
        assert_eq!(current(&dir, &b.name).unwrap().total, b.total);
        // undo: back to a, then back to the built-in, then nothing left
        undo_in(&dir, &b.name).unwrap();
        assert_eq!(current(&dir, &b.name).unwrap().total, a.total);
        assert!(undo_in(&dir, &b.name).unwrap().contains("built-in"));
        assert_eq!(revision(&current(&dir, &b.name).unwrap()), first);
        assert!(undo_in(&dir, &b.name).is_err());
        let mut empty = base.clone();
        empty.slots.clear();
        assert!(save_in(&dir, &empty, None).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn all_built_in_name() -> String {
        built_in_files().iter().find_map(|(_, t)| serde_json::from_str::<Preset>(t).ok()).unwrap().name
    }

    #[test]
    fn built_in_presets_read() {
        // every shipped file parses
        for (name, text) in &super::built_in_files() {
            let text = text.as_str();
            if name == "captions.json" {
                serde_json::from_str::<Vec<crate::talk::CaptionStyle>>(text).unwrap();
            } else if name == "lang.json" || name == "prompts.json" {
                serde_json::from_str::<serde_json::Value>(text).unwrap();
            } else if name == "grades.json" {
                serde_json::from_str::<Vec<crate::grades::Grade>>(text).unwrap();
            } else if name == "moves.json" {
                serde_json::from_str::<Vec<crate::preset::Motion>>(text).unwrap();
            } else {
                serde_json::from_str::<crate::preset::Preset>(text).unwrap_or_else(|e| panic!("{name}: {e}"));
            }
        }
    }

    /// The shipped library stays sound: the errors found in it once (zooms near 1000, keys stacked or out of order,
    /// 1.5-frame shots learned from flash frames, a look that does not exist) never come back.
    #[test]
    fn built_in_presets_are_sound() {
        let looks = crate::looks::names();
        for (name, text) in super::built_in_files().iter().filter(|(n, _)| !super::DATA.contains(&n.as_str())) {
            let p: crate::preset::Preset = serde_json::from_str(text).unwrap();
            assert!(!p.slots.is_empty() && p.total > 0.0, "{name}: empty");
            assert!(p.look.as_ref().is_none_or(|l| looks.contains(&l.as_str())), "{name}: unknown look {:?}", p.look);
            for (i, s) in p.slots.iter().enumerate() {
                assert!(s.end - s.start >= 0.066, "{name} shot {i}: {:.3}s is a flicker, not a shot", s.end - s.start);
                assert!((0.1..=8.0).contains(&s.speed), "{name} shot {i}: speed {}", s.speed);
                for (k, c) in [("zoom", &s.zoom), ("x", &s.x), ("y", &s.y), ("rot", &s.rot), ("shake", &s.shake), ("blur", &s.blur), ("exposure", &s.exposure), ("ramp", &s.ramp)] {
                    assert!(c.keys.windows(2).all(|w| w[0].0 <= w[1].0) && c.keys.iter().all(|k| (0.0..=1.0).contains(&k.0)), "{name} shot {i}: {k} keys out of order or outside the shot");
                }
                assert!(s.zoom.keys.iter().all(|k| (0.04..=5.0).contains(&k.1)), "{name} shot {i}: zoom {:?}", s.zoom.keys.iter().map(|k| k.1).collect::<Vec<_>>());
            }
        }
    }
}
