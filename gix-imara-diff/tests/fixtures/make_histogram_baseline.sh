#!/usr/bin/env bash
set -eu -o pipefail

# Generate with Git 2.54 or newer, which diffs a shifted histogram change group again (e417277ae9).
# Each case is one step of Git's histogram diff in `xdiff/` that gix-imara-diff once did differently.
# The inputs stay below 1024 bytes on at least one side, so `--unified=0` doesn't trim a common tail.

# After a match, Git skips the later occurrences of its first line in the old file up to the end
# of the match in the old file (`xdiff/xhistogram.c`). Git matches the second "B" of the old file.
# Comparing with the end of the match in the new file matches the first "B" instead.
mkdir skip-after-match
printf '\nA\nB\n\nB\n\n' >skip-after-match/old
printf 'C\nD\n\nB\n' >skip-after-match/new

# Git anchors on a line with up to 64 occurrences in the old file and keeps one "x". With a limit
# of 63, "x" can't be an anchor and the diff keeps no line.
mkdir occurrence-limit
for i in {1..64}; do
  printf 'x\n'
done >occurrence-limit/old
printf 'a\nb\nc\n\nd\ne\nx\nf\n' >occurrence-limit/new

# Git doesn't strip the common prefix and suffix before a histogram diff. Stripping the final "}"
# makes the first "    }" the anchor, where Git anchors the second.
mkdir common-ends
printf '    }\n    }\n}\n' >common-ends/old
printf '        }\n    }\n}\n}\n' >common-ends/new

# Sliding merges change groups. Git diffs a group again if it moved and the group in the other
# file isn't empty, for histogram only (`xdiff/xdiffi.c`). This finds the empty line that both
# groups have in common, so "L" becomes "R" and "Q", and "P" is removed.
mkdir rediff-after-slide
printf 'L\n\nP\nQ\n\n' >rediff-after-slide/old
printf 'R\nQ\n\nQ\n\n' >rediff-after-slide/new

cases=(skip-after-match occurrence-limit common-ends rediff-after-slide)
for case in "${cases[@]}"; do
  mkdir "$case-reversed"
  cp "$case/new" "$case-reversed/old"
  cp "$case/old" "$case-reversed/new"
done

baseline() {
  local name=$1
  shift
  # --no-index returns 1 for differences. Do not hide actual Git failures.
  # Zero context exposes each compacted hunk's ranges; ignore file headers and function context.
  git diff --no-index --no-ext-diff --no-textconv --no-color --diff-algorithm=histogram \
    --unified=0 "$@" -- old new >"$name.patch" || test "$?" -eq 1
  awk '/^@@ / {print $2, $3}' "$name.patch" >"$name.hunks"
}

for case in */; do
  (
    cd "$case"
    baseline NoHeuristic --no-indent-heuristic
    baseline IndentHeuristic --indent-heuristic
  )
done
