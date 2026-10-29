#!/usr/bin/env bash
set -eu -o pipefail

# Common Git-clean cases are kept separate from conservative repository/precious protection.
git init -q simple
(
  cd simple
  mkdir tracked-dir
  printf 'tracked\n' >tracked-dir/tracked
  printf 'ignored-file\nignored-dir/\nmixed/ignored\n' >.gitignore
  git add .
  git commit -qm initial
  mkdir -p untracked-dir ignored-dir empty-dir mixed
  printf 'untracked\n' >untracked
  printf 'untracked\n' >tracked-dir/untracked
  printf 'untracked\n' >untracked-dir/file
  printf 'untracked\n' >untracked-dir.sibling
  printf 'ignored\n' >ignored-file
  printf 'ignored\n' >ignored-dir/file
  printf 'ignored\n' >mixed/ignored
  printf 'untracked\n' >mixed/untracked
)

git init -q protected
mkdir outside
printf 'outside\n' >outside/sentinel
(
  cd protected
  git config gitoxide.parsePrecious true
  mkdir tracked-dir outer
  printf 'tracked\n' >tracked-file
  printf 'tracked\n' >outer/tracked
  printf 'tracked\n' >tracked-dir/tracked
  printf 'ignored/\n$precious\n$precious-dir/keep\n' >.gitignore
  git add .
  git commit -qm initial
  mkdir -p ignored precious-dir
  git update-index --assume-unchanged tracked-dir/tracked
  git init -q nested
  git init -q --bare bare
  git init -q ignored/nested
  git init -q --bare ignored/bare
  git worktree add -q --detach ignored/linked HEAD
  printf 'precious\n' >precious
  printf 'precious\n' >precious-dir/keep
  printf 'remove\n' >precious-dir/trash
  printf 'remove\n' >ignored/trash
  printf 'remove\n' >untracked
  # A tracked path whose disk type changed still belongs to the user.
  rm tracked-file
  mkdir tracked-file
  printf 'keep\n' >tracked-file/child
  rm outer/tracked
  mkdir outer/tracked
  printf 'keep\n' >outer/tracked/child
  printf 'remove\n' >outer/trash
  # Traversal and deletion must never follow these links outside the worktree.
  ln -s ../outside outside-link 2>/dev/null || true
  if ln -s ../outside tracked-dir-link 2>/dev/null; then
    rm -r tracked-dir
    mv tracked-dir-link tracked-dir
  fi
)
