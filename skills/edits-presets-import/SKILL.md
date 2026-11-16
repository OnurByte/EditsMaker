---
name: edits-presets-import
description: Add new edit presets to `edits` from Alight Motion projects (.xml, zip, folder), CapCut drafts (draft folder or zip) or Premiere Pro presets (.prfpset), and learn a style from reference edits. Use when the user hands over preset files, project exports or example edits to copy.
---

# New presets

```sh
edits preset import pack.zip project.xml folder/ capcut-draft/ transitions.prfpset   # into ~/.config/edits/presets
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
- CapCut drafts (a draft folder, its zip, or its `draft_content.json`) become presets: each main-track clip's
  timing, speed and curve-speed ramp, zoom/slide/turn (keyframed or still), effect clips named like flash, shake,
  blur, RGB/glitch or invert, white-flash transitions and the song's name. Drafts live in
  `%LOCALAPPDATA%\CapCut\User Data\Projects\com.lveditor.draft\` (Windows) or
  `~/Movies/CapCut/User Data/Projects/com.lveditor.draft/` (macOS). A CapCut *template link* has no draft to read:
  use it in CapCut, export the video and `edits learn` it. Newer CapCut and JianYing 6+ encrypt drafts; those are
  skipped with a message, so `edits learn` the exported video instead. Filters and stickers are not read.
- Premiere Lumetri colour presets are opaque binary inside `.prfpset`: export the look from Lumetri as a `.cube`
  and use it with `--lut` instead. DaVinci Resolve `.setting` packs are not read.
- Premiere `.prfpset` / `.prfpset.xml` files become cut moves (spins, whips, slides, zooms) that music edits put on the hardest beats.
- Alight's Transform effect adds its zoom and tilt to the shot, a wipe becomes a slide in, fade a dip from or to
  black, jaws (ccjaws) a close and open in its colour, a noise layer grain, a drop shadow a vignette. Title styling
  (letter spacing, bevels, gradient plates) is counted apart: give it a caption style (`heisty` is one).
- Alight adjustment layers ("copy background" shapes with effects on them) become hits on the shots under them:
  blur pulses, brightening (exposure, contrast, glow passes), shakes, RGB splits, inversions, saturation pops.
- The import says which effects it could not carry over (`effects not carried over: wipe2 x7, ...`): those are
  the gaps; mention them instead of promising the preset looks exactly like the original.
- After importing, rename the preset for what it does (`edits preset show NAME`, change `name`, keep the old one in
  `aka`), open it and check it reads sensibly.

Reference edits (mp4s of edits the user likes):
- `edits learn edit.mp4` reverse-engineers one into a preset: cuts (adaptive threshold, flashes kept apart), flashes,
  and each shot's zoom, slide, turn and shake from the frames' motion. Then `--preset <its name>` replays it over
  other footage, retimed to the song. Speed ramps and effects are not read; add them by changing the JSON.
- `edits study <folder>` measures their frame, colour and loudness and makes them the defaults.
