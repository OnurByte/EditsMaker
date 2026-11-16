---
name: edits-preset-tweak
description: Read an `edits` preset as JSON and change it to suit the footage, song or mood (speed, zooms, shakes, cut timing, colour), then render with it or save it. Use when a preset edit looks too wild, too calm, too long, wrong for the song, or the user wants a variation.
---

# Changing a preset for the moment

Presets are plain JSON. Nothing is fixed: read one, change what this edit needs, use it.

```sh
edits preset show spin-slide-shake > p.json      # the whole preset
# edit p.json
edits <input> --preset p.json            # use it for this edit only
edits preset save p.json                 # or keep it (same name replaces the built-in one)
```

Over MCP: `edits_preset_get` → change → `edits_make` with `preset_json`, or `edits_preset_save`.

## What is in it

- `w`, `h`: frame. `total`: seconds. `song`: `{title, from}` (searched on YouTube by title).
- `look`: colour name (`edits looks`), `intensity`: 1 as made, `fit_beats`: with another song, cuts move onto its beats.
- `shots`: one per cut, `start`/`end` in seconds, `speed` (footage speed, 2-4 = velocity), then curves.
  A curve is a list of `[time 0..1 of the shot, value, easing]`; easing is `linear`, `out K`, `in K`, `inout K`.
  - `zoom` (1 = framed), `x` / `y` (fraction of the frame, + = right / down), `rot` (degrees),
    `shake` (fraction of frame width, `shake_hz` its speed), `blur` (0..1, on above 0.25), `exposure` (0..1)
  - `saturation` (1 normal, 0 black and white), `rgb` (colour split), `invert`, `motion_blur`.
  - `hue` curve in degrees (`[[0,180,"linear"]]` shifts every colour, `[[0,0],[1,360]]` cycles the rainbow across the shot).
  - Look per shot: `contrast` (1 normal), `kelvin` (3000 orange, 12000 blue, 0 off), `vignette`, `grain`, `glow`, `sharpen` (0..1),
    `pixelate` (block px), `lens` (-1..1, above 0 fisheye bulge), `trails` (0..1 ghost echo), `flip` (mirror).
- `flashes`: coloured layers over the picture, `{start, end, color "rrggbb", opacity curve, blink}`.

## Common changes

- Too wild for this footage: `intensity` 0.5, or lower the `zoom`/`shake` values on the loud shots.
- Text-heavy or static footage: shorter shots (split long ones), higher `speed`, add a zoom curve.
- Sad song: `speed` toward 0.8, remove `shake`, `look` "bw" or "faded", fewer flashes.
- Longer song section: add shots at the end, keep `total` equal to the last `end`.
Render one, look at the frames, adjust, repeat.
