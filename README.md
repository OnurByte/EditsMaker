<p align="center"><img src="assets/banner.svg" alt="EditsMaker: link in, edits out" width="100%"></p>

# EditsMaker

Give it a YouTube link, a topic or a video file. It downloads the videos, finds the moments worth cutting and
renders finished edits: reframed, captioned, graded, loudness-matched. One Rust binary, everything runs on your machine.

```sh
edits "recep ivedik komik sahneler"            # search, analyse 5 videos, cut the 3 best moments
edits https://www.youtube.com/watch?v=...  -n 5
edits https://www.youtube.com/@channel --videos 4   # a channel's newest uploads
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

## Music edits

```sh
edits https://www.youtube.com/@channel --music https://www.youtube.com/watch?v=... --sad --voice --tag @you
```

The edits people repost: the footage's best shots cut onto a song.

- **Beats.** The song's beats are found in the binary (spectral flux, tempo, dynamic-programming beat tracking).
  The loudest stretches become the edits, one per `-n`.
- **Shots.** Every video is scanned at 4 frames a second for hard cuts, movement, brightness and faces. Faces and
  movement win; black frames, intros and end screens lose. The best shot opens the edit, the rest keep story order,
  and pieces of one scene are never used back to back.
- **Transitions from the music.** Hard cuts on the beat; on the hardest hits a white flash or a zoom.
  `--sad` dissolves, dips to black on the hardest hits, slows the picture to 80% and turns it black and white.
- **Every shot moves.** A slow push-in; fast edits land each cut with a quick punch.
- **`--voice`** puts the speaker's best line over it, the music ducking under the words, with captions:
  one word at a time in heavy capitals for fast edits, two or three words fading for sad ones.
- **`--text "THE PROBLEM"`** lays one line over the whole edit, **`--tag`** signs it.

Only publish what you have the rights to: the song and the footage both belong to someone.

## Why it is fast

It never downloads a whole video. Measured on two 8-minute videos from a channel, three clips out:

| | Whole videos + whisper | Now |
|---|---|---|
| First run | 315 s | 50 s |
| Same videos again | | 18 s |

- **Audio first.** Analysis needs sound, comments and replay data: a few MB, about 8 s per video.
- **The site's captions.** YouTube already has word-timed speech recognition; whisper only runs when there are
  none, or when they mask profanity as `[ __ ]`.
- **Only the moments.** The picture is downloaded for the chosen seconds alone, by HTTP range.
- **All at once.** Videos are analysed in parallel, clips are fetched and rendered four at a time.
- **Cached by video id** in `~/.cache/edits`: plain files, no database to run or corrupt.

## What an edit gets

- **Reframing.** A wide video becomes 9:16, 1:1 or 4:3 by cropping around the faces it detects, not around the
  middle. With no face in the shot (screen recordings, graphics) the whole frame is kept over a blurred copy;
  `--fit` forces that.
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
