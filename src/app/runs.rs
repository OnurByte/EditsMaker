//! What the window lists and starts: past runs in edits' output folder, `edits` jobs run beside the app, and what
//! the marketplace and the review screen read from edits as JSON.
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

/// This edits itself: jobs run as it, beside the app.
pub fn edits() -> PathBuf {
    std::env::current_exe().unwrap_or_else(|_| "edits".into())
}

/// Where edits puts its runs.
pub fn out_dir() -> PathBuf {
    crate::out_base()
}

fn command(args: &[&str]) -> Command {
    let mut cmd = Command::new(edits());
    cmd.args(args).stdin(Stdio::null());
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x0800_0000); // CREATE_NO_WINDOW: no console flashes up
    cmd
}

/// `edits ARGS` to its end: what it printed, or what went wrong.
pub fn output(args: &[&str]) -> Result<String, String> {
    let out = command(args).output().map_err(|e| format!("{}: {e}", edits().display()))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("edits failed").trim_start_matches("edits: ").to_string())
    }
}

/// A pack here, as `edits packs --json` lists it.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub author: String,
    pub description: String,
    pub kinds: Vec<String>,
    /// an automation runs only once trusted; the rest always are
    pub trusted: bool,
    /// the preset it carries, by name
    pub preset: Option<String>,
    #[serde(default)]
    pub feeds: Vec<FeedRef>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "yes")]
    pub enabled: bool,
    /// built-in, community or local (made here)
    #[serde(default)]
    pub origin: String,
    /// when it came here (seconds), 0 for a built-in one
    #[serde(default)]
    pub added: u64,
    pub preview_file: Option<PathBuf>,
    pub preview_svg: Option<String>,
}

/// A pack in the marketplace's list (packs.json), as `edits packs browse --json` gives it.
#[derive(Clone, Deserialize)]
pub struct Listed {
    pub id: String,
    pub name: String,
    pub description: String,
    pub kinds: Vec<String>,
    pub added: bool,
}

fn yes() -> bool {
    true
}

impl Pack {
    pub fn is(&self, kind: &str) -> bool {
        self.kinds.iter().any(|k| k == kind)
    }

    /// What `edits rss` and `edits follow` call a feed pack: football for edits.rss-football.
    pub fn short(&self) -> String {
        self.id.strip_prefix("edits.rss-").unwrap_or(&self.id).to_string()
    }
}

#[derive(Clone, Deserialize)]
pub struct FeedRef {
    pub name: String,
    pub url: String,
}

/// A follow, as the app shows it: a flow from a source, made into edits, posted to accounts.
#[derive(Clone, Deserialize)]
pub struct Flow {
    pub source: String,
    pub args: Vec<String>,
    pub post: String,
    pub every: u64,
    pub titles: bool,
}

pub fn flows() -> Result<Vec<Flow>, String> {
    serde_json::from_value(crate::follows_json()).map_err(|e| e.to_string())
}

pub fn packs() -> Result<Vec<Pack>, String> {
    serde_json::from_value(crate::packs_json()).map_err(|e| e.to_string())
}

pub fn listed() -> Result<Vec<Listed>, String> {
    serde_json::from_value(crate::listed_json(&crate::packs::registry()?)).map_err(|e| e.to_string())
}

/// What the settings screen shows, from `edits settings --json`.
#[derive(Clone, Deserialize)]
pub struct Settings {
    pub version: String,
    pub ai: Ai,
    pub providers: Vec<Provider>,
    pub encoder: String,
    pub encoders: Vec<String>,
    pub updates: bool,
    pub out: PathBuf,
    pub accounts: Vec<Account>,
    /// where an automation can post
    pub sites: Vec<String>,
}

/// A provider as t3code's cards show one: how it is reached, ready here or not, the one in use.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub name: String,
    /// what t3code calls it
    pub label: String,
    /// once (asked with the prompt), acp (spoken to over ACP), http
    pub how: String,
    /// the agent's command
    pub command: String,
    /// subscription (its own CLI), api (a key), local (a server on this computer)
    pub kind: String,
    pub ready: bool,
    pub in_use: bool,
    pub base: String,
    pub key_var: String,
    pub about: String,
}

#[derive(Clone, Deserialize)]
pub struct Ai {
    pub model: String,
    pub ready: bool,
}

#[derive(Clone, Deserialize)]
pub struct Account {
    pub name: String,
    pub platform: String,
    pub user: String,
}

pub fn settings() -> Result<Settings, String> {
    serde_json::from_value(crate::settings_json()).map_err(|e| e.to_string())
}

