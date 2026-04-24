# gix-protocol

Implements git wire protocols (V0/V1/V2) including handshake, ls-refs, and fetch.

**Tier:** plumbing
**Errors:** thiserror
**Notable:** V1 stateful connections (ssh/git/file) may hang; does not affect cloning (per SHORTCOMINGS.md).

See root `CLAUDE.md` for branch discipline, agent roster (gix-architect / gix-steward / gix-runner), and the parity-loop structure.
