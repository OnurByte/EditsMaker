//! The presets: JSON files built into the binary from presets/, and your own in ~/.config/edits/presets
//! (a saved preset with the same name replaces the built-in one). `moves.json` holds the cut moves.
use crate::preset::{Motion, Preset};
use std::path::PathBuf;
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/built_in.rs"));

pub fn user_dir() -> PathBuf {
    crate::util::home().join(".config/edits/presets")
}

/// (file name, contents) of every preset file: built in first, then saved ones over them.
fn files() -> Vec<(String, String)> {
    let mut all: Vec<(String, String)> = BUILT_IN.iter().map(|(n, t)| (n.to_string(), t.to_string())).collect();
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
    #[test]
    fn built_in_presets_read() {
        // every shipped file parses
        for (name, text) in super::BUILT_IN {
            if *name == "captions.json" {
                serde_json::from_str::<Vec<crate::talk::CaptionStyle>>(text).unwrap();
            } else if *name == "lang.json" || *name == "prompts.json" {
                serde_json::from_str::<serde_json::Value>(text).unwrap();
            } else if *name == "grades.json" {
                serde_json::from_str::<Vec<crate::grades::Grade>>(text).unwrap();
            } else if *name == "moves.json" {
                serde_json::from_str::<Vec<crate::preset::Motion>>(text).unwrap();
            } else {
                serde_json::from_str::<crate::preset::Preset>(text).unwrap_or_else(|e| panic!("{name}: {e}"));
            }
        }
    }
}
