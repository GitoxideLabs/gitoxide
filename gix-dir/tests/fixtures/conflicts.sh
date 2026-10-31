#!/usr/bin/env bash
set -eu -o pipefail

# Populate every nonempty combination of conflict stages, at the root, nested,
# and under an ignore rule. Files remain on disk even for DD: any index stage
# must keep them tracked regardless of ignore rules or directory collapsing.
# The nested UU path has a symlink at stage 2 to check our side's type preference.
git init -q
printf 'ignored/*.txt\n*.txt/\nfile-dir-*/\n' >.gitignore
git add .gitignore
git commit -qm init
mkdir nested ignored
blob_id=$(printf 'content\n' | git hash-object -w --stdin)
for prefix in '' nested/ ignored/; do
  for conflict in 'DD:1' 'AU:2' 'UA:3' 'UD:1 2' 'DU:1 3' 'AA:2 3' 'UU:1 2 3'; do
    path="${prefix}${conflict%%:*}.txt"
    printf 'content\n' >"$path"
    for stage in ${conflict#*:}; do
      mode=100644
      if [[ "$path" == nested/UU.txt && "$stage" == 2 ]]; then
        mode=120000
      fi
      printf '%s %s %s\t%s\n' "$mode" "$blob_id" "$stage" "$path"
    done
  done
done | git update-index --index-info

# A file/directory conflict can index both the directory's name and its child.
# Ignoring the directory must never hide the indexed child from a cleanup walk.
for conflict in 'file-dir-base:1' 'file-dir-ours:2' 'file-dir-theirs:3'; do
  path=${conflict%:*}
  stage=${conflict#*:}
  child_stage=3
  if [[ "$stage" == 3 ]]; then
    child_stage=2
  fi
  mkdir "$path"
  printf 'content\n' >"$path/bar"
  printf '100644 %s %s\t%s\n100644 %s %s\t%s/bar\n' \
    "$blob_id" "$stage" "$path" "$blob_id" "$child_stage" "$path"
done | git update-index --index-info
touch untracked nested/untracked

# Keep Git's baselines outside the walked worktree so they cannot affect the results.
git -c core.ignoreCase=false clean -ndx -- 'file-dir-*' >.git/baseline-clean
for ignore_case in false true; do
  git -c core.ignoreCase="$ignore_case" ls-files --others --exclude-standard >".git/baseline-untracked-$ignore_case"
done
git ls-files --stage -- nested/UU.txt >.git/baseline-stages
