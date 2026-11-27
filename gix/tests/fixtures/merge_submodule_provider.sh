#!/usr/bin/env bash
set -eu -o pipefail
export GIT_AUTHOR_DATE='1112911993 -0700' GIT_COMMITTER_DATE='1112911993 -0700'

# The on-disk commit-graph must not restore parents hidden by a shallow boundary
# or replacement. Linked repositories exercise the existing discovery helpers
# both at a checkout's .git file and at a superproject's private modules path.
git init history
empty_tree_id=$(git -C history mktree </dev/null)
base_commit_id=$(git -C history commit-tree "$empty_tree_id" -m base)
ahead_commit_id=$(git -C history commit-tree "$empty_tree_id" -p "$base_commit_id" -m ahead)
further_commit_id=$(git -C history commit-tree "$empty_tree_id" -p "$ahead_commit_id" -m further)
unrelated_commit_id=$(git -C history commit-tree "$empty_tree_id" -m unrelated)
git -C history update-ref refs/heads/main "$further_commit_id"
git -C history update-ref refs/heads/unrelated "$unrelated_commit_id"
git -C history symbolic-ref HEAD refs/heads/main

# Short private-module paths avoid Git's stricter Windows Git-directory limit.
for layout in shallow replacement missing-graph-tip linked-submodule linked-super incomplete-modules corrupt-config; do
  root=$layout
  if test "$layout" = linked-super; then root=main; fi
  git init "$root"
  git -C "$root" submodule add ../history module
  (
    cd "$root"
    if test "$layout" = incomplete-modules; then
      # The unrelated section is first so a lookup must skip its missing path.
      { printf '%s\n' '[submodule "unrelated"]' '    url = ./unused'; cat .gitmodules; } >modules.tmp
      mv modules.tmp .gitmodules
      git add .gitmodules
    fi
    modules_blob_id=$(git rev-parse :.gitmodules)
    tree () {
      git read-tree --empty
      git update-index --add --cacheinfo "100644,$modules_blob_id,.gitmodules"
      git update-index --add --cacheinfo "160000,$1,module"
      git write-tree
    }
    base_tree_id=$(tree "$base_commit_id")
    our_tree_id=$(tree "$ahead_commit_id")
    their_tree_id=$(tree "$further_commit_id")
    base_super_commit_id=$(git commit-tree "$base_tree_id" -m base)
    our_super_commit_id=$(git commit-tree "$our_tree_id" -p "$base_super_commit_id" -m ours)
    their_super_commit_id=$(git commit-tree "$their_tree_id" -p "$base_super_commit_id" -m theirs)
    git update-ref refs/heads/A "$our_super_commit_id"
    git update-ref refs/heads/B "$their_super_commit_id"
    git symbolic-ref HEAD refs/heads/A
    git read-tree "$our_tree_id"
    printf '%s\n' "$base_tree_id" "$our_tree_id" "$their_tree_id" >trees
    case "$layout" in
      shallow|replacement)
        git --git-dir=.git/modules/module commit-graph write --reachable
        if test "$layout" = shallow; then
          printf '%s\n' "$ahead_commit_id" >.git/modules/module/shallow
        else
          git --git-dir=.git/modules/module update-ref "refs/replace/$ahead_commit_id" "$unrelated_commit_id"
        fi
        ;;
      missing-graph-tip)
        git --git-dir=.git/modules/module commit-graph write --reachable
        # Leave the fast-forward tip in the graph after removing its loose object.
        rm ".git/modules/module/objects/${further_commit_id:0:2}/${further_commit_id:2}"
        ;;
      linked-submodule)
        mv module primary
        git config --file .git/modules/module/config core.worktree ../../../primary
        git -C primary worktree add --detach ../module "$further_commit_id"
        # Keep both directions relative so a cached fixture can be relocated.
        printf 'gitdir: ../.git/modules/module/worktrees/module\n' >module/.git
        printf '../../../../../module/.git\n' >.git/modules/module/worktrees/module/gitdir
        ;;
      linked-super)
        git worktree add --detach ../linked-super A
        printf 'gitdir: ../main/.git/worktrees/linked-super\n' >../linked-super/.git
        printf '../../../../linked-super/.git\n' >.git/worktrees/linked-super/gitdir
        cp trees ../linked-super/trees
        git -c core.longpaths=true -C ../linked-super submodule update --init module
        # No checkout .git remains, so discovery must use this worktree's
        # private modules repository instead of the main worktree's instance.
        git -C ../linked-super submodule deinit -f module
        ;;
      incomplete-modules)
        # With no checkout, discovery has to look up the path in .gitmodules.
        git submodule deinit -f module
        ;;
      corrupt-config)
        # The checkout's gitdir file is valid, but the target repository is
        # corrupt. Neither missing-history fallback nor a merge result is safe.
        printf 'invalid repository configuration\n' >.git/modules/module/config
        ;;
    esac
  )
  (
    cd "$layout"
    for direction in A-B B-A; do
      current=${direction%-*}
      other=${direction#*-}
      status=0
      git merge-tree -z --write-tree "$current" "$other" >"$direction.merge-info" || status=$?
      echo "$status" >"$direction.status"
      if test "$layout" = corrupt-config; then
        test "$status" -gt 1
      else
        if test "$layout" = missing-graph-tip; then test "$status" -eq 1; fi
        if test "$layout" = incomplete-modules; then test "$status" -eq 0; fi
        test "$status" -le 1
        IFS= read -r -d '' tree_id <"$direction.merge-info" || test -n "$tree_id"
        printf '%s\n' "$tree_id" >"$direction.tree"
      fi
    done
  )
done
