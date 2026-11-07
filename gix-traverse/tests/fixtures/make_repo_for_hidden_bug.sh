#!/usr/bin/env bash
set -eu -o pipefail

function commit_at() {
  local message=${1:?first argument is the commit message}
  local timestamp=${2:?second argument is the timestamp}
  GIT_COMMITTER_DATE="$timestamp -0700"
  GIT_AUTHOR_DATE="$timestamp -0700"
  export GIT_COMMITTER_DATE GIT_AUTHOR_DATE
  git commit --allow-empty -m "$message"
}

function optimize() {
  git commit-graph write --no-progress --reachable
  git repack -adq
}

# Print the id of a new commit with the empty tree, passing the remaining arguments, like `-p <parent>`, to `git commit-tree`.
function empty_commit_at() {
  local message=${1:?first argument is the commit message}
  local timestamp=${2:?second argument is the timestamp}
  shift 2
  GIT_AUTHOR_DATE="$timestamp +0000" GIT_COMMITTER_DATE="$timestamp +0000" \
    git commit-tree "$(git hash-object -w -t tree --stdin </dev/null)" -m "$message" "$@"
}

# `main` merges `unrelated`, an older history, after `base`. `topic` forks from `base`, and `combined`
# merges `unrelated` into `base`, both after the last commit on `main`. Commit times advance by the
# first argument in seconds, so `0` gives every commit the same date.
function make_unrelated_history() {
  local step=${1:?first argument is the number of seconds between commits}
  local commit_id
  commit_id=$(empty_commit_at unrelated-0 1700000000)
  for ((i=1; i<8; i++)); do
    commit_id=$(empty_commit_at "unrelated-$i" "$((1700000000 + step*i))" -p "$commit_id")
  done
  git update-ref refs/heads/unrelated "$commit_id"
  commit_id=$(empty_commit_at main-0 "$((1700000000 + step*10))")
  for ((i=1; i<40; i++)); do
    if test "$i" = 36; then
      commit_id=$(empty_commit_at merge "$((1700000000 + step*(9 + 2*i)))" -p "$commit_id" -p unrelated)
    fi
    commit_id=$(empty_commit_at "main-$i" "$((1700000000 + step*(10 + 2*i)))" -p "$commit_id")
    if test "$i" = 29; then
      git update-ref refs/heads/base "$commit_id"
    fi
  done
  git update-ref refs/heads/main "$commit_id"
  commit_id=$(empty_commit_at topic-0 "$((1700000000 + step*100))" -p base)
  commit_id=$(empty_commit_at topic-1 "$((1700000000 + step*101))" -p "$commit_id")
  git update-ref refs/heads/topic "$commit_id"
  git update-ref refs/heads/combined "$(empty_commit_at combined "$((1700000000 + step*102))" -p base -p unrelated)"
  for parents in do-walk first-parent; do
    git rev-list "--$parents" topic ^main >"rev-list-topic-$parents.baseline"
    git rev-list "--$parents" main ^topic >"rev-list-main-$parents.baseline"
    git rev-list "--$parents" combined ^main >"rev-list-combined-$parents.baseline"
    git rev-list "--$parents" combined ^base >"rev-list-combined-base-$parents.baseline"
  done
}

# Record Git's traversal baselines alongside each repository so caches and archives
# contain the expected results without running Git again in the tests.

# A visible root can sort ahead of the hidden paint when its child has an older date.
(git init -q -b main skewed_dates && cd skewed_dates
  commit_at root 1700000002
  git tag root
  commit_at skewed 1700000000
  commit_at visible 1700000003
  git tag visible
  commit_at hidden 1700000004
  git tag hidden
  optimize
  git rev-list visible root ^hidden >rev-list.baseline
  git rev-list visible root hidden ^hidden ^hidden >rev-list-repeated-exclusions.baseline
)

# A short visible range must not read the shared history below its boundary.
(git init -q -b main short_visible_range && cd short_visible_range
  for ((i=0; i<64; i++)); do
    commit_at "shared-$i" "$((1700000100 + i))"
  done
  git tag hidden
  commit_at visible 1700000200
  optimize
  git rev-list main ^hidden >rev-list.baseline
)

# The fifth hidden step loads gap, whose already-loaded root must still receive hidden paint.
(git init -q -b main skewed_lookahead && cd skewed_lookahead
  commit_at root 1700000080
  git tag root
  commit_at gap 1700000020
  for ((i=1; i<=4; i++)); do
    commit_at "hidden-$i" "$((1700000000 + 10*i))"
  done
  commit_at hidden 1700000050
  git branch hidden
  git checkout -q -b visible root
  commit_at visible 1700000100
  optimize
  git rev-list visible ^hidden >rev-list.baseline
)

