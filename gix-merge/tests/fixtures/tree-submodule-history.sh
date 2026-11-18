#!/usr/bin/env bash
set -eu -o pipefail
export GIT_AUTHOR_DATE='1112911993 -0700' GIT_COMMITTER_DATE='1112911993 -0700'

# The gitlinks below refer to real commits in a separate repository. Both sides
# can advance from the recorded base, rewind behind it, or name unavailable
# history. Keeping the submodule initialized, deinitialized, and in old form
# exercises repository discovery independently of the selected gitlink value.
git init history
empty_tree_id=$(git -C history mktree </dev/null)
root_commit_id=$(git -C history commit-tree "$empty_tree_id" -m root)
base_commit_id=$(git -C history commit-tree "$empty_tree_id" -p "$root_commit_id" -m base)
ahead_commit_id=$(git -C history commit-tree "$empty_tree_id" -p "$base_commit_id" -m ahead)
further_commit_id=$(git -C history commit-tree "$empty_tree_id" -p "$ahead_commit_id" -m further)
other_commit_id=$(git -C history commit-tree "$empty_tree_id" -p "$base_commit_id" -m other)
merge_commit_id=$(git -C history commit-tree "$empty_tree_id" -p "$further_commit_id" -p "$other_commit_id" -m merge)
unrelated_commit_id=$(git -C history commit-tree "$empty_tree_id" -m unrelated)
missing_commit_id=$(git -C history cat-file commit "$base_commit_id" | sed '$s/base/missing/' | git -C history hash-object -t commit --stdin)
git -C history update-ref refs/heads/main "$merge_commit_id"
git -C history update-ref refs/heads/unrelated "$unrelated_commit_id"
git -C history symbolic-ref HEAD refs/heads/main
printf '%s\n' "$root_commit_id" "$base_commit_id" "$ahead_commit_id" "$further_commit_id" "$other_commit_id" "$merge_commit_id" "$unrelated_commit_id" "$missing_commit_id" >history/commits

