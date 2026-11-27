#!/usr/bin/env bash
set -eu -o pipefail

mkdir base
(cd base
  git init -q

  git checkout -b main
  touch a && git add a
  git commit -q -m c1
  echo 1 >> a
  git commit -q -am c2
  echo 1 >> a
  git commit -q -am c3
)

mkdir empty
(cd empty
  git init -q

  git checkout -b main
  touch a && git add a
  git commit -q -m c1
  touch .git/shallow
)

git clone --depth 1 --bare file://$PWD/base shallow.git
git -C  shallow.git commit-graph write --no-progress --reachable

git clone --depth 1 file://$PWD/base shallow
git -C  shallow commit-graph write --no-progress --reachable

# Keep all three commits on disk so reaching the root would expose a missed boundary.
boundary=$(git -C base rev-parse HEAD^)
for name in trailing-space trailing-hex trailing-data short-id leading-space oversized-record; do
  cp -R base "$name"
done

printf '%s \n' "$boundary" >trailing-space/.git/shallow
# In a SHA-1 repository this looks like a complete SHA-256 ID.
printf '%s000000000000000000000000\n' "$boundary" >trailing-hex/.git/shallow
printf '%s suffix without newline' "$boundary" >trailing-data/.git/shallow

if [[ ${#boundary} -eq 64 ]]; then
  printf '%.40s\n' "$boundary" >short-id/.git/shallow
else
  printf '%.39s\n' "$boundary" >short-id/.git/shallow
fi
printf ' %s\n' "$boundary" >leading-space/.git/shallow
printf '%s%1023s\n' "$boundary" '' >oversized-record/.git/shallow

for name in trailing-space trailing-hex trailing-data; do
  git -C "$name" rev-list HEAD >"$name/.git/rev-list.baseline"
done
for name in short-id leading-space oversized-record; do
  if git -C "$name" rev-list HEAD >/dev/null 2>&1; then
    echo "Expected Git to reject the shallow file in $name" >&2
    exit 1
  fi
done
