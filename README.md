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

Then `edits doctor`. You also need [ffmpeg](https://ffmpeg.org) and [yt-dlp](https://github.com/yt-dlp/yt-dlp).

### Or ask your agent

```text
Install EditsMaker from github.com/OnurByte/EditsMaker, run `edits doctor`, add it as an MCP server
(`edits mcp`) and install its skills (`edits skills install`).
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

```sh
edits <link|channel|topic|file> --preset NAME   # replay a preset over the footage
edits ... --music trend:3                        # TikTok's 3rd trending song (or a link/file)
edits ... --look tealorange --aspect 4:5         # colour and frame (--size 1080x1350, --lut file.cube)
edits ... --videos 12 --use 4                    # read 12 videos' captions, download the best 4
edits ... --intensity 0.6                        # calmer moves (2 = wilder)
edits ... --music song.mp3 --sad                 # a music montage without a preset
edits <link>                                     # talking clips with captions

edits preset list | show NAME | save FILE.json | import <zip|xml|prfpset>
edits trends [country]    edits looks    edits doctor
edits mcp                 edits skills install
```

</details>

<details>
<summary><b>Presets</b></summary>

30 edits (anime and 9:16 community edits made in Alight Motion) and 36 cut moves (Premiere transitions), kept as
JSON in [`presets/`](presets). Nothing is fixed: `edits preset show NAME` prints one whole, change shots, speed,
zooms, shakes, flashes, look or song, then use the file with `--preset file.json` or keep it with
`edits preset save`. With another song, cuts move onto its beats.

</details>

<details>
<summary><b>Agents</b></summary>

```sh
claude mcp add edits -- edits mcp
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

Only cut what you have the rights to: songs, footage and presets belong to their makers. Public domain
([Unlicense](LICENSE)); the face model is [rustface](https://github.com/atomashpolskiy/rustface)'s (BSD-2).

</details>
