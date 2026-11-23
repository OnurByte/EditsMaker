---
name: edits-velocity
description: "Cut an edit in one of the Velocity & colour split presets with edits (flash-velocity-square, rgb-zoom-square, velocity-ramp, phonk-drift). Speed that rushes in and slows at the peak, colour split on the snares. Phonk, drift and car edits. Use when the user asks for one of them by name, or for a slow, medium, fast, 9:16, 1:1, blur edit."
---

# Velocity & colour split

Speed that rushes in and slows at the peak, colour split on the snares. Phonk, drift and car edits.

- `flash-velocity-square`: Square speed ramps with colour split and white flashes.
- `rgb-zoom-square`: Square zooms with colour split, blur and speed.
- `velocity-ramp`: Every shot rushes in, slows to a third at its peak, rushes out.
- `phonk-drift`: A cut on every beat, a kick zoom snapping back, colour split on the snares, a cold grade.

```sh
edits <link|topic|video> --preset flash-velocity-square
edits <link> --preset flash-velocity-square --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` set to one of the names above. Read one with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.
