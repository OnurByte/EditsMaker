//! The editor's video player. No GPUI player fits gpui-fast (alhazen-gpui and gpui-video-player are built on upstream
//! gpui, and alhazen-core's AV1 decoder needs nasm to build), so it plays on the ffmpeg edits already needs: one
//! ffmpeg decodes the picture, scaled to what the window shows, as raw BGRA frames on a pipe, each shown when the
//! clock reaches it; the sound plays beside it (ffplay with no window, else ffmpeg's own sound output). Seeking starts
//! both again at the new time; paused, one frame is taken at the playhead.
use gpui_kit::*;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Frames a second the player shows; edits' clips are 30.
pub const FPS: f64 = 30.0;

/// The newest decoded frame: which start it belongs to, its time, its pixels.
type Latest = Arc<Mutex<Option<(u64, f64, Vec<u8>)>>>;

pub struct Player {
    pub clip: PathBuf,
    /// the frame size it decodes at
    pub w: u32,
    pub h: u32,
    pub total: f64,
    /// where the playhead was when it last started or stopped
    at: f64,
    /// when it started playing, None when paused
    since: Option<Instant>,
    pub muted: bool,
    /// trim points picked by hand, for `edits cut`
    pub mark_in: Option<f64>,
    pub mark_out: Option<f64>,
    latest: Latest,
    /// bumped at every start or seek: an older decoder stops by itself
    generation: Arc<AtomicU64>,
    shown: Option<(f64, Arc<RenderImage>)>,
    /// images painted before, to drop from the sprite atlas (else it grows by a frame a frame)
    retired: Vec<Arc<RenderImage>>,
    video: Option<Child>,
    audio: Option<Child>,
}

/// A frame size of at most `max` tall, even on both sides as ffmpeg's scaler wants, keeping the shape.
pub fn fit(w: u32, h: u32, max: u32) -> (u32, u32) {
    if w == 0 || h == 0 {
        return (max * 9 / 16 / 2 * 2, max / 2 * 2);
    }
    let s = (max as f64 / h as f64).min(1.0);
    (((w as f64 * s) as u32 / 2 * 2).max(2), ((h as f64 * s) as u32 / 2 * 2).max(2))
}

/// The ffmpeg arguments that decode the picture from `t` on, as BGRA frames of `w` x `h` at FPS; `one` takes one.
pub fn video_args(clip: &Path, t: f64, w: u32, h: u32, one: bool) -> Vec<String> {
    let mut a: Vec<String> = ["-v", "error", "-nostdin", "-ss"].iter().map(|s| s.to_string()).collect();
    a.extend([format!("{t:.3}"), "-i".into(), clip.display().to_string(), "-an".into(), "-vf".into(), format!("scale={w}:{h},fps={FPS}"),
        "-pix_fmt".into(), "bgra".into(), "-f".into(), "rawvideo".into()]);
    if one {
        a.extend(["-frames:v".into(), "1".into()]);
    }
    a.push("-".into());
    a
}

