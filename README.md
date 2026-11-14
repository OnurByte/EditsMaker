<p align="center"><img src="assets/banner.svg" alt="EditsMaker: link in, edits out" width="100%"></p>

# EditsMaker

Give it a YouTube link, a topic or a video file. It downloads the videos, finds the moments worth cutting and
renders finished edits: reframed, captioned, graded, loudness-matched. One Rust binary, everything runs on your machine.

```sh
edits "recep ivedik komik sahneler"            # search, analyse 5 videos, cut the 3 best moments
edits https://www.youtube.com/watch?v=...  -n 5
edits interview.mp4 --aspect 9:16 --grade teal
```

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/OnurByte/EditsMaker/main/install.sh | sh
```

Or from source: `cargo install --git https://github.com/OnurByte/EditsMaker`.

It drives tools you install yourself. `edits doctor` tells you what is missing:

| Tool | Needed for |
|---|---|
| `ffmpeg` with libass | everything |
| `yt-dlp` | links and topics (local files work without it) |
| `whisper-cli` (whisper.cpp) + a `ggml-*.bin` model | captions and the transcript |
| an LLM command (`claude -p` by default, set `EDITS_LLM`) | judging what is funny; optional |

## How it finds the moments

Models are bad at humour, so the model is asked last. People are asked first.

1. **Viewer signals.** YouTube's "most replayed" heatmap, and comments that carry a timestamp
   (`1:25 metin abinin çıkışı efsane 😂`), weighted by likes and laughter.
2. **The video itself.** Loudness spikes, and laughter or applause that whisper hears between the words.
3. **Your taste.** The transcript, annotated line by line with those signals, goes to the LLM together with
   `~/.config/edits/taste.md`. That file says, in plain words, what you find funny: swearing is a punchline,
   contrast, absurd confidence, cut right after the punchline. Edit it. It is yours.
4. **Feedback.** `edits rate clip.mp4 +` or `-` appends that clip to the taste file as an example.

For a topic it analyses every downloaded video and renders only the best moments overall. Without an LLM
(`--no-llm`, or none installed) it cuts around the strongest signals.

## What an edit gets

- **Reframing.** A wide video becomes 9:16, 1:1 or 4:3 by cropping around the faces it detects, not around the
  middle. `--fit` keeps the whole frame over a blurred copy instead.
- **Captions.** Word-timed, three words at a time. The punchline turns red and upper case, and the picture punches in.
- **Grade.** `bw`, `dark`, `teal` or `none`.
- **Sound.** Loudness-normalised, short fades in and out.
- **A sidecar `.json`** per clip: source, timestamps, transcript, why it was picked.

`edits study <folder>` measures edits you like (shape, saturation, loudness, shot length) and makes their
shape, grade and loudness your defaults.

## Options

```
-n, --moments N     clips to make, the best across all videos (default 3)
    --videos N      videos to download and analyse for a topic (default 5)
    --aspect A      9:16 (default), 1:1, 4:3 or 16:9
    --fit           keep the whole frame over a blurred copy instead of cropping around faces
    --grade G       bw (default), dark, teal or none
    --lang L        spoken language for captions (default tr)
-o, --out DIR       output folder (default ./edits-out)
    --join          also join all clips into joined.mp4
    --no-captions   no burned-in captions
    --no-llm        pick moments from viewer signals only
```

## Limits

- One fixed crop per clip (the median face). A two-person scene is framed on whoever is on screen most.
- The face detector is a frontal-face model: profiles and tiny faces fall back to a centre crop.
- Small whisper models mishear Turkish. `ggml-large-v3-turbo` is worth the download.
- Viewer signals exist only for YouTube. A local file is judged on sound, transcript and your taste file.
- Only cut what you have the right to use.

## Design

The mark, colours and caption rules are in [`design/`](design/kit.html) with tokens in
[`design/tokens.css`](design/tokens.css).

## License

Public domain ([Unlicense](LICENSE)). The bundled face detection model is from
[rustface](https://github.com/atomashpolskiy/rustface) under its own BSD 2-Clause license.
