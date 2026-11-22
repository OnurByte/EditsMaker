---
name: edits
description: Make short viral edits (TikTok, Reels, Shorts) from a YouTube link, a channel, a topic or a video file with the `edits` command. Use when asked to make an edit, a sad edit, a velocity/anime-style edit, or clips from a video or channel.
---

# Making edits with `edits`

**Use the commands, never your own scripts.** Every step has one: `edits words` finds where a line is said (ready
`--say ID@A-B`), `edits frames` shows footage, `edits check` measures a render, `edits preset set` changes a preset,
`edits song` names the song in a clip, `edits cut` copies a stretch out losslessly, `edits looks` / `edits library` list
what there is. Do not write Python, shell loops, ffmpeg, whisper or yt-dlp
commands, not even to test a change: they are slower and break where these work, and in Claude Code `edits guard`
refuses them. If no command does it, tell the user what is missing.

## Work like an editor, not a command runner

The tool alone makes a generic edit: it picks moments from the transcript (a `claude -p` call that reads text, never
sees or hears the footage) and replays a preset. What makes an edit land is your judgment on top. Every edit:

1. **Load the tools first.** If `mcp__edits__*` is listed as deferred, load all of it in one call:
   `ToolSearch select:mcp__edits__edits_make,mcp__edits__edits_presets,mcp__edits__edits_preset_get,mcp__edits__edits_preset_save,mcp__edits__edits_looks,mcp__edits__edits_trends,mcp__edits__edits_library,mcp__edits__edits_learn,mcp__edits__edits_check,mcp__edits__edits_preset_create,mcp__edits__edits_doctor`.
   Use them over the CLI: only `edits_make` takes `line_texts`, `preset_json`, `overlays`, `svgs`, `whoosh`/`boom`.
2. **Decide the idea before any command**: one sentence on what the viewer feels (laughs at X, aches at Y) and the
   one line or moment it hangs on. No idea, no render.
3. **Know the footage.** Every downloaded video gets a contact sheet; the log names it (`footage ID: sheet PATH`),
   Read it along with the captions/`.json` instead of running ffmpeg yourself. For a preset edit, choose the shots
   yourself too: `shots: ["ID@98.6", "ID@58.3-61", ...]` (CLI `--shot`), one per preset shot in order, each playing
   on from where it stopped the time before; never leave a character edit or a single scene to the automatic pick,
   and never hand-cut footage with ffmpeg for it. Choose the moment
   yourself and force it with `--say ID@A-B` (MCP: `say: ["ID@A-B"]`, plus `lang`) instead of trusting the automatic pick.
4. **Write the words yourself** (`line_texts` / `--line`): short, felt, in the viewer's language, none copied raw
   from the transcript unless it IS the punchline. A line on screen is where an edit becomes funny or moving.
5. **Match song, speed and colour to the feeling** (see "Funny" and "Emotional" below), then tweak the preset
   (edits-preset-tweak) rather than taking the stock one.
6. **Render one, measure it, look at its frames, judge honestly, fix, render again.** `edits_check` (CLI `edits check clip.mp4`) gives length, LUFS, cut rhythm and a contact sheet jpg to Read. To look at moments of the footage before picking `shots` by hand, `edits_frames` (CLI `edits frames ID 5:36 390+4`) gives one jpg with a row per time; never hand-roll ffmpeg frame grabs. First render is a draft. Check: does the
   first second hook? does the line land on the beat? is any face/text cut off? Only then show it, with the clip's
   absolute path. Rate good ones with `edits rate clip.mp4 +` so taste.md learns.

Taste that holds for every run lives in `~/.config/edits/style.json`: `"sfx": false` (no made whoosh/boom) and
`"hook": false` (no hook line on top) turn them off for good; `--sfx` / `--hook` bring them back once.
Presets are named for what they do and `edits_presets` tags each (fast/medium/slow, frame, zoom slide spin shake
blur rgb velocity flash); pick by the tags, not by the name of the anime it came from.

