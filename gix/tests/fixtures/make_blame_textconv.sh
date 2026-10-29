#!/usr/bin/env bash
set -eu -o pipefail
# Textconv expands each stored pipe-delimited record into independently blamed lines.
# A rename keeps history attribution while a worktree-only driver override checks
# that current attributes, rather than only the committed index, are consulted.
git init -q
cat > render <<'CONVERTER'
#!/bin/sh
tr '|' '\n' < "$1"
CONVERTER
chmod +x render
git config diff.render.textconv "$PWD/render"
printf '*.dat diff=render\noverride.dat !diff\n' > .gitattributes
printf 'first|second|third|' > original.dat
cp original.dat history.dat
cp original.dat override.dat
ln -s 'first|second|' link.dat
git add .
git commit -qm root
printf 'first|SECOND|third|' > original.dat
cp original.dat history.dat
cp original.dat override.dat
rm link.dat
ln -s 'first|SECOND|' link.dat
git commit -qam second
git mv original.dat renamed.dat
printf 'first|SECOND|THIRD|' > renamed.dat
cp renamed.dat history.dat
cp renamed.dat override.dat
git commit -qam renamed
printf 'first|SECOND|THIRD|fourth|' > renamed.dat
cp renamed.dat history.dat
cp renamed.dat override.dat
printf '*.dat diff=render\n' > .gitattributes
