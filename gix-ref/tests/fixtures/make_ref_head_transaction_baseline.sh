#!/usr/bin/env bash
set -eu -o pipefail

# Compare direct branch edits and HEAD symref edits, including unborn targets
# and no-ops, without retargeting branch configuration or other worktrees.
mode=$1
scenario=$2
storage=$3
operation=$4

git init -q -b source
git commit -q --allow-empty -m initial
destination_oid=$(git rev-parse HEAD)
git commit -q --allow-empty -m second
source_oid=$(git rev-parse HEAD)
git branch destination "$destination_oid"

case "$storage" in
  loose) ;;
  packed) git pack-refs --all --prune ;;
  both) git pack-refs --all --no-prune ;;
  *) echo "unknown storage: $storage" >&2; exit 1 ;;
esac

head=refs/heads/source
case "$mode:$scenario" in
  symref:existing) new_target=refs/heads/destination ;;
  symref-referent:existing) new_target=refs/heads/destination ;;
  symref-referent:destination-unborn) new_target=refs/heads/unborn-destination ;;
  symref:source-unborn)
    head=refs/heads/unborn-source
    new_target=refs/heads/destination
    ;;
  symref:destination-unborn) new_target=refs/heads/unborn-destination ;;
  symref:both-unborn)
    head=refs/heads/unborn-source
    new_target=refs/heads/unborn-destination
    ;;
  symref:same-existing) new_target=$head ;;
  symref:same-unborn)
    head=refs/heads/unborn-source
    new_target=$head
    ;;
  direct:changed) new_target=$destination_oid ;;
  direct:no-op) new_target=$source_oid ;;
  direct:unrelated-changed)
    head=refs/heads/destination
    new_target=$destination_oid
    ;;
  direct:unrelated-no-op)
    head=refs/heads/destination
    new_target=$source_oid
    ;;
  *) echo "unknown operation case: $mode:$scenario" >&2; exit 1 ;;
esac
git symbolic-ref HEAD "$head"
printf '%s\n' "$new_target" >new-target

status=0
case "$operation" in
  setup) ;;
  git)
    case "$mode" in
      symref|symref-referent)
        updated_ref=HEAD
        if test "$mode" = symref-referent; then
          updated_ref=refs/heads/source
        fi
        printf 'start\nsymref-update %s %s\nprepare\ncommit\n' "$updated_ref" "$new_target" |
          git update-ref --no-deref --stdin -m change || status=$?
        ;;
      direct)
        git update-ref -m change refs/heads/source "$new_target" || status=$?
        ;;
    esac
    ;;
  *) echo "unknown operation: $operation" >&2; exit 1 ;;
esac
printf '%s\n' "$status" >status
