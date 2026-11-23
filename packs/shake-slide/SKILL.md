---
name: edits-shake-slide
description: "Cut an edit in one of the Shake & slide presets with edits (shake-hold-anime, shake-only-square, shake-slide-9x16, shake-slide-anime, shake-zoom-long, shake-zoom-rush, soft-shake-720, soft-shake-9x16, soft-zoom-shake, zoom-shake-9x16, slow-slide-shake, slide-turn-4x5, slide-transition-pack). Handheld energy: shakes, slides and soft zooms in every frame shape. The everyday anime and fan edit. Use when the user asks for one of them by name, or for a slow, medium, fast, 9:16, 4:5, 1:1 edit."
---

# Shake & slide

Handheld energy: shakes, slides and soft zooms in every frame shape. The everyday anime and fan edit.

- `shake-hold-anime`: Held shots with a steady shake. Lets the face do the work.
- `shake-only-square`: Square frame, nothing but shake. Raw and simple.
- `shake-slide-9x16`: Vertical slides into a shake on each cut.
- `shake-slide-anime`: Slides and zooms with a shake, made for anime scenes.
- `shake-zoom-long`: Longer shots that zoom slowly under a shake.
- `shake-zoom-rush`: Fast slides and zooms, shaking all the way.
- `soft-shake-720`: A softer shake with gentle zooms and slides.
- `soft-shake-9x16`: Slow and soft: drifting slides under a light shake.
- `soft-zoom-shake`: Soft zoom-ins with a calm shake. Easy on the eyes.
- `zoom-shake-9x16`: Zoom in, shake, cut. Vertical and steady.
- `slow-slide-shake`: Slow slides with a shake on each landing.
- `slide-turn-4x5`: 4:5 slides that turn as they land, with a shake.
- `slide-transition-pack`: Square cuts joined by sliding transitions.

```sh
edits <link|topic|video> --preset shake-hold-anime
edits <link> --preset shake-hold-anime --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` set to one of the names above. Read one with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.
