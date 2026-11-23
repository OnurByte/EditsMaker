//! Packs: a preset, an agent skill, an automation, RSS feeds or a source program, in one folder with a manifest.json
//! and a cover picture (PACKS.md is the standard; the manifest follows Omarchy's plugin manifests). The packs in packs/ are built in (ids `edits.*`); community packs live in their
//! own GitHub repositories, are listed in packs.json at the root of this repository (the marketplace), and are added
//! into ~/.config/edits/packs/<id>, each in its own folder.
use crate::preset::Preset;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/built_in.rs"));

/// The files a pack is made of, beside its programs and its cover.
const PACK_FILES: &[&str] = &["manifest.json", "preset.json", "SKILL.md", "automation.js", "scene.js"];

/// The community list `edits packs browse` reads: a pull request adding a line to packs.json is how a pack is listed.
const REGISTRY: &str = "https://raw.githubusercontent.com/OnurByte/EditsMaker/main/packs.json";
const PREVIEWS: &[&str] = &["preview.svg", "preview.png", "preview.jpg", "preview.jpeg", "preview.webp", "preview.gif"];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    pub schema_version: u32,
    /// unique: lowercase letters, digits, dots and dashes (io.github.you.my-pack); `edits.` is the built-in ones'
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    /// what is in it: "preset" (preset.json), "skill" (SKILL.md, in every pack), "automation" (automation.js),
    /// "feed" (`feeds` below)
    pub kinds: Vec<String>,
    /// the picture beside the manifest: preview.svg, .png, .jpg or .webp
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// for an automation: the one site its automation.js runs on
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub automation: Option<Automation>,
    /// for a feed pack: the RSS or Atom feeds `edits rss` and `edits follow` read, first one by default
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feeds: Vec<Feed>,
    /// for an automation on more than one site: each site's block, its script in sites/SITE.js
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sites: Vec<Automation>,
    /// for a feed pack of many topics: each by its short name (`edits rss football`), with its own feeds
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<Topic>,
    /// for a 3D scene (scene.js, three.js): how long it plays and how smoothly, laid over an edit as `--overlay ID`
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<Scene>,
    /// for a source: the program edits runs, on its own, for new things to make (source.rs)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// what the person can set, as Omarchy's plugin schema: `edits packs set ID KEY=VALUE`
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings: Vec<Setting>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    /// the program beside the manifest, run with sh (a .sh or any script with its #! line) or as itself
    pub run: String,
    /// Windows' own (a .ps1 runs with PowerShell); without it the source is not run on Windows
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows: Option<String>,
    /// whether it may reach the internet; nothing else is ever reachable
    #[serde(default)]
    pub network: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub default: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Topic {
    pub key: String,
    pub name: String,
    pub feeds: Vec<Feed>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    pub seconds: f64,
    #[serde(default = "fps")]
    pub fps: u32,
}

fn fps() -> u32 {
    30
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Feed {
    pub name: String,
    pub url: String,
}

/// Where an automation runs. Its script runs only inside `page`, a page of `domain`, signed in with the person's own
/// session in a browser profile of its own; it gets nothing from edits but the clip and what it is asked to do.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Automation {
    /// what `edits connect SITE` and `edits post --to` call it
    pub site: String,
    /// the sign-in page, and the page the script runs in
    pub login: String,
    pub page: String,
    /// the cookie that says someone is signed in, and the site it belongs to
    pub cookie: String,
    pub domain: String,
}

pub struct Pack {
    pub manifest: Manifest,
    pub preset: Option<String>,
    pub skill: Option<String>,
    pub automation: Option<String>,
    /// a family's presets (presets/*.json), beside or instead of preset.json
    pub presets: Vec<String>,
    /// a 3D scene's script (scene.js)
    pub scene: Option<String>,
    /// more agent skills (skills/*.md), each with its own name
    pub skills: Vec<String>,
    /// more sites' scripts (sites/SITE.js), by site
    pub site_js: Vec<(String, String)>,
}

impl Pack {
    /// Every preset it carries, as text: preset.json, then the family's.
    pub fn all_presets(&self) -> Vec<&str> {
        self.preset.iter().chain(&self.presets).map(String::as_str).collect()
    }

    /// Every site it automates, with its script: its automation.js, then sites/SITE.js.
    pub fn automations(&self) -> Vec<(Automation, String)> {
        let first = self.manifest.automation.clone().zip(self.automation.clone());
        first.into_iter().chain(self.manifest.sites.iter().filter_map(|a| Some((a.clone(), self.site_js.iter().find(|x| x.0 == a.site)?.1.clone())))).collect()
    }

    /// Every skill it carries, (name, SKILL.md): its SKILL.md, then skills/*.md.
    pub fn all_skills(&self) -> Vec<(String, String)> {
        self.skill.iter().chain(&self.skills).filter_map(|t| Some((skill_name(t)?.to_string(), t.clone()))).collect()
    }

}

/// A field of SKILL.md's front matter.
fn front<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    text.lines().take_while(|l| !l.is_empty()).find_map(|l| l.strip_prefix(key)?.strip_prefix(": ")).map(str::trim)
}

fn skill_name(text: &str) -> Option<&str> {
    front(text, "name").filter(|n| valid_id(n))
}

/// The packs folder community packs are added into.
pub fn dir() -> PathBuf {
    crate::util::home().join(".config/edits/packs")
}

fn valid_id(id: &str) -> bool {
    (1..=100).contains(&id.len()) && id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-' || c == '_')
        && !id.starts_with('.') && !id.contains("..")
}

/// The site an https page is on: the last two parts of its host (studio.youtube.com: youtube.com).
fn site_of(url: &str) -> Option<String> {
    let host = url.strip_prefix("https://")?.split(['/', '?', '#', ':']).next()?.to_lowercase();
    let parts: Vec<&str> = host.split('.').collect();
    (parts.len() >= 2 && parts.iter().all(|p| !p.is_empty())).then(|| parts[parts.len() - 2..].join("."))
}

