#!/usr/bin/env bash
set -eu -o pipefail

# A structural review has a file-tree overview even for empty, binary,
# malformed, unsupported, and mode-only changes. A file rename is exact;
# declaration renames and hidden-body edits are separate changes. A merge
# distinguishes both parent scopes. Staged and unstaged signatures differ.
git init -q -b main
mkdir -p src
cat >src/modified.rs <<'EOF'
impl Widget {
    pub fn old(limit: usize, value: i32) -> i32 {
        let secret_before = value + 1;
        secret_before
    }
}
pub struct Config {
    pub before: i32,
}
EOF
cat >src/old.rs <<'EOF'
fn unchanged() { secret_renamed_file(); }
EOF
cat >src/removed.rs <<'EOF'
fn removed() { secret_removed(); }
EOF
cat >src/executable.rs <<'EOF'
fn executable() { secret_mode(); }
EOF
cat >Cargo.toml <<'EOF'
[package]
name = "secret_before"
EOF
cat >README.md <<'EOF'
# Guide

## Details
secret_before
EOF
printf 'secret_before\n' >unsupported.txt
git add .
git commit -qm base
git branch base

cat >src/modified.rs <<'EOF'
impl Widget {
    pub fn new(limit: usize, value: u32) -> u32 {
        let secret_after = value + 2;
        secret_after
    }
}
pub struct Config {
    pub after: u32,
}
EOF
git mv src/old.rs src/moved.rs
rm src/removed.rs
mkdir -p src/nested
cat >src/nested/added.rs <<'EOF'
fn added() { secret_added(); }
EOF
: >src/empty.rs
printf 'fn broken(\n' >src/broken.rs
printf '\377\n' >src/non-utf8.rs
printf '\0binary\n' >binary.rs
chmod +x src/executable.rs
sed 's/secret_before/secret_after/' Cargo.toml >Cargo.toml.new
mv Cargo.toml.new Cargo.toml
sed 's/secret_before/secret_after/' README.md >README.md.new
mv README.md.new README.md
printf 'secret_after\n' >unsupported.txt
git add .
git commit -qm updated
git branch tip

git checkout -qb side base
mkdir -p side
cat >side/parent.rs <<'EOF'
fn side_parent() { secret_side(); }
EOF
git add .
git commit -qm side
git checkout -q main
git merge -q --no-ff side -m merge

# Preserve distinct HEAD, index, and worktree resource identities.
sed 's/value: u32/value: u64/' src/modified.rs >src/modified.rs.new
mv src/modified.rs.new src/modified.rs
git add src/modified.rs
sed 's/value: u64/value: u128/' src/modified.rs >src/modified.rs.new
mv src/modified.rs.new src/modified.rs
cat >src/untracked.rs <<'EOF'
fn untracked() { secret_untracked(); }
EOF
