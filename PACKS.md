# The pack standard

Version 1 (`"schemaVersion": 1`). This is what `edits` reads, checks and refuses; anything that follows it can be shared
as a pack, made by hand, by `edits` or by an agent.

## A pack is a folder

```
sad-rain/
  manifest.json   what the pack is (always)
  preset.json     an edit's shape: shots, moves, flashes, look   (kind "preset")
  SKILL.md        instructions an AI agent follows               (kind "skill")
  automation.js   a script that does a job on one website        (kind "automation")
  preview.svg     its picture: .svg, .png, .jpg, .jpeg or .webp
```

Nothing else is read. When a pack is added, only these files are copied into a folder of its own
(`~/.config/edits/packs/<id>/`); each must be a plain file (no links) under 20 MB. A pack never reaches into another
pack's folder, and removing it removes all of it.

## manifest.json

```json
{
  "schemaVersion": 1,
  "id": "io.github.you.sad-rain",
  "name": "Sad rain",
  "version": "1.0.0",
  "author": "you",
  "description": "Long grey holds that breathe, for a quiet song.",
  "kinds": ["preset", "skill"],
  "preview": "preview.svg",
  "tags": ["slow", "9:16"],
  "license": "MIT"
}
```

| Field | Needed | What it holds |
| --- | --- | --- |
| `schemaVersion` | yes | `1`. Any other number is refused. |
| `id` | yes | Unique, 1 to 100 of `a-z 0-9 . - _`, not starting with `.`, no `..`. Reverse-domain for shared packs (`io.github.you.name`). `edits.` belongs to the packs built into `edits`; no other pack may use it. |
| `name` | yes | What people see. |
| `version` | yes | `MAJOR.MINOR.PATCH`. Raise it when you change the pack. |
| `author` | yes | Who made it. Keep the original maker's name when you rebuild someone's preset. |
| `description` | yes | One or two sentences on how the edit feels or what the skill does. |
| `kinds` | yes | Some of `"preset"`, `"skill"`, `"automation"`: exactly the files that are there. |
| `preview` | no | One of `preview.svg`, `preview.png`, `preview.jpg`, `preview.jpeg`, `preview.webp`. Leave it out and `edits` draws one. |
| `tags` | no | Short words to find it by: pace, moves, frame (`fast`, `zoom`, `9:16`). |
| `license` | no | An SPDX name (`MIT`, `CC-BY-4.0`). |
| `automation` | for an automation | Where its script runs; see below. |

Fields are camelCase. Unknown fields are ignored, so a newer pack still opens in an older `edits`.

## preset.json

The same JSON `edits preset show NAME` prints, and `edits preset save` takes. It needs at least one shot.

| Field | What it holds |
| --- | --- |
| `name`, `about` | Its name (what `--preset` finds) and a line on it. `aka`: older names that still find it. |
| `w`, `h`, `total` | Frame size in pixels and length in seconds. |
| `shots` | The cuts, in order: `start`, `end` (seconds), `speed`, and curves for `zoom`, `x`, `y`, `rot`, `shake`, `blur`, `exposure`, `hue`; plus `saturation`, `contrast`, `kelvin`, `vignette`, `grain`, `glow`, `sharpen`. |
| `flashes` | White or coloured hits between shots. |
| `look` | A colour look from `edits looks`, or nothing to keep the footage's colours. |
| `intensity` | `1` as made, `0.5` calmer, `2` wilder: scales every move. |
| `fit_beats` | With another song, move each cut onto the nearest beat (default `true`). |
| `song` | The song it was cut to, when it has one. |

A curve is a list of `[time 0..1 within the shot, value, ease]` keys; an ease is `"linear"` or `"in K"`, `"out K"`,
`"inout K"` (K the steepness, as in `"out 4"`). A flash has `start`, `end`, `color` (`"#RRGGBB"`), an `opacity` curve and `blink`.

## SKILL.md

An [agent skill](https://docs.claude.com/en/docs/agents-and-tools/agent-skills): front matter with a `name:` (lowercase,
digits, dashes; it becomes the skill's folder name) and a `description:` saying when to use it, then the instructions.

```markdown
---
name: sad-rain
description: Cut a slow grey edit with the sad-rain preset. Use when the user wants a quiet, sad edit.
---

# Sad rain
...
```

A skill is instructions an agent will follow on someone else's computer: no hidden steps, no downloads, no commands
that do more than the skill says. Skills are read before a pack is listed.

## automation.js

A job on one website, done from the person's own signed-in account: posting a clip, for now. The `edits.post-tiktok`,
`edits.post-youtube` and `edits.post-instagram` packs are the examples.

```json
"automation": {
  "site": "tiktok",
  "login": "https://www.tiktok.com/login",
  "page": "https://www.tiktok.com/tiktokstudio/upload",
  "cookie": "sessionid",
  "domain": "tiktok.com"
}
```

| Field | What it holds |
| --- | --- |
| `site` | Its name in `edits connect SITE` and `edits post --to SITE`. |
| `login`, `page` | The sign-in page, and the page the script runs in: both `https://`, both on one site. |
| `cookie`, `domain` | The cookie that says someone is signed in, and the domain it is set on. |

The file is one function, `async (p) => { ... }`, comments above it allowed. `edits` runs it inside `page` in a browser
profile of its own, one per account, and passes `p`: `{"do": "who"}` returns `{id, name}` of the signed-in account;
`{"do": "post", caption, private, expect}` posts the file `edits` put in the page's `#__edits_file` input and returns
the post's address. Throw `"login"` when the sign-in has expired.

**Isolated.** An automation sees one site, one account's profile and one clip, nothing else on the computer. An added
one does not run at all until the person has read it and said so with `edits packs trust ID` (or in the app); if its
`automation.js` changes, it asks again. Built-in ones are trusted.

## Making one

```sh
edits packs create footage.mp4 --name sad-rain      # a new pack from a video: its rhythm, moves borrowed from presets
edits packs make DIR [--preset NAME] [--id ID]      # turn a folder (or a saved preset) into a pack: manifest, picture, check
```

Both write `manifest.json` and `preview.svg` for you and check the pack against this page. Packs made here land in
`~/.config/edits/packs/<id>/`, next to the ones you added, and their presets work with `--preset` at once.

## Sharing one

1. A public GitHub repository with the pack at its root, a README and a license.
2. Check it: `edits packs add https://github.com/you/repo`.
3. A pull request adding one line to [`packs.json`](packs.json):

```json
{"id": "io.github.you.sad-rain", "repo": "https://github.com/you/repo", "name": "Sad rain", "description": "...", "kinds": ["preset"]}
```

Then anyone finds it in the marketplace (the app, or `edits packs browse`) and adds it with `edits packs add io.github.you.sad-rain`, and take it away with
`edits packs remove io.github.you.sad-rain`. The list's `id` must be the manifest's, unique and not `edits.`.

## Built into edits

A folder in [`packs/`](packs) named after the pack, id `edits.<folder>`, author `edits` or the original maker. Then
`edits packs make packs/NAME --id edits.NAME`, `edits packs gallery packs` and `cargo test`: every shipped pack must pass
this page's checks and have a picture. Only add presets you made or may share.
