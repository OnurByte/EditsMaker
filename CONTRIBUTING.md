# Contributing

Thanks for helping. Bug reports, new presets, looks, fixes and docs are all welcome.

**Quick path:** fork, branch, `cargo test`, open a pull request. Open an issue first for anything that changes a
command, a flag or the preset format.

## Build and test

```sh
cargo test                                   # unit tests, no network, no ffmpeg needed
cargo clippy --all-targets -- -D warnings    # CI runs this too
cargo build --release && ./target/release/edits doctor
```

- New logic needs a test beside it (`#[cfg(test)]` in the same file).
- Keep it one binary: new crates only when the standard library or ffmpeg cannot do the job.
- Check a change on a real video before calling it done: render one, look at the frames.

## Packs

A pack is a folder holding a preset (`preset.json`), an agent skill (`SKILL.md`) or both, with a `manifest.json` and
a picture. The ones in [`packs/`](packs) are built into the binary; community packs live in their own repositories.

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

- `id`: lowercase letters, digits, dots and dashes, unique; `edits.` is the built-in packs'.
- `kinds`: `preset` when `preset.json` is there, `skill` when `SKILL.md` is (with a `name:` in its front matter).
- `preview`: `preview.svg`, `.png`, `.jpg` or `.webp` beside the manifest. Leave it out and one is drawn.
- `edits packs make DIR [--preset NAME] [--id ID]` writes the manifest and the picture for you, and checks the pack.

**A community pack:** a public GitHub repository with the pack at its root, a README and a license. Check it with
`edits packs add https://github.com/you/repo`, then open a pull request adding a line to [`packs.json`](packs.json):

```json
{"id": "io.github.you.sad-rain", "repo": "https://github.com/you/repo", "name": "Sad rain", "description": "...", "kinds": ["preset"]}
```

Everyone can then `edits packs browse` and `edits packs add io.github.you.sad-rain`. A pack's skill is instructions
agents follow, so it is read before it is listed.

**A built-in pack:** a folder in `packs/` (from Alight Motion or Premiere: `edits preset import <zip|xml|folder|prfpset>
--into packs/NAME`, then rename the file to `preset.json`), `edits packs make packs/NAME --id edits.NAME`,
`edits packs gallery packs`, and `cargo test` (every shipped pack must be whole). Only add presets you made or may
share, and keep the maker's name in `name` or `about`.

## Pull requests

- One change per pull request, with a line on what it does and how you checked it.
- Commit messages say what changed and why, in plain words.

By contributing you agree that your work is released under the [MIT License](LICENSE).
