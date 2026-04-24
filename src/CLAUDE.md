# src — gix + ein binary source

Workspace-root binary target. Compiles into two CLIs:
- **`gix`** (plumbing CLI) — entry: `src/gix.rs`, dispatch: `src/plumbing/`
- **`ein`** (porcelain CLI) — entry: `src/ein.rs`, dispatch: `src/porcelain/`

Shared helpers in `src/shared.rs`. `src/uni.rs` is the single-binary variant.

A new gix command lands in two places: `src/plumbing/options/<cmd>.rs` (Clap `Subcommands` variant + args) and `src/plumbing/main.rs` (match arm that calls into `gitoxide-core`). Keep the CLI crate thin — real logic belongs in `gitoxide-core` and `gix-*`.

See root `CLAUDE.md` for branch discipline and agent roster.
