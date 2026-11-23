---
name: edits-embers
description: "Lay the Embers 3D scene (three.js) over an edit with edits: sparks rising from a fire below, flickering orange to red as they climb, in 3D, looping. Use when the user wants embers, sparks, fire or a 3D overlay on a hype, phonk or villain edit."
---

# Embers

Sparks rising from a fire below, flickering orange to red as they climb, in 3D. It loops every 5 seconds.

```sh
edits <link|topic|video> --overlay embers
edits <link> --overlay embers@2-6:0.8   # from 2 s to 6 s, at 80%
```

Its scene.js runs alone: in a browser of its own with no window, no files and nothing of the person's; only three.js
is loaded for it. A 3D scene pack of your own is one function in scene.js, `(THREE, scene, camera, opts) => (t) => {}`
(PACKS.md).
