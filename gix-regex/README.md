# gix-regex

A byte-oriented POSIX regular expression engine maintained for gitoxide.

This crate is derived from [`posix-regex` 0.1.4](https://crates.io/crates/posix-regex/0.1.4),
originally developed for Redox's relibc. The imported upstream revision is
[`063882aa6051b5075062d20d00b41ddc0fb8cd89`](https://gitlab.redox-os.org/redox-os/posix-regex/-/commit/063882aa6051b5075062d20d00b41ddc0fb8cd89).
The original MIT license and copyright notice are preserved in `LICENSE-MIT`.

The import supports basic and extended expressions, byte classes, captures,
backreferences, and an optional `no_std` build. Its original regression tests
are retained. Git compatibility fixes will be added before this engine is used
to interpret Git configuration; this initial import is not yet a conformant
replacement for Git's regular expression implementation.