Gotchas: `trend:N` uses the chart of `lang` (`--lang tr` gives Turkey's), else the system's. `--say` makes a talking clip
(captions, `--bed`) when there is no `--music`. Whisper mishears Turkish ("mecburum" became "meşgul"): read the
clip's `.json` `text` against what you chose and fix wrong caption words before showing it.

Over-effecting reads amateur: flashes and shakes only on the real peaks; one signature effect plus one colour look.

If the user shows a reference edit they like, use `edits learn ref.mp4` (preset) or `--like ref.mp4` (music, cuts,
colour), and take only the parts they name; if they only say "like this", ask which part (pacing, colour, effects).

Check the tools once: `edits doctor` (needs ffmpeg; yt-dlp for links; whisper only for captions).

## Funny

- It is the cut, not the footage: start on the setup's shortest form, cut 0-1 s after the punchline, no explaining.
- Land the punchline with one hit (`boom`, one `zoom`/`shake`, a caption word), never a pile of effects.
- A deadpan or a silence before the line is funnier than speeding up. Do not velocity-ramp a joke.
- A second, ironic line on screen under a serious moment ("him: calm") often beats the original audio alone.

## Emotional

- Slow, simple and quiet: `--sad`, `speed` 0.8-0.9, `look` bw/faded/cold, no shake, no flash, few cuts.
- Pick a slow song and cut on its phrases, not every beat. Let one shot breathe for 2-3 s on a face or an empty place.
- Say one honest sentence (`line_texts`, 3-8 words, lowercase, no cliché like "never give up"), and let it appear late.
- End on stillness: the last shot holds, the song fades, no outro line.
- Sad is not just slow: if the footage has no feeling in it (a gameplay clip, a lecture), say so and ask for better
  footage instead of rendering it.

## Character edit (dizi): make one person look huge

The idea is to praise a character, not to tell a story. The viewer should leave thinking "he is the one".

- One character, the whole edit. Every shot is him or someone reacting to him (fear, respect, silence); cut shots
  of anyone else that do not feed that. List his shots with `shots` (the automatic pick mixes in other people).
- Build up, then land: start on reactions and the walk-in, hold his face when it matters, and put the hardest hit
  (flash, shake, zoom, the bass) on his best line or look. Do not spend the hits early.
- Low angle, slow push-in, slow-mo (`speed` 0.8) on his big moment; fast cuts for everyone else's reaction.
- Sound: a song with a real drop, cut on its beats; take one short line of his own voice (3-6 s, a threat or a
  flex) over the drop: give the preset edit `say: ["ID@A-B"]` and edits widens it to whole words, ducks the song
  under it and ends it on the hardest cut it fits before. (`voice` without `say` is still ignored by presets.)
- Colour: one grade for the whole thing (teal, or black and white for a colder, bigger look).
- Optional: a caption of one word about him (his name, "KRAL"), never a sentence that explains the scene.
- Check: can you say in one sentence why he is the strongest person in it? If not, the shots are wrong.

## What the user's own reference edits look like (measured from ~/Documents/edits, 11 clips)

Study results, not taste guesses. Match these before adding anything fancy:

- **Loud and bass-heavy.** Fast montages sit at -9 to -12 LUFS; talking-head clips at -16 to -17. A first emotional
  test came out at -16 LUFS with the sub (<120 Hz) 10-15 dB below the references and the user called the sound
  disgusting. Renders are loudness-normalised; `edits_check` measures each one, keep montages near -11 LUFS.
- **A quiet "fon müziği" bed under dialogue is wrong.** Do not use soft stock piano/ambient tracks (trend lists have
  them, e.g. "En Duygusal Fon Müziği"). Use a song with real low end, from the library or the trend chart, and
  say which one you chose.
- **Frames:** mostly 1:1 (720 or 1080) and 4:3, some 9:16. Square is the default (`~/.config/edits/style.json`).
- **Cuts:** fast montages cut every 0.5-1.5 s (a few 2-3 s holds near the start, then tighter); no cut longer than
  ~3 s except a deliberate hold. A single 50 s scene is not this style.
- **Formats seen:** (1) fast cuts of Turkish-series scenes, teal/green or black-and-white grade, music-driven;
  (2) a speaker in black and white with one big caption line at a time; (3) "THE PROBLEM / THE SOLUTION" meme cuts;
  (4) one big uppercase word at a time over a talking face. Pick one of these, do not invent a new format.
- Before rendering, compare with a reference by running `edits learn ref.mp4` or `--like ref.mp4`; the loudness and
  cut-length numbers above are what to hit.

## Why edits come out bad, and the fix

Each of these was a real complaint or a measured fault; check them before and after rendering.

- **Soft picture.** A 9:16 crop of a landscape video keeps a third of its width: from 720p it was blown up 2.7x
  before any zoom. edits now downloads 1080p (1.8x); with landscape footage prefer `aspect` 1:1 or 4:5 (the
  user's default is 1:1), and keep zoom-heavy presets for sharp sources.
- **Wrong people, wrong scenes.** The automatic pick does not know who the character is. Look at the footage
  sheets and list the shots with `shots` (in order, one per preset shot).
- **The preset fights the footage.** Anime presets carry big, decaying impact shakes and 2-3x zooms; on live-action
  dizi footage they look chaotic: use `intensity` 0.5-0.7, or a medium/slow preset by its tags.
- **No shape.** Same pace from start to end feels flat. Build up, then land: `buildup-drop` (voice line on the held
  beat before the drop with `say`), `sad-holds` for sad edits.
- **The song is wrong or starts in its intro.** With `music`, edits starts at the song's loudest stretch; check the
  log's song line, and pick a song that fits the person and the mood (not a soft piano bed).
- **Text and boom the user hates.** No captions or boom on dizi edits (style.json has them off).
- After rendering, `edits_check` the clip: a cut average far from the preset's pace, black or frozen frames on the
  sheet, or a face cut off means re-pick the shots, not re-render the same plan.

## Pick the kind of edit

Preset edits look best: a real edit's timing, speed ramps, zooms, shakes and flashes replayed over the footage.
Start there unless the user asks for something else.

```sh
edits preset list                                   # what exists
edits <link|channel|topic|file> --preset spin-slide-shake --music trend:3 --look tealorange -n 3
```

- `--music`: a song link, file or title, `trend:N` (TikTok's Nth trending song) or `spotify:N` (Spotify's daily chart,
  see the edits-trends skill), or leave it out when the preset names its own song. A reference edit's song:
  `edits song ref.mp4` (Shazam), then `--music "Artist - Title"` for the clean original.
- `--look`: colour (`edits looks`; furkancc-N and rajjcc-N are Alight colour packs). `--aspect 9:16|4:5|1:1|...` or `--size 1080x1350` for the frame.
- `--intensity 0.5` calmer, `2` wilder. `--tag @name` signs it.
- For a channel: `--videos 12 --use 4` reads every video's captions first and downloads only the best four.

Talking clips (a speaker's best moments, cut tight, captioned) are good too:

```sh
edits <link|channel> -n 3 --aspect 9:16 --captions hormozi --bed trend:7
```

- Pauses are cut out (jump cuts) and every other cut punches in; the punchline punches in harder with a boom.
- `--captions`: hormozi, mrbeast, karaoke, box, minimal, neon (JSON in presets/captions.json, change freely).
  Any Google Fonts family works as a style's `font` ("Bebas Neue", "Montserrat Black"): edits fetches it the first
  time; `edits font NAME` fetches one now.
- `--bed SONG`: music under the voice, ducked while they talk. A hook line shows on top for the first seconds.
- `--no-jumpcuts`, `--no-sfx`, `--no-hook`, `--no-captions` turn parts off.
- Screen recordings with no face keep the whole frame over a blurred copy; faces are cropped around.

Other kinds, when asked for:
- Music montage with no preset: `--music SONG` (add `--sad` for slow, black-and-white, dissolves).
- A speaker's line over the edit: `--voice`, or `--say VIDEO@FROM-TO` to choose it. These rarely beat preset edits.

## Make it hit harder

- `--fx shake,flash,rgb,glitch,invert,blur,freeze,zoom`: effects on the hardest beats (freeze = the "buz" hold).
- `--velocity`: speed ramps on longer shots; `--speed 0.8` slower, `1.3` faster (talking clips: speech speed).
- `--line "..."` (repeat) puts words on screen over music/preset edits. Read the videos' own words and pick
  short, felt sentences yourself; `--lines N` picks automatically when you cannot.
- Faces are followed through each shot automatically. `--blur-faces`, `--blur-box x,y,w,h` hide things.
- Point at things with drawn graphics: `--svg NAME[@FROM-TO][:X,Y,SIZE][#COLOUR]` (repeatable), or afterwards
  `edits draw clip.mp4 SPEC...`. Names: arrow, circle, underline, cross, check, burst, question, exclaim, heart,
  sparkle, or an .svg file. X,Y is the centre and SIZE the width as fractions; look at a frame first so the ring
  lands on the face or the detail, and keep each up for 1.5-2.5 s on the line it explains.
- The user's own pack assets (`edits library` lists them; `edits library add pack.zip` takes a pack in):
  `--overlay NAME[@FROM[-TO]][:OPACITY]` (repeatable, also `edits draw clip.mp4 --overlay ...`) lays a library clip
  over the edit: textures like "vhs texture:0.5" run screened under all of it, a transition or meme plays once at
  FROM, green screens are keyed. `--music NAME` takes a library song; talking clips take `--whoosh NAME` and
  `--boom NAME`. A meme or background's path works as the input footage.
- Meme words and the spoken line, drawn by edits (emoji in colour): `--caption "sayın barış özbay 🗿@0-3.5"` (top,
  seconds of the edit, repeatable), `--say-text` (the `--say` line at the bottom as it is said). MCP: `captions_at`,
  `say_text`.
- Chaos for "math"/sigma edits: `--math` (formulas, sine waves and the song's waveform flash in for 0.17 s on the
  cuts, never sitting on screen), `--formula '\frac{\partial u}{\partial t} = \alpha\nabla^2 u'` (your own
  formulas in LaTeX instead of the built-in ones, repeat; Greek, \frac, \sqrt, \sum, \int, ^ and _), `--flash-text "5.2:dışarısı bir simülasyon"` (a subliminal line, two frames on
  black). Library transitions over a black-and-white edit turn grey by themselves.
- Before and after, never by hand with ffmpeg: `--intro VIDEOID@FROM-TO[/PICTURE_FROM]` (that video's words, its
  picture from PICTURE_FROM; only that stretch is downloaded), `--intro-bed SONG` (quiet under it, stopped by a
  record scratch), `--intro-caption "avenox 🤡"`, `--outro edits` (the EditsMaker card with a bass hit). The .srt
  moves with the intro. MCP: `intro`, `intro_bed`, `intro_caption`, `outro`.
- Finishing: `--stabilize`, `--denoise`, `--smooth`, `--fps 60`, `--codec h265|prores|av1`, `--quality master`.
  `--denoise` on a `--say` edit runs the line through DeepFilterNet before the mix: a dizi line loses its score and
  room noise, the song stays untouched.
- Colour and sound over the whole edit: `--curves vintage` (or points `"0/0 0.4/0.3 1/1"`), `--pop red` (only red stays),
  `--bass 6` (phonk low end), `--pitch -2` (semitones, tempo kept), `--reverb` (slowed + reverb, sad edits).
- `--anim pop|slam|slide|drop|blur|type`: how words and drawings come in (`slam` on a punchline, `type` for a quote);
  one drawing its own with `~pop` at the end of its `--svg` spec.
- `--fix`: footage that looks dark, flat or dull is brought up before the look goes on.
- `--pip FILE[@FROM-TO][:X,Y,SIZE]`: another video small in a white frame (a reaction, a source, a flashback).
- `--transition circleopen|dissolve|wipeleft|slideup|pixelize|...`: music edits, every soft cut uses it.
- Shots are spread over every downloaded video; do not settle for an edit cut from one video.

## After rendering

edits picks the output folder itself, one per run: `~/Videos/edits/DAY/HH.MM <video or topic title>/` (or `$EDITS_OUT`), with a `.json` beside each clip; the log's last line names it. Never choose a folder yourself (no `--out`, no `work/`): the automatic one is where the user looks. Look at frames before calling it done: `edits_check` writes the clip's sheet. If a preset does not suit
the footage, change the preset (edits-preset-tweak skill) rather than giving up on it.

## Learned the hard way (küfür/shitpost kanalları)

- **Kanal**: `@Kanal` tek başına verilince edits onu kanalın `/videos` sayfası yapar (aramaya düşmez) ve log'a
  `videos from "KANAL ADI"` yazar; yanlış kanalsa orada görürsün. Yine de klibin `.json`'undaki `source`'a bak.
- **Küfür odaklı**: moment seçimini `~/.config/edits/taste.md` yönlendirir ("küfür punchline'dır"). Render sonrası
  `.json` içindeki `text` alanını oku. Küfür yoksa klibi at. `--lang tr` ver.
- **Düz konuşma klibi sönük kalır**: küfür anına vurgu için `--fx shake,flash,rgb,zoom --intensity 1.5`,
  renk/kontrast için `--look pop` (canlı) ya da `bleach` (sert kontrast), bass için
  `--boom "Distored Bass Drop 01"` (`edits library sfx "bass drop"` ile diğerleri). `bw/noir/dark/sepia` "sigma" havası verir, istenmiyorsa kullanma.
- **Yavaşlık**: `whisper-cli` CPU'da 12-20 dk'lık videolarda saatler sürüyor ve log'a yazmıyor. GPU sürümü kur:
  Arch'ta `ggml-vulkan` + `whisper-cpp` paketleri. Sudo yoksa (Claude Code içinde `no new privileges`):
  `pacman -Sp whisper-cpp ggml ggml-vulkan` URL'lerini indir, `bsdtar -xf` ile `~/.local/whispervk` altına aç,
  `usr/lib/ggml/*.so` dosyalarını `usr/bin/` yanına symlink'le (backend'ler exe klasöründen yükleniyor),
  `LD_LIBRARY_PATH=~/.local/whispervk/usr/lib` ile saran bir `whisper-cli` script'ini PATH'e koy. Sonuç: Radeon 680M (Vulkan cihaz 0), RTX 3050 VRAM'i başka süreçlerce dolu olabilir.
- Uzun süren işi `nohup ... > log &` ile başlat. Süreci `pkill -f` ile öldürürken kendi shell'ini vurma, PID kullan.
- Bitince log'un son satırındaki klasörde `.mp4` ve `.json`'a bak; log "rendered" yazınca bitmiş demektir.

## Never cut a spoken sentence mid-word (voice over a hand-picked moment)

Cutting the voice by eye (a guessed `-t`) clipped the last word ("çalışıyorsun" cut at 6.45 s, it ends at 6.76 s).
`say` does this itself (whole words, +0.25 s after the last one), in talking, music and preset edits. Get the times
from `edits words ID --find "a word of the line"`: each line comes out as a ready `ID@A-B`. Never mix a voice by hand.

- After rendering, `edits words out.mp4` reads what the render says: its first and last word must be the ones you chose.
- Song intros are often silent: a YouTube song already starts at its most replayed part; time the hook to the visual hit.