# Both visible merge parents have their own hidden boundary. Repeated diamonds
# join those boundaries above a long shared tail that painting must not read.
(git init -q -b main merged_visible_range && cd merged_visible_range
  empty_tree_id=$(git hash-object -w -t tree --stdin </dev/null)
  commit_tree_at() {
    local message=${1:?} timestamp=${2:?}
    shift 2
    GIT_AUTHOR_DATE="$timestamp +0000" GIT_COMMITTER_DATE="$timestamp +0000" \
      git commit-tree "$empty_tree_id" -m "$message" "$@"
  }
  shared_commit_id=$(commit_tree_at shared-0 1700000100)
  for ((i=1; i<128; i++)); do
    shared_commit_id=$(commit_tree_at "shared-$i" "$((1700000100 + i))" -p "$shared_commit_id")
  done
  for side in left right; do
    offset=1700001000
    if test "$side" = right; then
      offset=1700002000
    fi
    boundary_commit_id=$shared_commit_id
    for ((i=0; i<16; i++)); do
      first_commit_id=$(commit_tree_at "$side-first-$i" "$((offset + 3*i))" -p "$boundary_commit_id")
      second_commit_id=$(commit_tree_at "$side-second-$i" "$((offset + 3*i + 1))" -p "$boundary_commit_id")
      boundary_commit_id=$(commit_tree_at "$side-merge-$i" "$((offset + 3*i + 2))" -p "$first_commit_id" -p "$second_commit_id")
    done
    if test "$side" = left; then
      left_commit_id=$boundary_commit_id
    else
      right_commit_id=$boundary_commit_id
    fi
  done
  hidden_commit_id=$(commit_tree_at hidden 1700003000 -p "$left_commit_id" -p "$right_commit_id")
  left_commit_id=$(commit_tree_at visible-left 1700003001 -p "$left_commit_id")
  right_commit_id=$(commit_tree_at visible-right 1700003002 -p "$right_commit_id")
  visible_commit_id=$(commit_tree_at visible 1700003003 -p "$left_commit_id" -p "$right_commit_id")
  git update-ref refs/heads/main "$visible_commit_id"
  git update-ref refs/heads/hidden "$hidden_commit_id"
  # A larger visible range must still avoid traversing the old hidden merge history.
  for ((i=1; i<40; i++)); do
    left_commit_id=$(commit_tree_at "visible-wide-left-$i" "$((1700004000 + 2*i))" -p "$left_commit_id")
    right_commit_id=$(commit_tree_at "visible-wide-right-$i" "$((1700004001 + 2*i))" -p "$right_commit_id")
  done
  visible_commit_id=$(commit_tree_at visible-wide 1700005000 -p "$left_commit_id" -p "$right_commit_id")
  git update-ref refs/heads/wide "$visible_commit_id"
  optimize
  git rev-list main ^hidden >rev-list.baseline
  git rev-list --first-parent main ^hidden >rev-list-first-parent.baseline
  git rev-list wide ^hidden >rev-list-wide.baseline
  git rev-list --first-parent wide ^hidden >rev-list-wide-first-parent.baseline
  for order in topo-order date-order; do
    for parents in do-walk first-parent; do
      for tip in main wide; do
        git rev-list "--$order" "--$parents" "$tip" ^hidden >"$order-$parents-$tip.baseline"
      done
    done
  done
)

# A merge reaches the hidden root both directly and through a skewed parent.
(git init -q -b main skewed_merge && cd skewed_merge
  commit_at root 1700000002
  git tag root
  commit_at skewed 1700000000
  commit_at shared 1700000003
  git tag shared
  commit_at hidden 1700000004
  git tag hidden
  GIT_COMMITTER_DATE='1700000005 -0700'
  export GIT_COMMITTER_DATE
  visible_commit_id=$(echo visible | git commit-tree 'HEAD^{tree}' -p shared -p root)
  git update-ref refs/heads/visible "$visible_commit_id"
  optimize
  git rev-list visible ^hidden >rev-list.baseline
)

# Reaching one hidden boundary does not prove visibility relative to another.
(git init -q -b main crossed_hidden_boundaries && cd crossed_hidden_boundaries
  commit_at root 1700000002
  commit_at hidden-base 1700000004
  git tag hidden-base
  commit_at visible-middle 1700000003
  git tag visible-middle
  commit_at skewed 1700000000
  commit_at hidden-top 1700000005
  git tag hidden-top
  commit_at visible-tip 1700000006
  optimize
  git rev-list main visible-middle ^hidden-base ^hidden-top >rev-list.baseline
)

# Test 1: Hidden traversal has a longer path to shared ancestors
# Graph structure:
#   A(tip) --> shared
#            /
#   H(hidden) --> X --> Y --> shared  
#
# This tests that shared is correctly hidden even though the interesting
# path (A->shared) is shorter than the hidden path (H->X->Y->shared).

(git init long_hidden_path && cd long_hidden_path
  git checkout -b main
  
  # Create base commit with oldest timestamp
  commit_at "shared" 1000000000
  
  # Create hidden branch with intermediate commits
  git checkout -b hidden_branch
  commit_at "Y" 1000000100
  commit_at "X" 1000000200
  commit_at "H" 1000000300  # hidden tip
  
  # Go back to main and create tip A (newest timestamp)
  git checkout main
  commit_at "A" 1000000400  # tip
  
  optimize
  git rev-list main --not hidden_branch >rev-list.baseline
)

