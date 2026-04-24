# tests/it — integration-test crate

Rust integration tests that exercise the workspace end-to-end. Not the journey tests (those are bash in `tests/journey/`).

Use when: a test needs Rust-level API surface access, fixture repository setup via `gix-testtools`, or cross-crate integration. Use journey tests when testing CLI-level behavior.

See root `CLAUDE.md` for branch discipline.
