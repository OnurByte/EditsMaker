---
name: edits-trends
description: Find what is trending on TikTok (or Spotify) right now and cut an edit to it with `edits`. Use when the user wants a trending or viral song, a trend, the current TikTok sound, or a chart hit for an edit.
---

# Trending songs

```sh
edits trends          # TikTok's chart for your country now (from the system language; kworb.net)
edits trends us       # another country
edits trends spotify tr       # Spotify's daily chart (also: global)
edits <input> --preset NAME --music trend:3     # cut to the 3rd song
edits <input> --preset NAME --music spotify:tr:1
```

Over MCP: `edits_trends` (`chart: "spotify"` for Spotify), then `edits_make` with `music: "trend:N"` or `"spotify:N"`.
A song heard in someone's edit: `edits song <link|file>` (`edits_song`) names it.

Match the song to the preset: phonk / "sped up" songs suit fast velocity presets (high `speed`, many shots);
slow or emotional songs suit calm presets or `--sad`. The song is found on YouTube by its title; check the log line
`song "…"` to see what was used. The chart page carries no date, so it can lag by a few days.
