# tests — gitoxide test harness

Structure:
- `tests/journey/{gix,ein}.sh` — Sebastian's bash end-to-end CLI tests. Sourced from `tests/journey.sh`.
- `tests/journey/parity/<cmd>.sh` (to be created) — per-command git↔gix parity tests using `expect_parity` helper.
- `tests/helpers.sh` / `tests/utilities.sh` — shared bash helpers (`title`, `when`, `with`, `it`, `expect_run`, snapshot mechanism).
- `tests/fixtures/` — tar-archived test fixture repos.
- `tests/snapshots/` — auto-populated expected-output snapshots (created on first run, committed, then diffed thereafter).
- `tests/tools/` — `gix-testtools` + the `jtt` journey-test-tool binary.
- `tests/it/` — Rust integration tests.

Key idioms:
- Sections use `title "..."` on a line (greppable). Nested context via `(when "...")` / `(with "...")` subshells.
- Assertions: `it "..." && { expect_run $EXPECTED_EXIT_CODE <cmd> ... }`.
- `WITH_SNAPSHOT=<path> expect_run 0 <cmd>` auto-populates snapshots on first run — free fixtures.
- **Do not** run `just test` during the parity loop — pre-existing unrelated failures trip `set -eu` before your test reaches the suite.
- Invoke a single parity file directly: `bash tests/parity.sh tests/journey/parity/<cmd>.sh` (wrapper to be created).

See root `CLAUDE.md` for the parity loop structure.
