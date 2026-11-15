---
name: edits
description: Make short viral edits (TikTok, Reels, Shorts) from a YouTube link, a channel, a topic or a video file with the `edits` command. Use when asked to make an edit, a sad edit, a velocity/anime-style edit, or clips from a video or channel.
---

# Making edits with `edits`

Check the tools once: `edits doctor` (needs ffmpeg; yt-dlp for links; whisper only for captions).

## Pick the kind of edit

Preset edits look best: a real edit's timing, speed ramps, zooms, shakes and flashes replayed over the footage.
Start there unless the user asks for something else.

```sh
edits preset list                                   # what exists
edits <link|channel|topic|file> --preset yuraprst --music trend:3 --look tealorange -n 3
```

- `--music`: a song link or file, `trend:N` (TikTok's Nth trending song, see the edits-trends skill), or leave it
  out when the preset names its own song.
- `--look`: colour (`edits looks`). `--aspect 9:16|4:5|1:1|...` or `--size 1080x1350` for the frame.
- `--intensity 0.5` calmer, `2` wilder. `--tag @name` signs it.
- For a channel: `--videos 12 --use 4` reads every video's captions first and downloads only the best four.

Talking clips (a speaker's best moments, cut tight, captioned) are good too:

```sh
edits <link|channel> -n 3 --aspect 9:16 --captions hormozi --bed trend:7
```

- Pauses are cut out (jump cuts) and every other cut punches in; the punchline punches in harder with a boom.
- `--captions`: hormozi, mrbeast, karaoke, box, minimal, neon (JSON in presets/captions.json, change freely).
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
- Finishing: `--stabilize`, `--denoise`, `--smooth`, `--fps 60`, `--codec h265|prores|av1`, `--quality master`.
- Shots are spread over every downloaded video; do not settle for an edit cut from one video.

## After rendering

Clips land in `edits-out/` with a `.json` beside each. Look at frames before calling it done:
`ffmpeg -i edits-out/x.mp4 -vf fps=1.5,scale=100:-2,tile=18x2 -frames:v 1 sheet.jpg`. If a preset does not suit
the footage, change the preset (edits-preset-tweak skill) rather than giving up on it.
