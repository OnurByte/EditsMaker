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

## Presets

Presets are JSON in [`presets/`](presets), built into the binary.

- From Alight Motion or Premiere: `edits preset import <zip|xml|folder|prfpset> --into presets/`
- By hand: copy one, change it, check it with `edits preset show NAME`, then `cargo test` (every shipped preset
  must parse).
- Only add presets you made or may share, and keep the maker's name in `name` or `about`.

## Pull requests

- One change per pull request, with a line on what it does and how you checked it.
- Commit messages say what changed and why, in plain words.

By contributing you agree that your work is released under the [MIT License](LICENSE).
