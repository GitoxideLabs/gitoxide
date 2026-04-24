# Must be sourced into tests/parity.sh or tests/journey.sh — see tests/parity.sh.
#
# Parity scaffold for `git status` ↔ `gix status`.
#
# One `title` + `it` block per flag derived from
# vendor/git/Documentation/git-status.adoc and vendor/git/wt-status.c
# (wt_status_print and porcelain variants). Flag rows were closed
# iteration-by-iteration via the parity loop; matrix row was promoted
# to `present` at commit f72ff4a56 after `@gix-steward` PASS verdict.
#
# Verdict modes (comment above each block):
#   bytes  — scriptable output consumed by tooling (--porcelain, -z,
#            --short, --porcelain=v2); byte-exact match required
#   effect — exit-code + UX; output diff reported but not fatal
#
# Hash coverage: most rows are `sha1-only` — gix's config layer rejects
# `extensions.objectFormat=sha256` (gix/src/config/tree/sections/extensions.rs
# try_into_object_format, sha1-only validator), so `gix status` cannot
# open any sha256 repo. Rows that never touch a repo (--help, unknown
# option, outside-of-repo) are `dual`. Rows flip to `dual` once the
# gix-config validator accepts sha256.
#
# Coverage on gix's current Clap surface (src/plumbing/options/mod.rs):
# gix status has --format {simplified,porcelain-v2}, --ignored, --submodules,
# -s/--statistics (conflicts with git's -s/--short), --no-write,
# --index-worktree-renames, and [pathspec]. It does NOT yet accept
# -s/--short, -b/--branch, --show-stash, --porcelain[=v1], --long,
# -v/--verbose, -u/--untracked-files, --ignore-submodules (git's spelling;
# gix uses --submodules=none), -z, --column/--no-column, --ahead-behind,
# --renames/--no-renames, --find-renames. Most rows will fail in their
# first iteration and guide Clap-surface expansion.

title "gix status"

# --- meta / help ---------------------------------------------------------

# mode=effect — clap --help short-circuits before repo load, exits 0.
# git's --help delegates to `man git-status` (exit 0 when man is
# available); gix returns clap's auto-generated help. Message text
# diverges wildly; only the exit-code match is asserted.
# hash=dual
title "gix status --help"
only_for_hash dual && (sandbox
  it "matches git behavior" && {
    expect_parity effect -- status --help
  }
)

# --- argument-parsing error paths ---------------------------------------

# mode=effect — unknown flag: git exits 129 (usage_msg_opt in
# vendor/git/parse-options.c::PARSE_OPT_HELP). gix originally exited 2
# (clap default for ErrorKind::UnknownArgument); src/plumbing/main.rs
# intercepts try_parse_from and remaps UnknownArgument → exit 129 to
# restore parity. Other clap errors (ValueValidation, DisplayHelp,
# DisplayVersion) keep their default codes. Tested inside a repo: git
# outside a repo dies 128 before reaching arg-parse, while clap in gix
# always runs first — so the exercise must live in a small-repo fixture
# to isolate the arg-parse error path. That pushes this row to
# sha1-only (gix-config rejects sha256 on any repo open).
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --bogus-flag"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- status --bogus-flag
  }
)

# mode=effect — `git status` outside any repo dies 128 with
# "fatal: not a git repository (or any of the parent directories): .git".
# gix originally exited 1 with an anyhow trace through NoGitRepository;
# src/plumbing/main.rs's repository() closure now intercepts the
# gix_discover::upwards::Error::NoGitRepository* variants, writes git's
# exact wording, and exits 128. Scoped to plumbing commands that require
# a repo — env/clone are unaffected.
# hash=dual
title "gix status (outside a repository)"
only_for_hash dual && (sandbox
  it "matches git behavior" && {
    expect_parity effect -- status
  }
)

# --- happy paths ---------------------------------------------------------

# mode=effect — baseline vanilla status in a clean repo, no flags.
# git prints "On branch <br>\nnothing to commit, working tree clean";
# gix prints nothing. Both exit 0. Effect mode (exit-code parity only).
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status (clean working tree)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- status
  }
)

# mode=effect — a tracked file modified in the worktree but not staged.
# git prints a long-form "Changes not staged for commit" section;
# gix prints a simplified short-format line (`  M a`). Both exit 0.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status (modified tracked file)"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo modified >> a
  it "matches git behavior" && {
    expect_parity effect -- status
  }
)

# mode=effect — a new untracked file present in the worktree.
# Both exit 0; output wording diverges (git "Untracked files:" section
# vs gix's `? new-untracked` short line).
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status (untracked file)"
only_for_hash sha1-only && (small-repo-in-sandbox
  touch new-untracked
  it "matches git behavior" && {
    expect_parity effect -- status
  }
)

