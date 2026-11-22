---
name: edits-velocity-ramp
description: "Cut an edit in the velocity-ramp style with edits (genre: velocity - every shot rushes in, slows to 0.35x at its peak and rushes out, zoom follows the speed). Use when the user asks for velocity-ramp or a slow, 9:16, zoom, velocity, flash edit."
---

# velocity-ramp

genre: velocity - every shot rushes in, slows to 0.35x at its peak and rushes out, zoom follows the speed

```sh
edits <link|topic|video> --preset velocity-ramp
edits <link> --preset velocity-ramp --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` "velocity-ramp". Read it with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.

16.8 s, 14 shots, 1080x1920.
