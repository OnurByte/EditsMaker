---
name: edits-trends
description: Find what is trending on TikTok right now and cut an edit to it with `edits`. Use when the user wants a trending/viral song, "akım", or the current TikTok sound for an edit.
---

# Trending songs

```sh
edits trends          # TikTok's chart for Türkiye now (kworb.net)
edits trends us       # another country
edits <input> --preset NAME --music trend:3     # cut to the 3rd song
```

Over MCP: `edits_trends`, then `edits_make` with `music: "trend:N"`.

Match the song to the preset: phonk / "sped up" songs suit fast velocity presets (high `speed`, many shots);
slow or emotional songs suit calm presets or `--sad`. The song is found on YouTube by its title; check the log line
`song "…"` to see what was used. The chart page carries no date, so it can lag by a few days.
