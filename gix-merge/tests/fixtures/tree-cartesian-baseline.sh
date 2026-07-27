#!/usr/bin/env bash
set -eu -o pipefail

# This is a complete Cartesian product within a deliberately finite model:
#
# * one merge base;
# * one regular file whose identity can be modified, deleted, or renamed;
# * one occupied rename destination;
# * two initially free destinations;
# * at most one atomic operation per side;
# * exact rename detection and the built-in text merge driver.
#
# Modes, symlinks, submodules, copies, attributes, recursive merge bases, and
# multiple simultaneous operations are separate dimensions. Including them here
# would obscure the state-machine coverage with a mostly redundant cross product.
#
# The atomic states are:
#
# * keep, modify, or delete the source;
# * rename it unchanged to either free destination;
# * rename it with a content change to `free-a`;
# * add an unrelated file at either free destination;
# * modify or delete the occupied destination;
# * replace the occupied destination with the source, unchanged or modified;
# * replace the source file with a directory and child;
# * rename the source away and then create a directory and child at its old path.
#
# `free-b` exists to distinguish same-destination from different-destination
# interactions. Repeating the modified-rename state for `free-b` would only
# duplicate `free-a` cases under a path-name substitution.
operations=(
  keep
  modify-source
  delete-source
  rename-free-a
  rename-free-b
  rename-modify-free-a
  add-free-a
  add-free-b
  modify-occupied
  delete-occupied
  replace-occupied
  replace-modify-occupied
  file-to-dir
  rename-and-file-to-dir
)

function tick () {
  if test -z "${tick+set}"
  then
    tick=1112911993
  else
    tick=$((tick + 60))
  fi
  GIT_COMMITTER_DATE="$tick -0700"
  GIT_AUTHOR_DATE="$tick -0700"
  export GIT_COMMITTER_DATE GIT_AUTHOR_DATE
}

function apply_operation () {
  local side=${1:?side}
  local operation=${2:?operation}
  local payload="payload-${side}-${operation}"

  case "$operation" in
  keep)
    ;;
  modify-source)
    echo "$payload" >>source
    ;;
  delete-source)
    git rm source
    ;;
  rename-free-a)
    git mv source free-a
    ;;
  rename-free-b)
    git mv source free-b
    ;;
  rename-modify-free-a)
    echo "$payload" >>source
    git mv source free-a
    ;;
  add-free-a)
    echo "$payload" >free-a
    ;;
  add-free-b)
    echo "$payload" >free-b
    ;;
  modify-occupied)
    echo "$payload" >>occupied
    ;;
  delete-occupied)
    git rm occupied
    ;;
  replace-occupied)
    git rm occupied
    git mv source occupied
    ;;
  replace-modify-occupied)
    git rm occupied
    echo "$payload" >>source
    git mv source occupied
    ;;
  file-to-dir)
    git rm source
    mkdir source
    echo "$payload" >source/child
    ;;
  rename-and-file-to-dir)
    git mv source moved
    mkdir source
    echo "$payload" >source/child
    ;;
  *)
    echo "unknown operation: $operation" >&2
    return 1
    ;;
  esac
}

git init --object-format=sha1 matrix
(cd matrix
  git config user.name "Cartesian Merge Baseline"
  git config user.email "baseline@example.com"

  printf '%s\n' source source source source source >source
  printf '%s\n' occupied occupied occupied occupied occupied >occupied
  git add .
  tick
  git commit -m "base"
  git branch base
  git rev-parse HEAD^{tree} >.git/base.tree

  # Make a side-specific commit for every state. Content-changing states include
  # a unique payload so the Rust test can detect silently discarded changes.
  for side in A B
  do
    for operation in "${operations[@]}"
    do
      git checkout --detach --force base
      apply_operation "$side" "$operation"
      git add -A
      tick
      git commit --allow-empty -m "$side: $operation"
      git branch "$side-$operation"
    done
  done

  : >cases.tsv
  for ((left = 0; left < ${#operations[@]}; left++))
  do
    for ((right = left; right < ${#operations[@]}; right++))
    do
      left_operation=${operations[$left]}
      right_operation=${operations[$right]}
      case_name="${left_operation}--${right_operation}"
      forward_file="${case_name}-A-B.merge-info"
      reverse_file="${case_name}-B-A.merge-info"

      if git merge-tree -z --write-tree \
        "A-$left_operation" "B-$right_operation" >".git/$forward_file"
      then
        forward_status=clean
      else
        forward_status=conflicted
      fi
      if git merge-tree -z --write-tree \
        "B-$right_operation" "A-$left_operation" >".git/$reverse_file"
      then
        reverse_status=clean
      else
        reverse_status=conflicted
      fi

      printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$left_operation" \
        "$right_operation" \
        "$(git rev-parse "A-$left_operation")" \
        "$(git rev-parse "B-$right_operation")" \
        "$forward_file" \
        "$forward_status" \
        "$reverse_file" \
        "$reverse_status" >>cases.tsv
    done
  done
)
