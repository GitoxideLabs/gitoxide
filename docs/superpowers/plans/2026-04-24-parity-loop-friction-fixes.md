# Parity Loop Friction Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land four small, composable parity-loop primitives so the next ralph-wiggum run (git log) doesn't re-invent them: a stateful-fixture reset helper, a compat-only-flag helper, a file-default hash-reason convention, and an auto-generated shortcomings ledger.

**Architecture:** All four changes are additive. Helpers go into `tests/helpers.sh` next to `expect_parity`. A small bash generator at `etc/parity/shortcomings.sh` produces `docs/parity/SHORTCOMINGS.md` from the existing `shortcoming "..."` calls in `tests/journey/parity/*.sh`. The ralph prompt at `etc/parity/prompt.md` documents all four so the `git log` loop picks them up on iteration 0. Existing journey files (fetch/clone/push/status) are **not** retrofitted here — that's forward compatibility work, out of scope.

**Tech Stack:** bash (helpers + generator), markdown (ledger + prompt). No Rust changes. No new binaries.

---

## File Structure

**Modify:**
- `tests/helpers.sh` — add `expect_parity_reset` and `compat_effect` helpers alongside the existing `expect_parity`.
- `tests/journey/parity/_smoke.sh` — add dogfood coverage for both new helpers.
- `etc/parity/prompt.md` — document the four conventions under the per-iteration contract.
- `docs/parity/commands.md` — replace the scattered "Notes" deferrals with a cross-link to `docs/parity/SHORTCOMINGS.md`.

**Create:**
- `etc/parity/shortcomings.sh` — bash generator that scans `tests/journey/parity/*.sh` and writes `docs/parity/SHORTCOMINGS.md`.
- `docs/parity/SHORTCOMINGS.md` — generator output, committed.

**Do not touch:**
- `tests/journey/parity/{fetch,clone,push,status}.sh` — closed, steward-verified, retrofit is out of scope.
- `tests/parity.sh` — runner wraps the helpers; no changes needed.
- Any `gix-*` crate — this sprint is harness-only.

---

## Task 1: `expect_parity_reset` — per-binary fixture reset for stateful ops

