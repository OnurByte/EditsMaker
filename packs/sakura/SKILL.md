---
name: edits-sakura
description: "Lay the Sakura 3D scene (three.js) over an edit with edits: cherry petals drifting down on a breeze, tumbling as they fall, in 3D, looping. Use when the user wants sakura, cherry blossoms, falling petals or a 3D overlay on an anime, love or soft edit."
---

# Sakura

Cherry petals drifting down on a breeze, tumbling as they fall, in 3D. It loops every 8 seconds.

```sh
edits <link|topic|video> --overlay sakura
edits <link> --overlay sakura@2-6:0.8   # from 2 s to 6 s, at 80%
```

Its scene.js runs alone: in a browser of its own with no window, no files and nothing of the person's; only three.js
is loaded for it. A 3D scene pack of your own is one function in scene.js, `(THREE, scene, camera, opts) => (t) => {}`
(PACKS.md).
