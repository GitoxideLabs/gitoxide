#!/usr/bin/env bash
set -eu -o pipefail

# With no worktree or indexed .gitmodules, submodule configuration must come from
# the selected namespace's HEAD. Only that tree ignores the changed child.
git init -q --initial-branch=main superproject
(
  cd superproject
  git init -q --initial-branch=main d
  echo initial >d/file
  git -C d add file
  git -C d commit -qm initial

  git config -f .gitmodules submodule.d.path d
  git config -f .gitmodules submodule.d.url ./d
  git config -f .gitmodules submodule.d.ignore none
  git add .gitmodules d
  git commit -qm 'record gitlink'
  ordinary_commit=$(git rev-parse HEAD)

  git config -f .gitmodules submodule.d.ignore all
  git add .gitmodules
  git commit -qm 'ignore child in namespace'
  git update-ref refs/namespaces/status/refs/heads/main HEAD
  git symbolic-ref refs/namespaces/status/HEAD refs/heads/main
  git reset -q --hard "$ordinary_commit"

  git update-index --force-remove .gitmodules
  rm .gitmodules
  git -C d commit -q --allow-empty -m 'advance child HEAD'
)
