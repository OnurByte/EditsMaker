mod face;
mod fetch;
mod montage;
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
    /// a song (link or file): make music edits instead of talking clips
    music: Option<String>,
    look: montage::Look,
    /// put the speaker's best line over the music edit
    voice: bool,
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
        llm: true, join: false, music: None, voice: false, look: montage::Look { sad: false, length: 0.0, tag: String::new(), text: String::new() },
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
            "--music" => c.music = Some(val("--music")?),
            "--sad" => c.look.sad = true,
            "--voice" => c.voice = true,
            "--length" => c.look.length = val("--length")?.parse().ok().filter(|l| (6.0..=90.0).contains(l)).ok_or("--length must be 6-90 seconds")?,
            "--tag" => c.look.tag = montage::clean_tag(&val("--tag")?),
            "--text" => c.look.text = val("--text")?.chars().take(60).collect(),
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
    if c.look.length == 0.0 {
        c.look.length = if c.look.sad { 24.0 } else { 18.0 };
    }
    Ok(c)
}

struct Analysed {
    src: fetch::Source,
    words: Vec<transcribe::Word>,
    lines: Vec<transcribe::Line>,
    moments: Vec<pick::Moment>,
}

/// Everything about one video up to "which moments": transcript, signals, the pick.
fn analyse(src: fetch::Source, c: &Cli) -> Analysed {
    // no captions and no speech model is fine: the cut then rests on the other signals, without burned-in text
    let t = transcribe::transcript(&src.work, &src.wav, &c.lang, &src.language).unwrap_or_else(|e| {
        eprintln!("edits: no transcript for {} ({e})", src.title);
        transcribe::Transcript::default()
    });
    let lines = transcribe::lines(&t.words);
    let mut hits = signals::comment_hits(&src.comments, src.duration);
    let comments = hits.len();
    hits.extend(t.laughs.iter().map(|&s| signals::Hit { t: (s - 4.0).max(0.0), weight: 4.0, text: "(videoda kahkaha/alkış sesi)".into() }));
    let loud = signals::loudness(&src.wav).unwrap_or_default();
    let curve = signals::curve(src.duration, &src.heat, &hits, &loud);
    let moments = pick::pick(&src.title, &lines, &curve, &hits, c.n, src.duration, c.llm);
    eprintln!("edits: {} ({}): words from {}, {comments} timestamped comments, {} laughs, replay heat {}, {} moments",
        src.title, util::mmss(src.duration), if t.from.is_empty() { "nothing" } else { t.from }, t.laughs.len(), if src.heat.is_empty() { "no" } else { "yes" }, moments.len());
    Analysed { src, words: t.words, lines, moments }
}

/// Downloads the picture for one moment and renders it.
fn cut(a: &Analysed, m: &pick::Moment, c: &Cli, out: &Path) -> Result<serde_json::Value, String> {
    let (video, seek) = fetch::clip(&a.src, m.start, m.end)?;
    let (w, h) = fetch::dims(&video);
    let focus = if c.opts.layout == "fill" { face::center_x(&video, seek, m.end - m.start, w, h) } else { None };
    // the subtitle file is per clip: clips render at the same time
    let work = a.src.work.join(out.file_stem().unwrap_or_default());
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    // nobody to frame on (screen recordings, graphics): cropping would cut content at random, so keep the whole frame
    let mut opts = c.opts.clone();
    if focus.is_none() && w * opts.h > h * opts.w {
        opts.layout = "fit".into();
    }
    render::render(&video, seek, m, &a.words, &opts, focus.unwrap_or(0.5), &work, out)?;
    let text = a.lines.iter().filter(|l| l.s >= m.start - 0.5 && l.e <= m.end + 0.5).map(|l| l.text.as_str()).collect::<Vec<_>>().join(" ");
    let side = serde_json::json!({ "clip": out, "source": a.src.title, "id": a.src.id, "moment": m, "text": text, "framed_on_faces": focus.is_some() });
    std::fs::write(out.with_extension("json"), serde_json::to_string_pretty(&side).unwrap()).map_err(|e| e.to_string())?;
    Ok(side)
}

