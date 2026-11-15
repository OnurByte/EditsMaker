---
name: edits-tricks
description: Editing tricks from short-form editing guides (hook, beat sync, speed ramps, impact zoom, colour split, flashes, loops, genre grades) turned into `edits` preset values. Use when an edit feels flat, the hook is weak, cuts miss the beat, or the user asks for a genre (phonk, velocity, twixtor/smooth, glitch, retro, loop) or "make it hit harder".
---

# Tricks that make an edit hit

Every trick below is a preset change (see edits-preset-tweak for the JSON). Times in a curve are 0..1 of the shot.
The numbers come from creator and vendor guides, not a standard: treat them as a starting point, then render and look.

## Genre presets that already carry them

| preset | what it is |
|---|---|
| `phonk-drift` | cut on every beat, 6% kick zoom, colour split on snares, white flash and shakes from the drop, `cold` |
| `velocity-ramp` | each shot rushes in, slows to 0.35x at its peak, rushes out; zoom follows speed |
| `smooth-cinematic` | twixtor-style 0.5x, long push-ins, cuts under a blur dip, `tealorange` |
| `glitch-strobe` | 0.35 s cuts, colour split, inverted frame every 4th cut, 20 Hz jitter, strobe flashes, `noir` |
| `retro-vhs` | handheld wobble, colour split, faded `vintage`, warm light leaks |
| `loop-push` | one push-in across all cuts that returns to the opening framing: the end replays into the start |
| `beat-bounce` | clean 10% punch on every cut, no grade: dance, sport, colourful footage |

```sh
edits <input> --music SONG --preset phonk-drift
edits <input> --music SONG --preset velocity-ramp --intensity 0.7
```

## The hook (first second)

Viewers decide in about 1-3 s. Open on the most intense shot, not the calmest, and interrupt the pattern:
- first shot: `zoom [[0,1.35,"linear"],[0.35,1,"out 5"]]`, `shake [[0,0.12,"linear"],[0.4,0,"out 4"]]`, `shake_hz` 12, `motion_blur`.
- a white flash on frame one: `{start:0, end:0.1, color:"ffffff", opacity:[[0,1,"linear"],[1,0,"out 3"]]}`.
- no fade-in from black on a hype edit (the fade is for sad ones). No slow intro, no logo.

## Beat sync

- Cuts land on beats: keep `fit_beats: true` and pick shot lengths near the song's beat (128 BPM = 0.47 s per beat).
  Hype: a cut every beat or two. Sad: every 4-8 beats (phrases), see `--sad` and `limon-sad-retro`.
- Once cuts are on beats, change what happens inside shots (speed, zoom), not where they start and end.
- Kick → impact zoom: `zoom [[0,1.06,"linear"],[0.5,1,"out 6"]]` (4-6%, snapping back). After the drop go to 1.10-1.15.
- Snare → colour split: `rgb: true` on every 2nd shot, not every shot (or it stops reading as a hit).
- Drop → one 1-3 frame white flash (0.04-0.12 s) at the drop, plus `shake` for the next shots. One flash per drop, not per beat.

## Speed ramps (velocity)

- `ramp [[0,3,"linear"],[0.45,0.35,"out 3"],[1,3,"in 3"]]`: rushing in, slowest at the peak of the action, rushing out.
  Put the slow point on the visual peak (the punch landing, the jump's top, the face turning), which is mid-shot here.
- `motion_blur: true` on fast shots; a whoosh in the library (`edits library sfx whoosh`) sells each ramp.
- Slow motion looks smooth only from smooth footage: 60 fps sources, or keep `speed` ≥ 0.5.

## Smooth / cinematic

- Long shots (2-3 s), `speed` 0.5, slow push-in `zoom 1 → 1.08 "inout 2"` with a small `x` drift alternating side.
- Hide cuts in a blur dip: `blur [[0,0.6],[0.15,0,"out 2"],[0.85,0],[1,0.6,"in 2"]]`.
- One grade over the whole edit (`look`), never mixed. Teal/orange or `faded` for cinematic, `bw` for sad.

## Glitch and phonk grade

- Grade: desaturate 15-20% (`saturation` 0.8), high contrast, cold shadows: `look` `cold` or `noir`.
- Glitch: `rgb` + `invert` on single short shots, `shake_hz` 16-20 with a tiny `shake` (0.02-0.05).
- Use glitch on the punchline or climax only in non-glitch edits: one RGB split, one shake. More becomes noise.

## Retro

- `look` `vintage` or `sepia`, `saturation` 0.85, `rgb`, a constant `shake` 0.006 at 3 Hz (handheld), `speed` 0.9.
- Warm light leak instead of white flashes: `color "f2c48d"`, opacity 0.35, 0.25 s, across the cut.

## Loops and the end

- End where it began: same framing (`loop-push`), or the last shot cut on a beat into the first shot's footage,
  so the replay is seamless. Replays count as watch time.
- Don't end on a fade to black in hype edits; end on the last hit. Sad edits may fade out (`exposure` to -1).

## Talking and meme edits

- One quick zoom-in on a key word about every 5 s; captions with the key word bigger or coloured (`--captions`).
- Meme sound on the punchline only (vine boom, bruh): see the shitpost rules; one per punchline, varied.
- Music under the voice at 15-25%, voice in front (`--bed`).

## Checking

`edits check clip.mp4`: the contact sheet shows whether the hook frame is strong, cuts sit on beats and
flashes are rare enough. Too busy: `--intensity 0.6`. Too calm: `--intensity 1.4` or a genre preset above.
