#![recursion_limit = "512"]
mod acp;
mod ai;
#[cfg(feature = "app")]
mod app;
mod browser;
mod capcut;
mod check;
mod create;
mod face;
mod fetch;
mod follow;
mod fonts;
mod finish;
mod fx;
mod grades;
mod hook;
mod lang;
mod lrc;
mod learn;
mod library;
mod looks;
mod mcp;
mod montage;
mod ort;
mod pick;
mod premiere;
mod packs;
mod preset;
mod presets;
mod render;
mod review;
mod rss;
mod shazam;
mod social;
mod source;
mod signals;
mod stats;
mod study;
mod subject;
mod svg;
mod talk;
mod timeline;
mod tools;
mod transcribe;
mod update;
mod util;
mod vod;
mod wrap;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = include_str!("usage.txt");

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // started as edits-app (the macOS bundle's link, a launcher): the app
    if std::env::args().next().is_some_and(|a| Path::new(&a).file_stem().is_some_and(|n| n == "edits-app")) {
        args.insert(0, "app".into());
    }
    // the guard runs before every shell command an agent makes: it checks nothing, fetches nothing
    if !matches!(args.first().map(String::as_str), Some("update" | "info" | "-V" | "--version" | "-h" | "--help" | "help" | "guard") | None) {
        update::auto();
    }
    tools::add_to_path();
    // anything that cuts, measures or downloads gets ffmpeg and yt-dlp first, fetched if missing
    let light = matches!(args.first().map(String::as_str), None | Some("update" | "info" | "-V" | "--version" | "-h" | "--help" | "help" | "doctor" | "app" | "settings" | "setup" | "mcp" | "skills" | "packs" | "looks" | "rate" | "trends" | "guard"))
        || (matches!(args.first().map(String::as_str), Some("preset" | "packs")) && args.get(1).map(String::as_str) != Some("create"));
    if !light {
        if let Err(e) = tools::ensure() {
            eprintln!("edits: could not download ffmpeg/yt-dlp ({e}); trying the ones this machine has");
        }
    }
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
        Some("app") => app(&args[1..]),
        Some("settings") => settings(),
        Some("rss") => rss::cmd(&args[1..]),
        Some("source") => source::cmd(&args[1..]),
        Some("encoder") => util::encoder_cmd(args.get(1).map(String::as_str)),
        Some("setup") => tools::ensure().map(|_| println!("edits: ffmpeg and yt-dlp ready")),
        Some("update") => update::cmd(&args[1..]),
        Some("info") => update::info(),
        Some("trends") => trends(&args[1..]),
        Some("rate") => rate(&args[1..]),
        Some("preset") => preset_cmd(&args[1..]),
        Some("library") if args.get(1).is_some_and(|a| a == "add") && args.len() > 2 => library_add(&args[2..]),
        Some("library") => {
            library::list(&args[1..]);
            Ok(())
        }
        Some("mcp") if args.get(1).is_some_and(|a| a == "install") => mcp::install(),
        Some("guard") => mcp::guard_cmd(&args[1..]),
        Some("mcp") => mcp::serve(),
        Some("skills") => skills(&args[1..]),
        Some("packs") => packs_cmd(&args[1..]),
        Some("looks") => {
            looks();
            Ok(())
        }
        Some("learn") if args.len() > 1 => learn_cmd(&args[1..]),
        Some("font") if args.len() > 1 => args[1..].iter().try_for_each(|f| {
            let fresh = fonts::add(f)?;
            println!("{f}: {}", if fresh { "installed from Google Fonts" } else { "already here" });
            Ok(())
        }),
        Some("cube") => cube(&args[1..]),
        Some("draw") if args.len() > 2 => draw(&args[1], &args[2..]),
        Some("check") if args.len() > 1 => fetch::reference(&args[1]).and_then(|p| check::check(&p)).map(|s| println!("{s}")),
        Some("words") if args.len() > 1 => words(&args[1..]),
        Some("moments") if args.len() > 1 => moments_cmd(&args[1..]),
        Some("song") if args.len() > 1 => shazam::cmd(&args[1..]),
        Some("cut") if args.len() > 2 => check::cut(&args[1], &args[2..], &auto_out(&args[1])).map(|v| v.iter().for_each(|f| println!("{}", f.display()))),
        Some("frames") if args.len() > 2 => check::frames(&args[1], &args[2..]).map(|s| println!("{s}")),
        Some("study") if args.len() > 1 => study::study(Path::new(&args[1])),
        Some("stats") if args.len() > 1 => stats::cmd(&args[1..]),
        Some("review") if args.len() > 1 => review::cmd(&args[1..]),
        Some("ai") => ai::cmd(&args[1..]),
        Some("clean") => clean(&args[1..]),
        Some("connect") if args.len() > 1 => social::connect(&args[1..]),
        Some("accounts") => {
            social::list();
            Ok(())
        }
        Some("disconnect") if args.len() > 1 => social::disconnect(&args[1]),
        Some("post") if args.len() > 1 => social::cmd(&args[1..]),
        Some("follow") if args.len() > 1 => follow::add(&args[1..]),
        Some("follows") if args.iter().any(|a| a == "--json") => {
            println!("{}", follows_json());
            Ok(())
        }
        Some("follows") => {
            follow::list();
            Ok(())
        }
        Some("unfollow") if args.len() > 1 => follow::remove(&args[1]),
        Some("watch") => follow::watch(&args[1..]),
        Some("service") => follow::service(&args[1..]),
        // a command named without what it needs says so, instead of searching YouTube for the word
        Some(cmd) if USAGE_OF.iter().any(|u| u.0 == cmd) => Err(format!("usage: {}", USAGE_OF.iter().find(|u| u.0 == cmd).map_or("", |u| u.1))),
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

/// How each command that needs more is called, said when it is given too little.
const USAGE_OF: &[(&str, &str)] = &[
    ("learn", "edits learn <edit.mp4|link>... [--into DIR]   an edit you like into a preset"),
    ("font", "edits font NAME...   a Google Fonts family for caption styles"),
    ("draw", "edits draw <video> NAME[@FROM-TO][:X,Y,SIZE][#COLOUR] ... [--overlay NAME[@FROM[-TO]][:OPACITY]] ..."),
    ("check", "edits check <clip.mp4|link>   length, loudness, cut rhythm and a contact sheet to look at"),
    ("words", "edits words <link|ID|file> [A-B] [--find TEXT]   what it says, line by line"),
    ("song", "edits song <video|link|ID> [AT]   which song plays there"),
    ("cut", "edits cut <video|link|ID> A-B...   stretches copied out without re-encoding"),
    ("frames", "edits frames <video|ID> AT[+LEN]...   rows of frames at those seconds (336, 5:36, 336+4)"),
    ("study", "edits study <folder of edits you like>"),
    ("review", "edits review <clip.mp4> [--notes]   the clip in the browser: point at moments and spots, the notes come back"),
    ("connect", "edits connect instagram|tiktok|youtube | bluesky HANDLE | facebook PAGE_ID   connect an account to post to"),
    ("disconnect", "edits disconnect ACCOUNT   forget an account and its sign-in"),
    ("post", "edits post <clip.mp4>... [--to ACCOUNT,...|all] [--caption TEXT] [--private]"),
    ("follow", "edits follow <YouTube channel | Kick channel | RSS/Atom feed> [--every MIN] [--post ACCOUNT,...] [--private] [edit options]"),
    ("unfollow", "edits unfollow N|SOURCE   (edits follows lists them)"),
    ("stats", "edits stats <@channel | channel or profile link> [N]   which posted edits worked, and what made them"),
];

/// `edits trends [spotify] [country]`
fn trends(args: &[String]) -> Result<(), String> {
    let spotify = args.first().is_some_and(|a| a == "spotify");
    let kind = if spotify { "spotify" } else { "trend" };
    let cc = args.get(usize::from(spotify)).cloned().unwrap_or_else(|| lang::system().1);
    let list = fetch::trends(kind, &cc)?;
    list.iter().enumerate().take(30).for_each(|(i, s)| println!("{:>2}. {s}", i + 1));
    println!("\nuse one: --music {kind}:N  (another country: --music {kind}:us:N)");
    Ok(())
}

/// `edits library add <zip|folder>...`: the pack's media into the library, then its presets (Alight projects,
/// Premiere moves) like preset import.
fn library_add(srcs: &[String]) -> Result<(), String> {
    for src in srcs {
        let root = library::add(Path::new(src))?;
        let mut import = vec!["import".to_string(), root.to_string_lossy().into_owned()];
        import.extend(library::walk(&root).iter().map(|f| f.to_string_lossy().into_owned()).filter(|f| f.to_lowercase().contains(".prfpset")));
        preset_cmd(&import)?;
    }
    Ok(())
}

/// `edits clean [--all] [--dry-run]`: frees the disk the downloaded videos take, as Tehom's cleanup does. Transcripts
/// and video info stay, so a video used again downloads but is not transcribed again; files used in the last day stay
/// unless --all (a render may be using them).
fn clean(args: &[String]) -> Result<(), String> {
    let (all, dry) = (args.iter().any(|a| a == "--all"), args.iter().any(|a| a == "--dry-run"));
    let day = std::time::Duration::from_secs(86_400);
    let mut stack = vec![util::home().join(".cache/edits")];
    let (mut n, mut bytes) = (0u64, 0u64);
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let (p, m) = (e.path(), match e.metadata() { Ok(m) => m, Err(_) => continue });
            if m.is_dir() {
                stack.push(p);
                continue;
            }
            let media = p.extension().is_some_and(|x| ["mp4", "m4a", "webm", "mkv", "mov", "wav", "mp3", "opus", "part"].contains(&x.to_string_lossy().to_lowercase().as_str()));
            let old = m.modified().ok().and_then(|t| t.elapsed().ok()).is_some_and(|age| age > day);
            if media && (all || old) && (dry || std::fs::remove_file(&p).is_ok()) {
                n += 1;
                bytes += m.len();
            }
        }
    }
    println!("{} {n} downloaded file(s), {:.1} GB{}", if dry { "would delete" } else { "deleted" }, bytes as f64 / 1e9,
        if all { "" } else { " (files used in the last day stay; --all takes them too)" });
    Ok(())
}

/// `edits looks`: everything an edit can be styled with.
fn looks() {
    println!("colour (--look):");
    looks::LOOKS.iter().for_each(|(n, about, _)| println!("  {n:<12} {about}"));
    grades::all().iter().for_each(|g| println!("  {:<12} {}", g.name, g.about));
    println!("\ncaptions (--captions):");
    talk::styles().iter().for_each(|st| println!("  {:<12} {}", st.name, st.about));
    println!("\nbeat hits (--fx a,b,...): {}", fx::HITS.join(", "));
    println!("\ndrawn graphics (--svg NAME[@FROM-TO][:X,Y,SIZE][#COLOUR], or your own .svg):");
    svg::STICKERS.iter().for_each(|(n, about, _)| println!("  {n:<12} {about}"));
}

/// `edits learn <edit|link>... [--into DIR]`: each edit as a preset (default: your preset folder), and its song.
fn learn_cmd(args: &[String]) -> Result<(), String> {
    let at = args.iter().position(|a| a == "--into");
    let into = at.and_then(|i| args.get(i + 1)).map(PathBuf::from).unwrap_or_else(presets::user_dir);
    std::fs::create_dir_all(&into).map_err(|e| e.to_string())?;
    for (_, v) in args.iter().enumerate().filter(|(i, _)| at.is_none_or(|a| *i != a && *i != a + 1)) {
        let path = &fetch::reference(v)?;
        let name = path.file_stem().unwrap_or_default().to_string_lossy().into_owned();
        let p = learn::learn(path, &name)?;
        let file = into.join(format!("{}.json", preset::slug(&name)));
        std::fs::write(&file, preset::pretty(&p)).map_err(|e| e.to_string())?;
        println!("{}: {} ({:.1}s) -> --preset {}", file.display(), p.about, p.total, preset::slug(&name));
        if let Ok(Some(f)) = shazam::recognize(path, None) {
            println!("  its song: {} - {} (--music \"{} - {}\")", f.artist, f.title, f.artist, f.title);
        }
    }
    Ok(())
}

/// `edits cube LOOK [out.cube]`: an imported colour pack look as a .cube LUT.
fn cube(args: &[String]) -> Result<(), String> {
    let g = args.first().and_then(|n| grades::find(n)).ok_or("usage: edits cube <look from an Alight cc pack> [out.cube]   (edits looks lists them)")?;
    let made = grades::lut(&g, &std::env::temp_dir().join("edits-cube"))?;
    let to = args.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from(format!("{}.cube", g.name)));
    std::fs::copy(made.trim_start_matches(",lut3d=file="), &to).map_err(|e| e.to_string())?;
    println!("{}", to.display());
    Ok(())
}

/// `edits draw <video> SPEC... [--overlay SPEC]...`: graphics and library clips over a finished clip, as NAME-drawn.mp4.
fn draw(v: &str, specs: &[String]) -> Result<(), String> {
    // "--overlay SPEC" items are library clips, the rest drawn graphics
    let over: Vec<usize> = (1..specs.len()).filter(|&i| specs[i - 1] == "--overlay").collect();
    let ov = over.iter().map(|&i| library::parse(&specs[i])).collect::<Result<Vec<_>, _>>()?;
    let st = (0..specs.len()).filter(|i| !over.contains(i) && specs[*i] != "--overlay").map(|i| svg::parse(&specs[i])).collect::<Result<Vec<_>, _>>()?;
    let src = Path::new(v);
    let out = src.with_file_name(format!("{}-drawn.mp4", src.file_stem().unwrap_or_default().to_string_lossy()));
    std::fs::copy(src, &out).map_err(|e| format!("{v}: {e}"))?;
    svg::apply(&out, &st)?;
    library::apply(&out, &ov)?;
    println!("{}", out.display());
    Ok(())
}

