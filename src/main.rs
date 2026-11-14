mod face;
mod fetch;
mod pick;
mod render;
mod signals;
mod study;
mod transcribe;
mod util;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = include_str!("usage.txt");

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let res = match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            print!("{USAGE}");
            Ok(())
        }
        Some("-V" | "--version") => {
            println!("edits {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("doctor") => doctor(),
        Some("rate") => rate(&args[1..]),
        Some("study") => match args.get(1) {
            Some(dir) => study::study(Path::new(dir)),
            None => Err("usage: edits study <folder of edits you like>".into()),
        },
        Some(_) => make(&args),
    };
    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("edits: {e}");
            ExitCode::FAILURE
        }
    }
}

struct Cli {
    input: String,
    n: usize,
    videos: usize,
    lang: String,
    out: PathBuf,
    opts: render::Opts,
    llm: bool,
    join: bool,
}

fn size(aspect: &str) -> Option<(u32, u32)> {
    study::ASPECTS.iter().find(|a| a.0 == aspect).map(|a| (a.1, a.2))
}

/// Defaults come from `edits study` when it has been run; flags override them.
fn parse(args: &[String], style: Option<study::Style>) -> Result<Cli, String> {
    let style = style.unwrap_or(study::Style { aspect: "9:16".into(), grade: "bw".into(), lufs: -14.0 });
    let (w, h) = size(&style.aspect).unwrap_or((1080, 1920));
    let grade = if render::GRADES.contains(&style.grade.as_str()) { style.grade } else { "bw".into() };
    let mut c = Cli {
        input: String::new(), n: 3, videos: 5, lang: "tr".into(), out: PathBuf::from("edits-out"),
        opts: render::Opts { w, h, grade, captions: true, layout: "fill".into(), lufs: style.lufs.clamp(-30.0, -5.0) },
        llm: true, join: false,
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = |name: &str| it.next().cloned().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "-n" | "--moments" => c.n = val("-n")?.parse().ok().filter(|n| (1..=20).contains(n)).ok_or("-n must be 1-20")?,
            "--videos" => c.videos = val("--videos")?.parse().ok().filter(|n| (1..=10).contains(n)).ok_or("--videos must be 1-10")?,
            "--lang" => c.lang = val("--lang")?,
            "-o" | "--out" => c.out = PathBuf::from(val("--out")?),
            "--grade" => {
                c.opts.grade = val("--grade")?;
                if !render::GRADES.contains(&c.opts.grade.as_str()) {
                    return Err(format!("--grade must be one of: {}", render::GRADES.join(", ")));
                }
            }
            "--aspect" => (c.opts.w, c.opts.h) = size(&val("--aspect")?).ok_or("--aspect must be 9:16, 1:1, 4:3 or 16:9")?,
            "--fit" => c.opts.layout = "fit".into(),
            "--no-captions" => c.opts.captions = false,
            "--no-llm" => c.llm = false,
            "--join" => c.join = true,
            s if s.starts_with('-') => return Err(format!("unknown option {s}; see `edits --help`")),
            s if c.input.is_empty() => c.input = s.to_string(),
            s => {
                // a topic typed without quotes
                c.input.push(' ');
                c.input.push_str(s);
            }
        }
    }
    if c.input.is_empty() {
        return Err("give a YouTube link, a topic or a video file".into());
    }
    Ok(c)
}

struct Analysed {
    words: Vec<transcribe::Word>,
    lines: Vec<transcribe::Line>,
    work: PathBuf,
    moments: Vec<pick::Moment>,
}

fn make(args: &[String]) -> Result<(), String> {
    let c = parse(args, study::load())?;
    let cache = util::home().join(".cache/edits");
    std::fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&c.out).map_err(|e| e.to_string())?;
    eprintln!("edits: fetching \"{}\"", c.input);
    let sources = fetch::fetch(&c.input, c.videos, &cache)?;
    let mut analysed = vec![];
    for src in &sources {
        let work = cache.join(safe(&src.id));
        std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let wav = work.join("a.wav");
        transcribe::wav(&src.video, &wav)?;
        eprintln!("edits: transcribing {} ({})", src.title, util::mmss(src.duration));
        // no speech model or no speech is fine: the cut then rests on the other signals, without captions
        let words = transcribe::words(&wav, &c.lang).unwrap_or_else(|e| {
            eprintln!("edits: no transcript ({e})");
            vec![]
        });
        let lines = transcribe::lines(&words);
        let mut hits = signals::comment_hits(&src.comments, src.duration);
        let laughs = transcribe::laughs(&wav);
        hits.extend(laughs.iter().map(|&t| signals::Hit { t: (t - 4.0).max(0.0), weight: 4.0, text: "(videoda kahkaha/alkış sesi)".into() }));
        let loud = signals::loudness(&wav).unwrap_or_default();
        let curve = signals::curve(src.duration, &src.heat, &hits, &loud);
        eprintln!("edits: picking moments ({} timestamped comments, {} laughs, replay heat: {})", hits.len() - laughs.len(), laughs.len(), if src.heat.is_empty() { "no" } else { "yes" });
        let moments = pick::pick(&src.title, &lines, &curve, &hits, c.n, src.duration, c.llm);
        analysed.push(Analysed { words, lines, work, moments });
    }
    // every video was analysed; only the best moments overall get rendered
    let mut best: Vec<(usize, &pick::Moment)> = analysed.iter().enumerate().flat_map(|(i, a)| a.moments.iter().map(move |m| (i, m))).collect();
    best.sort_by(|a, b| b.1.score.total_cmp(&a.1.score));
    best.truncate(c.n);
    let mut clips: Vec<PathBuf> = vec![];
    for (k, (i, m)) in best.iter().enumerate() {
        let (src, a) = (&sources[*i], &analysed[*i]);
        let out = c.out.join(format!("{:02}-{}.mp4", k + 1, safe(&src.id)));
        let focus = if c.opts.layout == "fill" { face::center_x(&src.video, m.start, m.end - m.start, src.width, src.height) } else { None };
        eprintln!("edits: rendering {} [{}-{}] {}{}", out.display(), util::mmss(m.start), util::mmss(m.end), m.title, if focus.is_some() { " (framed on faces)" } else { "" });
        render::render(&src.video, m, &a.words, &c.opts, focus.unwrap_or(0.5), &a.work, &out)?;
        let text = a.lines.iter().filter(|l| l.s >= m.start - 0.5 && l.e <= m.end + 0.5).map(|l| l.text.as_str()).collect::<Vec<_>>().join(" ");
        let side = serde_json::json!({ "clip": out, "source": src.title, "id": src.id, "moment": m, "text": text });
        std::fs::write(out.with_extension("json"), serde_json::to_string_pretty(&side).unwrap()).map_err(|e| e.to_string())?;
        println!("{side}");
        clips.push(out);
    }
    if clips.is_empty() {
        return Err("found nothing worth cutting".into());
    }
    if c.join && clips.len() > 1 {
        let out = c.out.join("joined.mp4");
        render::join(&clips, &cache, &out)?;
        eprintln!("edits: joined {} clips into {}", clips.len(), out.display());
    }
    Ok(())
}

