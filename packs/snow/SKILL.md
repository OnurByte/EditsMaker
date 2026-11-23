---
name: edits-snow
description: "Lay the Snow 3D scene (three.js) over an edit with edits: soft snow of three sizes falling and swaying in 3D, after three.js's points-sprites example, looping. Use when the user wants snow, winter, falling flakes or a 3D overlay on a sad, cold or night edit."
---

# Snow

Soft snow of three sizes falling and swaying in 3D, after three.js's points-sprites example. It loops every 8 seconds.

```sh
edits <link|topic|video> --overlay snow
edits <link> --overlay snow@2-6:0.8   # from 2 s to 6 s, at 80%
```

Its scene.js runs alone: in a browser of its own with no window, no files and nothing of the person's; only three.js
is loaded for it. A 3D scene pack of your own is one function in scene.js, `(THREE, scene, camera, opts) => (t) => {}`
(PACKS.md).