fn make(args: &[String]) -> Result<(), String> {
    let c = &parse(args, study::load())?;
    let cache = &util::home().join(".cache/edits");
    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&c.out).map_err(|e| e.to_string())?;
    let clock = std::time::Instant::now();
    let lap = |what: &str| eprintln!("edits: [{:>5.1}s] {what}", clock.elapsed().as_secs_f64());
    if let Some(music) = &c.music {
        return make_music(c, music, cache, &lap);
    }

    // every video is fetched and analysed at the same time; one that fails is skipped, not fatal
    let analysed: Vec<Analysed> = if Path::new(&c.input).is_file() {
        vec![analyse(fetch::local(Path::new(&c.input), cache)?, c)]
    } else {
        let found = fetch::resolve(&c.input, c.videos)?;
        lap(&format!("found {} video(s)", found.len()));
        std::thread::scope(|s| {
            let jobs: Vec<_> = found.iter().map(|(id, url)| s.spawn(move || fetch::remote(id, url, cache).map(|src| analyse(src, c)))).collect();
            jobs.into_iter().filter_map(|j| match j.join() {
                Ok(Ok(a)) => Some(a),
                Ok(Err(e)) => {
                    eprintln!("edits: skipped a video: {e}");
                    None
                }
                Err(_) => None,
            }).collect()
        })
    };
    lap("analysed");

    // only the best moments overall get their picture downloaded and rendered, four at a time
    let mut best: Vec<(&Analysed, &pick::Moment)> = analysed.iter().flat_map(|a| a.moments.iter().map(move |m| (a, m))).collect();
    best.sort_by(|a, b| b.1.score.total_cmp(&a.1.score));
    best.truncate(c.n);
    let jobs: Vec<(PathBuf, &Analysed, &pick::Moment)> = best.iter().enumerate()
        .map(|(k, (a, m))| (c.out.join(format!("{:02}-{}.mp4", k + 1, fetch::safe(&a.src.id))), *a, *m)).collect();
    let mut clips: Vec<PathBuf> = vec![];
    for batch in jobs.chunks(4) {
        let done: Vec<_> = std::thread::scope(|s| {
            let running: Vec<_> = batch.iter().map(|(out, a, m)| s.spawn(move || cut(a, m, c, out))).collect();
            running.into_iter().map(|j| j.join().unwrap_or_else(|_| Err("render thread crashed".into()))).collect()
        });
        for ((out, _, m), res) in batch.iter().zip(done) {
            match res {
                Ok(side) => {
                    eprintln!("edits: {} [{}-{}] {}", out.display(), util::mmss(m.start), util::mmss(m.end), m.title);
                    println!("{side}");
                    clips.push(out.clone());
                }
                Err(e) => eprintln!("edits: could not make {}: {e}", out.display()),
            }
        }
    }
    lap("rendered");
    if clips.is_empty() {
        return Err("found nothing worth cutting".into());
    }
    if c.join && clips.len() > 1 {
        let out = c.out.join("joined.mp4");
        render::join(&clips, cache, &out)?;
        eprintln!("edits: joined {} clips into {}", clips.len(), out.display());
    }
    Ok(())
}

