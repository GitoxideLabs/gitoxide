#!/usr/bin/env bash
set -eu -o pipefail

# Filesystem and reflog collisions must not substitute for reference-name checks:
# packed refs and new parent/child pairs can conflict without either kind of file.
first_ref=$1
second_ref=$2
state=$3
storage=$4
namespace=$5
order=$6
operation=$7

git init -q -b main
git commit -q --allow-empty -m initial
commit_oid=$(git rev-parse HEAD)
prefix=
if test "$namespace" != '-'; then
  IFS=/ read -r -a components <<<"$namespace"
  for component in "${components[@]}"; do
    prefix="${prefix}refs/namespaces/$component/"
  done
fi
first="$prefix$first_ref"
second="$prefix$second_ref"
case "$state" in
  existing) git update-ref "$first" "$commit_oid" ;;
  new) ;;
  *) echo "unknown state: $state" >&2; exit 1 ;;
esac
case "$storage" in
  loose) ;;
  packed) git pack-refs --all --prune ;;
  both) git pack-refs --all --no-prune ;;
  *) echo "unknown storage: $storage" >&2; exit 1 ;;
esac
git config core.logAllRefUpdates false
rm -rf .git/logs
printf '%s\n' "$commit_oid" >commit-id

status=0
case "$operation" in
  setup) ;;
  git)
    first_edit="create $first $commit_oid"
    second_edit="create $second $commit_oid"
    if test "$state" = existing; then
      printf '%s\n' "$second_edit"
    elif test "$order" = first-first; then
      printf '%s\n%s\n' "$first_edit" "$second_edit"
    else
      printf '%s\n%s\n' "$second_edit" "$first_edit"
    fi >edits
    git update-ref --stdin -m create <edits >stdout 2>stderr || status=$?
    ;;
  *) echo "unknown operation: $operation" >&2; exit 1 ;;
esac
printf '%s\n' "$status" >status
