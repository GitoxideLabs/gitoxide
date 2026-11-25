#!/usr/bin/env bash
set -eu -o pipefail

# Each case is one step of Git's histogram diff in `xdiff/` that gix-imara-diff once did differently.
# The inputs stay below 1024 bytes on at least one side, so `--unified=0` doesn't trim a common tail.

if [[ ${1:-} == rediff-after-slide ]]; then
  # Git 2.54 diffs shifted histogram groups again (e417277ae9). This finds the empty line
  # shared by the merged groups, so "L" becomes "R" and "Q", and "P" is removed.
  mkdir rediff-after-slide
  printf 'L\n\nP\nQ\n\n' >rediff-after-slide/old
  printf 'R\nQ\n\nQ\n\n' >rediff-after-slide/new
  cases=(rediff-after-slide)
else
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

  # At 65 occurrences, "B" can seed a match and extend it through "A".
  # At 66, Git seeds from "A" and keeps all four lines of the new file.
  mkdir occurrence-seed-limit occurrence-overflow
  {
    printf '%s\n' B B B B A A B A
    for i in {1..60}; do
      printf 'B\n'
    done
  } >occurrence-seed-limit/old
  printf '%s\n' B A B A >occurrence-seed-limit/new
  cp occurrence-seed-limit/old occurrence-overflow/old
  printf 'B\n' >>occurrence-overflow/old
  cp occurrence-seed-limit/new occurrence-overflow/new

  # The unique unchanged lines split edits before a shared repetitive tail. Two final
  # blank lines in the mixed case can still seed a match after the prefix is gone.
  mkdir repetitive-tail alternating-tail mixed-frequency-tail
  for side in old new; do
    {
      for i in {1..8}; do
        printf '%s-%s\nkeep-%s\n' "$side" "$i" "$i"
      done
      printf '%s-last\n' "$side"
    } >"repetitive-tail/$side"
    cp "repetitive-tail/$side" "alternating-tail/$side"
    for i in {1..128}; do
      printf 'x\n' >>"repetitive-tail/$side"
      printf '%s\n' x y >>"alternating-tail/$side"
    done
    cp "repetitive-tail/$side" "mixed-frequency-tail/$side"
    printf '\n\n' >>"mixed-frequency-tail/$side"
  done

  # Git doesn't strip the common prefix and suffix before a histogram diff. Stripping the final "}"
  # makes the first "    }" the anchor, where Git anchors the second.
  mkdir common-ends
  printf '    }\n    }\n}\n' >common-ends/old
  printf '        }\n    }\n}\n}\n' >common-ends/new

  cases=(
    skip-after-match occurrence-limit occurrence-seed-limit occurrence-overflow
    repetitive-tail alternating-tail mixed-frequency-tail common-ends
  )
fi

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
