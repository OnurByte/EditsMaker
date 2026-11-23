---
name: edits-character
description: "Cut an edit in one of the Character presets with edits (karakter-buyuk, buildup-drop, sert-bw). A character's edit: the build, the held face, the drop of punch cuts, the still end. Use when the user asks for one of them by name, or for a medium, 9:16, 1:1, flash, rgb, shake edit."
---

# Character

A character's edit: the build, the held face, the drop of punch cuts, the still end.

- `karakter-buyuk`: Slow build, held face, a drop of punch cuts, a still end.
- `buildup-drop`: Slow push-ins, a held beat, a flash on the drop, then twenty-four punch cuts.
- `sert-bw`: Build-up push-ins, hard punch cuts, black and white.

```sh
edits <link|topic|video> --preset karakter-buyuk
edits <link> --preset karakter-buyuk --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` set to one of the names above. Read one with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.
