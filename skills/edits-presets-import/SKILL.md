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

- Alight Motion projects become edit presets: shot timing, speed, zooms, slides, turns, shakes, blurs, flashes,
  the song's name. Footage and music are never copied.
- Premiere `.prfpset` files become cut moves (spins, whips, slides, zooms) that music edits put on the hardest beats.
- After importing, open one with `edits preset show NAME` and check it reads sensibly.

Reference edits (mp4s of edits the user likes): `edits study <folder>` measures their frame, colour and loudness
and makes them the defaults.