struct Cli {
    /// how words and drawings come in (`svg::ANIMS`)
    anim: String,
    /// music edits: this xfade transition on every cut that is not a hard cut
    transition: Option<&'static str>,
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
    /// --preview: one edit at half size, encoded fast, to check before the full render
    preview: bool,
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
    /// VIDEO@FROM: the footage for each shot of a preset edit, in order, by hand (only the character, the right scene)
    shots: Vec<String>,
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
    /// timelines to write beside each clip: fcpxml, edl, srt, vtt
    export: Vec<String>,
    finish: finish::Finish,
    jumpcuts: bool,
    sfx: bool,
    hook: bool,
    bed: Option<String>,
    /// words on screen at given times (--caption TEXT@FROM-TO), emoji drawn in colour
    captions_at: Vec<(String, f64, f64)>,
    /// the spoken line drawn on the picture as it is said
    say_text: bool,
    /// math flashing in on the cuts
    math: bool,
    /// --formula: the math layer's own formulas, LaTeX already turned to Unicode
    formulas: Vec<String>,
    /// one-frame lines on black (--flash-text SECONDS:TEXT)
    flash_texts: Vec<(f64, String)>,
    /// an intro before the edit and an outro after it
    around: wrap::Around,
    /// --loop: the end runs into the start (picture and song), no fades
    looped: bool,
    /// the colour look as asked for (opts.grade becomes a LUT filter for imported looks), for the render log
    look_name: String,
    /// the person cut out: another background, words behind them, an outline
    subject: subject::Subject,
    /// --lyrics: the song's words on screen as they are sung, cuts on sung words
    lyrics: bool,
    /// --long: one edit over a minute, what TikTok's Creator Rewards pays for
    long: bool,
    /// --corecore: many unrelated videos, faded and grainy, headlines from their own words
    corecore: bool,
    /// --translate LANG: talking clips' captions in another language
    translate: Option<String>,
}

/// --long: TikTok's Creator Rewards pays for videos over a minute; a second of margin over it.
const LONG: f64 = 61.0;

fn size(aspect: &str) -> Option<(u32, u32)> {
    study::ASPECTS.iter().find(|a| a.0 == aspect).map(|a| (a.1, a.2))
}

/// Defaults come from `edits study` when it has been run; flags override them.
fn parse(args: &[String], style: Option<study::Style>) -> Result<Cli, String> {
    let style = style.unwrap_or(study::Style { aspect: "9:16".into(), grade: "bw".into(), lufs: -14.0, sfx: true, hook: true });
    let (w, h) = size(&style.aspect).unwrap_or((1080, 1920));
    let grade = if looks::names().contains(&style.grade.as_str()) { style.grade } else { "bw".into() };
    let mut c = Cli {
        anim: String::new(),
        transition: None,
        input: String::new(), n: 3, videos: 5, lang: lang::code().into(), out: PathBuf::new(),
        opts: render::Opts { w, h, grade, captions: true, layout: "fill".into(), lufs: style.lufs.clamp(-30.0, -5.0), cam: None },
        llm: true, join: false, music: None, like: None, preset: None, lut: None, sized: false, preview: false, looked: false, intensity: None, use_videos: 4, voice: false, say: vec![], shots: vec![], caption_style: "hormozi".into(), captions_given: false, speed: 1.0, velocity: false, hits: vec![], blur_faces: false, blur_box: None, svgs: vec![], overlays: vec![], whoosh: None, boom: None, lines: 0, line_texts: vec![], export: vec![], finish: finish::Finish::default(), jumpcuts: true, sfx: style.sfx, hook: style.hook, bed: None, captions_at: vec![], say_text: false, math: false, formulas: vec![], flash_texts: vec![], around: wrap::Around::default(), looped: false, look_name: String::new(), subject: subject::Subject::default(), lyrics: false, long: false, corecore: false, translate: None, look: montage::Look { sad: false, punch: true, length: 0.0, tag: String::new(), text: String::new(), pace: 1.0, hits: vec![], looped: false, core: false },
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
            "--encoder" => util::set_encoder(&val("--encoder")?)?,
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
            "--layout" => {
                let v = val("--layout")?;
                if !["fill", "fit", "stack", "inset"].contains(&v.as_str()) {
                    return Err(format!("--layout {v}: fill, fit, stack (facecam over the game) or inset (facecam small on top)"));
                }
                c.opts.layout = v;
            }
            "--music" => c.music = Some(val("--music")?),
            "--like" => c.like = Some(fetch::reference(&val("--like")?)?.to_string_lossy().into_owned()),
            "--preset" => c.preset = Some(val("--preset")?),
            "--sad" => c.look.sad = true,
            "--voice" => c.voice = true,
            "--shot" => c.shots.push(val("--shot")?),
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
                if let Some(bad) = c.export.iter().find(|x| !["fcpxml", "edl", "srt", "vtt"].contains(&x.as_str())) {
                    return Err(format!("--export: {bad} is not fcpxml, edl, srt or vtt"));
                }
            }
            "--pip" => {
                let mut o = library::parse(&val("--pip")?)?;
                o.place = o.place.or(Some((0.74, 0.22, 0.4)));
                c.overlays.push(o);
            }
            "--transition" => {
                let t = val("--transition")?;
                c.transition = Some(montage::XFADES.iter().find(|x| **x == t).copied().ok_or(format!("--transition must be one of {}", montage::XFADES.join(", ")))?);
            }
            "--anim" => {
                c.anim = val("--anim")?;
                if !svg::ANIMS.contains(&c.anim.as_str()) {
                    return Err(format!("--anim must be one of {}", svg::ANIMS.join(", ")));
                }
            }
            "--fix" => render::set_fix(),
            "--curves" => {
                c.finish.curves = val("--curves")?;
                fx::curves(&c.finish.curves).ok_or(format!("--curves must be one of {} or points like \"0/0 0.4/0.3 1/1\"", fx::CURVES.join(", ")))?;
            }
            "--pop" => {
                c.finish.pop = val("--pop")?.to_lowercase();
                fx::pop(&c.finish.pop).ok_or(format!("--pop must be one of {}", fx::POP.iter().map(|p| p.0).collect::<Vec<_>>().join(", ")))?;
            }
            "--bass" => c.finish.bass = val("--bass")?.parse().ok().filter(|d| (-12.0..=15.0).contains(d)).ok_or("--bass must be -12 to 15 dB")?,
            "--pitch" => c.finish.pitch = val("--pitch")?.parse().ok().filter(|d| (-12.0..=12.0).contains(d)).ok_or("--pitch must be -12 to 12 semitones")?,
            "--reverb" => c.finish.reverb = true,
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
            // style.json can turn these off for good; the flags bring them back for one run
            "--sfx" => c.sfx = true,
            "--hook" => c.hook = true,
            "--bed" => c.bed = Some(val("--bed")?),
            "--caption" => c.captions_at.push(wrap::parse_caption(&val("--caption")?)?),
            "--say-text" => c.say_text = true,
            "--math" => c.math = true,
            "--formula" => {
                c.formulas.push(wrap::tex(&val("--formula")?));
                c.math = true;
            }
            "--flash-text" => c.flash_texts.push(wrap::parse_flash(&val("--flash-text")?)?),
            "--intro" => c.around.intro = Some(wrap::parse_intro(&val("--intro")?)?),
            "--intro-bed" => c.around.intro_bed = Some(val("--intro-bed")?),
            "--intro-caption" => c.around.intro_caption = Some(val("--intro-caption")?),
            "--outro" => c.around.outro = Some(val("--outro")?),
            "--no-captions" => c.opts.captions = false,
            "--no-llm" => c.llm = false,
            "--join" => c.join = true,
            "--preview" => c.preview = true,
            "--bg" => c.subject.bg = Some(subject::parse_bg(&val("--bg")?)?),
            "--behind" => c.subject.behind.push(subject::parse_behind(&val("--behind")?)?),
            "--outline" => {
                let v = val("--outline")?;
                c.subject.outline = Some(subject::colour(&v).ok_or(format!("--outline {v}: give a colour, like white or #ff2a6d"))?);
            }
            "--lyrics" => c.lyrics = true,
            "--translate" => c.translate = Some(val("--translate")?),
            "--long" => c.long = true,
            "--corecore" => {
                c.corecore = true;
                c.look.core = true;
            }
            "--loop" => {
                c.looped = true;
                c.look.looped = true;
            }
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
    if c.corecore {
        if c.music.is_none() {
            return Err("--corecore needs a song: --music (slow and sad works best)".into());
        }
        // many sources, little from each
        if !args.iter().any(|a| a == "--videos") {
            c.videos = 12;
        }
        if !args.iter().any(|a| a == "--use") {
            c.use_videos = 8;
        }
        // headlines from the videos' own words, in boxes like a news ticker
        if c.lines == 0 && c.opts.captions {
            c.lines = 3;
        }
        if !c.captions_given {
            c.caption_style = "box".into();
            c.captions_given = true;
        }
    }
    if c.long {
        if c.look.length > 0.0 && c.look.length < LONG {
            return Err(format!("--long makes edits over a minute: drop --length, or give {LONG:.0}-90"));
        }
        if c.look.length == 0.0 {
            c.look.length = LONG + 3.0;
        }
        // talking clips: enough moments to choose a minute from (music and preset edits make one long edit each)
        if c.music.is_none() && c.like.is_none() && c.preset.is_none() && !args.iter().any(|a| a == "-n" || a == "--moments") {
            c.n = 10;
        }
    }
    c.look_name = c.lut.as_ref().map_or(c.opts.grade.clone(), |l| l.file_stem().unwrap_or_default().to_string_lossy().into_owned());
    if c.corecore && !c.looked {
        c.look_name = "corecore".into();
    }
    if c.look.length == 0.0 {
        c.look.length = if c.look.sad { 24.0 } else { 18.0 };
    }
    if c.preview {
        c.n = 1;
    }
    if c.looped && (c.around.intro.is_some() || c.around.outro.is_some()) {
        return Err("--loop plays round: it takes no --intro or --outro".into());
    }
    if c.looped && c.music.is_none() && c.like.is_none() && c.preset.is_none() {
        return Err("--loop is for music and preset edits (--music, --like or --preset)".into());
    }
    Ok(c)
}

/// Longer than this (seconds), a video's moments are found from its scored windows first.
const STREAM: f64 = 2700.0;

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
/// --preview halves the frame (even sides, as encoders want); everything placed on it scales with it.
fn preview_size(c: &Cli, o: &mut render::Opts) {
    if c.preview {
        (o.w, o.h) = (o.w / 4 * 2, o.h / 4 * 2);
    }
}

/// The videos named by `--say ID@A-B`, each once.
fn say_ids(c: &Cli) -> Vec<&str> {
    let mut ids: Vec<&str> = c.say.iter().filter_map(|w| w.rsplit_once('@').map(|x| x.0)).collect();
    ids.sort();
    ids.dedup();
    ids
}

/// Each video's words and passages, fetched side by side: the input file when the id is its name (it speaks only
/// then), any other id a YouTube video. A video that fails is reported and left out.
fn analyse_ids(ids: &[&str], c: &Cli, cache: &Path) -> Vec<Analysed> {
    let input = Path::new(&c.input);
    std::thread::scope(|s| {
        let jobs: Vec<_> = ids.iter().map(|&id| s.spawn(move || {
            let src = if input.is_file() && input.file_stem().is_some_and(|s| s == id) { fetch::local(input, cache) } else { fetch::remote(id, &format!("https://www.youtube.com/watch?v={id}"), cache) };
            src.map(|src| analyse(src, c, false))
        })).collect();
        jobs.into_iter().filter_map(|j| j.join().ok()?.map_err(|e| eprintln!("edits: no voice from a video: {e}")).ok()).collect()
    })
}

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
    // --say ID@FROM-TO in a talking-clip run: that stretch, widened to whole words, instead of the pick
    let by_hand: Vec<pick::Moment> = c.say.iter().filter_map(|w| {
        let (id, range) = w.split_once('@')?;
        let (a, b) = range.split_once('-')?;
        let (a, b): (f64, f64) = (a.parse().ok()?, b.parse().ok()?);
        (id == src.id).then(|| widen(&t.words, a, b)).flatten()
    }).collect();
    let moments = if !clips {
        vec![]
    } else if !by_hand.is_empty() {
        by_hand
    } else if src.duration > STREAM {
        // a stream or a podcast: hours of transcript do not fit the model, so it reads only the stretches the
        // loudness, reactions and chat point at
        let spoken = if src.language.is_empty() { c.lang.clone() } else { src.language.clone() };
        let (_, _, ws) = scan(&src, transcribe::Transcript { words: t.words.clone(), ..Default::default() }, &spoken);
        let cands: Vec<pick::Moment> = vod::best(&ws, c.n * 3).iter().map(vod::moment).collect();
        let near: Vec<transcribe::Line> = lines.iter().filter(|l| cands.iter().any(|m| l.e > m.start && l.s < m.end)).map(|l| transcribe::Line { s: l.s, e: l.e, text: l.text.clone() }).collect();
        eprintln!("edits: a long video: the model reads the {} best of {} stretches", cands.len(), ws.len());
        pick::pick(&src.title, &near, &curve, &hits, c.n, src.duration, c.llm)
    } else {
        pick::pick(&src.title, &lines, &curve, &hits, c.n, src.duration, c.llm)
    };
    eprintln!("edits: {} ({}): words from {}, {comments} timestamped comments, {} laughs, replay heat {}, {} moments",
        src.title, util::mmss(src.duration), if t.from.is_empty() { "nothing" } else { t.from }, t.laughs.len(), if src.heat.is_empty() { "no" } else { "yes" }, moments.len());
    Analysed { src, words: t.words, lines, moments, curve }
}

