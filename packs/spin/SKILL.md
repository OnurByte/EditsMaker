---
name: edits-spin
description: "Cut an edit in one of the Spin presets with edits (half-spin-square, shake-spin-square, spin-shake-anime, spin-slide-shake, spin-zoom-loop, quarter-spin-rush, slow-turn-five). Turns and spins between the shots: quarter turns, half turns, full loops. Movement that pulls the eye round. Use when the user asks for one of them by name, or for a slow, medium, fast, 9:16, 4:5, 1:1 edit."
---

# Spin

Turns and spins between the shots: quarter turns, half turns, full loops. Movement that pulls the eye round.

- `half-spin-square`: Square frame, a half turn sliding into each shot.
- `shake-spin-square`: Fast square spins with zooms and a shake.
- `spin-shake-anime`: Vertical spins into a shake, quick and loud.
- `spin-slide-shake`: Spins, slides and a blur between them.
- `spin-zoom-loop`: Fast zooms and slides that loop round on the beat.
- `quarter-spin-rush`: Quarter turns and speed ramps, rushing through 4:5 shots.
- `slow-turn-five`: Slow 4:5 zooms with a gentle turn.

```sh
edits <link|topic|video> --preset half-spin-square
edits <link> --preset half-spin-square --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` set to one of the names above. Read one with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.
