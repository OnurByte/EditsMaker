#!/bin/sh
# Release notes for tag $1: every release commit since the previous tag (its title and what it lists), then how
# to install (README's Install section).
set -e
tag="$1"
prev=$(git describe --tags --abbrev=0 "$tag^" 2>/dev/null || true)
range=${prev:+$prev..}$tag
first=1
commits=$(git log --no-merges --reverse --format=%H "$range")
count=$(printf '%s\n' "$commits" | grep -c .)
for c in $commits; do
  msg=$(git log -1 --format=%B "$c" | grep -v '^Co-Authored-By:')
  title=$(printf '%s\n' "$msg" | head -n1 | sed 's/^[0-9][0-9.]*: *//')
  body=$(printf '%s\n' "$msg" | tail -n +2 | sed '/./,$!d')
  if [ $first = 1 ]; then printf '%s\n\n## New\n\n' "$title"; first=0; fi
  # one commit needs no heading of its own: the release title already is it
  [ "$count" -gt 1 ] && printf '### %s\n\n' "$title"
  [ -n "$body" ] && printf '%s\n\n' "$body"
done
# README's Install section, its OS logo pictures turned into plain headings
sed -n '/^## Install/,/^<picture>/p' README.md | sed '$d' | sed 's/^### <picture>.*<\/picture> /### /'