/// Whether a pack holds what its manifest says, and nothing claims to be built in that is not.
fn check(m: &Manifest, preset: Option<&str>, skill: Option<&str>, automation: Option<&str>, built_in: bool) -> Result<(), String> {
    let id = &m.id;
    if m.schema_version != 1 {
        return Err(format!("{id}: schemaVersion {} is not known (1 is)", m.schema_version));
    }
    if !valid_id(id) {
        return Err(format!("\"{id}\": an id is lowercase letters, digits, dots and dashes (io.github.you.my-pack)"));
    }
    if built_in != id.starts_with("edits.") {
        return Err(format!("{id}: ids starting with edits. are the built-in packs'"));
    }
    for (k, v) in [("name", &m.name), ("version", &m.version), ("author", &m.author), ("description", &m.description)] {
        if v.trim().is_empty() {
            return Err(format!("{id}: manifest.json needs a {k}"));
        }
    }
    for (kind, file, there) in [("preset", "preset.json", preset.is_some()), ("skill", "SKILL.md", skill.is_some()), ("automation", "automation.js", automation.is_some())] {
        if m.kinds.iter().any(|k| k == kind) != there {
            return Err(format!("{id}: kinds says {:?} but {file} is {}", m.kinds, if there { "there" } else { "missing" }));
        }
    }
    if m.kinds.is_empty() || m.kinds.iter().any(|k| !["preset", "skill", "automation", "feed", "source", "scene"].contains(&k.as_str())) {
        return Err(format!("{id}: kinds is some of \"preset\", \"skill\", \"automation\", \"feed\", \"source\", \"scene\", not {:?}", m.kinds));
    }
    if m.kinds.iter().any(|k| k == "source") != m.source.is_some() {
        return Err(format!("{id}: kinds and the \"source\" block disagree"));
    }
    // a program and its settings stay inside the pack's folder: plain names, nothing like ../
    let plain = |f: &str| !f.is_empty() && f.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) && !f.starts_with('.');
    if let Some(src) = &m.source {
        if !std::iter::once(&src.run).chain(src.windows.as_ref()).all(|f| plain(f) && !PACK_FILES.contains(&f.as_str())) {
            return Err(format!("{id}: a source's program is a plain file name beside the manifest (source.sh)"));
        }
    }
    if m.settings.iter().any(|s| !valid_id(&s.key) || s.label.trim().is_empty()) {
        return Err(format!("{id}: each setting has a key (lowercase) and a label"));
    }
    if skill.is_none() {
        return Err(format!("{id}: every pack carries a SKILL.md (edits packs make drafts one)"));
    }
    if m.kinds.iter().any(|k| k == "feed") != (!m.feeds.is_empty() || !m.topics.is_empty()) {
        return Err(format!("{id}: kinds says {:?} but it has {} feeds", m.kinds, m.feeds.len()));
    }
    if m.topics.iter().any(|t| !valid_id(&t.key) || t.name.trim().is_empty() || t.feeds.is_empty()) {
        return Err(format!("{id}: each topic has a key (lowercase), a name and feeds"));
    }
    // a feed is only read, so plain http is fine too
    if let Some(f) = m.feeds.iter().chain(m.topics.iter().flat_map(|t| &t.feeds)).find(|f| f.name.trim().is_empty() || site_of(&f.url.replacen("http://", "https://", 1)).is_none()) {
        return Err(format!("{id}: feed \"{}\" needs a name and an http(s):// address", f.name));
    }
    if let Some(js) = automation {
        check_site(id, m.automation.as_ref().ok_or(format!("{id}: an automation needs an \"automation\" block in manifest.json (PACKS.md)"))?, js)?;
    }
    if let Some(p) = preset {
        let p: Preset = serde_json::from_str(p).map_err(|e| format!("{id}: preset.json is not a preset: {e}"))?;
        if p.slots.is_empty() {
            return Err(format!("{id}: preset.json has no shots"));
        }
    }
    if skill.is_some_and(|s| skill_name(s).is_none()) {
        return Err(format!("{id}: SKILL.md needs front matter with a name: line"));
    }
    if m.preview.as_ref().is_some_and(|p| !PREVIEWS.contains(&p.as_str())) {
        return Err(format!("{id}: preview is one of {}", PREVIEWS.join(", ")));
    }
    Ok(())
}

/// An automation's site and its script: one site's pages only, one function.
fn check_site(id: &str, a: &Automation, js: &str) -> Result<(), String> {
    if !valid_id(&a.site) || site_of(&a.page).is_none() || site_of(&a.page) != site_of(&a.login) || a.cookie.trim().is_empty() || a.domain.trim().is_empty() {
        return Err(format!("{id}: automation needs a site name, a cookie and its domain, and its login and page as https pages of one site (PACKS.md)"));
    }
    let code: String = js.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
    if !(code.trim_start().starts_with("async (p) => {") && code.trim_end().ends_with('}')) {
        return Err(format!("{id}: an automation's script is one function, async (p) => {{ ... }}"));
    }
    Ok(())
}

/// A whole pack checked: its files as the manifest says, a family's presets and a scene too.
fn check_pack(p: &Pack, built_in: bool) -> Result<(), String> {
    let m = &p.manifest;
    // a pack of sites stands in its first site for the one automation check() knows
    let (mut m1, mut js) = (m.clone(), p.automation.clone());
    if m1.automation.is_none() {
        if let Some((a, j)) = p.automations().into_iter().next() {
            (m1.automation, js) = (Some(a), Some(j));
        }
    }
    check(&m1, p.preset.as_deref().or(p.presets.first().map(String::as_str)), p.skill.as_deref(), js.as_deref(), built_in)?;
    for a in &m.sites {
        let j = p.site_js.iter().find(|x| x.0 == a.site).ok_or(format!("{}: site {} has no sites/{}.js", m.id, a.site, a.site))?;
        check_site(&m.id, a, &j.1)?;
    }
    for (i, t) in p.presets.iter().enumerate() {
        let x: Preset = serde_json::from_str(t).map_err(|e| format!("{}: presets/ file {} is not a preset: {e}", m.id, i + 1))?;
        if x.slots.is_empty() {
            return Err(format!("{}: preset {} has no shots", m.id, x.name));
        }
    }
    if m.kinds.iter().any(|k| k == "scene") != (p.scene.is_some() && m.scene.is_some()) {
        return Err(format!("{}: a scene needs \"scene\" in kinds, a \"scene\" block ({{\"seconds\": 4}}) and scene.js", m.id));
    }
    if let Some(i) = p.skills.iter().position(|t| skill_name(t).is_none()) {
        return Err(format!("{}: skills/ file {} needs front matter with a name: line", m.id, i + 1));
    }
    if m.scene.as_ref().is_some_and(|x| !(0.5..=30.0).contains(&x.seconds) || !(1..=60).contains(&x.fps)) {
        return Err(format!("{}: a scene lasts 0.5 to 30 seconds, at 1 to 60 fps", m.id));
    }
    Ok(())
}

/// A built-in pack's moving cover (preview.gif), when it has one.
#[cfg(any(feature = "app", test))]
pub fn built_in_gif(id: &str) -> Option<&'static [u8]> {
    BUILT_IN_PACKS.iter().find(|b| id.strip_prefix("edits.") == Some(b.0)).and_then(|b| b.9)
}

pub fn built_in() -> Vec<Pack> {
    BUILT_IN_PACKS.iter().filter_map(|(d, m, p, s, a, sc, family, skills, sites, _)| {
        let manifest: Manifest = serde_json::from_str(m).map_err(|e| eprintln!("edits: pack {d}: {e}")).ok()?;
        Some(Pack { manifest, preset: p.map(String::from), skill: s.map(String::from), automation: a.map(String::from), scene: sc.map(String::from), presets: family.iter().map(|t| t.to_string()).collect(), skills: skills.iter().map(|t| t.to_string()).collect(), site_js: sites.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect() })
    }).collect()
}

/// A family's presets in a pack folder, by name.
fn family(d: &Path) -> Vec<PathBuf> {
    files_in(&d.join("presets"), "json")
}

/// sites/SITE.js in a pack folder, as (site, script).
fn site_scripts(d: &Path) -> Vec<(String, String)> {
    files_in(&d.join("sites"), "js").iter().filter_map(|f| Some((f.file_stem()?.to_string_lossy().into_owned(), std::fs::read_to_string(f).ok()?))).collect()
}

/// A folder's plain files with one extension, by name.
fn files_in(d: &Path, ext: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(d).map(|r| r.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == ext) && p.is_file()).collect()).unwrap_or_default();
    v.sort();
    v
}

/// The files of a pack that sit in its own folders: presets/*.json and skills/*.md, as names in the pack.
fn nested(d: &Path) -> Vec<String> {
    [("presets", "json"), ("skills", "md"), ("sites", "js")].iter().flat_map(|(sub, ext)| files_in(&d.join(sub), ext).into_iter()
        .filter_map(move |f| Some(format!("{sub}/{}", f.file_name()?.to_string_lossy())))).collect()
}

/// A pack folder as it is on disk, checked.
pub fn read(d: &Path) -> Result<Pack, String> {
    let text = |f: &str| std::fs::read_to_string(d.join(f)).ok();
    let manifest: Manifest = serde_json::from_str(&text("manifest.json").ok_or(format!("{}: no manifest.json", d.display()))?)
        .map_err(|e| format!("{}/manifest.json: {e}", d.display()))?;
    let (preset, skill, automation, scene) = (text("preset.json"), text("SKILL.md"), text("automation.js"), text("scene.js"));
    let presets = family(d).iter().filter_map(|f| std::fs::read_to_string(f).ok()).collect();
    let skills = files_in(&d.join("skills"), "md").iter().filter_map(|f| std::fs::read_to_string(f).ok()).collect();
    let site_js = site_scripts(d);
    let p = Pack { manifest, preset, skill, automation, presets, scene, skills, site_js };
    check_pack(&p, false)?;
    Ok(p)
}

