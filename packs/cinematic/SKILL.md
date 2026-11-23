---
name: edits-cinematic
description: "Cut an edit in one of the Slow & cinematic presets with edits (sinema-bant, smooth-cinematic, fade-anilar, yavas-duygusal, sad-holds, loop-push). Long shots, slow motion and fades to black. For the sad song, the memory, the trailer. Use when the user asks for one of them by name, or for a slow, 9:16, 1:1, blur, cinema, fade edit."
---

# Slow & cinematic

Long shots, slow motion and fades to black. For the sad song, the memory, the trailer.

- `sinema-bant`: A trailer on a phone: cinema bars, long push-ins, a warm leak drifting through.
- `smooth-cinematic`: Smooth slow motion, long push-ins, teal and orange, cinema bars.
- `fade-anilar`: Memories: slow motion, warm light, every shot fading in from black.
- `yavas-duygusal`: Slow long shots with a soft drift, fading from and to dark.
- `sad-holds`: Seven long slow shots that breathe, three dips to black, black and white.
- `loop-push`: One slow push-in across every cut that returns to the start, so it loops.

```sh
edits <link|topic|video> --preset sinema-bant
edits <link> --preset sinema-bant --music trend:1     # on a trending song
```

MCP: `edits_make` with `preset` set to one of the names above. Read one with `edits_preset_get`; make it calmer or wilder with the edits-preset-tweak skill.
