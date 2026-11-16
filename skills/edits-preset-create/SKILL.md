---
name: edits-preset-create
description: Make a brand-new `edits` preset from a video, borrowing moves from existing presets for inspiration. Use when the user wants a new preset, a style of their own, or an edit shaped to their footage rather than a copy of an existing one.
---

# New preset from a video

Not a copy (that is `edits learn`) and not a tweak (that is edits-preset-tweak): the video sets the rhythm, other presets lend the moves.

```sh
edits preset create footage.mp4 --name mine                    # inspiration: the 3 presets whose shots run closest
edits preset create footage.mp4 --name mine --like spin-slide-shake,flash-velocity-square --length 12
edits --preset mine https://www.youtube.com/@channel           # replay it
```

MCP: `edits_preset_create` (`video`, `name`, `like`, `length`). A good turn:

1. `edits_presets` to see what exists; pick 2-3 whose feel fits the brief (calm, violent, velocity, glitchy) as `like`.
2. `edits_preset_create` on the footage.
3. `edits_preset_get` and read it. Too wild or too calm: change `intensity`, drop shots' `shake`/`rot`, set a `look`, then `edits_preset_save`.
4. `edits_make` with `preset`, then `edits_check` on the render.

What it does: shots are the video's own cuts, and any shot over 2 s is cut again where the picture changes most.
The hardest-moving shots get the strongest moves of the `like` presets (zoom, slide, turn, shake, blur), the quietest the
gentlest; the hardest cuts get a flash in the first preset's style, as often as those presets use them. Frame size is the
video's; look and intensity come from the first `like` preset; the song is dropped, so `--music` decides it.

Limits: only the first `--length` seconds (default 18) are read; the moves are borrowed, never invented, so a pool of
plain presets gives plain results: name wilder ones in `like`.
