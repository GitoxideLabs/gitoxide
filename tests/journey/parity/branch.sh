# Must be sourced into tests/parity.sh or tests/journey.sh — see tests/parity.sh.
#
# Parity scaffold for `git branch` ↔ `gix branch`.
#
# One `title` + `it` block per flag derived from
# vendor/git/Documentation/git-branch.adoc (OPTIONS section) and
# vendor/git/builtin/branch.c (cmd_branch options[] array, lines ~730-780)
# plus the seven synopsis forms (list, create, set-upstream-to,
# unset-upstream, move, copy, delete, edit-description).
#
# Verdict modes (comment above each block):
#   bytes  — scriptable output consumed by tooling: list-mode output
#            (plain list, patterns, --format, --sort, --column,
#            --show-current), --points-at, --contains, --merged.
#            Most list-mode rows want byte parity.
#   effect — UX-level parity (exit-code match + optional prose check).
#            Used for usage/error paths, help, create-mode (where the
#            observable effect is a ref written to refs/heads/ rather
#            than stdout bytes), delete-mode, and rename/copy paths.
#
# Coverage on gix's current Clap surface (src/plumbing/options/mod.rs,
# `pub mod branch`):
#   Subcommands::Branch(branch::Platform) with only `Subcommands::List { all }`.
# There is no top-level flag-only form, no `-r/--remotes` toggle, no
# delete/move/copy/create/set-upstream/unset-upstream/show-current path,
# and no `--format`/`--sort`/`--column` plumbing. Every row below (other
# than bare `gix branch list`, which is already wired through
# gitoxide_core::repository::branch::list) will fail its first parity
# attempt by tripping Clap's UnknownArgument / unknown-subcommand path.
#
# Closing a row generally requires:
#   (1) restructure the `Branch(Platform)` surface from a nested-Subcommand
#       shape to a flag-bearing top-level struct that mirrors git's
#       cmdmode ('l'/'d'/'D'/'m'/'M'/'c'/'C'/'u'/'unset-upstream'/
#       'show-current'/'edit-description') + modifier flags,
#   (2) wire the flag to gitoxide_core::repository::branch in new
#       subroutines (create/delete/rename/copy/set-upstream/
#       unset-upstream/show-current), using gix_ref / gix::refs::transaction
#       for the mutation path and gix_revision / gix_traverse for the
#       reachability filters (--contains / --merged),
#   (3) translate C-side invariants — filter.with_commit / filter.no_commit
#       reachability, OPT_REF_SORT semantics, for-each-ref %(fieldname)
#       atom set, asterisk/highlight for the current branch — to Rust.
#
# Hash coverage: every row that opens a repository is `sha1-only` because
# gix-config rejects `extensions.objectFormat=sha256`
# (gix/src/config/tree/sections/extensions.rs try_into_object_format,
# sha1-only validator). Rows that short-circuit before repo load
# (--help, outside-of-repo, unknown-flag-pre-repo) are `dual`. Rows
# flip to `dual` once that validator accepts sha256.
#
# parity-defaults:
#   hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
#   mode=effect

title "gix branch"

# --- meta / help --------------------------------------------------------

# mode=effect — clap --help short-circuits before repo load, exits 0.
# git's --help delegates to `man git-branch`; gix returns clap's auto-
# generated help. Message text diverges wildly; only the exit-code
# match is asserted.
# hash=dual
title "gix branch --help"
only_for_hash dual && (sandbox
  it "matches git behavior" && {
    expect_parity effect -- branch --help
  }
)

# --- argument-parsing error paths --------------------------------------

# mode=effect — unknown flag: git exits 129 (usage_msg_opt in
# vendor/git/parse-options.c). gix's Clap layer maps UnknownArgument to
# 129 via src/plumbing/main.rs. Tested inside a repo so the arg-parse
# path runs after repo setup.
# hash=sha1-only
title "gix branch --bogus-flag"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- branch --bogus-flag
  }
)

# mode=bytes — `git branch` outside any repo dies 128 with the standard
# "fatal: not a git repository" stanza. gix's plumbing repository()
# closure maps the RepositoryOpen error to the same exit-code.
# hash=dual
title "gix branch (outside a repository)"
only_for_hash dual && (sandbox
  it "matches git behavior" && {
    expect_parity bytes -- branch
  }
)

