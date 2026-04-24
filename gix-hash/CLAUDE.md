# gix-hash

Borrowed and owned git hash digests used to identify git objects.

**Tier:** plumbing
**Errors:** thiserror
**Notable:** Hash-kind parametric; avoid `[u8; 20]`, thread `gix_hash::Kind` through APIs.

See root `CLAUDE.md` for branch discipline, agent roster (gix-architect / gix-steward / gix-runner), and the parity-loop structure.
