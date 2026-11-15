---
name: edits-mcp
description: Connect the `edits` video editor to an AI agent as an MCP server. Use when setting up edits for Claude Code, Codex, Cursor or another MCP client.
---

# edits over MCP

`edits mcp` serves every command over stdio. Add it to a client:

```sh
claude mcp add edits -- edits mcp                       # Claude Code
codex mcp add edits -- edits mcp                        # Codex
```

Other clients: command `edits`, args `["mcp"]`.

Tools: `edits_make`, `edits_presets`, `edits_preset_get`, `edits_preset_save`, `edits_looks`, `edits_library`, `edits_trends`,
`edits_doctor`. A typical turn: `edits_presets` → `edits_preset_get` → change the JSON for the footage →
`edits_make` with `preset_json`. Rendering takes from seconds to a few minutes; the result lists each clip's path.
Install the skills too: `edits skills install`.
