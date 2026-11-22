//! The last pass over a finished clip, what an editor's export dialog and clean-up tools do: stabilise shaky
//! footage, clean the voice, smooth slow motion, and deliver at the frame rate, codec and quality asked for.
use crate::util::run;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default)]
pub struct Finish {
    pub stabilize: bool,
    /// noise reduction and dialogue lift
    pub denoise: bool,
    /// motion-interpolated frames instead of repeated ones (with `fps`, or to 60)
    pub smooth: bool,
    pub fps: Option<u32>,
    /// h264 (default), h265, prores, av1
    pub codec: String,
    /// draft, high (default), master
    pub quality: String,
    /// a tone curve over the whole edit (`fx::curves`)
    pub curves: String,
    /// colour pop: only this colour stays
    pub pop: String,
    /// low end boost in dB (phonk bass), 0 = off
    pub bass: f64,
    /// semitones up or down, tempo kept (so the cuts stay on the beat)
    pub pitch: f64,
    /// a room behind the sound (slowed + reverb, sad edits)
    pub reverb: bool,
}

impl Finish {
    pub fn wanted(&self) -> bool {
        self.stabilize || self.denoise || self.smooth || self.fps.is_some() || !self.curves.is_empty() || !self.pop.is_empty() || self.bass != 0.0 || self.pitch != 0.0 || self.reverb || !matches!(self.codec.as_str(), "" | "h264") || !matches!(self.quality.as_str(), "" | "high")
    }
}

pub const CODECS: &[&str] = &["h264", "h265", "prores", "av1"];
pub const QUALITIES: &[&str] = &["draft", "high", "master"];

/// Encoder arguments and the file extension for a codec at a quality.
pub fn encoder(codec: &str, quality: &str) -> (Vec<String>, &'static str) {
    let q = |d: u32, h: u32, m: u32| match quality { "draft" => d, "master" => m, _ => h };
    let (nv, vt) = (crate::util::gpu() == "nvenc", crate::util::gpu() == "videotoolbox");
    let v: Vec<&str> = match codec {
        "h265" if nv => vec!["-c:v", "hevc_nvenc", "-tag:v", "hvc1", "-preset", "p5", "-rc", "vbr", "-b:v", "0", "-cq"],
        "av1" if nv => vec!["-c:v", "av1_nvenc", "-preset", "p5", "-rc", "vbr", "-b:v", "0", "-cq"],
        "h264" | "" if nv => vec!["-c:v", "h264_nvenc", "-preset", if quality == "master" { "p7" } else { "p5" }, "-rc", "vbr", "-b:v", "0", "-cq"],
        "h265" if vt => vec!["-c:v", "hevc_videotoolbox", "-tag:v", "hvc1", "-b:v"],
        "h264" | "" if vt => vec!["-c:v", "h264_videotoolbox", "-profile:v", "high", "-b:v"],
        "h265" => vec!["-c:v", "libx265", "-tag:v", "hvc1", "-preset", "medium", "-crf"],
        "prores" => vec!["-c:v", "prores_ks", "-profile:v"],
        "av1" => vec!["-c:v", "libsvtav1", "-preset", "6", "-crf"],
        _ => vec!["-c:v", "libx264", "-preset", if quality == "master" { "slow" } else { "medium" }, "-crf"],
    };
    let mut a: Vec<String> = v.into_iter().map(String::from).collect();
    let h264 = q(28, 20, 15);
    a.push(match codec {
        // VideoToolbox takes a bitrate: the one this quality's x264 crf stands for; HEVC needs about two thirds of it
        "h265" if vt => crate::util::crf_bitrate(h264 as f64 + 6.0 * 1.5f64.log2()),
        "h264" | "" if vt => crate::util::crf_bitrate(h264 as f64),
        "h265" => q(30, 23, 18).to_string(),
        "prores" => q(0, 2, 3).to_string(), // proxy, standard, HQ
        "av1" => q(40, 30, 22).to_string(),
        _ => h264.to_string(),
    });
    if codec != "prores" {
        a.extend(["-pix_fmt".into(), "yuv420p".into()]);
    }
    (a, if codec == "prores" { "mov" } else { "mp4" })
}

