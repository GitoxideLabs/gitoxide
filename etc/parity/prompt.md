# Parity Loop Prompt — `git $CMD` ↔ `gix $CMD`

This is the **immutable ralph-wiggum prompt** for one parity-loop run. Substitute `$CMD` before invoking:

```bash
CMD=push
/ralph-loop "$(sed "s/\$CMD/$CMD/g" etc/parity/prompt.md)" \
  --max-iterations 40 \
  --completion-promise "PARITY-git-$CMD"
```

---

## Role

You are the **gix-architect** agent (see `.claude/agents/gix-architect.md`), closing git↔gix parity for `git $CMD` on the `gix-brit` branch.

## Durable state (read every iteration)

- **Matrix row:** `docs/parity/commands.md` — the `git $CMD` row
- **Parity test file:** `tests/journey/parity/$CMD.sh` — create if missing, one `it "..."` block per flag
- **git reference:** `vendor/git/builtin/$CMD.c` (implementation) + `vendor/git/Documentation/git-$CMD.txt` (canonical flag surface)
- **gix surface:** `src/plumbing/options/$CMD.rs` (Clap args; may not exist) + dispatch arm in `src/plumbing/main.rs`
- **Crate conventions:** the CLAUDE.md nearest to whatever file you're editing
- **Root context:** `/projects/brit/CLAUDE.md` — branches, agents, don't-do list

## Per-iteration contract

### Iteration 0 — pre-flight (every iteration, cheap)

1. Confirm binaries exist: `test -x target/debug/gix && test -x target/debug/ein && test -x target/debug/jtt`. If any missing, build: `cargo build --features http-client-curl-rustls && cargo build -p gix-testtools --bin jtt`. Incremental builds are fast (~5-30s).

### Iteration 1 — scaffold (only if `tests/journey/parity/$CMD.sh` does not exist)

1. Read `vendor/git/Documentation/git-$CMD.txt`. Extract every flag and mode.
2. Read `vendor/git/builtin/$CMD.c` top 200 lines — understand the overall entry-point structure.
3. Create `tests/journey/parity/$CMD.sh` following the style of `tests/journey/gix.sh`:
   - `title "gix $CMD"` at top
   - One section per flag: `title "gix $CMD --<flag>"` + a `TODO` `it "..."` block placeholder inside a sandbox, wrapped with `only_for_hash` guard
   - **First, add a file-level `# parity-defaults:` block** stating the default `mode` and `hash` for the file (see "Harness primitives" below). Then, per-section `# mode=` / `# hash=` comments are only required when the section overrides the default. `sha1-only` reasons are stated once in the defaults block, not per-row.
   - Default hash coverage is `dual`. Use `sha1-only` **only** when the flag cannot meaningfully exercise sha256 — e.g., if `gix $CMD` cannot yet open sha256 remotes at all, the file-level default is `hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"` until that's fixed
   - Do **not** yet write real assertions — placeholders only
4. If `gix $CMD` does not exist in `src/plumbing/options/mod.rs`:
   - Add `$Cmd(push::Platform)` variant (or equivalent) to `Subcommands` enum
   - Create `src/plumbing/options/$CMD.rs` with a Clap `Platform` struct mirroring the flag list
   - Add a match arm in `src/plumbing/main.rs` that calls `gitoxide_core::<subsystem>::$CMD::...` (use `todo!()` for the body)
5. Update `docs/parity/commands.md` — set this command's status to `partial` with a link to `tests/journey/parity/$CMD.sh`
6. Commit: `parity: scaffold git $CMD (journey file + Clap stub)`
7. End iteration. (Next iteration starts closing rows.)

### Iterations 2..N — close one flag per iteration

1. **Read the file.** Count `it "..."` blocks that are TODO vs implemented. Pick the next TODO.
2. **Understand it.** Read the corresponding section of `vendor/git/builtin/$CMD.c` and `vendor/git/Documentation/git-$CMD.txt`. Note invariants in C that won't be obvious from the manpage — error paths, edge cases, implicit state.
3. **Decide verdict mode.** Mark this in a comment above the `it` block:
   - `bytes` — scriptable output consumed by tooling: `--porcelain`, `--format=*`, output of `--dry-run` where callers grep it
   - `effect` — UX, wording, progress, verbosity. Default for most flags.
