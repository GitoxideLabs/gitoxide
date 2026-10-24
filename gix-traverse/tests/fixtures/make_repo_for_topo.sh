#!/usr/bin/env bash
set -eu -o pipefail

function tick () {
  if test -z "${tick+set}"
  then
    tick=1112911993
  else
    tick=$(($tick + 60))
  fi
  GIT_COMMITTER_DATE="$tick -0700"
  GIT_AUTHOR_DATE="$tick -0700"
  export GIT_COMMITTER_DATE GIT_AUTHOR_DATE
}

tick
function commit() {
  local message=${1:?first argument is the commit message}
  tick
  git commit --allow-empty -m "$message"
}

function optimize() {
  git commit-graph write --no-progress --reachable
  git repack -adq
}

function collect_baselines() {
  git rev-list --topo-order HEAD > all-commits.baseline
  git rev-list --topo-order --first-parent HEAD > first-parent.baseline

  case ${GIT_DEFAULT_HASH:-sha1} in
    sha1)
      hash="f1cce1b5c7efcdfa106e95caa6c45a2cae48a481" ;;
    sha256)
      hash="8a6ffed7d2ff3f19aca13dfb7a8cfd7c100a59d56fd615c5df3e0fd04852b855" ;;
    *)
      exit 1 ;;
  esac
  git rev-list --date-order ^$hash HEAD > date-order.baseline
}

git init
git config merge.ff false

git checkout -q -b main
for i in {0..5}; do
    commit c$i
done

git branch branch1
for i in {6..8}; do
    commit c$i
done

git checkout -q branch1
commit b1c1

git checkout -q main
commit c9

git merge branch1 -m merge

git checkout -q branch1
commit c10
commit c11

git checkout -q branch1
commit b1c2

git checkout -q main
git merge branch1 -m merge
commit c12

optimize
collect_baselines

# Hidden ends traverse every parent even when visible commits use first-parent traversal.
git init -q -b main hidden-ends
(
  cd hidden-ends
  commit base
  git checkout -q -b side
  for i in 1 2 3; do
    commit "side$i"
    git tag "side$i"
  done
  git checkout -q main
  commit main
  git merge -q --no-ff -m merge side
  git checkout -q -b visible
  commit visible
  optimize
)

# Newer timestamps explore fork/shared/merge before the hidden end paints them.
# The merge's second parent is also a tip, so incomplete hidden painting exposes it.
git init -q -b main hidden-skewed
(
  cd hidden-skewed
  empty_tree_id=$(git hash-object -w -t tree --stdin </dev/null)
  commit_at() {
    local commit_name=${1:?} commit_time=${2:?}
    shift 2
    local commit_id
    commit_id=$(GIT_AUTHOR_DATE="$commit_time +0000" GIT_COMMITTER_DATE="$commit_time +0000" git commit-tree "$empty_tree_id" -m "$commit_name" "$@")
    git update-ref "refs/heads/$commit_name" "$commit_id"
    printf '%s' "$commit_id"
  }
  base_commit_id=$(commit_at base 1000000100)
  first_commit_id=$(commit_at first 1000000200 -p "$base_commit_id")
  side_commit_id=$(commit_at side 1000000800 -p "$base_commit_id")
  merge_commit_id=$(commit_at merge 1000000700 -p "$first_commit_id" -p "$side_commit_id")
  shared_commit_id=$(commit_at shared 1000000801 -p "$merge_commit_id")
  fork_commit_id=$(commit_at fork 1000000850 -p "$shared_commit_id")
  parent_commit_id=$(commit_at parent 1000000400 -p "$fork_commit_id")
  commit_at hidden 1000000500 -p "$parent_commit_id" >/dev/null
  commit_at visible 1000000900 -p "$fork_commit_id" >/dev/null
  git symbolic-ref HEAD refs/heads/visible
  optimize
)
