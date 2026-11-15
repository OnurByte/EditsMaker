<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/hero-dark.svg">
  <img src="assets/readme/hero-light.svg" width="100%" alt="EditsMaker. Link in. Edits out.">
</picture>

<p align="center">
  <a href="https://github.com/OnurByte/EditsMaker/releases/latest"><b>Download</b></a> ⁜ Linux, macOS and Windows
</p>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/badges-dark.svg">
  <img src="assets/readme/badges-light.svg" width="100%" alt="MIT, Rust, Linux macOS Windows, MCP server, 5 agent skills, 30 presets, 36 cut moves, 16 looks">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/agents-dark.svg">
  <img src="assets/readme/agents-light.svg" width="100%" alt="Your AI agent edits for you: EditsMaker is an MCP server for Claude Code, Codex, Cursor, Gemini CLI and any MCP client">
</picture>

## Install

### <picture><source media="(prefers-color-scheme: dark)" srcset="assets/readme/os-apple-dark.svg"><img src="assets/readme/os-apple-light.svg" height="20" alt=""></picture> <picture><source media="(prefers-color-scheme: dark)" srcset="assets/readme/os-linux-dark.svg"><img src="assets/readme/os-linux-light.svg" height="20" alt=""></picture> macOS and Linux

```sh
curl -fsSL https://raw.githubusercontent.com/OnurByte/EditsMaker/main/install.sh | sh
```

### <picture><source media="(prefers-color-scheme: dark)" srcset="assets/readme/os-windows-dark.svg"><img src="assets/readme/os-windows-light.svg" height="20" alt=""></picture> Windows

```powershell
irm https://raw.githubusercontent.com/OnurByte/EditsMaker/main/install.ps1 | iex
```

Then check what is missing. You also need [ffmpeg](https://ffmpeg.org) and [yt-dlp](https://github.com/yt-dlp/yt-dlp).

```sh
edits doctor
```

### Or ask your agent

```text
Install EditsMaker from github.com/OnurByte/EditsMaker, run "edits doctor", add it as an MCP server with "edits mcp" and install its skills with "edits skills install".
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/steps-dark.svg">
  <img src="assets/readme/steps-light.svg" width="100%" alt="Link or topic, pick a preset, pick a song, your edit">
</picture>

```sh
edits https://www.youtube.com/@channel --preset yuraprst --music trend:3 --look tealorange
```

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/features-dark.svg">
  <img src="assets/readme/features-light.svg" width="100%" alt="Presets, trends, looks, agents, reads captions first, every frame size">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/fx-dark.svg">
  <img src="assets/readme/fx-light.svg" width="100%" alt="Everything an edit needs: speed ramps, freeze, face tracking, hits, captions, looks, LUTs, music ducking, stabilize, ProRes">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/presets-dark.svg">
  <img src="assets/readme/presets-light.svg" width="100%" alt="Real edits as data: 30 Alight Motion edits and 36 Premiere cut moves, every shot and keyframe as editable JSON">
</picture>

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/why-dark.svg">
  <img src="assets/readme/why-light.svg" width="100%" alt="Why EditsMaker: one command, trend-aware, reads first, MIT and runs on your computer">
</picture>

---

<details>
<summary><b>Command line</b></summary>

Replay a preset over a channel's videos:

```sh
edits https://www.youtube.com/@channel --preset yuraprst
```

Cut it to TikTok's 3rd trending song:

```sh
edits https://www.youtube.com/@channel --preset yuraprst --music trend:3
```

Colour and frame:

```sh
edits https://www.youtube.com/@channel --preset yuraprst --look tealorange --aspect 4:5
```

Read 12 videos' captions, download only the best 4:

```sh
edits https://www.youtube.com/@channel --preset yuraprst --videos 12 --use 4
```

Calmer moves (2 is wilder):

```sh
edits https://www.youtube.com/@channel --preset yuraprst --intensity 0.6
```

A sad music montage without a preset:

```sh
edits https://www.youtube.com/@channel --music song.mp3 --sad
```

Talking clips: pauses cut, word-by-word captions, a hook on top, music under the voice:

```sh
edits https://www.youtube.com/@channel -n 3 --captions hormozi --bed trend:7
```

Caption styles: hormozi, mrbeast, karaoke, box, minimal, neon:

```sh
edits https://www.youtube.com/watch?v=VIDEO_ID --captions mrbeast
```

Hits on the hardest beats, speed ramps and words on screen:

```sh
edits https://www.youtube.com/@channel --preset yuraprst --velocity --fx shake,rgb,freeze --line "Yok artık kaliteye bak."
```

Slow, faster, hidden faces, a cleaner finish:

```sh
edits https://www.youtube.com/watch?v=VIDEO_ID --speed 1.2 --blur-faces --stabilize --denoise --codec h265
```

Draw on it to point things out: an arrow, a ring, a cross, a burst, or your own .svg (`edits looks` lists them):

```sh
edits draw clip.mp4 "arrow@0.5-2.5:0.62,0.3,0.3" "circle@3-5:0.5,0.4,0.6#ffffff" "question@8-10:0.75,0.2,0.2"
```

What is trending on TikTok:

```sh
edits trends
```

Colour looks:

```sh
edits looks
```

Presets:

```sh
edits preset list
```

One preset in full:

```sh
edits preset show yuraprst
```

Save a changed preset:

```sh
edits preset save my-preset.json
```

Import Alight Motion or Premiere presets:

```sh
edits preset import presets.zip
```

Check the tools:

```sh
edits doctor
```

</details>

<details>
<summary><b>Presets</b></summary>

30 edits (anime and 9:16 community edits made in Alight Motion) and 36 cut moves (Premiere transitions), kept as
JSON in [`presets/`](presets). Nothing is fixed: print one whole, change shots, speed, zooms, shakes, flashes,
look or song, then use the file for one edit or save it. With another song, cuts move onto its beats.

```sh
edits preset show yuraprst > my-preset.json
```

```sh
edits https://www.youtube.com/@channel --preset my-preset.json
```

```sh
edits preset save my-preset.json
```

</details>

<details>
<summary><b>Agents</b></summary>

Add it to Claude Code:

```sh
claude mcp add edits -s user -- edits mcp
```

Install the skills:

```sh
edits skills install
```

Tools: `edits_make`, `edits_presets`, `edits_preset_get`, `edits_preset_save`, `edits_looks`, `edits_trends`,
`edits_doctor`. An agent reads a preset, changes it for the footage and renders with it.

</details>

<details>
<summary><b>Build from source</b></summary>

```sh
cargo install --git https://github.com/OnurByte/EditsMaker
```

Only cut what you have the rights to: songs, footage and presets belong to their makers.

</details>

<details>
<summary><b>Contributing</b></summary>

Fixes, presets and looks are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md).

[MIT](LICENSE). The face model is [rustface](https://github.com/atomashpolskiy/rustface)'s (BSD-2).

</details>
