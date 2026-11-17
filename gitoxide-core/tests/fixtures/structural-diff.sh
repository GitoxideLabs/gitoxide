#!/usr/bin/env bash
set -eu -o pipefail

# These inputs exercise read-only outlines. Distinct secret_* markers make any
# accidental rendering of executable code, configuration values, or prose visible.
cat >body-before.rs <<'EOF'
impl Widget {
	pub fn calculate(
		&self,
		input: i32,
	) -> i32 {
		let secret_body_before = input + 1;
		secret_body_before
	}
}
EOF
cat >body-after.rs <<'EOF'
impl Widget {
	pub fn calculate(
		&self,
		input: i32,
	) -> i32 {
		let secret_body_after = input + 2;
		secret_body_after
	}
}
EOF

cat >items-before.rs <<'EOF'
pub struct Kept {
    pub value: i32,
}

fn removed() {
    secret_removed();
}
EOF
cat >items-after.rs <<'EOF'
pub struct Kept {
    pub value: i32,
}

fn added() {
    secret_added();
}
EOF

cat >hidden-before.rs <<'EOF'
const CALLBACK: fn() -> i32 = || secret_closure_before();
const VALUE: i32 = { secret_const_before(); 1 };
fn signature(_: [u8; { secret_signature_before(); 3 }]) {
    secret_function_before();
}
struct Config {
    buffer: [u8; { secret_type_before(); 3 }],
}
enum Kind {
    Value = { secret_discriminant_before(); 1 },
}
macro_rules! opaque {
    () => { secret_macro_before() };
}
opaque! { fn generated() { secret_invocation_before(); } }
EOF
sed 's/_before/_after/g' hidden-before.rs >hidden-after.rs

cat >before.toml <<'EOF'
[package]
name = "secret_value_before"

[dependencies]
example = "1"
EOF
cat >after.toml <<'EOF'
[package]
name = "secret_value_after"

[dependencies]
example = "1"
EOF

cat >before.md <<'EOF'
# Guide

## Details

secret_prose_before

```rust
fn secret_markdown_before() {}
```
EOF
cat >after.md <<'EOF'
# Guide

## Details

secret_prose_after

```rust
fn secret_markdown_after() {}
```
EOF

cat >setext-before.md <<'EOF'
Guide
=====

Details
-------

secret_setext_before
EOF
sed 's/secret_setext_before/secret_setext_after/' setext-before.md >setext-after.md
printf '%s\n' 'secret_unheaded_before' >unheaded-before.md
printf '%s\n' 'secret_unheaded_after' >unheaded-after.md

# Git difftool can pass temporary paths with no useful extension.
cp body-before.rs temporary-before
cp body-after.rs temporary-after
cp body-before.rs unsupported.txt
printf '%s\n' 'fn malformed() { secret_unclosed_body();' >malformed.rs
: >empty.rs

# Context counts sibling declarations independently at each nesting level.
cat >context-before.rs <<'EOF'
struct Unrelated {
    hidden_field: u32,
}
impl Widget {
    fn first() {}
    fn previous() {}
    fn changed() { secret_before(); }
    fn next() {}
    fn last() {}
}
struct Tail;
EOF
sed 's/secret_before/secret_after/' context-before.rs >context-after.rs

# The renamed declaration crosses an unchanged anchor and also changes its body.
cat >rename-before.rs <<'EOF'
fn original(value: u32) -> u32 { secret_original(value) + 1 }
fn anchor() {}
EOF
cat >rename-after.rs <<'EOF'
fn anchor() {}
fn renamed(value: u32) -> u32 { secret_original(value) + 2 }
EOF

cat >words-before.rs <<'EOF'
impl Café {
	pub fn compute(
		&self,
		café: i32,
	) -> i32 { secret_before() }
}
EOF
sed -e 's/café/thé/g' -e 's/i32/u32/g' -e 's/secret_before/secret_after/' words-before.rs >words-after.rs

# Escape bytes in valid Markdown headings must never become terminal commands.
printf '# Before \033[31mheading\n\nsecret_before\n' >terminal-before.md
printf '# After \033[31mheading\n\nsecret_after\n' >terminal-after.md