/// Downloads the picture for one moment and renders it.
fn cut(a: &Analysed, m: &pick::Moment, c: &Cli, bed: Option<&(PathBuf, f64)>, cache: &Path, out: &Path) -> Result<serde_json::Value, String> {
    let (video, seek) = fetch::clip(&a.src, m.start, m.end)?;
    let (w, h) = fetch::dims(&video);
    let mut opts = c.opts.clone();
    preview_size(c, &mut opts);
    // a stream with a facecam in a corner: cropping around that small face would show the edge of the game, so the
    // face goes over the game (stack) unless another layout was asked for
    let wide = w * opts.h > h * opts.w;
    if wide && ["fill", "stack", "inset"].contains(&opts.layout.as_str()) {
        opts.cam = face::cam_box(&video, seek, m.end - m.start, w, h);
        match (opts.cam.is_some(), opts.layout.as_str()) {
            (true, "fill") => opts.layout = "stack".into(),
            (false, "stack" | "inset") => {
                eprintln!("edits: no facecam found in {}; the frame follows the faces instead", util::mmss(m.start));
                opts.layout = "fill".into();
            }
            _ => {}
        }
    }
    let focus = if opts.layout == "fill" { face::center_x(&video, seek, m.end - m.start, w, h) } else { None };
    // the subtitle file is per clip: clips render at the same time
    let work = a.src.work.join(fetch::safe(&out.file_stem().unwrap_or_default().to_string_lossy()));
    std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
    // nobody to frame on (screen recordings, graphics): cropping would cut content at random, so keep the whole frame
    if opts.layout == "fill" && focus.is_none() && wide {
        opts.layout = "fit".into();
    }
    // the hook: the model's title for the moment, else its first words
    // --translate: the captions, the hook and the .srt in that language, the voice as it was
    let (shown, title) = match &c.translate {
        Some(to) => talk::translated(&a.words, m, to).unwrap_or_else(|e| {
            eprintln!("edits: captions not translated ({e}); they stay as said");
            (vec![], m.title.clone())
        }),
        None => (vec![], m.title.clone()),
    };
    let said = if shown.is_empty() { &a.words[..] } else { &shown[..] };
    // its first sentence, if short enough to read in a glance
    let mut first: Vec<&str> = vec![];
    for w in said.iter().filter(|w| w.s >= m.start && w.e <= m.end) {
        first.push(&w.t);
        if w.t.ends_with(['.', '?', '!']) || first.len() == 9 {
            break;
        }
    }
    let first = first.join(" ");
    let hook = c.hook.then(|| if title.trim().is_empty() { first } else { title.clone() });
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
        defer_subs: c.subject.any(),
        shown: shown.clone(),
    };
    let segs = talk::render(&video, seek, m, &a.words, &opts, &t, focus.unwrap_or(0.5), cache, &work, out)?;
    let tl = timeline::Timeline {
        name: a.src.title.clone(), w: opts.w, h: opts.h,
        clips: segs.iter().map(|(x, y)| (video.clone(), x - m.start + seek, y - x)).collect(),
        music: bed.cloned(),
        captions: transcribe::lines(&said.iter().filter(|w| w.s >= m.start && w.e <= m.end).cloned().collect::<Vec<_>>()).iter()
            .filter_map(|l| Some((talk::remap(l.s, &segs)? / c.speed, talk::remap(l.e, &segs).unwrap_or(talk::remap(l.s, &segs)? + 1.0) / c.speed, l.text.clone()))).collect(),
        said: vec![],
        markers: vec![],
        drawn: vec![],
        ass: if c.subject.any() { std::fs::read_to_string(work.join("cap.ass")).unwrap_or_default() } else { String::new() },
    };
    let out = &deliver(c, out, &tl)?;
    let text = a.lines.iter().filter(|l| l.s >= m.start - 0.5 && l.e <= m.end + 0.5).map(|l| l.text.as_str()).collect::<Vec<_>>().join(" ");
    keep(c, out, serde_json::json!({ "clip": out, "kind": "clip", "source": a.src.title, "id": a.src.id, "moment": m, "text": text, "seconds": fetch::duration(out).ok(), "framed_on_faces": focus.is_some() }))
}

/// $EDITS_OUT or ~/Videos/edits: where runs go.
pub fn out_base() -> PathBuf {
    std::env::var_os("EDITS_OUT").map(PathBuf::from).unwrap_or_else(|| util::home().join("Videos/edits"))
}

/// `edits settings --json`: everything the app's settings show, in one go; each is changed by its own command.
fn settings() -> Result<(), String> {
    println!("{}", settings_json());
    Ok(())
}

/// What the settings show: the AI and its providers, the encoder, updates, where edits go, the accounts.
pub fn settings_json() -> serde_json::Value {
    let c = ai::choice();
    let accounts: Vec<serde_json::Value> = social::all().iter().map(|a| serde_json::json!({"name": a.name, "platform": a.platform, "user": a.user})).collect();
    let sites: Vec<String> = packs::all().iter().filter(|p| packs::trusted(p)).filter_map(|p| p.manifest.automation.as_ref().map(|a| a.site.clone())).collect();
    serde_json::json!({"version": env!("CARGO_PKG_VERSION"), "ai": {"provider": c.provider, "model": c.model, "endpoint": c.endpoint, "about": ai::describe(), "ready": ai::ready()},
        "providers": ai::providers(), "encoder": util::encoder_choice(), "encoders": util::ENCODERS, "updates": update::auto_on(), "out": out_base(),
        "accounts": accounts, "sites": sites})
}

/// Every pack here, on or off, with its cover: what the marketplace shows.
pub fn packs_json() -> serde_json::Value {
    let off = packs::disabled();
    serde_json::Value::Array(packs::every().iter().map(|p| {
        let m = &p.manifest;
        let built_in = m.id.starts_with("edits.");
        let preset = p.preset.as_deref().and_then(|t| serde_json::from_str::<preset::Preset>(t).ok());
        let file = m.preview.as_ref().map(|f| packs::dir().join(&m.id).join(f)).filter(|f| !built_in && f.is_file());
        serde_json::json!({"id": m.id, "name": m.name, "version": m.version, "author": m.author, "description": m.description,
            "kinds": m.kinds, "tags": m.tags, "trusted": packs::trusted(p),
            "site": m.automation.as_ref().map(|a| &a.site), "preset": preset.as_ref().map(|p| &p.name), "feeds": m.feeds,
            "enabled": !off.contains(&m.id), "origin": packs::origin(&m.id).name(), "source": m.source.is_some(),
            // when it came here, for "newest first": the folder's time for an added pack
            "added": std::fs::metadata(packs::dir().join(&m.id)).and_then(|x| x.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs()),
            "previewFile": file, "previewSvg": if file.is_none() { Some(packs::preview(m, preset.as_ref())) } else { None }})
    }).collect())
}

/// The marketplace's list (packs.json), with what is added here.
pub fn listed_json(list: &[packs::Entry]) -> serde_json::Value {
    let have: Vec<String> = packs::added().into_iter().map(|p| p.manifest.id).collect();
    serde_json::Value::Array(list.iter().map(|e| serde_json::json!({"id": e.id, "name": e.name, "description": e.description, "kinds": e.kinds, "repo": e.repo, "added": have.contains(&e.id)})).collect())
}

/// Each follow as a flow: source, edit, accounts.
pub fn follows_json() -> serde_json::Value {
    serde_json::Value::Array(follow::all().iter().map(|f| serde_json::json!({"source": f.source, "args": f.args, "post": f.post, "every": f.every, "titles": f.titles, "last": f.last})).collect())
}

/// Where a run's clips go without --out: one folder per run, the same place whoever starts edits (the CLI in any
/// folder, the MCP server): $EDITS_OUT or ~/Videos/edits, then the day, then what it was made from (every run on
/// the same footage that day sits together), then the run's "HH.MM".
fn auto_out(input: &str) -> PathBuf {
    let base = out_base();
    let (day, time) = util::now().unwrap_or(("undated".into(), "00.00".into()));
    let dir = base.join(day).join(folder_name(&subject(input))).join(time);
    // a second run in the same minute on the same footage gets its own folder, never overwrites
    (1..).map(|k| if k == 1 { dir.clone() } else { PathBuf::from(format!("{} ({k})", dir.display())) }).find(|d| !d.exists()).unwrap()
}

/// What the edit was made from, in words: a file's name, a video's or channel's title, or the topic searched.
fn subject(input: &str) -> String {
    let p = Path::new(input);
    if p.is_file() {
        return p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    }
    // a bare video ID: the title saved when it was fetched, else YouTube's; never the ID itself as a folder
    let is_id = input.len() == 11 && input.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if is_id {
        if let Some(t) = cached_title(input) {
            return t;
        }
    }
    let url = if is_id { format!("https://www.youtube.com/watch?v={input}") } else { input.to_string() };
    if url.starts_with("http://") || url.starts_with("https://") {
        let title = util::yt(&["--no-warnings", "--flat-playlist", "--playlist-items", "1", "--print", "%(playlist_title,title)s", "--", &url]);
        return title.ok().and_then(|t| t.lines().next().map(str::to_string)).filter(|t| !t.trim().is_empty() && t != "NA").unwrap_or_else(|| input.to_string());
    }
    input.to_string()
}

/// A fetched video's title, from the info yt-dlp left in the cache.
fn cached_title(id: &str) -> Option<String> {
    let t = std::fs::read_to_string(util::home().join(".cache/edits").join(fetch::safe(id)).join("src.info.json")).ok()?;
    serde_json::from_str::<serde_json::Value>(&t).ok()?.get("title")?.as_str().map(str::to_string)
}

/// A title as a folder name: words kept (any alphabet), path and shell characters dropped, at most 60 characters.
fn folder_name(title: &str) -> String {
    let words: String = title.chars().map(|c| if c.is_alphanumeric() || " -_.'&()".contains(c) { c } else { ' ' }).collect();
    // whole words up to 60 characters; a single longer word is cut
    let mut name = String::new();
    for w in words.split_whitespace() {
        if !name.is_empty() && name.chars().count() + 1 + w.chars().count() > 60 {
            break;
        }
        name = if name.is_empty() { w.chars().take(60).collect() } else { format!("{name} {w}") };
    }
    let name = name.trim_matches(|c: char| c == '.' || c == ' ').to_string();
    if name.is_empty() { "edit".into() } else { name }
}

fn make(args: &[String]) -> Result<(), String> {
    let mut cli = parse(args, study::load())?;
    if cli.preview {
        util::set_fast();
        eprintln!("edits: preview: one edit at half size, encoded fast; drop --preview for the full render");
    }
    let cache = &util::home().join(".cache/edits");
    std::fs::create_dir_all(cache).map_err(|e| e.to_string())?;
    if let Some(l) = cli.lut.clone() {
        cli.opts.grade = looks::lut(&l, cache)?;
    } else if let Some(g) = grades::find(&cli.opts.grade) {
        cli.opts.grade = grades::lut(&g, cache)?;
    }
    // ffmpeg runs from work folders, so a relative footage path must become absolute
    if let Ok(abs) = std::fs::canonicalize(&cli.input).map_err(|_| ()).and_then(|p| if p.is_file() { Ok(p) } else { Err(()) }) {
        cli.input = abs.to_string_lossy().into_owned();
    }
    let auto = cli.out.as_os_str().is_empty();
    if auto {
        cli.out = auto_out(&cli.input);
    }
    let c = &cli;
    std::fs::create_dir_all(&c.out).map_err(|e| e.to_string())?;
    // a run that made nothing leaves no empty folder behind (remove_dir only removes an empty one)
    struct RemoveIfEmpty(Option<PathBuf>);
    impl Drop for RemoveIfEmpty {
        fn drop(&mut self) {
            if let Some(d) = &self.0 {
                let _ = std::fs::remove_dir(d);
            }
        }
    }
    let _cleanup = RemoveIfEmpty(auto.then(|| c.out.clone()));
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
    let bed_track = bed_song.as_ref().map(|s| (s.file.clone(), s.starts(60.0, 1).first().copied().unwrap_or(0.0)));
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
    // --long: as many of the best as it takes to pass a minute together (pauses cut take ~15% off)
    let n = if c.long {
        let mut sum = 0.0;
        best.iter().take_while(|(_, m)| { let more = sum < LONG + 3.0; sum += (m.end - m.start) * 0.85 / c.speed; more }).count().clamp(1, 20)
    } else { c.n };
    best.truncate(n);
    let jobs: Vec<(PathBuf, &Analysed, &pick::Moment)> = best.iter().enumerate()
        .map(|(k, (a, m))| (c.out.join(format!("{:02} {}.mp4", k + 1, folder_name(if m.title.is_empty() { &a.src.title } else { &m.title }))), *a, *m)).collect();
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
    if (c.join || c.long) && clips.len() > 1 {
        let out = c.out.join(if c.long { "long.mp4" } else { "joined.mp4" });
        render::join(&clips, cache, &out)?;
        let secs = fetch::duration(&out).unwrap_or(0.0);
        eprintln!("edits: joined {} clips into {} ({secs:.1}s)", clips.len(), out.display());
        if c.long {
            if secs < LONG {
                eprintln!("edits: {secs:.0}s is under a minute: give a channel or topic with more to cut (--videos), or more moments");
            }
            let side = keep(c, &out, serde_json::json!({ "clip": out, "kind": "long", "seconds": secs, "parts": clips }))?;
            println!("{side}");
        }
    }
    Ok(())
}

/// The footage to cut from: picture-only downloads (or the file given), and its shots.
/// (id, file) per video, every shot, and each video's measured frames.
type Footage = (Vec<(String, PathBuf)>, Vec<montage::Shot>, Vec<Vec<montage::Frame>>);

/// The video ids named in --shot and --say (VIDEO@...), each once.
fn named_videos(c: &Cli) -> Vec<String> {
    let mut ids: Vec<String> = c.shots.iter().chain(&c.say).filter_map(|w| w.rsplit_once('@').map(|x| x.0.to_string())).filter(|id| !id.is_empty()).collect();
    ids.sort();
    ids.dedup();
    ids
}

/// --shot VIDEO@FROM[-TO]: which video (by id, or a local file by name; empty for the only one), where it starts
/// and where the usable stretch ends (TO, else open).
fn hand_shot(want: &str, videos: &[(String, PathBuf)]) -> Option<(usize, f64, f64, Option<f64>)> {
    let (id, range) = want.rsplit_once('@')?;
    // ID@FROM[-TO][:X]: X (0..1 across the source) is where the frame looks, for footage with no face to follow
    let (range, look) = match range.split_once(':') {
        Some((r, x)) => (r, Some(x.trim().parse::<f64>().ok().filter(|x| (0.0..=1.0).contains(x))?)),
        None => (range, None),
    };
    let mut ends = range.split('-');
    let from: f64 = ends.next()?.trim().parse().ok()?;
    let to: f64 = ends.next().and_then(|t| t.trim().parse().ok()).filter(|t| *t > from).unwrap_or(f64::MAX);
    let v = if id.is_empty() && videos.len() == 1 { 0 } else {
        videos.iter().position(|(vid, f)| vid == id || f.file_stem().is_some_and(|s| s.to_string_lossy() == id))?
    };
    Some((v, from.max(0.0), to, look))
}

