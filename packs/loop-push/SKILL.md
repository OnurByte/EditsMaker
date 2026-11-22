---
name: edits-loop-push
description: "Cut an edit in the loop-push style with edits (trick: replay loop - a single slow push-in across every cut that returns to the opening framing on the last shot, so the end runs back into the start). Use when the user asks for loop-push or a slow, 9:16, zoom edit."
---

# loop-push

trick: replay loop - a single slow push-in across every cut that returns to the opening framing on the last shot, so the end runs back into the start

```sh
edits <link|topic|video> --preset loop-push
edits <link> --preset loop-push --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` "loop-push". Read it with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.

15.4 s, 14 shots, 1080x1920.
