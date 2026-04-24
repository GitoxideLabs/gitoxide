# gix-index

Read, decode, verify, and write the git index file across V2/V3/V4.

**Tier:** plumbing
**Errors:** thiserror
**Notable:** `link` extension readable but not writable; a mutating op disables split-index (per SHORTCOMINGS.md).

See root `CLAUDE.md` for branch discipline, agent roster (gix-architect / gix-steward / gix-runner), and the parity-loop structure.
