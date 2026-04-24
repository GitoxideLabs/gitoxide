# src/porcelain — ein CLI dispatch

Clap options + main dispatch for the **`ein`** binary (porcelain CLI — human-facing, verbose by default, localtime, stderr progress).

- `options.rs` — `Subcommands` enum
- `main.rs` — match arms calling `gitoxide-core`

`ein` is the workflow-tool companion, not a git-command clone. Things like `ein tool organize` have **no git counterpart** and are out of parity-loop scope (see `docs/parity/commands.md` — ein is enumerated separately, marked brit-native).

See root `CLAUDE.md` for branch discipline and agent roster.
