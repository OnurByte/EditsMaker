use crate::util::run;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct Source {
    pub video: PathBuf,
    pub id: String,
    pub title: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    /// YouTube "most replayed": (start, end, value 0..1)
    pub heat: Vec<(f64, f64, f64)>,
    /// (text, likes)
    pub comments: Vec<(String, u64)>,
}

pub fn duration(video: &Path) -> Result<f64, String> {
    let v = video.to_string_lossy();
    let out = run(None, "ffprobe", &["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", v.as_ref()])?;
    out.trim().parse().map_err(|_| format!("ffprobe gave no duration for {v}"))
}

/// Frame size as displayed (0, 0 if ffprobe cannot tell).
pub fn dims(video: &Path) -> (u32, u32) {
    let v = video.to_string_lossy();
    let out = run(None, "ffprobe", &["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0", v.as_ref()]).unwrap_or_default();
    let mut it = out.trim().split(',').filter_map(|x| x.parse().ok());
    (it.next().unwrap_or(0), it.next().unwrap_or(0))
}

/// A local file is used as is; a URL is downloaded; anything else is a YouTube search for `videos` results.
pub fn fetch(input: &str, videos: usize, work: &Path) -> Result<Vec<Source>, String> {
    let local = Path::new(input);
    if local.is_file() {
        let id = local.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "video".into());
        let (width, height) = dims(local);
        return Ok(vec![Source { video: local.to_path_buf(), title: id.clone(), id, duration: duration(local)?, width, height, heat: vec![], comments: vec![] }]);
    }
    let is_url = input.starts_with("http://") || input.starts_with("https://");
    let target = if is_url { input.to_string() } else { format!("ytsearch{videos}:{input}") };
    let tmpl = work.join("%(id)s").join("src.%(ext)s");
    let mut args: Vec<String> = [
        "--no-warnings", "--no-playlist", "--no-progress",
        "-f", "bv*[height<=1080][ext=mp4]+ba[ext=m4a]/b[height<=1080]/b",
        "--merge-output-format", "mp4",
        "--write-info-json", "--write-comments",
        "--extractor-args", "youtube:max_comments=300,all,0,0;comment_sort=top",
        "--print", "after_move:filepath",
    ].iter().map(|s| s.to_string()).collect();
    if !is_url {
        // a topic search must not pull a 3-hour stream
        args.extend(["--match-filter".to_string(), "duration<1800".to_string()]);
    }
    // "--" so a topic starting with "-" can never be read as a yt-dlp option
    args.extend(["-o".to_string(), tmpl.to_string_lossy().into_owned(), "--".to_string(), target]);
    let out = run(None, "yt-dlp", &args)?;
    let mut sources = vec![];
    for path in out.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let video = PathBuf::from(path);
        let info: Value = std::fs::read_to_string(video.with_extension("info.json"))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null);
        let (width, height) = dims(&video);
        sources.push(Source {
            width,
            height,
            id: info["id"].as_str().unwrap_or("video").to_string(),
            title: info["title"].as_str().unwrap_or("").to_string(),
            duration: match info["duration"].as_f64() { Some(d) => d, None => duration(&video)? },
            heat: info["heatmap"].as_array().map(|a| a.iter().filter_map(|h| {
                Some((h["start_time"].as_f64()?, h["end_time"].as_f64()?, h["value"].as_f64()?))
            }).collect()).unwrap_or_default(),
            comments: info["comments"].as_array().map(|a| a.iter().filter_map(|c| {
                Some((c["text"].as_str()?.to_string(), c["like_count"].as_u64().unwrap_or(0)))
            }).collect()).unwrap_or_default(),
            video,
        });
    }
    if sources.is_empty() {
        return Err(format!("yt-dlp downloaded nothing for \"{input}\""));
    }
    Ok(sources)
}
