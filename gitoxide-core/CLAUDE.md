# gitoxide-core

The library implementing all capabilities of the gitoxide CLI.

**Tier:** CLI-glue
**Errors:** thiserror
**Notable:** Shared glue between the `gix` and `ein` binaries. Organized per subsystem: `pack/`, `repository/`, `index/`, `hours/`, `corpus/`, etc. New CLI features land here first, then get wired into `src/plumbing/` or `src/porcelain/`. This is the "do the thing" layer; `src/` is just Clap dispatch.

See root `CLAUDE.md` for branch discipline, agent roster (gix-architect / gix-steward / gix-runner), and the parity-loop structure.
