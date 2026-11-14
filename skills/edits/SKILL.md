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

Other kinds, when asked for:
- Music montage with no preset: `--music SONG` (add `--sad` for slow, black-and-white, dissolves).
- Talking clips with captions from a video's funniest moments: just `edits <link>`.
- A speaker's line over the edit: `--voice`, or `--say VIDEO@FROM-TO` to choose it. These rarely beat preset edits.

## After rendering

Clips land in `edits-out/` with a `.json` beside each. Look at frames before calling it done:
`ffmpeg -i edits-out/x.mp4 -vf fps=1.5,scale=100:-2,tile=18x2 -frames:v 1 sheet.jpg`. If a preset does not suit
the footage, change the preset (edits-preset-tweak skill) rather than giving up on it.
