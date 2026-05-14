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
