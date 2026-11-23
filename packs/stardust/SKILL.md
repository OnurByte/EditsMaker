---
name: edits-stardust
description: "Lay the Stardust 3D scene (three.js) over an edit with edits: soft points of light drifting towards you, looping. Use when the user wants stardust, particles, floating lights or a 3D overlay on a sad, dreamy or night edit."
---

# Stardust

Soft points of light drifting towards you in 3D, drawn with three.js. It loops every 6 seconds.

```sh
edits <link|topic|video> --overlay stardust
edits <link> --overlay stardust@2-8:0.7   # from 2 s to 8 s, at 70%
```

Its scene.js runs alone: in a browser of its own with no window, no files and nothing of the person's; only three.js
is loaded for it. A 3D scene pack of your own is one function in scene.js, `(THREE, scene, camera, opts) => (t) => {}`
(PACKS.md).