/// The video filter chain for the pass (after stabilisation analysis), empty when nothing changes the picture.
pub fn video_chain(f: &Finish, trf: Option<&str>) -> String {
    let mut v: Vec<String> = vec![];
    if let Some(t) = trf {
        v.push(format!("vidstabtransform=input={t}:smoothing=12:zoom=3:optzoom=0,unsharp=5:5:0.6"));
    }
    // colour after stabilising, before new frames are made from it
    let colour = format!("{}{}", crate::fx::curves(&f.curves).unwrap_or_default(), crate::fx::pop(&f.pop).unwrap_or_default());
    if !colour.is_empty() {
        v.push(colour[1..].to_string());
    }
    match (f.smooth, f.fps) {
        (true, fps) => v.push(format!("minterpolate=fps={}:mi_mode=mci:mc_mode=aobmc:vsbmc=1", fps.unwrap_or(60))),
        (false, Some(fps)) => v.push(format!("fps={fps}")),
        _ => {}
    }
    v.join(",")
}

pub fn audio_chain(f: &Finish) -> String {
    let mut a: Vec<String> = vec![];
    if f.denoise {
        a.push("afftdn=nf=-25,dialoguenhance".into());
    }
    if f.bass != 0.0 {
        a.push(format!("bass=g={:.1}:f=110:w=0.6", f.bass.clamp(-12.0, 15.0)));
    }
    if f.pitch != 0.0 {
        // faster sample rate = higher voice and song, atempo takes the speed back out
        let r = 2f64.powf(f.pitch.clamp(-12.0, 12.0) / 12.0);
        a.push(format!("aresample=48000,asetrate={:.0},aresample=48000,atempo={:.5}", 48000.0 * r, 1.0 / r));
    }
    if f.reverb {
        a.push("aecho=0.8:0.88:60|120|200:0.3|0.22|0.15".into());
    }
    if !a.is_empty() {
        // a boost or a room changes loudness: back to the platforms' level, never clipping
        a.push("loudnorm=I=-14:TP=-1.5".into());
    }
    a.join(",")
}

/// A voice line cleaned before it meets the music: its stretch of `v.file` (half a second either side) through
/// DeepFilterNet, which keeps the speech and drops the noise and score under it; ffmpeg's afftdn when DeepFilterNet
/// cannot run here. The music itself is never touched. Falls back to the voice as it was.
pub fn clean_voice(v: crate::montage::Voice, work: &Path) -> crate::montage::Voice {
    let pad = v.start.min(0.5);
    let dir = work.join("clean");
    let raw = dir.join(format!("voice-{:.2}-{:.2}.wav", v.start, v.len));
    let done = dir.join("out").join(raw.file_name().unwrap_or_default());
    let res = (|| {
        if done.is_file() {
            return Ok(());
        }
        std::fs::create_dir_all(dir.join("out")).map_err(|e| e.to_string())?;
        let src = v.file.to_string_lossy();
        let cut = |af: &str, to: &Path| run(None, "ffmpeg", &["-v", "error", "-y", "-ss", &format!("{:.3}", v.start - pad), "-t", &format!("{:.3}", v.len + pad + 0.5),
            "-i", src.as_ref(), "-vn", "-ac", "1", "-ar", "48000", "-af", af, &to.to_string_lossy()]).map(|_| ());
        cut("anull", &raw)?;
        match crate::tools::deep_filter() {
            // -D: no delay added, so the words stay where they were
            Ok(df) => run(None, &df, &["-D", "-o", &dir.join("out").to_string_lossy(), &raw.to_string_lossy()]).map(|_| ()),
            Err(e) => {
                eprintln!("edits: {e}; the voice is cleaned with ffmpeg instead");
                cut("afftdn=nf=-25,dialoguenhance", &done)
            }
        }
    })();
    match res {
        Ok(()) if done.is_file() => crate::montage::Voice { file: done, start: pad, ..v },
        other => {
            eprintln!("edits: could not clean the voice ({}); it keeps its noise", other.err().unwrap_or_else(|| "no output".into()));
            v
        }
    }
}

