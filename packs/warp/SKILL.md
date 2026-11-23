---
name: edits-warp
description: "Lay the Warp 3D scene (three.js) over an edit with edits: stars streaking past at light speed, out from the middle of the frame, in 3D, looping. Use when the user wants warp, hyperspace, light speed, speed lines or a 3D overlay on the drop of a hype edit."
---

# Warp

Stars streaking past at light speed, out from the middle of the frame, in 3D. It loops every 4 seconds.

```sh
edits <link|topic|video> --overlay warp
edits <link> --overlay warp@2-6:0.8   # from 2 s to 6 s, at 80%
```

Its scene.js runs alone: in a browser of its own with no window, no files and nothing of the person's; only three.js
is loaded for it. A 3D scene pack of your own is one function in scene.js, `(THREE, scene, camera, opts) => (t) => {}`
(PACKS.md).
