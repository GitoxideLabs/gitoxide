#!/usr/bin/env bash
set -eu -o pipefail

git init

EMPTY_TREE=$(git mktree </dev/null)
function ofs_commit () {
  local OFFSET_SECONDS=$1
  local COMMIT_NAME=$2
  shift 2

  PARENTS= 
  for P; do
    PARENTS="${PARENTS}-p $P "
  done

  GIT_COMMITTER_DATE="$((400403349 + OFFSET_SECONDS)) +0000"
  GIT_AUTHOR_DATE=$GIT_COMMITTER_DATE
  export GIT_COMMITTER_DATE GIT_AUTHOR_DATE

  commit=$(echo $COMMIT_NAME | git commit-tree $EMPTY_TREE ${PARENTS:-})

  git update-ref "refs/tags/$COMMIT_NAME" "$commit"
  echo $commit
}

# Each sample records names, resolved IDs, ahead/behind counts (pairs only),
# then merge bases. Keep both Git oracles in the cached fixture so tests never
# need to spawn Git to compare the same pairs, including all permutations.
function baseline() {
  echo "$@"
  echo $(git rev-parse "$@")
  if [ "$#" -eq 2 ]; then
    git rev-list --left-right --count "$1...$2"
  else
    echo
  fi
  git merge-base --all "$@" || :
  echo
}

# Merge-bases adapted from Git test suite
# No merge base
ofs_commit 0 DA
ofs_commit 100 DB
{
  echo "just-one-returns-one-in-code"
  echo $(git rev-parse DA)
  echo
  echo $(git rev-parse DA)
  echo
  baseline DA DB
  baseline DA DA DB
} > 1_disjoint.baseline

# A graph that is purposefully using times that can't be trusted, i.e. the root E
# has a higher time than its future commits, so that it would be preferred
# unless if there was an additional pruning step to deal with this case.
# E---D---C---B---A
# \"-_         \   \
#  \  `---------G   \
#   \                \
#    F----------------H
E=$(ofs_commit 5 E)
D=$(ofs_commit 4 D $E)
F=$(ofs_commit 6 F $E)
C=$(ofs_commit 3 C $D)
B=$(ofs_commit 2 B $C)
A=$(ofs_commit 1 A $B)
G=$(ofs_commit 7 G $B $E)
H=$(ofs_commit 8 H $A $F)

{
  baseline G H
} > 2_a.baseline

# Permutation testing - let's do it early to avoid too many permutations
commits=$(git log --all --format=%s)
commit_array=($commits)
num_commits=${#commit_array[@]}

for ((i=0; i<num_commits; i++)); do
    for ((j=0; j<num_commits; j++)); do
        baseline ${commit_array[$i]} ${commit_array[$j]}
    done
done > 3_permutations.baseline

# Timestamps cannot be trusted.
#
#               Relative
# Structure     timestamps
#
#   PL  PR        +4  +4
#  /  \/  \      /  \/  \
# L2  C2  R2    +3  -1  +3
# |   |   |     |   |   |
# L1  C1  R1    +2  -2  +2
# |   |   |     |   |   |
# L0  C0  R0    +1  -3  +1
#   \ |  /        \ |  /
#     S             0
#
# The left and right chains of commits can be of any length and complexity as
# long as all of the timestamps are greater than that of S.
S=$(ofs_commit  0 S)

C0=$(ofs_commit -3 C0 $S)
C1=$(ofs_commit -2 C1 $C0)
C2=$(ofs_commit -1 C2 $C1)

L0=$(ofs_commit 1 L0 $S)
L1=$(ofs_commit 2 L1 $L0)
L2=$(ofs_commit 3 L2 $L1)

R0=$(ofs_commit 1 R0 $S)
R1=$(ofs_commit 2 R1 $R0)
R2=$(ofs_commit 3 R2 $R1)

PL=$(ofs_commit 4 PL $L2 $C2)
PR=$(ofs_commit 4 PR $C2 $R2)

{
  baseline PL PR
} > 4_b.baseline


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
  local date=${2:-}
  if [ -n "$date" ]; then
    export GIT_COMMITTER_DATE="$date"
  else
    tick
  fi
  git commit --allow-empty -m "$message"
  git tag "$message"
}

# * C (MMC) * B (MMB) * A  (MMA)
# * o       * o       * o
# * o       * o       * o
# * o       * o       * o
# * o       | _______/
# |         |/
# |         * 1 (MM1)
# | _______/
# |/
# * root (MMR)

commit MMR
commit MM1
commit MM-o
commit MM-p
commit MM-q
commit MMA
git checkout MM1
commit MM-r
commit MM-s
commit MM-t
commit MMB
git checkout MMR
commit MM-u
commit MM-v
commit MM-w
commit MM-x
commit MMC

{
  baseline MMA MMB MMC
} > 5_c.baseline

merge () {
  label="$1"
  shift
  tick
  git merge -m "$label" "$@"
  git tag "$label"
}

#             JE
#            / |
#           /  |
#          /   |
#  JAA    /    |
#   |\   /     |
#   | \  | JDD |
#   |  \ |/ |  |
#   |   JC JD  |
#   |    | /|  |
#   |    |/ |  |
#  JA    |  |  |
#   |\  /|  |  |
#   X JB |  X  X
#   \  \ | /   /
#    \__\|/___/
#        J
commit J
commit JB
git reset --hard J
commit JC
git reset --hard J
commit JTEMP1
merge JA JB
merge JAA JC
git reset --hard J
commit JTEMP2
merge JD JB
merge JDD JC
git reset --hard J
commit JTEMP3
merge JE JC

{
  baseline JAA JDD JE
} > 5_c.baseline

git commit-graph write --no-progress --reachable
git repack -adq

# Create a child only after writing the commit-graph: its cached parent A has
# reliable generations, but UNCACHED_A does not. Do not rewrite the graph here;
# read-only traversal tests must exercise the transition back into cached history.
ofs_commit 9 UNCACHED_A "$A"
{
  baseline UNCACHED_A A
  baseline A UNCACHED_A
} > 6_uncached.baseline
