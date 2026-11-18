#!/usr/bin/env bash
set -eu -o pipefail
export GIT_AUTHOR_DATE='1112911993 -0700' GIT_COMMITTER_DATE='1112911993 -0700'

# Two independent file identities interact at rename sources and destinations.
# Each layout is generated with regular files, executable-bit disagreement, and
# either side as a symlink. Optional additions occupy both preferred conflict
# names, once on each side. The unoccupied and regular-file variants are the
# reduced forms of the richer histories, all retained for failure minimization.
# No worktree files are needed: index entries also make symlink cases portable.
git init matrix
cd matrix
git config merge.directoryRenames true
git config merge.conflictStyle merge

tree () {
  git read-tree --empty
  while test "$#" -gt 0; do
    git update-index --add --cacheinfo "$2,$3,$1"
    shift 3
  done
  git write-tree
}

reference () {
  local ours=$1 theirs=$2 name=$3
  local status=0
  git merge-tree -z --write-tree "$ours" "$theirs" >"$name.merge-info" || status=$?
  test "$status" -le 1
  echo "$status" >".git/$ours--$theirs.status"
  printf 'matrix merge %s %s %s %s %s expected\n' \
    "$(git rev-parse "$ours")" "$ours" "$(git rev-parse "$theirs")" "$theirs" \
    "$name.merge-info" >>../baseline.cases
}

left_blob_id=$(printf '%s\n' left payload | git hash-object -w --stdin)
right_blob_id=$(printf '%s\n' right payload | git hash-object -w --stdin)
occupied_blob_id=$(printf '%s\n' unrelated | git hash-object -w --stdin)
gitlink_commit_id=$(git commit-tree "$(tree)" -m gitlink-entry)
: >../baseline.cases

for occupancy in none left right; do
  for modes in regular executable left-link right-link; do
    left_mode=100644
    right_mode=100644
    case "$modes" in
      executable) right_mode=100755 ;;
      left-link) left_mode=120000 ;;
      right-link) right_mode=120000 ;;
    esac
    for layout in same-destination nested-destinations replaced-source directory-destination crossed-sources; do
      name="$layout-$modes-$occupancy"
      ours="A-$name"
      theirs="B-$name"
      base_entries=(a "$left_mode" "$left_blob_id" b "$right_mode" "$right_blob_id")
      left_entries=(dst "$left_mode" "$left_blob_id" b "$right_mode" "$right_blob_id")
      case "$layout" in
        same-destination)
          right_entries=(a "$left_mode" "$left_blob_id" dst "$right_mode" "$right_blob_id") ;;
        nested-destinations)
          right_entries=(a "$left_mode" "$left_blob_id" dst/child "$right_mode" "$right_blob_id") ;;
        replaced-source)
          right_entries=(a/child "$right_mode" "$right_blob_id") ;;
        directory-destination)
          base_entries=(a "$left_mode" "$left_blob_id" old/child "$right_mode" "$right_blob_id")
          left_entries=(dst "$left_mode" "$left_blob_id" old/child "$right_mode" "$right_blob_id")
          right_entries=(a "$left_mode" "$left_blob_id" dst/child "$right_mode" "$right_blob_id") ;;
        crossed-sources)
          left_entries=(b "$left_mode" "$left_blob_id")
          right_entries=(a "$right_mode" "$right_blob_id") ;;
      esac
      occupied_entries=("dst~$ours" 100644 "$occupied_blob_id" "dst~$theirs" 100644 "$occupied_blob_id")
      case "$occupancy" in
        left) left_entries+=("${occupied_entries[@]}") ;;
        right) right_entries+=("${occupied_entries[@]}") ;;
      esac
      base_tree_id=$(tree "${base_entries[@]}")
      left_tree_id=$(tree "${left_entries[@]}")
      right_tree_id=$(tree "${right_entries[@]}")
      base_commit_id=$(git commit-tree "$base_tree_id" -m "$name base")
      git update-ref "refs/heads/$ours" "$(git commit-tree "$left_tree_id" -p "$base_commit_id" -m "$ours")"
      git update-ref "refs/heads/$theirs" "$(git commit-tree "$right_tree_id" -p "$base_commit_id" -m "$theirs")"
      reference "$ours" "$theirs" "$name-A-B"
      reference "$theirs" "$ours" "$name-B-A"
    done
  done
done

# A source content conflict is merged before a rename meets an added link or
# gitlink. The subsequent type split owns the final conflict paths and stages;
# the premerge's original inputs must not replace those normalized stages.
source_base_blob_id=$(printf '%s\n' base one two three four five six seven eight | git hash-object -w --stdin)
source_left_blob_id=$(printf '%s\n' ours one two three four five six seven eight | git hash-object -w --stdin)
source_right_blob_id=$(printf '%s\n' theirs one two three four five six seven eight | git hash-object -w --stdin)
for added_mode in 120000 160000; do
  added_id=$occupied_blob_id
  if test "$added_mode" = 160000; then added_id=$gitlink_commit_id; fi
  name="source-premerge-distinct-$added_mode"
  ours="A-$name"
  theirs="B-$name"
  base_tree_id=$(tree foo 100644 "$source_base_blob_id")
  left_tree_id=$(tree bar 100644 "$source_left_blob_id")
  right_tree_id=$(tree foo 100644 "$source_right_blob_id" bar "$added_mode" "$added_id")
  base_commit_id=$(git commit-tree "$base_tree_id" -m "$name base")
  git update-ref "refs/heads/$ours" "$(git commit-tree "$left_tree_id" -p "$base_commit_id" -m "$ours")"
  git update-ref "refs/heads/$theirs" "$(git commit-tree "$right_tree_id" -p "$base_commit_id" -m "$theirs")"
  reference "$ours" "$theirs" "$name-A-B"
  reference "$theirs" "$ours" "$name-B-A"