/// Runs the pass over `clip` in place (a ProRes delivery becomes a .mov beside it). Returns the final file.
pub fn apply(clip: &Path, f: &Finish) -> Result<PathBuf, String> {
    if !f.wanted() {
        return Ok(clip.to_path_buf());
    }
    let dir = clip.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let name = clip.file_name().unwrap_or_default().to_string_lossy().into_owned();
    let trf = format!("{name}.trf");
    if f.stabilize {
        run(Some(dir), "ffmpeg", &["-v", "error", "-y", "-i", &name, "-vf", &format!("vidstabdetect=shakiness=6:accuracy=12:result={trf}"), "-f", "null", "-"])?;
    }
    let (enc, ext) = encoder(&f.codec, &f.quality);
    let tmp = format!("{name}.finish.{ext}");
    let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into(), "-i".into(), name.clone()];
    let vf = video_chain(f, f.stabilize.then_some(trf.as_str()));
    if !vf.is_empty() {
        args.extend(["-vf".into(), vf]);
    }
    let af = audio_chain(f);
    if !af.is_empty() {
        args.extend(["-af".into(), af]);
    }
    args.extend(enc);
    args.extend(["-c:a", if ext == "mov" { "pcm_s16le" } else { "aac" }, "-b:a", "192k", "-movflags", "+faststart"].iter().map(|s| s.to_string()));
    args.push(tmp.clone());
    let res = run(Some(dir), "ffmpeg", &args);
    let _ = std::fs::remove_file(dir.join(&trf));
    res?;
    let fin = clip.with_extension(ext);
    std::fs::rename(dir.join(&tmp), &fin).map_err(|e| e.to_string())?;
    if fin != clip {
        let _ = std::fs::remove_file(clip);
    }
    Ok(fin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_asked_nothing_done() {
        assert!(!Finish::default().wanted());
        assert!(!Finish { codec: "h264".into(), quality: "high".into(), ..Default::default() }.wanted());
        assert_eq!(apply(Path::new("/nonexistent.mp4"), &Finish::default()).unwrap(), PathBuf::from("/nonexistent.mp4"));
    }

    #[test]
    fn chains_and_encoders() {
        let f = Finish { stabilize: true, smooth: true, fps: Some(60), denoise: true, ..Default::default() };
        assert_eq!(video_chain(&f, Some("x.trf")), "vidstabtransform=input=x.trf:smoothing=12:zoom=3:optzoom=0,unsharp=5:5:0.6,minterpolate=fps=60:mi_mode=mci:mc_mode=aobmc:vsbmc=1");
        assert_eq!(video_chain(&Finish { fps: Some(24), ..Default::default() }, None), "fps=24");
        assert!(audio_chain(&f).starts_with("afftdn") && audio_chain(&f).ends_with("loudnorm=I=-14:TP=-1.5"));
        assert_eq!(audio_chain(&Finish::default()), "");
        let fx = Finish { bass: 6.0, pitch: -2.0, reverb: true, curves: "darker".into(), pop: "red".into(), ..Default::default() };
        assert!(fx.wanted());
        assert_eq!(audio_chain(&fx), "bass=g=6.0:f=110:w=0.6,aresample=48000,asetrate=42763,aresample=48000,atempo=1.12246,aecho=0.8:0.88:60|120|200:0.3|0.22|0.15,loudnorm=I=-14:TP=-1.5");
        assert_eq!(video_chain(&fx, None), "curves=preset=darker,huesaturation=colors=y+g+c+b+m:saturation=-1:strength=100");
        let (a, ext) = encoder("prores", "master");
        assert_eq!((a.join(" "), ext), ("-c:v prores_ks -profile:v 3".to_string(), "mov"));
        assert_eq!(crate::mcp::args_for("edits_encoder", &serde_json::json!({"use": "cpu"})).unwrap(), ["encoder", "cpu"]);
        // the encoder is the GPU's when this machine has a working one (auto), else x264
        let h = encoder("h264", "draft").0.join(" ");
        assert_eq!(h, match crate::util::gpu() {
            "nvenc" => "-c:v h264_nvenc -preset p5 -rc vbr -b:v 0 -cq 28 -pix_fmt yuv420p",
            "videotoolbox" => "-c:v h264_videotoolbox -profile:v high -b:v 3780k -pix_fmt yuv420p",
            _ => "-c:v libx264 -preset medium -crf 28 -pix_fmt yuv420p",
        });
        assert!(encoder("h265", "high").0.contains(&"hvc1".to_string()));
        // VideoToolbox's bitrate follows x264's quality ladder: crf 18 = 12 Mb/s, 6 steps halve it
        assert_eq!((crate::util::crf_bitrate(18.0), crate::util::crf_bitrate(24.0), crate::util::crf_bitrate(12.0)), ("12000k".into(), "6000k".into(), "24000k".into()));
        assert!(crate::util::set_encoder("metal").is_err());
    }
}
