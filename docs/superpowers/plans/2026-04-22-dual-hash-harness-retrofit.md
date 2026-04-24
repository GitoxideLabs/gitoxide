# Dual-Hash (SHA-1 / SHA-256) Parity Harness Retrofit

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the parity test harness dual-hash-ready so every closed parity row is validated under both SHA-1 and SHA-256 fixtures (or explicitly flagged `sha1-only` with justification). Prevents compounding rework as more commands close.

**Architecture:** Follow Sebastian's existing `GIX_TEST_FIXTURE_HASH` env-var pattern used workspace-wide in `justfile` for dual-hash crate tests. Fixture helpers in `tests/utilities.sh` respect the env var when calling `git init`. `tests/parity.sh` re-runs the target file once per hash kind. Per-row coverage is annotated with a `# hash=<dual|sha1-only>` comment above each `title` block (parallel to existing `# mode=` comments). A helper `only_for_hash` short-circuits rows that should skip under the current hash env. Steward's completion-gate enforces `hash=dual` or operator-approved `sha1-only`.

**Tech Stack:** Bash (journey harness), `git init --object-format={sha1,sha256}` (git 2.47+), no new Rust.

---

## Conflict-risk acknowledgment

A parallel ralph session is running the `git push` pilot on the same `gix-brit` branch. Its touched files:

- `src/plumbing/options/push.rs`, `src/plumbing/main.rs`, `src/plumbing/options/mod.rs`
- `gitoxide-core/src/repository/push.rs`, `gitoxide-core/src/repository/mod.rs`
- `tests/journey/parity/push.sh`
- `docs/parity/commands.md` (per-row status updates)

**This plan does NOT modify any of those.** Our files:

- `tests/helpers.sh` (add helpers)
- `tests/utilities.sh` (modify fixture init)
- `tests/parity.sh` (loop over hash kinds)
- `tests/journey/parity/_smoke.sh` (extend smoke)
- `.claude/agents/gix-steward.md` (completion-gate update)
- `etc/parity/prompt.md` (iteration contract)

**`docs/parity/commands.md`: explicit non-target.** Original spec suggested a hash-coverage column; we defer that to avoid conflict with the pilot's row-state updates. Per-row hash coverage is tracked **inside each parity `.sh` file** via `# hash=` comments, not in the matrix. This also keeps the matrix a thin index and pushes detail where the detail lives.

## File structure

| File | Action | Responsibility |
|---|---|---|
| `tests/helpers.sh` | modify | Add `only_for_hash <coverage>` helper; update `expect_parity` OK line to show active hash |
| `tests/utilities.sh` | modify | Extend `small-repo-in-sandbox`, `repo-with-remotes`, `bare-repo-with-remotes` to pass `--object-format=$GIX_TEST_FIXTURE_HASH` (defaulting to sha1) to `git init` |
| `tests/parity.sh` | modify | Wrap the `source "$target"` call in a loop over `sha1` and `sha256`, setting `GIX_TEST_FIXTURE_HASH` per iteration |
| `tests/journey/parity/_smoke.sh` | modify | Add a row that asserts the active hash matches what's baked into the sandbox's `.git/config` |
| `.claude/agents/gix-steward.md` | modify | Completion-gate REJECT unless every row in the parity `.sh` has `# hash=dual` OR `# hash=sha1-only <reason>` |
| `etc/parity/prompt.md` | modify | Add iteration-contract rule: every new `it` block must have `# hash=` comment; default `dual`; `sha1-only` requires a reason string |

---

## Task 1: Add `only_for_hash` helper to `tests/helpers.sh`

**Files:**
- Modify: `tests/helpers.sh` (append at end, after `launch-git-daemon`)
- Test: `tests/journey/parity/_smoke.sh` (new section exercising the helper)

**Context:** `only_for_hash <coverage>` returns 0 when the current `$GIX_TEST_FIXTURE_HASH` (default `sha1`) is in the coverage set, else 1 with a terse skip message to stderr. Coverage values: `dual` (always run), `sha1-only` (run under sha1, skip under sha256). Used via `only_for_hash dual && (small-repo-in-sandbox ...)` — the subshell runs only when the guard returns 0.