/// Music edits: the song's beats, the footage's best shots, one ffmpeg run per edit.
fn make_music(c: &Cli, music: &str, cache: &Path, lap: &dyn Fn(&str)) -> Result<(), String> {
    let (song, videos) = std::thread::scope(|s| {
        let song = s.spawn(|| montage::song(music, cache));
        // picture only, every video at the same time
        let videos: Vec<(String, PathBuf)> = if Path::new(&c.input).is_file() {
            vec![(c.input.clone(), PathBuf::from(&c.input))]
        } else {
            let found = fetch::resolve(&c.input, c.videos).unwrap_or_else(|e| {
                eprintln!("edits: {e}");
                vec![]
            });
            let jobs: Vec<_> = found.into_iter().map(|(id, url)| s.spawn(move || montage::video_file(&id, &url, cache).map(|f| (id, f)))).collect();
            jobs.into_iter().filter_map(|j| j.join().ok()?.map_err(|e| eprintln!("edits: skipped a video: {e}")).ok()).collect()
        };
        (song.join().unwrap_or_else(|_| Err("song thread crashed".into())), videos)
    });
    let song = song?;
    if videos.is_empty() {
        return Err("no footage to cut".into());
    }
    // the speaker's best lines, found the same way as for talking clips; video ids tie them to the footage
    let spoken: Vec<Analysed> = if !c.voice {
        vec![]
    } else if Path::new(&c.input).is_file() {
        vec![analyse(fetch::local(Path::new(&c.input), cache)?, c)]
    } else {
        std::thread::scope(|s| {
            let jobs: Vec<_> = videos.iter().map(|(id, _)| s.spawn(move || {
                fetch::remote(id, &format!("https://www.youtube.com/watch?v={id}"), cache).map(|src| analyse(src, c))
            })).collect();
            jobs.into_iter().filter_map(|j| j.join().ok()?.map_err(|e| eprintln!("edits: no voice from a video: {e}")).ok()).collect()
        })
    };
    let mut lines: Vec<(&Analysed, &pick::Moment)> = spoken.iter().flat_map(|a| a.moments.iter().map(move |m| (a, m))).collect();
    lines.sort_by(|a, b| b.1.score.total_cmp(&a.1.score));
    if c.voice && lines.is_empty() {
        return Err("--voice: found no spoken line to use".into());
    }
    lap(&format!("song \"{}\": {} beats; {} video(s)", song.title, song.beats.len(), videos.len()));
    let shots: Vec<montage::Shot> = std::thread::scope(|s| {
        let jobs: Vec<_> = videos.iter().enumerate().map(|(i, (_, f))| s.spawn(move || montage::frames(f).map(|fs| montage::segment(&fs, i)))).collect();
        jobs.into_iter().flat_map(|j| j.join().ok().and_then(|r| r.map_err(|e| eprintln!("edits: {e}")).ok()).unwrap_or_default()).collect()
    });
    lap(&format!("{} shots", shots.len()));
    let mut used = vec![false; shots.len()];
    let kind = match (c.look.sad, c.voice) { (true, true) => "sad-voice", (true, false) => "sad", (false, true) => "voice", _ => "edit" };
    let plans: Vec<_> = (0..c.n).filter_map(|k| {
        // with a voice the edit lasts as long as the line, plus a moment of music after the last word
        let voice = lines.get(k).map(|(a, m)| (*a, *m));
        if c.voice && voice.is_none() {
            return None;
        }
        let length = voice.map_or(c.look.length, |(_, m)| (m.end - m.start + 1.5).clamp(6.0, 60.0));
        let wins = montage::windows(&song.beats, &song.loud, length, k + 1);
        let start = *wins.get(k).or(wins.last())?;
        let cuts = montage::cuts(&song.beats, start, length, c.look.shot());
        let hits: Vec<f64> = cuts.iter().map(|t| montage::hit_at(&song.beats, &song.strength, start + t)).collect();
        let joins = montage::joins(&hits, c.look.sad);
        let spans = montage::pieces(&cuts, &joins);
        let needs: Vec<f64> = spans.iter().map(|(_, l)| l * c.look.speed() + 0.25).collect();
        let picks = montage::assign(&shots, &mut used, &needs);
        if picks.len() < needs.len() {
            eprintln!("edits: not enough usable shots for edit {}", k + 1);
            return None;
        }
        Some((k, start, cuts, joins, picks, voice))
    }).collect();
    let made: Vec<_> = std::thread::scope(|s| {
        let jobs: Vec<_> = plans.iter().map(|(k, start, cuts, joins, picks, voice)| {
            let (song, shots, videos) = (&song, &shots, &videos);
            s.spawn(move || {
                let out = c.out.join(format!("{kind}-{:02}.mp4", k + 1));
                let files: Vec<(PathBuf, f64, f64)> = picks.iter().map(|(i, st)| (videos[shots[*i].video].1.clone(), *st, shots[*i].focus.unwrap_or(0.5))).collect();
                let work = cache.join("renders").join(format!("{kind}-{:02}", k + 1));
                std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
                let v = voice.map(|(a, m)| montage::Voice { file: a.src.audio.clone(), start: m.start, len: m.end - m.start });
                let (words, said) = match voice {
                    Some((a, m)) if c.opts.captions => (&a.words[..], Some(*m)),
                    _ => (&[][..], None),
                };
                let subs = montage::captions(words, said, &c.look.text, c.look.sad, *cuts.last().unwrap(), &c.opts);
                montage::render(&files, cuts, joins, &song.file, *start, v.as_ref(), &subs, &c.look, &c.opts, &work, &out)?;
                let side = serde_json::json!({
                    "clip": out, "song": song.title, "song_from": start, "seconds": cuts.last(),
                    "voice": voice.map(|(a, m)| serde_json::json!({ "video": a.src.id, "moment": m,
                        "text": a.lines.iter().filter(|l| l.s >= m.start - 0.5 && l.e <= m.end + 0.5).map(|l| l.text.as_str()).collect::<Vec<_>>().join(" ") })),
                    "shots": picks.iter().map(|(i, st)| serde_json::json!({ "video": videos[shots[*i].video].0, "from": st })).collect::<Vec<_>>(),
                });
                std::fs::write(out.with_extension("json"), serde_json::to_string_pretty(&side).unwrap()).map_err(|e| e.to_string())?;
                Ok::<_, String>(side)
            })
        }).collect();
        jobs.into_iter().map(|j| j.join().unwrap_or_else(|_| Err("render thread crashed".into()))).collect()
    });
    let mut ok = 0;
    for r in made {
        match r {
            Ok(side) => {
                println!("{side}");
                ok += 1;
            }
            Err(e) => eprintln!("edits: {e}"),
        }
    }
    lap(&format!("{ok} edit(s) in {}", c.out.display()));
    if ok == 0 {
        return Err("no edits made".into());
    }
    Ok(())
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
}
