---
name: edits-blur-reveal
description: "Cut an edit in one of the Blur & reveal presets with edits (blur-hit-anime, blur-shake-rush, blur-shake-soft, blur-spin-montagem, hard-zoom-blur-flash, slide-zoom-anime, slow-reveal-anime, rgb-blur-reveal). Motion blur, speed ramps and slow reveals: the shot hides, then hits. For the moment a face turns round. Use when the user asks for one of them by name, or for a slow, medium, fast, 9:16, 4:5, blur edit."
---

# Blur & reveal

Motion blur, speed ramps and slow reveals: the shot hides, then hits. For the moment a face turns round.

- `blur-hit-anime`: Slow 4:5 shots that blur into a speed ramp on the hit.
- `blur-shake-rush`: Fast vertical zooms, blurred and shaking.
- `blur-shake-soft`: Softer blur and shake, with zooms and slides.
- `blur-spin-montagem`: Slow blurred spins and speed ramps, cut to the song.
- `hard-zoom-blur-flash`: Hard zooms into a blur and a white flash.
- `slide-zoom-anime`: 4:5 slides and zooms with a soft blur.
- `slow-reveal-anime`: A slow zoom and a speed ramp that reveal the face.
- `rgb-blur-reveal`: Blur, colour split and a speed ramp for a reveal.

```sh
edits <link|topic|video> --preset blur-hit-anime
edits <link> --preset blur-hit-anime --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` set to one of the names above. Read one with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.
