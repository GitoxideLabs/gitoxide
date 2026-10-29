#!/usr/bin/env bash
set -eu -o pipefail

# Pack shared direct references, retain symbolic and per-worktree references,
# and preserve preexisting packed values when loose refs shadow them.
git init -q -b main
git commit -q --allow-empty -m base
git tag -am first annotated
git pack-refs --all
git commit -q --allow-empty -m next
git tag -am second annotated-two
git tag lightweight
git update-ref refs/custom/name HEAD
git symbolic-ref refs/heads/alias refs/heads/main
git update-ref refs/bisect/good HEAD
git update-ref refs/worktree/private HEAD
git update-ref refs/namespaces/other/refs/heads/main HEAD
git worktree add -q linked -b linked
git -C linked update-ref refs/worktree/private HEAD~1
