# tests/tools — gix-testtools + jtt

Provides:
- `gix-testtools` crate — fixture-repo setup helpers, `Result` alias for tests, archive extraction (`GIX_TEST_IGNORE_ARCHIVES=1` skips on macOS/Windows)
- `jtt` binary — the **j**ourney-**t**est **t**ool, invoked by `tests/journey.sh` as the third positional arg

Journey-test fixture scripts produce tar archives under `tests/fixtures/`. When adding a new fixture script, validate on macOS/Windows with `GIX_TEST_IGNORE_ARCHIVES=1` set.

See root `CLAUDE.md` for branch discipline and agent roster.
