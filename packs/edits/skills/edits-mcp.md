---
name: edits-mcp
description: Connect the `edits` video editor to an AI agent as an MCP server. Use when setting up edits for Claude Code, ChatGPT (Codex), OpenCode, Cursor, Windsurf, Gemini CLI, Claude Desktop, VS Code or another MCP client, or when its tools do not show up.
---

# edits over MCP

`edits mcp` serves every command over stdio. Add it to every agent on this machine at once:

```sh
edits mcp install      # Claude Code, ChatGPT (codex), OpenCode, Cursor, Windsurf, Gemini CLI, Claude Desktop, VS Code
edits skills install   # the skills into each agent's skill folder
```

Optional, only when the user wants it: `edits guard install` adds a guard to Claude Code (`~/.claude/settings.json`,
a Bash PreToolUse hook running `edits guard`): a shell command that does video work by hand is refused with what to
use instead; `--ask` asks the user each time instead of refusing. `edits guard remove` takes it out, and nothing
puts it back unasked (an update only points an installed guard at the new edits). Never install it for the user
without asking: it changes how Claude Code works in every project.

Restart the agent after. Any other client: command `edits` (full path if the client has no PATH), args `["mcp"]`.
OpenCode 2: `opencode mcp list` can say "No MCP servers configured" on its first run while it starts; run it again.

Tools: `edits_make`, `edits_presets`, `edits_preset_get`, `edits_preset_save`, `edits_preset_set`, `edits_pack_create`,
`edits_learn`, `edits_check`, `edits_frames`, `edits_words`, `edits_moments`, `edits_looks`, `edits_library`, `edits_trends`, `edits_encoder` (GPU or CPU, saved), `edits_doctor`. A typical turn: `edits_presets` → `edits_preset_get` → change the JSON for the footage →
`edits_make` with `preset_json`. Rendering takes from seconds to a few minutes; the result lists each clip's path.
Install the skills too: `edits skills install`.