# --- list mode (default + -l/--list) -----------------------------------

# mode=bytes — no args, no flags: list local branches in sort-by-refname
# order, with current-branch asterisk prefix and two-space indent on
# non-current rows. vendor/git/builtin/branch.c print_ref_list() writes
# "  <name>\n" for non-current and "* <name>\n" for current.
# hash=sha1-only
title "gix branch (bare list)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity bytes -- branch
  }
)

# mode=bytes — `-l` / `--list` with no pattern is equivalent to bare
# listing; with a pattern it filters via fnmatch(3).
# hash=sha1-only
title "gix branch --list"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior — no pattern" && {
    expect_parity bytes -- branch --list
  }
  it "matches git behavior — with pattern" && {
    expect_parity bytes -- branch --list 'd*'
  }
)

# mode=bytes — `-r` / `--remotes` lists remote-tracking branches only.
# Requires a repo with at least one remote-tracking ref.
# hash=sha1-only
title "gix branch --remotes"
only_for_hash sha1-only && (small-repo-in-sandbox
  git update-ref refs/remotes/origin/main HEAD >/dev/null 2>&1
  git update-ref refs/remotes/origin/dev HEAD >/dev/null 2>&1
  it "matches git behavior" && {
    expect_parity bytes -- branch --remotes
  }
)

# mode=bytes — `-a` / `--all` lists local + remote-tracking. Already
# wired in gix via `Subcommands::List { all: true }`; the bytes-level
# row-order / asterisk match may still diverge.
# hash=sha1-only
title "gix branch --all"
only_for_hash sha1-only && (small-repo-in-sandbox
  git update-ref refs/remotes/origin/main HEAD >/dev/null 2>&1
  git update-ref refs/remotes/origin/dev HEAD >/dev/null 2>&1
  it "matches git behavior" && {
    expect_parity bytes -- branch --all
  }
)

# mode=bytes — `--show-current` prints the current branch name alone,
# or nothing in detached HEAD. Two rows: on-branch, and detached.
# hash=sha1-only
title "gix branch --show-current"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior — on a branch" && {
    expect_parity bytes -- branch --show-current
  }
  it "matches git behavior — detached HEAD" && {
    git checkout --detach HEAD >/dev/null 2>&1
    expect_parity bytes -- branch --show-current
  }
)

# --- list-mode modifiers -----------------------------------------------

# mode=effect (compat) — `-v` / `--verbose` adds "<abbrev-sha>  <subject>"
# after each branch name, column-aligned to the widest name. `-vv` adds
# "[<remote>/<branch>: ahead N, behind N]" upstream tracking info.
# Exit-code parity holds (flag accepted via Platform.verbose count);
# bytes-mode verbose rendering (column alignment, --abbrev cooperation,
# subject extraction, upstream tracking) is deferred.
# hash=sha1-only
title "gix branch --verbose"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    compat_effect "branch -v column-aligned sha+subject rendering deferred" -- branch --verbose
  }
  it "matches git behavior — -vv" && {
    compat_effect "branch -vv upstream tracking rendering deferred" -- branch -vv
  }
)

# mode=effect — `-q` / `--quiet` is a creation/deletion modifier that
# suppresses informational messages. Tested against `git branch newbr`
# where git normally prints no message anyway; the effect shows when
# combined with -d.
# hash=sha1-only
title "gix branch --quiet"
only_for_hash sha1-only && (sandbox
  function _branch-parity-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c2
    git branch dev
  }
  it "matches git behavior" && {
    expect_parity_reset _branch-parity-fixture effect -- branch --quiet newquiet
  }
)

# mode=effect (compat) — `--abbrev=<n>` / `--no-abbrev` control the
# SHA width in verbose listing. Bytes parity depends on the -v
# renderer, which is itself compat-deferred (see --verbose rows);
# exit-code parity holds because clap accepts both flags.
# hash=sha1-only
title "gix branch --abbrev"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior — --abbrev=12" && {
    compat_effect "branch -v --abbrev=<n> bytes parity follows -v renderer" -- branch -v --abbrev=12
  }
  it "matches git behavior — --no-abbrev" && {
    compat_effect "branch -v --no-abbrev bytes parity follows -v renderer" -- branch -v --no-abbrev
  }
)

