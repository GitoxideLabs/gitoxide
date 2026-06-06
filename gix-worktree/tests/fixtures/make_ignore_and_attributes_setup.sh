#!/usr/bin/env bash
set -eu -o pipefail

cat <<EOF >user.exclude
# a custom exclude configured per user
user-file-anywhere
/user-file-from-top

user-Dir-anywhere/
/user-dir-from-top

user-subdir/file
**/user-subdir-anywhere/file
a/b/*
z/x
EOF

mkdir repo;
(cd repo
  git init -q
  git config core.excludesFile ../user.exclude

  cat <<EOF >.git/info/exclude
# a sample .git/info/exclude
file-anywhere
/file-from-top

dir-anywhere/
/dir-from-top

subdir/file
**/subdir-anywhere/file
EOF

  cat <<EOF >.gitignore
# a sample .gitignore
top-level-local-file-anywhere
d/e/*
e/f

# directory ignored despite containing a tracked file
bin/
EOF

  mkdir dir-with-ignore
  cat <<EOF >dir-with-ignore/.gitignore
# a sample .gitignore
sub-level-local-file-anywhere
sub-Level-dir-anywhere/
!/negated
/negated-dir/
!/negated-dir/
EOF

  mkdir -p src/bin other/bin
  echo "fn main() {}" >src/bin/stub_gen.rs

  git add .gitignore dir-with-ignore
  git add -f src/bin/stub_gen.rs
  git commit --allow-empty -m "init"

  # just add this git-ignore file, so it's a new file that doesn't exist on disk.
  mkdir other-dir-with-ignore
  skip_worktree_ignore=other-dir-with-ignore/.gitignore
  cat <<EOF >"$skip_worktree_ignore"
# a sample .gitignore
other-sub-level-local-file-anywhere
other-sub-level-dir-anywhere/
EOF
  git add $skip_worktree_ignore && git update-index --skip-worktree $skip_worktree_ignore && rm $skip_worktree_ignore

  mkdir user-dir-anywhere user-dir-from-top dir-anywhere dir-from-top
  mkdir -p dir/user-dir-anywhere dir/dir-anywhere
  echo "extra" >src/bin/extra.txt
  echo "other" >other/bin/file.txt

  cat <<EOF >../git-check-ignore.paths
dir-with-ignore/sub-level-dir-anywhere/
dir-with-ignore/foo/Sub-level-dir-anywhere/
dir-with-ignore/Sub-level-dir-anywhere
user-file-anywhere
dir/user-file-anywhere
user-file-from-top
no-match/user-file-from-top
USER-dir-anywhere
user-dir-from-top
no-match/user-dir-from-top
user-subdir/file
subdir/user-subdir-anywhere/file
user-dir-anywhere/hello
dir/user-dir-anywhere/hello
file-anywhere
dir/file-anywhere
file-from-top
no-match/file-from-top
dir-anywhere
dir/dir-anywhere
dir-from-top
no-match/dir-from-top
subdir/file
subdir/subdir-anywhere/file
top-level-local-file-anywhere
dir/top-level-local-file-anywhere
no-match/sub-level-local-file-anywhere
dir-with-ignore/sub-level-local-file-anywhere
dir-with-ignore/sub-dir/sub-level-local-file-anywhere
other-dir-with-ignore/other-sub-level-local-file-anywhere
other-dir-with-ignore/sub-level-local-file-anywhere
other-dir-with-ignore/sub-dir/other-sub-level-local-file-anywhere
other-dir-with-ignore/no-match/sub-level-local-file-anywhere
non-existing/dir-anywhere
dir-anywhere/hello
dir/dir-anywhere/hello
no-match/sub-level-dir-anywhere/hello
no-match/other-sub-level-dir-anywhere/hello
dir-with-ignore/sub-level-dir-anywhere/hello
dir-with-ignore/sub-level-dir-anywhere/
other-dir-with-ignore/sub-level-dir-anywhere/hello
other-dir-with-ignore/other-sub-level-dir-anywhere/hello
other-dir-with-ignore/other-sub-level-dir-anywhere/
dir-with-ignore/negated
dir-with-ignore/negated-dir/hello
src/bin
src/bin/
src/bin/stub_gen.rs
src/bin/extra.txt
other/bin
other/bin/file.txt
User-file-ANYWHERE
User-Dir-ANYWHERE
a/b/C
a/B/c
A/B/C
z/x
Z/x
z/X
Z/X
d/e/F
d/e/f
D/e/F
D/E/F
e/f
e/F
E/f
E/F
EOF

  git check-ignore -vn --stdin 2>&1 <../git-check-ignore.paths >../git-check-ignore.baseline || :
)
