# Rules

- Never run ffmpeg, ffprobe, yt-dlp or whisper yourself, not even to test. Every job has an `edits` command
  (`edits words`, `edits frames`, `edits check`, `edits make`); `edits guard` (installed on this machine with
  `edits guard install`; never added unasked) refuses the rest.
- Renders the user sees come from the installed `edits` (MCP or CLI), never `target/`. A code change is
  `cargo test`, then release, `edits update`, render.
- If `edits` cannot do something, say so and add it to `edits`; do not script around it.
