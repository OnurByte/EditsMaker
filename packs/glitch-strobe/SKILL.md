---
name: edits-glitch-strobe
description: "Cut an edit in the glitch-strobe style with edits (genre: glitch - 0.35 s cuts, colour split on all, inverted frame every 4th cut, 20 Hz jitter, strobing flashes, real flicker every 4th shot). Use when the user asks for glitch-strobe or a fast, 9:16, zoom, shake, rgb, velocity, strobe, flash edit."
---

# glitch-strobe

genre: glitch - 0.35 s cuts, colour split on all, inverted frame every 4th cut, 20 Hz jitter, strobing flashes, real flicker every 4th shot

```sh
edits <link|topic|video> --preset glitch-strobe
edits <link> --preset glitch-strobe --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` "glitch-strobe". Read it with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.

14.0 s, 40 shots, 1080x1920.
