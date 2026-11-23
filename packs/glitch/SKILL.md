---
name: edits-glitch
description: "Cut an edit in one of the Glitch & strobe presets with edits (glitch-strobe, glitch-transition, strobe-rave). Short cuts, flicker, colour split and flashes. Hardstyle, techno, the loud ones. Use when the user asks for one of them by name, or for a slow, fast, 9:16, 1:1, flash, rgb edit."
---

# Glitch & strobe

Short cuts, flicker, colour split and flashes. Hardstyle, techno, the loud ones.

- `glitch-strobe`: Very short cuts, colour split on all of them, a flicker every fourth shot.
- `glitch-transition`: A slow square edit joined by glitch transitions.
- `strobe-rave`: Club strobe: a flicker on every other shot, zoom pulses, flashes on the hardest beats.

```sh
edits <link|topic|video> --preset glitch-strobe
edits <link> --preset glitch-strobe --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` set to one of the names above. Read one with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.