fn trust_file(id: &str) -> PathBuf {
    dir().join(id).join(".trusted")
}

/// The files of a pack that run: its automation.js and its source's programs.
pub fn programs(m: &Manifest) -> Vec<String> {
    let mut v: Vec<String> = m.automation.as_ref().map(|_| "automation.js".to_string()).into_iter().collect();
    // a scene's script runs too, in a browser of its own, and each site's
    v.extend(m.scene.as_ref().map(|_| "scene.js".to_string()));
    v.extend(m.sites.iter().map(|a| format!("sites/{}.js", a.site)));
    if let Some(s) = &m.source {
        v.push(s.run.clone());
        v.extend(s.windows.clone());
    }
    v
}

/// What the person read: the checksums of the pack's programs, one line each.
fn fingerprint(id: &str, m: &Manifest) -> Result<String, String> {
    programs(m).iter().map(|f| crate::update::sha256(&dir().join(id).join(f))).collect::<Result<Vec<_>, _>>().map(|v| v.join("\n"))
}

/// Whether a pack's programs may run: built-in ones may; an added one only once the person read them and ran
/// `edits packs trust ID`, and only while they are the ones they read.
pub fn trusted(p: &Pack) -> bool {
    let id = &p.manifest.id;
    id.starts_with("edits.") || programs(&p.manifest).is_empty()
        || fingerprint(id, &p.manifest).is_ok_and(|h| std::fs::read_to_string(trust_file(id)).is_ok_and(|t| t.trim() == h.trim()))
}

/// Lets an added pack's programs run, as they are now.
pub fn trust(id: &str) -> Result<PathBuf, String> {
    let p = read(&dir().join(id)).map_err(|_| format!("\"{id}\" is not an added pack (edits packs lists them)"))?;
    let files = programs(&p.manifest);
    let first = files.first().ok_or(format!("{id} runs nothing, so there is nothing to trust"))?;
    std::fs::write(trust_file(id), fingerprint(id, &p.manifest)?).map_err(|e| e.to_string())?;
    Ok(dir().join(id).join(first))
}

fn settings_file(id: &str) -> PathBuf {
    crate::util::home().join(".config/edits/pack-settings").join(format!("{id}.json"))
}

/// A pack's settings: its defaults, with what the person set over them.
pub fn settings(m: &Manifest) -> serde_json::Map<String, serde_json::Value> {
    let set: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(settings_file(&m.id)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    m.settings.iter().map(|s| (s.key.clone(), set.get(&s.key).cloned().unwrap_or_else(|| serde_json::Value::String(s.default.clone())))).collect()
}

/// `edits packs set ID KEY=VALUE...`
pub fn set(id: &str, pairs: &[String]) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let p = all().into_iter().find(|p| p.manifest.id == id).ok_or(format!("no pack \"{id}\" (edits packs lists them)"))?;
    let f = settings_file(id);
    let mut set: serde_json::Map<String, serde_json::Value> = std::fs::read_to_string(&f).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    for kv in pairs {
        let (k, v) = kv.split_once('=').ok_or(format!("{kv}: use KEY=VALUE"))?;
        if !p.manifest.settings.iter().any(|s| s.key == k) {
            return Err(format!("{id} has no setting {k}; it has: {}", p.manifest.settings.iter().map(|s| s.key.as_str()).collect::<Vec<_>>().join(", ")));
        }
        set.insert(k.to_string(), serde_json::Value::String(v.to_string()));
    }
    std::fs::create_dir_all(f.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
    std::fs::write(&f, serde_json::to_string_pretty(&set).unwrap_or_default()).map_err(|e| e.to_string())?;
    Ok(settings(&p.manifest))
}

/// The community packs added here. A broken one is said and skipped.
pub fn added() -> Vec<Pack> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir()).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    dirs.sort();
    dirs.iter().filter_map(|d| read(d).map_err(|e| eprintln!("edits: {e}")).ok()).collect()
}

/// Every pack here, on or off.
pub fn every() -> Vec<Pack> {
    built_in().into_iter().chain(added()).collect()
}

/// The packs that are on: what presets, skills, feeds and automations come from.
/// The marketplace's tags: a few words a person would look for, not how a preset moves. A pack carries at most three.
pub const TAGS: [&str; 10] = ["hype", "sad", "anime", "cinematic", "glitch", "retro", "meme", "news", "3d", "automation"];

/// A pack's tags as the marketplace shows them: each word mapped onto TAGS (a preset's "velocity" is hype, "rgb"
/// glitch), the rest dropped, the first three kept.
pub fn tidy_tags<S: AsRef<str>>(raw: &[S]) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for t in raw {
        let t = t.as_ref().trim().to_lowercase();
        let m = match t.as_str() {
            "fast" | "velocity" | "hype" | "phonk" | "drop" => "hype",
            "sad" | "fade" | "dreamy" | "emotional" | "chill" => "sad",
            "anime" => "anime",
            "cinema" | "cinematic" | "letterbox" | "film" => "cinematic",
            "glitch" | "rgb" | "strobe" => "glitch",
            "retro" | "vhs" | "tape" => "retro",
            "comic" | "split" | "meme" | "funny" => "meme",
            "news" | "rss" | "feed" => "news",
            "3d" | "scene" | "overlay" => "3d",
            "automation" | "post" | "agent" | "mcp" | "skills" | "tiktok" | "instagram" | "youtube" => "automation",
            _ => continue,
        };
        debug_assert!(TAGS.contains(&m));
        if !out.iter().any(|x| x == m) {
            out.push(m.into());
        }
    }
    out.truncate(3);
    out
}

pub fn all() -> Vec<Pack> {
    let off = disabled();
    every().into_iter().filter(|p| !off.contains(&p.manifest.id)).collect()
}

/// Where a pack came from: built into edits, added from the community, or made here.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Origin {
    BuiltIn,
    Community,
    Local,
}

impl Origin {
    pub fn name(self) -> &'static str {
        match self {
            Origin::BuiltIn => "built-in",
            Origin::Community => "community",
            Origin::Local => "local",
        }
    }
}

/// An added pack remembers its repository in .from; one without it was made here.
pub fn origin(id: &str) -> Origin {
    if id.starts_with("edits.") {
        Origin::BuiltIn
    } else if dir().join(id).join(".from").is_file() {
        Origin::Community
    } else {
        Origin::Local
    }
}

fn disabled_file() -> PathBuf {
    crate::util::home().join(".config/edits/packs-off.json")
}

