---
name: edits-series
description: "Cut an edit in one of the Series scenes presets with edits (dizi-diyalog, dizi-teal, kahvehane, sokak-montaj, yuz-sirasi, kitap-slayt, problem-cozum). Shapes learned from TV series: dialogue, a table conversation, faces in turn. Made for scenes, not montages. Use when the user asks for one of them by name, or for a slow, medium, 1:1, wide, shake, slide edit."
---

# Series scenes

Shapes learned from TV series: dialogue, a table conversation, faces in turn. Made for scenes, not montages.

- `dizi-diyalog`: Shot, reverse shot: a dialogue cut with moving shots and one flash.
- `dizi-teal`: Static held shots on the beat, under a teal grade.
- `kahvehane`: Fifty-two quick cuts round a table conversation.
- `sokak-montaj`: A street montage of faces in quick moving cuts.
- `yuz-sirasi`: Faces one after another, then the whole group.
- `kitap-slayt`: Pages and covers in steady moving shots: a book or product promo.
- `problem-cozum`: The problem, the solution: held setups, then the reveal.

```sh
edits <link|topic|video> --preset dizi-diyalog
edits <link> --preset dizi-diyalog --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` set to one of the names above. Read one with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.