fn have(tool: &str) -> bool {
    Command::new(tool).arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

/// The sound from `t` on, to the speakers.
fn play_sound(clip: &Path, t: f64) -> Option<Child> {
    let c = clip.display().to_string();
    let t = format!("{t:.3}");
    let quiet = |cmd: &mut Command| cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().ok();
    static FFPLAY: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *FFPLAY.get_or_init(|| have("ffplay")) {
        return quiet(Command::new("ffplay").args(["-v", "quiet", "-nodisp", "-autoexit", "-ss", &t, &c]));
    }
    // no ffplay: ffmpeg's own sound output, where the system has one
    let out: &[&str] = if cfg!(target_os = "macos") { &["-f", "audiotoolbox", "-"] } else if cfg!(target_os = "linux") { &["-f", "pulse", "edits"] } else { return None };
    quiet(Command::new("ffmpeg").args(["-v", "quiet", "-nostdin", "-re", "-ss", &t, "-i", &c, "-vn"]).args(out))
}

impl Player {
    pub fn open(clip: &Path, total: f64) -> Self {
        let (vw, vh) = crate::fetch::dims(clip);
        let (w, h) = fit(vw, vh, 960);
        let total = if total > 0.0 { total } else { crate::fetch::duration(clip).unwrap_or(0.0) };
        let mut p = Player { clip: clip.to_path_buf(), w, h, total, at: 0.0, since: None, muted: false, mark_in: None, mark_out: None,
            latest: Default::default(), generation: Default::default(), shown: None, retired: vec![], video: None, audio: None };
        p.decode(0.0, true);
        p
    }

    pub fn playing(&self) -> bool {
        self.since.is_some()
    }

    /// Where the playhead is now.
    pub fn position(&self) -> f64 {
        let t = self.at + self.since.map_or(0.0, |s| s.elapsed().as_secs_f64());
        t.min(self.total.max(0.0))
    }

    fn stop_children(&mut self) {
        for c in [self.video.take(), self.audio.take()].into_iter().flatten() {
            let mut c = c;
            let _ = c.kill();
            // reaped on a thread: a killed ffmpeg is gone at once, but wait() must still be called
            std::thread::spawn(move || c.wait());
        }
    }

    /// Starts a decoder at `t`: one frame when paused, else frames on the clock until the end.
    fn decode(&mut self, t: f64, one: bool) {
        let g = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        let Ok(mut child) = Command::new("ffmpeg").args(video_args(&self.clip, t, self.w, self.h, one)).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn() else { return };
        let Some(mut out) = child.stdout.take() else { return };
        self.video = Some(child);
        let (latest, generation, size, start) = (self.latest.clone(), self.generation.clone(), (self.w * self.h * 4) as usize, Instant::now());
        std::thread::spawn(move || {
            let mut n = 0u64;
            loop {
                let mut buf = vec![0u8; size];
                if out.read_exact(&mut buf).is_err() || generation.load(Ordering::SeqCst) != g {
                    break;
                }
                let due = Duration::from_secs_f64(n as f64 / FPS);
                // ahead of the clock: wait for it; behind it: show this one anyway, the next catches up
                if !one {
                    if let Some(wait) = due.checked_sub(start.elapsed()) {
                        std::thread::sleep(wait);
                    }
                }
                if generation.load(Ordering::SeqCst) != g {
                    break;
                }
                *latest.lock().unwrap() = Some((g, t + n as f64 / FPS, buf));
                n += 1;
            }
        });
    }

    pub fn play(&mut self) {
        if self.playing() {
            return;
        }
        if self.at >= self.total - 0.05 {
            self.at = 0.0;
        }
        self.stop_children();
        self.decode(self.at, false);
        if !self.muted {
            self.audio = play_sound(&self.clip, self.at);
        }
        self.since = Some(Instant::now());
    }

    pub fn pause(&mut self) {
        self.at = self.position();
        self.since = None;
        self.stop_children();
        self.generation.fetch_add(1, Ordering::SeqCst);
    }

    pub fn toggle(&mut self) {
        if self.playing() { self.pause() } else { self.play() }
    }

    pub fn seek(&mut self, t: f64) {
        let playing = self.playing();
        self.pause();
        self.at = t.clamp(0.0, self.total.max(0.0));
        if playing {
            self.play();
        } else {
            self.decode(self.at, true);
        }
    }

    pub fn set_muted(&mut self, m: bool) {
        self.muted = m;
        if self.playing() {
            let t = self.position();
            self.seek(t);
        }
    }

    /// Called each frame the window draws: ends at the end, and hands the newest frame to paint.
    pub fn frame(&mut self, window: &mut Window) -> Option<Arc<RenderImage>> {
        if self.playing() && self.position() >= self.total - 0.01 {
            self.pause();
            self.at = self.total;
        }
        for old in self.retired.drain(..) {
            let _ = window.drop_image(old);
        }
        let now = self.generation.load(Ordering::SeqCst);
        let fresh = self.latest.lock().unwrap().take();
        if let Some((g, t, bgra)) = fresh {
            // a frame from a decoder since stopped is still the right picture when it is the last one asked for
            if g + 1 >= now {
                if let Some(buf) = image::RgbaImage::from_raw(self.w, self.h, bgra) {
                    let img = Arc::new(RenderImage::new([image::Frame::new(buf)]));
                    if let Some((_, old)) = self.shown.replace((t, img)) {
                        self.retired.push(old);
                    }
                }
            }
        }
        self.shown.as_ref().map(|s| s.1.clone())
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.stop_children();
    }
}

/// "1:05.3" for a time in seconds.
pub fn clock(t: f64) -> String {
    let t = t.max(0.0);
    format!("{}:{:04.1}", (t / 60.0) as u64, t % 60.0)
}

#[cfg(test)]
mod tests {
    // not super::*: gpui_kit's own `test` attribute would come with it
    use super::{clock, fit, video_args};
    use std::path::Path;

    #[test]
    fn frames_keep_their_shape_and_stay_even() {
        assert_eq!(fit(1080, 1920, 960), (540, 960));
        assert_eq!(fit(1080, 1080, 960), (960, 960));
        assert_eq!(fit(641, 361, 960), (640, 360), "never scaled up, always even");
        assert_eq!(fit(0, 0, 960), (540, 960));
    }

    #[test]
    fn the_decoder_reads_from_the_playhead() {
        let a = video_args(Path::new("/v/a b.mp4"), 2.5, 540, 960, true);
        assert_eq!(&a[4..7], ["2.500", "-i", "/v/a b.mp4"]);
        assert!(a.contains(&"scale=540:960,fps=30".to_string()) && a.ends_with(&["-frames:v".into(), "1".into(), "-".into()]));
        assert_eq!(clock(65.3), "1:05.3");
    }
}
