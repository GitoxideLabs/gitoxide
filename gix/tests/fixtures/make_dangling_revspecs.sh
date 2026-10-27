#!/usr/bin/env bash
set -eu -o pipefail

function baseline() {
  printf '%s\n' "$1" >>baseline.git
  MSYS2_ARG_CONV_EXCL='*' git rev-parse -q --verify "$1" >>baseline.git 2>/dev/null || echo $? >>baseline.git
}

git init -q -b main names
(
  cd names
  git commit -q --allow-empty -m initial
  # Earlier DWIM candidates must not hide branches, remote HEADs or object prefixes.
  git symbolic-ref refs/tags/main refs/tags/missing
  git symbolic-ref refs/tags/cycle refs/tags/cycle
  git branch cycle
  git symbolic-ref refs/heads/origin refs/heads/missing
  git update-ref refs/remotes/origin/main HEAD
  git symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/main
  commit_prefix=$(git rev-parse --short=7 HEAD)
  git symbolic-ref "refs/heads/$commit_prefix" refs/heads/missing
  printf '%s\n' "$commit_prefix" >commit-prefix
  for spec in main cycle origin "$commit_prefix"; do
    baseline "$spec"
  done
)

git init -q -b main deleted
(
  cd deleted
  git commit -q --allow-empty -m initial
  # A deleted prior branch still DWIM-matches an unresolved remote HEAD.
  git symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/missing
  git checkout -q -b origin
  git checkout -q main
  git branch -q -D origin
  baseline '@{-1}'
)

git init -q -b main detached-cycle
(
  cd detached-cycle
  git commit -q --allow-empty -m initial
  commit_id=$(git rev-parse HEAD)
  git checkout -q --detach "$commit_id"
  git checkout -q main
  # A full ID from a detached checkout remains valid despite a cyclic ref collision.
  git symbolic-ref "refs/tags/$commit_id" "refs/tags/$commit_id"
  baseline '@{-1}'
)

git init -q -b main search
(
  cd search
  git commit -q --allow-empty -m wip
  git checkout -q --orphan unborn
  git symbolic-ref refs/heads/alias refs/heads/missing
  git symbolic-ref refs/heads/cycle refs/heads/cycle
  # A failed anchor must not turn a bounded search into a search of every ref.
  for anchor in HEAD alias cycle; do
    baseline "$anchor^{/wip}"
    baseline "$anchor^{/}"
  done
  baseline ':/wip'
  baseline 'main^{/wip}'
)