done

# A divergent rename keeps the original identity's ancestor at its source only
# while that source remains a file. An unrelated added child makes it a directory,
# so Git omits the executable ancestor rather than moving it to either destination.
base_tree_id=$(tree source 100755 "$left_blob_id")
left_tree_id=$(tree left 100644 "$left_blob_id" source/child 100644 "$right_blob_id")
right_tree_id=$(tree right 100644 "$left_blob_id")
base_commit_id=$(git commit-tree "$base_tree_id" -m source-directory-base)
git update-ref refs/heads/A-source-directory "$(git commit-tree "$left_tree_id" -p "$base_commit_id" -m A-source-directory)"
git update-ref refs/heads/B-source-directory "$(git commit-tree "$right_tree_id" -p "$base_commit_id" -m B-source-directory)"
reference A-source-directory B-source-directory source-directory-A-B
reference B-source-directory A-source-directory source-directory-B-A

# An accepted directory rename relocates a new file into an opposing symlink.
# The type collision is resolved at that effective destination in either order.
base_tree_id=$(tree old/existing 100644 "$left_blob_id")
left_tree_id=$(tree old/existing 100644 "$left_blob_id" old/new 100644 "$right_blob_id")
right_tree_id=$(tree newdir/existing 100644 "$left_blob_id" newdir/new 120000 "$occupied_blob_id")
base_commit_id=$(git commit-tree "$base_tree_id" -m relocated-addition-base)
git update-ref refs/heads/A-relocated-addition "$(git commit-tree "$left_tree_id" -p "$base_commit_id" -m A-relocated-addition)"
git update-ref refs/heads/B-relocated-addition "$(git commit-tree "$right_tree_id" -p "$base_commit_id" -m B-relocated-addition)"
reference A-relocated-addition B-relocated-addition relocated-addition-A-B
reference B-relocated-addition A-relocated-addition relocated-addition-B-A

# A renamed source can remain as an ancestor-stage entry below an independent
# stage-zero file. Git permits this mixed-stage overlap even though a tree or a
# fully resolved index cannot contain a file alongside its descendants.
base_tree_id=$(tree dir/foo 100644 "$left_blob_id")
left_tree_id=$(tree ours 100644 "$left_blob_id" dir 100644 "$right_blob_id")
right_tree_id=$(tree theirs 100644 "$left_blob_id")
base_commit_id=$(git commit-tree "$base_tree_id" -m renamed-source-under-file-base)
git update-ref refs/heads/A-renamed-source-under-file "$(git commit-tree "$left_tree_id" -p "$base_commit_id" -m A-renamed-source-under-file)"
git update-ref refs/heads/B-renamed-source-under-file "$(git commit-tree "$right_tree_id" -p "$base_commit_id" -m B-renamed-source-under-file)"
reference A-renamed-source-under-file B-renamed-source-under-file renamed-source-under-file-A-B
reference B-renamed-source-under-file A-renamed-source-under-file renamed-source-under-file-B-A

# Rename/delete records are created before a distinct-type collision splits the
# destination. Every index stage must follow its own side to the final name;
# keeping the old destination would create a phantom unmerged entry. Replacing
# the source instead of deleting it also checks where a surviving base belongs.
for source_mode in 100644 100755 120000 160000; do
  for replacement_mode in 100644 100755 120000 160000; do
    case "$source_mode-$replacement_mode" in
      100*-100*|120000-120000|160000-160000) continue ;;
    esac
    source_id=$left_blob_id
    replacement_id=$right_blob_id
    if test "$source_mode" = 160000; then source_id=$gitlink_commit_id; fi
    if test "$replacement_mode" = 160000; then replacement_id=$gitlink_commit_id; fi
    for source_state in deleted replaced; do
      name="rename-$source_state-distinct-$source_mode-$replacement_mode"
      ours="A-$name"
      theirs="B-$name"
      base_tree_id=$(tree c "$source_mode" "$source_id")
      if test "$source_state" = deleted; then
        left_tree_id=$(tree dst/y "$replacement_mode" "$replacement_id")
      else
        left_tree_id=$(tree c "$replacement_mode" "$replacement_id")
      fi
      right_tree_id=$(tree dst/y "$source_mode" "$source_id")
      base_commit_id=$(git commit-tree "$base_tree_id" -m "$name base")
      git update-ref "refs/heads/$ours" "$(git commit-tree "$left_tree_id" -p "$base_commit_id" -m "$ours")"
      git update-ref "refs/heads/$theirs" "$(git commit-tree "$right_tree_id" -p "$base_commit_id" -m "$theirs")"
      reference "$ours" "$theirs" "$name-A-B"
      reference "$theirs" "$ours" "$name-B-A"
    done
  done
done
