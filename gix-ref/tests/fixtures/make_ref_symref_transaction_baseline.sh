#!/usr/bin/env bash
set -eu -o pipefail

# Unconstrained no-deref edits can repair a symbolic ref whose target cannot
# resolve. Expected symbolic targets require resolution; unresolvable new
# targets are accepted without appending a reflog entry.
scenario=$1
corruption=$2
storage=$3
operation=$4

git init -q -b source
git commit -q --allow-empty -m initial
git branch destination
git commit -q --allow-empty -m second
git symbolic-ref -m original refs/heads/alias refs/heads/source

case "$storage" in
  loose) ;;
  packed) git pack-refs --all --prune ;;
  both) git pack-refs --all --no-prune ;;
  *) echo "unknown storage: $storage" >&2; exit 1 ;;
esac

case "$corruption" in
  malformed) printf 'not an object id\n' >.git/refs/heads/broken ;;
  cycle)
    git symbolic-ref refs/heads/broken refs/heads/cycle
    git symbolic-ref refs/heads/cycle refs/heads/broken
    ;;
  *) echo "unknown corruption: $corruption" >&2; exit 1 ;;
esac
if test "$scenario" != new-target; then
  git symbolic-ref refs/heads/alias refs/heads/broken
fi
if test "$scenario" = delete-head; then
  git symbolic-ref HEAD refs/heads/alias
fi

status=0
case "$operation" in
  setup) ;;
  git)
    case "$scenario" in
      delete|delete-head) git update-ref --no-deref -m change -d refs/heads/alias || status=$? ;;
      delete-expected)
        printf 'symref-delete refs/heads/alias refs/heads/broken\n' |
          git update-ref --no-deref --stdin -m change || status=$?
        ;;
      replace-direct)
        git update-ref --no-deref -m change refs/heads/alias "$(git rev-parse refs/heads/destination)" || status=$?
        ;;
      replace-symbolic|replace-symbolic-expected)
        expected=
        if test "$scenario" = replace-symbolic-expected; then
          expected=' ref refs/heads/broken'
        fi
        printf 'symref-update refs/heads/alias refs/heads/destination%s\n' "$expected" |
          git update-ref --no-deref --stdin -m change || status=$?
        ;;
      new-target)
        printf 'symref-update refs/heads/alias refs/heads/broken\n' |
          git update-ref --no-deref --stdin -m change || status=$?
        ;;
      *) echo "unknown scenario: $scenario" >&2; exit 1 ;;
    esac
    ;;
  *) echo "unknown operation: $operation" >&2; exit 1 ;;
esac
printf '%s\n' "$status" >status
