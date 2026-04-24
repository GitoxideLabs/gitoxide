# gix-transport

Implements the git transport layer abstracting over ssh, git, http(s), and file schemes.

**Tier:** plumbing
**Errors:** thiserror
**Notable:** Async variants are feature-gated (`async-client`/`async-transport`); blocking and async are mutually exclusive.

See root `CLAUDE.md` for branch discipline, agent roster (gix-architect / gix-steward / gix-runner), and the parity-loop structure.
