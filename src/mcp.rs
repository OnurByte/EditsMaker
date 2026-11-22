//! `edits mcp`: every command as an MCP tool over stdio (newline-delimited JSON-RPC 2.0), so an AI agent can make
//! edits, read a preset, change it for the moment and save it. Each tool runs this same binary.
use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

fn tools() -> Value {
    json!([
        {"name": "edits_make", "description": "Make edits. `input` is a YouTube link, a channel link, a topic to search, or a video file. With `preset` (a name, or a full preset object in `preset_json` changed for this edit) it replays that edit over the footage; with `music` (a song link/file or \"trend:N\" for TikTok's Nth trending song) it cuts music edits. Returns one JSON line per finished clip.",
         "inputSchema": {"type": "object", "properties": {
            "input": {"type": "string"},
            "preset": {"type": "string", "description": "preset name from edits_presets"},
            "preset_json": {"type": "object", "description": "a whole preset (as from edits_preset_get), changed as needed; used as-is"},
            "music": {"type": "string"},
            "look": {"type": "string", "description": "colour look from edits_looks: a built-in grade or one of the Alight Motion colour-pack looks (furkancc-1..4, rajjcc-1..20)"},
            "aspect": {"type": "string", "enum": ["9:16", "4:5", "1:1", "3:4", "4:3", "16:9", "21:9"]},
            "size": {"type": "string", "description": "WxH, like 1080x1350"},
            "intensity": {"type": "number", "description": "preset moves: 0.5 calmer, 2 wilder"},
            "n": {"type": "integer", "minimum": 1, "maximum": 20},
            "videos": {"type": "integer", "description": "videos to look at for a channel or topic"},
            "use": {"type": "integer", "description": "of those, how many to download after reading captions"},
            "sad": {"type": "boolean"}, "voice": {"type": "boolean"},
            "speed": {"type": "number", "description": "footage speed (talking clips: speech speed), 0.5 slow motion, 2 double"},
            "velocity": {"type": "boolean", "description": "speed ramps on every longer shot: slow, rushing, slow onto the beat"},
            "fx": {"type": "string", "description": "hits on the hardest beats, comma-separated: shake, flash, rgb, glitch, invert, blur, freeze, zoom, strobe, bw, pulse"},
            "lines": {"type": "integer", "description": "sentences from the videos' own words shown on screen (music and preset edits)"},
            "line_texts": {"type": "array", "items": {"type": "string"}, "description": "your own lines on screen, in order (read the videos' words and pick the strongest)"},
            "captions": {"type": "string", "description": "caption style from edits_looks: hormozi, mrbeast, karaoke, box, minimal, neon, cinematic, glow, redglow, thunder, typewriter"},
            "bed": {"type": "string", "description": "music under talking clips, ducked under the voice"},
            "blur_faces": {"type": "boolean"}, "blur_box": {"type": "string", "description": "x,y,w,h fractions of the frame to blur (a watermark, a plate)"},
            "anim": {"type": "string", "enum": ["rise", "pop", "slam", "slide", "drop", "blur", "type"], "description": "how captions_at and svgs come in: pop springs in, slam lands big on a beat, type writes itself out"},
            "fix": {"type": "boolean", "description": "dark, flat or dull footage brought up before the look"},
            "pips": {"type": "array", "items": {"type": "string"}, "description": "picture in picture: another video (file or library clip) small in a white frame over the edit, FILE[@FROM-TO][:X,Y,SIZE], centre and width as fractions, default top right"},
            "transition": {"type": "string", "description": "music edits: one ffmpeg xfade transition on every soft cut, e.g. dissolve, wipeleft, slideup, circleopen, pixelize, radial, squeezeh, coverleft"},
            "curves": {"type": "string", "description": "tone curve over the edit: vintage, strong_contrast, medium_contrast, increase_contrast, lighter, darker, cross_process, linear_contrast, negative, color_negative, or points \"0/0 0.4/0.3 1/1\""},
            "pop": {"type": "string", "enum": ["red", "yellow", "green", "cyan", "blue", "magenta"], "description": "colour pop: only this colour stays, the rest goes grey"},
            "bass": {"type": "number", "description": "low end boost in dB, -12..15 (6 = phonk)"},
            "pitch": {"type": "number", "description": "semitones up or down, -12..12, tempo kept"},
            "reverb": {"type": "boolean", "description": "a room behind the sound (slowed + reverb, sad edits)"},
            "stabilize": {"type": "boolean"}, "denoise": {"type": "boolean"}, "smooth": {"type": "boolean", "description": "motion-interpolated frames"},
            "fps": {"type": "integer"}, "codec": {"type": "string", "enum": ["h264", "h265", "prores", "av1"]}, "quality": {"type": "string", "enum": ["draft", "high", "master"]}, "encoder": {"type": "string", "enum": ["auto", "nvenc", "videotoolbox", "cpu"], "description": "auto (default) renders on Apple VideoToolbox on a Mac or an NVIDIA GPU elsewhere when it works, else the CPU"},
            "svgs": {"type": "array", "items": {"type": "string"}, "description": "drawn graphics over the edit to point things out: NAME[@FROM-TO][:X,Y,SIZE][#COLOUR][~ANIM], e.g. arrow@1.2-2.5:0.6,0.3,0.25 or circle@4:0.5,0.35#ffffff~pop. Names: arrow, circle, underline, cross, check, burst, question, exclaim, heart, sparkle, or an .svg file. X,Y is the centre and SIZE the width, as fractions of the frame; times are seconds of the finished edit"},
            "overlays": {"type": "array", "items": {"type": "string"}, "description": "library clips over the finished edit, NAME[@FROM[-TO]][:OPACITY] (names from edits_library): textures and backgrounds run under the whole edit screened, a transition or meme plays once at FROM, green screens are keyed. E.g. \"vhs texture:0.5\", \"burn 3@4.2\""},
            "whoosh": {"type": "string", "description": "talking clips: a library sound (edits_library sfx) on every cut instead of the made whoosh"},
            "boom": {"type": "string", "description": "talking clips: a library sound on the punchline instead of the made boom"},
            "export": {"type": "string", "description": "timelines beside each clip: fcpxml,edl,srt,vtt"},
            "shots": {"type": "array", "items": {"type": "string"}, "description": "preset edits: the footage for each shot by hand, in order, VIDEOID@SECONDS[:X] (a local file by its name; X 0..1 = where the frame looks across the source, for footage with no face; cycles when shorter than the preset). Use it whenever only some scenes or one character belong in the edit: look at the footage sheets, then list where each shot starts"},
            "say": {"type": "array", "items": {"type": "string"}, "description": "choose the moment yourself instead of the automatic pick: VIDEOID@FROM-TO seconds (preset edits: VIDEOID@A-B+1.2+C-D joins stretches into one line with 1.2 s of silence between, like an offer, a cut to someone else and the answer), one edit per entry, widened to whole words (talking and music edits: over their own footage; with a preset: over the preset edit, the song ducked under it, ending on its hardest cut); implies voice. Look at the captions and the footage sheets in the log first"},
            "lang": {"type": "string", "description": "spoken language, two letters like tr or en (default: the system's)"},
            "like": {"type": "string", "description": "a reference edit (mp4 or an X/TikTok/Instagram/YouTube link) to copy: its music, cuts, flashes, colour and shape"},
            "text": {"type": "string", "description": "one line over the whole music edit, in heavy capitals"},
            "length": {"type": "number", "description": "seconds per music edit (default 18, sad 24)"},
            "lut": {"type": "string", "description": "a .cube LUT file over the picture"},
            "fit": {"type": "boolean", "description": "keep the whole frame over a blurred copy instead of cropping around faces"},
            "join": {"type": "boolean", "description": "also join all clips into joined.mp4"},
            "preview": {"type": "boolean", "description": "one edit at half size, encoded fast: check it, then render again without"},
            "no_jumpcuts": {"type": "boolean"}, "no_sfx": {"type": "boolean", "description": "no made whoosh/boom (use for quiet, emotional edits)"},
            "no_hook": {"type": "boolean"}, "no_captions": {"type": "boolean"}, "no_llm": {"type": "boolean", "description": "pick moments from viewer signals only"},
            "tag": {"type": "string"},
            "captions_at": {"type": "array", "items": {"type": "string"}, "description": "words on screen at the top, TEXT@FROM-TO seconds of the edit, emoji drawn in colour: meme labels like \"sayın barış özbay 🗿@0-3.5\""},
            "say_text": {"type": "boolean", "description": "draw the spoken say line as it is said, in the captions style (cinematic, typewriter, glow...; default karaoke)"},
            "math": {"type": "boolean", "description": "formulas, sine waves and the song's waveform flashing in for a split second on the cuts (chaotic 'math' edits)"},
            "formulas": {"type": "array", "items": {"type": "string"}, "description": "your own formulas for the math layer, in LaTeX (\\int_0^\\infty e^{-x^2}dx, \\frac{a}{b}, \\sum, Greek); implies math"},
            "flash_texts": {"type": "array", "items": {"type": "string"}, "description": "subliminal lines, SECONDS:TEXT, each shown for two frames on black in the captions style"},
            "intro": {"type": "string", "description": "a short piece before the edit: VIDEOID@FROM-TO[/PICTURE_FROM] (that video's words from FROM to TO, its picture from PICTURE_FROM), or a video file"},
            "intro_bed": {"type": "string", "description": "a song under the intro (library name, file), quiet, stopped by a record scratch"},
            "intro_caption": {"type": "string", "description": "words over the intro, emoji allowed"},
            "outro": {"type": "string", "description": "after the edit: \"edits\" for the EditsMaker card, or a video file or library clip"},
            "loop": {"type": "boolean", "description": "the end runs into the start (picture and song), no fades: plays round seamlessly; rewatches count. Music and preset edits; edits_check reports how far apart the last and first frames are"},
            "long": {"type": "boolean", "description": "one edit over a minute, what TikTok's Creator Rewards pays for: talking clips joined into long.mp4, music edits 64 s, a preset played round on one song"},
            "lyrics": {"type": "boolean", "description": "the song's words on screen as they are sung (the record's lyrics from LRCLIB, else whisper), and music edits cut on the sung words"},
            "layout": {"type": "string", "enum": ["fill", "fit", "stack", "inset"], "description": "talking clips: fill crops around the faces, fit keeps the whole frame, stack puts a streamer's facecam over the game (chosen by itself when a facecam is found), inset lays the facecam small on top of the game"},
            "translate": {"type": "string", "description": "talking clips: the captions, hook and .srt in this language (tr, en, es...), the voice as said"},
            "corecore": {"type": "boolean", "description": "corecore: many unrelated videos of a topic, faded and grainy, dips and long dissolves, glimpses, headlines from their own words; needs music (slow, sad)"},
            "bg": {"type": "string", "description": "the person cut out (Robust Video Matting, people only) with another background: blur, a colour (black, #ff2a6d), or a picture/video file or library background"},
            "behind": {"type": "array", "items": {"type": "string"}, "description": "huge words between the background and the person, at the height the person covers least; TEXT or TEXT@FROM-TO seconds"},
            "outline": {"type": "string", "description": "a glowing outline around the person, a colour: white, #00e5ff"}},
          "required": ["input"]}},
        {"name": "edits_presets", "description": "List the presets: name, shots, length and what each does (pace fast/medium/slow, frame, and its moves: zoom, slide, spin, shake, blur, rgb, velocity, flash; transition = a short piece, song = brings its own). Names say what a preset does; old project names still work.", "inputSchema": {"type": "object", "properties": {}}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_preset_get", "description": "A preset in full as JSON: frame, song, look, intensity, and every shot with its speed and its zoom/x/y/rot/shake/blur/exposure/hue curves, contrast, kelvin, vignette, grain, glow, sharpen, pixelate, lens, trails, flip, letterbox, split, mirror, strobe, comic, scanlines, leak, fade_in, fade_out ([time 0..1 of the shot, value, easing]) plus flashes; mark one shot \"drop\": true and a say line ends as it starts. Change it and pass it to edits_make as preset_json, or keep it with edits_preset_save.",
         "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_preset_set", "description": "Change a preset field by field, no script: changes are KEY=VALUE like total=12, shots.0.zoom=1.3, shots.*.speed=2 or look=\"bw\" (lists count from 0, * is every shot; a value is JSON, else text). `as` saves it under a new name. `revision`: the one edits_preset_get's log gave; the change is refused if the preset changed since. edits_preset_undo takes a change back.",
         "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}, "changes": {"type": "array", "items": {"type": "string"}}, "as": {"type": "string"}, "revision": {"type": "string"}}, "required": ["name", "changes"]}},
        {"name": "edits_words", "description": "What a video says, line by line with times, each line as a ready say spec (ID@A-B) for edits_make say. Use it to find a line instead of transcribing yourself. `input`: a link, a YouTube ID or a file; `range` \"60-90\" seconds; `find` keeps the lines with those words.",
         "inputSchema": {"type": "object", "properties": {"input": {"type": "string"}, "range": {"type": "string"}, "find": {"type": "string"}, "lang": {"type": "string"}}, "required": ["input"]}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_moments", "description": "The moments worth a clip in a long video (a stream's VOD, a podcast, hours long): every 30 s scored by loudness against the whole video, reactions said out loud (in the video's language), how much is said and how hard the chat went, the best ones 90 s apart, judged by the clipping rubric. Each comes with flags (profanity, sexual, spoiler, loop), a sheet of 12 frames to look at before cutting, a post caption and its ready cut command (edits_make with say). Look at each sheet: is the promised thing on screen, where is the facecam, is it only a menu. `find`: regexes for lines already clipped elsewhere (quiet finds, one-line punchlines the scores miss).",
         "inputSchema": {"type": "object", "properties": {"input": {"type": "string", "description": "link, YouTube ID or file"}, "n": {"type": "integer", "description": "how many (default 20)"}, "find": {"type": "array", "items": {"type": "string"}}, "lang": {"type": "string", "description": "spoken language when the site does not say (tr, en)"}, "llm": {"type": "boolean", "description": "false: the scores alone pick"}, "sheets": {"type": "boolean", "description": "false: no frame sheets (faster)"}}, "required": ["input"]}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_preset_save", "description": "Save a preset (a whole preset object). Saved presets replace a built-in one with the same name. `revision` (from edits_preset_get's log) refuses the save if the preset changed since.",
         "inputSchema": {"type": "object", "properties": {"preset": {"type": "object"}, "revision": {"type": "string"}}, "required": ["preset"]}},
        {"name": "edits_preset_undo", "description": "Take a preset back one change: the version a save or set replaced, or the built-in one when the first change was over it. Each call goes one step further back.",
         "inputSchema": {"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}},
        {"name": "edits_learn", "description": "Reverse-engineer a finished edit (an mp4 the user likes, or its X/TikTok/Instagram/YouTube link) into a preset: its cuts, flashes and each shot's zoom, slide, turn and shake. Saved like any preset; use it with edits_make preset, read or change it with edits_preset_get.",
         "inputSchema": {"type": "object", "properties": {"video": {"type": "string", "description": "path or link of the edit"}}, "required": ["video"]}},
        {"name": "edits_preset_create", "description": "Make a new preset from scratch for a video: the video sets the rhythm (its cuts, extra cuts where it moves most) and which shots hit hardest; the moves, flashes and look are borrowed from other presets (`like`: names from edits_presets, default the three with the closest shot length). Saved like any preset; read it with edits_preset_get, change it, then edits_make preset.",
         "inputSchema": {"type": "object", "properties": {"video": {"type": "string", "description": "path of the footage"}, "name": {"type": "string"}, "like": {"type": "array", "items": {"type": "string"}, "description": "presets to take inspiration from"}, "length": {"type": "number", "description": "seconds of the video to use (default 18)"}}, "required": ["video"]}},
        {"name": "edits_check", "description": "Measure a finished clip: length, loudness (LUFS), cut rhythm, and write a contact sheet jpg beside it. Read the sheet to look at the edit before showing it; fast montages usually sit near -11 LUFS.",
         "inputSchema": {"type": "object", "properties": {"clip": {"type": "string", "description": "path of the rendered mp4, or a link to an edit"}}, "required": ["clip"]}},
        {"name": "edits_frames", "description": "Look at moments of footage before picking shots by hand: each time becomes a row of 6 frames over its length (default 3 s), all rows in one jpg to Read. Times as 336, 5:36 or 336+4 (4 s long). The video is a path, a link, or a YouTube ID edits already fetched. Then use the good ones as edits_make shots [\"ID@336\"].",
         "inputSchema": {"type": "object", "properties": {"video": {"type": "string", "description": "path, or a fetched video's ID"}, "at": {"type": "array", "items": {"type": "string"}}}, "required": ["video", "at"]}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_looks", "description": "Everything to style an edit with: colour looks, caption styles, the beat hits for fx and the drawn graphics for svgs.", "inputSchema": {"type": "object", "properties": {}}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_library", "description": "The user's asset library from imported editing packs: sound effects, music, overlays, transitions, green screens, backgrounds, memes, images. Without arguments: how many of each kind. With `kind` and/or `words`: the matching files. Use a name with edits_make music, overlays, whoosh or boom; a meme or background path works as `input`.",
         "inputSchema": {"type": "object", "properties": {"kind": {"type": "string", "enum": ["sfx", "music", "overlay", "transition", "greenscreen", "background", "meme", "video", "image"]}, "words": {"type": "string"}}}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_song", "description": "Which song plays in a clip, file or link (Shazam): artist - title and where in the song the clip starts. Use the name as edits_make music. `at` listens from that time (seconds or m:ss).",
         "inputSchema": {"type": "object", "properties": {"video": {"type": "string", "description": "path, link, or a fetched video's ID"}, "at": {"type": "string"}}, "required": ["video"]}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_cut", "description": "Cut stretches out of a video without re-encoding (lossless, instant): each span like \"12-20\" or \"1:05-1:12\" becomes a file in a new output folder. A span starts on the keyframe at or before its start.",
         "inputSchema": {"type": "object", "properties": {"video": {"type": "string", "description": "path, link, or a fetched video's ID"}, "spans": {"type": "array", "items": {"type": "string"}}}, "required": ["video", "spans"]}},
        {"name": "edits_trends", "description": "What is trending now in a country (two letters, default tr): TikTok's sounds, or with chart \"spotify\" the day's most streamed songs (country may be \"global\"). Use one with music \"trend:N\" or \"spotify:N\".",
         "inputSchema": {"type": "object", "properties": {"country": {"type": "string"}, "chart": {"type": "string", "enum": ["tiktok", "spotify"]}}}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_encoder", "description": "What renders the video, saved for every run: auto (the default: Apple VideoToolbox on a Mac, an NVIDIA GPU elsewhere, when its test encode works, else the CPU), nvenc, videotoolbox or cpu. Without `use`: what it is now and which hardware encoder works here.",
         "inputSchema": {"type": "object", "properties": {"use": {"type": "string", "enum": ["auto", "nvenc", "videotoolbox", "cpu"]}}}},
        {"name": "edits_stats", "description": "How posted edits did: a channel's or profile's latest posts (\"@name\" is a YouTube channel's Shorts; TikTok links work) with views, likes and comments, which of them are edits made with edits (matched by their sound against the render log) and each preset, song and look ranked by median views. Talking clips that did best and worst become taste examples. Use it before choosing a preset or song for that channel.",
         "inputSchema": {"type": "object", "properties": {"channel": {"type": "string"}, "n": {"type": "integer", "minimum": 1, "maximum": 200, "description": "posts to read (default 30)"}}, "required": ["channel"]}},
        {"name": "edits_review", "description": "Show a finished clip to the user in a window of its own (a Chromium-family app window, else their browser) and wait (up to 20 minutes) for their notes: they click a moment, a shot or a spot in the frame and say what to change. Returns each note with its time, shot number and place in the frame; change exactly that and render again. `saved`: true returns the last notes without opening the page.",
         "inputSchema": {"type": "object", "properties": {"clip": {"type": "string"}, "saved": {"type": "boolean"}}, "required": ["clip"]}},
        {"name": "edits_doctor", "description": "Check the tools edits needs (ffmpeg, yt-dlp, whisper).", "inputSchema": {"type": "object", "properties": {}}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_accounts", "description": "The accounts clips can be posted to (Instagram, TikTok, YouTube Shorts, Bluesky, Facebook Reels), by name.", "inputSchema": {"type": "object", "properties": {}}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_connect", "description": "Connect an Instagram, TikTok or YouTube Shorts account: a browser window opens and the user signs in there themselves (up to 10 minutes; edits never sees the password). Bluesky and Facebook need a secret typed in a terminal: tell the user to run `edits connect bluesky HANDLE` or `edits connect facebook PAGE_ID`. Unofficial routes can get an account restricted; say so.",
         "inputSchema": {"type": "object", "properties": {"platform": {"type": "string", "enum": ["instagram", "tiktok", "youtube"]}}, "required": ["platform"]}},
        {"name": "edits_post", "description": "Post finished clips to connected accounts (names from edits_accounts, a platform name, or \"all\"). The caption defaults to the one edits wrote for the clip. Ask the user before posting: it publishes.",
         "inputSchema": {"type": "object", "properties": {"clips": {"type": "array", "items": {"type": "string"}}, "to": {"type": "string"}, "caption": {"type": "string"}, "private": {"type": "boolean"}}, "required": ["clips", "to"]}},
        {"name": "edits_follow", "description": "Follow a YouTube channel, a Kick channel or an RSS/Atom feed: every new video is made into clips with `options` (edits command-line options such as [\"--preset\", \"NAME\", \"-n\", \"2\"]) and, with `post`, posted to those accounts. Videos there now are skipped. `every`: minutes between checks (default 60). It runs while `edits watch` or the background service (`edits service install`) does.",
         "inputSchema": {"type": "object", "properties": {"source": {"type": "string"}, "options": {"type": "array", "items": {"type": "string"}}, "post": {"type": "string"}, "every": {"type": "integer", "minimum": 5, "maximum": 1440}, "private": {"type": "boolean"}}, "required": ["source"]}},
        {"name": "edits_follows", "description": "What is followed, when each was last checked, and the latest automation events (clips made, posts, failures).", "inputSchema": {"type": "object", "properties": {}}, "annotations": {"readOnlyHint": true}},
        {"name": "edits_unfollow", "description": "Stop following a source: its number from edits_follows, or its link.", "inputSchema": {"type": "object", "properties": {"which": {"type": "string"}}, "required": ["which"]}}
    ])
}

/// The command line a tool call stands for.
pub fn args_for(name: &str, a: &Value) -> Result<Vec<String>, String> {
    let s = |k: &str| a[k].as_str().map(String::from);
    let mut v: Vec<String> = vec![];
    match name {
        "edits_presets" => v.extend(["preset".into(), "list".into()]),
        "edits_preset_get" => v.extend(["preset".into(), "show".into(), s("name").ok_or("name is needed")?]),
        "edits_preset_save" => {
            v.extend(["preset".into(), "save".into(), "-".into()]);
            v.extend(s("revision").into_iter().flat_map(|r| ["--if".into(), r]));
        }
        "edits_preset_undo" => v.extend(["preset".into(), "undo".into(), s("name").ok_or("name is needed")?]),
        "edits_preset_set" => {
            v.extend(["preset".into(), "set".into(), s("name").ok_or("name is needed")?]);
            v.extend(a["changes"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)));
            if let Some(n) = s("as") {
                v.extend(["--as".into(), n]);
            }
            v.extend(s("revision").into_iter().flat_map(|r| ["--if".into(), r]));
        }
        "edits_moments" => {
            v.extend(["moments".into(), s("input").ok_or("input is needed")?]);
            if let Some(n) = a["n"].as_u64() {
                v.extend(["-n".into(), n.to_string()]);
            }
            for f in a["find"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--find".into(), f.into()]);
            }
            if let Some(l) = s("lang") {
                v.extend(["--lang".into(), l]);
            }
            if a["llm"] == false {
                v.push("--no-llm".into());
            }
            if a["sheets"] == false {
                v.push("--no-sheets".into());
            }
        }
        "edits_words" => {
            v.extend(["words".into(), s("input").ok_or("input is needed")?]);
            v.extend(s("range"));
            for k in ["find", "lang"] {
                if let Some(x) = s(k) {
                    v.extend([format!("--{k}"), x]);
                }
            }
        }
        "edits_looks" => v.push("looks".into()),
        "edits_check" => v.extend(["check".into(), s("clip").ok_or("clip is needed")?]),
        "edits_frames" => {
            v.extend(["frames".into(), s("video").ok_or("video is needed")?]);
            v.extend(a["at"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from).or(x.as_f64().map(|n| n.to_string()))));
        }
        "edits_preset_create" => {
            v.extend(["preset".into(), "create".into(), s("video").ok_or("video is needed")?]);
            for (key, flag) in [("name", "--name"), ("length", "--length")] {
                if let Some(x) = a[key].as_str().map(String::from).or(a[key].as_f64().map(|n| n.to_string())) {
                    v.extend([flag.into(), x]);
                }
            }
            let like: Vec<&str> = a["like"].as_array().into_iter().flatten().filter_map(|x| x.as_str()).collect();
            if !like.is_empty() {
                v.extend(["--like".into(), like.join(",")]);
            }
        }
        "edits_learn" => v.extend(["learn".into(), s("video").ok_or("video is needed")?]),
        "edits_cut" => {
            v.extend(["cut".into(), s("video").ok_or("video is needed")?]);
            v.extend(a["spans"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)));
        }
        "edits_song" => {
            v.extend(["song".into(), s("video").ok_or("video is needed")?]);
            v.extend(s("at"));
        }
        "edits_doctor" => v.push("doctor".into()),
        "edits_accounts" => v.push("accounts".into()),
        "edits_connect" => v.extend(["connect".into(), s("platform").ok_or("platform is needed")?]),
        "edits_post" => {
            v.push("post".into());
            v.extend(a["clips"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)));
            v.extend(["--to".into(), s("to").ok_or("to is needed")?]);
            if let Some(c) = s("caption") {
                v.extend(["--caption".into(), c]);
            }
            if a["private"].as_bool() == Some(true) {
                v.push("--private".into());
            }
        }
        "edits_follow" => {
            v.extend(["follow".into(), s("source").ok_or("source is needed")?]);
            v.extend(a["every"].as_u64().map(|n| ["--every".to_string(), n.to_string()]).into_iter().flatten());
            v.extend(s("post").map(|p| ["--post".to_string(), p]).into_iter().flatten());
            if a["private"].as_bool() == Some(true) {
                v.push("--private".into());
            }
            v.extend(a["options"].as_array().into_iter().flatten().filter_map(|x| x.as_str().map(String::from)));
        }
        "edits_follows" => v.push("follows".into()),
        "edits_unfollow" => v.extend(["unfollow".into(), s("which").ok_or("which is needed")?]),
        "edits_stats" => {
            v.extend(["stats".into(), s("channel").ok_or("channel is needed")?]);
            v.extend(a["n"].as_u64().map(|n| n.to_string()));
        }
        "edits_review" => {
            v.extend(["review".into(), s("clip").ok_or("clip is needed")?]);
            if a["saved"].as_bool() == Some(true) {
                v.push("--notes".into());
            }
        }
        "edits_encoder" => {
            v.push("encoder".into());
            v.extend(s("use"));
        }
        "edits_library" => {
            v.push("library".into());
            v.extend(s("kind"));
            v.extend(s("words").iter().flat_map(|w| w.split_whitespace().map(String::from)));
        }
        "edits_trends" => {
            v.push("trends".into());
            if s("chart").as_deref() == Some("spotify") {
                v.push("spotify".into());
            }
            v.push(s("country").unwrap_or_else(|| crate::lang::system().1));
        }
        "edits_make" => {
            v.push(s("input").ok_or("input is needed")?);
            for (key, flag) in [("preset", "--preset"), ("music", "--music"), ("look", "--look"), ("aspect", "--aspect"), ("size", "--size"), ("tag", "--tag"),
                ("fx", "--fx"), ("captions", "--captions"), ("bed", "--bed"), ("whoosh", "--whoosh"), ("boom", "--boom"), ("blur_box", "--blur-box"), ("codec", "--codec"), ("quality", "--quality"), ("encoder", "--encoder"), ("export", "--export"),
                ("lang", "--lang"), ("like", "--like"), ("text", "--text"), ("lut", "--lut"), ("intro", "--intro"), ("intro_bed", "--intro-bed"), ("intro_caption", "--intro-caption"), ("outro", "--outro"), ("bg", "--bg"), ("outline", "--outline"), ("transition", "--transition"), ("curves", "--curves"), ("pop", "--pop"), ("anim", "--anim"), ("layout", "--layout"), ("translate", "--translate")] {
                if let Some(x) = s(key) {
                    v.extend([flag.into(), x]);
                }
            }
            for (key, flag) in [("n", "-n"), ("videos", "--videos"), ("use", "--use"), ("intensity", "--intensity"), ("speed", "--speed"), ("lines", "--lines"), ("fps", "--fps"), ("length", "--length"), ("bass", "--bass"), ("pitch", "--pitch")] {
                if let Some(x) = a[key].as_f64() {
                    v.extend([flag.into(), if matches!(key, "intensity" | "speed" | "length" | "bass" | "pitch") { x.to_string() } else { (x as u64).to_string() }]);
                }
            }
            for t in a["line_texts"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--line".into(), t.into()]);
            }
            for t in a["overlays"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--overlay".into(), t.into()]);
            }
            for t in a["pips"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--pip".into(), t.into()]);
            }
            for t in a["svgs"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--svg".into(), t.into()]);
            }
            for t in a["shots"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--shot".into(), t.into()]);
            }
            for t in a["say"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--say".into(), t.into()]);
            }
            for t in a["captions_at"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--caption".into(), t.into()]);
            }
            for t in a["formulas"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--formula".into(), t.into()]);
            }
            for t in a["flash_texts"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--flash-text".into(), t.into()]);
            }
            for t in a["behind"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                v.extend(["--behind".into(), t.into()]);
            }
            for (key, flag) in [("fit", "--fit"), ("join", "--join"), ("preview", "--preview"), ("no_jumpcuts", "--no-jumpcuts"), ("no_sfx", "--no-sfx"), ("no_hook", "--no-hook"), ("no_captions", "--no-captions"), ("no_llm", "--no-llm"),
                ("sad", "--sad"), ("voice", "--voice"), ("velocity", "--velocity"), ("blur_faces", "--blur-faces"), ("stabilize", "--stabilize"), ("denoise", "--denoise"), ("smooth", "--smooth"), ("say_text", "--say-text"), ("math", "--math"), ("reverb", "--reverb"), ("fix", "--fix"), ("loop", "--loop"), ("long", "--long"), ("lyrics", "--lyrics"), ("corecore", "--corecore")] {
                if a[key].as_bool() == Some(true) {
                    v.push(flag.into());
                }
            }
        }
        _ => return Err(format!("no tool {name}")),
    }
    Ok(v)
}

