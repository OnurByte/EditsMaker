<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/readme/hero-dark.svg">
  <img src="assets/readme/hero-light.svg" width="100%" alt="EditsMaker. Link in. Edits out.">
</picture>

<p align="center">
  <a href="https://github.com/OnurByte/EditsMaker/releases/latest"><b>Download</b></a> ⁜ Linux, macOS and Windows
</p>

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

Talking clips with captions from one video:

```sh
edits https://www.youtube.com/watch?v=VIDEO_ID
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
