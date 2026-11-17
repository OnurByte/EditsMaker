#!/bin/sh
# Release notes for tag $1: every release commit since the previous tag (its title and what it lists), then how
# to install (README's Install section) on top.
set -e
tag="$1"
prev=$(git describe --tags --abbrev=0 "$tag^" 2>/dev/null || true)
range=${prev:+$prev..}$tag
commits=$(git log --no-merges --reverse --format=%H "$range")
count=$(printf '%s\n' "$commits" | grep -c .)
# the release's title is its own commit's (the last one: "0.14.0: ..."), not whatever landed first since the last tag
last=$(git log --no-merges -1 --format=%s "$tag" | sed 's/^[0-9][0-9.]*: *//')
# how to install comes first, what changed under it (the release's title opening it)
# README's Install section alone (it ends at the "---" before the folded sections), OS logo pictures as plain headings
sed -n '/^## Install/,/^---$/p' README.md | sed '$d' | sed 's/^### <picture>.*<\/picture> /### /'
printf '## New: %s\n\n' "$last"
for c in $commits; do
  msg=$(git log -1 --format=%B "$c" | grep -v '^Co-Authored-By:')
  title=$(printf '%s\n' "$msg" | head -n1 | sed 's/^[0-9][0-9.]*: *//')
  body=$(printf '%s\n' "$msg" | tail -n +2 | sed '/./,$!d')
  # one commit needs no heading of its own: the release title already is it
  [ "$count" -gt 1 ] && printf '### %s\n\n' "$title"
  [ -n "$body" ] && printf '%s\n\n' "$body"
done
