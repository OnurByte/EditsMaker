//! Everything that depends on the spoken language (laughter words, words that carry feeling, how capitals work,
//! what to tell the transcriber) and the words the models are prompted with live in presets/lang.json and
//! presets/prompts.json, so a language is added or a prompt reworded without touching the code.
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::OnceLock;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Lang {
    /// in comments: someone laughing
    #[serde(default)]
    pub laugh: Vec<String>,
    /// in caption tags like [laughter]: laughing or clapping in the video
    #[serde(default)]
    pub sound_tags: Vec<String>,
    /// words that make a sentence worth putting on screen
    #[serde(default)]
    pub feeling: Vec<String>,
    /// tells the transcriber to keep swearing as it is
    #[serde(default)]
    pub whisper_prompt: String,
    /// i has a dot in capitals (İ) and ı has none (I), as in Turkish and Azerbaijani
    #[serde(default)]
    pub dotted_i: bool,
}

static CODE: OnceLock<String> = OnceLock::new();

/// The language for this run, when it differs from the system's (from --lang).
pub fn set(code: &str) {
    let _ = CODE.set(code.to_lowercase());
}

/// The language and country to work in: EDITS_LANG (tr, or tr_TR) when set, else the system's from LC_ALL / LANG
/// (tr_TR.UTF-8: tr, tr), else en, us.
pub fn system() -> (String, String) {
    let v = ["EDITS_LANG", "LC_ALL", "LC_MESSAGES", "LANG"].iter().filter_map(|k| std::env::var(k).ok()).find(|v| !v.is_empty() && v != "C" && v != "POSIX").unwrap_or_default();
    let base = v.split('.').next().unwrap_or("");
    let mut it = base.split(['_', '-']);
    let lang = it.next().filter(|l| l.len() == 2).unwrap_or("en").to_lowercase();
    let country = it.next().filter(|c| c.len() == 2).unwrap_or(if lang == "en" { "us" } else { &lang }).to_lowercase();
    (lang, country)
}

pub fn code() -> &'static str {
    // the tests' sample sentences are Turkish, so they run in Turkish whatever the machine speaks
    CODE.get_or_init(|| if cfg!(test) { "tr".into() } else { system().0 })
}

fn table() -> BTreeMap<String, Lang> {
    crate::presets::file("lang.json").and_then(|t| serde_json::from_str(&t).map_err(|e| eprintln!("edits: lang.json is not valid: {e}")).ok()).unwrap_or_default()
}

/// The language's words, with the ones every language shares.
pub fn get() -> &'static Lang {
    static L: OnceLock<Lang> = OnceLock::new();
    L.get_or_init(|| words_for(&table(), code()))
}

fn words_for(t: &BTreeMap<String, Lang>, code: &str) -> Lang {
    let any = t.get("any").cloned().unwrap_or_default();
    let mut l = t.get(code).cloned().unwrap_or_default();
    l.laugh.extend(any.laugh);
    l.sound_tags.extend(any.sound_tags);
    l
}

/// Capitals the way the language writes them.
pub fn upper(s: &str) -> String {
    upper_with(get(), s)
}

fn upper_with(l: &Lang, s: &str) -> String {
    if l.dotted_i { s.replace('i', "İ").to_uppercase() } else { s.to_uppercase() }
}

/// A piece of a prompt (`group.part` in prompts.json) with its {placeholders} filled in.
pub fn prompt(group: &str, part: &str, vars: &[(&str, String)]) -> String {
    static P: OnceLock<BTreeMap<String, BTreeMap<String, String>>> = OnceLock::new();
    let all = P.get_or_init(|| crate::presets::file("prompts.json").and_then(|t| serde_json::from_str(&t).map_err(|e| eprintln!("edits: prompts.json is not valid: {e}")).ok()).unwrap_or_default());
    let mut s = all.get(group).and_then(|g| g.get(part)).cloned().unwrap_or_default();
    for (k, v) in vars {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn languages_come_from_the_file() {
        let t: BTreeMap<String, Lang> = serde_json::from_str(crate::presets::file("lang.json").unwrap().as_str()).unwrap();
        let tr = words_for(&t, "tr");
        assert!(tr.dotted_i && tr.laugh.iter().any(|w| w == "😂") && tr.laugh.iter().any(|w| w == "koptum"));
        let en = words_for(&t, "en");
        assert!(!en.dotted_i && !en.laugh.iter().any(|w| w == "koptum"));
        assert_eq!((upper_with(&tr, "hi ılık"), upper_with(&en, "hi")), ("Hİ ILIK".to_string(), "HI".to_string()));
        // a language nobody wrote up still gets the shared words
        assert!(words_for(&t, "xx").laugh.contains(&"lol".to_string()));
        assert!(prompt("lines", "tail", &[("n", "3".into())]).contains("the 3 numbers"));
    }
}