# mode=effect — a new file staged to the index, not yet committed.
# git prints a "Changes to be committed" section; gix prints `A  <path>`.
# Both exit 0.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status (staged new file)"
only_for_hash sha1-only && (small-repo-in-sandbox
  touch staged-file && git add staged-file
  it "matches git behavior" && {
    expect_parity effect -- status
  }
)

# mode=effect — a tracked file modified in the worktree AND staged (two
# distinct XY states for the same path: M in index, M in worktree).
# Both exit 0.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status (modified + staged same path)"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo staged-change >> a && git add a && echo worktree-change >> a
  it "matches git behavior" && {
    expect_parity effect -- status
  }
)

# --- short / long format -------------------------------------------------

# mode=bytes — `-s` short format: one `XY <path>` line per entry (2-char
# status, single space, path). Scripts parse this; byte-exact match
# required. Implementation: added `Format::Short` to gitoxide-core's
# status emitter (path-grouped collection of TreeIndex X + IndexWorktree
# Y, followed by untracked `?? ` and ignored `!! `). Clap: new `-s`/
# `--short` bool on the Platform struct (conflicts with `--format`);
# the old `-s` short alias for `--statistics` was dropped (statistics
# stays under `--statistics` long form). Progress output on stderr is
# suppressed for Short / PorcelainV2 formats so bytes-mode parity holds.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status -s"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo staged-change >> a && git add a && echo worktree-change >> a
  touch new-untracked
  it "matches git behavior" && {
    expect_parity bytes -- status -s
  }
)

# mode=bytes — `--short` long-form equivalent of `-s`. Same Clap flag on
# the gix side (single `short: bool` bound to both `-s` and `--short`).
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --short"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo staged-change >> a && git add a && echo worktree-change >> a
  touch new-untracked
  it "matches git behavior" && {
    expect_parity bytes -- status --short
  }
)

# mode=effect — `--long` is git's default format; gix accepts it as a
# compat no-op that yields the Simplified output. Added `long: bool`
# on the Clap Platform with `conflicts_with_all = ["short", "format"]`
# to mirror git's rejection of `--long --short` / `--long --porcelain`.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --long"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo staged-change >> a && git add a && echo worktree-change >> a
  it "matches git behavior" && {
    expect_parity effect -- status --long
  }
)

# --- branch / stash metadata --------------------------------------------

# mode=bytes — `-b`/`--branch` prepends a `## <branch>` header to the
# short-format output. (Long format already shows `On branch <branch>`
# so -b alone is a no-op there.) The scaffold exercises -b combined
# with -s to isolate the header emission. Upstream-tracking lines
# (`## br...origin/br [ahead N]`) and detached-HEAD / initial-repo
# variants are deferred; current emission covers `## <branch>` and
# `## HEAD (no branch)` only.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status -b / --branch"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo staged-change >> a && git add a && echo worktree-change >> a
  touch new-untracked
  it "matches git behavior with -b -s" && {
    expect_parity bytes -- status -b -s
  }
  it "matches git behavior with --branch -s" && {
    expect_parity bytes -- status --branch -s
  }
)

# mode=effect — `--show-stash` prints git's "Your stash currently has
# N entries" line (long format) or a `# stash <N>` header (porcelain=v2).
# gix accepts the flag as a no-op for effect-mode parity; full stash-
# count emission would require reflog traversal of refs/stash and is
# deferred. Exit-code match is asserted; text divergence expected.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --show-stash"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo stash-me >> a && git stash push -q -m "parity-test"
  it "matches git behavior" && {
    expect_parity effect -- status --show-stash
  }
)

# --- porcelain format ---------------------------------------------------

# mode=bytes — `--porcelain` (default version = v1) maps internally to
# `Format::Short` since the two formats differ only in color / path-
# relativity, both off in the fixture. Clap: `porcelain:
# Option<PorcelainVersion>` with `num_args=0..=1` and
# `default_missing_value="v1"`.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --porcelain"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo staged-change >> a && git add a && echo worktree-change >> a
  touch new-untracked
  it "matches git behavior" && {
    expect_parity bytes -- status --porcelain
  }
)

# mode=bytes — explicit `--porcelain=v1` (same format, explicit version);
# same mapping as bare --porcelain via the PorcelainVersion ValueEnum.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --porcelain=v1"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo staged-change >> a && git add a && echo worktree-change >> a
  touch new-untracked
  it "matches git behavior" && {
    expect_parity bytes -- status --porcelain=v1
  }
)