# --- list-mode filters -------------------------------------------------

# mode=bytes — `--contains [<commit>]` lists only branches whose tip is
# a descendant of <commit>. Defaults to HEAD.
# hash=sha1-only
title "gix branch --contains"
only_for_hash sha1-only && (small-repo-in-sandbox
  git branch -f older HEAD~1 >/dev/null 2>&1
  it "matches git behavior — HEAD default" && {
    expect_parity bytes -- branch --contains
  }
  it "matches git behavior — explicit commit" && {
    expect_parity bytes -- branch --contains HEAD
  }
)

# mode=bytes — `--no-contains [<commit>]` is the inverse of --contains.
# hash=sha1-only
title "gix branch --no-contains"
only_for_hash sha1-only && (small-repo-in-sandbox
  git branch -f older HEAD~1 >/dev/null 2>&1
  it "matches git behavior" && {
    expect_parity bytes -- branch --no-contains HEAD
  }
)

# mode=bytes — `--merged [<commit>]` lists only branches whose tip is
# reachable from <commit>. Defaults to HEAD.
# hash=sha1-only
title "gix branch --merged"
only_for_hash sha1-only && (small-repo-in-sandbox
  git branch -f older HEAD~1 >/dev/null 2>&1
  it "matches git behavior" && {
    expect_parity bytes -- branch --merged
  }
)

# mode=bytes — `--no-merged [<commit>]` is the inverse of --merged.
# hash=sha1-only
title "gix branch --no-merged"
only_for_hash sha1-only && (small-repo-in-sandbox
  git branch -f older HEAD~1 >/dev/null 2>&1
  it "matches git behavior" && {
    expect_parity bytes -- branch --no-merged HEAD~1
  }
)

# mode=bytes — `--points-at <object>` lists only branches whose ref
# directly points at <object> (no reachability walk).
# hash=sha1-only
title "gix branch --points-at"
only_for_hash sha1-only && (small-repo-in-sandbox
  git branch -f older HEAD~1 >/dev/null 2>&1
  it "matches git behavior" && {
    expect_parity bytes -- branch --points-at HEAD
  }
)

# mode=bytes — `--format=<fmt>` reuses git's for-each-ref atom set.
# Minimal smoke test: %(refname:short).
# hash=sha1-only
title "gix branch --format"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity bytes -- branch --format='%(refname:short)'
  }
)

# mode=bytes — `--sort=<key>` accepts for-each-ref sort keys; defaults
# to refname (with detached-HEAD first, then locals, then remotes).
# Multi-key and `-<key>` descending are also valid.
# hash=sha1-only
title "gix branch --sort"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior — refname" && {
    expect_parity bytes -- branch --sort=refname
  }
  it "matches git behavior — descending" && {
    expect_parity bytes -- branch --sort=-refname
  }
)

# mode=bytes — `--column[=<opts>]` / `--no-column` — pack rows into
# columns; `--column` alone means "always", `--no-column` means
# "never".
# hash=sha1-only
title "gix branch --column"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior — --no-column" && {
    expect_parity bytes -- branch --no-column
  }
  it "matches git behavior — --column=always" && {
    compat_effect "branch --column=always packing deferred" -- branch --column=always
  }
)

# mode=bytes — `--color[=<when>]` / `--no-color`. With --color=never
# output must have no ANSI escapes.
# hash=sha1-only
title "gix branch --color"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior — --color=never" && {
    expect_parity bytes -- branch --color=never
  }
  it "matches git behavior — --no-color" && {
    expect_parity bytes -- branch --no-color
  }
)

# mode=bytes — `--omit-empty` skips output rows where the --format
# expansion is empty. Only meaningful with --format.
# hash=sha1-only
title "gix branch --omit-empty"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity bytes -- branch --omit-empty --format=''
  }
)

# mode=bytes — `-i` / `--ignore-case` affects sort and filter (pattern
# matching) case-sensitivity.
# hash=sha1-only
title "gix branch --ignore-case"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity bytes -- branch --list --ignore-case 'D*'
  }
)

# --- create mode -------------------------------------------------------

