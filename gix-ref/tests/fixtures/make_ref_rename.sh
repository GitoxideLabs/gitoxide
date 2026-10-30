#!/usr/bin/env bash
set -eu -o pipefail

# Cache one repository per combination; Rust copies `setup` and observes the
# corresponding host-Git result without ever invoking Git itself.
source_branch=$1
destination_branch=$2
storage=$3
state=$4
head=$5
operation=$6
source_ref="refs/heads/$source_branch"
destination_ref="refs/heads/$destination_branch"

git init -q -b "$source_branch"
git commit -q --allow-empty -m initial
initial_oid=$(git rev-parse HEAD)
git commit -q --allow-empty -m second
source_oid=$(git rev-parse HEAD)
git branch other "$initial_oid"
git checkout -q --detach

case "$state" in
  plain) ;;
  destination|no-source-log-destination)
    git branch "$destination_branch" "$initial_oid"
    ;;
  destination-same-target) git branch "$destination_branch" "$source_oid" ;;
  malformed-packed-destination*|no-source-log-malformed-packed-destination*)
    case "$state" in
      *same-target) git branch "$destination_branch" "$source_oid" ;;
      *) git branch "$destination_branch" "$initial_oid" ;;
    esac
    ;;
  symbolic-destination|cyclic-destination|malformed-target-destination)
    git symbolic-ref -m destination "$destination_ref" refs/heads/other
    ;;
  broken-destination)
    git symbolic-ref "$destination_ref" refs/heads/missing
    ;;
  stale-destination-log|no-source-log-stale-destination-log)
    mkdir -p ".git/logs/$(dirname "$destination_ref")"
    cp .git/logs/refs/heads/other ".git/logs/$destination_ref"
    ;;
  no-source-log|no-logs) ;;
  empty-source-log-directory|empty-source-log-directory-destination|empty-source-log-directory-stale-destination-log|nonempty-source-log-directory)
    case "$state" in
      empty-source-log-directory-destination) git branch "$destination_branch" "$initial_oid" ;;
      empty-source-log-directory-stale-destination-log)
        mkdir -p ".git/logs/$(dirname "$destination_ref")"
        cp .git/logs/refs/heads/other ".git/logs/$destination_ref"
        ;;
    esac
    rm ".git/logs/$source_ref"
    mkdir ".git/logs/$source_ref"
    if test "$state" = nonempty-source-log-directory; then
      cp .git/logs/refs/heads/other ".git/logs/$source_ref/child"
    fi
    ;;
  malformed-destination)
    mkdir -p ".git/$(dirname "$destination_ref")"
    printf 'broken\n' >".git/$destination_ref"
    ;;
  malformed-parent|malformed-child|malformed-unrelated)
    case "$state" in
      malformed-parent) malformed_ref=$(dirname "$destination_ref") ;;
      malformed-child) malformed_ref="$destination_ref/blocker" ;;
      malformed-unrelated) malformed_ref=refs/heads/unrelated ;;
    esac
    mkdir -p ".git/$(dirname "$malformed_ref")"
    printf 'broken\n' >".git/$malformed_ref"
    ;;
  opaque-source-log)
    # Reflog movement preserves bytes, even when old entries cannot be parsed.
    printf 'opaque historical data\n' >>".git/logs/$source_ref"
    ;;
  tag|tag-log)
    git update-ref -d "$source_ref"
    source_ref="refs/tags/$source_branch"
    destination_ref="refs/tags/$destination_branch"
    if test "$state" = tag-log; then
      git update-ref --create-reflog -m initial "$source_ref" "$initial_oid"
    fi
    git update-ref -m second "$source_ref" "$source_oid"
    ;;
  worktree)
    # Rust opens the copied common/private stores directly, so the worktree's
    # checkout pointer does not need to be rewritten after copying this fixture.
    git worktree add -q linked "$source_branch"
    ;;
  missing-source)
    git update-ref -d "$source_ref"
    ;;
  symbolic-source)
    git symbolic-ref -m source "$source_ref" refs/heads/other
    ;;
  broken-source)
    git symbolic-ref "$source_ref" refs/heads/missing
    ;;
  sibling)
    git branch "$(dirname "$source_branch")/sibling" "$initial_oid"
    ;;
  destination-parent)
    git branch "$(dirname "$destination_branch")" "$initial_oid"
    ;;
  destination-child)
    git branch "$destination_branch/child" "$initial_oid"
    ;;
  source-lock|destination-lock|packed-lock|head-lock) ;;
  reflog-directory)
    mkdir -p ".git/logs/$destination_ref"
    cp .git/logs/refs/heads/other ".git/logs/$destination_ref/child"
    ;;
  publication-blocker)
    # Invalid ref names are not namespace conflicts, but this file keeps the
    # destination directory nonempty until the final reference publication.
    mkdir -p ".git/$destination_ref"
    printf 'preserve this blocker\n' >".git/$destination_ref/.keep"
    ;;
  namespace)
    # Real and namespaced refs deliberately disagree, exposing accidental access
    # to the outer store during rename, reflog movement, or packed-ref deletion.
    prefix=refs/namespaces/scope/refs/namespaces/nested/
    git update-ref --create-reflog -m initial "$prefix$source_ref" "$initial_oid"
    git update-ref -m second "$prefix$source_ref" "$source_oid"
    git update-ref -m outer "$source_ref" "$initial_oid"
    ;;
  *) echo "unknown state: $state" >&2; exit 1 ;;
