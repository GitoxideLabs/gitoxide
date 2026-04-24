# gix-worktree-stream

Generate a byte-stream from a git tree (archive-like, internal format).

**Tier:** plumbing
**Errors:** gix-error
**Notable:** Entries must be read to exhaustion; dropping early taints the stream and panics next `next_entry()`.

See root `CLAUDE.md` for branch discipline, agent roster (gix-architect / gix-steward / gix-runner), and the parity-loop structure.
