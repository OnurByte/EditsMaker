#!/bin/sh
# Release notes for tag $1: what the release commit says changed, then how to install (README's Install section).
set -e
tag="$1"
msg=$(git log --no-merges -1 --format=%B "$tag" | grep -v '^Co-Authored-By:')
title=$(printf '%s\n' "$msg" | head -n1 | sed 's/^[0-9][0-9.]*: *//')
body=$(printf '%s\n' "$msg" | tail -n +2 | sed '/./,$!d')
printf '%s\n\n' "$title"
[ -n "$body" ] && printf '## New\n\n%s\n\n' "$body"
# README's Install section, its OS logo pictures turned into plain headings
sed -n '/^## Install/,/^<picture>/p' README.md | sed '$d' | sed 's/^### <picture>.*<\/picture> /### /'
