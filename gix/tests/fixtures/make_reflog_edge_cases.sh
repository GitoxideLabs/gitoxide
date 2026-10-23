#!/usr/bin/env bash
set -eu -o pipefail

function baseline() {
  printf '%s\n' "$1" >>baseline.git
  MSYS2_ARG_CONV_EXCL='*' git rev-parse -q --verify "$1" >>baseline.git 2>/dev/null || echo $? >>baseline.git
}

function initial() {
  git init -q -b main "$1"
  (
    cd "$1"
    GIT_COMMITTER_DATE='2000-01-01 00:00:00 +0000' git commit -q --allow-empty -m one
  )
}

initial expired
(
  cd expired
  GIT_COMMITTER_DATE='2000-01-02 00:00:00 +0000' git commit -q --allow-empty -m two
  # Keep only one -> two: the oldest previous ID remains a valid prior value.
  git reflog delete 'main@{1}'
  for spec in 'main@{0}' 'main@{1}' 'main@{2}' 'main@{1999-01-01 00:00:00 +0000}'; do
    baseline "$spec"
  done
)

initial deletion
(
  cd deletion
  # HEAD retains a deletion entry between two unrelated commits.
  GIT_COMMITTER_DATE='2000-01-02 00:00:00 +0000' git update-ref -d refs/heads/main
  GIT_COMMITTER_DATE='2000-01-03 00:00:00 +0000' git commit -q --allow-empty -m two
  for spec in 'HEAD@{0}' 'HEAD@{1}' 'HEAD@{1}^{/two}' 'HEAD@{2}' \
              'HEAD@{2000-01-02 12:00:00 +0000}' 'HEAD@{1999-01-01 00:00:00 +0000}'; do
    baseline "$spec"
  done
)

initial stale
(
  cd stale
  # Symbolic refs keep their own log while their target moves later.
  GIT_COMMITTER_DATE='2000-01-01 12:00:00 +0000' git symbolic-ref refs/heads/alias refs/heads/main
  git update-ref refs/remotes/origin/main HEAD
  GIT_COMMITTER_DATE='2000-01-01 12:00:00 +0000' git symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/main
  GIT_COMMITTER_DATE='2000-01-02 00:00:00 +0000' git commit -q --allow-empty -m two
  git update-ref refs/remotes/origin/main HEAD
  for name in alias origin; do
    for query in 0 1 now '2030-01-01 00:00:00 +0000' '2000-01-01 12:00:00 +0000' '1999-01-01 00:00:00 +0000'; do
      baseline "$name@{$query}"
    done
  done
)

initial dangling
(
  cd dangling
  # A reflog survives even when its symbolic target no longer exists.
  git symbolic-ref refs/remotes/origin/HEAD refs/heads/main
  git symbolic-ref refs/remotes/origin/HEAD refs/heads/missing
  git symbolic-ref HEAD refs/heads/missing
  for name in HEAD origin; do
    for query in 0 1 now '2030-01-01 00:00:00 +0000'; do
      baseline "$name@{$query}"
    done
  done
)

git init -q -b main null-log
(
  cd null-log
  # Keep a null -> null entry for an unborn HEAD, even when checkout writes no log.
  git checkout -q -b missing
  null_id=$(git hash-object --stdin </dev/null)
  null_id=${null_id//?/0}
  mkdir -p .git/logs
  printf '%s %s committer <committer@example.com> 946684800 +0000\tcheckout: moving from main to missing\n' \
    "$null_id" "$null_id" >.git/logs/HEAD
  read -r previous_id new_id _ <.git/logs/HEAD
  test "$previous_id" = "$null_id" && test "$new_id" = "$null_id"
  baseline 'HEAD@{0}'
  baseline 'HEAD@{now}'
)