# Test 2: Similar structure but with interesting path longer than hidden path
# Graph structure:
#   A(tip) --> B --> C --> D(shared)
#                        /
#   H(hidden) --------->+
#
# This tests that D is correctly hidden when the interesting path
# (A->B->C->D) is longer than the hidden path (H->D).

(git init long_interesting_path && cd long_interesting_path
  git checkout -b main
  
  # Create base commit with oldest timestamp
  commit_at "D" 1000000000
  
  # Create hidden branch (direct to D)
  git checkout -b hidden_branch
  commit_at "H" 1000000100  # hidden tip, direct child of D
  
  # Go back to main and create longer path
  git checkout main
  commit_at "C" 1000000200
  commit_at "B" 1000000300
  commit_at "A" 1000000400  # tip
  
  optimize
  git rev-list main --not hidden_branch >rev-list.baseline
)

(git init -q -b main unrelated_history && cd unrelated_history
  make_unrelated_history 60
  optimize
)

(git init -q -b main unrelated_history_same_date && cd unrelated_history_same_date
  make_unrelated_history 0
  optimize
)

# Only the commit-graph knows the tip of `unrelated`, as its object is deleted.
(git init -q -b main unrelated_history_missing_commit && cd unrelated_history_missing_commit
  make_unrelated_history 60
  git commit-graph write --no-progress --reachable
  git log --all --format='%H %s' >commit-names.baseline
  unrelated_commit_id=$(git rev-parse unrelated)
  rm ".git/objects/${unrelated_commit_id:0:2}/${unrelated_commit_id:2}"
)

# Criss-cross merges: `a2` and `b2` both merge `a1` and `b1`.
(git init -q -b main criss_cross && cd criss_cross
  git tag root "$(empty_commit_at root 1700000000)"
  git tag a1 "$(empty_commit_at a1 1700000060 -p root)"
  git tag b1 "$(empty_commit_at b1 1700000120 -p root)"
  git tag a2 "$(empty_commit_at a2 1700000180 -p a1 -p b1)"
  git tag b2 "$(empty_commit_at b2 1700000240 -p b1 -p a1)"
  git tag a3 "$(empty_commit_at a3 1700000300 -p a2)"
  git tag b3 "$(empty_commit_at b3 1700000360 -p b2)"
  git tag both "$(empty_commit_at both 1700000420 -p a3 -p b3)"
  optimize
  for parents in do-walk first-parent; do
    git rev-list "--$parents" a3 ^b3 >"rev-list-a3-b3-$parents.baseline"
    git rev-list "--$parents" b3 ^a3 >"rev-list-b3-a3-$parents.baseline"
    git rev-list "--$parents" both ^a2 >"rev-list-both-a2-$parents.baseline"
    git rev-list "--$parents" a3 b3 ^a2 ^b2 >"rev-list-a3-b3-a2-b2-$parents.baseline"
  done
)

# Only `base` is in the commit-graph. `hidden` reaches `middle` through `skewed`, which is dated
# before it, so `middle` pops with only visible paint while `skewed` is still queued.
(git init -q -b main skewed_partial_graph && cd skewed_partial_graph
  git tag base "$(empty_commit_at base 1700000000)"
  git commit-graph write --no-progress --reachable
  git tag middle "$(empty_commit_at middle 1700000200 -p base)"
  git tag skewed "$(empty_commit_at skewed 1700000100 -p middle)"
  git tag hidden "$(empty_commit_at hidden 1700000400 -p skewed)"
  git tag visible "$(empty_commit_at visible 1700000300 -p middle)"
  for parents in do-walk first-parent; do
    git rev-list "--$parents" visible ^hidden ^base >"rev-list-$parents.baseline"
  done
)

# `hidden` and `visible` share `middle`. Tests clear the generation of `hidden` in the commit-graph,
# as a legacy writer would, which sorts it below `other` and every other commit.
(git init -q -b main uncomputed_generation && cd uncomputed_generation
  git tag root "$(empty_commit_at root 1700000100)"
  git tag middle "$(empty_commit_at middle 1700000200 -p root)"
  git tag visible "$(empty_commit_at visible 1700000300 -p middle)"
  git tag hidden "$(empty_commit_at hidden 1700000400 -p middle)"
  git tag hidden-child "$(empty_commit_at hidden-child 1700000500 -p hidden)"
  git tag other "$(empty_commit_at other 1700000000)"
  optimize
  for hidden in hidden hidden-child; do
    git rev-list visible "^$hidden" ^other >"rev-list-$hidden.baseline"
  done
)

for repo in skewed_dates short_visible_range skewed_lookahead merged_visible_range skewed_merge crossed_hidden_boundaries long_hidden_path long_interesting_path \
    unrelated_history unrelated_history_same_date criss_cross skewed_partial_graph uncomputed_generation; do
  git -C "$repo" log --all --format='%H %s' >"$repo/commit-names.baseline"
done
