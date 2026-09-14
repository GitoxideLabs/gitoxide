#!/usr/bin/env bash
set -eu -o pipefail

# Include the umask in the fixture arguments so cached modes match the test process.
umask "$1"

# More than three commits so cloning at depth two and deepening by one both leave a shallow file.
git init -q base
for n in 1 2 3 4; do
  echo "$n" > base/file
  git -C base add file
  git -C base commit -q -m "commit $n"
done
git clone --depth 2 --no-local base base.shallow

for shared in group 0640 false; do
  requested="git-$shared-base"
  # Pass sharing via -c so it applies during repository initialization on older Git versions.
  git -c "core.sharedRepository=$shared" clone --bare --no-local --depth 2 base "$requested"
  # Keep a mode-preserving snapshot before fetch replaces the initial shallow file.
  cp -p "$requested/shallow" "$requested/shallow.initial"
  git -C "$requested" -c "core.sharedRepository=$shared" fetch --deepen 1

  # No depth requested: the source advertises a shallow boundary of its own.
  git -c "core.sharedRepository=$shared" clone --bare --no-local base.shallow "git-$shared-base.shallow"
done
