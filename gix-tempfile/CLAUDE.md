# gix-tempfile

A tempfile implementation with a global registry to assure cleanup.

**Tier:** plumbing
**Errors:** none
**Notable:** ST2-stable; signal-aware cleanup — call `signal::setup()` before creating the first tempfile.

See root `CLAUDE.md` for branch discipline, agent roster (gix-architect / gix-steward / gix-runner), and the parity-loop structure.