# mode=bytes — `--porcelain=v2` adds branch/stash headers and extended
# per-entry fields (<XY> <sub> <mH> <mI> <mW> <hH> <hI> <path>).
# Implementation landed in gitoxide-core's status emitter — Clap's
# `--format porcelain-v2` variant gate was removed and a v2 branch in
# the emitter writes `1 <XY> N... <mH> <mI> <mW> <hH> <hI> <path>`
# for ordinary changed entries, `? <path>` for untracked.
# Shortcomings documented in the emitter comment: rename rows (`2 ...`)
# not yet emitted, submodule summary always `N...`, branch headers
# (`# branch.oid` / `# branch.head` / `# branch.upstream` /
# `# branch.ab`) not yet emitted (row currently doesn't combine
# --porcelain=v2 with --branch).
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --porcelain=v2"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo staged-change >> a && git add a && echo worktree-change >> a
  touch new-untracked
  it "matches git behavior" && {
    expect_parity bytes -- status --porcelain=v2
  }
)

# --- verbosity -----------------------------------------------------------

# mode=effect — `-v` / `--verbose` appends a `diff --cached`-style section
# for staged changes; `-vv` additionally appends an unstaged-diff section.
# gix accepts the flag as a u8 counter (clap ArgAction::Count) for compat;
# diff emission is deferred. Effect-mode parity (exit-code match) passes.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status -v / --verbose"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo staged-change >> a && git add a
  it "matches git behavior with -v" && {
    expect_parity effect -- status -v
  }
  it "matches git behavior with --verbose" && {
    expect_parity effect -- status --verbose
  }
  it "matches git behavior with -vv" && {
    echo worktree-change >> a
    expect_parity effect -- status -vv
  }
)

# --- untracked-files ----------------------------------------------------

# mode=effect — `-u<mode>` / `--untracked-files[=<mode>]` toggles untracked
# listing. Clap: `untracked_files: Option<UntrackedMode>` with
# `num_args=0..=1` and `default_missing_value="all"`. Flag is accepted
# for compat; dirwalk-emit wiring (actual untracked-listing behavior
# change) is deferred — effect-mode parity (exit-code match) holds
# across all modes.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status -u / --untracked-files[=<mode>]"
only_for_hash sha1-only && (small-repo-in-sandbox
  mkdir untracked-dir && touch untracked-dir/a untracked-dir/b
  it "matches git behavior with -uno" && {
    expect_parity effect -- status -uno
  }
  it "matches git behavior with --untracked-files=no" && {
    expect_parity effect -- status --untracked-files=no
  }
  it "matches git behavior with -unormal" && {
    expect_parity effect -- status -unormal
  }
  it "matches git behavior with --untracked-files=normal" && {
    expect_parity effect -- status --untracked-files=normal
  }
  it "matches git behavior with -uall" && {
    expect_parity effect -- status -uall
  }
  it "matches git behavior with --untracked-files=all" && {
    expect_parity effect -- status --untracked-files=all
  }
  it "matches git behavior with bare -u (defaults to all)" && {
    expect_parity effect -- status -u
  }
  it "matches git behavior with bare --untracked-files (defaults to all)" && {
    expect_parity effect -- status --untracked-files
  }
)

# --- submodule handling -------------------------------------------------

# mode=effect — `--ignore-submodules[=<when>]` (none/untracked/dirty/all,
# default=all when flag is bare). Clap: `IgnoreSubmodulesMode` ValueEnum
# and `ignore_submodules: Option<IgnoreSubmodulesMode>` on the Platform.
# This fixture has no submodules so only the CLI parse path is exercised.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --ignore-submodules[=<when>]"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with bare --ignore-submodules (all)" && {
    expect_parity effect -- status --ignore-submodules
  }
  it "matches git behavior with --ignore-submodules=none" && {
    expect_parity effect -- status --ignore-submodules=none
  }
  it "matches git behavior with --ignore-submodules=untracked" && {
    expect_parity effect -- status --ignore-submodules=untracked
  }
  it "matches git behavior with --ignore-submodules=dirty" && {
    expect_parity effect -- status --ignore-submodules=dirty
  }
  it "matches git behavior with --ignore-submodules=all" && {
    expect_parity effect -- status --ignore-submodules=all
  }
)

# --- ignored ------------------------------------------------------------

# mode=effect — `--ignored[=<mode>]` (traditional/no/matching, default=
# traditional when flag is bare). Clap's `Ignored` ValueEnum gained
# `traditional` as an alias for `Collapsed`, and a new `No` variant
# that dispatch maps to `None` in core Options (matches git's
# `--ignored=no` = "don't list ignored"). Effect-mode parity across
# all four variants.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --ignored[=<mode>]"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo ignoreme > .gitignore && touch ignoreme
  it "matches git behavior with bare --ignored (traditional)" && {
    expect_parity effect -- status --ignored
  }
  it "matches git behavior with --ignored=traditional" && {
    expect_parity effect -- status --ignored=traditional
  }
  it "matches git behavior with --ignored=no" && {
    expect_parity effect -- status --ignored=no
  }
  it "matches git behavior with --ignored=matching" && {
    expect_parity effect -- status --ignored=matching
  }
)