fn safe(id: &str) -> String {
    id.chars().map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).take(60).collect()
}

/// `edits rate clip.mp4 +|-` appends the clip's text to the taste file as a liked or disliked example.
fn rate(args: &[String]) -> Result<(), String> {
    let (clip, mark) = match args {
        [clip, m] if m == "+" || m == "-" => (Path::new(clip), if m == "+" { "👍 böyle" } else { "👎 böyle değil" }),
        _ => return Err("usage: edits rate <clip.mp4> +|-".into()),
    };
    let side = std::fs::read_to_string(clip.with_extension("json")).map_err(|_| format!("no {} next to the clip", clip.with_extension("json").display()))?;
    let v: serde_json::Value = serde_json::from_str(&side).map_err(|e| e.to_string())?;
    let text = v["text"].as_str().unwrap_or("").trim();
    if text.is_empty() {
        return Err("that clip has no transcript to learn from".into());
    }
    let mut taste = pick::taste();
    taste.push_str(&format!("- {mark}: \"{text}\"\n"));
    std::fs::write(pick::taste_path(), taste).map_err(|e| e.to_string())?;
    println!("edits: noted in {}", pick::taste_path().display());
    Ok(())
}

fn doctor() -> Result<(), String> {
    let mut missing = 0;
    let mut check = |ok: bool, name: &str, fix: &str, required: bool| {
        println!("{} {name}{}", if ok { "ok  " } else if required { "MISS" } else { "warn" }, if ok { String::new() } else { format!(" -> {fix}") });
        if !ok && required {
            missing += 1;
        }
    };
    check(util::have("ffmpeg") && util::have("ffprobe"), "ffmpeg + ffprobe", "install ffmpeg", true);
    let libass = util::run(None, "ffmpeg", &["-hide_banner", "-filters"]).is_ok_and(|f| f.contains(" ass "));
    check(libass, "ffmpeg has libass (captions)", "install a full ffmpeg build", true);
    check(util::have("yt-dlp"), "yt-dlp (links and topics)", "install yt-dlp; local files work without it", false);
    check(util::have("whisper-cli"), "whisper-cli (captions, transcript)", "install whisper.cpp", false);
    let model = transcribe::model();
    check(model.is_some(), &format!("whisper model{}", model.map(|m| format!(": {}", m.display())).unwrap_or_default()),
        "put a ggml-*.bin in ~/.local/share/edits/models or set EDITS_WHISPER_MODEL (ggml-large-v3-turbo is best for Turkish)", false);
    let llm = pick::llm_cmd();
    check(util::have(llm.split_whitespace().next().unwrap_or("")), &format!("LLM command: {llm}"), "set EDITS_LLM to a command that reads a prompt on stdin; without it cuts follow viewer signals only", false);
    println!("taste file: {}", pick::taste_path().display());
    if missing > 0 {
        return Err(format!("{missing} required tool(s) missing"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn cli() {
        let p = |s: &str| parse(&args(s), None);
        let c = p("recep ivedik komik -n 5 --aspect 1:1 --grade dark --no-llm --fit").unwrap();
        assert_eq!((c.input.as_str(), c.n, c.opts.w, c.opts.h, c.opts.grade.as_str(), c.llm, c.opts.layout.as_str()), ("recep ivedik komik", 5, 1080, 1080, "dark", false, "fit"));
        assert!(p("-n 3").is_err());
        assert!(p("x -n 0").is_err());
        assert!(p("x --grade pink").is_err());
        assert!(p("x --aspect 2:1").is_err());
        assert!(p("x --wat").is_err());
        assert!(p("x -n").is_err());
    }

    #[test]
    fn studied_style_sets_defaults_and_flags_win() {
        let style = || Some(study::Style { aspect: "4:3".into(), grade: "nonsense".into(), lufs: -99.0 });
        let c = parse(&args("x"), style()).unwrap();
        assert_eq!((c.opts.w, c.opts.h, c.opts.grade.as_str(), c.opts.lufs), (1440, 1080, "bw", -30.0));
        assert_eq!(parse(&args("x --aspect 9:16"), style()).unwrap().opts.h, 1920);
    }

    #[test]
    fn safe_names() {
        assert_eq!(safe("a/b c.mp4"), "a_b_c_mp4");
    }
}