fn call(name: &str, a: &Value) -> Result<String, String> {
    let mut args = args_for(name, a)?;
    let mut stdin = None;
    if name == "edits_preset_save" {
        stdin = Some(a["preset"].to_string());
    }
    // a changed preset for this edit only: written to a file and named by path
    // calls run side by side: each changed preset gets its own file
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let tmp = std::env::temp_dir().join(format!("edits-mcp-{}-{}.json", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
    if name == "edits_make" && a["preset_json"].is_object() {
        std::fs::write(&tmp, a["preset_json"].to_string()).map_err(|e| e.to_string())?;
        args.extend(["--preset".into(), tmp.to_string_lossy().into_owned()]);
    }
    let exe = EXE.get().cloned().map_or_else(std::env::current_exe, Ok).map_err(|e| e.to_string())?;
    let mut child = std::process::Command::new(exe).args(&args)
        .stdin(if stdin.is_some() { std::process::Stdio::piped() } else { std::process::Stdio::null() })
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped())
        .spawn().map_err(|e| e.to_string())?;
    if let (Some(text), Some(mut pipe)) = (stdin, child.stdin.take()) {
        let _ = pipe.write_all(text.as_bytes());
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let _ = std::fs::remove_file(&tmp);
    let (o, e) = (String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    // progress lines help an agent see what was skipped and why; keep the last of them
    let log: Vec<&str> = e.lines().rev().take(40).collect::<Vec<_>>().into_iter().rev().collect();
    let text = format!("{}{}", o.trim(), if log.is_empty() { String::new() } else { format!("\n\n--- log ---\n{}", log.join("\n")) });
    if out.status.success() { Ok(text) } else { Err(text) }
}

fn reply(id: &Value, result: Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

/// Sent to the agent on connect: agents that script around the tools break things the tools already do.
const INSTRUCTIONS: &str = "Every editing job has a command here. Never write Python, shell loops, ffmpeg, ffprobe, whisper or yt-dlp yourself, not even to test: they break where these work. Find where a line is said: edits_words. Look at footage: edits_frames. Measure a render: edits_check. Show a render to the user and get their notes: edits_review. See what worked on a channel: edits_stats. Name the song in a clip: edits_song. Change a preset: edits_preset_set (or get, change, edits_preset_save). Render: edits_make. If no tool does what you need, say so to the user instead of scripting around it.";

pub fn handle(msg: &Value) -> Option<Value> {
    let id = msg.get("id")?.clone(); // notifications get no answer
    let method = msg["method"].as_str().unwrap_or("");
    Some(match method {
        "initialize" => reply(&id, json!({"protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
            "capabilities": {"tools": {}}, "instructions": INSTRUCTIONS, "serverInfo": {"name": "edits", "version": env!("CARGO_PKG_VERSION")}})),
        "ping" => reply(&id, json!({})),
        "tools/list" => reply(&id, json!({"tools": tools()})),
        "tools/call" => {
            let name = msg["params"]["name"].as_str().unwrap_or("");
            let r = call(name, &msg["params"]["arguments"]);
            let (text, err) = match r { Ok(t) => (t, false), Err(t) => (t, true) };
            reply(&id, json!({"content": [{"type": "text", "text": text}], "isError": err}))
        }
        _ => json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32601, "message": format!("no method {method}")}}),
    })
}

/// Where each agent keeps its MCP servers, as a JSON file: (client, file, key holding the servers, the entry).
/// Clients whose config is not plain JSON (Claude Code's ~/.claude.json is live state, ChatGPT's codex is TOML) are added
/// through their own CLI instead.
fn json_clients(exe: &str) -> Vec<(&'static str, PathBuf, &'static [&'static str], Value)> {
    let h = crate::util::home();
    let roaming = std::env::var("APPDATA").map(PathBuf::from).unwrap_or_else(|_| h.join("AppData/Roaming"));
    let app = |linux: &str, mac: &str, win: &str| if cfg!(windows) { roaming.join(win) } else if cfg!(target_os = "macos") { h.join("Library/Application Support").join(mac) } else { h.join(".config").join(linux) };
    let plain = json!({"command": exe, "args": ["mcp"]});
    vec![
        ("OpenCode", h.join(".config/opencode/opencode.json"), &["mcp", "servers"], json!({"type": "local", "command": [exe, "mcp"]})),
        ("Cursor", h.join(".cursor/mcp.json"), &["mcpServers"], plain.clone()),
        ("Windsurf", h.join(".codeium/windsurf/mcp_config.json"), &["mcpServers"], plain.clone()),
        ("Gemini CLI", h.join(".gemini/settings.json"), &["mcpServers"], plain.clone()),
        ("Claude Desktop", app("Claude", "Claude", "Claude").join("claude_desktop_config.json"), &["mcpServers"], plain),
        ("VS Code", app("Code/User", "Code/User", "Code/User").join("mcp.json"), &["servers"], json!({"type": "stdio", "command": exe, "args": ["mcp"]})),
    ]
}

/// Puts `entry` at keys.."edits" in a JSON config, keeping everything else; a file that is not JSON is left alone.
fn add_to(text: &str, keys: &[&str], entry: Value) -> Option<String> {
    let mut v: Value = if text.trim().is_empty() { json!({}) } else { serde_json::from_str(text).ok()? };
    let mut at = &mut v;
    for k in keys {
        at = at.as_object_mut()?.entry(*k).or_insert_with(|| json!({}));
    }
    at.as_object_mut()?.insert("edits".into(), entry);
    serde_json::to_string_pretty(&v).ok().map(|s| s + "\n")
}

/// `edits mcp install`: edits as an MCP server in every agent on this machine (one whose config folder exists).
pub fn install() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?.to_string_lossy().into_owned();
    let mut done = vec![];
    // their own CLIs; removed first so a moved binary is picked up again
    for (name, cli, rm, add) in [
        ("Claude Code", "claude", vec!["mcp", "remove", "-s", "user", "edits"], vec!["mcp", "add", "-s", "user", "edits", "--", &exe, "mcp"]),
        ("ChatGPT (codex)", "codex", vec!["mcp", "remove", "edits"], vec!["mcp", "add", "edits", "--", &exe, "mcp"]),
    ] {
        if crate::util::have(cli) {
            let _ = crate::util::run(None, cli, &rm);
            match crate::util::run(None, cli, &add) {
                Ok(_) => done.push(name),
                Err(e) => eprintln!("edits: {name}: {e}"),
            }
        }
    }
    for (name, file, keys, entry) in json_clients(&exe) {
        if !file.parent().is_some_and(Path::is_dir) {
            continue;
        }
        let text = std::fs::read_to_string(&file).unwrap_or_default();
        match add_to(&text, keys, entry) {
            Some(out) => {
                std::fs::write(&file, out).map_err(|e| format!("{}: {e}", file.display()))?;
                done.push(name);
            }
            None => eprintln!("edits: {name}: {} is not plain JSON, add it by hand: command {exe}, args [\"mcp\"]", file.display()),
        }
    }
    // the guard changes how Claude Code works everywhere: only when asked for (edits guard install)
    if crate::util::home().join(".claude").is_dir() && !guarded() {
        println!("edits: to stop Claude Code running ffmpeg, yt-dlp or whisper by hand: edits guard install (or --ask to be asked each time)");
    }
    if done.is_empty() {
        println!("no agents found; add it by hand: command {exe}, args [\"mcp\"]");
    } else {
        println!("edits MCP server added to {} (restart them to see it)", done.join(", "));
    }
    Ok(())
}

/// The rule every agent gets: a shell command that runs ffmpeg, ffprobe, yt-dlp or whisper itself. Grepping for
/// the word, a commit message or cargo test are not that.
pub fn by_hand(cmd: &str) -> bool {
    let re = regex::Regex::new(r#"(?m)(^|[;&|(`]|\$\()\s*((sudo|nohup|exec|time)\s+|timeout\s+\S+\s+|[A-Za-z_]+=\S*\s+)*(\S*/)?(ffmpeg|ffprobe|ffplay|yt-dlp|whisper-cli|whisper)(\.exe)?(\s|$)"#).unwrap();
    re.is_match(cmd)
}

const RULE: &str = "edits: no ffmpeg, ffprobe, yt-dlp or whisper by hand, not even to test. Use edits: edits words (find a line), edits frames (look at footage or a render), edits check (measure a render), edits make (render). If edits cannot do it, tell the user what is missing instead of scripting around it. (This rule is the user's choice, set with `edits guard install`; `edits guard remove` turns it off.)";

/// `edits guard [--ask]`: a Claude Code PreToolUse hook. Reads the tool call on stdin; a Bash command that does
/// video work by hand is refused (exit 2, the reason goes to the agent), or with --ask put to the user to allow.
pub fn guard(args: &[String]) -> Result<(), String> {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input).map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&input).unwrap_or_default();
    if by_hand(v["tool_input"]["command"].as_str().unwrap_or_default()) {
        if args.iter().any(|a| a == "--ask") {
            println!("{}", ask(RULE));
            return Ok(());
        }
        eprintln!("{RULE}");
        std::process::exit(2);
    }
    Ok(())
}

