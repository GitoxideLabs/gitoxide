#!/usr/bin/env bash
set -eu -o pipefail

# Two branches with interleaved dates distinguish topological and date ordering.
# Keep all commits named so tests can select tips, hide ancestry, and prune paths.
git init -q
tree=$(git mktree </dev/null)
commit() {
  local name=$1 seconds=$2
  shift 2
  local commit_id
  commit_id=$(GIT_AUTHOR_DATE="$seconds +0000" GIT_COMMITTER_DATE="$seconds +0000" \
    git commit-tree "$tree" -m "$name" "$@")
  git update-ref "refs/heads/$name" "$commit_id"
  echo "$commit_id"
}
root=$(commit root 946684800)
main_base=$(commit main-base 946684803 -p "$root")
side_base=$(commit side-base 946684804 -p "$root")
main=$(commit main 946684805 -p "$main_base")
side=$(commit side 946684806 -p "$side_base")
merge=$(commit merge 946684807 -p "$main" -p "$side")
tip=$(commit tip 946684808 -p "$merge")
git symbolic-ref HEAD refs/heads/tip
git commit-graph write --reachable

for order in topo-order date-order; do
  for parents in all first; do
    args=("--$order")
    if test "$parents" = first; then
      args+=(--first-parent)
    fi
    git -c core.commitGraph=false rev-list "${args[@]}" tip >"$order-$parents-all.baseline"
    git -c core.commitGraph=false rev-list "${args[@]}" tip ^main >"$order-$parents-hidden.baseline"
    git -c core.commitGraph=false rev-list "${args[@]}" main side main ^root >"$order-$parents-multiple.baseline"
  done
done

# A depth-one repository must never try to read the missing parent of its tip.
git init -q shallow
git cat-file commit "$tip" | git -C shallow hash-object -w -t commit --stdin
git -C shallow update-ref refs/heads/tip "$tip"
git -C shallow symbolic-ref HEAD refs/heads/tip
echo "$tip" >shallow/.git/shallow

# Preserve full objects and a stale commit-graph behind a shallow boundary.
# Hiding main must stop there: root remains visible through the side branch.
mkdir shallow-with-objects
cp -R .git shallow-with-objects/.git
echo "$main" >shallow-with-objects/.git/shallow
git -C shallow-with-objects -c core.commitGraph=false rev-list --topo-order tip ^main >shallow-hidden.baseline
