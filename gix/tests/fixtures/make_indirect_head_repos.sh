#!/usr/bin/env bash
set -eu -o pipefail

# HEAD follows two symbolic branches before reaching an unborn or existing branch.
# Keep Git's resolved name, tree and status beside the index, outside the worktree.
for state in empty staged committed packed; do
  git init -q -b main "$state"
  (
    cd "$state"
    git symbolic-ref refs/heads/main refs/heads/alias
    git symbolic-ref refs/heads/alias refs/heads/topic
    if test "$state" != empty; then
      echo content >a
      git add a
    fi
    if test "$state" = committed || test "$state" = packed; then
      git commit -qm initial
      if test "$state" = packed; then
        git pack-refs --all --prune
      fi
      git rev-parse 'HEAD^{tree}' >.git/tree.git
    else
      git hash-object -t tree --stdin </dev/null >.git/tree.git
    fi
    git symbolic-ref HEAD >.git/head-name.git
    git status --porcelain=v1 >.git/status.git
  )
done

cp -R committed malformed
(
  cd malformed
  printf 'invalid reference\n' >.git/refs/heads/alias
  if git rev-parse --verify HEAD >.git/head.git 2>.git/head-error.git; then
    echo 'a malformed symbolic referent must not resolve to an object' >&2
    exit 1
  fi
)

git init -q -b main cycle
(
  cd cycle
  git symbolic-ref refs/heads/main refs/heads/topic
  git symbolic-ref refs/heads/topic refs/heads/main
  if git rev-parse --verify HEAD >.git/head.git 2>.git/head-error.git; then
    echo 'a symbolic HEAD cycle must not resolve to an object' >&2
    exit 1
  fi
)
