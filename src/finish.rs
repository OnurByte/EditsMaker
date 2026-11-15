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
}

impl Finish {
    pub fn wanted(&self) -> bool {
        self.stabilize || self.denoise || self.smooth || self.fps.is_some() || !matches!(self.codec.as_str(), "" | "h264") || !matches!(self.quality.as_str(), "" | "high")
    }
}

pub const CODECS: &[&str] = &["h264", "h265", "prores", "av1"];
pub const QUALITIES: &[&str] = &["draft", "high", "master"];

/// Encoder arguments and the file extension for a codec at a quality.
pub fn encoder(codec: &str, quality: &str) -> (Vec<String>, &'static str) {
    let q = |d: u32, h: u32, m: u32| match quality { "draft" => d, "master" => m, _ => h }.to_string();
    let nv = crate::util::nvenc();
    let v: Vec<&str> = match codec {
        "h265" if nv => vec!["-c:v", "hevc_nvenc", "-tag:v", "hvc1", "-preset", "p5", "-rc", "vbr", "-b:v", "0", "-cq"],
        "av1" if nv => vec!["-c:v", "av1_nvenc", "-preset", "p5", "-rc", "vbr", "-b:v", "0", "-cq"],
        "h264" | "" if nv => vec!["-c:v", "h264_nvenc", "-preset", if quality == "master" { "p7" } else { "p5" }, "-rc", "vbr", "-b:v", "0", "-cq"],
        "h265" => vec!["-c:v", "libx265", "-tag:v", "hvc1", "-preset", "medium", "-crf"],
        "prores" => vec!["-c:v", "prores_ks", "-profile:v"],
        "av1" => vec!["-c:v", "libsvtav1", "-preset", "6", "-crf"],
        _ => vec!["-c:v", "libx264", "-preset", if quality == "master" { "slow" } else { "medium" }, "-crf"],
    };
    let mut a: Vec<String> = v.into_iter().map(String::from).collect();
    a.push(match codec {
        "h265" => q(30, 23, 18),
        "prores" => q(0, 2, 3), // proxy, standard, HQ
        "av1" => q(40, 30, 22),
        _ => q(28, 20, 15),
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
    match (f.smooth, f.fps) {
        (true, fps) => v.push(format!("minterpolate=fps={}:mi_mode=mci:mc_mode=aobmc:vsbmc=1", fps.unwrap_or(60))),
        (false, Some(fps)) => v.push(format!("fps={fps}")),
        _ => {}
    }
    v.join(",")
}

pub fn audio_chain(f: &Finish) -> String {
    if f.denoise { "afftdn=nf=-25,dialoguenhance,loudnorm=I=-14:TP=-1.5".into() } else { String::new() }
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
        assert!(audio_chain(&f).starts_with("afftdn"));
        let (a, ext) = encoder("prores", "master");
        assert_eq!((a.join(" "), ext), ("-c:v prores_ks -profile:v 3".to_string(), "mov"));
        assert_eq!(encoder("h264", "draft").0.join(" "), "-c:v libx264 -preset medium -crf 28 -pix_fmt yuv420p");
        assert!(encoder("h265", "high").0.contains(&"hvc1".to_string()));
    }
}
