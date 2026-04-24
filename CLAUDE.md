# brit

A fork of [gitoxide](https://github.com/GitoxideLabs/gitoxide) (pure-Rust git implementation) that will eventually layer covenantal protocol extensions on top. **Current focus is git↔gix parity, not the brit vision** — parity first, covenant extensions later.

## Branches

| Branch | Purpose | Discipline |
|---|---|---|
| `main` | The brit covenant vision (paused pending parity) | Sebastian-spec when it resumes |
| `gix-main` | Pristine mirror of upstream gitoxide | **Never commit directly.** Only rebased-ready PRs land here, off clean cherry-picks from `gix-brit` |
| `gix-brit` | Active development for git↔gix parity | **Fire at will.** Speed over polish. `.expect("why")` OK. Cross-crate commits OK. Never `.unwrap()`. Never `just test` (pre-existing failures trip `set -eu`). |

Upstream handoff (gix-brit → PR against GitoxideLabs/gitoxide) is **hand-crafted and human-gated**, never automated by the loop. Repackage gix-brit commits into Sebastian-spec form on a branch off `gix-main` before opening the PR.

## Agents

| Agent | Model | When to invoke |
|---|---|---|
| `gix-architect` | sonnet | Design, C→Rust translation, implementation. The default working agent inside the loop. |
| `gix-steward` | opus | **Only three moments.** (1) Before emitting a completion promise, to verify claims. (2) When architect is genuinely blocked between two defensible designs. (3) When architect proposes deferring a row. Not per iteration. |
| `gix-runner` | haiku | Mechanical tasks — cargo matrix checks, greps, scaffolding from templates, structured output extraction. Fails fast on ambiguity. |

## The parity loop (gix-brit only)

- **Reference:** `vendor/git/` — upstream git source as submodule. Consult `vendor/git/builtin/<cmd>.c` and `vendor/git/Documentation/git-<cmd>.txt` before writing Rust.
- **Matrix:** `docs/parity/commands.md` (to be created) — top-level git-cmd × gix-cmd status.
- **Tests:** `tests/journey/parity/<cmd>.sh` (to be created) — one file per command, extending Sebastian's existing journey harness with an `expect_parity` helper.
- **Loop prompt:** `etc/parity/prompt.md` (to be created) — immutable ralph prompt, `$CMD`-substituted per invocation.

## Conventions (most live upstream)

Primary refs, in order of priority:
1. `DEVELOPMENT.md` — test-first, commit conventions, plumbing vs porcelain
2. `.github/copilot-instructions.md` — the AI-specific version of the above
3. `STABILITY.md` — tiered release cadence (T1/T2/T3)
4. `crate-status.md` — per-crate parity scoreboard
5. `SHORTCOMINGS.md` — **historical context, not a deferral whitelist.** Most entries are "unfinished," not "forbidden."

Hard rules worth repeating here:
- **No `.unwrap()`, ever** — `.expect("why")` with a genuine reason is the default replacement when `?` doesn't fit. `.unwrap_or*` variants are fine (not panics).
- **Byte-first paths** — `BString`/`&BStr`; cross to `OsStr`/`Path` only at OS boundaries.
- **Parametric hashing** — thread `gix_hash::Kind`; no literal 20.
- **Plumbing ≠ Porcelain** — plumbing takes references, no clones, feature-flag-aware; porcelain may clone Repository.
- **Commits follow purposeful conventional commits** — `feat:`/`fix:` only if user-visible; `change!:`/`rename!:`/`remove!:` for breaking.

## What NOT to do here

- Don't treat `SHORTCOMINGS.md` as permission to defer — most entries are closeable parity work.
- Don't modify files under `vendor/git/` — it's a submodule, git upstream.
- Don't commit brit-vision work to `gix-brit` (that belongs on `main`).
- Don't run `just test` during the parity loop — pre-existing failures (e.g., panic-behaviour snapshot drift) will trip `set -eu` before your test runs. Invoke single parity files directly.
- Don't `git push --force` to `gix-main` — it's pristine. Rebase-ready PRs only.

## Quick commands

```bash
just check                                      # build under suitable configs
just clippy                                     # lint only
cargo test -p <crate>                           # single-crate unit tests
cargo check -p gix --no-default-features --features <small|lean|max-pure|max>
cargo clippy --workspace --all-targets -- -D warnings -A unknown-lints --no-deps
cargo fmt                                       # before every commit

# Parity loop scaffolding once it exists
bash tests/parity.sh tests/journey/parity/<cmd>.sh   # single command's parity test
```

## Repo layout pointers

- `gix-*/` — plumbing crates (most of the workspace)
- `gix/` — porcelain library entry point
- `gitoxide-core/` — shared CLI glue for `gix` (plumbing) and `ein` (porcelain) binaries
- `src/` — the binary source for `gix` and `ein`
- `tests/journey/` — Sebastian's bash-driven end-to-end CLI tests
- `docs/plans/` — brit EPR vision plans (paused; **wrong branch for parity work**)
- `etc/plan/` — Sebastian's upstream parity plans (gix-error migration, sha256 support)
- `vendor/git/` — git upstream as submodule (reference only; read-only)