**Problem (friction #1):** `expect_parity` runs git and gix back-to-back against a single `$PWD`. For ops like `git fetch --unshallow` that mutate the fixture (repo becomes complete after git's run), gix then sees post-git state and diverges. `fetch.sh --unshallow` captured this as a `shortcoming` rather than a closable row.

**Shape:** `expect_parity_reset <setup-fn> <mode> [--] <args...>`. The helper creates sibling tmp workdirs `parity-git/` and `parity-gix/`, runs the named `<setup-fn>` inside each (with `cd` into the workdir so the setup materializes its fixture in the right place), then runs `git <args>` and `$exe_plumbing <args>` respectively in their own workdirs. Output capture, exit-code comparison, and byte-mode diffing match `expect_parity` verbatim.

**Files:**
- Modify: `tests/helpers.sh` — append `expect_parity_reset` after the existing `expect_parity` (ends at `:175`).
- Modify: `tests/journey/parity/_smoke.sh` — add coverage block.

- [ ] **Step 1: Write the failing smoke test**

Append the following block to `tests/journey/parity/_smoke.sh` inside the outer `(sandbox ... )` group, after the existing `"fixture helpers respect GIX_TEST_FIXTURE_HASH"` block (around line 92) and before `"parity.sh runs each file under both hash kinds"` (around line 94):

```bash
  (with "expect_parity_reset runs setup per-binary"
    # Prove: the named setup-fn fires once before git runs and once before
    # gix (stand-in = git again) runs. Without reset, stateful fixture ops
    # like `git fetch --unshallow` can't be asserted back-to-back.
    SEEN_FILE="$(mktemp)"
    function _setup_mark_and_init() {
      echo x >> "$SEEN_FILE"
      git init -q
      git config commit.gpgsign false
      git -c user.email=x@x -c user.name=x commit -q --allow-empty -m init
    }
    saved_exe_plumbing="$exe_plumbing"
    exe_plumbing="$(command -v git)"
    it "setup fires once per binary invocation" && {
      expect_parity_reset _setup_mark_and_init effect -- status
      count="$(wc -l < "$SEEN_FILE" | tr -d ' ')"
      if [[ "$count" != "2" ]]; then
        fail "expected setup to run 2x (once per binary), got $count"
      fi
      echo 1>&2 "${GREEN} - OK (setup ran per-binary, count=$count)"
    }
    it "byte-level divergence in reset mode is reported and FAILs" && {
      # Craft a setup that produces different content for each invocation
      # by reading $SEEN_FILE's current line count, so git and gix see
      # different statuses and bytes-mode should FAIL. We toggle set -e
      # off briefly so the intended FAIL doesn't abort the suite.
      > "$SEEN_FILE"
      function _setup_divergent() {
        echo x >> "$SEEN_FILE"
        git init -q
        git config commit.gpgsign false
        local n
        n="$(wc -l < "$SEEN_FILE" | tr -d ' ')"
        touch "file-$n"
      }
      set +e
      expect_parity_reset _setup_divergent bytes -- status --porcelain 2>/dev/null
      rc=$?
      set -e
      if [[ "$rc" == "0" ]]; then
        fail "expected bytes-mode FAIL (setup is divergent), got OK"
      fi
      echo 1>&2 "${GREEN} - OK (bytes-mode divergence surfaces, rc=$rc)"
    }
    rm -f "$SEEN_FILE"
    exe_plumbing="$saved_exe_plumbing"
  )
```

- [ ] **Step 2: Run the smoke test to verify it fails (helper not yet defined)**

Run: `bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -30`
Expected: FAIL with `expect_parity_reset: command not found` (or bash's equivalent) — the helper doesn't exist yet.

- [ ] **Step 3: Add `expect_parity_reset` to `tests/helpers.sh`**

Append to `tests/helpers.sh` after the existing `expect_parity` function (file currently ends at line 175 with the closing `}` of `expect_parity`):

```bash

# expect_parity_reset — like expect_parity, but invokes <setup-fn> in a
# fresh per-binary workdir before each binary runs. Use for stateful ops
# whose side-effects on the fixture would poison the second binary's run
# (classic case: `fetch --unshallow`, which mutates .git/shallow).
#
# Usage: expect_parity_reset <setup-fn> <effect|bytes> [--] <shared-args...>
#
# The setup-fn is invoked in each per-binary workdir with cwd already
# inside it. It should materialize whatever fixture the assertion needs
# (e.g. `git init`, seed commits, clone from a sibling bare upstream).
# The same $exe_plumbing binary contract as expect_parity applies.
function expect_parity_reset() {
  local setup="${1:?expect_parity_reset: need <setup-fn> name}"
  local mode="${2:?expect_parity_reset: need mode (effect|bytes)}"
  shift 2
  [[ "${1:-}" == "--" ]] && shift

  if ! declare -F "$setup" >/dev/null; then
    echo 1>&2 "${RED}expect_parity_reset: setup-fn '$setup' is not a defined function"
    return 2
  fi

  local root git_wd gix_wd
  root="$(mktemp -d -t parity-reset.XXXXXX)"
  git_wd="$root/git"
  gix_wd="$root/gix"
  mkdir -p "$git_wd" "$gix_wd"

  local git_out git_exit gix_out gix_exit
  set +e
  ( cd "$git_wd" && "$setup" >/dev/null 2>&1 )
  git_out="$(cd "$git_wd" && git "$@" 2>&1)"; git_exit=$?

  ( cd "$gix_wd" && "$setup" >/dev/null 2>&1 )
  gix_out="$(cd "$gix_wd" && "$exe_plumbing" "$@" 2>&1)"; gix_exit=$?
  set -e

  rm -rf "$root"

  export PARITY_GIT_OUT="$git_out" PARITY_GIT_EXIT="$git_exit"
  export PARITY_GIX_OUT="$gix_out" PARITY_GIX_EXIT="$gix_exit"

  if [[ "$mode" != "effect" && "$mode" != "bytes" ]]; then
    echo 1>&2 "${RED}expect_parity_reset: unknown mode '$mode' (want effect|bytes)"
    return 2
  fi

  if [[ "$git_exit" != "$gix_exit" ]]; then
    echo 1>&2 "${RED} - FAIL (exit-code divergence: git=$git_exit gix=$gix_exit)"
    echo 1>&2 "${WHITE}\$ (reset=$setup) $*"
    echo 1>&2 "--- git ---"; echo 1>&2 "$git_out"
    echo 1>&2 "--- gix ---"; echo 1>&2 "$gix_out"
    return 1
  fi

  if [[ "$mode" == "bytes" && "$git_out" != "$gix_out" ]]; then
    echo 1>&2 "${RED} - FAIL (byte-level output divergence, exit=$git_exit)"
    echo 1>&2 "${WHITE}\$ (reset=$setup) $*"
    diff <(echo "$git_out") <(echo "$gix_out") 1>&2 || true
    return 1
  fi

  local active_hash="${GIX_TEST_FIXTURE_HASH:-sha1}"
  echo 1>&2 "${GREEN} - OK ($mode parity via reset=$setup, hash=$active_hash, exit=$git_exit)"
  return 0
}
```

- [ ] **Step 4: Run the smoke test to verify it passes**

Run: `bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -30`
Expected: two new `- OK` lines mentioning `reset=_setup_mark_and_init` and `bytes-mode divergence surfaces`. No `FAIL` lines.

- [ ] **Step 5: Commit**

```bash
git add tests/helpers.sh tests/journey/parity/_smoke.sh
git commit -m "$(cat <<'EOF'
parity: expect_parity_reset for stateful-fixture ops

Runs the named setup-fn in its own per-binary workdir before each of
git and $exe_plumbing runs. Closes the `fetch --unshallow`-class gap
where the first binary mutates the fixture (.git/shallow) and the
second sees post-first-run state.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: `compat_effect` — standard marker for compat-only flags

**Problem (friction #2):** Many flags land as "Clap accepts it, semantics deferred" — exit-code parity holds, byte-output parity is a known follow-up. Today each loop writes a free-form sentence in the `# mode=effect` header comment describing the deferred wiring, with no grep-able pattern. The `status.sh` file has 6+ of these written differently.

**Shape:** `compat_effect "<reason>" [--] <args...>` — a thin wrapper around `expect_parity effect -- <args>` that additionally emits a grep-able `[compat] <reason>` line to stderr when the row is green. Generator (Task 3) picks these up as first-class ledger entries.

**Files:**
- Modify: `tests/helpers.sh` — add `compat_effect` after `expect_parity_reset`.
- Modify: `tests/journey/parity/_smoke.sh` — add coverage.

- [ ] **Step 1: Write the failing smoke test**

Append to `tests/journey/parity/_smoke.sh` inside the outer `(sandbox ... )` group, immediately after the `expect_parity_reset` block from Task 1:

```bash
  (with "compat_effect emits grep-able marker on green rows"
    # Pin $exe_plumbing to git so effect-mode parity trivially passes;
    # we're asserting the marker shape, not semantics.
    saved_exe_plumbing="$exe_plumbing"
    exe_plumbing="$(command -v git)"
    it "OK row carries [compat] <reason> on stderr" && {
      out="$(compat_effect "diff emission deferred under -v" -- --version 2>&1)"
      if [[ "$out" != *"[compat] diff emission deferred under -v"* ]]; then
        fail "compat_effect missing [compat] marker; got: $out"
      fi
      if [[ "$out" != *"- OK"* ]]; then
        fail "compat_effect did not surface the underlying OK line; got: $out"
      fi
      echo 1>&2 "${GREEN} - OK (marker + OK both present)"
    }
    it "propagates FAIL when underlying parity diverges" && {
      # Force a divergence by pointing $exe_plumbing at /bin/true, which
      # returns exit 0 with no output vs git --version's versioned text.
      # effect mode only cares about exit codes, so this still passes;
      # use -c nosuch.key=... -c to force git to fail differently.
      set +e
      exe_plumbing="/bin/false"
      compat_effect "forced divergence" -- --version >/dev/null 2>&1
      rc=$?
      exe_plumbing="$(command -v git)"
      set -e
      if [[ "$rc" == "0" ]]; then
        fail "expected compat_effect to FAIL when exit codes diverge, got rc=0"
      fi
      echo 1>&2 "${GREEN} - OK (FAIL propagates, rc=$rc)"
    }
    exe_plumbing="$saved_exe_plumbing"
  )
```

- [ ] **Step 2: Run the smoke test to verify it fails**

Run: `bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -20`
Expected: FAIL with `compat_effect: command not found` (or bash equivalent).

- [ ] **Step 3: Add `compat_effect` to `tests/helpers.sh`**

Append to `tests/helpers.sh` directly after the `expect_parity_reset` function added in Task 1:

```bash

# compat_effect — canonical marker for "Clap wires the flag, byte-output
# semantics deferred." Runs `expect_parity effect` under the hood and
# additionally emits a grep-able `[compat] <reason>` line on stderr
# when the row is green, so the shortcomings ledger generator
# (etc/parity/shortcomings.sh) can surface it.
#
# Usage: compat_effect "<reason>" [--] <shared-args...>
#
# The reason must be a single-line human-readable phrase describing the
# deferred semantic gap (e.g. "diff emission deferred under -v"). It is
# NOT a snippet of documentation — keep it to one sentence.
function compat_effect() {
  local reason="${1:?compat_effect: need <reason> string}"
  shift
  [[ "${1:-}" == "--" ]] && shift

  expect_parity effect -- "$@" || return $?
  echo 1>&2 "${YELLOW}   [compat] $reason"
  return 0
}
```

- [ ] **Step 4: Run the smoke test to verify it passes**

Run: `bash tests/parity.sh tests/journey/parity/_smoke.sh 2>&1 | tail -20`
Expected: two new `- OK` lines for `marker + OK both present` and `FAIL propagates`. No `FAIL` lines.

- [ ] **Step 5: Commit**

```bash
git add tests/helpers.sh tests/journey/parity/_smoke.sh
git commit -m "$(cat <<'EOF'
parity: compat_effect marker for clap-wired-semantics-deferred rows

Wraps expect_parity effect with a grep-able `[compat] <reason>` line
so the shortcomings ledger generator can surface compat-only rows
without free-form prose in each file's header.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Shortcomings ledger generator + seed output

**Problem (friction #4):** `shortcoming "..."` calls in journey files, "known follow-ups" prose in file headers, and deferral notes in `docs/parity/commands.md` are three parallel, hand-maintained lists. A reader assembles them by grepping. The generator makes `shortcoming` lines the canonical source and produces `docs/parity/SHORTCOMINGS.md`; the matrix cross-links instead of duplicating.

**Shape:** `etc/parity/shortcomings.sh` walks `tests/journey/parity/*.sh`, extracts `shortcoming "<reason>"` and `compat_effect "<reason>"` calls with their nearest preceding `title "..."`, and writes a grouped markdown table to `docs/parity/SHORTCOMINGS.md`. Runs in O(N lines) with awk. Idempotent — rerun produces the same bytes.

**Files:**
- Create: `etc/parity/shortcomings.sh`
- Create: `docs/parity/SHORTCOMINGS.md` (generator output)
- Modify: `docs/parity/commands.md` — replace per-cmd deferral prose with cross-link.

- [ ] **Step 1: Create the generator script**

Write `etc/parity/shortcomings.sh`:

```bash
#!/usr/bin/env bash
# Walk tests/journey/parity/*.sh and produce docs/parity/SHORTCOMINGS.md
# as the canonical ledger of:
#   - `shortcoming "<reason>"` calls (closed-as-deferred rows)
#   - `compat_effect "<reason>"` calls (clap-wired, semantics-deferred rows)
#
# Output is stable: re-running on unchanged input produces byte-identical
# bytes so CI can gate on `git diff --exit-code docs/parity/SHORTCOMINGS.md`.
#
# Usage:
#   bash etc/parity/shortcomings.sh              # writes docs/parity/SHORTCOMINGS.md
#   bash etc/parity/shortcomings.sh --check      # diffs against committed file, exits 1 if stale
set -eu

repo_root="$(cd "$(dirname "$0")/../.." && pwd)"
out="$repo_root/docs/parity/SHORTCOMINGS.md"
check_mode=0
[[ "${1:-}" == "--check" ]] && check_mode=1

tmp="$(mktemp)"
trap 'rm -f "$tmp"' EXIT

{
  echo "# Parity Shortcomings Ledger"
  echo
  echo "Auto-generated from \`tests/journey/parity/*.sh\` by \`etc/parity/shortcomings.sh\`."
  echo "**Do not edit by hand** — re-run the generator."
  echo
  echo "Two row classes:"
  echo "- **deferred** — \`shortcoming \"<reason>\"\`: row closed as a legitimate deferral; reason describes the gap."
  echo "- **compat** — \`compat_effect \"<reason>\"\`: row green at effect mode (exit-code parity); byte-output parity is a known follow-up."
  echo
}  > "$tmp"

for f in "$repo_root"/tests/journey/parity/*.sh; do
  [[ "$(basename "$f")" == "_smoke.sh" ]] && continue
  cmd="$(basename "$f" .sh)"

  # awk: walk the file, remember the most recent `title "..."`, and when
  # we hit a `shortcoming` or `compat_effect` call, emit one CSV-ish row
  # with class|title|line|reason. Title strings are single-quoted-safe
  # because the matcher is anchored to double-quote delimiters.
  rows="$(awk -v cmd="$cmd" '
    /^[[:space:]]*title[[:space:]]+"/ {
      match($0, /title[[:space:]]+"([^"]*)"/, a)
      if (a[1] != "") title = a[1]
      next
    }
    /^[[:space:]]*shortcoming[[:space:]]+"/ {
      match($0, /shortcoming[[:space:]]+"([^"]*)"/, a)
      if (a[1] != "") printf("deferred|%s|%d|%s\n", title, NR, a[1])
      next
    }
    /^[[:space:]]*compat_effect[[:space:]]+"/ {
      match($0, /compat_effect[[:space:]]+"([^"]*)"/, a)
      if (a[1] != "") printf("compat|%s|%d|%s\n", title, NR, a[1])
      next
    }
  ' "$f")"

  [[ -z "$rows" ]] && continue

  {
    echo "## ${cmd}"
    echo
    echo "| Class | Section | Reason | Source |"
    echo "|---|---|---|---|"
  } >> "$tmp"

  # Sort by line number for stable output.
  echo "$rows" | sort -t'|' -k3,3n | while IFS='|' read -r class title line reason; do
    printf '| %s | `%s` | %s | [%s:%s](../../tests/journey/parity/%s.sh#L%s) |\n' \
      "$class" "$title" "$reason" "$(basename "$f")" "$line" "$cmd" "$line" >> "$tmp"
  done
  echo >> "$tmp"
done

if [[ $check_mode -eq 1 ]]; then
  if ! diff -u "$out" "$tmp" >&2; then
    echo "shortcomings.sh --check: $out is stale — re-run without --check to regenerate" >&2
    exit 1
  fi
  echo "shortcomings.sh --check: $out is up to date" >&2
  exit 0
fi

mv "$tmp" "$out"
trap - EXIT
echo "wrote $out"
```

- [ ] **Step 2: Make it executable and run it**

```bash
chmod +x etc/parity/shortcomings.sh
bash etc/parity/shortcomings.sh
```

Expected: `wrote /projects/brit/docs/parity/SHORTCOMINGS.md`. No errors.

- [ ] **Step 3: Inspect the output**

Read `docs/parity/SHORTCOMINGS.md` and spot-check:
- Top heading + "Auto-generated" note present.
- `## fetch` section contains rows for `--shallow-exclude`, `--unshallow` (happy path), `--negotiate-only`, `--multiple`.
- `## clone` section contains rows for `-b/--branch=<name>`, `--revision`, `--reference`, `--reference-if-able`, `--dissociate`, `--depth=0`, `--shallow-since`, `--shallow-exclude`.
- No `## _smoke` or `## status` section (status has no `shortcoming` or `compat_effect` calls yet — it uses inline prose; that's retrofit-era work).
- Rows are sorted by line number within each section.

If any of these fail, fix the generator and re-run — don't commit broken output.

- [ ] **Step 4: Verify idempotence (--check mode)**

Run: `bash etc/parity/shortcomings.sh --check`
Expected: `shortcomings.sh --check: .../SHORTCOMINGS.md is up to date`, exit 0.

- [ ] **Step 5: Replace the matrix's scattered deferral prose with a cross-link**

Edit `docs/parity/commands.md`. In the Porcelain table (around lines 13-45), trim the "Notes" column for the four present-verified rows (push, fetch, clone, status) so that all deferred-flag prose is replaced by a short pointer. Keep the `it`-block counts and the verdict mode summary — those are ledger-orthogonal.

Concretely, change each present-row's Notes cell from e.g.
`happy-path + error-path parity across the full documented flag surface (89 green it blocks across 61 sections); sha256 remotes, --shallow-exclude protocol-error alignment, --unshallow happy-path..., --negotiate-only happy-path..., and --multiple positional re-dispatch are documented in the fetch.sh header as deferred follow-ups`
to
`89 green `it` blocks across 61 sections. Deferrals: see [SHORTCOMINGS.md#fetch](SHORTCOMINGS.md#fetch). sha256 blocker: gix cannot open sha256 remotes yet (every row `sha1-only`).`

Apply the same shape to the push, clone, status rows. Don't touch the non-present rows (pull/merge/rebase/…) — those have no ledger entries.

Also add a sentence at the top of the matrix file, under the existing "Legend" paragraph, pointing at the ledger:

```markdown
Deferred flag-level rows and compat-only rows live in [SHORTCOMINGS.md](SHORTCOMINGS.md), regenerated from `tests/journey/parity/*.sh` by `etc/parity/shortcomings.sh`.
```

- [ ] **Step 6: Commit**

```bash
git add etc/parity/shortcomings.sh docs/parity/SHORTCOMINGS.md docs/parity/commands.md
git commit -m "$(cat <<'EOF'
parity: generate SHORTCOMINGS.md from journey-file markers

Walks tests/journey/parity/*.sh for shortcoming() and compat_effect()
calls and produces docs/parity/SHORTCOMINGS.md as the canonical
deferral ledger. commands.md matrix cross-links instead of duplicating
per-cmd deferral prose. --check mode gates regeneration staleness.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Update the ralph prompt with the new conventions

**Problem:** The three new primitives (`expect_parity_reset`, `compat_effect`, shortcomings generator) + one new documentation convention (file-default hash reason) exist but are invisible to the git-log loop unless the prompt tells it to reach for them.

**Shape:** Amend `etc/parity/prompt.md` under the per-iteration contract. Four small paragraphs, one per primitive. Explicit "when to use" / "when NOT to use" for each.

**Files:**
- Modify: `etc/parity/prompt.md`

- [ ] **Step 1: Add a "Harness primitives" section after "Per-iteration contract"**

Edit `etc/parity/prompt.md`. Insert a new section *after* the existing `## Per-iteration contract` block (before `## Rules (gix-brit fire-at-will discipline)`, around line 84). The new section:

```markdown
## Harness primitives (reach for these; do not re-invent)

Four helpers + one convention live in `tests/helpers.sh` and `etc/parity/`. Use them — don't roll a new per-command workaround.

### `expect_parity` — default for idempotent flags

Runs git and gix back-to-back in the current `$PWD`. Use for flags whose execution doesn't mutate the fixture (read-only parse, status output, error-path assertions, `--help`).

### `expect_parity_reset <setup-fn>` — stateful-fixture ops

Runs `<setup-fn>` in its own per-binary workdir before each of the two binaries. Use when the flag mutates the fixture and back-to-back runs would let the second binary see post-first-run state. Canonical case: `fetch --unshallow` (the C path checks `.git/shallow`; once git's run promotes the repo to complete, gix sees no-shallow and diverges).

```bash
function _shallow-clone-from-bare-upstream() {
  git init -q --bare ../upstream.git >/dev/null 2>&1 || :
  git clone -q --depth=1 "$(cd .. && pwd)/upstream.git" . 2>/dev/null
}
expect_parity_reset _shallow-clone-from-bare-upstream effect -- fetch --unshallow origin
```

### `compat_effect "<reason>" --` — clap-wired, semantics-deferred

Thin wrapper over `expect_parity effect` that additionally emits `[compat] <reason>` on stderr. Use when the flag is accepted by gix's Clap layer and exit-code parity holds, but bytes-mode divergence is a known follow-up you're intentionally deferring this iteration. The `<reason>` is surfaced by `etc/parity/shortcomings.sh` as a compat-class ledger row — make it a single grep-able sentence, not prose.

```bash
it "matches git behavior with -v" && {
  compat_effect "diff emission under -v deferred" -- status -v
}
```

Do **not** use `compat_effect` for rows where bytes parity is actually achievable this iteration — close them with `expect_parity bytes` instead. Do **not** use it for rows where exit codes diverge — those are true failures, not compat deferrals.

### `# parity-defaults:` file-level header — de-duplicate sha1-only reasons

Every parity file today starts with a single file-level reason for why its rows are `sha1-only` (e.g. `"gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"` for fetch/push/clone, `"gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"` for status). Stating that reason once in a top-of-file block is enough; per-row `# hash=sha1-only` comments can then omit the string.

```bash
# parity-defaults:
#   hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
#   mode=effect
```

Per-row `# hash=` / `# mode=` lines only need to appear when they **override** the file default. Rows that diverge (e.g. a `bytes`-mode row in an otherwise `effect` file) still annotate explicitly. A fully-dual file (rare today, expected for `log` once `gix log` grows sha256) sets `hash=dual` in the defaults.

### `shortcoming "<reason>"` + shortcomings generator — canonical ledger

When you legitimately defer a row (operator-approved or steward-verdict `DEFER-LEGITIMATE`), close it with `shortcoming "<reason>"` as the body of the `only_for_hash` block. Then regenerate the ledger:

```bash
bash etc/parity/shortcomings.sh
```

Commit the updated `docs/parity/SHORTCOMINGS.md` in the same commit as the deferred row. The matrix at `docs/parity/commands.md` cross-links to the ledger — do not duplicate deferral prose there.

Completion gate (`@gix-steward` verification) runs `bash etc/parity/shortcomings.sh --check`. A stale ledger is a steward-reject.
```

- [ ] **Step 2: Amend the "Per-iteration contract" iteration-1 instructions**

In the existing section `### Iteration 1 — scaffold`, step 3 currently says:

> Every section preceded by TWO comment lines: `# mode=<effect|bytes>` AND `# hash=<dual|sha1-only "<reason>">`

Change to:

> First, add a file-level `# parity-defaults:` block stating the default `mode` and `hash` for the file (see "Harness primitives" above). Then, per-section `# mode=` / `# hash=` comments are only required when the section overrides the default. `sha1-only` reasons are stated once in the defaults block, not per-row.

- [ ] **Step 3: Amend the "Completion" block**

In the existing `## Completion` section, add a new step 0 before the existing "Invoke @gix-steward":

```markdown
0. Regenerate the ledger: `bash etc/parity/shortcomings.sh`. If diff is non-empty, commit as `parity: regenerate SHORTCOMINGS.md for git $CMD`.
```

And in the steward-prompt, change:

> Return PASS or REJECT with evidence.

to:

> Run `bash etc/parity/shortcomings.sh --check` as part of verification. Return PASS or REJECT with evidence.

- [ ] **Step 4: Verify the prompt is self-consistent**

Read `etc/parity/prompt.md` end-to-end. Check:
- Every reference to `expect_parity`, `expect_parity_reset`, `compat_effect`, `shortcoming`, or `etc/parity/shortcomings.sh` resolves to a definition in the prompt OR `tests/helpers.sh`.
- No step assumes an old convention that the new primitives replace (e.g. nothing instructs the loop to write `# hash=sha1-only "..."` per row).
- The iteration-1 scaffold flow and the iteration-2+ close-one-row flow both reference the primitives where relevant.

- [ ] **Step 5: Commit**

```bash
git add etc/parity/prompt.md
git commit -m "$(cat <<'EOF'
parity: document harness primitives in the ralph prompt

Adds a "Harness primitives" section covering expect_parity_reset,
compat_effect, the file-level `# parity-defaults:` header, and the
shortcomings generator. Amends iteration-1 scaffold and completion
gate to reference them so the git-log loop picks them up on
iteration 0 without re-inventing.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

## Post-plan verification

Before declaring the sprint done:

- [ ] `bash tests/parity.sh tests/journey/parity/_smoke.sh` — fully green, including new `expect_parity_reset` and `compat_effect` blocks.
- [ ] `bash etc/parity/shortcomings.sh --check` — clean (no diff).
- [ ] `git log --oneline -5` — four commits, each small and purposeful, matching the conventional-commits style (`parity: <verb> <target>`).
- [ ] `etc/parity/prompt.md` reads end-to-end without referencing anything undefined.
- [ ] No changes to `tests/journey/parity/{fetch,clone,push,status}.sh` — retrofit is explicitly out of scope.
- [ ] No changes to `gix-*` crates — this sprint is harness-only.
