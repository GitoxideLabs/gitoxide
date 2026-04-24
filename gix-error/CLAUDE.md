# gix-error

A crate of the gitoxide project to provide common errors and error-handling utilities.

**Tier:** plumbing
**Errors:** none
**Notable:** Defines the `Exn<Message>` / `or_raise` / `message()` pattern. Ongoing migration target — as of 2026-04-22, 32 of ~65 plumbing crates migrated, 33 pending; see `etc/plan/gix-error.md` for the checklist and `src/lib.rs` module docs for the canonical usage guide. Per-crate choice is binary: if the crate's `Cargo.toml` lists `gix-error`, use it throughout; otherwise stay on `thiserror` until the migration commit lands.

See root `CLAUDE.md` for branch discipline, agent roster (gix-architect / gix-steward / gix-runner), and the parity-loop structure.
