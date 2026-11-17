#!/usr/bin/env bash
set -eu -o pipefail

# Production-only review removes inline tests, test-only edits, and files in
# test/benchmark/example/fixture directories. Malformed fixture code must also
# disappear. Production mode changes, exact file renames, docs, and unknown
# file types remain visible. The fixture has thirteen changed files, five live.
git init -q -b main
mkdir -p src tests benches benchmarks examples support/fixtures docs
cat >src/mixed.rs <<'EOF'
mod api {
    pub fn production() { secret_before(); }
    #[cfg(test)]
    mod tests {
        #[test]
        fn smoke() { secret_before(); }
    }
}
EOF
cat >src/test-only.rs <<'EOF'
fn unchanged() {}
#[cfg(test)]
mod tests {
    fn test_helper() { secret_before(); }
}
EOF
cat >src/unit.rs <<'EOF'
#![cfg(test)]
fn unit_helper() { secret_before(); }
EOF
cat >src/test-mode.rs <<'EOF'
#![cfg(test)]
fn test_mode() {}
EOF
printf 'fn production_mode() {}\n' >src/mode.rs
printf 'fn unchanged_file() {}\n' >src/old.rs
for file in tests/integration.rs benches/benchmark.rs benchmarks/compare.rs examples/demo.rs; do
    printf 'fn helper() { secret_before(); }\n' >"$file"
done
printf 'fn broken( before\n' >support/fixtures/input.rs
printf '# Guide\n\nbefore\n' >docs/guide.md
printf 'before\n' >unknown.txt
git add .
git commit -qm first
git branch base

for file in src/mixed.rs src/test-only.rs src/unit.rs \
    tests/integration.rs benches/benchmark.rs benchmarks/compare.rs \
    examples/demo.rs support/fixtures/input.rs docs/guide.md unknown.txt; do
    sed 's/before/after/g' "$file" >"$file.new"
    mv "$file.new" "$file"
done
chmod +x src/mode.rs src/test-mode.rs
git mv src/old.rs src/moved.rs
git add .
git commit -qm second
git branch tip
