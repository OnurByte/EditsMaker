//! 3D scenes: a pack's scene.js, drawn with three.js, laid over an edit (`--overlay ID`). The script is one
//! function, `(THREE, scene, camera, opts) => (t) => { ... }`: it fills the scene once, then moves it to `t` seconds
//! for each frame. It runs alone, like a source's program: in a browser of its own with no window, a throwaway
//! profile, a blank page and nothing of the person's (no files, no sign-ins); only three.js is loaded for it. An added
//! scene runs only after `edits packs trust ID`. Its frames keep their transparency, so only what it draws is laid on.
use crate::browser::Browser;
use crate::packs::{self, Pack};
use std::path::PathBuf;
use std::time::Duration;

/// three.js, pinned, as an ES module.
const THREE: &str = "https://cdn.jsdelivr.net/npm/three@0.170.0/build/three.module.min.js";

/// A scene pack by its id or the last part of it (edits.stardust: stardust).
pub fn find(what: &str) -> Option<Pack> {
    let want = what.to_lowercase();
    packs::all().into_iter().find(|p| p.scene.is_some() && (p.manifest.id == want || p.manifest.id.rsplit('.').next() == Some(want.as_str())))
}

/// The page script that loads three.js, sets up a transparent renderer `w`x`h`, and makes `window.__draw(t)` return
/// the frame at `t` as a PNG data URL.
pub fn page_script(scene_js: &str, w: u32, h: u32) -> String {
    format!(r#"(async () => {{
  const THREE = await import("{THREE}");
  const canvas = document.createElement("canvas");
  document.body.style.margin = "0";
  document.body.appendChild(canvas);
  const renderer = new THREE.WebGLRenderer({{ canvas, alpha: true, antialias: true, preserveDrawingBuffer: true }});
  renderer.setPixelRatio(1);
  renderer.setSize({w}, {h}, false);
  renderer.setClearColor(0x000000, 0);
  const scene = new THREE.Scene();
  const camera = new THREE.PerspectiveCamera(45, {w} / {h}, 0.1, 1000);
  camera.position.set(0, 0, 6);
  const make = ({scene_js});
  const step = make(THREE, scene, camera, {{ width: {w}, height: {h} }});
  window.__draw = (t) => {{ step(t); renderer.render(scene, camera); return canvas.toDataURL("image/png"); }};
  return 1;
}})()"#)
}

/// The scene drawn `w`x`h` for its length, as a .mov with transparency (cached by its version and size).
pub fn render(p: &Pack, w: u32, h: u32) -> Result<PathBuf, String> {
    let m = &p.manifest;
    let (js, sc) = (p.scene.as_deref().ok_or(format!("{} has no scene.js", m.id))?, m.scene.as_ref().ok_or(format!("{} has no scene block", m.id))?);
    if !packs::trusted(p) {
        return Err(format!("{0} runs a script: read {1}, then edits packs trust {0}", m.id, packs::dir().join(&m.id).join("scene.js").display()));
    }
    let cache = crate::util::home().join(".cache/edits/scenes");
    let out = cache.join(format!("{}-{}-{w}x{h}-{:08x}.mov", m.id, m.version, crate::util::crc32(js.as_bytes())));
    if out.is_file() {
        return Ok(out);
    }
    let work = cache.join(format!("work-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(work.join("frames")).map_err(|e| e.to_string())?;
    let res = (|| {
        frames(m, js, (sc.seconds * sc.fps as f64).round() as usize, sc.fps, w, h, &work)?;
        let tmp = work.join("scene.mov");
        // qtrle keeps the alpha, and every ffmpeg reads it back
        crate::util::run(None, "ffmpeg", &["-v", "error", "-y", "-framerate", &sc.fps.to_string(), "-i", &work.join("frames/%05d.png").to_string_lossy(),
            "-c:v", "qtrle", "-pix_fmt", "argb", &tmp.to_string_lossy()])?;
        std::fs::rename(&tmp, &out).map_err(|e| e.to_string())?;
        Ok(out.clone())
    })();
    let _ = std::fs::remove_dir_all(&work);
    res
}

/// Draws `n` frames of a scene in a windowless browser of its own, as WORK/frames/00000.png...
fn frames(m: &packs::Manifest, js: &str, n: usize, fps: u32, w: u32, h: u32, work: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(work.join("frames")).map_err(|e| e.to_string())?;
    let mut b = Browser::headless(&work.join("profile"))?;
    let drawn = (|| {
        b.call("Emulation.setDeviceMetricsOverride", serde_json::json!({"width": w, "height": h, "deviceScaleFactor": 1, "mobile": false}), Duration::from_secs(10))?;
        b.eval(&page_script(js, w, h), Duration::from_secs(60)).map_err(|e| format!("{}: scene.js did not start: {e}", m.id))?;
        for i in 0..n {
            let url = b.eval(&format!("window.__draw({})", i as f64 / fps as f64), Duration::from_secs(30))?;
            let png = crate::util::base64_decode(url.as_str().and_then(|u| u.split_once(',')).map_or("", |x| x.1));
            if png.len() < 8 {
                return Err(format!("{}: frame {i} came back empty", m.id));
            }
            std::fs::write(work.join("frames").join(format!("{i:05}.png")), png).map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    b.close();
    drawn
}

#[cfg(test)]
mod tests {
    /// The browser half of a render, for real: Chromium, three.js from the network, Stardust's frames with alpha.
    /// `cargo test -- --ignored scene_draws` (no ffmpeg involved).
    #[test]
    #[ignore]
    fn scene_draws_in_a_real_browser() {
        let p = super::find("stardust").unwrap();
        let work = std::env::temp_dir().join(format!("edits-scene-{}", std::process::id()));
        super::frames(&p.manifest, p.scene.as_deref().unwrap(), 3, 30, 270, 480, &work).unwrap();
        for i in 0..3 {
            let png = std::fs::read(work.join(format!("frames/{i:05}.png"))).unwrap();
            assert!(png.starts_with(b"\x89PNG") && png.len() > 2000, "frame {i}: {} bytes", png.len());
            // colour type 6: RGBA, so only what it draws covers the edit
            assert_eq!(png[25], 6, "frame {i} keeps its transparency");
        }
        println!("{}", work.display());
        if std::env::var("KEEP").is_err() {
            let _ = std::fs::remove_dir_all(&work);
        }
    }

    #[test]
    fn the_page_loads_three_and_draws_with_alpha() {
        let s = super::page_script("(THREE, scene, camera) => (t) => {}", 1080, 1920);
        assert!(s.contains("three@0.170.0") && s.contains("alpha: true") && s.contains("setSize(1080, 1920"));
        assert!(super::find("stardust").is_some_and(|p| p.scene.is_some()), "the built-in scene is found by its short name");
        assert!(super::find("punch-whip").is_none());
    }
}