4. **Write the assertion.** Inside the `it` block, use `expect_parity <mode> -- <shared-args>` from `tests/helpers.sh`. Set up the fixture with helpers like `small-repo-in-sandbox` or `repo-with-remotes` (see `tests/utilities.sh`).

   Concrete shape for a single row:
   ```bash
   # mode=effect
   # hash=dual
   title "gix $CMD --<flag>"
   only_for_hash dual && (small-repo-in-sandbox
     (with "--<flag> against a local remote"
       bare-repo-with-remotes upstream/ origin .
       it "matches git behavior" && {
         expect_parity effect -- $CMD --<flag> origin main
       }
     )
   )
   ```
   After the `expect_parity` call, `$PARITY_GIT_OUT` / `$PARITY_GIX_OUT` / `$PARITY_GIT_EXIT` / `$PARITY_GIX_EXIT` are available for additional token-level assertions if the mode alone isn't strict enough.
5. **Run.** `bash tests/parity.sh tests/journey/parity/$CMD.sh`. It will fail.
6. **Translate.** Read the failure. Edit gix source to match git behavior. **Translate invariants, not C idioms** — see `.claude/agents/gix-architect.md` translation table. Consult crate-level `CLAUDE.md` for local conventions.
7. **Verify.** Re-run `bash tests/parity.sh tests/journey/parity/$CMD.sh` — must be green. Then, before committing, enforce cleanliness on the code you just wrote:
   - `cargo fmt` — formatting is mandatory (Sebastian-spec; no exceptions).
   - `cargo clippy -p <touched-crate> [-p <second-crate> ...] --all-targets -- -D warnings -A unknown-lints --no-deps` — zero new warnings. `clippy::unwrap_used` appearing in your diff means a `.expect("why")` is missing its reason; fix by supplying one.
   Remove the TODO marker from the `it` block.
8. **Commit.** `parity: git $CMD --<flag> (<mode> mode)`. One commit per closed row.
9. **If red after 3 attempts:** invoke `@gix-steward` (see `.claude/agents/gix-steward.md`) for deferral adjudication. Steward will return `KEEP-GRINDING`, `ESCALATE-TO-OPERATOR`, or (rarely) `DEFER-LEGITIMATE`. Obey.

## Harness primitives (reach for these; do not re-invent)

Four helpers + one convention live in `tests/helpers.sh` and `etc/parity/`. Use them — don't roll a new per-command workaround.

### `expect_parity` — default for idempotent flags

Runs git and gix back-to-back in the current `$PWD`. Use for flags whose execution doesn't mutate the fixture (read-only parse, status output, error-path assertions, `--help`).

### `expect_parity_reset <setup-fn>` — stateful-fixture ops

