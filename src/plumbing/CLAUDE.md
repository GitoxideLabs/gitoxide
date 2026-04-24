# src/plumbing — gix CLI dispatch

Clap options + main dispatch for the **`gix`** binary (plumbing CLI — scripts, tooling, `--format json`, quiet by default).

- `options/mod.rs` — `Subcommands` enum, the canonical list of top-level `gix` subcommands
- `options/<cmd>.rs` — per-command args
- `main.rs` — match arms calling `gitoxide-core`

Porcelain output conventions (from DEVELOPMENT.md): plumbing is quiet, UTC timestamps, progress via `--verbose` only. Feature flags (`small` / `lean` / `max-pure` / `max`) gate networking, async, color. New subcommands must compile under all variants.

See root `CLAUDE.md` for branch discipline and agent roster.
