//! Packs: a preset, an agent skill, an automation or some of each, in one folder with a manifest.json and a preview
//! picture (PACKS.md is the standard). The packs in packs/ are built in (ids `edits.*`); community packs live in their
//! own GitHub repositories, are listed in packs.json at the root of this repository (the marketplace), and are added
//! into ~/.config/edits/packs/<id>, each in its own folder.
use crate::preset::Preset;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

include!(concat!(env!("OUT_DIR"), "/built_in.rs"));

/// The community list `edits packs browse` reads: a pull request adding a line to packs.json is how a pack is listed.
const REGISTRY: &str = "https://raw.githubusercontent.com/OnurByte/EditsMaker/main/packs.json";
const PREVIEWS: &[&str] = &["preview.svg", "preview.png", "preview.jpg", "preview.jpeg", "preview.webp"];

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
    /// for a feed pack: the RSS or Atom feeds `edits news` and `edits follow` read, first one by default
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub feeds: Vec<Feed>,
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
}

impl Pack {
    /// The skill's own name (its folder in an agent's skills), from the SKILL.md front matter.
    pub fn skill_name(&self) -> Option<&str> {
        self.skill.as_deref().and_then(skill_name)
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
    if m.kinds.is_empty() || m.kinds.iter().any(|k| !["preset", "skill", "automation", "feed"].contains(&k.as_str())) {
        return Err(format!("{id}: kinds is some of \"preset\", \"skill\", \"automation\", \"feed\", not {:?}", m.kinds));
    }
    if skill.is_none() {
        return Err(format!("{id}: every pack carries a SKILL.md (edits packs make drafts one)"));
    }
    if m.kinds.iter().any(|k| k == "feed") != !m.feeds.is_empty() {
        return Err(format!("{id}: kinds says {:?} but it has {} feeds", m.kinds, m.feeds.len()));
    }
    // a feed is only read, so plain http is fine too
    if let Some(f) = m.feeds.iter().find(|f| f.name.trim().is_empty() || site_of(&f.url.replacen("http://", "https://", 1)).is_none()) {
        return Err(format!("{id}: feed \"{}\" needs a name and an http(s):// address", f.name));
    }
    if let Some(js) = automation {
        let a = m.automation.as_ref().ok_or(format!("{id}: an automation needs an \"automation\" block in manifest.json (PACKS.md)"))?;
        if !valid_id(&a.site) || site_of(&a.page).is_none() || site_of(&a.page) != site_of(&a.login) || a.cookie.trim().is_empty() || a.domain.trim().is_empty() {
            return Err(format!("{id}: automation needs a site name, a cookie and its domain, and its login and page as https pages of one site (PACKS.md)"));
        }
        let code: String = js.lines().filter(|l| !l.trim_start().starts_with("//")).collect::<Vec<_>>().join("\n");
        if !(code.trim_start().starts_with("async (p) => {") && code.trim_end().ends_with('}')) {
            return Err(format!("{id}: automation.js is one function, async (p) => {{ ... }}"));
        }
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

pub fn built_in() -> Vec<Pack> {
    BUILT_IN_PACKS.iter().filter_map(|(d, m, p, s, a)| {
        let manifest: Manifest = serde_json::from_str(m).map_err(|e| eprintln!("edits: pack {d}: {e}")).ok()?;
        Some(Pack { manifest, preset: p.map(String::from), skill: s.map(String::from), automation: a.map(String::from) })
    }).collect()
}

/// A pack folder as it is on disk, checked.
pub fn read(d: &Path) -> Result<Pack, String> {
    let text = |f: &str| std::fs::read_to_string(d.join(f)).ok();
    let manifest: Manifest = serde_json::from_str(&text("manifest.json").ok_or(format!("{}: no manifest.json", d.display()))?)
        .map_err(|e| format!("{}/manifest.json: {e}", d.display()))?;
    let (preset, skill, automation) = (text("preset.json"), text("SKILL.md"), text("automation.js"));
    check(&manifest, preset.as_deref(), skill.as_deref(), automation.as_deref(), false)?;
    Ok(Pack { manifest, preset, skill, automation })
}

fn trust_file(id: &str) -> PathBuf {
    dir().join(id).join(".trusted")
}

/// Whether an automation may run: built-in ones may; an added one only once the person read it and ran
/// `edits packs trust ID`, and only while its automation.js is the one they read.
pub fn trusted(p: &Pack) -> bool {
    let id = &p.manifest.id;
    id.starts_with("edits.") || crate::update::sha256(&dir().join(id).join("automation.js")).is_ok_and(|h| std::fs::read_to_string(trust_file(id)).is_ok_and(|t| t.trim() == h))
}

/// Lets an added automation run, as it is now.
pub fn trust(id: &str) -> Result<PathBuf, String> {
    let p = read(&dir().join(id)).map_err(|_| format!("\"{id}\" is not an added pack (edits packs lists them)"))?;
    if p.automation.is_none() {
        return Err(format!("{id} has no automation to trust"));
    }
    let js = dir().join(id).join("automation.js");
    std::fs::write(trust_file(id), crate::update::sha256(&js)?).map_err(|e| e.to_string())?;
    Ok(js)
}

/// The community packs added here. A broken one is said and skipped.
pub fn added() -> Vec<Pack> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(dir()).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    dirs.sort();
    dirs.iter().filter_map(|d| read(d).map_err(|e| eprintln!("edits: {e}")).ok()).collect()
}

pub fn all() -> Vec<Pack> {
    built_in().into_iter().chain(added()).collect()
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

/// Adds a community pack by its id in packs.json or its GitHub link: downloads the repository, checks it, and keeps
/// only the pack's own files (manifest.json, preset.json, SKILL.md, the preview) in ~/.config/edits/packs/<id>.
pub fn add(what: &str) -> Result<Pack, String> {
    let repo = if what.contains("github.com") {
        github(what)?
    } else {
        let list = registry()?;
        let e = list.iter().find(|e| e.id == what).ok_or(format!("no pack \"{what}\" in the list (edits packs browse)"))?;
        github(&e.repo)?
    };
    let tmp = std::env::temp_dir().join(format!("edits-pack-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let res = (|| {
        let tar = tmp.join("pack.tar.gz");
        crate::util::run(None, "curl", &["-fsSL", "--max-time", "120", "-o", &tar.to_string_lossy(), &format!("https://codeload.github.com/{repo}/tar.gz/HEAD")])
            .map_err(|e| format!("could not download {repo}: {e}"))?;
        let out = tmp.join("x");
        std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
        crate::util::run(None, "tar", &["-xzf", &tar.to_string_lossy(), "-C", &out.to_string_lossy()])?;
        // the archive holds one folder, named after the repository and commit
        let top = std::fs::read_dir(&out).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).find(|p| p.is_dir()).ok_or("the download was empty")?;
        let pack = read(&top)?;
        let to = dir().join(&pack.manifest.id);
        let _ = std::fs::remove_dir_all(&to);
        std::fs::create_dir_all(&to).map_err(|e| e.to_string())?;
        for f in ["manifest.json", "preset.json", "SKILL.md", "automation.js"].iter().copied().chain(pack.manifest.preview.as_deref()) {
            let src = top.join(f);
            match std::fs::symlink_metadata(&src) {
                Ok(m) if m.is_file() && m.len() <= 20 << 20 => std::fs::copy(&src, to.join(f)).map(|_| ()).map_err(|e| e.to_string())?,
                Ok(m) if m.is_file() => return Err(format!("{repo}: {f} is over 20 MB")),
                Ok(_) => return Err(format!("{repo}: {f} is not a plain file")),
                Err(_) => {}
            }
        }
        read(&to)
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    res
}

/// Takes an added pack away. Built-in packs stay.
pub fn remove(id: &str) -> Result<Pack, String> {
    if !valid_id(id) || id.starts_with("edits.") {
        return Err(format!("\"{id}\" is not an added pack (edits packs lists them)"));
    }
    let d = dir().join(id);
    let pack = read(&d).map_err(|_| format!("\"{id}\" is not added (edits packs lists them)"))?;
    std::fs::remove_dir_all(&d).map_err(|e| e.to_string())?;
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
    let p: Option<Preset> = text("preset.json").map(|t| serde_json::from_str(&t).map_err(|e| format!("{}/preset.json: {e}", d.display()))).transpose()?;
    let (skill, automation) = (text("SKILL.md"), text("automation.js"));
    let old: Option<Manifest> = text("manifest.json").and_then(|t| serde_json::from_str(&t).ok());
    // an OPML list (a reader's export) gives the feeds, and its first group's name the pack's
    let listed = opml.map(|f| std::fs::read_to_string(f).map_err(|e| format!("{}: {e}", f.display())).and_then(|x| crate::rss::opml(&x))).transpose()?;
    let feeds = listed.as_ref().map(|l| l.1.clone()).or(old.as_ref().map(|m| m.feeds.clone())).unwrap_or_default();
    let group = listed.and_then(|l| l.0);
    if p.is_none() && skill.is_none() && automation.is_none() && feeds.is_empty() {
        return Err(format!("{}: put a preset.json, a SKILL.md or an automation.js in it (or feeds in its manifest.json), or give --preset NAME", d.display()));
    }
    let folder = d.canonicalize().ok().and_then(|c| c.file_name().map(|n| n.to_string_lossy().into_owned())).unwrap_or_else(|| "pack".into());
    let about = skill.as_deref().and_then(|s| front(s, "description")).map(String::from);
    let id = id.map(String::from).or(old.as_ref().map(|m| m.id.clone())).unwrap_or_else(|| crate::preset::slug(&folder));
    // a SKILL.md is drafted when there is none, so every pack has one
    let kinds: Vec<String> = [("preset", p.is_some()), ("skill", true), ("automation", automation.is_some()), ("feed", !feeds.is_empty())].iter().filter(|k| k.1).map(|k| k.0.to_string()).collect();
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
        tags: old.as_ref().map(|m| m.tags.clone()).filter(|t| !t.is_empty()).unwrap_or_else(|| p.as_ref().map(|p| p.tags().iter().map(|t| t.to_string()).collect()).unwrap_or_default()),
        license: old.as_ref().and_then(|m| m.license.clone()),
        automation: old.and_then(|m| m.automation),
        feeds,
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
    check(&m, text("preset.json").as_deref(), Some(&skill), automation.as_deref(), m.id.starts_with("edits."))?;
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
    let kind = if has("automation") { "Automation" } else if has("feed") { "RSS" } else { "Pack" };
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
        None if has("feed") => {
            let some: Vec<String> = m.feeds.iter().take(3).map(|f| f.name.chars().take(24).collect()).collect();
            let more = if m.feeds.len() > 3 { format!("  +{} more", m.feeds.len() - 3) } else { String::new() };
            s += &format!("<text x=\"32\" y=\"336\" {font} font-size=\"14\" fill=\"{ink}\">{}{more}</text>\n", esc(&some.join("  ·  ")));
        }
        None if has("automation") => s += &format!("<text x=\"32\" y=\"336\" {font} font-size=\"14\" fill=\"{ink}\">edits connect {}</text>\n", esc(m.automation.as_ref().map_or("", |a| &a.site))),
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
    fn built_in_packs_are_whole() {
        assert!(BUILT_IN_PACKS.len() >= 70);
        let mut ids = vec![];
        for (d, m, p, s, a) in BUILT_IN_PACKS {
            let m: Manifest = serde_json::from_str(m).unwrap_or_else(|e| panic!("{d}: {e}"));
            check(&m, *p, *s, *a, true).unwrap();
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
        let (_, _, p, _, _) = BUILT_IN_PACKS.iter().find(|x| x.2.is_some()).unwrap();
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("preset.json"), p.unwrap()).unwrap();
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