/// What the review screen shows of a clip, from `edits review CLIP --frames`.
#[derive(Clone, Deserialize)]
pub struct Sheet {
    pub clip: PathBuf,
    pub name: String,
    pub total: f64,
    pub cuts: Vec<f64>,
    pub about: Vec<String>,
    pub frames: Vec<Frame>,
    pub notes: PathBuf,
}

#[derive(Clone, Deserialize)]
pub struct Frame {
    pub t: f64,
    pub file: PathBuf,
}

pub fn sheet(clip: &Path) -> Result<Sheet, String> {
    serde_json::from_value(crate::review::sheet(clip)?).map_err(|e| e.to_string())
}

/// A note on a moment: when, where in the frame (0..1 across and down), what to change.
#[derive(Clone)]
pub struct Note {
    pub t: f64,
    pub at: Option<(f64, f64)>,
    pub text: String,
}

/// NAME.notes.json as `edits review` reads it back.
pub fn save_notes(s: &Sheet, notes: &[Note], general: &str) -> std::io::Result<()> {
    let list: Vec<Value> = notes.iter().map(|n| json!({"t": n.t, "x": n.at.map(|a| a.0), "y": n.at.map(|a| a.1), "text": n.text})).collect();
    std::fs::write(&s.notes, serde_json::to_string_pretty(&json!({"clip": s.clip, "notes": list, "general": general})).unwrap_or_default())
}

/// How long ago, as t3code says it: now, 24m, 3h, 2d.
pub fn ago(when: std::time::SystemTime) -> String {
    let s = when.elapsed().map(|d| d.as_secs()).unwrap_or(0);
    match s {
        0..60 => "now".into(),
        60..3600 => format!("{}m", s / 60),
        3600..86400 => format!("{}h", s / 3600),
        _ => format!("{}d", s / 86400),
    }
}

/// The first letters of up to two words, for a badge: "Barış Özcan" BÖ.
pub fn initials(s: &str) -> String {
    s.split_whitespace().take(2).filter_map(|w| w.chars().next()).flat_map(char::to_uppercase).collect()
}

/// Words as a shell splits them, enough for a link, its options and a quoted path: "a b" stays one word.
pub fn split_args(s: &str) -> Vec<String> {
    let (mut out, mut word, mut quoted, mut any) = (vec![], String::new(), false, false);
    for c in s.chars() {
        match c {
            '"' => (quoted, any) = (!quoted, true),
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut word));
                }
                any = false;
            }
            c => (word, any) = (word + &c.to_string(), true),
        }
    }
    if any {
        out.push(word);
    }
    out
}

/// A run edits made: OUT/DAY/SUBJECT/HH.MM.
pub struct Run {
    pub subject: String,
    pub when: String,
    pub dir: PathBuf,
    pub made: std::time::SystemTime,
}

fn dirs(p: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(p).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default();
    v.sort();
    v
}

fn name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

/// The runs in `base`, newest first.
pub fn past(base: &Path) -> Vec<Run> {
    let mut v = vec![];
    for day in dirs(base) {
        for subject in dirs(&day) {
            for time in dirs(&subject) {
                let made = std::fs::metadata(&time).and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
                v.push(Run { subject: name(&subject), when: format!("{} {}", name(&day), name(&time)), dir: time, made });
            }
        }
    }
    v.sort_by(|a, b| b.when.cmp(&a.when));
    v.truncate(300);
    v
}

/// The files of a run, by name.
pub fn files(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.path()).filter(|p| p.is_file()).collect()).unwrap_or_default();
    v.sort();
    v
}

/// The files and folders a job's output names ("PATH" or "PATH: what it is"): where its clips and packs went.
pub fn paths_in(lines: &[String]) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = vec![];
    for l in lines {
        let l = l.trim();
        let p = [l, l.split(": ").next().unwrap_or("")].into_iter().map(PathBuf::from).find(|p| p.is_absolute() && p.exists());
        if let Some(p) = p.filter(|p| !v.contains(p)) {
            v.push(p);
        }
    }
    v
}

/// The clips of a run (its videos), by name.
pub fn clips(dir: &Path) -> Vec<PathBuf> {
    files(dir).into_iter().filter(|p| p.extension().is_some_and(|e| ["mp4", "mov", "webm"].contains(&e.to_string_lossy().to_lowercase().as_str()))).collect()
}

/// `edits ARGS` running beside the app: its output lines as they come, and whether it worked once it ends.
pub struct Job {
    args: Vec<String>,
    lines: Arc<Mutex<Vec<String>>>,
    ok: Arc<Mutex<Option<bool>>>,
    pub started: std::time::Instant,
    /// how long it took, once it ended
    pub took: Option<std::time::Duration>,
}

