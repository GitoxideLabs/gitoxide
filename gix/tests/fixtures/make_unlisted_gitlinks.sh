#!/usr/bin/env bash
set -eu -o pipefail

# Git tracks embedded repositories as gitlinks even without a matching .gitmodules
# entry. Commit changes, tracked changes and untracked files all affect their status.
for modules in absent unrelated listed; do
  for state in clean commit modified staged untracked gitfile gitfile-worktree-inference gitfile-worktree-config gitfile-empty-worktree-config uninitialized; do
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
        gitfile-worktree-inference)
          # The gitfile supplies the checkout location: the relocated repository has
          # no core.worktree, and its old checkout stays clean while d is modified.
          mv d .git/target
          mkdir d
          cp .git/target/a d/a
          echo changed >>d/a
          echo 'gitdir: ../.git/target/.git' >d/.git
          ;;
        gitfile-worktree-config|gitfile-empty-worktree-config)
          # Keep d clean while the configured checkout is dirty. The gitfile's `..`
          # must not cause the repository's own core.worktree to be ignored.
          mv d/.git .git/embedded
          mkdir .git/checkout
          cp d/a .git/checkout/a
          echo changed >>.git/checkout/a
          echo 'gitdir: ../.git/embedded' >d/.git
          if test "$state" = gitfile-worktree-config; then
            git config -f .git/embedded/config core.worktree ../checkout
          else
            # An empty value is invalid to gix even with a checkout .git entry present.
            git config -f .git/embedded/config core.worktree ''
          fi
          ;;
        uninitialized) rm -rf d/.git ;;
      esac
      # Empty core.worktree exercises gix's validation rather than Git status parity.
      if test "$state" != gitfile-empty-worktree-config; then
        for ignore in none untracked dirty all; do
          git status --porcelain=v1 --ignore-submodules="$ignore" >".git/status-$ignore.git"
        done
      fi
    )
  done
done

# Submodule opening retains its supplied checkout despite core.bare=true, while
# ordinary gitdir opening suppresses it. The empty core.worktree remains ignored;
# this variant inherits no Git status baselines from the validation fixture.
cp -R listed-gitfile-empty-worktree-config listed-gitfile-bare
git config -f listed-gitfile-bare/.git/embedded/config core.bare true

cp -R absent-commit global-ignore
git -C global-ignore config diff.ignoreSubmodules all
git -C global-ignore status --porcelain=v1 >global-ignore/.git/status-configured.git

# Repository opening permits a configured checkout that does not exist yet.
# Reconstructing the status provider must preserve this state without requiring it to exist.
cp -R listed-commit missing-worktree
git -C missing-worktree config diff.ignoreSubmodules all
git -C missing-worktree config core.worktree ../missing-checkout
