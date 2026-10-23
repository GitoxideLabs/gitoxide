#!/usr/bin/env bash
set -eu -o pipefail

# Git tracks embedded repositories as gitlinks even without a matching .gitmodules
# entry. Commit changes, tracked changes and untracked files all affect their status.
for modules in absent unrelated listed; do
  for state in clean commit modified staged untracked gitfile uninitialized; do
    git init -q "$modules-$state"
    (
      cd "$modules-$state"
      git init -q d
      echo initial >d/a
      git -C d add a
      git -C d commit -qm initial
      case "$modules" in
        unrelated) git config -f .gitmodules submodule.other.path other ;;
        listed) git config -f .gitmodules submodule.d.path d ;;
      esac
      git add .
      git commit -qm 'record gitlink'
      case "$state" in
        commit) git -C d commit -q --allow-empty -m next ;;
        modified) echo changed >>d/a ;;
        staged) echo changed >>d/a; git -C d add a ;;
        untracked) echo new >d/untracked ;;
        gitfile)
          mv d/.git .git/embedded
          git config -f .git/embedded/config core.worktree ../../d
          echo 'gitdir: ../.git/embedded' >d/.git
          git -C d commit -q --allow-empty -m next
          ;;
        uninitialized) rm -rf d/.git ;;
      esac
      for ignore in none untracked dirty all; do
        git status --porcelain=v1 --ignore-submodules="$ignore" >".git/status-$ignore.git"
      done
    )
  done
done

cp -R absent-commit global-ignore
git -C global-ignore config diff.ignoreSubmodules all
git -C global-ignore status --porcelain=v1 >global-ignore/.git/status-configured.git