esac

case "$storage" in
  loose) ;;
  packed) git pack-refs --all --prune ;;
  both)
    git pack-refs --all --no-prune
    # Keep a different packed target so deleting only the loose source cannot
    # accidentally look correct when the stale packed value becomes visible.
    sed "s|^$source_oid $source_ref\$|$initial_oid $source_ref|" .git/packed-refs >packed-refs
    mv packed-refs .git/packed-refs
    ;;
  *) echo "unknown storage: $storage" >&2; exit 1 ;;
esac

case "$state" in
  cyclic-destination|malformed-target-destination)
    # The destination itself is a valid symref even if following it fails.
    git symbolic-ref "$destination_ref" refs/heads/broken
    if test "$state" = cyclic-destination; then
      git symbolic-ref refs/heads/broken refs/heads/cycle
      git symbolic-ref refs/heads/cycle refs/heads/broken
    else
      printf 'broken\n' >.git/refs/heads/broken
    fi
    ;;
  malformed-packed-destination*|no-source-log-malformed-packed-destination*)
    # Hide the valid packed destination with a malformed loose reference; Git
    # replaces only that loose file and retains the packed target underneath.
    mkdir -p ".git/$(dirname "$destination_ref")"
    printf 'broken\n' >".git/$destination_ref"
    ;;
esac

case "$state" in
  no-source-log*) rm -f ".git/logs/$source_ref" ;;
  no-logs) rm -rf .git/logs ;;
  source-lock) printf 'preserve this lock\n' >".git/$source_ref.lock" ;;
  destination-lock)
    mkdir -p ".git/$(dirname "$destination_ref")"
    printf 'preserve this lock\n' >".git/$destination_ref.lock"
    ;;
  packed-lock) printf 'preserve this lock\n' >.git/packed-refs.lock ;;
esac

case "$head" in
  detached) ;;
  source) git symbolic-ref HEAD "$source_ref" ;;
  destination) git symbolic-ref HEAD "$destination_ref" ;;
  *) echo "unknown HEAD: $head" >&2; exit 1 ;;
esac
if test "$state" = head-lock; then
  printf 'preserve this lock\n' >.git/HEAD.lock
fi

status=0
case "$operation" in
  setup) ;;
  git)
    # Detached HEAD makes porcelain branch -M equivalent to refs_rename_ref():
    # no branch configuration or worktree HEAD needs to be changed here.
    test "$head" = detached
    git branch -M "$source_branch" "$destination_branch" >stdout 2>stderr || status=$?
    ;;
  *) echo "unknown operation: $operation" >&2; exit 1 ;;
esac
printf '%s\n' "$status" >status