: >cases
for layout in forward reverse divergent rewind-one rewind-both missing-tip missing-base wrong-kind-tip wrong-kind-base corrupt-parent missing-tip-corrupt-parent unavailable deinitialized old-form unregistered add-add type-file-base type-link-base equal equal-malformed malformed-unavailable one-side rename recursive-forward recursive-divergent; do
  git init "$layout"
  git -C "$layout" submodule add ../history module
  (
    cd "$layout"
    modules_blob_id=$(git rev-parse :.gitmodules)
    renamed_modules_blob_id=$(sed 's/path = module/path = renamed/' .gitmodules | git hash-object -w --stdin)
    tree () {
      local gitlink_commit_id=$1 path=${2-module} config_blob_id=${3-$modules_blob_id} mode=${4-160000}
      git read-tree --empty
      if test -n "$config_blob_id"; then
        git update-index --add --cacheinfo "100644,$config_blob_id,.gitmodules"
      fi
      if test -n "$gitlink_commit_id"; then
        git update-index --add --cacheinfo "$mode,$gitlink_commit_id,$path"
      fi
      git write-tree
    }
    old_commit_id=$base_commit_id
    our_commit_id=$ahead_commit_id
    their_commit_id=$further_commit_id
    case "$layout" in
      reverse) our_commit_id=$further_commit_id; their_commit_id=$ahead_commit_id ;;
      divergent) their_commit_id=$other_commit_id ;;
      rewind-one) old_commit_id=$ahead_commit_id; our_commit_id=$base_commit_id ;;
      rewind-both) old_commit_id=$ahead_commit_id; our_commit_id=$base_commit_id; their_commit_id=$root_commit_id ;;
      missing-tip) their_commit_id=$missing_commit_id ;;
      missing-base) old_commit_id=$missing_commit_id ;;
      wrong-kind-tip|wrong-kind-base)
        # Existing objects of another kind are not usable submodule commits.
        blob_id=$(printf 'not a commit\n' | git --git-dir=.git/modules/module hash-object -w --stdin)
        if test "$layout" = wrong-kind-tip; then
          their_commit_id=$blob_id
        else
          old_commit_id=$blob_id
        fi
        ;;
      corrupt-parent|missing-tip-corrupt-parent)
        # The named tip exists, but one of its parents is absent. Availability
        # must be checked for all inputs before walking this corrupt ancestry.
        our_commit_id=$(git -C ../history cat-file commit "$ahead_commit_id" |
          sed -e "s/^parent $base_commit_id$/parent $missing_commit_id/" -e '$s/ahead/corrupt/' |
          git --git-dir=.git/modules/module hash-object -t commit -w --stdin)
        if test "$layout" = missing-tip-corrupt-parent; then their_commit_id=$missing_commit_id; fi
        ;;
      add-add) old_commit_id= ;;
      equal|equal-malformed) their_commit_id=$our_commit_id ;;
      one-side) our_commit_id=$old_commit_id ;;
      rename) our_commit_id=$old_commit_id ;;
      unregistered) modules_blob_id= ;;
    esac
    base_tree_id=$(tree "$old_commit_id")
    if test "$layout" = type-file-base || test "$layout" = type-link-base; then
      base_blob_id=$(printf 'old\n' | git hash-object -w --stdin)
      mode=100644
      if test "$layout" = type-link-base; then mode=120000; fi
      base_tree_id=$(tree "$base_blob_id" module "$modules_blob_id" "$mode")
    fi
    our_tree_id=$(tree "$our_commit_id")
    their_tree_id=$(tree "$their_commit_id")
    if test "$layout" = rename; then
      our_tree_id=$(tree "$our_commit_id" renamed "$renamed_modules_blob_id")
      mv module renamed
      sed 's/path = module/path = renamed/' .gitmodules >renamed.gitmodules
      mv renamed.gitmodules .gitmodules
      git config --file .git/modules/module/config core.worktree ../../../renamed
    fi
    base_super_commit_id=$(git commit-tree "$base_tree_id" -m base)
    if test "$layout" = recursive-forward || test "$layout" = recursive-divergent; then
      # Two incomparable superproject bases must first merge their gitlinks.
      # A forward virtual-base merge selects `further`; a divergent one keeps
      # the original gitlink. The tips then expose a wrong virtual selection.
      left_tree_id=$(tree "$ahead_commit_id")
      right_commit_id=$further_commit_id
      if test "$layout" = recursive-divergent; then right_commit_id=$other_commit_id; fi
      right_tree_id=$(tree "$right_commit_id")
      left_super_commit_id=$(git commit-tree "$left_tree_id" -p "$base_super_commit_id" -m left)
      right_super_commit_id=$(git commit-tree "$right_tree_id" -p "$base_super_commit_id" -m right)
      if test "$layout" = recursive-forward; then
        our_tree_id=$(tree "$ahead_commit_id")
        their_tree_id=$(tree "$base_commit_id")
      else
        our_tree_id=$(tree "$base_commit_id")
        their_tree_id=$(tree "$ahead_commit_id")
      fi
      our_super_commit_id=$(git commit-tree "$our_tree_id" -p "$left_super_commit_id" -p "$right_super_commit_id" -m ours)
      their_super_commit_id=$(git commit-tree "$their_tree_id" -p "$right_super_commit_id" -p "$left_super_commit_id" -m theirs)
    else
      our_super_commit_id=$(git commit-tree "$our_tree_id" -p "$base_super_commit_id" -m ours)
      their_super_commit_id=$(git commit-tree "$their_tree_id" -p "$base_super_commit_id" -m theirs)
    fi
    git update-ref refs/heads/A "$our_super_commit_id"
    git update-ref refs/heads/B "$their_super_commit_id"
    git symbolic-ref HEAD refs/heads/A
    printf '%s\n' "$base_tree_id" "$our_tree_id" "$their_tree_id" >trees
    printf '%s\n' "$base_super_commit_id" "$our_super_commit_id" "$their_super_commit_id" >commits
    if test "$layout" = unavailable || test "$layout" = malformed-unavailable; then
      rm -rf module .git/modules/module
      mkdir module
    elif test "$layout" = deinitialized; then
      git read-tree "$our_tree_id"
      git submodule deinit -f module
    elif test "$layout" = old-form; then
      rm module/.git
      mv .git/modules/module module/.git
      git config --file module/.git/config --unset core.worktree
    elif test "$layout" = unregistered; then
      rm .gitmodules
    fi
    if test "$layout" = equal-malformed || test "$layout" = malformed-unavailable; then
      # Discovery is lazy: malformed local metadata is irrelevant to a trivial
      # merge, but must surface if it is needed to find unavailable history.
      printf 'invalid submodule metadata\n' >.gitmodules
    fi
    for direction in A-B B-A; do
      current=${direction%-*}
      other=${direction#*-}
      status=0
      git merge-tree -z --write-tree "$current" "$other" >"$direction.merge-info" || status=$?
      if test "$layout" != corrupt-parent && test "$layout" != malformed-unavailable; then
        test "$status" -le 1
      fi
      echo "$status" >"$direction.status"
      if test "$status" -le 1; then
        IFS= read -r -d '' tree_id <"$direction.merge-info" || test -n "$tree_id"
        printf '%s\n' "$tree_id" >"$direction.tree"
      fi
    done
  )
  echo "$layout" >>cases
done
