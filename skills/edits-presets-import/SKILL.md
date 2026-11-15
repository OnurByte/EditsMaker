---
name: edits-presets-import
description: Add new edit presets to `edits` from Alight Motion projects (.xml, zip, folder) or Premiere Pro presets (.prfpset), and learn a style from reference edits. Use when the user hands over preset files, project exports or example edits to copy.
---

# New presets

```sh
edits preset import pack.zip project.xml folder/ transitions.prfpset   # into ~/.config/edits/presets
edits preset import ... --into presets/        # into the repo, to build them into the binary
edits preset list
```

- A whole editing pack (sounds, music, overlays, transitions, green screens, memes, images, fonts, presets):
  `edits library add pack.zip` copies the media to ~/.config/edits/library, installs the fonts (name one in a
  caption style's `font`) and imports the presets inside. `edits library [KIND] [WORDS]` lists it.
- Alight Motion projects become edit presets: shot timing, speed, zooms, slides, turns, shakes, blurs, flashes,
  the song's name. Footage and music are never copied.
- Alight colour packs ("cc", adjustment layers with no footage of their own) become looks in grades.json, one per
  look the pack shows: `--look rajjcc-4`. Each is its layer stack as JSON (effects, opacity, blend mode) baked into
  a LUT; `edits cube NAME` writes it as a .cube for Premiere, Resolve or CapCut. Glows and streaks are left out.
- Text packs: their styles are not imported automatically; read the text layers (font, colour, glow, letter
  spacing, how they come in) and add a caption style to captions.json (`glow`, `spacing`, `reveal`).
- Premiere `.prfpset` / `.prfpset.xml` files become cut moves (spins, whips, slides, zooms) that music edits put on the hardest beats.
- After importing, open one with `edits preset show NAME` and check it reads sensibly.

Reference edits (mp4s of edits the user likes):
- `edits learn edit.mp4` reverse-engineers one into a preset: cuts (adaptive threshold, flashes kept apart), flashes,
  and each shot's zoom, slide, turn and shake from the frames' motion. Then `--preset <its name>` replays it over
  other footage, retimed to the song. Speed ramps and effects are not read; add them by changing the JSON.
- `edits study <folder>` measures their frame, colour and loudness and makes them the defaults.
