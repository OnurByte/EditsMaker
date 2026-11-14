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
    /// a reference edit to copy: its music, cuts, flashes, colour and shape
    like: Option<String>,
    look: montage::Look,
    /// put the speaker's best line over the music edit
    voice: bool,
    /// VIDEO@SECONDS: the passages to use, by hand
    say: Vec<String>,
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
        llm: true, join: false, music: None, like: None, voice: false, say: vec![], look: montage::Look { sad: false, punch: true, length: 0.0, tag: String::new(), text: String::new() },
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
            "--like" => c.like = Some(val("--like")?),
            "--sad" => c.look.sad = true,
            "--voice" => c.voice = true,
            "--say" => {
                c.say.push(val("--say")?);
                c.voice = true;
            }
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
    /// viewer interest per second
    curve: Vec<f64>,
}

/// Everything about one video up to "which moments": transcript, signals, the pick.
/// `clips`: also pick talking-clip moments (music edits pick whole passages themselves).
fn analyse(src: fetch::Source, c: &Cli, clips: bool) -> Analysed {
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
    let moments = if clips { pick::pick(&src.title, &lines, &curve, &hits, c.n, src.duration, c.llm) } else { vec![] };
    eprintln!("edits: {} ({}): words from {}, {comments} timestamped comments, {} laughs, replay heat {}, {} moments",
        src.title, util::mmss(src.duration), if t.from.is_empty() { "nothing" } else { t.from }, t.laughs.len(), if src.heat.is_empty() { "no" } else { "yes" }, moments.len());
    Analysed { src, words: t.words, lines, moments, curve }
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
    if c.music.is_some() || c.like.is_some() {
        return make_music(c, cache, &lap);
    }

    // every video is fetched and analysed at the same time; one that fails is skipped, not fatal
    let analysed: Vec<Analysed> = if Path::new(&c.input).is_file() {
        vec![analyse(fetch::local(Path::new(&c.input), cache)?, c, true)]
    } else {
        let found = fetch::resolve(&c.input, c.videos)?;
        lap(&format!("found {} video(s)", found.len()));
        std::thread::scope(|s| {
            let jobs: Vec<_> = found.iter().map(|(id, url)| s.spawn(move || fetch::remote(id, url, cache).map(|src| analyse(src, c, true)))).collect();
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

/// One music edit, decided and ready to render.
struct Plan<'a> {
    k: usize,
    /// where in the song the edit starts
    start: f64,
    cuts: Vec<f64>,
    joins: Vec<montage::Join>,
    /// the music's beats on the edit's timeline
    beats: Vec<f64>,
    /// (video file, from, focus) per piece
    files: Vec<(PathBuf, f64, f64)>,
    /// (video id, from) per piece, for the sidecar
    labels: Vec<(String, f64)>,
    voice: Option<(&'a Analysed, &'a pick::Moment)>,
    /// where the voice comes in
    at: f64,
}

/// What the speaker says in a moment, as text.
fn said(a: &Analysed, m: &pick::Moment) -> String {
    a.words.iter().filter(|w| w.s >= m.start && w.e <= m.end).map(|w| w.t.as_str()).collect::<Vec<_>>().join(" ")
}

/// Music edits: the song's beats (or a reference edit's cuts), the footage's best shots, and with --voice the
/// speaker's best whole passage coming in on a beat; one ffmpeg run per edit.
fn make_music(c: &Cli, cache: &Path, lap: &dyn Fn(&str)) -> Result<(), String> {
    let template = match &c.like {
        Some(r) => Some(montage::template(Path::new(r), cache)?),
        None => None,
    };
    let mut opts = c.opts.clone();
    if let Some(t) = &template {
        if let Some((w, h)) = size(t.aspect) {
            (opts.w, opts.h) = (w, h);
        }
        opts.grade = t.grade.clone();
        lap(&format!("reference: {} cuts, {} white flashes, {} dips, grade {}, {}", t.cuts.len() - 2,
            t.joins.iter().filter(|j| j.kind == "fadewhite").count(), t.joins.iter().filter(|j| j.kind == "fadeblack").count(), t.grade, t.aspect));
    }
    let look = montage::Look { punch: !c.look.sad && template.is_none(), ..c.look.clone() };
    let music = template.as_ref().map(|t| t.music.to_string_lossy().into_owned()).or(c.music.clone()).ok_or("--music or --like is needed")?;
    let local = Path::new(&c.input).is_file();
    let (song, videos) = std::thread::scope(|s| {
        let song = s.spawn(|| montage::song(&music, cache));
        // picture only, every video at the same time
        let videos: Vec<(String, PathBuf)> = if local {
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
    lap(&format!("song \"{}\": {} beats; {} video(s)", song.title, song.beats.len(), videos.len()));

    // the speaker's words, and the whole passages worth putting over music
    let spoken: Vec<Analysed> = if !c.voice {
        vec![]
    } else if local {
        vec![analyse(fetch::local(Path::new(&c.input), cache)?, c, false)]
    } else {
        std::thread::scope(|s| {
            let jobs: Vec<_> = videos.iter().map(|(id, _)| s.spawn(move || {
                fetch::remote(id, &format!("https://www.youtube.com/watch?v={id}"), cache).map(|src| analyse(src, c, false))
            })).collect();
            jobs.into_iter().filter_map(|j| j.join().ok()?.map_err(|e| eprintln!("edits: no voice from a video: {e}")).ok()).collect()
        })
    };
    let reach = template.as_ref().map_or(40.0, |t| t.cuts.last().unwrap() - 1.4).max(6.0);
    let mut cands: Vec<(usize, pick::Moment)> = spoken.iter().enumerate()
        .flat_map(|(i, a)| pick::passages(&a.words, &a.curve, 6.0, reach).into_iter().take(6).map(move |m| (i, m))).collect();
    cands.sort_by(|a, b| b.1.score.total_cmp(&a.1.score));
    cands.truncate(15);
    let listed: Vec<(String, pick::Moment, String)> = cands.iter().map(|(i, m)| (spoken[*i].src.title.clone(), m.clone(), said(&spoken[*i], m))).collect();
    for (k, (title, m, text)) in listed.iter().enumerate() {
        eprintln!("edits: line {k}: {title} @{:.1} ({:.0}s, interest {:.1}): {text}", m.start, m.end - m.start, m.score);
    }
    // --say VIDEO@FROM-TO: that stretch, widened to whole words
    let said_by_hand: Vec<(usize, pick::Moment)> = c.say.iter().filter_map(|want| {
        let (id, range) = want.split_once('@')?;
        let (a, b) = range.split_once('-')?;
        let (a, b): (f64, f64) = (a.parse().ok()?, b.parse().ok()?);
        let i = spoken.iter().position(|x| x.src.id == id)?;
        let ws: Vec<&transcribe::Word> = spoken[i].words.iter().filter(|w| w.s >= a - 0.3 && w.s < b).collect();
        let (first, last) = (ws.first()?, ws.last()?);
        Some((i, pick::Moment { start: (first.s - 0.05).max(0.0), end: last.e + 0.25, punch: last.s, score: 0.0, title: String::new(), why: "chosen by hand".into() }))
    }).collect();
    if c.say.len() != said_by_hand.len() {
        return Err("--say: give VIDEO@FROM-TO with a video from this run and seconds where it speaks".into());
    }
    let chosen: Vec<(usize, pick::Moment)> = if !said_by_hand.is_empty() {
        said_by_hand
    } else {
        pick::choose_passages(&listed, c.n, c.llm).into_iter().map(|k| cands[k].clone()).collect()
    };
    if c.voice && chosen.is_empty() {
        return Err("--voice: found no whole passage of speech to use".into());
    }

    let shots: Vec<montage::Shot> = std::thread::scope(|s| {
        let jobs: Vec<_> = videos.iter().enumerate().map(|(i, (_, f))| s.spawn(move || montage::frames(f).map(|fs| montage::segment(&fs, i)))).collect();
        jobs.into_iter().flat_map(|j| j.join().ok().and_then(|r| r.map_err(|e| eprintln!("edits: {e}")).ok()).unwrap_or_default()).collect()
    });
    lap(&format!("{} shots", shots.len()));
    let mut used = vec![false; shots.len()];
    let kind = match (&c.like, c.look.sad, c.voice) {
        (Some(r), _, _) => format!("like-{}", fetch::safe(&Path::new(r).file_stem().unwrap_or_default().to_string_lossy())),
        (None, true, true) => "sad-voice".into(),
        (None, true, false) => "sad".into(),
        (None, false, true) => "voice".into(),
        _ => "edit".into(),
    };
    let plans: Vec<Plan> = (0..c.n).filter_map(|k| {
        let voice = chosen.get(k).map(|(i, m)| (&spoken[*i], m));
        if c.voice && voice.is_none() {
            return None;
        }
        let voice_video = |a: &Analysed| videos.iter().position(|(id, _)| *id == a.src.id).or(if local { Some(0) } else { None });
        let mut files: Vec<(PathBuf, f64, f64)> = vec![];
        let mut taken = used.clone();
        let (start, mut cuts, at) = match (&template, voice) {
            // the story: a beat of music, the line over the footage it was said over, and the drop as it ends
            (None, Some((a, m))) => {
                let (lead, vlen) = (0.6, m.end - m.start);
                // the drop needs 2.5 s of song after it; a short song gets a shorter montage
                let end = song.loud.len() as f64 * montage::hop_s();
                let want = (c.look.length * 0.45).clamp(4.0, 9.0).min(end - lead - vlen - 0.2).max(2.5);
                let Some(drop) = montage::drop_at(&song.beats, &song.strength, &song.loud, lead + vlen, want)
                    .or_else(|| montage::drop_at(&song.beats, &song.strength, &song.loud, lead + vlen, 2.5)) else {
                    eprintln!("edits: the song is too short for line {} ({vlen:.0}s)", k + 1);
                    return None;
                };
                let tail = want.min(end - drop - 0.1);
                let start = drop - lead - vlen;
                let d = lead + vlen;
                let beats: Vec<f64> = song.beats.iter().map(|b| b - start).filter(|b| *b > d + 0.3 && *b <= d + tail + 0.6).collect();
                let k = ((look.shot() / montage::beat_period(&song.beats)).round() as usize).max(1);
                let mut cuts = vec![0.0, d];
                cuts.extend(beats.iter().step_by(k).copied().filter(|b| *b - d >= 0.4));
                if cuts.len() < 3 {
                    cuts.push(d + tail);
                }
                let v = voice_video(a)?;
                // the speaker's own picture, in step with the words
                let from = (m.start - lead).max(0.0);
                let focus = shots.iter().filter(|x| x.video == v && x.e > from && x.s < m.end).find_map(|x| x.focus).unwrap_or(0.5);
                files.push((videos[v].1.clone(), from, focus));
                shots.iter().enumerate().filter(|(_, x)| x.video == v && x.e > from - 5.0 && x.s < m.end + 5.0).for_each(|(i, _)| taken[i] = true);
                (start, cuts, lead)
            }
            (Some(t), _) => {
                let total = *t.cuts.last()?;
                let at = voice.map_or(0.0, |(_, m)| t.cuts.iter().copied().find(|x| *x >= 0.5 && x + m.end - m.start <= total - 0.6).unwrap_or(0.3));
                (0.0, t.cuts.clone(), at)
            }
            (None, None) => {
                let wins = montage::windows(&song.beats, &song.loud, c.look.length, k + 1);
                let start = *wins.get(k).or(wins.last())?;
                (start, montage::cuts(&song.beats, start, c.look.length, look.shot()), 0.0)
            }
        };
        let total = *cuts.last()?;
        let beats: Vec<f64> = song.beats.iter().map(|b| b - start).filter(|b| *b >= 0.0 && *b <= total).collect();
        let mut joins = match &template {
            Some(t) => t.joins.clone(),
            None => montage::joins(&cuts.iter().map(|t| montage::hit_at(&song.beats, &song.strength, start + t)).collect::<Vec<_>>(), c.look.sad),
        };
        if template.is_none() && voice.is_some() && joins.len() > 2 {
            // the drop
            joins[1] = if c.look.sad { montage::Join { kind: "fadeblack", dur: 0.5 } } else { montage::Join { kind: "fadewhite", dur: 0.3 } };
        }
        if template.is_none() && voice.is_none() {
            cuts.dedup();
        }
        let spans = montage::pieces(&cuts, &joins);
        let needs: Vec<f64> = spans.iter().skip(files.len()).map(|(_, l)| l * look.speed() + 0.25).collect();
        // footage shown while the words were said goes first in line
        let prefer = voice.and_then(|(a, m)| Some((voice_video(a)?, m.start - 40.0, m.end + 40.0)));
        let picks = montage::assign(&shots, &mut taken, &needs, prefer);
        if picks.len() < needs.len() {
            eprintln!("edits: not enough usable shots for edit {}", k + 1);
            return None;
        }
        picks.iter().for_each(|(i, _)| used[*i] = true);
        files.extend(picks.iter().map(|(i, st)| (videos[shots[*i].video].1.clone(), *st, shots[*i].focus.unwrap_or(0.5))));
        let labels = files.iter().map(|(f, st, _)| (videos.iter().find(|(_, p)| p == f).map(|x| x.0.clone()).unwrap_or_default(), *st)).collect();
        Some(Plan { k, start, cuts, joins, beats, files, labels, voice, at })
    }).collect();
    let made: Vec<_> = std::thread::scope(|s| {
        let jobs: Vec<_> = plans.iter().map(|p| {
            let (song, kind, look, opts) = (&song, &kind, &look, &opts);
            s.spawn(move || {
                let out = c.out.join(format!("{kind}-{:02}.mp4", p.k + 1));
                let work = cache.join("renders").join(format!("{kind}-{:02}", p.k + 1));
                std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
                let v = p.voice.map(|(a, m)| montage::Voice { file: a.src.audio.clone(), start: m.start, len: m.end - m.start, at: p.at });
                let (words, line) = match p.voice {
                    Some((a, m)) if opts.captions => (&a.words[..], Some(m)),
                    _ => (&[][..], None),
                };
                let subs = montage::captions(words, line, p.at, &look.text, !look.punch, *p.cuts.last().unwrap(), opts);
                montage::render(&p.files, &p.cuts, &p.joins, &p.beats, &song.file, p.start, v.as_ref(), &subs, look, opts, &work, &out)?;
                let side = serde_json::json!({
                    "clip": out, "song": song.title, "song_from": p.start, "seconds": p.cuts.last(),
                    "voice": p.voice.map(|(a, m)| serde_json::json!({ "video": a.src.id, "moment": m, "at": p.at, "text": said(a, m) })),
                    "shots": p.labels.iter().map(|(v, st)| serde_json::json!({ "video": v, "from": st })).collect::<Vec<_>>(),
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