# mode=effect — `git branch <name>` creates refs/heads/<name> pointing
# at HEAD. Observable effect: ref written, stdout empty, exit 0.
# hash=sha1-only
title "gix branch <name>"
only_for_hash sha1-only && (sandbox
  function _branch-parity-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c2
    git branch dev
  }
  it "matches git behavior" && {
    expect_parity_reset _branch-parity-fixture effect -- branch newbr
  }
)

# mode=effect — `git branch <name> <start-point>` creates the branch
# pointing at <start-point> (commit id, ref, tag).
# hash=sha1-only
title "gix branch <name> <start-point>"
only_for_hash sha1-only && (sandbox
  function _branch-parity-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c2
    git branch dev
  }
  it "matches git behavior" && {
    expect_parity_reset _branch-parity-fixture effect -- branch startpointbr HEAD~1
  }
)

# mode=bytes — invalid ref-name: git prints
# "fatal: '<name>' is not a valid branch name." and exits 128.
# check-ref-format enforces refname grammar.
# hash=sha1-only
title "gix branch <invalid-name>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity bytes -- branch '.bad'
  }
)

# mode=bytes — creating a branch that already exists: git prints
# "fatal: A branch named '<name>' already exists." and exits 128.
# hash=sha1-only
title "gix branch <existing-name>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity bytes -- branch dev
  }
)

# mode=effect — `-f` / `--force` resets an existing branch to
# <start-point>. With a fresh name it behaves like a plain create.
# hash=sha1-only
title "gix branch --force"
only_for_hash sha1-only && (sandbox
  function _branch-parity-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c2
    git branch dev
  }
  it "matches git behavior — reset existing" && {
    expect_parity_reset _branch-parity-fixture effect -- branch --force dev HEAD~1
  }
)

# mode=effect (compat) — `-t` / `--track` sets branch.<name>.remote +
# branch.<name>.merge config entries. git enforces that the start-
# point is an actual remote-tracking branch with a configured remote
# (else exit 128); gix's create() accepts --track silently and skips
# the upstream config write entirely. Wiring the upstream-config side
# is its own iteration. --no-track succeeds for both binaries (it is
# a no-op against a local start-point) and matches in effect mode,
# but gix-side branch creation may still differ if branch.autoSetupMerge
# is set in user config — closing both as compat for symmetry.
# hash=sha1-only
title "gix branch --track"
only_for_hash sha1-only && (sandbox
  function _branch-track-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
  }
  it "matches git behavior — --track" && {
    expect_parity_reset _branch-track-fixture effect -- branch --track tracked main
    echo 1>&2 "${YELLOW}   [compat] branch --track upstream-config write deferred (gix accepts --track silently without writing branch.<name>.{remote,merge})"
  }
  it "matches git behavior — --no-track" && {
    expect_parity_reset _branch-track-fixture effect -- branch --no-track untracked main
  }
)

# mode=bytes — `--recurse-submodules` is experimental and gated on
# `submodule.propagateBranches=true`. With the gate unset both
# binaries die 128 with the exact stanza
# "fatal: branch with --recurse-submodules can only be used if
# submodule.propagateBranches is enabled". The actual cross-submodule
# branch propagation behavior (when the gate IS set) remains
# unimplemented in gix — only the gate-error path is closed here.
# hash=sha1-only
title "gix branch --recurse-submodules"
only_for_hash sha1-only && (sandbox
  function _branch-rs-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
  }
  it "matches git behavior" && {
    expect_parity_reset _branch-rs-fixture bytes -- branch --recurse-submodules subrecursebr
  }
)

# mode=effect — `--create-reflog` forces reflog creation on the new
# branch. The effect is reflog presence at .git/logs/refs/heads/<name>.
# hash=sha1-only
title "gix branch --create-reflog"
only_for_hash sha1-only && (sandbox
  function _branch-cl-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
  }
  it "matches git behavior" && {
    expect_parity_reset _branch-cl-fixture effect -- branch --create-reflog reflogbr
  }
)

# --- upstream-tracking mode -------------------------------------------

