---
name: edits-reverse
description: Rebuild an edit someone made (a TikTok, Reel or Short) as an `edits` preset and render it again, checked against the original cut by cut. Use to reverse, recreate, remake, replicate or copy an edit, "bunun aynısını yap", or to put new footage into an edit's exact shape.
---

# Reverse an edit

The result is a preset whose render matches the original's cuts, moves, flashes, colour and length, and that
replays over any footage. Reverse edits the person has the right to use. Rebuild it as it is; change it only when
they ask. Every step is an `edits` command (see the edits skill); nothing is scripted by hand.

## The original decides

1. `edits check original.mp4`: length, frame size, loudness, cut count, average and longest shot, a contact sheet.
   These are the numbers the render must hit. Render at its size (`--size WxH` or `--aspect`).
2. `edits learn original.mp4`: every cut (flashes kept apart), each flash, each shot's zoom, slide, turn and shake,
   as a preset named after the file, and the song it uses (Shazam). `edits preset show NAME` and read it whole.
   Render with `--music "Artist - Title"` from that line: the clean original, not the edit's own mix.
3. `edits frames original.mp4 T+1 ...` at each cut, flash and big move, a second either side: what `learn` does
   not read is seen here. Speed ramps (a shot that rushes then slows), blur dips, colour splits, inversions,
   captions and their style, stickers, the grade.
4. Put what you saw into the preset (`edits preset set NAME shots.3.speed=0.4 ...`; the keys are in
   edits-preset-tweak, genre values in edits-tricks). The grade is a `--look` or `--lut`; captions a caption style.
   A pack the original came from (Alight, CapCut) is better imported than guessed: edits-presets-import.

## Render and compare once

1. `edits <footage> --preset NAME --music <the original's song>`. Same footage when the person wants a copy, theirs when they want the shape.
   `--like original.mp4` is the quick version: music, cuts, flashes, colour and shape without hand fixes.
2. `edits check` the render and set its numbers beside the original's: length, cut count, average shot, loudness.
3. `edits frames` both at the same seconds where they differ. Fix what you can see: a missing cut or flash, a
   move the wrong way, a wrong speed or colour. Render and check once more, then stop.

Work alone: one agent holding the whole edit beats several each re-watching it. Tell the person what is rebuilt,
what is approximate (a substituted font, an effect `edits` has no move for) and the last check's numbers beside
the original's. An effect no preset key can make is a gap in `edits`: name it, do not script around it.