- [ ] **Step 1: Write the failing test**

Append to `tests/journey/parity/_smoke.sh`:
```bash
(with "only_for_hash guard"
  saved_hash="${GIX_TEST_FIXTURE_HASH:-sha1}"

  export GIX_TEST_FIXTURE_HASH=sha1
  it "runs under sha1 when coverage=dual" && {
    only_for_hash dual && expect_run 0 true
  }
  it "runs under sha1 when coverage=sha1-only" && {
    only_for_hash sha1-only && expect_run 0 true
  }

  export GIX_TEST_FIXTURE_HASH=sha256
  it "runs under sha256 when coverage=dual" && {
    only_for_hash dual && expect_run 0 true
  }
  it "skips under sha256 when coverage=sha1-only" && {
    if only_for_hash sha1-only; then
      fail "expected only_for_hash sha1-only to return non-zero under sha256"
    fi
    echo 1>&2 "${GREEN} - OK (skip path taken)"
  }

  GIX_TEST_FIXTURE_HASH="$saved_hash"
)
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /projects/brit && bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -20
```
Expected: FAIL with `only_for_hash: command not found` (function doesn't exist yet).

- [ ] **Step 3: Implement `only_for_hash` in `tests/helpers.sh`**

Append after the `launch-git-daemon` function:
```bash

# only_for_hash <coverage> — returns 0 if the active GIX_TEST_FIXTURE_HASH
# is in the coverage set, else 1 (caller's subshell short-circuits).
#
# Coverage values:
#   dual       — always run (SHA-1 and SHA-256)
#   sha1-only  — run under sha1, skip under sha256 (legitimate when
#                the feature genuinely can't exercise hashing, e.g.,
#                operating on a remote that gix doesn't yet support
#                in sha256 mode)
function only_for_hash() {
  local want="${1:?only_for_hash: need coverage (dual|sha1-only)}"
  local have="${GIX_TEST_FIXTURE_HASH:-sha1}"
  case "$want" in
    dual) return 0 ;;
    sha1-only)
      if [[ "$have" == "sha1" ]]; then
        return 0
      else
        echo 1>&2 "${YELLOW}  [hash=$have] skipped (row coverage: sha1-only)"
        return 1
      fi
      ;;
    *)
      echo 1>&2 "${RED}only_for_hash: unknown coverage '$want' (want dual|sha1-only)"
      return 2
      ;;
  esac
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd /projects/brit && bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -20
```
Expected: all four `it` blocks report OK (the "skips under sha256" case prints the skip line AND "OK (skip path taken)").

- [ ] **Step 5: Commit**

```bash
cd /projects/brit && git add tests/helpers.sh tests/journey/parity/_smoke.sh
git commit -m "test: add only_for_hash guard helper for dual-hash parity rows"
```

---

## Task 2: Surface active hash in `expect_parity` OK message

**Files:**
- Modify: `tests/helpers.sh` (inside `expect_parity` function)
- Test: `tests/journey/parity/_smoke.sh` (existing row — verify output includes hash)

**Context:** The existing `expect_parity` prints `"- OK (effect parity, exit=0)"`. Under dual-hash runs we need to know which hash each row passed under. Change the OK line to include the active hash.

- [ ] **Step 1: Write the failing test**

Append to `tests/journey/parity/_smoke.sh` inside the existing "expect_parity exit-code match path" `(with ...)` block, AFTER the existing `it`:
```bash
    it "OK message includes active hash" && {
      export GIX_TEST_FIXTURE_HASH=sha256
      out="$(expect_parity effect -- --version 2>&1 || true)"
      unset GIX_TEST_FIXTURE_HASH
      if [[ "$out" != *"hash=sha256"* ]]; then
        fail "expect_parity OK line missing hash tag; got: $out"
      fi
      echo 1>&2 "${GREEN} - OK (hash tag present)"
    }
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /projects/brit && bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -15
```
Expected: FAIL with "expect_parity OK line missing hash tag".

- [ ] **Step 3: Modify `expect_parity` OK line**

In `tests/helpers.sh`, find the final OK line in `expect_parity`:
```bash
  echo 1>&2 "${GREEN} - OK ($mode parity, exit=$git_exit)"
```
Replace with:
```bash
  local active_hash="${GIX_TEST_FIXTURE_HASH:-sha1}"
  echo 1>&2 "${GREEN} - OK ($mode parity, hash=$active_hash, exit=$git_exit)"
```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd /projects/brit && bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -15
```
Expected: all `it` blocks pass; OK lines show `hash=sha1` (default) or `hash=sha256` (when env set).

- [ ] **Step 5: Commit**

```bash
cd /projects/brit && git add tests/helpers.sh tests/journey/parity/_smoke.sh
git commit -m "test: surface active hash in expect_parity OK message"
```

---

## Task 3: Extend fixture helpers to respect `GIX_TEST_FIXTURE_HASH`

**Files:**
- Modify: `tests/utilities.sh` (three functions: `small-repo-in-sandbox`, `repo-with-remotes`, `bare-repo-with-remotes`)
- Test: `tests/journey/parity/_smoke.sh`

**Context:** `git init` accepts `--object-format=sha1|sha256`. Default is sha1 unless the user passes the flag or sets `init.defaultObjectFormat`. We pass `--object-format=$GIX_TEST_FIXTURE_HASH` unconditionally, defaulting the env to `sha1`. The resulting repo's `.git/config` will have `[extensions] objectformat = sha256` when sha256 is selected.

- [ ] **Step 1: Write the failing test**

Append to `tests/journey/parity/_smoke.sh`:
```bash
(with "fixture helpers respect GIX_TEST_FIXTURE_HASH"
  saved_hash="${GIX_TEST_FIXTURE_HASH:-sha1}"

  export GIX_TEST_FIXTURE_HASH=sha256
  (small-repo-in-sandbox
    it "small-repo-in-sandbox honors sha256" && {
      format="$(git config --local extensions.objectformat 2>/dev/null || echo sha1)"
      if [[ "$format" != "sha256" ]]; then
        fail "expected sha256, got $format"
      fi
      echo 1>&2 "${GREEN} - OK (small-repo-in-sandbox → sha256)"
    }
  )

  (bare-repo-with-remotes "$(mktemp -d)" nothing _
    it "bare-repo-with-remotes honors sha256" && {
      true  # placeholder; real check below after we drop this assertion
    }
  )

  GIX_TEST_FIXTURE_HASH="$saved_hash"
)
```

Note: `bare-repo-with-remotes` returns early if not given an even number of (remote, url) args. The test above triggers its error path intentionally — we'll refine in step 3. For now the point is to see the first `it` fail before the helpers are updated.

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /projects/brit && bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -20
```
Expected: FAIL — "expected sha256, got sha1" on the small-repo-in-sandbox assertion (because the helper currently calls bare `git init`).

- [ ] **Step 3: Update fixture helpers to pass `--object-format`**

In `tests/utilities.sh`, add this helper near the top (after `set-static-git-environment`):
```bash

# git-init-hash-aware — drop-in replacement for `git init` in fixture
# helpers. Respects GIX_TEST_FIXTURE_HASH (sha1 default).
function git-init-hash-aware() {
  git init --object-format="${GIX_TEST_FIXTURE_HASH:-sha1}" "$@"
}
```

Replace each `git init` call in these three functions with `git-init-hash-aware`:

- In `small-repo-in-sandbox` (~line 62), change:
  ```bash
  git init
  ```
  to:
  ```bash
  git-init-hash-aware
  ```

- In `repo-with-remotes` (~line 28), change:
  ```bash
  git init
  ```
  to:
  ```bash
  git-init-hash-aware
  ```

- In `bare-repo-with-remotes` (~line 51), change:
  ```bash
  git init --bare
  ```
  to:
  ```bash
  git-init-hash-aware --bare
  ```

- [ ] **Step 4: Run test to verify it passes**

```bash
cd /projects/brit && bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -25
```
Expected: the small-repo-in-sandbox → sha256 assertion passes.

- [ ] **Step 5: Drop the placeholder bare-repo assertion and add a real one**

Replace the `bare-repo-with-remotes` section in the smoke test with:
```bash
  export GIX_TEST_FIXTURE_HASH=sha256
  bare_target="$(mktemp -d)/bare.git"
  bare-repo-with-remotes "$bare_target" origin /tmp/whatever
  (cd "$bare_target"
    it "bare-repo-with-remotes honors sha256" && {
      format="$(git config --local extensions.objectformat 2>/dev/null || echo sha1)"
      if [[ "$format" != "sha256" ]]; then
        fail "expected sha256 in bare repo, got $format"
      fi
      echo 1>&2 "${GREEN} - OK (bare-repo-with-remotes → sha256)"
    }
  )
  rm -rf "$(dirname "$bare_target")"
```

- [ ] **Step 6: Re-run smoke**

```bash
cd /projects/brit && bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -25
```
Expected: both fixture-helper assertions pass.

- [ ] **Step 7: Commit**

```bash
cd /projects/brit && git add tests/utilities.sh tests/journey/parity/_smoke.sh
git commit -m "test: fixture helpers respect GIX_TEST_FIXTURE_HASH for dual-hash parity"
```

---

## Task 4: Update `tests/parity.sh` to loop over hash kinds

**Files:**
- Modify: `tests/parity.sh`
- Test: `tests/journey/parity/_smoke.sh` (already exercises dual behavior via its existing assertions once they run twice)

**Context:** Currently `tests/parity.sh` sources the target file once. We wrap that `source` in a loop over `sha1 sha256`, setting `GIX_TEST_FIXTURE_HASH` for each iteration. The loop announces the active hash with a clear header so downstream parsers (and the steward) can section output per hash.

- [ ] **Step 1: Write the failing test**

This one is naturally a verification-by-output check. Create this assertion tooling at the end of `tests/journey/parity/_smoke.sh` (will only pass once Task 4 implementation lands):
```bash
(with "parity.sh runs each file under both hash kinds"
  # We are inside parity.sh right now. Check that the outer caller set
  # GIX_TEST_FIXTURE_HASH to something (either sha1 or sha256) — not empty.
  it "GIX_TEST_FIXTURE_HASH is set by the runner" && {
    if [[ -z "${GIX_TEST_FIXTURE_HASH:-}" ]]; then
      fail "GIX_TEST_FIXTURE_HASH not set — parity.sh should set it per iteration"
    fi
    echo 1>&2 "${GREEN} - OK (runner set GIX_TEST_FIXTURE_HASH=$GIX_TEST_FIXTURE_HASH)"
  }
)
```

- [ ] **Step 2: Run to verify it fails**

```bash
cd /projects/brit && bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -10
```
Expected: FAIL — env var not set by the runner yet.

- [ ] **Step 3: Wrap `source` in a hash loop**

In `tests/parity.sh`, find:
```bash
# shellcheck disable=1090
source "$target"
```

Replace with:
```bash
# Run the target file under each hash kind. Per-row skipping (for rows
# marked `# hash=sha1-only`) is enforced inside the target via the
# only_for_hash helper.
for hash_kind in sha1 sha256; do
  echo "${WHITE}====================================================="
  echo "${GREEN}HASH = $hash_kind"
  echo "${WHITE}====================================================="
  export GIX_TEST_FIXTURE_HASH="$hash_kind"
  # shellcheck disable=1090
  source "$target"
done
unset GIX_TEST_FIXTURE_HASH
```

- [ ] **Step 4: Run to verify it passes**

```bash
cd /projects/brit && bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -40
```
Expected: smoke test output appears TWICE, once under `HASH = sha1` header, once under `HASH = sha256`. All `it` blocks pass under both.

- [ ] **Step 5: Commit**

```bash
cd /projects/brit && git add tests/parity.sh tests/journey/parity/_smoke.sh
git commit -m "test: parity.sh runs each file under both sha1 and sha256"
```

---

## Task 5: Update `gix-steward.md` completion-gate for hash coverage

**Files:**
- Modify: `.claude/agents/gix-steward.md` (the "Completion-promise gate" section)

**Context:** Steward currently checks that every flag has an `it` block. Add a check that every section has a `# hash=` annotation and that coverage is `dual` or explicitly `sha1-only <reason>`.

- [ ] **Step 1: Read the current completion-gate section**

```bash
cd /projects/brit && grep -n "### 1. Completion-promise gate" .claude/agents/gix-steward.md
```
Expected: line number ~48.

- [ ] **Step 2: Append hash-coverage check to the completion-gate evidence list**

In `.claude/agents/gix-steward.md`, find the existing bullet list under "Required evidence" in section 1. The last bullet is "Cleanliness gate" (from the earlier commit). Append a new bullet immediately after:

```markdown
- **Hash coverage.** Every `title` section in `tests/journey/parity/<cmd>.sh` must be preceded by a `# hash=<coverage>` comment on its own line, where `<coverage>` is one of:
  - `dual` — section runs under both sha1 and sha256 via `tests/parity.sh`'s hash loop
  - `sha1-only <reason>` — section skips under sha256; `<reason>` must be a concrete justification (e.g., "gix push cannot open sha256 remotes yet, see gix/src/clone/fetch/mod.rs unimplemented!()") — not "TODO" or "later"
  No annotation, empty `sha1-only` reason, or coverage token other than those two = REJECT.
  Independently, run `bash tests/parity.sh tests/journey/parity/<cmd>.sh` — every section's `it` blocks must pass under sha1; sections with `# hash=dual` must also pass under sha256.
```

- [ ] **Step 3: Update the REJECT template's MISSING list to include the hash-coverage failure mode**

Find the block:
```
MISSING:
  - flag=--<flag-name>  ·  source=vendor/git/Documentation/git-<cmd>.txt L<N>  ·  no matching it-block in tests/journey/parity/<cmd>.sh
  - flag=--<flag-name>  ·  it-block exists but does not invoke git — only gix
  - flag=--<flag-name>  ·  expect_parity mode=effect but flag is scriptable (e.g., --porcelain) and should be mode=bytes
```

Add these lines to the list (preserve the existing three):
```
  - flag=--<flag-name>  ·  no `# hash=` annotation above its title
  - flag=--<flag-name>  ·  `# hash=sha1-only` without a concrete reason string
  - flag=--<flag-name>  ·  `# hash=dual` but fails under sha256
```

- [ ] **Step 4: Verify the edit**

```bash
cd /projects/brit && grep -c "hash=" .claude/agents/gix-steward.md
```
Expected: output at least 4 (the new additions).

- [ ] **Step 5: Commit**

```bash
cd /projects/brit && git add .claude/agents/gix-steward.md
git commit -m "steward: require hash-coverage annotation in completion gate"
```

---

## Task 6: Update `etc/parity/prompt.md` iteration contract

**Files:**
- Modify: `etc/parity/prompt.md`

**Context:** Architect needs to know: every row written or scaffolded must carry a `# hash=` annotation. Default `dual` for most flags. `sha1-only <reason>` is legitimate only when the feature genuinely can't exercise sha256 (e.g., gix can't yet open sha256 remotes for push — push parity may be entirely sha1-only until that's fixed).

- [ ] **Step 1: Append hash annotation to the scaffold step in Iteration 1**

In `etc/parity/prompt.md`, find Iteration 1 step 3 ("Create `tests/journey/parity/$CMD.sh`..."). Replace the bullet list under it with:
```markdown
3. Create `tests/journey/parity/$CMD.sh` following the style of `tests/journey/gix.sh`:
   - `title "gix $CMD"` at top
   - One section per flag: `title "gix $CMD --<flag>"` + a `TODO` `it "..."` block placeholder inside a sandbox, wrapped with `only_for_hash` guard
   - **Every section preceded by TWO comment lines:** `# mode=<effect|bytes>` AND `# hash=<dual|sha1-only "<reason>">`
   - Default hash coverage is `dual`. Use `sha1-only` **only** when the flag cannot meaningfully exercise sha256 — e.g., if gix push cannot yet open sha256 remotes at all, all push rows are `sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"` until that's fixed
   - Do **not** yet write real assertions — placeholders only
```

- [ ] **Step 2: Update the "Concrete shape for a single row" example**

In Iteration 2..N step 4, find the concrete shape block. Replace with:
```bash
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
```

- [ ] **Step 3: Add a rule to the "Rules" section**

In the "Rules (gix-brit fire-at-will discipline)" bullet list, add immediately before the `Never modify vendor/git/` bullet:
```markdown
- **Every `title` section has two comment lines above it**: `# mode=<effect|bytes>` and `# hash=<dual|sha1-only "<reason>">`. No unannotated sections. `sha1-only` requires a concrete reason, not a placeholder.
- **Wrap each section's body with `only_for_hash <coverage> && ( ... )`** so rows skip correctly when `tests/parity.sh` runs under the non-matching hash. The guard is cheap and the skip message is informative.
```

- [ ] **Step 4: Verify edits**

```bash
cd /projects/brit && grep -c "hash=" etc/parity/prompt.md
```
Expected: at least 6.

- [ ] **Step 5: Commit**

```bash
cd /projects/brit && git add etc/parity/prompt.md
git commit -m "parity: require hash-coverage annotation in per-iteration contract"
```

---

## Task 7: Full end-to-end verification

**Files:**
- Read-only verification; no new modifications expected.

**Context:** After Tasks 1-6 land, the full smoke test should pass under the dual-hash runner without any rework needed. Any surprises indicate an integration gap.

- [ ] **Step 1: Run the full smoke test**

```bash
cd /projects/brit && bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tee /tmp/dual-hash-smoke.log
echo "EXIT: $?"
```
Expected: `EXIT: 0`. Output contains both `HASH = sha1` and `HASH = sha256` sections. All `it` blocks in both sections report OK. The `sha1-only` skip assertion prints the YELLOW "skipped" line under sha256.

- [ ] **Step 2: Verify the git log**

```bash
cd /projects/brit && git log --oneline -7
```
Expected: 6 commits from this plan (one per task's commit step), each with a clear scope.

- [ ] **Step 3: Push**

```bash
cd /projects/brit && git push origin gix-brit
```
Expected: fast-forward push succeeds (or non-FF if pilot pushed concurrently — in that case fetch/rebase first, then push).

---

## Self-review summary

- **Spec coverage:** all 5 spec deliverables mapped to tasks. Item 1 (matrix column) deferred per conflict risk — noted under "Conflict-risk acknowledgment" with alternative (comment-based per-row annotation).
- **Placeholder scan:** no TBDs, no "fill in details," no "handle edge cases." Every step shows actual code and actual commands.
- **Type consistency:** `only_for_hash` signature is the same wherever it's referenced (Task 1 definition, Task 6 usage in prompt template). `expect_parity` OK line format is referenced consistently (Task 2 defines, Task 4 and smoke tests consume). `# hash=` / `# mode=` comment conventions are stable across all files (Task 5 steward check, Task 6 prompt rule).
- **Conflict avoidance:** no files modified that the ralph pilot is touching. `docs/parity/commands.md` schema change deferred.

---

## Execution options

Plan complete and saved to `docs/superpowers/plans/2026-04-22-dual-hash-harness-retrofit.md`.

**Timing note:** Pilot ralph session is modifying gix-brit in parallel. Executing this plan NOW lands commits on gix-brit between the pilot's commits — which is OK for the files in this plan (none overlap with pilot's touched set) but means the git log will be interleaved. That's acceptable for gix-brit's "fire-at-will" discipline.

**Two execution options:**

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration. Good for this plan because each task is small and self-contained.

2. **Inline Execution** — batch tasks in the current session with checkpoints for review. Slower but gives you mid-plan visibility.

**Which approach?**