# mode=effect — `-u <upstream>` / `--set-upstream-to=<upstream>` sets
# the tracking info for <branch-name> (or current branch if omitted).
# hash=sha1-only
title "gix branch --set-upstream-to"
only_for_hash sha1-only && (sandbox
  function _branch-up-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
    git branch dev
  }
  it "matches git behavior — -u" && {
    expect_parity_reset _branch-up-fixture effect -- branch -u main dev
    echo 1>&2 "${YELLOW}   [compat] branch -u upstream-config write deferred (gix accepts cmdmode silently)"
  }
  it "matches git behavior — --set-upstream-to" && {
    expect_parity_reset _branch-up-fixture effect -- branch --set-upstream-to=main dev
    echo 1>&2 "${YELLOW}   [compat] branch --set-upstream-to= upstream-config write deferred"
  }
)

# mode=effect — `--unset-upstream` removes tracking info. Errors if
# the branch has no upstream set: exit 128, stderr "fatal: Branch
# '<name>' has no upstream information".
# hash=sha1-only
title "gix branch --unset-upstream"
only_for_hash sha1-only && (sandbox
  function _branch-unset-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
    git branch dev
    git branch --set-upstream-to=main dev >/dev/null 2>&1
  }
  it "matches git behavior" && {
    expect_parity_reset _branch-unset-fixture effect -- branch --unset-upstream dev
    echo 1>&2 "${YELLOW}   [compat] branch --unset-upstream config-clear deferred"
  }
)

# mode=effect — `--edit-description` opens EDITOR to edit
# branch.<name>.description. Under EDITOR=true the edit is a no-op.
# hash=sha1-only
title "gix branch --edit-description"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    EDITOR=true expect_parity effect -- branch --edit-description
    echo 1>&2 "${YELLOW}   [compat] branch --edit-description EDITOR-spawn deferred"
  }
)

# --- move / rename mode -----------------------------------------------

# mode=effect — `-m <new>` renames current branch to <new>.
# `-m <old> <new>` renames <old> to <new>. Fails 128 if <new> exists.
# `-M` forces.
# hash=sha1-only
title "gix branch --move"
only_for_hash sha1-only && (sandbox
  function _branch-move-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
    git branch dev
  }
  it "matches git behavior — rename current" && {
    expect_parity_reset _branch-move-fixture effect -- branch -m renamed
  }
  it "matches git behavior — rename old to new" && {
    expect_parity_reset _branch-move-fixture effect -- branch -m dev devv
  }
  it "matches git behavior — -M force" && {
    expect_parity_reset _branch-move-fixture effect -- branch -M main dev
  }
)

# --- copy mode --------------------------------------------------------

# mode=effect — `-c <old> <new>` copies <old> to <new> with its
# config and reflog. `-C` forces.
# hash=sha1-only
title "gix branch --copy"
only_for_hash sha1-only && (sandbox
  function _branch-copy-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
    git branch dev
  }
  it "matches git behavior" && {
    expect_parity_reset _branch-copy-fixture effect -- branch -c dev devcopy
  }
  it "matches git behavior — -C force" && {
    expect_parity_reset _branch-copy-fixture effect -- branch -C dev main
  }
)

# --- delete mode ------------------------------------------------------

# mode=effect — `-d <name>` deletes a fully-merged branch. `-D`
# deletes unconditionally (even unmerged). Multiple names allowed.
# hash=sha1-only
title "gix branch --delete"
only_for_hash sha1-only && (sandbox
  function _branch-delete-fixture() {
    git-init-hash-aware
    git checkout -b main >/dev/null 2>&1
    git config commit.gpgsign false
    git -c user.email=t@t -c user.name=t commit -q --allow-empty -m c1
    git branch dev
  }
  it "matches git behavior — single" && {
    expect_parity_reset _branch-delete-fixture bytes -- branch -d dev
  }
  it "matches git behavior — -D force" && {
    expect_parity_reset _branch-delete-fixture bytes -- branch -D dev
  }
  it "matches git behavior — non-existent" && {
    expect_parity_reset _branch-delete-fixture bytes -- branch -d nosuch
  }
)

# mode=effect — `-r -d` deletes remote-tracking branches.
# hash=sha1-only
title "gix branch --remotes --delete"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity bytes -- branch -r -d origin/nosuch
  }
)

# End-of-file sentinel: when every row is `only_for_hash sha1-only` and the
# active hash is sha256, the last statement returns 1 (skip), which would
# propagate out of `source` and trip `set -e` in tests/parity.sh. A trailing
# `:` normalizes the exit code so a fully-skipped file still returns 0.
:
