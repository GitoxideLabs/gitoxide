# gix-features

A crate to integrate various capabilities using compile-time feature flags.

**Tier:** plumbing
**Errors:** thiserror
**Notable:** Threading primitives live at `gix_features::threading::*`; pick the Sync-safe variants. Interior-mutability types switch implementation with the `parallel` / `once_cell` features.

See root `CLAUDE.md` for branch discipline, agent roster (gix-architect / gix-steward / gix-runner), and the parity-loop structure.
