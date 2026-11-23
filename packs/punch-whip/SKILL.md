---
name: edits-punch-whip
description: "Cut an edit in one of the Punch & whip presets with edits (akis-whip, beat-bounce, heisty-whip-shake, punch-zoom-60s, zoom-slide-wide). Hard punch-ins and whip slides that land on the beat. For edits that hit from the first second. Use when the user asks for one of them by name, or for a medium, fast, 9:16, 4:5, 1:1, wide edit."
---

# Punch & whip

Hard punch-ins and whip slides that land on the beat. For edits that hit from the first second.

- `akis-whip`: Whip slides from both sides, a punch on every fourth cut.
- `beat-bounce`: A clean 10% punch-in on every cut. No grade, no flashes: just the bounce.
- `heisty-whip-shake`: A long held open, then fast square cuts: motion-blurred whips, punch-zooms and tilts, shakes, dips to black.
- `punch-zoom-60s`: Eighty-one quick shots, each punching in to 1.5x and easing out. A full minute of momentum.
- `zoom-slide-wide`: Wide frame, fast cuts: zooms and slides with a shake on the hits.

```sh
edits <link|topic|video> --preset akis-whip
edits <link> --preset akis-whip --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` set to one of the names above. Read one with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.

The timing of punch-zoom-60s comes from github.com/Davidb-2107/capcut-cli-david (MIT).
