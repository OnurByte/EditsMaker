---
name: edits-playful
description: "Cut an edit in one of the Kaleidoscope, comic & split presets with edits (ayna-kaleydo, comic-pop, split-ekran). The playful ones: a picture mirrored on itself, a comic book, a split screen on the beat. Use when the user asks for one of them by name, or for a medium, 9:16, comic, flash, shake, slide edit."
---

# Kaleidoscope, comic & split

The playful ones: a picture mirrored on itself, a comic book, a split screen on the beat.

- `ayna-kaleydo`: Kaleidoscope: mirrored on itself, colours turning, a slow turn and glow.
- `comic-pop`: Comic book: drawn edges, bright colour, slam zooms and white panels on the drop.
- `split-ekran`: Split screen on the beat: full shots that split into 4 and 9 tiles on the hits.

```sh
edits <link|topic|video> --preset ayna-kaleydo
edits <link> --preset ayna-kaleydo --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` set to one of the names above. Read one with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.
