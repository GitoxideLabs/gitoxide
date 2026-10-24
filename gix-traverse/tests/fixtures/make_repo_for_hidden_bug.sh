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
)

# A short visible range must not read the shared history below its boundary.
(git init -q -b main short_visible_range && cd short_visible_range
  for ((i=0; i<64; i++)); do
    commit_at "shared-$i" "$((1700000100 + i))"
  done
  git tag hidden
  commit_at visible 1700000200
  optimize
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
)
