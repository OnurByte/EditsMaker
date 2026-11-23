---
name: edits-tape
description: "Cut an edit in one of the Tape presets with edits (retro-vhs, vhs-tarama, vhs-soft-zoom-60s). An old tape: scan lines, grain, faded colour and warm leaks. Nostalgia, old songs, the 2000s. Use when the user asks for one of them by name, or for a slow, 9:16, flash, retro, rgb, shake edit."
---

# Tape

An old tape: scan lines, grain, faded colour and warm leaks. Nostalgia, old songs, the 2000s.

- `retro-vhs`: Slow handheld wobble, colour split, faded colour, warm leaks, scan lines and grain.
- `vhs-tarama`: Scan lines, grain and a slow wobble over faded vintage colour.
- `vhs-soft-zoom-60s`: Forty-nine gentle zoom-outs under a VHS look. A minute long.

```sh
edits <link|topic|video> --preset retro-vhs
edits <link> --preset retro-vhs --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` set to one of the names above. Read one with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.

The timing of vhs-soft-zoom-60s comes from github.com/Davidb-2107/capcut-cli-david (MIT).
