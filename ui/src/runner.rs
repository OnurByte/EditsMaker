use eframe::egui;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver},
    Arc, Mutex,
};

#[derive(Clone, Debug)]
pub struct LogLine {
    pub is_err: bool,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct ClipResult {
    pub clip_path: PathBuf,
    pub summary: String,
    pub raw_json: String,
}

pub enum JobEvent {
    Log(LogLine),
    Clip(ClipResult),
    Finished { ok: bool, message: String },
}

pub struct Runner {
    pub running: bool,
    pub active_label: String,
    pub logs: Vec<LogLine>,
    pub clips: Vec<ClipResult>,
    pub status: String,
    rx: Option<Receiver<JobEvent>>,
    child: Arc<Mutex<Option<Child>>>,
    cancelled: Arc<AtomicBool>,
}

impl Default for Runner {
    fn default() -> Self {
        Self {
            running: false,
            active_label: String::new(),
            logs: Vec::new(),
            clips: Vec::new(),
            status: "Hazır".into(),
            rx: None,
            child: Arc::new(Mutex::new(None)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl Runner {
    pub fn poll(&mut self) -> Option<String> {
        let rx = self.rx.as_ref()?;
        let mut finished = false;
        let mut completed_ok = None;
        while let Ok(ev) = rx.try_recv() {
            match ev {
                JobEvent::Log(line) => {
                    self.logs.push(line);
                    if self.logs.len() > 1200 {
                        let drop_n = self.logs.len() - 1200;
                        self.logs.drain(..drop_n);
                    }
                }
                JobEvent::Clip(clip) => {
                    self.clips.push(clip);
                }
                JobEvent::Finished { ok, message } => {
                    self.running = false;
                    if ok {
                        self.status = format!("Tamamlandı: {message}");
                        completed_ok = Some(self.active_label.clone());
                    } else {
                        self.status = format!("Hata / Durduruldu: {message}");
                    }
                    finished = true;
                }
            }
        }
        if finished {
            self.rx = None;
            if let Ok(mut guard) = self.child.lock() {
                *guard = None;
            }
        }
        completed_ok
    }

    pub fn start(&mut self, ctx: &egui::Context, label: &str, args: Vec<String>, clear_clips: bool) {
        if self.running {
            return;
        }
        self.running = true;
        self.active_label = label.to_string();
        self.status = format!("Çalışıyor: {label}");
        self.logs.clear();
        if clear_clips {
            self.clips.clear();
        }
        self.cancelled.store(false, Ordering::SeqCst);

        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);
        let child_arc = Arc::clone(&self.child);
        let cancel_arc = Arc::clone(&self.cancelled);
        let ctx = ctx.clone();

        std::thread::spawn(move || {
            let (program, full_args) = resolve_edits_cmd(&args);
            let _ = tx.send(JobEvent::Log(LogLine {
                is_err: false,
                text: format!("> {} {}", program, format_args_preview(&full_args)),
            }));
            ctx.request_repaint();

            let mut cmd = Command::new(&program);
            cmd.args(&full_args)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            hide_console_window(&mut cmd);

            let spawned = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(JobEvent::Finished {
                        ok: false,
                        message: format!("{program} başlatılamadı: {e}"),
                    });
                    ctx.request_repaint();
                    return;
                }
            };

            {
                if let Ok(mut guard) = child_arc.lock() {
                    *guard = Some(spawned);
                }
            }

            let stdout = child_arc.lock().ok().and_then(|mut g| g.as_mut().and_then(|c| c.stdout.take()));
            let stderr = child_arc.lock().ok().and_then(|mut g| g.as_mut().and_then(|c| c.stderr.take()));

            let tx_out = tx.clone();
            let ctx_out = ctx.clone();
            let h_out = std::thread::spawn(move || {
                if let Some(out) = stdout {
                    for line in BufReader::new(out).lines().map_while(Result::ok) {
                        if let Some(clip) = parse_clip_json(&line) {
                            let _ = tx_out.send(JobEvent::Clip(clip));
                        }
                        let _ = tx_out.send(JobEvent::Log(LogLine { is_err: false, text: line }));
                        ctx_out.request_repaint();
                    }
                }
            });

            let tx_err = tx.clone();
            let ctx_err = ctx.clone();
            let h_err = std::thread::spawn(move || {
                if let Some(err) = stderr {
                    for line in BufReader::new(err).lines().map_while(Result::ok) {
                        let _ = tx_err.send(JobEvent::Log(LogLine { is_err: true, text: line }));
                        ctx_err.request_repaint();
                    }
                }
            });

            let status = loop {
                if cancel_arc.load(Ordering::SeqCst) {
                    if let Ok(mut guard) = child_arc.lock() {
                        if let Some(c) = guard.as_mut() {
                            let _ = c.kill();
                        }
                    }
                }
                let done = {
                    let mut guard = child_arc.lock().ok();
                    guard.as_mut().and_then(|g| g.as_mut()).and_then(|c| c.try_wait().ok().flatten())
                };
                if let Some(st) = done {
                    break Ok(st);
                }
                std::thread::sleep(std::time::Duration::from_millis(80));
            };

            let _ = h_out.join();
            let _ = h_err.join();

            let was_cancelled = cancel_arc.load(Ordering::SeqCst);
            let (ok, message) = match status {
                _ if was_cancelled => (false, "Kullanıcı tarafından durduruldu".into()),
                Ok(st) if st.success() => (true, "Başarılı".into()),
                Ok(st) => (false, format!("Çıkış kodu: {}", st.code().unwrap_or(-1))),
                Err(e) => (false, e),
            };
            let _ = tx.send(JobEvent::Finished { ok, message });
            ctx.request_repaint();
        });
    }

    pub fn cancel(&mut self) {
        if self.running {
            self.cancelled.store(true, Ordering::SeqCst);
            if let Ok(mut guard) = self.child.lock() {
                if let Some(c) = guard.as_mut() {
                    let _ = c.kill();
                }
            }
            self.status = "Durduruluyor...".into();
        }
    }
}

fn parse_clip_json(line: &str) -> Option<ClipResult> {
    let trimmed = line.trim();
    if !trimmed.starts_with('{') {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let clip = v.get("clip")?.as_str()?;
    let mut parts = Vec::new();
    if let Some(p) = v.get("preset").and_then(|x| x.as_str()) {
        parts.push(format!("preset: {p}"));
    }
    if let Some(s) = v.get("song").and_then(|x| x.as_str()) {
        parts.push(format!("müzik: {s}"));
    }
    if let Some(sec) = v.get("seconds").and_then(|x| x.as_f64()) {
        parts.push(format!("{sec:.1}s"));
    }
    if let Some(src) = v.get("source").and_then(|x| x.as_str()) {
        parts.push(src.to_string());
    }
    Some(ClipResult {
        clip_path: PathBuf::from(clip),
        summary: parts.join(" · "),
        raw_json: serde_json::to_string_pretty(&v).unwrap_or_else(|_| trimmed.to_string()),
    })
}

pub fn resolve_edits_cmd(args: &[String]) -> (String, Vec<String>) {
    let exe_name = if cfg!(windows) { "edits.exe" } else { "edits" };
    if let Ok(cur) = std::env::current_exe() {
        if let Some(dir) = cur.parent() {
            let sibling = dir.join(exe_name);
            if sibling.is_file() {
                return (sibling.to_string_lossy().into_owned(), args.to_vec());
            }
        }
    }
    if edits::util::have("edits") {
        return ("edits".to_string(), args.to_vec());
    }
    let mut cargo_args = vec!["run".to_string(), "--bin".to_string(), "edits".to_string(), "--".to_string()];
    cargo_args.extend(args.iter().cloned());
    ("cargo".to_string(), cargo_args)
}

pub fn format_args_preview(args: &[String]) -> String {
    args.iter()
        .map(|a| {
            if a.is_empty() || a.contains([' ', '"', '\'', '&', '|']) {
                format!("\"{}\"", a.replace('"', "\\\""))
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(windows)]
pub fn hide_console_window(cmd: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub fn hide_console_window(_cmd: &mut Command) {}

pub fn open_in_os(path: &Path) {
    if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "start", "", &path.to_string_lossy()]);
        hide_console_window(&mut cmd);
        let _ = cmd.spawn();
    } else if cfg!(target_os = "macos") {
        let _ = Command::new("open").arg(path).spawn();
    } else {
        let _ = Command::new("xdg-open").arg(path).spawn();
    }
}
