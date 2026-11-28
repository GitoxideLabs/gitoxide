#!/usr/bin/env bash
set -eu -o pipefail

# Equivalent reference-store data, created through Git rather than ref-file writes.
# Git 2.45 or newer is needed to regenerate the reftable variant. Tests use the
# packaged archives because reftable filenames vary between otherwise equal runs.
make_repository() (
  mode=$1
  ref_format=$mode
  if test "$mode" = packed; then
    ref_format=files
  fi

  mkdir "$mode"
  cd "$mode"
  git init -q --initial-branch=main --ref-format="$ref_format" repo
  cd repo
  git config core.logAllRefUpdates true
  git commit -q --allow-empty -m first
  first_commit_id=$(git rev-parse HEAD)
  git commit -q --allow-empty -m second
  second_commit_id=$(git rev-parse HEAD)

  # Distinct targets make tag/branch lookup precedence observable.
  git branch shared "$first_commit_id"
  git tag shared "$second_commit_id"
  git tag -m 'annotated tag' annotated "$first_commit_id"
  git symbolic-ref refs/heads/alias refs/heads/main
  git update-ref refs/remotes/origin/main "$second_commit_id"
  git symbolic-ref refs/remotes/origin/HEAD refs/remotes/origin/main
  git update-ref refs/prefix/one "$first_commit_id"
  git update-ref refs/prefix/nested/two "$second_commit_id"
  git update-ref refs/prefix-suffix "$first_commit_id"
  git update-ref refs/namespaces/test/refs/heads/main "$first_commit_id"
  git update-ref refs/namespaces/test/refs/tags/tag "$second_commit_id"
  git update-ref --create-reflog -m created refs/heads/topic "$first_commit_id"
  git update-ref -m advanced refs/heads/topic "$second_commit_id" "$first_commit_id"

  git update-ref refs/worktree/private "$second_commit_id"
  git worktree add -q --detach ../linked "$first_commit_id"
  # Keep only repository-topology pointers relative so archives can be relocated.
  printf 'gitdir: ../repo/.git/worktrees/linked\n' > ../linked/.git
  printf '../../../../linked/.git\n' > .git/worktrees/linked/gitdir
  git -C ../linked update-ref refs/worktree/private "$first_commit_id"

  if test "$mode" = packed; then
    git pack-refs --all --prune
  fi

  printf '%s\n' "$first_commit_id" > ../first.id
  printf '%s\n' "$second_commit_id" > ../second.id
  git rev-parse refs/tags/annotated > ../annotated.id
  git symbolic-ref HEAD > ../head
  git -C ../linked rev-parse HEAD > ../linked-head.id
  git for-each-ref --format='%(refname)' > ../refs
  git -C ../linked for-each-ref --format='%(refname)' > ../linked-refs
  git reflog show --format='%H %gs' refs/heads/topic > ../topic.log
)

make_repository files
make_repository packed
make_repository reftable
