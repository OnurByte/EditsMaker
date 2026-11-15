#![recursion_limit = "256"]
mod face;
mod fetch;
mod finish;
mod fx;
mod grades;
mod lang;
mod learn;
mod library;
mod looks;
mod mcp;
mod montage;
mod pick;
mod premiere;
mod preset;
mod presets;
mod render;
mod signals;
mod study;
mod svg;
mod talk;
mod timeline;
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
        Some("trends") => fetch::trends(&args.get(1).cloned().unwrap_or_else(|| lang::system().1)).map(|list| {
            list.iter().enumerate().take(30).for_each(|(i, s)| println!("{:>2}. {s}", i + 1));
            println!("\nuse one: --music trend:N  (another country: --music trend:us:N)");
        }),
        Some("rate") => rate(&args[1..]),
        Some("preset") => preset_cmd(&args[1..]),
        Some("library") if args.get(1).is_some_and(|a| a == "add") && args.len() > 2 => (|| {
            // the pack's media into the library, then its presets (Alight projects, Premiere moves) like preset import
            for src in &args[2..] {
                let root = library::add(Path::new(src))?;
                let mut import = vec!["import".to_string(), root.to_string_lossy().into_owned()];
                import.extend(library::walk(&root).iter().map(|f| f.to_string_lossy().into_owned()).filter(|f| f.to_lowercase().contains(".prfpset")));
                preset_cmd(&import)?;
            }
            Ok(())
        })(),
        Some("library") => {
            library::list(&args[1..]);
            Ok(())
        }
        Some("mcp") => mcp::serve(),
        Some("skills") => skills(&args[1..]),
        Some("looks") => {
            println!("colour (--look):");
            looks::LOOKS.iter().for_each(|(n, about, _)| println!("  {n:<12} {about}"));
            grades::all().iter().for_each(|g| println!("  {:<12} {}", g.name, g.about));
            println!("\ncaptions (--captions):");
            talk::styles().iter().for_each(|st| println!("  {:<12} {}", st.name, st.about));
            println!("\nbeat hits (--fx a,b,...): {}", fx::HITS.join(", "));
            println!("\ndrawn graphics (--svg NAME[@FROM-TO][:X,Y,SIZE][#COLOUR], or your own .svg):");
            svg::STICKERS.iter().for_each(|(n, about, _)| println!("  {n:<12} {about}"));
            Ok(())
        }
        Some("learn") if args.len() > 1 => (|| {
            // --into DIR writes elsewhere; default: your preset folder
            let into = args.iter().position(|a| a == "--into").and_then(|i| args.get(i + 1)).map(PathBuf::from).unwrap_or_else(presets::user_dir);
            std::fs::create_dir_all(&into).map_err(|e| e.to_string())?;
            for v in args[1..].iter().filter(|a| a.as_str() != "--into" && Path::new(a) != into) {
                let path = Path::new(v);
                let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
                let p = learn::learn(path, &name)?;
                let file = into.join(format!("{}.json", preset::slug(&name)));
                std::fs::write(&file, preset::pretty(&p)).map_err(|e| e.to_string())?;
                println!("{}: {} ({:.1}s) -> --preset {}", file.display(), p.about, p.total, preset::slug(&name));
            }
            Ok(())
        })(),
        Some("cube") => match (args.get(1).and_then(|n| grades::find(n)), args.get(2)) {
            (Some(g), out) => (|| {
                let tmp = std::env::temp_dir().join("edits-cube");
                let made = grades::lut(&g, &tmp)?;
                let to = out.map(PathBuf::from).unwrap_or_else(|| PathBuf::from(format!("{}.cube", g.name)));
                std::fs::copy(made.trim_start_matches(",lut3d=file="), &to).map_err(|e| e.to_string())?;
                println!("{}", to.display());
                Ok(())
            })(),
            _ => Err("usage: edits cube <look from an Alight cc pack> [out.cube]   (edits looks lists them)".into()),
        },
        Some("draw") => match (args.get(1), args.len() > 2) {
            (Some(v), true) => (|| {
                // "--overlay SPEC" items are library clips, the rest drawn graphics
                let over: Vec<usize> = (2..args.len()).filter(|&i| args[i - 1] == "--overlay").collect();
                let ov = over.iter().map(|&i| library::parse(&args[i])).collect::<Result<Vec<_>, _>>()?;
                let st = (2..args.len()).filter(|i| !over.contains(i) && args[*i] != "--overlay").map(|i| svg::parse(&args[i])).collect::<Result<Vec<_>, _>>()?;
                let src = Path::new(v);
                let out = src.with_file_name(format!("{}-drawn.mp4", src.file_stem().unwrap_or_default().to_string_lossy()));
                std::fs::copy(src, &out).map_err(|e| format!("{v}: {e}"))?;
                svg::apply(&out, &st)?;
                library::apply(&out, &ov)?;
                println!("{}", out.display());
                Ok(())
            })(),
            _ => Err("usage: edits draw <video> NAME[@FROM-TO][:X,Y,SIZE][#COLOUR] ... [--overlay NAME[@FROM[-TO]][:OPACITY]] ...".into()),
        },
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
    /// an Alight Motion project to replay over the footage
    preset: Option<String>,
    /// a .cube LUT applied over the look
    lut: Option<PathBuf>,
    /// --aspect or --size was given: it wins over a preset's own frame
    sized: bool,
    /// --look was given: it wins over a preset's own look
    looked: bool,
    /// --intensity: scales a preset's moves
    intensity: Option<f64>,
    /// of the videos found, how many to download after reading their captions
    use_videos: usize,
    look: montage::Look,
    /// put the speaker's best line over the music edit
    voice: bool,
    /// VIDEO@SECONDS: the passages to use, by hand
    say: Vec<String>,
    /// talking clips: caption style, jump cuts, sound effects, hook line, music bed
    caption_style: String,
    captions_given: bool,
    /// footage speed for every shot, 1 as made
    speed: f64,
    velocity: bool,
    /// effects on the hardest beats
    hits: Vec<String>,
    blur_faces: bool,
    blur_box: Option<(f64, f64, f64, f64)>,
    /// drawn graphics over the finished edit (--svg)
    svgs: Vec<svg::Sticker>,
    /// library clips over the finished edit
    overlays: Vec<library::Overlay>,
    /// talking clips: library sounds in place of the made whoosh and boom
    whoosh: Option<PathBuf>,
    boom: Option<PathBuf>,
    /// sentences on screen in music and preset edits
    lines: usize,
    /// lines given by hand (--line), used instead of picked ones
    line_texts: Vec<String>,
    /// timelines to write beside each clip: fcpxml, edl, srt
    export: Vec<String>,
    finish: finish::Finish,
    jumpcuts: bool,
    sfx: bool,
    hook: bool,
    bed: Option<String>,
}