/// Claude Code's answer from a PreToolUse hook that leaves the call to the user.
fn ask(why: &str) -> Value {
    json!({"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "ask", "permissionDecisionReason": why}})
}

/// Is this hook entry edits' guard?
fn is_guard(e: &Value) -> bool {
    e["hooks"].as_array().is_some_and(|h| h.iter().any(|x| x["command"].as_str().is_some_and(|c| c.contains("edits") && (c.ends_with(" guard") || c.ends_with(" guard --ask")))))
}

/// Claude Code's user settings.
fn settings() -> PathBuf {
    crate::util::home().join(".claude").join("settings.json")
}

/// Whether the guard is in Claude Code's user settings.
pub fn guarded() -> bool {
    std::fs::read_to_string(settings()).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok())
        .is_some_and(|v| v["hooks"]["PreToolUse"].as_array().is_some_and(|l| l.iter().any(is_guard)))
}

/// The guard hook in a Claude Code settings.json, keeping everything else: `exe` is edits' path, `ask` puts the
/// call to the user instead of refusing it, `on` false takes it out. None when the text is not plain JSON.
fn set_guard(text: &str, exe: &str, ask: bool, on: bool) -> Option<String> {
    let mut v: Value = if text.trim().is_empty() { json!({}) } else { serde_json::from_str(text).ok()? };
    let hooks = v.as_object_mut()?.entry("hooks").or_insert_with(|| json!({}));
    let pre = hooks.as_object_mut()?.entry("PreToolUse").or_insert_with(|| json!([]));
    let list = pre.as_array_mut()?;
    // an older edits path or mode is replaced, not added twice
    list.retain(|e| !is_guard(e));
    if on {
        list.push(json!({"matcher": "Bash", "hooks": [{"type": "command", "command": format!("\"{exe}\" guard{}", if ask { " --ask" } else { "" })}]}));
    } else if list.is_empty() {
        // nothing of edits' left behind
        hooks.as_object_mut()?.remove("PreToolUse");
        if hooks.as_object().is_some_and(|h| h.is_empty()) {
            v.as_object_mut()?.remove("hooks");
        }
    }
    serde_json::to_string_pretty(&v).ok().map(|s| s + "\n")
}

fn write_guard(ask: bool, on: bool) -> Result<bool, String> {
    let file = settings();
    let exe = std::env::current_exe().map_err(|e| e.to_string())?.to_string_lossy().into_owned();
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    let out = set_guard(&text, &exe, ask, on).ok_or(format!("{} is not plain JSON; change the edits guard hook by hand", file.display()))?;
    if out == text {
        return Ok(false);
    }
    std::fs::write(&file, out).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(true)
}

/// `edits guard install [--ask]` and `edits guard remove`: only ever on the user's word.
pub fn guard_cmd(args: &[String]) -> Result<(), String> {
    let ask = args.iter().any(|a| a == "--ask");
    match args.first().map(String::as_str) {
        Some("install") => {
            if !crate::util::home().join(".claude").is_dir() {
                return Err("Claude Code is not on this machine (no ~/.claude)".into());
            }
            write_guard(ask, true)?;
            println!("edits: Claude Code {} ffmpeg, yt-dlp and whisper run by hand ({}); edits guard remove takes it out",
                if ask { "asks you before running" } else { "now refuses" }, settings().display());
        }
        Some("refresh") => guard_refresh(),
        Some("remove") => {
            let changed = write_guard(false, false)?;
            println!("edits: {}", if changed { format!("the guard is out of {}", settings().display()) } else { "there was no guard to remove".into() });
        }
        _ => return guard(args),
    }
    Ok(())
}

/// After an update: a guard the user installed points at the new edits, keeping its mode. Never adds one.
pub fn guard_refresh() {
    let text = std::fs::read_to_string(settings()).unwrap_or_default();
    let ask = text.contains(" guard --ask");
    if guarded() {
        let _ = write_guard(ask, true);
    }
}

/// The binary's path as the server started: once `edits update` replaces the file, Linux names the running one
/// ".../edits (deleted)", and tool calls must start the new edits at the old path instead.
static EXE: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();

fn send(answer: Option<Value>) {
    if let Some(a) = answer {
        // one line per message, never interleaved: the lock holds stdout for the whole line
        let mut out = std::io::stdout().lock();
        let _ = writeln!(out, "{a}").and_then(|_| out.flush());
    }
}

pub fn serve() -> Result<(), String> {
    if let Ok(p) = std::env::current_exe() {
        let _ = EXE.set(p);
    }
    let mut running = vec![];
    for line in std::io::stdin().lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(&line) {
            // a render takes minutes: it runs beside the rest, so pings, lists and other calls are answered meanwhile
            Ok(m) if m["method"] == "tools/call" => {
                running.retain(|t: &std::thread::JoinHandle<()>| !t.is_finished());
                running.push(std::thread::spawn(move || send(handle(&m))));
            }
            Ok(m) => send(handle(&m)),
            Err(e) => send(Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}}))),
        }
    }
    // the client closed its end: calls still running finish and answer before the server goes
    running.into_iter().for_each(|t| { let _ = t.join(); });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_a_config_without_losing_it() {
        let out = add_to(r#"{"theme": "x", "mcp": {"servers": {"rea": {"type": "local"}}}}"#, &["mcp", "servers"], json!({"type": "local"})).unwrap();
        let v: Value = serde_json::from_str(&out).unwrap();
        assert!(v["theme"] == "x" && v["mcp"]["servers"]["rea"].is_object() && v["mcp"]["servers"]["edits"]["type"] == "local", "{out}");
        assert!(add_to("", &["mcpServers"], json!({})).unwrap().contains("\"edits\""));
        assert!(add_to("// comments\n{}", &["mcpServers"], json!({})).is_none(), "JSONC is left alone");
        // the guard joins other hooks, and a moved edits replaces itself instead of piling up
        let one = set_guard(r#"{"model": "x", "hooks": {"PreToolUse": [{"matcher": "Edit", "hooks": []}]}}"#, "/old/edits", false, true).unwrap();
        let two: Value = serde_json::from_str(&set_guard(&one, "/new/edits", true, true).unwrap()).unwrap();
        let pre = two["hooks"]["PreToolUse"].as_array().unwrap();
        assert!(two["model"] == "x" && pre.len() == 2 && pre[1]["hooks"][0]["command"] == "\"/new/edits\" guard --ask", "{two}");
        // removed, the user's own hooks stay; with nothing else, nothing of edits' is left
        let off: Value = serde_json::from_str(&set_guard(&two.to_string(), "/new/edits", false, false).unwrap()).unwrap();
        assert!(off["hooks"]["PreToolUse"].as_array().unwrap().len() == 1 && off["model"] == "x", "{off}");
        let bare: Value = serde_json::from_str(&set_guard(&set_guard("{\"a\": 1}", "/e/edits", false, true).unwrap(), "/e/edits", false, false).unwrap()).unwrap();
        assert_eq!(bare, json!({"a": 1}));
        assert!(set_guard("// jsonc\n{}", "/e/edits", false, true).is_none());
        assert_eq!(ask("why")["hookSpecificOutput"]["permissionDecision"], "ask");
        for bad in ["ffmpeg -i a.mp4 b.png", "cd x && ffmpeg -y o.mp4", "nohup /usr/bin/yt-dlp URL", "whisper-cli -m m.bin", "X=1 ffprobe a.mp4", "a; timeout 60 ffmpeg -i x", "ls\nffmpeg -i x"] {
            assert!(by_hand(bad), "{bad}");
        }
        for ok in ["cargo test", "grep -n ffmpeg src/wrap.rs", "edits frames ID 30", "git commit -m \"no ffmpeg\"", "edits doctor"] {
            assert!(!by_hand(ok), "{ok}");
        }
    }

    #[test]
    fn protocol() {
        let init = handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18"}})).unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "edits");
        assert!(init["result"]["instructions"].as_str().unwrap().contains("Never write Python"));
        assert!(handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).is_none());
        let list = handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"})).unwrap();
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 27);
        assert_eq!(args_for("edits_preset_set", &json!({"name": "x", "changes": ["total=12"], "revision": "ab12cd34"})).unwrap(), ["preset", "set", "x", "total=12", "--if", "ab12cd34"]);
        assert_eq!(args_for("edits_preset_undo", &json!({"name": "x"})).unwrap(), ["preset", "undo", "x"]);
        assert_eq!(args_for("edits_song", &json!({"video": "a.mp4", "at": "0:30"})).unwrap(), ["song", "a.mp4", "0:30"]);
        assert_eq!(handle(&json!({"jsonrpc": "2.0", "id": 3, "method": "nope"})).unwrap()["error"]["code"], -32601);
    }

    #[test]
    fn tool_calls_become_command_lines() {
        let a = json!({"input": "https://www.youtube.com/@x", "preset": "megumi", "music": "trend:2", "n": 2, "intensity": 1.5, "sad": true, "voice": false, "fx": "shake,freeze", "velocity": true, "lines": 3, "out": "work/x"});
        assert_eq!(args_for("edits_make", &a).unwrap(), ["https://www.youtube.com/@x", "--preset", "megumi", "--music", "trend:2", "--fx", "shake,freeze", "-n", "2", "--intensity", "1.5", "--lines", "3", "--sad", "--velocity"]);
        assert_eq!(args_for("edits_check", &json!({"clip": "a.mp4"})).unwrap(), ["check", "a.mp4"]);
        assert_eq!(args_for("edits_preset_set", &json!({"name": "yura", "changes": ["total=12", "shots.*.speed=2"], "as": "y2"})).unwrap(), ["preset", "set", "yura", "total=12", "shots.*.speed=2", "--as", "y2"]);
        assert_eq!(args_for("edits_words", &json!({"input": "abc", "range": "60-90", "find": "borç"})).unwrap(), ["words", "abc", "60-90", "--find", "borç"]);
        assert_eq!(args_for("edits_moments", &json!({"input": "https://twitch.tv/videos/1", "n": 10, "find": ["uçuyor", "no way"], "lang": "tr", "sheets": false})).unwrap(),
            ["moments", "https://twitch.tv/videos/1", "-n", "10", "--find", "uçuyor", "--find", "no way", "--lang", "tr", "--no-sheets"]);
        assert_eq!(args_for("edits_make", &json!({"input": "abc", "layout": "stack", "translate": "tr"})).unwrap(), ["abc", "--layout", "stack", "--translate", "tr"]);
        assert_eq!(args_for("edits_frames", &json!({"video": "abc", "at": ["5:36", 390]})).unwrap(), ["frames", "abc", "5:36", "390"]);
        assert_eq!(args_for("edits_preset_create", &json!({"video": "a.mp4", "name": "x", "like": ["yura", "madara"], "length": 12})).unwrap(), ["preset", "create", "a.mp4", "--name", "x", "--length", "12", "--like", "yura,madara"]);
        assert_eq!(args_for("edits_preset_get", &json!({"name": "yura"})).unwrap(), ["preset", "show", "yura"]);
        assert_eq!(args_for("edits_library", &json!({"kind": "sfx", "words": "fast woosh"})).unwrap(), ["library", "sfx", "fast", "woosh"]);
        assert_eq!(args_for("edits_make", &json!({"input": "a.mp4", "whoosh": "main swoosh", "overlays": ["vhs:0.5"]})).unwrap(), ["a.mp4", "--whoosh", "main swoosh", "--overlay", "vhs:0.5"]);
        assert_eq!(args_for("edits_make", &json!({"input": "a.mp4", "say": ["abc@10-40"], "lang": "tr", "length": 20.5, "no_sfx": true, "fit": false})).unwrap(), ["a.mp4", "--lang", "tr", "--length", "20.5", "--say", "abc@10-40", "--no-sfx"]);
        assert_eq!(args_for("edits_make", &json!({"input": "a.mp4", "outro": "edits", "intro": "x@1-3", "captions_at": ["hi 🗿@0-2"], "flash_texts": ["3:yo"], "math": true, "say_text": true})).unwrap(),
            ["a.mp4", "--intro", "x@1-3", "--outro", "edits", "--caption", "hi 🗿@0-2", "--flash-text", "3:yo", "--say-text", "--math"]);
        assert_eq!(args_for("edits_make", &json!({"input": "a.mp4", "music": "trend", "bg": "blur", "outline": "white", "behind": ["RUN@1-3"], "loop": true, "long": true, "lyrics": true, "corecore": true})).unwrap(),
            ["a.mp4", "--music", "trend", "--bg", "blur", "--outline", "white", "--behind", "RUN@1-3", "--loop", "--long", "--lyrics", "--corecore"]);
        assert_eq!(args_for("edits_stats", &json!({"channel": "@x", "n": 20})).unwrap(), ["stats", "@x", "20"]);
        assert_eq!(args_for("edits_review", &json!({"clip": "a.mp4", "saved": true})).unwrap(), ["review", "a.mp4", "--notes"]);
        assert!(args_for("edits_make", &json!({})).is_err() && args_for("nope", &json!({})).is_err());
    }
}
