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
- Rust and packs only: no HTML, JS or Python beside the code. A script a website must run is an automation pack.
- `edits` stays one binary: new crates only when the standard library or ffmpeg cannot do the job. The app is
  `src/app` (GPUI, the `app` feature; `--no-default-features` builds without it), and its look follows [BRAND.md](BRAND.md).
- Check a change on a real video before calling it done: render one, look at the frames.

## Packs

A pack is a folder holding a preset (`preset.json`), an agent skill (`SKILL.md`) or both, with a `manifest.json` and
a picture. The ones in [`packs/`](packs) are built into the binary; community packs live in their own repositories.
What a pack must hold, how to make one (`edits packs create`, `edits packs make`) and how to list it: [PACKS.md](PACKS.md).

## Pull requests

- One change per pull request, with a line on what it does and how you checked it.
- Commit messages say what changed and why, in plain words.

By contributing you agree that your work is released under the [MIT License](LICENSE).
