---
name: edits-mcp
description: Connect the `edits` video editor to an AI agent as an MCP server. Use when setting up edits for Claude Code, Codex, OpenCode, Cursor, Windsurf, Gemini CLI, Claude Desktop, VS Code or another MCP client, or when its tools do not show up.
---

# edits over MCP

`edits mcp` serves every command over stdio. Add it to every agent on this machine at once:

```sh
edits mcp install      # Claude Code, Codex, OpenCode, Cursor, Windsurf, Gemini CLI, Claude Desktop, VS Code
edits skills install   # the skills into each agent's skill folder
```

`edits mcp install` (and every `edits update`) also adds a guard to Claude Code (`~/.claude/settings.json`, a
Bash PreToolUse hook running `edits guard`): a shell command that does video work by hand is refused with what to
use instead. `edits guard install` adds it alone.

Restart the agent after. Any other client: command `edits` (full path if the client has no PATH), args `["mcp"]`.
OpenCode 2: `opencode mcp list` can say "No MCP servers configured" on its first run while it starts; run it again.

Tools: `edits_make`, `edits_presets`, `edits_preset_get`, `edits_preset_save`, `edits_preset_set`, `edits_preset_create`,
`edits_learn`, `edits_check`, `edits_frames`, `edits_words`, `edits_looks`, `edits_library`, `edits_trends`, `edits_doctor`. A typical turn: `edits_presets` → `edits_preset_get` → change the JSON for the footage →
`edits_make` with `preset_json`. Rendering takes from seconds to a few minutes; the result lists each clip's path.
Install the skills too: `edits skills install`.
