#!/bin/sh
# EditsMaker installer: downloads the prebuilt `edits` binary from GitHub Releases, verifies its SHA-256, installs it.
# No Rust, no compiler. ffmpeg and yt-dlp come along: `edits setup` downloads them at the end (whisper-cli is optional).
#   EDITS_VERSION=v0.1.0   pick a release (default: latest)
#   EDITS_BIN=~/bin        install directory (default: ~/.local/bin)
set -eu
REPO="OnurByte/EditsMaker"
BIN="${EDITS_BIN:-$HOME/.local/bin}"

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64)              T=x86_64-unknown-linux-gnu ;;
  Linux-aarch64|Linux-arm64) T=aarch64-unknown-linux-gnu ;;
  Darwin-x86_64)             T=x86_64-apple-darwin ;;
  Darwin-arm64)              T=aarch64-apple-darwin ;;
  *) echo "edits: no prebuilt binary for $(uname -s)/$(uname -m). See https://github.com/$REPO/releases" >&2; exit 1 ;;
esac

if [ -n "${EDITS_VERSION:-}" ]; then BASE="${EDITS_BASE:-https://github.com/$REPO/releases/download/$EDITS_VERSION}"
else BASE="${EDITS_BASE:-https://github.com/$REPO/releases/latest/download}"; fi

TMP="$(mktemp -d)"; trap 'rm -rf "$TMP"' EXIT
ASSET="edits-$T.tar.gz"
dl() { if command -v curl >/dev/null 2>&1; then curl -fsSL "$1" -o "$2"; else fetch -qo "$2" "$1"; fi; } # FreeBSD base has fetch, not curl
dl "$BASE/$ASSET" "$TMP/$ASSET" || { echo "edits: no release asset $ASSET at $BASE" >&2; exit 1; }
dl "$BASE/SHA256SUMS" "$TMP/SHA256SUMS"

WANT="$(awk -v f="$ASSET" '$2==f {print $1}' "$TMP/SHA256SUMS")"
if command -v sha256sum >/dev/null 2>&1; then GOT="$(sha256sum "$TMP/$ASSET" | cut -d' ' -f1)"
elif command -v shasum >/dev/null 2>&1; then GOT="$(shasum -a 256 "$TMP/$ASSET" | cut -d' ' -f1)"
else GOT="$(sha256 -q "$TMP/$ASSET")"; fi
[ -n "$WANT" ] && [ "$WANT" = "$GOT" ] || { echo "edits: checksum mismatch, refusing to install" >&2; exit 1; }

tar -xzf "$TMP/$ASSET" -C "$TMP"
mkdir -p "$BIN"
install -m 755 "$TMP/edits" "$BIN/edits"
echo "edits: installed $("$BIN/edits" --version) to $BIN/edits"
case ":$PATH:" in *":$BIN:"*) ;; *) echo "edits: add $BIN to your PATH" ;; esac
# ffmpeg and yt-dlp, if this machine lacks them: static builds into ~/.config/edits/bin, checksum checked
"$BIN/edits" setup || echo "edits: ffmpeg/yt-dlp download failed; edits tries again on first use"