fn size(aspect: &str) -> Option<(u32, u32)> {
    study::ASPECTS.iter().find(|a| a.0 == aspect).map(|a| (a.1, a.2))
}

/// Defaults come from `edits study` when it has been run; flags override them.
fn parse(args: &[String], style: Option<study::Style>) -> Result<Cli, String> {
    let style = style.unwrap_or(study::Style { aspect: "9:16".into(), grade: "bw".into(), lufs: -14.0 });
    let (w, h) = size(&style.aspect).unwrap_or((1080, 1920));
    let grade = if looks::names().contains(&style.grade.as_str()) { style.grade } else { "bw".into() };
    let mut c = Cli {
        input: String::new(), n: 3, videos: 5, lang: lang::code().into(), out: PathBuf::from("edits-out"),
        opts: render::Opts { w, h, grade, captions: true, layout: "fill".into(), lufs: style.lufs.clamp(-30.0, -5.0) },
        llm: true, join: false, music: None, like: None, preset: None, lut: None, sized: false, looked: false, intensity: None, use_videos: 4, voice: false, say: vec![], caption_style: "hormozi".into(), captions_given: false, speed: 1.0, velocity: false, hits: vec![], blur_faces: false, blur_box: None, svgs: vec![], overlays: vec![], whoosh: None, boom: None, lines: 0, line_texts: vec![], export: vec![], finish: finish::Finish::default(), jumpcuts: true, sfx: true, hook: true, bed: None, look: montage::Look { sad: false, punch: true, length: 0.0, tag: String::new(), text: String::new(), pace: 1.0, hits: vec![] },
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut val = |name: &str| it.next().cloned().ok_or(format!("{name} needs a value"));
        match a.as_str() {
            "-n" | "--moments" => c.n = val("-n")?.parse().ok().filter(|n| (1..=20).contains(n)).ok_or("-n must be 1-20")?,
            "--videos" => c.videos = val("--videos")?.parse().ok().filter(|n| (1..=40).contains(n)).ok_or("--videos must be 1-40")?,
            "--lang" => {
                c.lang = val("--lang")?.to_lowercase();
                lang::set(&c.lang);
            }
            "-o" | "--out" => c.out = PathBuf::from(val("--out")?),
            "--look" => {
                c.opts.grade = val("--look")?;
                c.looked = true;
                if !looks::names().contains(&c.opts.grade.as_str()) && grades::find(&c.opts.grade).is_none() {
                    let more: Vec<String> = grades::all().into_iter().map(|g| g.name).collect();
                    return Err(format!("--look must be one of: {}, {}", looks::names().join(", "), more.join(", ")));
                }
            }
            "--lut" => c.lut = Some(PathBuf::from(val("--lut")?)),
            "--aspect" => {
                let a = val("--aspect")?;
                (c.opts.w, c.opts.h) = size(&a).ok_or(format!("--aspect must be one of: {}", study::ASPECTS.iter().map(|a| a.0).collect::<Vec<_>>().join(", ")))?;
                c.sized = true;
            }
            "--size" => {
                let v = val("--size")?;
                let (w, h) = v.split_once('x').and_then(|(w, h)| Some((w.parse::<u32>().ok()?, h.parse::<u32>().ok()?)))
                    .filter(|(w, h)| (240..=4096).contains(w) && (240..=4096).contains(h)).ok_or("--size must be WxH, like 1080x1350")?;
                (c.opts.w, c.opts.h) = (w / 2 * 2, h / 2 * 2);
                c.sized = true;
            }
            "--intensity" => c.intensity = Some(val("--intensity")?.parse().ok().filter(|k| (0.0..=3.0).contains(k)).ok_or("--intensity must be 0-3")?),
            "--use" => c.use_videos = val("--use")?.parse().ok().filter(|n| (1..=20).contains(n)).ok_or("--use must be 1-20")?,
            "--fit" => c.opts.layout = "fit".into(),
            "--music" => c.music = Some(val("--music")?),
            "--like" => c.like = Some(val("--like")?),
            "--preset" => c.preset = Some(val("--preset")?),
            "--sad" => c.look.sad = true,
            "--voice" => c.voice = true,
            "--say" => {
                c.say.push(val("--say")?);
                c.voice = true;
            }
            "--length" => c.look.length = val("--length")?.parse().ok().filter(|l| (6.0..=90.0).contains(l)).ok_or("--length must be 6-90 seconds")?,
            "--tag" => c.look.tag = montage::clean_tag(&val("--tag")?),
            "--text" => c.look.text = val("--text")?.chars().take(60).collect(),
            "--captions" => {
                c.caption_style = val("--captions")?;
                c.captions_given = true;
                talk::style(&c.caption_style)?;
            }
            "--speed" => c.speed = val("--speed")?.parse().ok().filter(|k| (0.1..=8.0).contains(k)).ok_or("--speed must be 0.1-8")?,
            "--velocity" => c.velocity = true,
            "--fx" => {
                c.hits = val("--fx")?.split(',').map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect();
                if let Some(bad) = c.hits.iter().find(|h| !fx::HITS.contains(&h.as_str())) {
                    return Err(format!("--fx: {bad} is not one of {}", fx::HITS.join(", ")));
                }
            }
            "--blur-faces" => c.blur_faces = true,
            "--blur-box" => c.blur_box = Some(fx::parse_box(&val("--blur-box")?).ok_or("--blur-box must be x,y,w,h as fractions, like 0.7,0.85,0.3,0.15")?),
            "--line" => {
                c.line_texts.push(val("--line")?.chars().take(120).collect());
                c.lines = c.lines.max(c.line_texts.len());
            }
            "--lines" => c.lines = val("--lines")?.parse().ok().filter(|n| (1..=12).contains(n)).ok_or("--lines must be 1-12")?,
            "--svg" => c.svgs.push(svg::parse(&val("--svg")?)?),
            "--overlay" => c.overlays.push(library::parse(&val("--overlay")?)?),
            "--whoosh" | "--boom" => {
                let name = val(a)?;
                let f = library::find(&["sfx"], &name).ok_or(format!("{a} {name}: no such file or library sound (edits library sfx)"))?;
                if a == "--whoosh" { c.whoosh = Some(f) } else { c.boom = Some(f) }
            }
            "--export" => {
                c.export = val("--export")?.split(',').map(|x| x.trim().to_lowercase()).collect();
                if let Some(bad) = c.export.iter().find(|x| !["fcpxml", "edl", "srt"].contains(&x.as_str())) {
                    return Err(format!("--export: {bad} is not fcpxml, edl or srt"));
                }
            }
            "--stabilize" => c.finish.stabilize = true,
            "--denoise" => c.finish.denoise = true,
            "--smooth" => c.finish.smooth = true,
            "--fps" => c.finish.fps = Some(val("--fps")?.parse().ok().filter(|f| (12..=120).contains(f)).ok_or("--fps must be 12-120")?),
            "--codec" => {
                c.finish.codec = val("--codec")?;
                if !finish::CODECS.contains(&c.finish.codec.as_str()) {
                    return Err(format!("--codec must be one of {}", finish::CODECS.join(", ")));
                }
            }
            "--quality" => {
                c.finish.quality = val("--quality")?;
                if !finish::QUALITIES.contains(&c.finish.quality.as_str()) {
                    return Err(format!("--quality must be one of {}", finish::QUALITIES.join(", ")));
                }
            }
            "--no-jumpcuts" => c.jumpcuts = false,
            "--no-sfx" => c.sfx = false,
            "--no-hook" => c.hook = false,
            "--bed" => c.bed = Some(val("--bed")?),
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
    hits.extend(t.laughs.iter().map(|&s| signals::Hit { t: (s - 4.0).max(0.0), weight: 4.0, text: lang::prompt("notes", "laugh", &[]) }));
    let loud = signals::loudness(&src.wav).unwrap_or_default();
    let curve = signals::curve(src.duration, &src.heat, &hits, &loud);
    let moments = if clips { pick::pick(&src.title, &lines, &curve, &hits, c.n, src.duration, c.llm) } else { vec![] };
    eprintln!("edits: {} ({}): words from {}, {comments} timestamped comments, {} laughs, replay heat {}, {} moments",
        src.title, util::mmss(src.duration), if t.from.is_empty() { "nothing" } else { t.from }, t.laughs.len(), if src.heat.is_empty() { "no" } else { "yes" }, moments.len());
    Analysed { src, words: t.words, lines, moments, curve }
}

/// Downloads the picture for one moment and renders it.
fn cut(a: &Analysed, m: &pick::Moment, c: &Cli, bed: Option<&(PathBuf, f64)>, cache: &Path, out: &Path) -> Result<serde_json::Value, String> {
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
    // the hook: the model's title for the moment, else its first words
    // its first sentence, if short enough to read in a glance
    let mut first: Vec<&str> = vec![];
    for w in a.words.iter().filter(|w| w.s >= m.start && w.e <= m.end) {
        first.push(&w.t);
        if w.t.ends_with(['.', '?', '!']) || first.len() == 9 {
            break;
        }
    }
    let first = first.join(" ");
    let hook = c.hook.then(|| if m.title.trim().is_empty() { first } else { m.title.clone() });
    let t = talk::Talk {
        captions: if opts.captions { Some(talk::style(&c.caption_style)?) } else { None },
        jumpcuts: c.jumpcuts,
        sfx: c.sfx,
        whoosh: c.whoosh.clone(),
        boom: c.boom.clone(),
        hook,
        bed: bed.cloned(),
        pace: c.speed,
        frames: if opts.layout == "fill" { montage::frames(&video).unwrap_or_default() } else { vec![] },
        pre: {
            let mut pre = c.blur_box.map(|b| fx::blur_box("tb", b)).unwrap_or_default();
            if c.blur_faces {
                if let Some(b) = fx::face_box(&mut face::boxes(&video, seek, m.end - m.start, w, h)) {
                    pre.push_str(&fx::blur_box("tf", b));
                }
            }
            pre
        },
    };
    let segs = talk::render(&video, seek, m, &a.words, &opts, &t, focus.unwrap_or(0.5), cache, &work, out)?;
    let tl = timeline::Timeline {
        name: a.src.title.clone(), w: opts.w, h: opts.h,
        clips: segs.iter().map(|(x, y)| (video.clone(), x - m.start + seek, y - x)).collect(),
        music: bed.cloned(),
        captions: transcribe::lines(&a.words.iter().filter(|w| w.s >= m.start && w.e <= m.end).cloned().collect::<Vec<_>>()).iter()
            .filter_map(|l| Some((talk::remap(l.s, &segs)? / c.speed, talk::remap(l.e, &segs).unwrap_or(talk::remap(l.s, &segs)? + 1.0) / c.speed, l.text.clone()))).collect(),
    };
    let out = &deliver(c, out, &tl)?;
    let text = a.lines.iter().filter(|l| l.s >= m.start - 0.5 && l.e <= m.end + 0.5).map(|l| l.text.as_str()).collect::<Vec<_>>().join(" ");
    let side = serde_json::json!({ "clip": out, "source": a.src.title, "id": a.src.id, "moment": m, "text": text, "framed_on_faces": focus.is_some() });
    std::fs::write(out.with_extension("json"), serde_json::to_string_pretty(&side).unwrap()).map_err(|e| e.to_string())?;
    Ok(side)
}

fn make(args: &[String]) -> Result<(), String> {
    let mut cli = parse(args, study::load())?;
    let cache = &util::home().join(".cache/edits");
    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    if let Some(l) = cli.lut.clone() {
        cli.opts.grade = looks::lut(&l, cache)?;
    } else if let Some(g) = grades::find(&cli.opts.grade) {
        cli.opts.grade = grades::lut(&g, cache)?;
    }
    let c = &cli;
    std::fs::create_dir_all(&c.out).map_err(|e| e.to_string())?;
    let clock = std::time::Instant::now();
    let lap = |what: &str| eprintln!("edits: [{:>5.1}s] {what}", clock.elapsed().as_secs_f64());
    if let Some(name) = &c.preset {
        return make_preset(c, name, cache, &lap);
    }
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

    // a music bed under the talking: the song's loudest stretch
    let bed_song = match &c.bed {
        Some(b) => Some(montage::song(&fetch::music(b)?, cache)?),
        None => None,
    };
    let bed_track = bed_song.as_ref().map(|s| (s.file.clone(), montage::windows(&s.beats, &s.loud, 60.0, 1).first().copied().unwrap_or(0.0)));
    let bed = bed_track.as_ref();
    // only the best moments overall get their picture downloaded and rendered, four at a time
    let mut best: Vec<(&Analysed, &pick::Moment)> = analysed.iter().flat_map(|a| a.moments.iter().map(move |m| (a, m))).collect();
    best.sort_by(|a, b| b.1.score.total_cmp(&a.1.score));
    // the best moment of each video first, then seconds: three clips from three videos beat three from one
    let mut seen: Vec<String> = vec![];
    let (firsts, rest): (Vec<_>, Vec<_>) = best.into_iter().partition(|(a, _)| {
        let new = !seen.contains(&a.src.id);
        if new {
            seen.push(a.src.id.clone());
        }
        new
    });
    let mut best: Vec<_> = firsts.into_iter().chain(rest).collect();
    best.truncate(c.n);
    let jobs: Vec<(PathBuf, &Analysed, &pick::Moment)> = best.iter().enumerate()
        .map(|(k, (a, m))| (c.out.join(format!("{:02}-{}.mp4", k + 1, fetch::safe(&a.src.id))), *a, *m)).collect();
    let mut clips: Vec<PathBuf> = vec![];
    for batch in jobs.chunks(4) {
        let done: Vec<_> = std::thread::scope(|s| {
            let running: Vec<_> = batch.iter().map(|(out, a, m)| s.spawn(move || cut(a, m, c, bed, cache, out))).collect();
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

/// The footage to cut from: picture-only downloads (or the file given), and its shots.
/// (id, file) per video, every shot, and each video's measured frames.
type Footage = (Vec<(String, PathBuf)>, Vec<montage::Shot>, Vec<Vec<montage::Frame>>);

fn footage(c: &Cli, cache: &Path) -> Footage {
    let videos: Vec<(String, PathBuf)> = if Path::new(&c.input).is_file() {
        vec![(c.input.clone(), PathBuf::from(&c.input))]
    } else {
        let found = fetch::resolve(&c.input, c.videos).unwrap_or_else(|e| {
            eprintln!("edits: {e}");
            vec![]
        });
        let found = fetch::screen(found, c.use_videos, cache, &c.lang, c.llm);
        std::thread::scope(|s| {
            let jobs: Vec<_> = found.into_iter().map(|(id, url)| s.spawn(move || montage::video_file(&id, &url, cache).map(|f| (id, f)))).collect();
            jobs.into_iter().filter_map(|j| j.join().ok()?.map_err(|e| eprintln!("edits: skipped a video: {e}")).ok()).collect()
        })
    };
    let frames: Vec<Vec<montage::Frame>> = std::thread::scope(|s| {
        let jobs: Vec<_> = videos.iter().map(|(_, f)| s.spawn(move || montage::frames(f))).collect();
        jobs.into_iter().map(|j| j.join().ok().and_then(|r| r.map_err(|e| eprintln!("edits: {e}")).ok()).unwrap_or_default()).collect()
    });
    let shots = frames.iter().enumerate().flat_map(|(i, fs)| montage::segment(fs, i)).collect();
    (videos, shots, frames)
}

/// An Alight Motion project replayed over the footage: its shot timing, speed ramps, camera moves, shakes,
/// blurs and flashes, with its own song (found by name) unless --music says otherwise.
fn make_preset(c: &Cli, name: &str, cache: &Path, lap: &dyn Fn(&str)) -> Result<(), String> {
    let p = preset::load(name)?;
    lap(&format!("preset {}: {} shots in {:.1}s, {}x{}, {} flashes{}", p.name, p.slots.len(), p.total, p.w, p.h, p.flashes.len(),
        p.song.as_ref().map(|s| format!(", song \"{}\"", s.title)).unwrap_or_default()));
    let music = match (&c.music, &p.song) {
        (Some(m), _) => fetch::music(m)?,
        (None, Some(s)) => fetch::find_song(&s.title)?,
        (None, None) => return Err("this preset names no song: give one with --music".into()),
    };
    let (song, (videos, shots, frames)) = std::thread::scope(|s| {
        let song = s.spawn(|| montage::song(&music, cache));
        (song.join().unwrap_or_else(|_| Err("song thread crashed".into())), footage(c, cache))
    });
    let song = song?;
    if shots.is_empty() {
        return Err("no footage to cut".into());
    }
    lap(&format!("song \"{}\"; {} shots from {} video(s)", song.title, shots.len(), videos.len()));
    // where the song starts: the preset's own offset into its own song, else the loudest stretch
    let song_from = match (&c.music, &p.song) {
        (None, Some(s)) => s.from,
        _ => montage::windows(&song.beats, &song.loud, p.total, 1).first().copied().unwrap_or(0.0),
    };
    // the preset bends to the moment: its own intensity (or --intensity), and with another song its cuts on that song's beats
    let mut p = p.clone().with_intensity(c.intensity.unwrap_or(p.intensity));
    if c.music.is_some() && p.fit_beats {
        let beats: Vec<f64> = song.beats.iter().map(|b| b - song_from).filter(|b| *b >= 0.0 && *b <= p.total).collect();
        p = p.fit_to(&beats);
    }
    // --speed plays every shot faster or slower; --velocity gives each longer shot a speed ramp
    for s in &mut p.slots {
        s.speed = (s.speed * c.speed).clamp(0.1, 8.0);
        if c.velocity && s.ramp.is_empty() && s.end - s.start >= 0.45 {
            s.ramp = fx::velocity();
        }
    }
    // one piece per slot, end to end: a slot runs until the next one starts
    let starts: Vec<f64> = p.slots.iter().enumerate().map(|(i, s)| if i == 0 { 0.0 } else { s.start }).collect();
    let spans: Vec<(f64, f64)> = (0..p.slots.len()).map(|i| (starts[i], starts.get(i + 1).copied().unwrap_or(p.total))).filter(|(a, b)| b - a > 0.02).collect();
    let mut opts = c.opts.clone();
    if !c.sized {
        (opts.w, opts.h) = (p.w, p.h);
    }
    if let (false, Some(look)) = (c.looked, &p.look) {
        opts.grade = look.clone();
    }
    // the hardest beats the shots start on get the hits (--fx); the very hardest also freeze
    let strength: Vec<f64> = spans.iter().map(|(a, _)| montage::hit_at(&song.beats, &song.strength, song_from + a)).collect();
    let mut ranked = strength.clone();
    ranked.sort_by(|a, b| b.total_cmp(a));
    let cut_at = |q: f64| ranked.get(((ranked.len() as f64 * q) as usize).min(ranked.len().saturating_sub(1))).copied().unwrap_or(f64::MAX);
    let (hard, hardest) = (cut_at(0.35), cut_at(0.12));
    // words on screen (--lines): the channel's most quotable short sentences, one per stretch of the edit
    let pool = match (c.lines, c.line_texts.is_empty()) {
        (0, _) => vec![],
        (_, false) => c.line_texts.clone(),
        (n, true) => quote_lines(&videos, cache, c, n * c.n),
    };
    let mut used = vec![false; shots.len()];
    let mut made = 0;
    for k in 0..c.n {
        let slots: Vec<&preset::Slot> = spans.iter().map(|(a, _)| p.slots.iter().rfind(|s| s.start <= a + 0.02).unwrap_or(&p.slots[0])).collect();
        let lens: Vec<f64> = spans.iter().zip(&slots).map(|((a, b), s)| (s.end - s.start).max(b - a)).collect();
        let needs: Vec<f64> = slots.iter().zip(&lens).map(|(s, l)| s.source_len(*l) + 0.2).collect();
        let picks = montage::assign(&shots, &mut used, &needs, None);
        if picks.len() < needs.len() {
            eprintln!("edits: not enough usable shots for edit {}", k + 1);
            break;
        }
        let out = c.out.join(format!("{}-{:02}.mp4", fetch::safe(&p.name), k + 1));
        let work = cache.join("renders").join(format!("{}-{:02}", fetch::safe(&p.name), k + 1));
        std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into()];
        let mut g = String::new();
        let mut tl = timeline::Timeline { name: p.name.clone(), w: opts.w, h: opts.h, music: Some((song.file.clone(), song_from)), ..Default::default() };
        for (i, (((a, b), s), (shot, from))) in spans.iter().zip(&slots).zip(&picks).enumerate() {
            let v = shots[*shot].video;
            args.extend(["-ss".into(), format!("{from:.3}"), "-t".into(), format!("{:.3}", needs[i] + 0.2), "-i".into(), videos[v].1.to_string_lossy().into_owned()]);
            // the frame follows the face through the shot; without one it holds the shot's centre
            let focus = fx::track(&frames[v], 4.0, *from, needs[i]).map_or(format!("{:.3}", shots[*shot].focus.unwrap_or(0.5)), |cv| cv.expr("t", lens[i]));
            let mut pre = String::new();
            if let Some(bx) = c.blur_box {
                pre.push_str(&fx::blur_box(&format!("bb{i}"), bx));
            }
            if c.blur_faces {
                let (sw, sh) = fetch::dims(&videos[v].1);
                if let Some(bx) = fx::face_box(&mut face::boxes(&videos[v].1, *from, needs[i], sw, sh)) {
                    pre.push_str(&fx::blur_box(&format!("bf{i}"), bx));
                }
            }
            let hit = i > 0 && strength[i] >= hard;
            let extra = preset::Extra {
                focus,
                pre,
                post: if hit { fx::hit(&c.hits, opts.w, opts.h) } else { String::new() },
                freeze: (i > 0 && strength[i] >= hardest && fx::wants_freeze(&c.hits)).then_some(0.3),
            };
            g.push_str(&preset::chain(s, i, opts.w, opts.h, &extra, lens[i]).replacen(&format!("trim=duration={:.3}", lens[i]), &format!("trim=duration={:.3}", b - a), 1));
            tl.clips.push((videos[v].1.clone(), *from, b - a));
        }
        let m = spans.len();
        // each edit its own lines when there are enough, else they repeat
        let quotes: Vec<String> = if pool.is_empty() { vec![] } else { (0..c.lines.min(pool.len())).map(|j| pool[(k * c.lines + j) % pool.len()].clone()).collect() };
        let subs = talk::lines_ass(&quotes, p.total, &line_style(c)?, opts.w, opts.h);
        let has_subs = subs.contains("Dialogue:");
        if has_subs {
            std::fs::write(work.join("cap.ass"), &subs).map_err(|e| e.to_string())?;
        }
        (0..m).for_each(|i| g.push_str(&format!("[v{i}]")));
        g.push_str(&format!("concat=n={m}:v=1:a=0,settb=1/30,fps=30{}{}{},fade=t=out:st={:.3}:d=0.5", render::grade(&opts.grade), preset::flashes(&p),
            if has_subs { ",ass=cap.ass" } else { "" }, (p.total - 0.5).max(0.0)));
        if !c.look.tag.is_empty() {
            g.push_str(&format!(",drawtext=text='{}':font=Adwaita Sans:fontsize=h*0.022:fontcolor=white@0.7:x=(w-tw)/2:y=h*0.94", c.look.tag));
        }
        g.push_str(&format!(",format=yuv420p[vout];[{m}:a]afade=t=out:st={:.3}:d=0.6,loudnorm=I={:.1}:TP=-1.5[aout]", (p.total - 0.6).max(0.0), opts.lufs));
        args.extend(["-ss".into(), format!("{song_from:.3}"), "-t".into(), format!("{:.3}", p.total), "-i".into(), song.file.to_string_lossy().into_owned()]);
        args.extend(["-filter_complex".into(), g, "-map".into(), "[vout]".into(), "-map".into(), "[aout]".into()]);
        args.extend(["-t", &format!("{:.3}", p.total), "-fps_mode", "cfr", "-r", "30", "-c:v", "libx264", "-preset", "medium", "-crf", "20", "-maxrate", "8M", "-bufsize", "16M",
            "-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-movflags", "+faststart"].iter().map(|s| s.to_string()));
        args.push(std::path::absolute(&out).map_err(|e| e.to_string())?.to_string_lossy().into_owned());
        let _ = std::fs::write(work.join("ffmpeg.txt"), args.iter().map(|a| format!("'{}'", a.replace('\'', "'\\''"))).collect::<Vec<_>>().join(" "));
        match util::run(Some(&work), "ffmpeg", &args) {
            Ok(_) => {
                tl.captions = talk::lines_srt(&quotes, p.total);
                let fin = deliver(c, &out, &tl)?;
                let side = serde_json::json!({ "clip": fin, "preset": p.name, "song": song.title, "song_from": song_from, "seconds": p.total, "lines": quotes,
                    "shots": picks.iter().map(|(i, st)| serde_json::json!({ "video": videos[shots[*i].video].0, "from": st })).collect::<Vec<_>>() });
                std::fs::write(fin.with_extension("json"), serde_json::to_string_pretty(&side).unwrap()).map_err(|e| e.to_string())?;
                println!("{side}");
                made += 1;
            }
            Err(e) => eprintln!("edits: {e}"),
        }
    }
    lap(&format!("{made} edit(s) in {}", c.out.display()));
    if made == 0 {
        return Err("no edits made".into());
    }
    Ok(())
}

/// The last pass (stabilise, clean the voice, frame rate, codec) and the timelines for other editors.
fn deliver(c: &Cli, out: &Path, tl: &timeline::Timeline) -> Result<PathBuf, String> {
    svg::apply(out, &c.svgs)?;
    library::apply(out, &c.overlays)?;
    let fin = finish::apply(out, &c.finish)?;
    if !c.export.is_empty() {
        for p in tl.write(&fin, &c.export)? {
            eprintln!("edits: wrote {}", p.display());
        }
    }
    Ok(fin)
}

/// The caption style for words on screen: --captions when given, else a quiet one.
fn line_style(c: &Cli) -> Result<talk::CaptionStyle, String> {
    talk::style(if c.captions_given { &c.caption_style } else { "karaoke" })
}

/// Short sentences worth putting on screen, from the footage videos' captions: the model picks when it can,
/// else sentences of 4 to 10 words that carry feeling.
fn quote_lines(videos: &[(String, PathBuf)], cache: &Path, c: &Cli, n: usize) -> Vec<String> {
    let mut all: Vec<String> = vec![];
    for (id, _) in videos {
        if let Ok(pv) = fetch::preview(id, &format!("https://www.youtube.com/watch?v={id}"), cache, &c.lang) {
            all.extend(talk::sentences(&pv.text));
        }
    }
    talk::best_lines(&all, n, c.llm)
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
    let look = montage::Look { punch: !c.look.sad && template.is_none(), pace: c.speed, hits: c.hits.clone(), ..c.look.clone() };
    let music = match (&template, &c.music) {
        (Some(t), _) => t.music.to_string_lossy().into_owned(),
        (None, Some(m)) => fetch::music(m)?,
        (None, None) => return Err("--music or --like is needed".into()),
    };
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
            // a video named in --say is used whatever the screening thinks of it
            let wanted: Vec<(String, String)> = found.iter().filter(|(id, _)| c.say.iter().any(|w| w.starts_with(&format!("{id}@")))).cloned().collect();
            let mut found = fetch::screen(found, c.use_videos, cache, &c.lang, c.llm);
            found.extend(wanted.into_iter().filter(|w| !found.contains(w)).collect::<Vec<_>>());
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
            joins[1] = if c.look.sad { montage::Join { kind: "fadeblack", dur: 0.5, motion: None } } else { montage::Join { kind: "fadewhite", dur: 0.3, motion: None } };
        }
        if template.is_none() && voice.is_none() {
            cuts.dedup();
        }
        if template.is_none() && !c.look.sad {
            montage::add_moves(&mut joins, k);
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
                let tl = timeline::Timeline {
                    name: format!("{kind}-{:02}", p.k + 1), w: opts.w, h: opts.h,
                    clips: p.files.iter().zip(p.cuts.windows(2)).map(|((f, from, _), w)| (f.clone(), *from, w[1] - w[0])).collect(),
                    music: Some((song.file.clone(), p.start)),
                    captions: vec![],
                };
                let out = deliver(c, &out, &tl)?;
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

const SKILLS: &[(&str, &str)] = &[
    ("edits", include_str!("../skills/edits/SKILL.md")),
    ("edits-preset-tweak", include_str!("../skills/edits-preset-tweak/SKILL.md")),
    ("edits-trends", include_str!("../skills/edits-trends/SKILL.md")),
    ("edits-presets-import", include_str!("../skills/edits-presets-import/SKILL.md")),
    ("edits-mcp", include_str!("../skills/edits-mcp/SKILL.md")),
];

/// `edits skills install [DIR]`: the agent skills into ~/.claude/skills (or DIR). `edits skills` lists them.
fn skills(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("install") => {
            let dir = args.get(1).map(PathBuf::from).unwrap_or_else(|| util::home().join(".claude/skills"));
            for (name, text) in SKILLS {
                let d = dir.join(name);
                std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
                std::fs::write(d.join("SKILL.md"), text).map_err(|e| e.to_string())?;
            }
            println!("installed {} skills in {}", SKILLS.len(), dir.display());
            Ok(())
        }
        _ => {
            for (name, text) in SKILLS {
                let about = text.lines().find_map(|l| l.strip_prefix("description: ")).unwrap_or("");
                println!("{name:<22} {}", about.chars().take(110).collect::<String>());
            }
            println!("\ninstall them: edits skills install [DIR]   (default ~/.claude/skills)");
            Ok(())
        }
    }
}

/// `edits preset ...`: list, show, save, import. Presets are JSON: read one, change what the moment needs, save it.
fn preset_cmd(args: &[String]) -> Result<(), String> {
    match (args.first().map(String::as_str), args.get(1)) {
        (Some("list") | None, _) => {
            for p in presets::all() {
                println!("{:<40} {:>2} shots {:>5.1}s {}x{}{}", p.name, p.slots.len(), p.total, p.w, p.h, p.song.as_ref().map(|s| format!("  song: {}", s.title)).unwrap_or_default());
            }
            println!("\n{} cut moves; your presets: {}", presets::motions().len(), presets::user_dir().display());
            Ok(())
        }
        (Some("show"), Some(name)) => {
            println!("{}", preset::pretty(&preset::load(name)?));
            Ok(())
        }
        (Some("save"), Some(file)) => {
            let text = if file == "-" {
                let mut t = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut t).map_err(|e| e.to_string())?;
                t
            } else {
                std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?
            };
            let p: preset::Preset = serde_json::from_str(&text).map_err(|e| format!("not a valid preset: {e}"))?;
            if p.slots.is_empty() {
                return Err("a preset needs at least one shot".into());
            }
            std::fs::create_dir_all(presets::user_dir()).map_err(|e| e.to_string())?;
            let to = presets::user_dir().join(format!("{}.json", preset::slug(&p.name)));
            std::fs::write(&to, preset::pretty(&p)).map_err(|e| e.to_string())?;
            println!("saved \"{}\" to {}", p.name, to.display());
            Ok(())
        }
        (Some("import"), Some(_)) => {
            // --into DIR writes elsewhere (presets/ to build them in); default: your preset folder
            let (into, sources): (PathBuf, Vec<&String>) = match args.iter().position(|a| a == "--into") {
                Some(i) => (PathBuf::from(args.get(i + 1).ok_or("--into needs a folder")?), args[1..].iter().enumerate().filter(|(k, _)| *k + 1 != i && *k + 1 != i + 1).map(|x| x.1).collect()),
                None => (presets::user_dir(), args[1..].iter().collect()),
            };
            std::fs::create_dir_all(&into).map_err(|e| e.to_string())?;
            // a saved moves.json replaces the built-in one, so it starts from the built-in moves
            let mut moves: Vec<preset::Motion> = std::fs::read_to_string(into.join("moves.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_else(|| presets::motions().to_vec());
            let mut taken: Vec<String> = vec![];
            for src in sources {
                let path = Path::new(src);
                // Premiere exports are named x.prfpset or x.prfpset.xml
                let file = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
                if let Some(stem) = file.strip_suffix(".prfpset").or(file.strip_suffix(".prfpset.xml")) {
                    let xml = std::fs::read_to_string(path).map_err(|e| format!("{src}: {e}"))?;
                    let found = premiere::parse(&xml, stem)?;
                    println!("{src}: {} cut moves", found.len());
                    moves.retain(|m| !found.iter().any(|f| f.name == m.name));
                    moves.extend(found);
                    continue;
                }
                let found = preset::read_all(path)?;
                // no footage layers: maybe a colour pack, whose adjustment layers become looks
                let mut found = found;
                // no footage to fill, or one demo clip under many colour layers: a colour pack, whose looks we keep
                if found.iter().all(|p| p.slots.len() <= 1) && path.extension().is_some_and(|e| e.eq_ignore_ascii_case("xml")) {
                    let xml = std::fs::read_to_string(path).map_err(|e| format!("{src}: {e}"))?;
                    if let Some(gs) = grades::from_xml(&xml, &path.file_stem().unwrap_or_default().to_string_lossy()).ok().filter(|g| found.is_empty() || g.len() >= 2) {
                        let file = into.join("grades.json");
                        let mut all: Vec<grades::Grade> = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
                        all.retain(|g| !gs.iter().any(|n| n.name == g.name));
                        for g in &gs {
                            println!("grades.json: look {} ({} layers)", g.name, g.layers.len());
                        }
                        all.extend(gs);
                        found.clear();
                        std::fs::write(&file, preset::pretty(&all)).map_err(|e| e.to_string())?;
                    }
                }
                for p in found {
                    let mut slug = preset::slug(&p.name);
                    while taken.contains(&slug) {
                        slug.push('-');
                        slug.push('2');
                    }
                    taken.push(slug.clone());
                    std::fs::write(into.join(format!("{slug}.json")), preset::pretty(&p)).map_err(|e| e.to_string())?;
                    println!("{slug}.json: {} ({} shots, {:.1}s)", p.name, p.slots.len(), p.total);
                }
            }
            if !moves.is_empty() {
                moves.sort_by(|a, b| a.name.cmp(&b.name));
                std::fs::write(into.join("moves.json"), preset::pretty(&moves)).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
        _ => Err("usage: edits preset list | show NAME | save FILE.json|- | import <zip|xml|folder|prfpset>... [--into DIR]".into()),
    }
}

/// `edits rate clip.mp4 +|-` appends the clip's text to the taste file as a liked or disliked example.
fn rate(args: &[String]) -> Result<(), String> {
    let (clip, mark) = match args {
        [clip, m] if m == "+" || m == "-" => (Path::new(clip), lang::prompt("notes", if m == "+" { "liked" } else { "disliked" }, &[])),
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
        let c = p("recep ivedik komik -n 5 --aspect 1:1 --look dark --no-llm --fit").unwrap();
        assert_eq!((c.input.as_str(), c.n, c.opts.w, c.opts.h, c.opts.grade.as_str(), c.llm, c.opts.layout.as_str()), ("recep ivedik komik", 5, 1080, 1080, "dark", false, "fit"));
        assert!(p("-n 3").is_err());
        assert!(p("x -n 0").is_err());
        assert!(p("x --look pink").is_err());
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
