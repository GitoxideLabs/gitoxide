# gix-regex

A byte-oriented POSIX regular expression engine maintained for gitoxide.

This crate is derived from [`posix-regex` 0.1.4](https://crates.io/crates/posix-regex/0.1.4),
originally developed for Redox's relibc. The imported upstream revision is
[`063882aa6051b5075062d20d00b41ddc0fb8cd89`](https://gitlab.redox-os.org/redox-os/posix-regex/-/commit/063882aa6051b5075062d20d00b41ddc0fb8cd89).
The original MIT license and copyright notice are preserved in `LICENSE-MIT`.

The parser supports basic and extended expressions, ASCII byte classes, captures,
backreferences, and GNU word-boundary escapes. An optional `no_std` build uses `alloc`.
Matching selects the earliest start and longest complete match there; equal-length
matches preserve alternative order and greedy repetition.

Pattern size and group nesting are limited during compilation. Matching is fallible
and has configurable work and capture-state memory budgets, shared by all matches
and retained results of a search. Exhaustion returns an error, never a partial match.

The original regression tests are retained, together with Git-generated compatibility
fixtures. The original matcher has been replaced to correct empty alternatives,
zero repetitions and capture precedence, and to bound matching resources.
