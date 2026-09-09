#!/usr/bin/env bash
set -eu -o pipefail

# core.worktree is literal even when its value resembles a path interpolation.
# Keep Git's baseline relative to each repository so the fixture can be relocated.
i=0
for path in '~' '~someone' '~/nested' '~someone/nested' '%(prefix)/nested'; do
  i=$((i + 1))
  repo="repo-$i"
  git init -q "$repo"
  mkdir -p "$repo/.git/$path"
  git -C "$repo" config core.worktree "$path"
  worktree=$(git -C "$repo" rev-parse --path-format=relative --show-toplevel)
  printf '%s\t%s\n' "$repo" "$worktree" >>worktrees.baseline
done