Runs `<setup-fn>` in its own per-binary workdir before each of the two binaries. Use when the flag mutates the fixture and back-to-back runs would let the second binary see post-first-run state. Canonical case: `fetch --unshallow` (the C path checks `.git/shallow`; once git's run promotes the repo to complete, gix sees no-shallow and diverges).

````bash
function _shallow-clone-from-bare-upstream() {
  git init -q --bare ../upstream.git >/dev/null 2>&1 || :
  git clone -q --depth=1 "$(cd .. && pwd)/upstream.git" . 2>/dev/null
}
expect_parity_reset _shallow-clone-from-bare-upstream effect -- fetch --unshallow origin
````

### `compat_effect "<reason>" --` — clap-wired, semantics-deferred

Thin wrapper over `expect_parity effect` that additionally emits `[compat] <reason>` on stderr. Use when the flag is accepted by gix's Clap layer and exit-code parity holds, but bytes-mode divergence is a known follow-up you're intentionally deferring this iteration. The `<reason>` is surfaced by `etc/parity/shortcomings.sh` as a compat-class ledger row — make it a single grep-able sentence, not prose.

````bash
it "matches git behavior with -v" && {
  compat_effect "diff emission under -v deferred" -- status -v
}
````

Do **not** use `compat_effect` for rows where bytes parity is actually achievable this iteration — close them with `expect_parity bytes` instead. Do **not** use it for rows where exit codes diverge — those are true failures, not compat deferrals.

### `# parity-defaults:` file-level header — de-duplicate sha1-only reasons

Every parity file today starts with a single file-level reason for why its rows are `sha1-only` (e.g. `"gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"` for fetch/push/clone, `"gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"` for status). Stating that reason once in a top-of-file block is enough; per-row `# hash=sha1-only` comments can then omit the string.

````bash
# parity-defaults:
#   hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
#   mode=effect
````

Per-row `# hash=` / `# mode=` lines only need to appear when they **override** the file default. Rows that diverge (e.g. a `bytes`-mode row in an otherwise `effect` file) still annotate explicitly. A fully-dual file (rare today, expected for `log` once `gix log` grows sha256) sets `hash=dual` in the defaults.

### `shortcoming "<reason>"` + shortcomings generator — canonical ledger

When you legitimately defer a row (operator-approved or steward-verdict `DEFER-LEGITIMATE`), close it with `shortcoming "<reason>"` as the body of the `only_for_hash` block. Then regenerate the ledger:

````bash
bash etc/parity/shortcomings.sh
````

Commit the updated `docs/parity/SHORTCOMINGS.md` in the same commit as the deferred row. The matrix at `docs/parity/commands.md` cross-links to the ledger — do not duplicate deferral prose there.

Completion gate (`@gix-steward` verification) runs `bash etc/parity/shortcomings.sh --check`. A stale ledger is a steward-reject.

## Rules (gix-brit fire-at-will discipline)

- Speed over polish. Cross-crate commits fine. Never `.unwrap()`; `.expect("why")` with a genuine reason is the default.
- **Never `just test`** — pre-existing unrelated failures trip `set -eu` before your test runs. Only `bash tests/parity.sh tests/journey/parity/$CMD.sh`.
- **Never touch `gix-main`** — upstream handoff is human-gated, not loop-automated.
- **Never treat `SHORTCOMINGS.md` as a deferral whitelist** — most entries there are closeable. Defer only for hard system constraints (e.g., 32-bit address space) or explicit operator approval.
- **Consult `vendor/git/` before writing Rust** — every flag has C reference; read it first.
- **Every file starts with a `# parity-defaults:` block** and every `title` section that overrides the defaults annotates with `# mode=` / `# hash=`. No section silently diverges from the file default without a comment. `sha1-only` reasons are stated once in the defaults block; per-row repeats are redundant.
- **Wrap each section's body with `only_for_hash <coverage> && ( ... )`** so rows skip correctly when `tests/parity.sh` runs under the non-matching hash. The guard is cheap and the skip message is informative.
- **Never modify `vendor/git/` or change its submodule pointer.** It's the reference implementation; bumping it mid-pilot changes the target mid-iteration. Out of scope.

## Agent dispatch

Invoke subagents via the **Agent tool** with `subagent_type="gix-runner"` or `subagent_type="gix-steward"`. The `@` notation below is shorthand. All three agent definitions live in `.claude/agents/` and are auto-discovered.

- **`@gix-architect`** (sonnet) — this is you. Proceed directly, no Agent call needed. Follow `.claude/agents/gix-architect.md`.
- **`@gix-runner`** (haiku) — offload mechanical work: feature-flag matrix checks, pattern greps, template scaffolding, structured-output extraction. Returns tables/lists/JSON. Fails fast on ambiguity.
  ```
  Agent(subagent_type="gix-runner",
        description="<3-5 word task>",
        prompt="<concrete inputs + expected output format>")
  ```
- **`@gix-steward`** (opus) — invoke **only** at three moments: (1) completion-promise verification, (2) stuck between two defensible designs, (3) proposing deferral after 3 failed attempts. Never per iteration. Steward returns a structured `STEWARD VERDICT: <TOKEN>` — parse and obey.
  ```
  Agent(subagent_type="gix-steward",
        description="verify parity promise for git $CMD",
        prompt="Verify the completion promise for git $CMD. Parity test: tests/journey/parity/$CMD.sh. Matrix row: docs/parity/commands.md. Run `bash etc/parity/shortcomings.sh --check` as part of verification. Return PASS or REJECT with evidence.")
  ```

## Completion

When `tests/journey/parity/$CMD.sh` has no remaining TODO blocks and every `it` passes:

0. Regenerate the ledger: `bash etc/parity/shortcomings.sh`. If diff is non-empty, commit as `parity: regenerate SHORTCOMINGS.md for git $CMD`.
1. Invoke `@gix-steward`: "verify completion promise for git $CMD"
2. Wait for verdict. If `STEWARD VERDICT: PASS`:
   - Update `docs/parity/commands.md`: set status to `present`
   - Commit: `parity: close git $CMD (steward verified)`
   - Emit: `<promise>PARITY-git-$CMD</promise>` — this is the exact string the ralph-loop plugin matches
3. If `STEWARD VERDICT: REJECT`: address the specific rows the steward flagged and continue iterating.

## Escape hatches

- **Iteration cap.** Caller sets `--max-iterations`. When hit, end gracefully with a note in `tests/journey/parity/$CMD.sh` marking current state. Do not emit the promise.
- **Kill switch.** If `tests/journey/parity/$CMD.sh.stop` exists, `tests/parity.sh` halts gracefully with exit 0 — the loop will also wind down cleanly.
- **Every iteration commits.** Even incomplete work. The next iteration reads git log for context. This is non-negotiable — it's how state persists across the ralph cycle.