/// The packs turned off.
pub fn disabled() -> Vec<String> {
    std::fs::read_to_string(disabled_file()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

/// Turns a pack on or off; any pack, built in too. An off pack's preset, skill, feeds and automation are not used.
pub fn enable(id: &str, on: bool) -> Result<Pack, String> {
    let p = every().into_iter().find(|p| p.manifest.id == id).ok_or(format!("no pack \"{id}\" (edits packs lists them)"))?;
    let mut off = disabled();
    off.retain(|x| x != id);
    if !on {
        off.push(id.to_string());
    }
    let f = disabled_file();
    std::fs::create_dir_all(f.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
    std::fs::write(&f, serde_json::to_string_pretty(&off).unwrap_or_default()).map_err(|e| e.to_string())?;
    Ok(p)
}

/// A pack's own files, as (name in the pack, contents): what export packs up and add copies. A built-in pack's come
/// from inside edits.
fn own_files(id: &str) -> Result<Vec<(String, Vec<u8>)>, String> {
    if origin(id) == Origin::BuiltIn {
        let p = built_in().into_iter().find(|p| p.manifest.id == id).ok_or(format!("no pack \"{id}\" (edits packs lists them)"))?;
        let mut v = vec![("manifest.json".to_string(), (serde_json::to_string_pretty(&p.manifest).map_err(|e| e.to_string())? + "\n").into_bytes())];
        for (f, t) in [("preset.json", &p.preset), ("SKILL.md", &p.skill), ("automation.js", &p.automation), ("scene.js", &p.scene)] {
            v.extend(t.as_ref().map(|t| (f.to_string(), t.clone().into_bytes())));
        }
        for t in &p.presets {
            let name = serde_json::from_str::<Preset>(t).map(|x| crate::preset::slug(&x.name)).unwrap_or_else(|_| "preset".into());
            v.push((format!("presets/{name}.json"), t.clone().into_bytes()));
        }
        for t in &p.skills {
            v.push((format!("skills/{}.md", skill_name(t).unwrap_or("skill")), t.clone().into_bytes()));
        }
        for (site, js) in &p.site_js {
            v.push((format!("sites/{site}.js"), js.clone().into_bytes()));
        }
        if let Some(gif) = BUILT_IN_PACKS.iter().find(|b| id.strip_prefix("edits.") == Some(b.0)).and_then(|b| b.9) {
            v.push(("preview.gif".into(), gif.to_vec()));
        }
        v.push(("preview.svg".into(), preview(&p.manifest, p.all_presets().first().and_then(|t| serde_json::from_str(t).ok()).as_ref()).into_bytes()));
        return Ok(v);
    }
    let d = dir().join(id);
    let p = read(&d)?;
    let mut names: Vec<String> = PACK_FILES.iter().map(|f| f.to_string()).chain(programs(&p.manifest)).chain(p.manifest.preview.clone()).collect();
    names.extend(nested(&d));
    names.dedup();
    Ok(names.into_iter().filter_map(|f| Some((f.clone(), std::fs::read(d.join(&f)).ok()?))).collect())
}

/// Any pack packed up as ID.tgz (its own files in a folder named after it), at `to` or in edits' share folder.
pub fn export(id: &str, to: Option<&Path>) -> Result<PathBuf, String> {
    if !valid_id(id) {
        return Err(format!("\"{id}\" is not a pack here (edits packs lists them)"));
    }
    let files = own_files(id)?;
    let tmp = std::env::temp_dir().join(format!("edits-export-{}-{id}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    let res = (|| {
        for (f, bytes) in &files {
            let p = tmp.join(id).join(f);
            std::fs::create_dir_all(p.parent().unwrap_or(&tmp)).map_err(|e| e.to_string())?;
            std::fs::write(&p, bytes).map_err(|e| e.to_string())?;
        }
        let out = to.map(PathBuf::from).unwrap_or_else(|| crate::util::home().join(".cache/edits/share").join(format!("{id}.tgz")));
        std::fs::create_dir_all(out.parent().unwrap_or(Path::new("."))).map_err(|e| e.to_string())?;
        crate::util::run(None, "tar", &["-czf", &out.to_string_lossy(), "-C", &tmp.to_string_lossy(), id])?;
        Ok(out)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    res
}

/// A pack made here, packed up to share: its .tgz, and the GitHub issue page to attach it to for the marketplace.
pub fn share(id: &str) -> Result<(PathBuf, String), String> {
    if origin(id) == Origin::BuiltIn {
        return Err(format!("{id} is built in already"));
    }
    let tgz = export(id, None)?;
    let m = read(&dir().join(id))?.manifest;
    let body = format!("A pack for the marketplace. The .tgz is attached below (drag it in).\n\n- id: `{}`\n- name: {}\n- what it is: {}\n- kinds: {}\n- tags: {}\n",
        m.id, m.name, m.description, m.kinds.join(", "), m.tags.join(", "));
    let url = format!("https://github.com/OnurByte/EditsMaker/issues/new?labels=pack&title={}&body={}", encode(&format!("Pack: {}", m.name)), encode(&body));
    Ok((tgz, url))
}

/// Percent-encoding for a URL's query.
fn encode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// A line of packs.json.
#[derive(Debug, Deserialize)]
pub struct Entry {
    pub id: String,
    /// https://github.com/OWNER/REPO, manifest.json at its root
    pub repo: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub kinds: Vec<String>,
}

#[derive(Deserialize)]
struct Registry {
    packs: Vec<Entry>,
}

fn parse_registry(text: &str) -> Result<Vec<Entry>, String> {
    let r: Registry = serde_json::from_str(text).map_err(|e| format!("packs.json: {e}"))?;
    let mut seen = vec![];
    for e in &r.packs {
        if !valid_id(&e.id) || e.id.starts_with("edits.") || seen.contains(&&e.id) {
            return Err(format!("packs.json: \"{}\" is not a usable id (or is there twice)", e.id));
        }
        github(&e.repo)?;
        seen.push(&e.id);
    }
    Ok(r.packs)
}

/// The community packs listed in packs.json.
pub fn registry() -> Result<Vec<Entry>, String> {
    parse_registry(&crate::util::run(None, "curl", &["-fsSL", "--max-time", "30", REGISTRY]).map_err(|e| format!("could not read the pack list: {e}"))?)
}

/// OWNER/REPO of a GitHub repository link.
fn github(url: &str) -> Result<String, String> {
    let rest = url.trim_end_matches('/').trim_end_matches(".git");
    let rest = rest.strip_prefix("https://github.com/").or(rest.strip_prefix("github.com/")).ok_or(format!("{url}: packs come from https://github.com/OWNER/REPO"))?;
    let parts: Vec<&str> = rest.split('/').collect();
    let ok = |s: &str| !s.is_empty() && s != "." && s != ".." && s.chars().all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c));
    if parts.len() != 2 || !parts.iter().all(|p| ok(p)) {
        return Err(format!("{url}: packs come from the root of https://github.com/OWNER/REPO"));
    }
    Ok(rest.to_string())
}

/// Copies a pack folder's own files (never links, what runs or is read under 20 MB, the cover any size) into
/// ~/.config/edits/packs/<id>, and remembers where it came from.
fn install(top: &Path, from: &str) -> Result<Pack, String> {
    let pack = read(top)?;
    let to = dir().join(&pack.manifest.id);
    let _ = std::fs::remove_dir_all(&to);
    for sub in ["presets", "skills", "sites"] {
        std::fs::create_dir_all(to.join(sub)).map_err(|e| e.to_string())?;
    }
    let mut names: Vec<String> = PACK_FILES.iter().map(|f| f.to_string()).chain(programs(&pack.manifest)).chain(pack.manifest.preview.clone()).collect();
    names.extend(nested(top));
    for f in names {
        let src = top.join(&f);
        // the cover can be as big as it likes (a GIF); what runs or is read stays small
        let cover = Some(f.as_str()) == pack.manifest.preview.as_deref();
        match std::fs::symlink_metadata(&src) {
            Ok(m) if m.is_file() && (cover || m.len() <= 20 << 20) => std::fs::copy(&src, to.join(&f)).map(|_| ()).map_err(|e| e.to_string())?,
            Ok(m) if m.is_file() => return Err(format!("{from}: {f} is over 20 MB")),
            Ok(_) => return Err(format!("{from}: {f} is not a plain file")),
            Err(_) => {}
        }
    }
    let _ = std::fs::remove_dir(to.join("presets"));
    let _ = std::fs::remove_dir(to.join("skills"));
    let _ = std::fs::remove_dir(to.join("sites"));
    // where it came from: a community pack, not one made here
    std::fs::write(to.join(".from"), format!("{from}\n")).map_err(|e| e.to_string())?;
    read(&to)
}

/// The one pack folder in an unpacked download or clone: itself, or the folder inside holding manifest.json.
fn pack_folder(d: &Path) -> Result<PathBuf, String> {
    if d.join("manifest.json").is_file() {
        return Ok(d.to_path_buf());
    }
    std::fs::read_dir(d).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).find(|p| p.join("manifest.json").is_file())
        .ok_or(format!("{}: no manifest.json in it, so it is not a pack (PACKS.md)", d.display()))
}

/// Whether `what` names a git repository to clone (any host), rather than a GitHub page or a pack's id.
pub fn is_git(what: &str) -> bool {
    what.starts_with("git@") || what.starts_with("ssh://") || what.starts_with("git://") || (what.starts_with("https://") && what.ends_with(".git") && !what.contains("github.com"))
}

/// Adds a pack: by its id in packs.json, its GitHub link, any git repository (cloned), or a file here (a folder, or
/// an exported .tgz). It is checked, and only its own files are kept, in ~/.config/edits/packs/<id>.
pub fn add(what: &str) -> Result<Pack, String> {
    let tmp = std::env::temp_dir().join(format!("edits-pack-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("x")).map_err(|e| e.to_string())?;
    let out = tmp.join("x");
    let res = (|| {
        let local = Path::new(what);
        if local.is_dir() {
            return install(&pack_folder(local)?, &format!("file:{}", local.display()));
        }
        if local.is_file() {
            crate::util::run(None, "tar", &["-xzf", &local.to_string_lossy(), "-C", &out.to_string_lossy()]).map_err(|e| format!("{what}: not an exported pack (.tgz): {e}"))?;
            return install(&pack_folder(&out)?, &format!("file:{}", local.display()));
        }
        if is_git(what) {
            crate::util::run(None, "git", &["clone", "--depth", "1", "--quiet", "--", what, &out.join("repo").to_string_lossy()]).map_err(|e| format!("could not clone {what}: {e}"))?;
            return install(&pack_folder(&out.join("repo"))?, what);
        }
        let repo = if what.contains("github.com") {
            github(what)?
        } else {
            let list = registry()?;
            let e = list.iter().find(|e| e.id == what).ok_or(format!("no pack \"{what}\" in the list (edits packs browse), and no such file or git repository"))?;
            github(&e.repo)?
        };
        let tar = tmp.join("pack.tar.gz");
        crate::util::run(None, "curl", &["-fsSL", "--max-time", "120", "-o", &tar.to_string_lossy(), &format!("https://codeload.github.com/{repo}/tar.gz/HEAD")])
            .map_err(|e| format!("could not download {repo}: {e}"))?;
        crate::util::run(None, "tar", &["-xzf", &tar.to_string_lossy(), "-C", &out.to_string_lossy()])?;
        // the archive holds one folder, named after the repository and commit
        let top = std::fs::read_dir(&out).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).find(|p| p.is_dir()).ok_or("the download was empty")?;
        install(&top, &format!("https://github.com/{repo}"))
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    res
}

/// Takes a pack away, made here or added. Built-in packs stay (turn them off instead).
pub fn remove(id: &str) -> Result<Pack, String> {
    if id.starts_with("edits.") {
        return Err(format!("{id} is built in: it cannot be deleted, only turned off (edits packs off {id})"));
    }
    if !valid_id(id) {
        return Err(format!("\"{id}\" is not a pack here (edits packs lists them)"));
    }
    let d = dir().join(id);
    let pack = read(&d).map_err(|_| format!("\"{id}\" is not added (edits packs lists them)"))?;
    std::fs::remove_dir_all(&d).map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(settings_file(id));
    if disabled().iter().any(|x| x == id) {
        let _ = enable(id, true);
    }
    Ok(pack)
}

/// A SKILL.md for a pack that has none: when an agent should reach for it, and the commands that use it.
pub fn draft_skill(m: &Manifest, p: Option<&Preset>) -> String {
    let has = |k: &str| m.kinds.iter().any(|x| x == k);
    // YAML: one double-quoted line, so a colon or a # in a preset's name cannot break the front matter
    let q = |s: &str| format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " "));
    let (about, body) = if let Some(a) = m.automation.as_ref().filter(|_| has("automation")) {
        (format!("Post finished clips to {} from the person's own signed-in account with edits ({}). Use when the user wants an edit posted on {}.", a.site, m.name, a.site),
         format!("```sh\nedits connect {0}                     # once: sign in, in a window of its own\nedits post clip.mp4 --to {0} --caption \"...\"\nedits follow <channel|feed> --post {0}   # every new video, made and posted\n```\n\nIt runs only on {0}, from the person's own account. Ask before posting anything they did not ask to post.\n", a.site))
    } else if has("source") {
        let short = m.id.rsplit('.').next().unwrap_or(&m.id);
        let set: Vec<String> = m.settings.iter().map(|x| format!("{}={}", x.key, if x.default.is_empty() { "..." } else { &x.default })).collect();
        (format!("Make videos from what the {} source finds, with edits: its program runs on its own and gives videos or headlines. Use when the user wants {} videos or that source on autopilot.", m.name, m.name),
         format!("```sh\nedits source {short} --list        # what it finds now (--json for programs)\nedits source {short} -n 3          # a video for each of the 3 first\n{}edits follow {short} --post tiktok  # as it finds new things: made and posted\n```\n\nIts program runs alone: its own folder, {}. Added from someone else, it runs only after the person reads it and runs edits packs trust {}.\n",
            if set.is_empty() { String::new() } else { format!("edits packs set {} {}\n", m.id, set.join(" ")) },
            if m.source.as_ref().is_some_and(|x| x.network) { "the internet" } else { "no internet" }, m.id))
    } else if has("scene") {
        (format!("Lay the {} 3D scene (three.js) over an edit with edits. Use when the user wants {} or a 3D overlay on a clip.", m.name, m.description.trim_end_matches('.')),
         format!("```sh\nedits <link|topic|video> --overlay {0}\nedits <link> --overlay {0}@2-6:0.8   # from 2 s to 6 s, at 80%\n```\n\nIts scene.js runs alone: in a browser of its own with no window, no files and nothing of the person's; only three.js is loaded for it. Added from someone else, it runs only after edits packs trust {0}.\n", m.id))
    } else if has("feed") {
        let short = m.id.strip_prefix("edits.rss-").unwrap_or(&m.id);
        let some: Vec<&str> = m.feeds.iter().take(3).map(|f| f.name.as_str()).collect();
        (format!("Make short videos from {} RSS ({} feeds, {} and more) with edits, each new post's headline cut from footage found for it and shown on screen. Use when the user wants {} videos from the news, posts on autopilot, or an automation from RSS to video to social media.", m.name, m.feeds.len(), some.join(", "), m.name),
         format!("```sh\nedits rss {short} --list                    # the newest posts (--json for programs)\nedits rss {short} -n 3                       # a video for each of the 3 newest\nedits rss {short} --feed all -n 5 --preset sinema-bant\nedits follow {short} --post tiktok --every 30 # RSS in, video made, posted out (edits watch runs it)\n```\n\nFeeds:\n\n{}\n\nEach video is footage found for the headline, with the headline on screen for its first seconds. Check a headline before posting it, and say whose post it was.\n",
            m.feeds.iter().map(|f| format!("- {} {}", f.name, f.url)).collect::<Vec<_>>().join("\n")))
    } else if let Some(p) = p {
        let tags = m.tags.join(", ");
        (format!("Cut an edit in the {} style with edits ({}). Use when the user asks for {} or a {} edit.", p.name, m.description, p.name, if tags.is_empty() { "similar" } else { &tags }),
         format!("```sh\nedits <link|topic|video> --preset {0}\nedits <link> --preset {0} --music trend:1     # on a trending song\n```\n\nMCP: `edits_make` with `preset` \"{0}\". Read it with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.\n\n{1:.1} s, {2} shots, {3}x{4}.\n", p.name, p.total, p.slots.len(), p.w, p.h))
    } else {
        (format!("{} with edits. Use when the user asks for {}.", m.description, m.name), String::new())
    };
    format!("---\nname: {}\ndescription: {}\n---\n\n# {}\n\n{}\n\n{body}", m.id.replace('.', "-"), q(&about), m.name, m.description)
}

/// Makes `d` a pack: with `preset` it first writes that preset there as preset.json; then writes manifest.json
/// (filling what it lacks from the preset or SKILL.md), a SKILL.md when it has none, and preview.svg, unless the pack
/// has its own picture.
pub fn make(d: &Path, preset: Option<&str>, id: Option<&str>, opml: Option<&Path>) -> Result<Manifest, String> {
    std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    if let Some(name) = preset {
        std::fs::write(d.join("preset.json"), crate::preset::pretty(&crate::preset::load(name)?)).map_err(|e| e.to_string())?;
    }
    let text = |f: &str| std::fs::read_to_string(d.join(f)).ok();
    // a family's first preset stands for it when there is no preset.json
    let first = text("preset.json").or_else(|| family(d).first().and_then(|f| std::fs::read_to_string(f).ok()));
    let p: Option<Preset> = first.map(|t| serde_json::from_str(&t).map_err(|e| format!("{}: its preset: {e}", d.display()))).transpose()?;
    let (skill, automation, scene) = (text("SKILL.md"), text("automation.js"), text("scene.js"));
    let old: Option<Manifest> = text("manifest.json").and_then(|t| serde_json::from_str(&t).ok());
    // an OPML list (a reader's export) gives the feeds, and its first group's name the pack's
    let listed = opml.map(|f| std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display())).and_then(|x| crate::rss::opml(&x))).transpose()?;
    let feeds = listed.as_ref().map(|l| l.1.clone()).or(old.as_ref().map(|m| m.feeds.clone())).unwrap_or_default();
    let group = listed.and_then(|l| l.0);
    let topics = old.as_ref().map(|m| m.topics.clone()).unwrap_or_default();
    if p.is_none() && skill.is_none() && automation.is_none() && scene.is_none() && feeds.is_empty() && topics.is_empty() && old.as_ref().is_none_or(|m| m.source.is_none()) {
        return Err(format!("{}: put a preset.json, a SKILL.md, an automation.js or a scene.js in it (or feeds in its manifest.json), or give --preset NAME", d.display()));
    }
    let folder = d.canonicalize().ok().and_then(|c| c.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_else(|| "pack".into());
    let about = skill.as_deref().and_then(|s| front(s, "description")).map(String::from);
    let id = id.map(String::from).or(old.as_ref().map(|m| m.id.clone())).unwrap_or_else(|| crate::preset::slug(&folder));
    // a SKILL.md is drafted when there is none, so every pack has one
    let has_source = old.as_ref().is_some_and(|m| m.source.is_some());
    let kinds: Vec<String> = [("preset", p.is_some()), ("skill", true), ("automation", automation.is_some() || old.as_ref().is_some_and(|m| !m.sites.is_empty())), ("feed", !feeds.is_empty() || !topics.is_empty()), ("source", has_source), ("scene", scene.is_some())]
        .iter().filter(|k| k.1).map(|k| k.0.to_string()).collect();
    let own_picture = old.as_ref().and_then(|m| m.preview.clone()).filter(|f| f != "preview.svg" && d.join(f).is_file());
    let m = Manifest {
        schema_version: 1,
        name: old.as_ref().map(|m| m.name.clone()).or(group.clone()).or(p.as_ref().map(|p| p.name.clone())).or(skill.as_deref().and_then(skill_name).map(String::from)).unwrap_or(folder),
        version: old.as_ref().map_or("1.0.0".into(), |m| m.version.clone()),
        author: old.as_ref().map(|m| m.author.clone()).unwrap_or_else(|| if id.starts_with("edits.") { "edits".into() } else { std::env::var("USER").or(std::env::var("USERNAME")).unwrap_or_else(|_| "you".into()) }),
        description: old.as_ref().map(|m| m.description.clone())
            .or(group.as_ref().map(|g| format!("{g} from {} feeds, {}: videos from their newest posts.", feeds.len(), feeds.iter().take(3).map(|f| f.name.as_str()).collect::<Vec<_>>().join(", "))))
            .or(p.as_ref().map(|p| p.about.clone()).filter(|a| !a.is_empty())).or(about).unwrap_or_default(),
        kinds,
        preview: Some(own_picture.clone().unwrap_or_else(|| "preview.svg".into())),
        tags: tidy_tags(&old.as_ref().map(|m| m.tags.clone()).filter(|t| !t.is_empty()).unwrap_or_else(|| p.as_ref().map(|p| p.tags().iter().map(|t| t.to_string()).collect()).unwrap_or_default())),
        license: old.as_ref().and_then(|m| m.license.clone()),
        source: old.as_ref().and_then(|m| m.source.clone()),
        settings: old.as_ref().map(|m| m.settings.clone()).unwrap_or_default(),
        scene: old.as_ref().and_then(|m| m.scene.clone()).or(scene.as_ref().map(|_| Scene { seconds: 4.0, fps: 30 })),
        sites: old.as_ref().map(|m| m.sites.clone()).unwrap_or_default(),
        automation: old.and_then(|m| m.automation),
        feeds,
        topics,
        id,
    };
    let skill = match skill {
        Some(s) => s,
        None => {
            let s = draft_skill(&m, p.as_ref());
            std::fs::write(d.join("SKILL.md"), &s).map_err(|e| e.to_string())?;
            s
        }
    };
    let presets = family(d).iter().filter_map(|f| std::fs::read_to_string(f).ok()).collect();
    let skills = files_in(&d.join("skills"), "md").iter().filter_map(|f| std::fs::read_to_string(f).ok()).collect();
    check_pack(&Pack { manifest: m.clone(), preset: text("preset.json"), skill: Some(skill), automation, presets, scene, skills, site_js: site_scripts(d) }, m.id.starts_with("edits."))?;
    if own_picture.is_none() {
        std::fs::write(d.join("preview.svg"), preview(&m, p.as_ref())).map_err(|e| e.to_string())?;
    }
    std::fs::write(d.join("manifest.json"), serde_json::to_string_pretty(&m).map_err(|e| e.to_string())? + "\n").map_err(|e| e.to_string())?;
    Ok(m)
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// `s` in lines of at most `n` characters, `max` of them, the last one cut with "...".
fn lines(s: &str, n: usize, max: usize) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for w in s.split_whitespace() {
        match out.last_mut() {
            Some(l) if l.chars().count() + 1 + w.chars().count() <= n => {
                l.push(' ');
                l.push_str(w);
            }
            _ => out.push(w.to_string()),
        }
    }
    if out.len() > max {
        out.truncate(max);
        let last = &mut out[max - 1];
        *last = last.chars().take(n.saturating_sub(3)).collect::<String>() + "...";
    }
    out
}

/// A 640x360 card for a pack: its name, what it is, and for a preset its shots, zoom curve and flashes over time,
/// tinted by its colour look.
pub fn preview(m: &Manifest, p: Option<&Preset>) -> String {
    const W: f64 = 640.0;
    let look = p.and_then(|p| p.look.clone()).unwrap_or_else(|| m.id.clone());
    let hue = crate::util::crc32(look.as_bytes()) % 360;
    let sat = if ["bw", "mono", "grey", "gray", "noir"].iter().any(|g| look.contains(g)) { 0 } else { 45 };
    let (bg, ink) = (format!("hsl({hue},{sat}%,11%)"), format!("hsl({hue},{}%,64%)", if sat == 0 { 0 } else { 70 }));
    let has = |k: &str| m.kinds.iter().any(|x| x == k);
    // one word in sentence case (BRAND.md § 4): a preset and a skill are both a pack to the person
    let kind = if has("automation") { "Automation" } else if has("feed") { "RSS" } else if has("scene") { "3D scene" } else { "Pack" };
    let font = "font-family=\"Inter, Helvetica, Arial, sans-serif\"";
    let mut s = format!("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 640 360\" width=\"640\" height=\"360\">\n\
        <rect width=\"640\" height=\"360\" rx=\"24\" fill=\"{bg}\"/>\n\
        <text x=\"32\" y=\"48\" {font} font-size=\"14\" font-weight=\"700\" fill=\"{ink}\">{kind}</text>\n\
        <text x=\"32\" y=\"94\" {font} font-size=\"38\" font-weight=\"800\" fill=\"#fff\">{}</text>\n",
        esc(&lines(&m.name, 26, 1).concat()));
    let about = lines(&m.description, 62, if p.is_some() { 2 } else { 6 });
    for (i, l) in about.iter().enumerate() {
        s += &format!("<text x=\"32\" y=\"{}\" {font} font-size=\"16\" fill=\"#fff\" fill-opacity=\"0.72\">{}</text>\n", 130 + i * 24, esc(l));
    }
    match p {
        Some(p) => {
            // the frame's shape, top right
            let (fh, fw) = (64.0, (64.0 * p.w as f64 / p.h.max(1) as f64).min(110.0));
            s += &format!("<rect x=\"{:.1}\" y=\"28\" width=\"{fw:.1}\" height=\"{fh}\" rx=\"6\" fill=\"none\" stroke=\"{ink}\" stroke-width=\"2\"/>\n", W - 32.0 - fw);
            let (x0, span, total) = (32.0, W - 64.0, p.total.max(0.01));
            let x = |t: f64| x0 + (t / total).clamp(0.0, 1.0) * span;
            for f in &p.flashes {
                s += &format!("<rect x=\"{:.1}\" y=\"206\" width=\"{:.1}\" height=\"110\" fill=\"#fff\" fill-opacity=\"0.22\"/>\n", x(f.start), (x(f.end) - x(f.start)).max(2.0));
            }
            let zmax = p.slots.iter().flat_map(|s| s.zoom.keys.iter().map(|k| k.1)).fold(1.3f64, f64::max).min(5.0);
            let y = |z: f64| 284.0 - ((z - 1.0) / (zmax - 1.0)).clamp(-0.3, 1.0) * 70.0;
            let mut pts = vec![];
            for (i, sl) in p.slots.iter().enumerate() {
                let (a, b) = (x(sl.start), x(sl.end));
                s += &format!("<rect x=\"{a:.1}\" y=\"296\" width=\"{:.1}\" height=\"20\" rx=\"3\" fill=\"{ink}\" fill-opacity=\"{}\"/>\n", (b - a - 2.0).max(1.0), if i % 2 == 0 { 0.95 } else { 0.55 });
                let keys: Vec<(f64, f64)> = if sl.zoom.keys.is_empty() { vec![(0.0, 1.0), (1.0, 1.0)] } else { sl.zoom.keys.iter().map(|k| (k.0, k.1)).collect() };
                pts.extend(keys.iter().map(|(t, z)| format!("{:.1},{:.1}", a + t * (b - a), y(*z))));
            }
            s += &format!("<polyline points=\"{}\" fill=\"none\" stroke=\"#fff\" stroke-width=\"2.5\" stroke-linejoin=\"round\"/>\n", pts.join(" "));
            s += &format!("<text x=\"32\" y=\"344\" {font} font-size=\"13\" fill=\"#fff\" fill-opacity=\"0.6\">{}</text>\n", esc(&m.tags.join("  ")));
            s += &format!("<text x=\"608\" y=\"344\" text-anchor=\"end\" {font} font-size=\"13\" fill=\"#fff\" fill-opacity=\"0.6\">{} shots  {:.1}s</text>\n", p.slots.len(), p.total);
        }
        None if !m.topics.is_empty() => {
            let some: Vec<&str> = m.topics.iter().take(5).map(|t| t.name.as_str()).collect();
            s += &format!("<text x=\"32\" y=\"336\" {font} font-size=\"14\" fill=\"{ink}\">{}  +{} more</text>\n", esc(&some.join("  ·  ")), m.topics.len().saturating_sub(5));
        }
        None if has("feed") => {
            let some: Vec<String> = m.feeds.iter().take(3).map(|f| f.name.chars().take(24).collect()).collect();
            let more = if m.feeds.len() > 3 { format!("  +{} more", m.feeds.len() - 3) } else { String::new() };
            s += &format!("<text x=\"32\" y=\"336\" {font} font-size=\"14\" fill=\"{ink}\">{}{more}</text>\n", esc(&some.join("  ·  ")));
        }
        None if has("automation") => {
            let sites: Vec<&str> = m.automation.iter().chain(&m.sites).map(|a| a.site.as_str()).collect();
            s += &format!("<text x=\"32\" y=\"336\" {font} font-size=\"14\" fill=\"{ink}\">edits connect {}</text>\n", esc(&sites.join("|")));
        }
        None if has("scene") => s += &format!("<text x=\"32\" y=\"336\" {font} font-size=\"14\" fill=\"{ink}\">edits &lt;link&gt; --overlay {}</text>\n", esc(m.id.rsplit('.').next().unwrap_or(&m.id))),
        None => s += &format!("<text x=\"32\" y=\"336\" {font} font-size=\"14\" fill=\"{ink}\">edits skills install</text>\n"),
    }
    s + "</svg>\n"
}

/// README.md in `d` showing every pack folder in it by its picture.
pub fn gallery(d: &Path) -> Result<usize, String> {
    let mut packs: Vec<(String, Manifest)> = std::fs::read_dir(d).map_err(|e| e.to_string())?.flatten()
        .filter_map(|e| Some((e.file_name().to_string_lossy().into_owned(), serde_json::from_str(&std::fs::read_to_string(e.path().join("manifest.json")).ok()?).ok()?)))
        .collect();
    packs.sort_by(|a, b| a.0.cmp(&b.0));
    let mut s = format!("# Packs\n\n{} built-in packs, each with a `manifest.json`, an agent skill and a picture. The marketplace: \
        `edits packs browse`, `edits packs add ID`, or the app. The standard, and making one: [PACKS.md](../PACKS.md).\n", packs.len());
    // the RSS banner, in the reader's light or dark
    if d.join("rss-light.svg").is_file() && d.join("rss-dark.svg").is_file() {
        s += "\n<picture><source media=\"(prefers-color-scheme: dark)\" srcset=\"rss-dark.svg\"><img src=\"rss-light.svg\" width=\"100%\" \
            alt=\"RSS in. Edits out. Each new post from BBC, CNN, The Guardian, NYT, Reddit, IGN, TechCrunch, NASA, Anadolu Ajansı or TRT Haber becomes an edit, and a flow posts it.\"></picture>\n";
    }
    let has = |m: &Manifest, k: &str| m.kinds.iter().any(|x| x == k);
    type Wants<'a> = &'a dyn Fn(&Manifest) -> bool;
    let sections: [(&str, Wants); 4] = [
        ("RSS", &|m| has(m, "feed")),
        ("Presets", &|m| has(m, "preset")),
        ("Automations", &|m| has(m, "automation")),
        ("Agent skills", &|m| !has(m, "feed") && !has(m, "preset") && !has(m, "automation")),
    ];
    for (title, wants) in sections {
        let these: Vec<&(String, Manifest)> = packs.iter().filter(|p| wants(&p.1)).collect();
        if these.is_empty() {
            continue;
        }
        s += &format!("\n## {title}\n\n<p>\n");
        for (f, m) in these {
            let pic = m.preview.as_deref().unwrap_or("preview.svg");
            s += &format!("<a href=\"{f}\"><img src=\"{f}/{pic}\" width=\"300\" alt=\"{}\"></a>\n", esc(&m.name));
        }
        s += "</p>\n";
    }
    std::fs::write(d.join("README.md"), s).map_err(|e| e.to_string())?;
    Ok(packs.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pack_carries_at_most_three_of_the_ten_tags() {
        assert_eq!(tidy_tags(&["slow", "9:16", "velocity", "rgb", "fade", "comic"]), ["hype", "glitch", "sad"]);
        for p in built_in() {
            let t = tidy_tags(&p.manifest.tags);
            assert_eq!(t, p.manifest.tags, "{}: its tags are already tidy", p.manifest.id);
            assert!(t.len() <= 3 && t.iter().all(|x| TAGS.contains(&x.as_str())), "{}", p.manifest.id);
        }
    }

    #[test]
    fn built_in_packs_are_whole() {
        assert!(BUILT_IN_PACKS.len() >= 15);
        let mut ids = vec![];
        for p in built_in() {
            check_pack(&p, true).unwrap_or_else(|e| panic!("{e}"));
        }
        for (d, m, ..) in BUILT_IN_PACKS {
            let m: Manifest = serde_json::from_str(m).unwrap_or_else(|e| panic!("{d}: {e}"));
            assert_eq!(m.id, format!("edits.{d}"));
            assert!(m.preview.as_ref().is_some_and(|f| Path::new(env!("CARGO_MANIFEST_DIR")).join("packs").join(d).join(f).is_file()), "{d}: no picture");
            assert!(!ids.contains(&m.id));
            ids.push(m.id);
        }
    }

    #[test]
    fn the_community_list_reads() {
        parse_registry(include_str!("../packs.json")).unwrap();
        assert!(parse_registry(r#"{"packs":[{"id":"edits.x","repo":"https://github.com/a/b","name":"x","description":"x"}]}"#).is_err());
    }

    #[test]
    fn only_plain_github_repositories_and_ids() {
        assert_eq!(github("https://github.com/a-b/c.d.git").unwrap(), "a-b/c.d");
        for bad in ["https://github.com/a/b/tree/main", "https://gitlab.com/a/b", "https://github.com/../b", "https://github.com/a"] {
            assert!(github(bad).is_err(), "{bad}");
        }
        assert!(valid_id("io.github.you.sad-pack") && !valid_id("../x") && !valid_id("A") && !valid_id("a/b"));
    }

    const SKILL: &str = "---\nname: x\ndescription: x\n---\n";

    #[test]
    fn share_links_carry_the_pack() {
        assert_eq!(encode("Pack: Sad rain/ü"), "Pack%3A%20Sad%20rain%2F%C3%BC");
        assert_eq!(origin("edits.punch-whip"), Origin::BuiltIn);
        assert!(remove("edits.punch-whip").err().is_some_and(|e| e.contains("cannot be deleted")));
        assert!(is_git("git@gitlab.com:a/b.git") && is_git("https://codeberg.org/a/b.git") && !is_git("https://github.com/a/b") && !is_git("io.github.a.b"));
    }

    #[test]
    fn a_pack_goes_out_as_a_file_and_comes_back_in() {
        let home = std::env::temp_dir().join(format!("edits-pack-export-{}", std::process::id()));
        let tgz = home.join("spin.tgz");
        let out = export("edits.spin", Some(&tgz)).unwrap();
        assert!(out.is_file() && std::fs::metadata(&out).unwrap().len() > 100);
        // unpacked, it is a whole pack with its family of presets
        let x = home.join("x");
        std::fs::create_dir_all(&x).unwrap();
        crate::util::run(None, "tar", &["-xzf", &out.to_string_lossy(), "-C", &x.to_string_lossy()]).unwrap();
        let f = pack_folder(&x).unwrap();
        assert!(f.join("presets").is_dir() && f.join("SKILL.md").is_file());
        assert_eq!(family(&f).len(), BUILT_IN_PACKS.iter().find(|b| b.0 == "spin").unwrap().6.len());
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn every_pack_needs_a_skill() {
        let m: Manifest = serde_json::from_value(serde_json::json!({"schemaVersion": 1, "id": "io.github.you.x", "name": "x", "version": "1.0.0",
            "author": "you", "description": "x", "kinds": ["feed", "skill"], "feeds": [{"name": "BBC", "url": "http://feeds.bbci.co.uk/news/rss.xml"}]})).unwrap();
        check(&m, None, Some(SKILL), None, false).unwrap();
        assert!(check(&m, None, None, None, false).is_err(), "no SKILL.md");
        let mut bad = m.clone();
        bad.feeds[0].url = "ftp://x/rss".into();
        assert!(check(&bad, None, Some(SKILL), None, false).is_err(), "a feed is http or https");
    }

    #[test]
    fn an_automation_stays_on_its_own_site() {
        assert_eq!(site_of("https://studio.youtube.com/x?y=1").as_deref(), Some("youtube.com"));
        for bad in ["http://www.tiktok.com/", "https://localhost/", "https://.com/", "tiktok.com"] {
            assert!(site_of(bad).is_none(), "{bad}");
        }
        assert_ne!(site_of("https://tiktok.com.evil.io/"), site_of("https://www.tiktok.com/"));
        let m: Manifest = serde_json::from_value(serde_json::json!({"schemaVersion": 1, "id": "io.github.you.x", "name": "x", "version": "1.0.0",
            "author": "you", "description": "x", "kinds": ["automation", "skill"],
            "automation": {"site": "x", "login": "https://x.com/login", "page": "https://evil.io/", "cookie": "auth", "domain": "x.com"}})).unwrap();
        assert!(check(&m, None, Some(SKILL), Some("async (p) => {\n}"), false).is_err(), "a page off its site");
        let mut ok = m.clone();
        ok.automation.as_mut().unwrap().page = "https://x.com/compose".into();
        check(&ok, None, Some(SKILL), Some("// posts\nasync (p) => {\n}"), false).unwrap();
        assert!(check(&ok, None, Some(SKILL), Some("fetch('https://evil.io')"), false).is_err(), "not one function");
        assert!(check(&ok, None, Some(SKILL), None, false).is_err(), "kinds says automation, no automation.js");
    }

    #[test]
    fn a_pack_is_made_from_a_preset_and_says_what_it_holds() {
        let d = std::env::temp_dir().join(format!("edits-pack-make-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        let p = BUILT_IN_PACKS.iter().find_map(|x| x.6.first()).unwrap();
        assert!(BUILT_IN_PACKS.iter().any(|x| !x.7.is_empty()), "a pack carries several skills");
        assert!(built_in_gif("edits.spin").is_some_and(|g| g.starts_with(b"GIF89a")), "built-in packs have moving covers");
        let post = built_in().into_iter().find(|p| p.manifest.id == "edits.post").unwrap();
        assert_eq!(post.automations().iter().map(|a| a.0.site.as_str()).collect::<Vec<_>>(), ["tiktok", "instagram", "youtube"], "one pack posts to every site");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("preset.json"), p).unwrap();
        let m = make(&d, None, Some("io.github.you.test"), None).unwrap();
        assert_eq!(m.kinds, ["preset", "skill"], "a skill is drafted when there is none");
        assert!(std::fs::read_to_string(d.join("SKILL.md")).unwrap().starts_with("---\nname: io-github-you-test\ndescription: \""));
        let svg = std::fs::read_to_string(d.join("preview.svg")).unwrap();
        assert!(svg.starts_with("<svg") && svg.contains("polyline"));
        assert_eq!(read(&d).unwrap().manifest.id, "io.github.you.test");
        // a pack that is added may not take a built-in id
        make(&d, None, Some("edits.mine"), None).unwrap();
        assert!(read(&d).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
