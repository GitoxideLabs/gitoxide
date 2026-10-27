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

for repo in skewed_dates short_visible_range skewed_lookahead merged_visible_range skewed_merge crossed_hidden_boundaries long_hidden_path long_interesting_path; do
  git -C "$repo" log --all --format='%H %s' >"$repo/commit-names.baseline"
done