fn footage(c: &Cli, cache: &Path) -> Footage {
    let videos: Vec<(String, PathBuf)> = if Path::new(&c.input).is_file() {
        vec![(c.input.clone(), PathBuf::from(&c.input))]
    } else {
        let found = fetch::resolve(&c.input, c.videos).unwrap_or_else(|e| {
            eprintln!("edits: {e}");
            vec![]
        });
        let mut found = fetch::screen(found, c.use_videos, cache, &c.lang, c.llm);
        // a video named in --shot or --say is used whatever the screening thinks of it
        for id in named_videos(c) {
            if !found.iter().any(|(f, _)| *f == id) {
                found.push((id.clone(), format!("https://www.youtube.com/watch?v={id}")));
            }
        }
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
    // --long: the preset played round until it passes a minute, as one edit on one song
    let p = if c.long { let n = ((LONG + 1.0) / p.total.max(0.5)).ceil() as usize; p.clone().repeated(n.max(2)) } else { p };
    lap(&format!("preset {}: {} shots in {:.1}s, {}x{}, {} flashes{}", p.name, p.slots.len(), p.total, p.w, p.h, p.flashes.len(),
        p.song.as_ref().map(|s| format!(", song \"{}\"", s.title)).unwrap_or_default()));
    let music = match (&c.music, &p.song) {
        (Some(m), _) => fetch::music(m)?,
        (None, Some(s)) => fetch::music(&s.title)?,
        (None, None) => return Err("this preset names no song: give one with --music".into()),
    };
    let (song, (videos, mut shots, frames)) = std::thread::scope(|s| {
        let song = s.spawn(|| montage::song(&music, cache));
        (song.join().unwrap_or_else(|_| Err("song thread crashed".into())), footage(c, cache))
    });
    let song = song?;
    if shots.is_empty() {
        return Err("no footage to cut".into());
    }
    lap(&format!("song \"{}\"; {} shots from {} video(s)", song.title, shots.len(), videos.len()));
    // where the song starts: the preset's own offset into its own song; else, when the preset marks its drop, so that
    // the song's own drop lands on it; else its most replayed (or loudest) stretch
    let marked = p.slots.iter().find(|s| s.drop).map(|s| s.start);
    let song_from = match (&c.music, &p.song) {
        (None, Some(s)) => s.from,
        _ => marked.and_then(|d| montage::drop_at(&song.beats, &song.strength, &song.loud, &song.heat, d, p.total - d).map(|b| b - d))
            .unwrap_or_else(|| song.starts(p.total, 1).first().copied().unwrap_or(0.0)),
    };
    let end = song.loud.len() as f64 * montage::hop_s();
    if song_from + p.total > end + 0.5 {
        eprintln!("edits: the song ends {:.0}s before the edit does; give a longer one with --music", song_from + p.total - end);
    }
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
    preview_size(c, &mut opts);
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
    // --say over a preset: the speaker's own words, whole, from the video they were said in
    let mut spoken = analyse_ids(&say_ids(c), c, cache);
    let mut voices: Vec<(usize, pick::Moment)> = vec![];
    for w in &c.say {
        match joined_say(w, &spoken, cache) {
            Some(joined) => {
                let (a, m) = joined?;
                spoken.push(a);
                voices.push((spoken.len() - 1, m));
            }
            None => voices.extend(by_hand(w, &spoken)),
        }
    }
    if voices.len() != c.say.len() {
        return Err("--say: give VIDEO@FROM-TO with the video's id (or the file's name) and seconds where it speaks".into());
    }
    let lyrics = if c.lyrics { montage::lyrics(&song, cache) } else { vec![] };
    let cut_starts: Vec<f64> = spans.iter().map(|(a, _)| *a).collect();
    // the line ends on the drop: the cut where the song gets loudest relative to the 1 s before (a drop comes out of a gap; a stab 2 s back must not hide it), beat strength breaking ties
    let mean = |a: f64, b: f64| {
        let (i, j) = ((a / montage::hop_s()).max(0.0) as usize, ((b / montage::hop_s()) as usize).min(song.loud.len()));
        if j > i { song.loud[i..j].iter().sum::<f64>() / (j - i) as f64 } else { 0.0 }
    };
    let lift: Vec<f64> = cut_starts.iter().zip(&strength).map(|(a, s)| { let t = song_from + a; mean(t, t + 2.0) / (mean(t - 1.0, t) + 1e-3) + 0.1 * s }).collect();
    // --shot: the footage chosen by hand, one per slot in order (cycling when there are fewer), instead of the pick
    let hand: Vec<(usize, f64, f64, Option<f64>)> = c.shots.iter().map(|w| hand_shot(w, &videos).ok_or(format!("--shot {w}: give VIDEO@SECONDS with a video of this run"))).collect::<Result<_, _>>()?;
    let hand_from = shots.len();
    shots.extend(hand.iter().map(|&(video, s, _, focus)| montage::Shot { video, s, e: s + 60.0, scene_end: s + 60.0, score: 0.0, focus }));
    // each hand-picked stretch plays on from where it stopped the time before, back to its start past TO
    let mut cursor: Vec<f64> = hand.iter().map(|h| h.1).collect();
    let mut used = vec![false; shots.len()];
    let mut made = 0;
    for k in 0..c.n {
        let voice = (!voices.is_empty()).then(|| {
            let (i, m) = &voices[k % voices.len()];
            let len = m.end - m.start;
            if len > p.total - 0.9 {
                eprintln!("edits: the --say line ({len:.1}s) is longer than the preset ({:.1}s); it is cut off", p.total);
            }
            let v = montage::Voice { file: spoken[*i].src.audio.clone(), start: m.start, len, at: voice_at(&cut_starts, &lift, len, p.total, p.slots.iter().find(|s| s.drop).map(|s| s.start)) };
            (&spoken[*i], m, if c.finish.denoise { finish::clean_voice(v, &cache.join("voices")) } else { v })
        });
        let slots: Vec<&preset::Slot> = spans.iter().map(|(a, _)| p.slots.iter().rfind(|s| s.start <= a + 0.02).unwrap_or(&p.slots[0])).collect();
        let lens: Vec<f64> = spans.iter().zip(&slots).map(|((a, b), s)| (s.end - s.start).max(b - a)).collect();
        let needs: Vec<f64> = slots.iter().zip(&lens).map(|(s, l)| s.source_len(*l) + 0.2).collect();
        let picks = if hand.is_empty() {
            montage::assign(&shots[..hand_from], &mut used[..hand_from], &needs, None)
        } else {
            (0..needs.len()).map(|i| {
                let j = (k * needs.len() + i) % hand.len();
                if cursor[j] + needs[i] > hand[j].2 {
                    cursor[j] = hand[j].1;
                }
                let at = cursor[j];
                cursor[j] += needs[i];
                (hand_from + j, at)
            }).collect()
        };
        if picks.len() < needs.len() {
            eprintln!("edits: not enough usable shots for edit {}", k + 1);
            break;
        }
        let mut picks = picks;
        // --loop: the last shot is what ran just before the first, so the end runs into the start
        if c.looped && picks.len() > 1 {
            let last = picks.len() - 1;
            let (sh, from) = picks[0];
            let need = slots[last].source_len(spans[last].1 - spans[last].0);
            // no room around the first shot: another shot long enough for both, unless the shots were chosen by hand
            let seamed = montage::seam((from, needs[0]), (shots[sh].s, shots[sh].scene_end.max(shots[sh].e)), need, sh < hand_from).map(|x| (sh, x))
                .or_else(|| hand.is_empty().then(|| montage::roomy(&shots, hand_from, needs[0] + need)).flatten().map(|j| (j, (shots[j].s + need, shots[j].s))));
            match seamed {
                Some((j, (a, b))) => {
                    picks[0] = (j, a);
                    picks[last] = (j, b);
                }
                None => eprintln!("edits: edit {}: the first shot has no footage before it; the loop shows a cut where it starts again", k + 1),
            }
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
            let aimed = shots[*shot].focus.filter(|_| *shot >= hand_from);
            let focus = if let Some(x) = aimed { format!("{x:.3}") } else { fx::track(&frames[v], 4.0, *from, needs[i]).map_or(format!("{:.3}", shots[*shot].focus.unwrap_or(0.5)), |cv| cv.expr("t", lens[i])) };
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
        // with the person cut out, the words go on after it (in deliver), not under the new background
        let has_subs = subs.contains("Dialogue:") && !c.subject.any();
        if has_subs {
            std::fs::write(work.join("cap.ass"), &subs).map_err(|e| e.to_string())?;
        }
        (0..m).for_each(|i| g.push_str(&format!("[v{i}]")));
        let fade_out = if c.looped { String::new() } else { format!(",fade=t=out:st={:.3}:d=0.5", (p.total - 0.5).max(0.0)) };
        g.push_str(&format!("concat=n={m}:v=1:a=0,settb=1/30,fps=30{}{}{}{fade_out}", render::grade(&opts.grade), preset::flashes(&p),
            if has_subs { ",ass=cap.ass" } else { "" }));
        if !c.look.tag.is_empty() {
            g.push_str(&format!(",drawtext=text='{}':font=Adwaita Sans:fontsize=h*0.022:fontcolor=white@0.7:x=(w-tw)/2:y=h*0.94", c.look.tag));
        }
        let pre = if c.looped { montage::preroll(&song.beats, song_from, p.total) } else { None };
        g.push_str(",format=yuv420p[vout];");
        match (c.looped, pre) {
            (true, Some(d)) => g.push_str(&montage::splice(m, m + 1 + usize::from(voice.is_some()), p.total, d)),
            (true, None) => g.push_str(&format!("[{m}:a]anull")),
            _ => g.push_str(&format!("[{m}:a]afade=t=out:st={:.3}:d=0.6", (p.total - 0.6).max(0.0))),
        }
        if let Some((_, _, v)) = &voice {
            g.push_str(&montage::duck(m + 1, v));
        }
        g.push_str(&format!(",loudnorm=I={:.1}:TP=-1.5[aout]", opts.lufs));
        args.extend(["-ss".into(), format!("{song_from:.3}"), "-t".into(), format!("{:.3}", p.total), "-i".into(), song.file.to_string_lossy().into_owned()]);
        if let Some((_, _, v)) = &voice {
            args.extend(["-ss".into(), format!("{:.3}", v.start), "-t".into(), format!("{:.3}", v.len), "-i".into(), v.file.to_string_lossy().into_owned()]);
        }
        if let Some(d) = pre {
            args.extend(["-ss".into(), format!("{:.3}", song_from - d), "-t".into(), format!("{d:.3}"), "-i".into(), song.file.to_string_lossy().into_owned()]);
        }
        args.extend(["-filter_complex".into(), g, "-map".into(), "[vout]".into(), "-map".into(), "[aout]".into()]);
        args.extend(["-t".into(), format!("{:.3}", p.total), "-fps_mode".into(), "cfr".into(), "-r".into(), "30".into()]);
        args.extend(util::h264(20));
        args.extend(["-maxrate", "8M", "-bufsize", "16M",
            "-c:a", "aac", "-b:a", "192k", "-ar", "48000", "-movflags", "+faststart"].iter().map(|s| s.to_string()));
        args.push(std::path::absolute(&out).map_err(|e| e.to_string())?.to_string_lossy().into_owned());
        let _ = std::fs::write(work.join("ffmpeg.txt"), args.iter().map(|a| format!("'{}'", a.replace('\'', "'\\''"))).collect::<Vec<_>>().join(" "));
        match util::run(Some(&work), "ffmpeg", &args) {
            Ok(_) => {
                tl.captions = talk::lines_srt(&quotes, p.total);
                if c.subject.any() {
                    tl.ass = subs.clone();
                }
                let beats: Vec<f64> = song.beats.iter().map(|b| b - song_from).collect();
                // the drop: the shot the preset marks, else where the voice line ends (it is placed to end on the hardest cut)
                let drop = p.slots.iter().find(|s| s.drop).map(|s| s.start).or(voice.as_ref().map(|v| v.2.at + v.2.len));
                tl.markers = timeline::markers(&beats, p.total, drop, voice.as_ref().map(|v| v.2.at));
                tl.drawn = montage::sung(&lyrics, song_from, p.total).into_iter().filter(|x| voice.as_ref().is_none_or(|(_, _, v)| x.0 >= v.at + v.len)).collect();
                tl.captions.extend(tl.drawn.clone());
                if let Some((a, m, v)) = &voice {
                    tl.said = voice_cues(&a.words, m, v.at);
                    tl.captions.extend(tl.said.clone());
                    tl.captions.sort_by(|x, y| x.0.total_cmp(&y.0));
                }
                let fin = deliver(c, &out, &tl)?;
                let side = serde_json::json!({ "clip": fin, "kind": "preset", "preset": p.name, "song": song.title, "song_from": song_from, "seconds": fetch::duration(&fin).unwrap_or(p.total), "lines": quotes,
                    "voice": voice.as_ref().map(|(a, m, v)| serde_json::json!({ "video": a.src.id, "moment": m, "at": v.at, "text": said(a, m) })),
                    "shots": picks.iter().map(|(i, st)| serde_json::json!({ "video": videos[shots[*i].video].0, "from": st })).collect::<Vec<_>>() });
                let side = keep(c, &fin, side)?;
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
    // the person cut out first, so every word and drawing goes on top of the rebuilt picture
    subject::apply(out, &c.subject)?;
    // words drawn as stickers (--caption, emoji in colour); the spoken line, lyrics, headlines and the flashes in the caption style
    let words = wrap::text_stickers(out, &c.captions_at, 0.11, 0.09, "cap")?;
    let mut said: Vec<(f64, f64, String)> = if c.say_text { tl.said.clone() } else { vec![] };
    said.extend(tl.drawn.iter().cloned());
    said.sort_by(|a, b| a.0.total_cmp(&b.0));
    let (w, h) = fetch::dims(out);
    let styled = talk::merge_ass(&talk::cues_ass(&said, &c.flash_texts, &line_style(c)?, w, h), &tl.ass);
    let mut st = c.svgs.clone();
    st.extend(words.iter().map(|w| w.0.clone()));
    st.iter_mut().filter(|s| s.anim.is_empty()).for_each(|s| s.anim = c.anim.clone());
    let drawn = svg::apply(out, &st);
    words.iter().for_each(|w| { let _ = std::fs::remove_file(&w.1); });
    drawn?;
    library::apply(out, &c.overlays)?;
    let cuts: Vec<f64> = tl.clips.iter().scan(0.0, |t, cl| { *t += cl.2; Some(*t) }).collect();
    wrap::math(out, &cuts[..cuts.len().saturating_sub(1)], c.math, &c.formulas, &styled, &c.lang)?;
    // music and preset edits had their voice line cleaned before the mix; the song must not go through a denoiser
    let edit = c.music.is_some() || c.preset.is_some() || c.like.is_some();
    let fin = finish::apply(out, &finish::Finish { denoise: c.finish.denoise && !edit, ..c.finish.clone() })?;
    let shift = wrap::around(&fin, &c.around, &c.lang, c.opts.lufs, &util::home().join(".cache/edits"))?;
    let mut tl = tl.clone();
    tl.captions.iter_mut().for_each(|x| { x.0 += shift; x.1 += shift; });
    // words in the edit (a voice line, lines on screen) always get an .srt beside the clip
    let mut kinds = c.export.clone();
    if !tl.captions.is_empty() && !kinds.iter().any(|k| k == "srt") {
        kinds.push("srt".into());
    }
    if !kinds.is_empty() {
        for p in tl.write(&fin, &kinds)? {
            eprintln!("edits: wrote {}", p.display());
        }
    }
    Ok(fin)
}

/// The sidecar beside a finished clip, also kept in the render log that `edits stats` matches posted videos against.
fn keep(c: &Cli, fin: &Path, mut side: serde_json::Value) -> Result<serde_json::Value, String> {
    side["look"] = serde_json::json!(c.look_name);
    side["made_from"] = serde_json::json!(c.input);
    std::fs::write(fin.with_extension("json"), serde_json::to_string_pretty(&side).unwrap()).map_err(|e| e.to_string())?;
    stats::record(fin, &side);
    Ok(side)
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
/// A voice line as subtitle cues on the edit's clock (it plays from `at`): one per phrase, broken after
/// punctuation or every five words.
fn voice_cues(words: &[transcribe::Word], m: &pick::Moment, at: f64) -> Vec<(f64, f64, String)> {
    // ">>" marks a change of speaker in YouTube's captions, it is not said
    let said: Vec<&transcribe::Word> = words.iter().filter(|w| w.s >= m.start && w.e <= m.end && w.t.trim() != ">>").collect();
    let mut cues: Vec<(f64, f64, String)> = vec![];
    let mut run: Vec<&transcribe::Word> = vec![];
    for (k, w) in said.iter().enumerate() {
        run.push(w);
        if w.t.ends_with(['.', ',', '?', '!']) || run.len() == 5 || k + 1 == said.len() {
            let end = if k + 1 == said.len() { m.end } else { w.e };
            cues.push((run[0].s - m.start + at, end - m.start + at, run.iter().map(|w| w.t.as_str()).collect::<Vec<_>>().join(" ")));
            run.clear();
        }
    }
    cues
}

fn said(a: &Analysed, m: &pick::Moment) -> String {
    a.words.iter().filter(|w| w.s >= m.start && w.e <= m.end).map(|w| w.t.as_str()).collect::<Vec<_>>().join(" ")
}

/// --say VIDEO@A-B+C-D: stretches of one video said as one line (an offer, then the answer to it), joined into one
/// sound with their words moved onto its clock; a bare number between them is that many seconds of silence. None when the line is a single stretch.
fn joined_say(want: &str, spoken: &[Analysed], cache: &Path) -> Option<Result<(Analysed, pick::Moment), String>> {
    let (id, ranges) = want.rsplit_once('@')?;
    if !ranges.contains('+') {
        return None;
    }
    let join = || -> Result<(Analysed, pick::Moment), String> {
        // each stretch is FROM-TO in the video, or bare seconds of silence between two (a cut to someone else)
        let parts: Vec<Option<(usize, pick::Moment)>> = ranges.split('+').map(|r| {
            if r.contains('-') { by_hand(&format!("{id}@{r}"), spoken).map(Some).ok_or(format!("--say {want}: nothing said in {r}")) }
            else { r.parse::<f64>().ok().filter(|g| *g > 0.0).map(|g| None.filter(|_: &(usize, pick::Moment)| g < 0.0)).ok_or(format!("--say {want}: {r} is not seconds of silence")) }
        }).collect::<Result<_, _>>()?;
        let gaps: Vec<f64> = ranges.split('+').map(|r| if r.contains('-') { 0.0 } else { r.parse().unwrap_or(0.0) }).collect();
        let first = parts.iter().flatten().next().ok_or(format!("--say {want}: no words"))?;
        let a = &spoken[first.0];
        let out = cache.join(format!("say-{}.wav", fetch::safe(&format!("{id}-{ranges}"))));
        let audio = a.src.audio.to_string_lossy().into_owned();
        let mut args: Vec<String> = vec!["-v".into(), "error".into(), "-y".into()];
        for (p, g) in parts.iter().zip(&gaps) {
            match p {
                Some((_, m)) => args.extend(["-ss".into(), format!("{:.3}", m.start), "-t".into(), format!("{:.3}", m.end - m.start), "-i".into(), audio.clone()]),
                None => args.extend(["-f".into(), "lavfi".into(), "-t".into(), format!("{g:.3}"), "-i".into(), "anullsrc=r=48000:cl=stereo".into()]),
            }
        }
        let same: String = (0..parts.len()).map(|k| format!("[{k}:a]aformat=sample_rates=48000:channel_layouts=stereo[a{k}];")).collect();
        let ins: String = (0..parts.len()).map(|k| format!("[a{k}]")).collect();
        args.extend(["-filter_complex".into(), format!("{same}{ins}concat=n={}:v=0:a=1[a]", parts.len()), "-map".into(), "[a]".into(), out.to_string_lossy().into_owned()]);
        util::run(None, "ffmpeg", &args.iter().map(String::as_str).collect::<Vec<_>>())?;
        let (mut words, mut at, mut punch) = (vec![], 0.0, 0.0);
        for (p, g) in parts.iter().zip(&gaps) {
            let Some((_, m)) = p else {
                at += g;
                continue;
            };
            words.extend(a.words.iter().filter(|w| w.s >= m.start && w.e <= m.end).map(|w| transcribe::Word { s: w.s - m.start + at, e: w.e - m.start + at, t: w.t.clone() }));
            punch = m.punch - m.start + at;
            at += m.end - m.start;
        }
        let src = fetch::Source { audio: out, duration: at, ..a.src.clone() };
        Ok((Analysed { src, words, lines: vec![], moments: vec![], curve: vec![] },
            pick::Moment { start: 0.0, end: at, punch, score: 0.0, title: String::new(), why: "chosen by hand".into() }))
    };
    Some(join())
}

/// --say VIDEO@FROM-TO: which of `spoken` and that stretch, widened to whole words (never cut inside the last one).
fn by_hand(want: &str, spoken: &[Analysed]) -> Option<(usize, pick::Moment)> {
    let (id, range) = want.rsplit_once('@')?;
    let (a, b) = range.split_once('-')?;
    let (a, b): (f64, f64) = (a.parse().ok()?, b.parse().ok()?);
    let i = spoken.iter().position(|x| x.src.id == id)?;
    Some((i, widen(&spoken[i].words, a, b)?))
}

/// The words starting in FROM-TO as a moment: from just before the first to 0.25 s after the last, but never into
/// the next word (caption words end where the next one starts, so the tail would play it).
fn widen(words: &[transcribe::Word], a: f64, b: f64) -> Option<pick::Moment> {
    let i = words.iter().position(|w| w.s >= a - 0.3 && w.s < b)?;
    let j = words.iter().rposition(|w| w.s >= a - 0.3 && w.s < b)?;
    let (first, last) = (&words[i], &words[j]);
    let end = words.get(j + 1).map_or(last.e + 0.25, |n| (last.e + 0.25).min(n.s - 0.02).max(last.e));
    Some(pick::Moment { start: (first.s - 0.05).max(0.0), end, punch: last.s, score: 0.0, title: String::new(), why: "chosen by hand".into() })
}

/// Where a voice of `len` s goes in a preset edit: ending just before the hardest cut it fits ahead of, so that
/// cut's hit lands on the last word; at 0.3 s when it fits ahead of none. A preset that marks its drop decides.
fn voice_at(cuts: &[f64], strength: &[f64], len: f64, total: f64, drop: Option<f64>) -> f64 {
    if let Some(d) = drop {
        return (d - len).max(0.0);
    }
    cuts.iter().zip(strength).filter(|(a, _)| **a - len >= 0.3 && **a <= total - 0.6)
        .max_by(|x, y| x.1.total_cmp(y.1)).map_or(0.3, |(a, _)| a - len)
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
    preview_size(c, &mut opts);
    if c.long && template.is_some() {
        eprintln!("edits: --long does not stretch a copied edit (--like): it keeps the reference's length");
    }
    let look = montage::Look { punch: !c.look.sad && !c.corecore && template.is_none(), pace: c.speed, hits: c.hits.clone(), ..c.look.clone() };
    // corecore grades itself; a --look given goes under its grain
    if c.corecore && !c.looked {
        opts.grade = "none".into();
    }
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
    let lyrics = if c.lyrics { montage::lyrics(&song, cache) } else { vec![] };
    if c.lyrics {
        lap(&format!("{} sung words", lyrics.len()));
    }
    // words on screen (--lines, --line, corecore's headlines): the videos' most quotable sentences
    let pool = match (c.lines, c.line_texts.is_empty()) {
        (0, _) => vec![],
        (_, false) => c.line_texts.clone(),
        (n, true) => quote_lines(&videos, cache, c, n * c.n),
    };

    // the speaker's words, and the whole passages worth putting over music
    let spoken: Vec<Analysed> = if !c.voice {
        vec![]
    } else if local {
        // the input file's own words, plus any --say line named by a YouTube id (silent footage, voice from the channel)
        let stem = Path::new(&c.input).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let ids: Vec<&str> = say_ids(c).into_iter().filter(|id| !id.is_empty() && *id != stem).collect();
        // silent footage has no words of its own: then the voice must come from a named video
        let mut out = match fetch::local(Path::new(&c.input), cache) {
            Ok(src) => vec![analyse(src, c, false)],
            Err(e) if !ids.is_empty() => {
                eprintln!("edits: no voice in {}: {e}", c.input);
                vec![]
            }
            Err(e) => return Err(e),
        };
        out.extend(analyse_ids(&ids, c, cache));
        out
    } else {
        analyse_ids(&videos.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(), c, cache)
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
    let said_by_hand: Vec<(usize, pick::Moment)> = c.say.iter().filter_map(|want| by_hand(want, &spoken)).collect();
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
        _ if c.corecore => "corecore".into(),
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
                // --long: the montage after the drop carries the edit past a minute
                let want = if c.long { (c.look.length - lead - vlen).max(4.0) } else { (c.look.length * 0.45).clamp(4.0, 9.0) }.min(end - lead - vlen - 0.2).max(2.5);
                let Some(drop) = montage::drop_at(&song.beats, &song.strength, &song.loud, &song.heat, lead + vlen, want)
                    .or_else(|| montage::drop_at(&song.beats, &song.strength, &song.loud, &song.heat, lead + vlen, 2.5)) else {
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
                let wins = song.starts(c.look.length, k + 1);
                let start = *wins.get(k).or(wins.last())?;
                let mut cuts = montage::cuts(&song.beats, &song.loud, start, c.look.length, look.shot());
                if c.corecore {
                    cuts = montage::glimpses(&cuts);
                }
                // --lyrics: the words land on the cuts
                if !lyrics.is_empty() {
                    let rel = |v: &mut dyn Iterator<Item = f64>| v.map(|t| t - start).filter(|t| (0.0..=c.look.length).contains(t)).collect::<Vec<f64>>();
                    cuts = montage::on_words(&cuts, &rel(&mut song.beats.iter().copied()), &rel(&mut lyrics.iter().map(|w| w.s)));
                }
                (start, cuts, 0.0)
            }
        };
        // a loop is whole bars long, so the song's rhythm carries on round it (a copied edit keeps its own length)
        if c.looped && template.is_none() {
            cuts = montage::whole_bars(&cuts, montage::beat_period(&song.beats));
        }
        let total = *cuts.last()?;
        let beats: Vec<f64> = song.beats.iter().map(|b| b - start).filter(|b| *b >= 0.0 && *b <= total).collect();
        let mut joins = match &template {
            Some(t) => t.joins.clone(),
            None if c.corecore => montage::joins_core(&cuts),
            None => montage::joins(&cuts.iter().map(|t| montage::hit_at(&song.beats, &song.strength, start + t)).collect::<Vec<_>>(), c.look.sad),
        };
        if template.is_none() && voice.is_some() && joins.len() > 2 {
            // the drop
            joins[1] = if c.look.sad { montage::Join { kind: "fadeblack", dur: 0.5, motion: None } } else { montage::Join { kind: "fadewhite", dur: 0.3, motion: None } };
        }
        if template.is_none() && voice.is_none() {
            cuts.dedup();
        }
        if template.is_none() && !c.look.sad && !c.corecore {
            montage::add_moves(&mut joins, k);
        }
        if let Some(t) = c.transition {
            // every real transition becomes the one asked for; hard cuts stay hard cuts
            joins.iter_mut().filter(|j| j.dur > montage::CUT.dur).for_each(|j| j.kind = t);
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
        // --loop: the last piece is what ran just before the first, so the end runs into the start
        if c.looped && files.len() > 1 {
            let (first, last) = (files[0].clone(), spans.len() - 1);
            let speed = look.speed();
            let shot = videos.iter().position(|(_, f)| *f == first.0)
                .and_then(|v| shots.iter().find(|x| x.video == v && x.s <= first.1 + 0.01 && first.1 < x.e));
            let (uses, need) = (spans[0].1 * speed, spans[last].1 * speed);
            let seamed = shot.and_then(|x| montage::seam((first.1, uses), (x.s, x.scene_end.max(x.e)), need, voice.is_none())).map(|ab| (first.0.clone(), first.2, ab))
                // no room around the first shot: another shot long enough for both (not when a voice line holds the first)
                .or_else(|| voice.is_none().then(|| montage::roomy(&shots, shots.len(), uses + need)).flatten()
                    .map(|j| (videos[shots[j].video].1.clone(), shots[j].focus.unwrap_or(0.5), (shots[j].s + need, shots[j].s))));
            match seamed {
                Some((file, focus, (a, b))) => {
                    files[0] = (file.clone(), a, focus);
                    files[last] = (file, b, focus);
                }
                None => eprintln!("edits: edit {}: the first shot has no footage before it; the loop shows a cut where it starts again", k + 1),
            }
        }
        let labels = files.iter().map(|(f, st, _)| (videos.iter().find(|(_, p)| p == f).map(|x| x.0.clone()).unwrap_or_default(), *st)).collect();
        Some(Plan { k, start, cuts, joins, beats, files, labels, voice, at })
    }).collect();
    let made: Vec<_> = std::thread::scope(|s| {
        let jobs: Vec<_> = plans.iter().map(|p| {
            let (song, kind, look, opts, lyrics, pool) = (&song, &kind, &look, &opts, &lyrics, &pool);
            s.spawn(move || {
                let out = c.out.join(format!("{kind}-{:02}.mp4", p.k + 1));
                let work = cache.join("renders").join(format!("{kind}-{:02}", p.k + 1));
                std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
                let v = p.voice.map(|(a, m)| montage::Voice { file: a.src.audio.clone(), start: m.start, len: m.end - m.start, at: p.at })
                    .map(|v| if c.finish.denoise { finish::clean_voice(v, &work) } else { v });
                let (words, line) = match p.voice {
                    Some((a, m)) if opts.captions => (&a.words[..], Some(m)),
                    _ => (&[][..], None),
                };
                let subs = montage::captions(words, line, p.at, &look.text, !look.punch, *p.cuts.last().unwrap(), opts);
                let burn = if c.subject.any() { "" } else { subs.as_str() };
                montage::render(&p.files, &p.cuts, &p.joins, &p.beats, &song.file, p.start, v.as_ref(), burn, look, opts, &work, &out)?;
                let tl = timeline::Timeline {
                    name: format!("{kind}-{:02}", p.k + 1), w: opts.w, h: opts.h,
                    clips: p.files.iter().zip(p.cuts.windows(2)).map(|((f, from, _), w)| (f.clone(), *from, w[1] - w[0])).collect(),
                    music: Some((song.file.clone(), p.start)),
                    captions: p.voice.map(|(a, m)| voice_cues(&a.words, m, p.at)).unwrap_or_default(),
                    said: p.voice.map(|(a, m)| voice_cues(&a.words, m, p.at)).unwrap_or_default(),
                    // a music edit's drop comes as the voice line ends
                    markers: timeline::markers(&p.beats, *p.cuts.last().unwrap(), p.voice.map(|(_, m)| p.at + m.end - m.start), p.voice.map(|_| p.at)),
                    // the song's words once the voice line is over
                    drawn: montage::sung(lyrics, p.start, *p.cuts.last().unwrap()).into_iter().filter(|x| p.voice.is_none_or(|(_, m)| x.0 >= p.at + m.end - m.start)).collect(),
                    ass: if c.subject.any() { subs.clone() } else { String::new() },
                };
                let mut tl = tl;
                // lines on screen, each edit its own when there are enough (after the voice line, like the lyrics)
                let quotes: Vec<String> = (0..c.lines.min(pool.len())).map(|j| pool[(p.k * c.lines + j) % pool.len()].clone()).collect();
                let from = p.voice.map_or(0.0, |(_, m)| p.at + m.end - m.start);
                tl.drawn.extend(talk::lines_srt(&quotes, *p.cuts.last().unwrap() - from).into_iter().map(|(a, b, t)| (a + from, b + from, t)));
                tl.captions.extend(tl.drawn.clone());
                tl.captions.sort_by(|x, y| x.0.total_cmp(&y.0));
                let out = deliver(c, &out, &tl)?;
                let side = serde_json::json!({
                    "clip": out, "kind": kind, "song": song.title, "lines": tl.drawn.iter().map(|x| x.2.clone()).collect::<Vec<_>>(), "song_from": p.start, "seconds": fetch::duration(&out).ok().or(p.cuts.last().copied()),
                    "voice": p.voice.map(|(a, m)| serde_json::json!({ "video": a.src.id, "moment": m, "at": p.at, "text": said(a, m) })),
                    "shots": p.labels.iter().map(|(v, st)| serde_json::json!({ "video": v, "from": st })).collect::<Vec<_>>(),
                });
                keep(c, &out, side)
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

/// (name, SKILL.md) of every pack with a skill: the built-in ones, then the added ones.
fn skill_list() -> Vec<(String, String)> {
    packs::all().into_iter().filter_map(|p| Some((p.skill_name()?.to_string(), p.skill?))).collect()
}

/// Skill packs from other projects, fetched from GitHub when asked for instead of built in: the binary stays small and
/// a pack is as new as its repository. (name, repository, folder of skills in it, what it is for)
const CATALOG: &[(&str, &str, &str, &str)] = &[
    ("remotion", "remotion-dev/skills", "skills", "Remotion, videos written in React: create, markup, captions, render, studio, maps"),
    ("youtube-research", "TubeLabHQ/youtube-research-skills", "skills", "YouTube research: niches, channels, titles, thumbnails, comments, trends (TubeLab)"),
];

/// Claude's skill folder and every other agent's that exists here, or the one asked for.
fn skill_dirs(given: Option<&String>) -> Vec<PathBuf> {
    let h = util::home();
    match given {
        Some(d) => vec![PathBuf::from(d)],
        None => std::iter::once(h.join(".claude/skills")).chain([".config/opencode/skills", ".codex/skills", ".cursor/skills", ".gemini/skills", ".codeium/windsurf/skills"]
            .iter().map(|d| h.join(d)).filter(|d| d.is_dir())).collect(),
    }
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if src.is_dir() { copy_tree(&src, &dst)? } else { std::fs::copy(&src, &dst).map(|_| ()).map_err(|e| format!("{}: {e}", dst.display()))? }
    }
    Ok(())
}

/// The skill folders (each holding a SKILL.md) under `root`, by name.
fn skill_folders(root: &Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = std::fs::read_dir(root).into_iter().flatten().flatten().map(|e| e.path())
        .filter(|p| p.join("SKILL.md").is_file()).map(|p| (p.file_name().unwrap_or_default().to_string_lossy().into_owned(), p)).collect();
    v.sort();
    v
}

/// `edits skills install [DIR]`: the agent skills into ~/.claude/skills (or DIR). `edits skills` lists them.
/// `edits skills catalog`: packs from other projects; `edits skills add PACK [SKILL...] [--into DIR]` fetches one.
fn skills(args: &[String]) -> Result<(), String> {
    let into = args.iter().position(|a| a == "--into").and_then(|i| args.get(i + 1));
    match args.first().map(String::as_str) {
        Some("install") => {
            // a folder given, or Claude's and every other agent's skill folder that exists
            let list = skill_list();
            for dir in &skill_dirs(args.get(1)) {
                // renamed in 0.31.0 (making a preset makes a pack now)
                let _ = std::fs::remove_dir_all(dir.join("edits-preset-create"));
                for (name, text) in &list {
                    let d = dir.join(name);
                    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
                    std::fs::write(d.join("SKILL.md"), text).map_err(|e| e.to_string())?;
                }
                println!("installed {} skills in {}", list.len(), dir.display());
            }
            Ok(())
        }
        Some("catalog") => {
            for (name, repo, _, about) in CATALOG {
                println!("{name:<18} {about}\n{:<18} github.com/{repo}", "");
            }
            println!("\nadd one: edits skills add SET [SKILL...]   (all of its skills, or only those named)");
            Ok(())
        }
        Some("add") => {
            let pack = args.get(1).ok_or("usage: edits skills add SET [SKILL...] [--into DIR]   (edits skills catalog lists them)")?;
            let &(_, repo, folder, _) = CATALOG.iter().find(|c| c.0 == pack).ok_or(format!("no pack \"{pack}\" (edits skills catalog lists them)"))?;
            let want: Vec<&String> = args[2..].iter().take_while(|a| *a != "--into").collect();
            let tmp = std::env::temp_dir().join(format!("edits-skills-{}", std::process::id()));
            std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
            let res = (|| {
                let tar = tmp.join("pack.tar.gz");
                util::run(None, "curl", &["-fsSL", "--max-time", "120", "-o", &tar.to_string_lossy(), &format!("https://codeload.github.com/{repo}/tar.gz/HEAD")])
                    .map_err(|e| format!("could not download {repo}: {e}"))?;
                util::run(None, "tar", &["-xzf", &tar.to_string_lossy(), "-C", &tmp.to_string_lossy()])?;
                // the archive holds one folder, named after the repository and commit
                let top = std::fs::read_dir(&tmp).map_err(|e| e.to_string())?.flatten().map(|e| e.path()).find(|p| p.is_dir()).ok_or("the download was empty")?;
                let found = skill_folders(&top.join(folder));
                let pick: Vec<&(String, PathBuf)> = found.iter().filter(|(n, _)| want.is_empty() || want.contains(&n)).collect();
                if pick.is_empty() {
                    return Err(format!("{pack} has: {}", found.iter().map(|f| f.0.as_str()).collect::<Vec<_>>().join(", ")));
                }
                for dir in &skill_dirs(into) {
                    for (name, src) in &pick {
                        copy_tree(src, &dir.join(name))?;
                    }
                    println!("{pack}: {} skill(s) in {}: {}", pick.len(), dir.display(), pick.iter().map(|p| p.0.as_str()).collect::<Vec<_>>().join(", "));
                }
                Ok(())
            })();
            let _ = std::fs::remove_dir_all(&tmp);
            res
        }
        _ => {
            for (name, text) in skill_list() {
                let about = text.lines().find_map(|l| l.strip_prefix("description: ")).unwrap_or("");
                println!("{name:<22} {}", about.chars().take(110).collect::<String>());
            }
            println!("\ninstall them: edits skills install [DIR]   (default ~/.claude/skills)");
            println!("more from other projects (Remotion, YouTube research): edits skills catalog; community packs: edits packs browse");
            Ok(())
        }
    }
}

/// `edits app [market|automations|settings | --review CLIP]`: the desktop app, in this same program.
#[cfg(feature = "app")]
fn app(args: &[String]) -> Result<(), String> {
    app::run(args);
    Ok(())
}

#[cfg(not(feature = "app"))]
fn app(_: &[String]) -> Result<(), String> {
    Err("this edits was built without its app (--no-default-features)".into())
}

/// `edits packs`: the packs here (built in and added); `browse` the community list; `add ID|LINK`, `remove ID`;
/// `create VIDEO` makes a new pack from a video; `make DIR [--preset NAME] [--id ID]` makes a folder a pack; `gallery DIR` writes the picture page of a packs folder.
fn packs_cmd(args: &[String]) -> Result<(), String> {
    let opt = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).map(String::as_str);
    let kinds = |k: &[String]| k.join("+");
    let words: Vec<&String> = args.iter().filter(|a| *a != "--json").collect();
    match (words.first().map(|w| w.as_str()), words.get(1).copied()) {
        (Some("list") | None, _) if args.iter().any(|a| a == "--json") => {
            println!("{}", packs_json());
            Ok(())
        }
        (Some("list") | None, _) => {
            let off = packs::disabled();
            for p in packs::every() {
                let m = &p.manifest;
                if off.contains(&m.id) {
                    print!("(off) ");
                }
                println!("{:<34} {:<12} {}", m.id, kinds(&m.kinds), m.description.chars().take(90).collect::<String>());
            }
            println!("
community packs: edits packs browse; added ones live in {}", packs::dir().display());
            Ok(())
        }
        (Some("browse"), _) => {
            let have: Vec<String> = packs::added().into_iter().map(|p| p.manifest.id).collect();
            let list = packs::registry()?;
            if args.iter().any(|a| a == "--json") {
                println!("{}", listed_json(&list));
                return Ok(());
            }
            for e in &list {
                println!("{:<34} {:<12} {}{}\n{:<34} {}", e.id, kinds(&e.kinds), e.name, if have.contains(&e.id) { "  (added)" } else { "" }, "", e.description.chars().take(100).collect::<String>());
            }
            println!("\n{} community pack(s). add one: edits packs add ID. list yours: a pull request to packs.json", list.len());
            Ok(())
        }
        (Some("add"), Some(what)) => {
            let p = packs::add(what)?;
            let m = &p.manifest;
            println!("added {} {} ({}) in {}", m.id, m.version, kinds(&m.kinds), packs::dir().join(&m.id).display());
            if let Some(preset) = p.preset.as_deref().and_then(|t| serde_json::from_str::<preset::Preset>(t).ok()) {
                println!("  its preset: --preset {}", preset.name);
            }
            if let (Some(name), Some(text)) = (p.skill_name(), p.skill.as_deref()) {
                // a skill is instructions an agent follows: say so, and where it went
                for dir in &skill_dirs(None) {
                    std::fs::create_dir_all(dir.join(name)).map_err(|e| e.to_string())?;
                    std::fs::write(dir.join(name).join("SKILL.md"), text).map_err(|e| e.to_string())?;
                    println!("  its skill {name} (instructions your agent will follow; read it) in {}", dir.display());
                }
            }
            if let Some(a) = &m.automation {
                // it would run in their signed-in account: nothing runs until they read it and say so
                println!("  its automation runs in your signed-in {}: read {}, then edits packs trust {}", a.site, packs::dir().join(&m.id).join("automation.js").display(), m.id);
            }
            Ok(())
        }
        (Some("trust"), Some(id)) => packs::trust(id).map(|f| println!("{id}: its program ({}) may run now; a changed one asks again", f.display())),
        (Some(w @ ("on" | "off")), Some(id)) => {
            let p = packs::enable(id, w == "on")?;
            // an off pack's skill leaves the agents' skill folders; on, it comes back
            if let (Some(name), Some(text)) = (p.skill_name(), p.skill.as_deref()) {
                for dir in &skill_dirs(None) {
                    if w == "on" {
                        std::fs::create_dir_all(dir.join(name)).map_err(|e| e.to_string())?;
                        std::fs::write(dir.join(name).join("SKILL.md"), text).map_err(|e| e.to_string())?;
                    } else {
                        let _ = std::fs::remove_dir_all(dir.join(name));
                    }
                }
            }
            println!("{id} is {w}");
            Ok(())
        }
        (Some("set"), Some(id)) => packs::set(id, &args[2..]).map(|v| println!("{id}: {}", serde_json::Value::Object(v))),
        (Some("share"), Some(id)) => {
            let (tgz, url) = packs::share(id)?;
            println!("{}\nattach it to the issue: {url}", tgz.display());
            util::open_url(&url)
        }
        (Some("remove" | "delete"), Some(id)) => {
            let p = packs::remove(id)?;
            if let Some(name) = p.skill_name() {
                for dir in &skill_dirs(None) {
                    let _ = std::fs::remove_dir_all(dir.join(name));
                }
            }
            println!("removed {id}");
            Ok(())
        }
        (Some("create"), Some(video)) => {
            // a pack from nothing but a video; --like a,b picks the presets whose moves it borrows (default: the closest three)
            let name = opt("--name").map(String::from).unwrap_or_else(|| Path::new(video).file_stem().unwrap_or_default().to_string_lossy().into_owned());
            let like: Vec<String> = opt("--like").map(|l| l.split(',').map(str::to_string).collect()).unwrap_or_default();
            let len = opt("--length").and_then(|l| l.parse().ok()).unwrap_or(18.0);
            let p = create::create(Path::new(video), &name, &like, len)?;
            let id = preset::slug(&name);
            let d = packs::dir().join(&id);
            std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
            std::fs::write(d.join("preset.json"), preset::pretty(&p)).map_err(|e| e.to_string())?;
            let m = packs::make(&d, None, Some(&id), None)?;
            println!("{}: pack {} ({}, {:.1}s) -> --preset {}; share it: PACKS.md", d.display(), m.id, p.about, p.total, p.name);
            Ok(())
        }
        (Some("make"), Some(dir)) => {
            let mut m = packs::make(Path::new(dir), opt("--preset"), opt("--id"), opt("--opml").map(Path::new))?;
            // --tag a,b: the tags it is found by, then made again so its picture shows them
            if let Some(t) = opt("--tag") {
                m.tags = t.split(',').map(|x| x.trim().to_lowercase()).filter(|x| !x.is_empty()).collect();
                std::fs::write(Path::new(dir).join("manifest.json"), serde_json::to_string_pretty(&m).map_err(|e| e.to_string())? + "\n").map_err(|e| e.to_string())?;
                m = packs::make(Path::new(dir), None, None, None)?;
            }
            println!("{}: {} {} ({}), picture {}", dir, m.id, m.version, kinds(&m.kinds), m.preview.unwrap_or_default());
            Ok(())
        }
        (Some("gallery"), Some(dir)) => packs::gallery(Path::new(dir)).map(|n| println!("{dir}/README.md: {n} packs")),
        _ => Err("usage: edits packs [--json] [browse [--json] | add ID|LINK | trust ID | remove ID | create VIDEO [--name N] [--like A,B] [--length S] | make DIR [--preset NAME] [--id ID] [--opml FILE] [--tag A,B] | on|off ID | set ID KEY=VALUE | share ID | gallery DIR]".into()),
    }
}

/// `edits words <link|ID|file> [A-B] [--find TEXT] [--lang xx]`: what a video says, line by line with times, each
/// line as a ready --say spec. The site's own captions when it has them, whisper otherwise.
fn words(args: &[String]) -> Result<(), String> {
    let opt = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let lang = opt("--lang").unwrap_or_else(|| lang::code().into());
    let find = opt("--find").map(|f| f.to_lowercase());
    let range = args[1..].iter().find(|a| !a.starts_with("--") && Some(*a) != opt("--lang").as_ref() && Some(*a) != opt("--find").as_ref())
        .and_then(|r| r.split_once('-')).and_then(|(a, b)| Some((a.parse::<f64>().ok()?, b.parse::<f64>().ok()?)));
    let cache = util::home().join(".cache/edits");
    let src = one_source(&args[0], &cache)?;
    let t = transcribe::transcript(&src.work, &src.wav, &lang, &src.language)?;
    eprintln!("edits: {} ({:.0}s), words from {}", src.title, src.duration, t.from);
    for l in transcribe::lines(&t.words) {
        let inside = range.is_none_or(|(a, b)| l.e > a && l.s < b);
        if inside && find.as_ref().is_none_or(|f| l.text.to_lowercase().contains(f)) {
            println!("{}@{:.2}-{:.2}\t{}", src.id, l.s, l.e, l.text);
        }
    }
    Ok(())
}

/// One video by file, ID or link, fetched (sound, captions, info) into the cache.
fn one_source(input: &str, cache: &Path) -> Result<fetch::Source, String> {
    if Path::new(input).is_file() {
        return fetch::local(&std::fs::canonicalize(input).map_err(|e| e.to_string())?, cache);
    }
    let is_id = input.len() == 11 && input.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    let (id, url) = if is_id { (input.to_string(), format!("https://www.youtube.com/watch?v={input}")) } else { fetch::resolve(input, 1)?.remove(0) };
    fetch::remote(&id, &url, cache)
}

/// A long video's signals scored into windows: (lines kept, chat, windows). Whisper's loops are taken out of the
/// words first; the reactions counted are the spoken language's.
fn scan(src: &fetch::Source, t: transcribe::Transcript, spoken: &str) -> (Vec<transcribe::Line>, vod::Chat, Vec<vod::Window>) {
    let (lines, dropped, loops) = vod::unloop(&transcribe::lines(&t.words));
    if !loops.is_empty() {
        eprintln!("edits: {} lines whisper made up over and over were taken out", loops.len());
    }
    let words = vod::keep_words(t.words, &dropped);
    let loud = signals::loudness(&src.wav).unwrap_or_default();
    let chat = fetch::chat(src);
    let hits = signals::comment_hits(&src.comments, src.duration);
    let viewers = if src.heat.is_empty() && hits.is_empty() { vec![] } else { signals::curve(src.duration, &src.heat, &hits, &[]) };
    let ws = vod::windows(src.duration, &loud, &words, &chat, &viewers, &loops, &lang::for_spoken(spoken));
    (lines, chat, ws)
}

/// `edits moments <link|ID|file> [-n N] [--find REGEX]... [--lang L] [--no-llm] [--no-sheets]`: the moments of a
/// long video worth a clip, each with a sheet of 12 frames to check and the command that cuts it.
fn moments_cmd(args: &[String]) -> Result<(), String> {
    let opt = |k: &str| args.iter().position(|a| a == k).and_then(|i| args.get(i + 1)).cloned();
    let finds: Vec<String> = args.windows(2).filter(|w| w[0] == "--find").map(|w| w[1].clone()).collect();
    let n: usize = opt("-n").map(|v| v.parse().map_err(|_| format!("-n {v}: give a number"))).transpose()?.unwrap_or(20).clamp(1, 50);
    let (use_llm, sheets) = (!args.iter().any(|a| a == "--no-llm"), !args.iter().any(|a| a == "--no-sheets"));
    let cache = util::home().join(".cache/edits");
    let src = one_source(&args[0], &cache)?;
    let spoken = opt("--lang").unwrap_or_else(|| if src.language.is_empty() { lang::code().to_string() } else { src.language.clone() });
    let t = transcribe::transcript(&src.work, &src.wav, &spoken, &opt("--lang").unwrap_or(src.language.clone()))?;
    let from = t.from;
    let (lines, chat, ws) = scan(&src, t, &spoken);
    eprintln!("edits: {} ({}): words from {from}, {} chat messages, {} windows of {:.0} s", src.title, util::mmss(src.duration), chat.len(), ws.len(), vod::WINDOW);
    let cands: Vec<pick::Moment> = vod::best(&ws, if use_llm { (n * 2).min(40) } else { n }).iter().map(vod::moment).collect();
    let judged = if use_llm && !cands.is_empty() {
        match crate::ai::ask(&vod::rubric_prompt(&src.title, &cands, &lines, &chat, n)).and_then(|r| vod::judged(&r, &cands)) {
            Ok(j) if !j.is_empty() => Some(j),
            Ok(_) => { eprintln!("edits: the model kept none of the candidates; the scores decide"); None }
            Err(e) => { eprintln!("edits: {e}; the scores decide"); None }
        }
    } else { None };
    let by = if judged.is_some() { "the model, by the rubric" } else { "the scores" };
    let judged: Vec<(pick::Moment, String)> = judged.unwrap_or_else(|| cands.into_iter().map(|m| (m, String::new())).collect()).into_iter().take(n).collect();
    // each kept moment's post caption, found again after the cuts are snapped to whole lines
    let posts: Vec<(f64, f64, String)> = judged.iter().map(|(m, p)| (m.start, m.end, p.clone())).collect();
    let post = |m: &pick::Moment| posts.iter().find(|p| p.0 < m.end && p.1 > m.start).map(|p| p.2.clone()).unwrap_or_default();
    let mut picked: Vec<pick::Moment> = judged.into_iter().map(|x| x.0).collect();
    // asked for by what was said: always in, after the rest
    picked.extend(vod::find(&lines, &finds)?);
    let picked = pick::refine(picked, &lines, src.duration);
    let out = auto_out(&args[0]);
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    // 12 frames of each, four at a time: a 360p copy of the stretch online
    let sheet_of = |k: usize, m: &pick::Moment| -> Option<PathBuf> {
        let sheet = out.join(format!("moment-{:02}.jpg", k + 1));
        fetch::peek(&src, m.start, m.end).and_then(|(f, at)| check::contact_sheet_at(&f, at, m.end - m.start, &sheet, 6, 2, 320))
            .map_err(|e| eprintln!("edits: no frames for moment {}: {e}", k + 1)).ok().map(|_| sheet)
    };
    let sheets: Vec<Option<PathBuf>> = if !sheets { vec![None; picked.len()] } else {
        picked.chunks(4).enumerate().flat_map(|(c, ms)| std::thread::scope(|s| {
            let jobs: Vec<_> = ms.iter().enumerate().map(|(i, m)| s.spawn(move || sheet_of(c * 4 + i, m))).collect();
            jobs.into_iter().map(|j| j.join().ok().flatten()).collect::<Vec<_>>()
        })).collect()
    };
    let input = src.local.as_ref().map_or(src.url.clone(), |f| f.display().to_string());
    let cut = |m: &pick::Moment| format!("edits \"{input}\" --say {}@{:.1}-{:.1}", src.id, m.start, m.end);
    let mut md = format!("# Moments: {}\n\n{} · {} · words from {from} · {} chat messages · chosen by {by}\n", src.title, if src.url.is_empty() { src.id.clone() } else { src.url.clone() }, util::mmss(src.duration), chat.len());
    let mut json = vec![];
    for (k, (m, sheet)) in picked.iter().zip(&sheets).enumerate() {
        let said: String = lines.iter().filter(|l| l.e > m.start && l.s < m.end).map(|l| l.text.as_str()).collect::<Vec<_>>().join(" ").chars().take(240).collect();
        let name = if m.title.is_empty() { format!("Moment {}", k + 1) } else { m.title.clone() };
        md.push_str(&format!("\n## {}. {name}\n{}-{} ({:.0} s) · score {:.1}\n{}\n\n> {said}\n\n", k + 1, util::mmss(m.start), util::mmss(m.end), m.end - m.start, m.score, m.why));
        if let Some(s) = sheet {
            md.push_str(&format!("![]({})\n\n", s.file_name().unwrap_or_default().to_string_lossy()));
        }
        if !post(m).is_empty() {
            md.push_str(&format!("post: {}\n\n", post(m)));
        }
        md.push_str(&format!("cut: `{}`\n", cut(m)));
        println!("{}. {}-{} ({:.0} s) {:.1} {name}: {}{}", k + 1, util::mmss(m.start), util::mmss(m.end), m.end - m.start, m.score, m.why, sheet.as_ref().map_or(String::new(), |s| format!("\n   frames: {}", s.display())));
        println!("   cut: {}", cut(m));
        json.push(serde_json::json!({ "start": m.start, "end": m.end, "score": m.score, "title": m.title, "why": m.why, "said": said, "sheet": sheet, "post": post(m), "cut": cut(m) }));
    }
    std::fs::write(out.join("moments.md"), md).map_err(|e| e.to_string())?;
    std::fs::write(out.join("moments.json"), serde_json::to_string_pretty(&json).unwrap_or_default()).map_err(|e| e.to_string())?;
    println!("moments: {}", out.join("moments.md").display());
    Ok(())
}

/// Saves a changed preset (refused with `expect` when it changed since that revision); `edits preset undo` takes it back.
fn save_preset(p: preset::Preset, expect: Option<&str>) -> Result<(), String> {
    let (to, rev) = presets::save(&p, expect)?;
    println!("saved \"{}\" to {} (revision {rev}; edits preset undo \"{}\" takes it back)", p.name, to.display(), p.name);
    Ok(())
}

/// Sets `path` (dots; numbers index lists from 0, * is every item) in a preset's JSON.
fn set_path(v: &mut serde_json::Value, path: &str, val: serde_json::Value) -> Result<(), String> {
    let (head, rest) = path.split_once('.').map_or((path, None), |(h, r)| (h, Some(r)));
    let targets: Vec<&mut serde_json::Value> = match (head, &mut *v) {
        ("*", serde_json::Value::Array(a)) => a.iter_mut().collect(),
        (n, serde_json::Value::Array(a)) => {
            let i: usize = n.parse().map_err(|_| format!("{n}: a list needs a number from 0 or *"))?;
            let len = a.len();
            vec![a.get_mut(i).ok_or(format!("{i}: there are {len} (0..{})", len.saturating_sub(1)))?]
        }
        (k, serde_json::Value::Object(o)) if rest.is_none() => {
            o.insert(k.to_string(), val);
            return Ok(());
        }
        (k, serde_json::Value::Object(o)) => vec![o.get_mut(k).ok_or(format!("{k}: no such field"))?],
        (k, _) => return Err(format!("{k}: nothing to go into there")),
    };
    for t in targets {
        match rest {
            Some(r) => set_path(t, r, val.clone())?,
            None => *t = val.clone(),
        }
    }
    Ok(())
}

/// `edits preset ...`: list, show, save, set, import. Presets are JSON: read one, change what the moment needs, save it.
fn preset_cmd(args: &[String]) -> Result<(), String> {
    match (args.first().map(String::as_str), args.get(1)) {
        (Some("list") | None, _) => {
            for p in presets::all() {
                println!("{:<26} {:>2} shots {:>5.1}s  {:<38}{}", p.name, p.slots.len(), p.total, p.tags().join(" "), p.song.as_ref().map(|s| format!("  song: {}", s.title)).unwrap_or_default());
            }
            println!("\n{} cut moves; your presets: {}", presets::motions().len(), presets::user_dir().display());
            Ok(())
        }
        // making a preset makes a pack now: same arguments, the pack lands with the added ones
        (Some("create"), Some(_)) => packs_cmd(args),
        (Some("show"), Some(name)) => {
            let p = preset::load(name)?;
            println!("{}", preset::pretty(&p));
            // beside the JSON, not in it: change it with --if this, and a change made meanwhile is not overwritten
            eprintln!("edits: revision {}", presets::revision(&p));
            Ok(())
        }
        (Some("undo"), Some(name)) => presets::undo(name).map(|m| println!("{m}")),
        (Some("save"), Some(file)) => {
            let text = if file == "-" {
                let mut t = String::new();
                std::io::Read::read_to_string(&mut std::io::stdin(), &mut t).map_err(|e| e.to_string())?;
                t
            } else {
                std::fs::read_to_string(file).map_err(|e| format!("{file}: {e}"))?
            };
            let expect = args.iter().position(|a| a == "--if").and_then(|i| args.get(i + 1)).map(String::as_str);
            save_preset(serde_json::from_str(&text).map_err(|e| format!("not a valid preset: {e}"))?, expect)
        }
        (Some("set"), Some(name)) => {
            // KEY=VALUE changes without a script: total=12, shots.0.zoom=1.3, shots.*.speed=2, look="bw"; --as NEW
            let mut v = serde_json::to_value(preset::load(name)?).map_err(|e| e.to_string())?;
            let mut rest = args[2..].iter();
            let mut expect = None;
            while let Some(a) = rest.next() {
                if a == "--as" {
                    v["name"] = rest.next().ok_or("--as needs a name")?.as_str().into();
                    continue;
                }
                if a == "--if" {
                    expect = Some(rest.next().ok_or("--if needs the revision edits preset show printed")?.as_str());
                    continue;
                }
                let (k, val) = a.split_once('=').ok_or(format!("{a}: changes look like KEY=VALUE (shots.0.zoom=1.3)"))?;
                set_path(&mut v, k, serde_json::from_str(val).unwrap_or_else(|_| val.into()))?;
            }
            save_preset(serde_json::from_value(v).map_err(|e| format!("not a valid preset after that: {e}"))?, expect)
        }
        (Some("import"), Some(_)) => {
            // --into DIR writes elsewhere; default: your preset folder (edits packs make turns one into a pack)
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
        _ => Err("usage: edits preset list | show NAME | set NAME KEY=VALUE... [--as NEW] [--if REV] | save FILE.json|- [--if REV] | undo NAME | import <zip|xml|folder|prfpset|capcut draft>... [--into DIR]".into()),
    }
}

/// `edits rate clip.mp4 +|-` appends the clip's text to the taste file as a liked or disliked example.
fn rate(args: &[String]) -> Result<(), String> {
    let (clip, mark_liked) = match args {
        [clip, m] if m == "+" || m == "-" => (Path::new(clip), m == "+"),
        _ => return Err("usage: edits rate <clip.mp4> +|-".into()),
    };
    let side = std::fs::read_to_string(clip.with_extension("json")).map_err(|_| format!("no {} next to the clip", clip.with_extension("json").display()))?;
    let v: serde_json::Value = serde_json::from_str(&side).map_err(|e| e.to_string())?;
    let text = v["text"].as_str().unwrap_or("").trim();
    if text.is_empty() {
        return Err("that clip has no transcript to learn from".into());
    }
    stats::note(text, mark_liked)?;
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
    check(util::have("ffmpeg") && util::have("ffprobe"), "ffmpeg + ffprobe", "run `edits setup` (it downloads a static build)", true);
    let libass = util::run(None, "ffmpeg", &["-hide_banner", "-filters"]).is_ok_and(|f| f.contains(" ass "));
    check(libass, "ffmpeg has libass (captions)", "install a full ffmpeg build", true);
    let hw = util::auto_gpu();
    check(hw != "cpu", &format!("hardware encoder: {}", util::describe(hw)), "none works here; renders run on the CPU (slower, same picture)", false);
    check(util::have("yt-dlp"), "yt-dlp (links and topics)", "run `edits setup`; local files work without it", false);
    check(util::have("whisper-cli"), "whisper-cli (captions, transcript)", "install whisper.cpp", false);
    let model = transcribe::model();
    check(model.is_some(), &format!("whisper model{}", model.map(|m| format!(": {}", m.display())).unwrap_or_default()),
        "put a ggml-*.bin in ~/.local/share/edits/models or set EDITS_WHISPER_MODEL (ggml-large-v3-turbo is best for Turkish)", false);
    check(ai::ready(), &format!("AI: {}", ai::describe()), "choose one with `edits ai` (a CLI, an API key or a local server); without it cuts follow viewer signals only", false);
    println!("taste file: {}", pick::taste_path().display());
    if missing > 0 {
        return Err(format!("{missing} required tool(s) missing"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_packs_copy_whole_folders() {
        let root = std::env::temp_dir().join(format!("edits-skilltest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("pack/a/refs")).unwrap();
        std::fs::create_dir_all(root.join("pack/not-a-skill")).unwrap();
        std::fs::write(root.join("pack/a/SKILL.md"), "name: a").unwrap();
        std::fs::write(root.join("pack/a/refs/more.md"), "x").unwrap();
        let found = super::skill_folders(&root.join("pack"));
        assert_eq!(found.iter().map(|f| f.0.as_str()).collect::<Vec<_>>(), ["a"]);
        super::copy_tree(&found[0].1, &root.join("out/a")).unwrap();
        assert!(root.join("out/a/refs/more.md").is_file());
        assert!(super::CATALOG.iter().all(|c| c.1.contains('/')));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn say_stops_before_the_next_word() {
        let w = |s: f64, e: f64| transcribe::Word { s, e, t: String::new() };
        let caps = [w(443.1, 443.13), w(446.13, 446.5), w(446.5, 446.69)];
        assert_eq!(widen(&caps, 443.11, 446.2).unwrap().end, 446.5, "caption words: no tail into 'Genel'");
        let gap = [w(1.0, 1.4), w(2.0, 2.3)];
        assert_eq!(widen(&gap, 1.0, 1.5).unwrap().end, 1.65, "a real pause keeps the 0.25 s tail");
    }

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn a_voice_line_becomes_subtitles_on_the_edit_clock() {
        let w = |t: &str, s: f64, e: f64| crate::transcribe::Word { t: t.into(), s, e };
        let words = vec![w("önce", 9.0, 9.4), w("Herkes", 10.0, 10.4), w("soruyor,", 10.5, 11.0), w("yanlış", 11.2, 11.6), w("bu.", 11.7, 12.0), w("sonra", 12.1, 12.5)];
        let m = crate::pick::Moment { start: 10.0, end: 12.05, punch: 11.8, score: 0.0, title: String::new(), why: String::new() };
        let cues = super::voice_cues(&words, &m, 2.0);
        assert_eq!(cues.len(), 2);
        assert_eq!(cues[0].2, "Herkes soruyor,");
        assert_eq!(cues[1].2, "yanlış bu.");
        assert!((cues[0].0 - 2.0).abs() < 1e-9 && (cues[1].1 - 4.05).abs() < 1e-9);
        // YouTube's speaker mark is not a word
        let words = vec![w(">>", 10.0, 10.1), w("Yok", 10.2, 10.5), w("güzel.", 10.6, 11.0)];
        assert_eq!(super::voice_cues(&words, &m, 0.0)[0].2, "Yok güzel.");
    }

    #[test]
    fn hand_picked_shots_name_a_video_and_a_second() {
        let videos = vec![("abc123".to_string(), PathBuf::from("/c/abc123/v.mp4")), ("/home/me/clip.mp4".to_string(), PathBuf::from("/home/me/clip.mp4"))];
        assert_eq!(super::hand_shot("abc123@98.6", &videos), Some((0, 98.6, f64::MAX, None)));
        assert_eq!(super::hand_shot("clip@4-6.5", &videos), Some((1, 4.0, 6.5, None)), "a local file by its name");
        assert_eq!(super::hand_shot("clip@4:0.3", &videos), Some((1, 4.0, f64::MAX, Some(0.3))), "aimed at 0.3 across");
        assert_eq!(super::hand_shot("clip@4:2", &videos), None);
        assert_eq!(super::hand_shot("nope@3", &videos), None);
        assert_eq!(super::hand_shot("@3", &videos[..1]), Some((0, 3.0, f64::MAX, None)), "the only video needs no name");
    }

    #[test]
    fn an_offer_and_its_answer_join_into_one_line() {
        // shortcut: needs ffmpeg, which the CI runner lacks; it runs wherever edits renders
        if crate::util::run(None, "ffmpeg", &["-version"]).is_err() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("edits-say-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let audio = dir.join("x.wav");
        crate::util::run(None, "ffmpeg", &["-v", "error", "-y", "-f", "lavfi", "-i", "sine=d=10", &audio.to_string_lossy()]).unwrap();
        let w = |s: f64, e: f64, t: &str| crate::transcribe::Word { s, e, t: t.into() };
        let src = crate::fetch::Source { local: None, url: String::new(), work: dir.clone(), wav: audio.clone(), audio, id: "x".into(), title: String::new(),
            duration: 10.0, language: String::new(), heat: vec![], comments: vec![] };
        let spoken = vec![super::Analysed { src, words: vec![w(1.0, 2.0, "seç."), w(2.0, 3.0, "oturma"), w(6.0, 7.0, "memnunum.")], lines: vec![], moments: vec![], curve: vec![] }];
        let (a, m) = super::joined_say("x@0.9-1.9+1.5+5.9-6.9", &spoken, &dir).unwrap().unwrap();
        let said: Vec<&str> = a.words.iter().map(|w| w.t.as_str()).collect();
        assert_eq!(said, ["seç.", "memnunum."], "the words between the two stretches are left out");
        assert!((a.words[1].s - a.words[0].s - (1.98 - 0.95) - 1.5).abs() < 0.05, "the answer comes after the offer and the 1.5 s pause: {:?} {m:?}", a.words);
        let len = crate::fetch::duration(&a.src.audio).unwrap();
        assert!((len - m.end).abs() < 0.1, "the joined sound is as long as the line: {len} vs {}", m.end);
        assert!(super::joined_say("x@0.9-1.9", &spoken, &dir).is_none(), "one stretch is an ordinary --say");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_preset_that_marks_its_drop_puts_the_voice_before_it() {
        assert_eq!(super::voice_at(&[0.0, 4.0, 12.4, 19.4], &[0.1, 0.2, 0.3, 0.9], 7.9, 26.0, Some(12.4)), 12.4 - 7.9, "the marked drop, not the hardest cut");
    }

    #[test]
    fn a_preset_voice_ends_on_the_hardest_cut_it_fits_before() {
        let cuts = [0.0, 1.0, 2.0, 4.0, 6.0];
        let strength = [0.0, 0.9, 0.5, 0.8, 0.2];
        assert_eq!(super::voice_at(&cuts, &strength, 1.5, 8.0, None), 2.5, "cut 1.0 is too early for 1.5 s, so 4.0 (0.8) wins");
        assert_eq!(super::voice_at(&cuts, &strength, 9.0, 8.0, None), 0.3, "fits before no cut");
    }

    #[test]
    fn output_folders_are_readable_and_never_reused() {
        assert_eq!(folder_name("İskender Büyük'ten Tataroğlu Ailesine Ağır Darbe! | Kurtlar Vadisi Pusu"), "İskender Büyük'ten Tataroğlu Ailesine Ağır Darbe Kurtlar");
        assert_eq!(folder_name("../../etc/passwd"), "etc passwd");
        assert_eq!(folder_name("???"), "edit");
        assert_eq!(subject("sad cats"), "sad cats");
        let base = std::env::temp_dir().join(format!("edits-out-test-{}", std::process::id()));
        std::env::set_var("EDITS_OUT", &base);
        let first = auto_out("sad cats");
        assert!(first.starts_with(&base) && first.parent().unwrap().ends_with("sad cats"));
        std::fs::create_dir_all(&first).unwrap();
        assert!(auto_out("sad cats").to_string_lossy().ends_with(" (2)"));
        std::fs::remove_dir_all(&base).unwrap();
        assert!(parse(&args("x"), None).unwrap().out.as_os_str().is_empty(), "no --out means the automatic folder");
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
        let style = || Some(study::Style { aspect: "4:3".into(), grade: "nonsense".into(), lufs: -99.0, sfx: true, hook: true });
        let c = parse(&args("x"), style()).unwrap();
        assert_eq!((c.opts.w, c.opts.h, c.opts.grade.as_str(), c.opts.lufs), (1440, 1080, "bw", -30.0));
        assert_eq!(parse(&args("x --aspect 9:16"), style()).unwrap().opts.h, 1920);
        // --preview: one edit, and the frame it settles on comes out at half size
        let pv = parse(&args("x -n 3 --preview"), style()).unwrap();
        let mut o = pv.opts.clone();
        (o.w, o.h) = (1080, 1920);
        preview_size(&pv, &mut o);
        assert_eq!((pv.n, o.w, o.h), (1, 540, 960));
    }
}