impl Job {
    pub fn start(args: Vec<String>) -> Job {
        let job = Job { args, lines: Default::default(), ok: Default::default(), started: std::time::Instant::now(), took: None };
        let args: Vec<&str> = job.args.iter().map(String::as_str).collect();
        match command(&args).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            Err(e) => {
                job.lines.lock().unwrap().push(format!("could not start {}: {e}", edits().display()));
                *job.ok.lock().unwrap() = Some(false);
            }
            Ok(mut child) => {
                let pipes: [Option<Box<dyn Read + Send>>; 2] = [child.stdout.take().map(|p| Box::new(p) as _), child.stderr.take().map(|p| Box::new(p) as _)];
                let readers: Vec<_> = pipes.into_iter().flatten().map(|pipe| {
                    let lines = job.lines.clone();
                    std::thread::spawn(move || {
                        for l in BufReader::new(pipe).split(b'\n').map_while(Result::ok) {
                            // a progress line redraws itself with \r: keep what it says last
                            let l = String::from_utf8_lossy(&l);
                            lines.lock().unwrap().push(l.rsplit('\r').find(|p| !p.trim().is_empty()).unwrap_or("").trim_end().to_string());
                        }
                    })
                }).collect();
                let ok = job.ok.clone();
                std::thread::spawn(move || {
                    let status = child.wait();
                    readers.into_iter().for_each(|r| drop(r.join()));
                    *ok.lock().unwrap() = Some(status.is_ok_and(|s| s.success()));
                });
            }
        }
        job
    }

    pub fn title(&self) -> String {
        self.args.join(" ")
    }

    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().unwrap().clone()
    }

    /// None while it runs, then whether it worked.
    pub fn finished(&self) -> Option<bool> {
        *self.ok.lock().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badges_and_ages() {
        assert_eq!(initials("Barış Özcan"), "BÖ");
        assert_eq!(initials("madara"), "M");
        let t = std::time::SystemTime::now();
        assert_eq!(ago(t), "now");
        assert_eq!(ago(t - std::time::Duration::from_secs(3 * 3600 + 5)), "3h");
        assert_eq!(ago(t - std::time::Duration::from_secs(2 * 86400)), "2d");
    }

    #[test]
    fn notes_are_saved_as_edits_reads_them() {
        let d = std::env::temp_dir().join(format!("edits-app-notes-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let s = Sheet { clip: d.join("a.mp4"), name: "a.mp4".into(), total: 4.0, cuts: vec![], about: vec![], frames: vec![], notes: d.join("a.notes.json") };
        save_notes(&s, &[Note { t: 1.5, at: Some((0.5, 0.25)), text: "darker".into() }, Note { t: 3.0, at: None, text: "cut".into() }], "louder").unwrap();
        let v: Value = serde_json::from_str(&std::fs::read_to_string(&s.notes).unwrap()).unwrap();
        assert_eq!(v["notes"][0], json!({"t": 1.5, "x": 0.5, "y": 0.25, "text": "darker"}));
        assert_eq!(v["notes"][1]["x"], Value::Null);
        assert_eq!(v["general"], "louder");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn words_split_like_a_shell() {
        assert_eq!(split_args(r#"  https://youtu.be/x  --preset "sad rain" "C:\My Videos\a.mp4" "" "#), ["https://youtu.be/x", "--preset", "sad rain", r"C:\My Videos\a.mp4", ""]);
        assert!(split_args("   ").is_empty());
    }

    #[test]
    fn runs_are_read_as_edits_lays_them_out_newest_first() {
        let base = std::env::temp_dir().join(format!("edits-app-runs-{}", std::process::id()));
        for d in ["2026-11-21/Madara/22.10", "2026-11-22/Madara/09.05", "2026-11-22/Barış/09.30"] {
            std::fs::create_dir_all(base.join(d)).unwrap();
        }
        let clip = base.join("2026-11-22/Barış/09.30/clip 1.mp4");
        std::fs::write(&clip, b"").unwrap();
        let runs = past(&base);
        let got: Vec<(&str, &str)> = runs.iter().map(|r| (r.subject.as_str(), r.when.as_str())).collect();
        assert_eq!(got, [("Barış", "2026-11-22 09.30"), ("Madara", "2026-11-22 09.05"), ("Madara", "2026-11-21 22.10")]);
        assert_eq!(files(&runs[0].dir), std::slice::from_ref(&clip));
        let lines = vec!["edits: cutting".to_string(), clip.display().to_string(), format!("{}: pack x", base.display()), clip.display().to_string()];
        assert_eq!(paths_in(&lines), [clip, base.clone()]);
        let _ = std::fs::remove_dir_all(&base);
    }
}