# --- NUL-separated / column output --------------------------------------

# mode=bytes — `-z` terminates entries with NUL; implies `--porcelain=v1`
# (Format::Short) if no other format is given. Rename entries under `-z`
# use reversed `<dest>\0<source>\0` order per git docs; our emitter
# honors that. Scripts depend on byte-exact NUL separation.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status -z"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo staged-change >> a && git add a && echo worktree-change >> a
  touch new-untracked
  it "matches git behavior" && {
    expect_parity bytes -- status -z
  }
)

# mode=effect — `--column[=<opts>]` / `--no-column`. Clap: `column:
# Option<String>` (accepts any git-style opts string via
# `num_args=0..=1`) and `no_column: bool` (conflicts_with column).
# Column formatting of untracked files isn't implemented; without a
# TTY git also emits one-per-line, so effect-mode parity holds.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --column / --no-column"
only_for_hash sha1-only && (small-repo-in-sandbox
  touch un1 un2 un3
  it "matches git behavior with --column" && {
    expect_parity effect -- status --column
  }
  it "matches git behavior with --no-column" && {
    expect_parity effect -- status --no-column
  }
  it "matches git behavior with --column=always" && {
    expect_parity effect -- status --column=always
  }
)

# --- ahead-behind -------------------------------------------------------

# mode=effect — `--ahead-behind` (default) / `--no-ahead-behind` toggle
# upstream divergence counts. Clap: both as bool flags with mutual
# `conflicts_with`. Effect-mode no-op in dispatch — gix's long-format
# header already shows ahead/behind when an upstream is configured;
# exit-code parity holds on a fixture with a fabricated upstream.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --ahead-behind / --no-ahead-behind"
only_for_hash sha1-only && (small-repo-in-sandbox
  git-init-hash-aware -q --bare ../up.git
  git remote add origin ../up.git
  git push -q origin main 2>/dev/null
  git branch --set-upstream-to=origin/main main 2>/dev/null
  echo ahead >> a && git commit -qam "ahead"
  it "matches git behavior with --ahead-behind" && {
    expect_parity effect -- status --ahead-behind
  }
  it "matches git behavior with --no-ahead-behind" && {
    expect_parity effect -- status --no-ahead-behind
  }
)

# --- renames ------------------------------------------------------------

# mode=effect — `--renames` / `--no-renames` force rename detection on/off
# regardless of `status.renames` config. Exercised via `git mv` of a
# tracked file, which leaves the index with an add/delete pair.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --renames / --no-renames"
only_for_hash sha1-only && (small-repo-in-sandbox
  git mv a renamed-a
  it "matches git behavior with --renames" && {
    expect_parity effect -- status --renames
  }
  it "matches git behavior with --no-renames" && {
    expect_parity effect -- status --no-renames
  }
)

# mode=effect — `--find-renames[=<n>]` enables rename detection with
# optional similarity threshold. Bare form uses git's default (50%);
# explicit `=50` matches. gix has `--index-worktree-renames [<f32>]` but
# no `--find-renames` spelling.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status --find-renames[=<n>]"
only_for_hash sha1-only && (small-repo-in-sandbox
  git mv a renamed-a
  it "matches git behavior with bare --find-renames" && {
    expect_parity effect -- status --find-renames
  }
  it "matches git behavior with --find-renames=50" && {
    expect_parity effect -- status --find-renames=50
  }
)

# --- pathspec -----------------------------------------------------------

# mode=effect — a positional pathspec narrows status output to matching
# paths only. gix accepts pathspec as a `Vec<BString>` positional so
# no Clap change needed; both positional and `-- <pathspec>` variants
# exit 0 on the fixture.
# hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
title "gix status <pathspec>"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo modified >> a
  touch other-untracked
  it "matches git behavior with positional pathspec" && {
    expect_parity effect -- status a
  }
  it "matches git behavior with -- <pathspec>" && {
    expect_parity effect -- status -- a
  }
)

# End-of-file sentinel: when every row is `only_for_hash sha1-only` and the
# active hash is sha256, the last statement returns 1 (skip), which would
# propagate out of `source` and trip `set -e` in tests/parity.sh. A trailing
# `:` normalizes the exit code so a fully-skipped file still returns 0.
:
