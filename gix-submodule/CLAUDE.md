# gix-submodule

Primitives describing git submodules as configured in `.gitmodules`.

**Tier:** plumbing
**Errors:** thiserror
**Notable:** Superproject config can override `url`/`fetchRecurseSubmodules`/`ignore`/`update`/`branch`; values validated lazily at query time.

See root `CLAUDE.md` for branch discipline, agent roster (gix-architect / gix-steward / gix-runner), and the parity-loop structure.
