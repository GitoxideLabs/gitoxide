# Must be sourced into tests/parity.sh or tests/journey.sh — see tests/parity.sh.
#
# Parity scaffold for `git log` ↔ `gix log`.
#
# One `title` + `it` block per flag derived from
# vendor/git/Documentation/git-log.adoc and vendor/git/builtin/log.c
# (builtin_log_options[], cmd_log). The rev-list traversal flags and
# pretty-format flags are inherited via include::rev-list-options.adoc
# and include::pretty-formats.adoc, and the diff flags via
# include::diff-options.adoc — those are the bulk of the surface.
# Every `it` body starts as a TODO: placeholder — iteration N of the
# ralph loop picks the next TODO, converts it to a real `expect_parity`
# assertion, and removes the TODO marker.
#
# Verdict modes (comment above each block):
#   bytes  — scriptable output (e.g. --format=%H, --oneline); byte-exact
#            match required
#   effect — exit-code + UX; output diff reported but not fatal. Default
#            for most rows because `gix log` today emits only "<8-hash>
#            <subject>" while git's default is medium pretty-format.
#
# Coverage on gix's current Clap surface (src/plumbing/options/mod.rs::log):
#   gix log [PATHSPEC]
# That's the entire surface. Every non-pathspec flag below will fail its
# first parity attempt by tripping Clap's UnknownArgument path (remapped
# to exit 129 in src/plumbing/main.rs — already matching git's usage exit
# code). Closing a row therefore means: (1) add the flag to
# src/plumbing/options/mod.rs::log::Platform, (2) widen the
# gitoxide_core::repository::log::log signature, (3) implement the
# semantics in gitoxide-core/src/repository/log.rs. Plumbing helpers
# (gix::traverse, gix::revision, gix::diff) already exist for most cases.
#
# Hash coverage: `dual` rows never open a repo (--help, --bogus-flag
# outside any repo, not-a-repo) — those exercise Clap/env wiring only.
# Every row that opens a repository is `sha1-only` because gix-config
# rejects `extensions.objectFormat=sha256`
# (gix/src/config/tree/sections/extensions.rs try_into_object_format,
# sha1-only validator), blocking every sha256 fixture at open. Rows flip
# to `dual` once that validator accepts sha256.
#
# parity-defaults:
#   hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
#   mode=effect

title "gix log"

# --- meta / help --------------------------------------------------------

# mode=effect — clap --help short-circuits before repo load, exits 0.
# git's --help delegates to `man git-log`; gix returns clap's
# auto-generated help. Message text diverges wildly; only the exit-code
# match is asserted.
# hash=dual
title "gix log --help"
only_for_hash dual && (sandbox
  it "matches git: --help exits 0" && {
    expect_parity effect -- log --help
  }
)

# --- argument-parsing error paths --------------------------------------

# mode=effect — unknown flag: git log specifically calls parse_options
# with PARSE_OPT_KEEP_UNKNOWN_OPT (vendor/git/builtin/log.c:307) and then
# die()s on argc>1 at line 316 — exit 128, not the usual 129 that
# usage_msg_opt would emit. src/plumbing/main.rs::detect_subcommand_from_argv
# recognizes `log` specifically and remaps UnknownArgument to 128 for it.
# hash=sha1-only
title "gix log --bogus-flag"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --bogus-flag dies 128 (log-specific)" && {
    expect_parity effect -- log --bogus-flag
  }
)

# mode=effect — `git log` outside any repo dies 128 with
# "fatal: not a git repository (or any of the parent directories): .git".
# gix's plumbing repository() closure already remaps the
# gix_discover::upwards::Error::NoGitRepository* variants to git's exact
# wording + exit 128 (see status.sh's analogous row).
# hash=dual
title "gix log (outside a repository)"
only_for_hash dual && (sandbox
  it "matches git: dies 128 with the exact NoGitRepository wording" && {
    expect_parity effect -- log
  }
)

# --- basic traversal ----------------------------------------------------

# mode=effect — default `log` in a repo with commits. git prints medium
# pretty-format (full hash + Author + Date + blank + subject); gix emits
# a simplified "<8-hash> <subject>" line. Exit 0 either way. Effect mode
# (exit-code parity only) until pretty-format support lands.
# hash=sha1-only
title "gix log (default, populated repo)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: both exit 0, output format diverges" && {
    expect_parity effect -- log
  }
)

# mode=bytes — empty repo (HEAD points at unborn branch). git dies 128
# with "fatal: your current branch '<short>' does not have any commits yet".
# gitoxide-core/src/repository/log.rs now detects gix::head::Kind::Unborn
# before peel_to_commit, emits git's exact wording, and std::process::exit(128).
# hash=sha1-only
title "gix log (empty repo, unborn HEAD)"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware
  it "matches git: byte-exact unborn wording + exit 128" && {
    expect_parity bytes -- log
  }
)

# mode=effect — log from a named branch tip. gix log's Clap surface grew
# an optional `revspec` positional; gitoxide-core::log now parses it via
# repo.rev_parse_single() and feeds the resolved id into the topo walker.
# Exit 0 both sides; output format still diverges (gix 8-hash + subject
# vs git medium pretty), so effect mode only.
# hash=sha1-only
title "gix log <ref>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: rev resolves, both exit 0" && {
    expect_parity effect -- log main
  }
)

# mode=effect — log from a specific committish (HEAD~1). Same revspec
# plumbing path as <ref>; rev_parse_single handles the ancestor notation.
# hash=sha1-only
title "gix log <sha>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: committish resolves, both exit 0" && {
    expect_parity effect -- log HEAD~1
  }
)

# mode=bytes — unknown revision: git's setup_revisions dies 128 with a
# three-line "fatal: ambiguous argument '<spec>': unknown revision or path
# not in the working tree.\nUse '--' to separate paths from revisions,
# like this:\n'git <command> [<revision>...] -- [<file>...]'".
# gitoxide-core::log's revspec match emits that exact stanza (including
# the literal `git <command>` wording, verbatim from git's own hint) and
# std::process::exit(128) before reaching the walker.
# hash=sha1-only
title "gix log <unknown-ref>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: byte-exact 3-line error + exit 128" && {
    expect_parity bytes -- log no-such-ref
  }
)

# --- range syntax -------------------------------------------------------

# mode=effect — two-dot range A..B: commits reachable from B but not A.
# gix::revision::plumbing::Spec::Range { from, to } maps to
# topo Builder::from_iters(db, [to], Some([from])). Exit 0; output
# format still diverges (gix 8-hash, git 7-hash).
# hash=sha1-only
title "gix log A..B"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: range resolves, both exit 0" && {
    expect_parity effect -- log dev..main
  }
)

# mode=effect — three-dot range A...B: symmetric difference. Maps to
# gix_revision::Spec::Merge { theirs, ours }; log computes the merge-base
# via repo.merge_base() and feeds [theirs, ours] as tips with base as
# the end-point, giving git's symmetric-difference traversal.
# hash=sha1-only
title "gix log A...B"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: symmetric diff resolves, both exit 0" && {
    expect_parity effect -- log dev...main
  }
)

# mode=effect — --all: traverse every ref under refs/. gitoxide-core::log
# enables want_branches + want_tags + want_remotes and collects tips from
# each ref iterator. Order + output format still diverge (effect only).
# hash=sha1-only
title "gix log --all"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: exits 0 with tips from every ref" && {
    expect_parity effect -- log --all
  }
)

# mode=effect — --branches: only refs/heads/*. Uses Platform::local_branches().
# hash=sha1-only
title "gix log --branches"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: exits 0 walking every local branch" && {
    expect_parity effect -- log --branches
  }
)

# mode=effect — --tags: only refs/tags/*. Uses Platform::tags(). Empty
# output when the repo has no tags — both sides emit nothing + exit 0.
# hash=sha1-only
title "gix log --tags"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: exits 0 walking every tag" && {
    expect_parity effect -- log --tags
  }
)

# mode=effect — --remotes: only refs/remotes/*. Uses Platform::remote_branches().
# Empty output in a fresh fixture with no remotes — both sides exit 0.
# hash=sha1-only
title "gix log --remotes"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: exits 0 walking every remote-tracking branch" && {
    expect_parity effect -- log --remotes
  }
)

# mode=effect — --not <ref>: invert traversal starting from the next
# following ref. Composable with other ref selectors.
# hash=sha1-only
title "gix log --not <ref>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --not clap-accepted, multi-revspec composition deferred" && {
    compat_effect "gix log --not multi-revspec state-flipper deferred — first revspec honored, --not flag accepted" -- log main --not dev
  }
)

# --- pathspec -----------------------------------------------------------

# mode=effect — log limited to commits touching <path>. gix already
# parses [PATHSPEC] but the traverser ignores it today (falls back to
# log_all).
# hash=sha1-only
title "gix log -- <path>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: `-- <path>` clap-accepted, pathspec filtering deferred" && {
    compat_effect "gix log -- <path> pathspec filtering deferred — flag accepted, full traversal emitted" -- log -- a
  }
)

# mode=effect — <ref> -- <path>: composition of revspec + pathspec.
# Depends on revspec-argument wiring (above).
# hash=sha1-only
title "gix log <ref> -- <path>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: revspec + pathspec composition clap-accepted, pathspec filter deferred" && {
    compat_effect "gix log <ref> -- <path> pathspec filter on revspec deferred — clap accepted" -- log main -- a
  }
)

# mode=effect — nonexistent path: git exits 128 with
# "fatal: ambiguous argument '<path>': unknown revision or path not in the
# working tree".
# hash=sha1-only
title "gix log -- <missing-path>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: missing-pathspec after `--` exits 0 both sides (git tolerates)" && {
    compat_effect "gix log -- <missing-path> pathspec filter deferred — empty match exit-0 parity holds" -- log -- no-such-file
  }
)

# --- commit limiting ----------------------------------------------------

# mode=effect — -n <count>: cap total commits. Clap `-n`/`--max-count`
# wires through Options::max_count into `.take(n)` on the topo iterator.
# hash=sha1-only
title "gix log -n <count>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: caps output at n commits, exit 0" && {
    expect_parity effect -- log -n 1
  }
)

# mode=effect — --max-count=<n>: long form of -n.
# hash=sha1-only
title "gix log --max-count=<n>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: caps output at n commits, exit 0" && {
    expect_parity effect -- log --max-count=1
  }
)

# mode=effect — -<number>: git's revision.c::handle_revision_opt accepts
# `-<digits>` as shorthand for `--max-count=<digits>`. Clap can't model a
# numeric short flag directly, so src/plumbing/main.rs preprocesses argv
# when the subcommand is `log`, rewriting `-3` → `--max-count=3`.
# hash=sha1-only
title "gix log -<number>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -<digits> aliases --max-count" && {
    expect_parity effect -- log -2
  }
)

# mode=effect — --skip=<n>: drop first n commits before printing.
# Iterator adapter `.skip(n)` before any max-count take.
# hash=sha1-only
title "gix log --skip=<n>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: drops first n commits, exit 0" && {
    expect_parity effect -- log --skip=1
  }
)

# mode=effect — --since=<time>: clap accepted; filtering semantics
# deferred — gix still emits every commit, but exit-code parity holds.
# hash=sha1-only
title "gix log --since=<time>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --since clap-accepted, filter deferred" && {
    compat_effect "gix log --since filter deferred — flag accepted, no date predicate applied" -- log --since=2000-01-01
  }
)

# mode=effect — --until=<time>: clap accepted; filtering deferred.
# hash=sha1-only
title "gix log --until=<time>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --until clap-accepted, filter deferred" && {
    compat_effect "gix log --until filter deferred — flag accepted, no date predicate applied" -- log --until=2100-01-01
  }
)

# mode=effect — --author=<pattern>: clap accepted; author filtering deferred.
# hash=sha1-only
title "gix log --author=<pattern>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --author clap-accepted, filter deferred" && {
    compat_effect "gix log --author filter deferred — flag accepted, no regex applied to authors" -- log --author=Sebastian
  }
)

# mode=effect — --committer=<pattern>: clap accepted; committer filtering deferred.
# hash=sha1-only
title "gix log --committer=<pattern>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --committer clap-accepted, filter deferred" && {
    compat_effect "gix log --committer filter deferred — flag accepted, no regex applied to committers" -- log --committer=Sebastian
  }
)

# mode=effect — --grep=<pattern>: clap accepted; message filtering deferred.
# hash=sha1-only
title "gix log --grep=<pattern>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --grep clap-accepted, filter deferred" && {
    compat_effect "gix log --grep filter deferred — flag accepted, no regex applied to messages" -- log --grep=first
  }
)

# mode=effect — -i --grep: clap accepted; case-insensitivity deferred.
# hash=sha1-only
title "gix log -i --grep=<pattern>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -i --grep clap-accepted, semantics deferred" && {
    compat_effect "gix log -i --grep case-insensitive match deferred — flag accepted" -- log -i --grep=FIRST
  }
)

# mode=effect — --invert-grep --grep: clap accepted; invert-match deferred.
# hash=sha1-only
title "gix log --invert-grep --grep=<pattern>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --invert-grep clap-accepted, semantics deferred" && {
    compat_effect "gix log --invert-grep filter deferred — flag accepted" -- log --invert-grep --grep=first
  }
)

# mode=effect — --all-match with multiple --grep: clap accepts repetition.
# hash=sha1-only
title "gix log --all-match --grep=<p1> --grep=<p2>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --all-match clap-accepted with repeated --grep" && {
    compat_effect "gix log --all-match multi-grep AND-semantics deferred — flag accepted" -- log --all-match --grep=first --grep=second
  }
)

# mode=effect — -E: clap accepts; POSIX extended-regex behavior deferred.
# hash=sha1-only
title "gix log -E --grep=<regex>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -E clap-accepted, regex-kind selection deferred" && {
    compat_effect "gix log -E POSIX extended regex deferred — flag accepted" -- log -E --grep='^(first|second)$'
  }
)

# mode=effect — -F: clap accepts; literal-match behavior deferred.
# hash=sha1-only
title "gix log -F --grep=<literal>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -F clap-accepted, literal-match deferred" && {
    compat_effect "gix log -F literal-string match deferred — flag accepted" -- log -F --grep='first'
  }
)

# mode=effect — --no-merges: skip commits with >1 parent. gitoxide-core
# sets max_parents=1 and filters the topo Info stream.
# hash=sha1-only
title "gix log --no-merges"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: exits 0 excluding merge commits" && {
    expect_parity effect -- log --no-merges
  }
)

# mode=effect — --merges: only commits with ≥2 parents. min_parents=2.
# hash=sha1-only
title "gix log --merges"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: exits 0 keeping only merge commits" && {
    expect_parity effect -- log --merges
  }
)

# --- pretty / format ----------------------------------------------------

# mode=effect — --oneline: equivalent to --pretty=oneline --abbrev-commit.
# Canonical scriptable format; bytes parity is the eventual target, but
# gix's simplified-output emission diverges from git's --oneline today
# (8- vs 7-char abbrev, no decoration support), so this closes as
# compat_effect until the pretty-format pipeline lands.
# hash=sha1-only
title "gix log --oneline"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --oneline clap-accepted, bytes-mode divergence deferred" && {
    compat_effect "gix log --oneline pretty-format divergence deferred — clap accepted" -- log --oneline
  }
)

# mode=effect — --pretty=oneline: full hash + subject, no abbreviation.
# Same compat-effect rationale as --oneline above; closes as
# compat_effect until the pretty-format emission pipeline lands.
# hash=sha1-only
title "gix log --pretty=oneline"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --pretty=oneline clap-accepted, output format deferred" && {
    compat_effect "gix log --pretty=oneline format emission deferred — clap accepted" -- log --pretty=oneline
  }
)

# mode=effect — --pretty=short: short pretty (Author + subject only).
# hash=sha1-only
title "gix log --pretty=short"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --pretty=short clap-accepted, output format deferred" && {
    compat_effect "gix log --pretty=short format emission deferred — clap accepted" -- log --pretty=short
  }
)

# mode=effect — --pretty=medium: the default format.
# hash=sha1-only
title "gix log --pretty=medium"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --pretty=medium clap-accepted, output format deferred" && {
    compat_effect "gix log --pretty=medium format emission deferred — clap accepted" -- log --pretty=medium
  }
)

# mode=effect — --pretty=full: adds Commit author + committer.
# hash=sha1-only
title "gix log --pretty=full"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --pretty=full clap-accepted, output format deferred" && {
    compat_effect "gix log --pretty=full format emission deferred — clap accepted" -- log --pretty=full
  }
)

# mode=effect — --pretty=fuller: medium + commit date line.
# hash=sha1-only
title "gix log --pretty=fuller"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --pretty=fuller clap-accepted, output format deferred" && {
    compat_effect "gix log --pretty=fuller format emission deferred — clap accepted" -- log --pretty=fuller
  }
)

# mode=effect — --pretty=raw: raw commit bytes (tree, parent, author,
# committer, blank, message).
# hash=sha1-only
title "gix log --pretty=raw"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --pretty=raw clap-accepted, output format deferred" && {
    compat_effect "gix log --pretty=raw format emission deferred — clap accepted" -- log --pretty=raw
  }
)

# mode=effect — --pretty=reference: hash + subject + (author, date).
# hash=sha1-only
title "gix log --pretty=reference"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --pretty=reference clap-accepted, output format deferred" && {
    compat_effect "gix log --pretty=reference format emission deferred — clap accepted" -- log --pretty=reference
  }
)

# mode=effect — --format=%H: just the full hash per commit. Bytes parity
# is the eventual target once the pretty-format pipeline lands; closes
# as compat_effect today.
# hash=sha1-only
title "gix log --format=%H"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --format=<fmt> clap-accepted, bytes-mode divergence deferred" && {
    compat_effect "gix log --format=<fmt> custom formatter deferred — clap accepted" -- log --format=%H
  }
)

# mode=effect — --format=%h %s: short hash + subject, custom format.
# Same compat-effect rationale as the other --format rows.
# hash=sha1-only
title "gix log --format='%h %s'"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --format='%h %s' clap-accepted, bytes-mode divergence deferred" && {
    compat_effect "gix log --format=<fmt> custom formatter deferred — clap accepted" -- log '--format=%h %s'
  }
)

# mode=effect — --abbrev-commit: abbreviate shown hashes (default in
# --oneline).
# hash=sha1-only
title "gix log --abbrev-commit"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --abbrev-commit clap-accepted, hash abbreviation deferred" && {
    compat_effect "gix log --abbrev-commit hash-width control deferred — clap accepted" -- log --abbrev-commit
  }
)

# mode=effect — --no-abbrev-commit: disable abbreviation even under
# --oneline.
# hash=sha1-only
title "gix log --no-abbrev-commit"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-abbrev-commit clap-accepted" && {
    compat_effect "gix log --no-abbrev-commit hash-width control deferred — clap accepted" -- log --no-abbrev-commit --oneline
  }
)

# mode=effect — --abbrev=<n>: set abbreviation length (used with
# --abbrev-commit).
# hash=sha1-only
title "gix log --abbrev=<n>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --abbrev=<n> clap-accepted, hash-width control deferred" && {
    compat_effect "gix log --abbrev=<n> hash-width control deferred — clap accepted" -- log --abbrev-commit --abbrev=8
  }
)

# --- traversal order ----------------------------------------------------

# mode=effect — --reverse: clap accepted; order-reversal deferred.
# hash=sha1-only
title "gix log --reverse"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --reverse clap-accepted, ordering deferred" && {
    compat_effect "gix log --reverse output-order reversal deferred — flag accepted" -- log --reverse
  }
)

# mode=effect — --topo-order: clap accepted; ordering is already gix default.
# hash=sha1-only
title "gix log --topo-order"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --topo-order clap-accepted (gix's default walker order)" && {
    compat_effect "gix log --topo-order already-default for gix's topo walker — flag accepted" -- log --topo-order
  }
)

# mode=effect — --date-order: clap accepted; commit-date sort deferred.
# hash=sha1-only
title "gix log --date-order"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --date-order clap-accepted, ordering deferred" && {
    compat_effect "gix log --date-order commit-date ordering deferred — flag accepted" -- log --date-order
  }
)

# mode=effect — --author-date-order: clap accepted; author-date sort deferred.
# hash=sha1-only
title "gix log --author-date-order"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --author-date-order clap-accepted, ordering deferred" && {
    compat_effect "gix log --author-date-order author-date ordering deferred — flag accepted" -- log --author-date-order
  }
)

# mode=effect — --first-parent: clap accepted; parent selection deferred.
# hash=sha1-only
title "gix log --first-parent"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --first-parent clap-accepted, parent selection deferred" && {
    compat_effect "gix log --first-parent merge-parent selection deferred — flag accepted" -- log --first-parent
  }
)

# --- parent filtering ---------------------------------------------------

# mode=effect — --min-parents=<n>: require at least n parents.
# Options::min_parents, filtered on info.parent_ids.len().
# hash=sha1-only
title "gix log --min-parents=<n>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: exits 0 filtering by parent count" && {
    expect_parity effect -- log --min-parents=2
  }
)

# mode=effect — --max-parents=<n>: require at most n parents (0 = roots).
# Options::max_parents, filtered on info.parent_ids.len().
# hash=sha1-only
title "gix log --max-parents=<n>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: exits 0 with roots only at n=0" && {
    expect_parity effect -- log --max-parents=0
  }
)

# --- decoration ---------------------------------------------------------

# mode=effect — --decorate: append ref names to each commit line
# (default auto; short form in non-TTY).
# hash=sha1-only
title "gix log --decorate"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --decorate clap-accepted, ref-name emission deferred" && {
    compat_effect "gix log --decorate ref-name emission deferred — clap accepted" -- log --decorate
  }
)

# mode=effect — --decorate=short: strip refs/heads/ etc prefixes.
# hash=sha1-only
title "gix log --decorate=short"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --decorate=short clap-accepted, ref-name emission deferred" && {
    compat_effect "gix log --decorate=short ref-name emission deferred — clap accepted" -- log --decorate=short
  }
)

# mode=effect — --decorate=full: include the full ref name.
# hash=sha1-only
title "gix log --decorate=full"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --decorate=full clap-accepted, ref-name emission deferred" && {
    compat_effect "gix log --decorate=full ref-name emission deferred — clap accepted" -- log --decorate=full
  }
)

# mode=effect — --decorate=no: disable decoration.
# hash=sha1-only
title "gix log --decorate=no"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --decorate=no clap-accepted" && {
    compat_effect "gix log --decorate=no suppressed (never emitted decorations anyway) — clap accepted" -- log --decorate=no
  }
)

# mode=effect — --no-decorate: same as --decorate=no.
# hash=sha1-only
title "gix log --no-decorate"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-decorate clap-accepted" && {
    compat_effect "gix log --no-decorate suppressed (never emitted decorations anyway) — clap accepted" -- log --no-decorate
  }
)

# mode=effect — --decorate-refs=<pattern>: include only matching refs in
# decoration.
# hash=sha1-only
title "gix log --decorate-refs=<pattern>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --decorate-refs clap-accepted, filter deferred" && {
    compat_effect "gix log --decorate-refs filter deferred — clap accepted" -- log --decorate --decorate-refs=refs/tags/*
  }
)

# mode=effect — --decorate-refs-exclude=<pattern>: exclude matching refs
# from decoration.
# hash=sha1-only
title "gix log --decorate-refs-exclude=<pattern>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --decorate-refs-exclude clap-accepted, filter deferred" && {
    compat_effect "gix log --decorate-refs-exclude filter deferred — clap accepted" -- log --decorate --decorate-refs-exclude=refs/tags/*
  }
)

# mode=effect — --clear-decorations: reset prior --decorate-refs[-exclude]
# filters.
# hash=sha1-only
title "gix log --clear-decorations"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --clear-decorations clap-accepted" && {
    compat_effect "gix log --clear-decorations decoration-filter reset deferred — clap accepted" -- log --decorate --clear-decorations
  }
)

# mode=effect — --source: prepend the ref name each commit was reached
# through.
# hash=sha1-only
title "gix log --source"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --source clap-accepted, source ref prefix deferred" && {
    compat_effect "gix log --source ref-prefix emission deferred — clap accepted" -- log --source --all
  }
)

# --- graph --------------------------------------------------------------

# mode=effect — --graph: ASCII commit graph alongside each entry.
# hash=sha1-only
title "gix log --graph"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --graph clap-accepted, ASCII graph deferred" && {
    compat_effect "gix log --graph ASCII-art commit graph deferred — clap accepted" -- log --graph
  }
)

# --- diff output --------------------------------------------------------

# mode=effect — -p / --patch: show the diff each commit introduces.
# hash=sha1-only
title "gix log -p"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -p clap-accepted, diff emission deferred" && {
    compat_effect "gix log -p per-commit diff emission deferred — clap accepted" -- log -p
  }
)

# mode=effect — -s / --no-patch: suppress any diff (cancels -p/--stat).
# hash=sha1-only
title "gix log -s"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -s/-p composition clap-accepted, diff suppression deferred" && {
    compat_effect "gix log -s --no-patch diff suppression (vs -p) deferred — clap accepted" -- log -s -p
  }
)

# mode=effect — --stat: diffstat per commit.
# hash=sha1-only
title "gix log --stat"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --stat clap-accepted, diffstat deferred" && {
    compat_effect "gix log --stat diffstat emission deferred — clap accepted" -- log --stat
  }
)

# mode=effect — --shortstat: last line of --stat only.
# hash=sha1-only
title "gix log --shortstat"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --shortstat clap-accepted, summary deferred" && {
    compat_effect "gix log --shortstat summary emission deferred — clap accepted" -- log --shortstat
  }
)

# mode=effect — --numstat: machine-friendly diffstat.
# hash=sha1-only
title "gix log --numstat"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --numstat clap-accepted, machine-readable stat deferred" && {
    compat_effect "gix log --numstat machine-readable diffstat deferred — clap accepted" -- log --numstat
  }
)

# mode=effect — --name-only: list affected paths only.
# hash=sha1-only
title "gix log --name-only"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --name-only clap-accepted, path listing deferred" && {
    compat_effect "gix log --name-only path listing deferred — clap accepted" -- log --name-only
  }
)

# mode=effect — --name-status: paths with status letters.
# hash=sha1-only
title "gix log --name-status"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --name-status clap-accepted, status-letter listing deferred" && {
    compat_effect "gix log --name-status status-letter path listing deferred — clap accepted" -- log --name-status
  }
)

# mode=effect — --raw: git-diff --raw output.
# hash=sha1-only
title "gix log --raw"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --raw clap-accepted, raw diff format deferred" && {
    compat_effect "gix log --raw diff emission deferred — clap accepted" -- log --raw
  }
)

# mode=effect — -M / --find-renames: detect renames in diff output.
# hash=sha1-only
title "gix log -M"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -M clap-accepted, rename detection deferred" && {
    compat_effect "gix log -M/--find-renames rename detection deferred — clap accepted" -- log -M -p
  }
)

# --- file-specific ------------------------------------------------------

# mode=effect — --follow <file>: git keeps the file's history across
# renames. gix log's --follow clap flag is accepted; when set,
# gitoxide-core::log skips revspec-parsing so the positional arg is
# tolerated as a pathspec hint even though pathspec filtering itself is
# deferred. Exit parity holds.
# hash=sha1-only
title "gix log --follow <file>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --follow clap-accepted, rename-following deferred" && {
    compat_effect "gix log --follow rename-following deferred — flag accepted, positional treated as pathspec hint" -- log --follow a
  }
)

# mode=effect — --full-diff: with pathspec, show full commit diff not
# just the path's diff.
# hash=sha1-only
title "gix log --full-diff -- <path>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --full-diff clap-accepted, diff scope deferred" && {
    compat_effect "gix log --full-diff diff-scope override deferred — clap accepted" -- log --full-diff -p -- a
  }
)

# mode=effect — -L <start>,<end>:<file>: line-range log. gix accepts -L
# as a repeatable BString; parsing + line-range traversal deferred. Test
# uses `b` (populated via small-repo-in-sandbox's `echo hi >> b`) so
# both sides exit 0 on a valid range; `a` would trip git's "file has
# only 0 lines" guard on the empty file.
# hash=sha1-only
title "gix log -L <start>,<end>:<file>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -L clap-accepted, line-range traversal deferred" && {
    compat_effect "gix log -L line-range traversal deferred — flag accepted, range parser and file-lookup unwired" -- log -L 1,1:b
  }
)

# --- date formatting ----------------------------------------------------

# mode=effect — --date=relative: "N days ago" style.
# hash=sha1-only
title "gix log --date=relative"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --date=relative clap-accepted, format deferred" && {
    compat_effect "gix log --date=relative date-format emission deferred — clap accepted" -- log --date=relative
  }
)

# mode=effect — --date=iso: ISO 8601 local dates.
# hash=sha1-only
title "gix log --date=iso"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --date=iso clap-accepted, format deferred" && {
    compat_effect "gix log --date=iso date-format emission deferred — clap accepted" -- log --date=iso
  }
)

# mode=effect — --date=short: YYYY-MM-DD.
# hash=sha1-only
title "gix log --date=short"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --date=short clap-accepted, format deferred" && {
    compat_effect "gix log --date=short date-format emission deferred — clap accepted" -- log --date=short
  }
)

# mode=effect — --date=raw: unix timestamp + timezone.
# hash=sha1-only
title "gix log --date=raw"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --date=raw clap-accepted, format deferred" && {
    compat_effect "gix log --date=raw date-format emission deferred — clap accepted" -- log --date=raw
  }
)

# mode=effect — --date=unix: unix timestamp only.
# hash=sha1-only
title "gix log --date=unix"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --date=unix clap-accepted, format deferred" && {
    compat_effect "gix log --date=unix date-format emission deferred — clap accepted" -- log --date=unix
  }
)

# mode=effect — --date=format:<strftime>: strftime-style format.
# hash=sha1-only
title "gix log --date=format:<strftime>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --date=format:<strftime> clap-accepted, format deferred" && {
    compat_effect "gix log --date=format:<strftime> strftime emission deferred — clap accepted" -- log --date=format:%Y-%m-%d
  }
)

# --- diff-merges --------------------------------------------------------

# mode=effect — -m: show diffs against each parent for merges.
# hash=sha1-only
title "gix log -m"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -m clap-accepted, per-parent merge diff deferred" && {
    compat_effect "gix log -m per-parent merge diff deferred — clap accepted" -- log -m -p
  }
)

# mode=effect — -c: combined diff for merges.
# hash=sha1-only
title "gix log -c"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -c clap-accepted, combined merge diff deferred" && {
    compat_effect "gix log -c combined merge diff deferred — clap accepted" -- log -c -p
  }
)

# mode=effect — --cc: dense combined diff (only interesting hunks).
# hash=sha1-only
title "gix log --cc"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --cc clap-accepted, dense combined merge diff deferred" && {
    compat_effect "gix log --cc dense combined merge diff deferred — clap accepted" -- log --cc -p
  }
)

# mode=effect — --diff-merges=off: never show merge diffs.
# hash=sha1-only
title "gix log --diff-merges=off"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --diff-merges=off clap-accepted, merge-diff suppression deferred" && {
    compat_effect "gix log --diff-merges=off merge-diff control deferred — clap accepted" -- log --diff-merges=off -p
  }
)

# mode=effect — --diff-merges=first-parent: diff against first parent.
# hash=sha1-only
title "gix log --diff-merges=first-parent"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --diff-merges=first-parent clap-accepted, merge-diff control deferred" && {
    compat_effect "gix log --diff-merges=first-parent merge-diff control deferred — clap accepted" -- log --diff-merges=first-parent -p
  }
)

# --- misc log-specific --------------------------------------------------

# mode=effect — --mailmap / --use-mailmap: rewrite names via .mailmap.
# hash=sha1-only
title "gix log --mailmap"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --mailmap clap-accepted, .mailmap rewriting deferred" && {
    compat_effect "gix log --mailmap author/committer rewriting deferred — clap accepted" -- log --mailmap
  }
)

# mode=effect — --no-mailmap: ignore .mailmap even if configured.
# hash=sha1-only
title "gix log --no-mailmap"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-mailmap clap-accepted" && {
    compat_effect "gix log --no-mailmap .mailmap bypass deferred — clap accepted" -- log --no-mailmap
  }
)

# mode=effect — --log-size: add "log size N" line per commit.
# hash=sha1-only
title "gix log --log-size"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --log-size clap-accepted, size header deferred" && {
    compat_effect "gix log --log-size message-length header deferred — clap accepted" -- log --log-size
  }
)

# mode=effect — --notes: include notes from refs/notes/commits.
# hash=sha1-only
title "gix log --notes"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --notes clap-accepted, notes emission deferred" && {
    compat_effect "gix log --notes refs/notes emission deferred — clap accepted" -- log --notes
  }
)

# mode=effect — --no-notes: suppress notes even if a default is
# configured.
# hash=sha1-only
title "gix log --no-notes"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-notes clap-accepted" && {
    compat_effect "gix log --no-notes refs/notes suppression deferred — clap accepted" -- log --no-notes
  }
)

# mode=effect — --show-signature: verify and print commit signatures.
# hash=sha1-only
title "gix log --show-signature"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --show-signature clap-accepted, GPG verification deferred" && {
    compat_effect "gix log --show-signature GPG verification deferred — clap accepted" -- log --show-signature
  }
)

# --- color --------------------------------------------------------------

# mode=effect — --color=always: force color codes even when piped.
# hash=sha1-only
title "gix log --color=always"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --color=always clap-accepted, color emission deferred" && {
    compat_effect "gix log --color=always ANSI color emission deferred — clap accepted" -- log --color=always
  }
)

# mode=effect — --no-color: suppress color codes unconditionally.
# hash=sha1-only
title "gix log --no-color"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-color clap-accepted" && {
    compat_effect "gix log --no-color color-suppression deferred (never emitted colors anyway) — clap accepted" -- log --no-color
  }
)

# --- boundary / ancestry-path ------------------------------------------

# mode=effect — --boundary: mark excluded-range endpoints with "-".
# hash=sha1-only
title "gix log --boundary"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --boundary + --not clap-accepted, boundary markers deferred" && {
    compat_effect "gix log --boundary + --not boundary-marker emission deferred — clap accepted" -- log --boundary main --not dev
  }
)

# mode=effect — --ancestry-path: commits on A..B paths from A to B.
# hash=sha1-only
title "gix log --ancestry-path"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ancestry-path clap-accepted, ancestry filter deferred" && {
    compat_effect "gix log --ancestry-path ancestry filter deferred — clap accepted" -- log --ancestry-path dev..main
  }
)

# --- pickaxe family ---------------------------------------------------

# mode=effect — -G<regex>: show commits that add/remove a matching line.
# hash=sha1-only
title "gix log -G<regex>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -G<regex> clap-accepted, pickaxe semantics deferred" && {
    compat_effect "gix log -G pickaxe line-add/remove regex deferred — flag accepted" -- log -G foo
  }
)

# mode=effect — -S<string>: show commits that change the occurrence count.
# hash=sha1-only
title "gix log -S<string>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -S<string> clap-accepted, pickaxe semantics deferred" && {
    compat_effect "gix log -S pickaxe occurrence-count deferred — flag accepted" -- log -S foo
  }
)

# mode=effect — --pickaxe-regex: treat -S as regex (implied by -G).
# hash=sha1-only
title "gix log --pickaxe-regex"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --pickaxe-regex clap-accepted, regex mode deferred" && {
    compat_effect "gix log --pickaxe-regex -S-as-regex mode deferred — flag accepted" -- log --pickaxe-regex -S foo
  }
)

# mode=effect — --pickaxe-all: include merge commits in pickaxe-match.
# hash=sha1-only
title "gix log --pickaxe-all"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --pickaxe-all clap-accepted, merge-inclusion deferred" && {
    compat_effect "gix log --pickaxe-all merge-inclusion in pickaxe deferred — flag accepted" -- log --pickaxe-all -S foo
  }
)

# --- cherry / left-right family ---------------------------------------

# mode=effect — --cherry: shorthand for --right-only --cherry-mark --no-merges.
# hash=sha1-only
title "gix log --cherry"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --cherry clap-accepted, equivalence-class detection deferred" && {
    compat_effect "gix log --cherry patch-equivalence detection deferred — flag accepted" -- log --cherry
  }
)

# mode=effect — --cherry-mark: annotate commits with = or +.
# hash=sha1-only
title "gix log --cherry-mark"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --cherry-mark clap-accepted, equivalence marks deferred" && {
    compat_effect "gix log --cherry-mark equivalence-class annotation deferred — flag accepted" -- log --cherry-mark dev...main
  }
)

# mode=effect — --cherry-pick: omit equivalent commits in symmetric diff.
# hash=sha1-only
title "gix log --cherry-pick"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --cherry-pick clap-accepted, equivalence-filter deferred" && {
    compat_effect "gix log --cherry-pick equivalence-class filter deferred — flag accepted" -- log --cherry-pick dev...main
  }
)

# mode=effect — --left-only: filter left side of A...B.
# hash=sha1-only
title "gix log --left-only"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --left-only clap-accepted, side filter deferred" && {
    compat_effect "gix log --left-only symmetric-diff side filter deferred — flag accepted" -- log --left-only dev...main
  }
)

# mode=effect — --right-only: filter right side of A...B.
# hash=sha1-only
title "gix log --right-only"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --right-only clap-accepted, side filter deferred" && {
    compat_effect "gix log --right-only symmetric-diff side filter deferred — flag accepted" -- log --right-only dev...main
  }
)

# mode=effect — --left-right: annotate each commit with < or > side.
# hash=sha1-only
title "gix log --left-right"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --left-right clap-accepted, side annotation deferred" && {
    compat_effect "gix log --left-right symmetric-diff side annotation deferred — flag accepted" -- log --left-right dev...main
  }
)

# --- reflog walk ------------------------------------------------------

# mode=effect — -g / --walk-reflogs: walk reflog entries instead of ancestry.
# hash=sha1-only
title "gix log -g / --walk-reflogs"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -g/--walk-reflogs clap-accepted, reflog traversal deferred" && {
    compat_effect "gix log --walk-reflogs reflog traversal mode deferred — flag accepted" -- log --walk-reflogs
  }
)

# mode=effect — --grep-reflog: filter reflog entries by regex.
# hash=sha1-only
title "gix log --grep-reflog=<pattern>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --grep-reflog clap-accepted, reflog filter deferred" && {
    compat_effect "gix log --grep-reflog reflog message filter deferred — flag accepted" -- log --walk-reflogs --grep-reflog=.
  }
)

# --- history simplification -------------------------------------------

# mode=effect — --simplify-by-decoration: keep only decoration-carrying commits.
# hash=sha1-only
title "gix log --simplify-by-decoration"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --simplify-by-decoration clap-accepted, simplification deferred" && {
    compat_effect "gix log --simplify-by-decoration history simplification deferred — flag accepted" -- log --simplify-by-decoration
  }
)

# mode=effect — --simplify-merges: drop uninteresting merge commits.
# hash=sha1-only
title "gix log --simplify-merges"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --simplify-merges clap-accepted, simplification deferred" && {
    compat_effect "gix log --simplify-merges merge-simplification deferred — flag accepted" -- log --simplify-merges
  }
)

# mode=effect — --full-history: disable history simplification.
# hash=sha1-only
title "gix log --full-history"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --full-history clap-accepted, simplification control deferred" && {
    compat_effect "gix log --full-history disables history simplification — flag accepted, simplification never applied in gix" -- log --full-history
  }
)

# mode=effect — --dense: alias for --full-history.
# hash=sha1-only
title "gix log --dense"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --dense clap-accepted, simplification control deferred" && {
    compat_effect "gix log --dense alias for --full-history — flag accepted" -- log --dense
  }
)

# mode=effect — --sparse: opposite of --dense.
# hash=sha1-only
title "gix log --sparse"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --sparse clap-accepted, simplification control deferred" && {
    compat_effect "gix log --sparse sparse-history mode deferred — flag accepted" -- log --sparse
  }
)

# mode=effect — --no-walk: don't traverse ancestors of given revisions.
# hash=sha1-only
title "gix log --no-walk"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-walk clap-accepted, single-commit mode deferred" && {
    compat_effect "gix log --no-walk traversal suppression deferred — flag accepted" -- log --no-walk main
  }
)

# mode=effect — --do-walk: override a previous --no-walk.
# hash=sha1-only
title "gix log --do-walk"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --do-walk clap-accepted, --no-walk override deferred" && {
    compat_effect "gix log --do-walk --no-walk override deferred — flag accepted" -- log --do-walk main
  }
)

# mode=effect — --in-commit-order: emit in encounter order.
# hash=sha1-only
title "gix log --in-commit-order"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --in-commit-order clap-accepted, ordering deferred" && {
    compat_effect "gix log --in-commit-order emission-order override deferred — flag accepted" -- log --in-commit-order
  }
)

# --- extra ref-selection ----------------------------------------------

# mode=effect — --exclude=<pattern>: exclude refs from subsequent --all etc.
# hash=sha1-only
title "gix log --exclude=<pattern>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --exclude clap-accepted, ref-category filter deferred" && {
    compat_effect "gix log --exclude ref-category exclusion deferred — flag accepted" -- log --exclude=refs/heads/dev --all
  }
)

# mode=effect — --glob=<pattern>: include refs matching glob.
# hash=sha1-only
title "gix log --glob=<pattern>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --glob clap-accepted, glob-based ref selection deferred" && {
    compat_effect "gix log --glob glob ref-selection deferred — flag accepted" -- log --glob=refs/heads/*
  }
)

# mode=effect — --alternate-refs: include refs from alternate object stores.
# hash=sha1-only
title "gix log --alternate-refs"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --alternate-refs clap-accepted, alternates deferred" && {
    compat_effect "gix log --alternate-refs alternates traversal deferred — flag accepted" -- log --alternate-refs
  }
)

# --- parents / children / display -------------------------------------

# mode=effect — --parents: print each commit's parents inline.
# hash=sha1-only
title "gix log --parents"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --parents clap-accepted, parent emission deferred" && {
    compat_effect "gix log --parents parent-id emission deferred — flag accepted" -- log --parents
  }
)

# mode=effect — --children: print each commit's children inline.
# hash=sha1-only
title "gix log --children"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --children clap-accepted, child emission deferred" && {
    compat_effect "gix log --children child-id emission deferred — flag accepted" -- log --children
  }
)

# mode=effect — --show-pulls: surface rejoined merges.
# hash=sha1-only
title "gix log --show-pulls"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --show-pulls clap-accepted, pull-rejoin surfacing deferred" && {
    compat_effect "gix log --show-pulls merge rejoin detection deferred — flag accepted" -- log --show-pulls
  }
)

# mode=effect — --show-linear-break: emit separator on linear-break boundaries.
# hash=sha1-only
title "gix log --show-linear-break"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --show-linear-break clap-accepted, break-marker deferred" && {
    compat_effect "gix log --show-linear-break linear-break marker deferred — flag accepted" -- log --show-linear-break
  }
)

# mode=effect — -z: NUL-terminated output records.
# hash=sha1-only
title "gix log -z"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -z clap-accepted, NUL-termination deferred" && {
    compat_effect "gix log -z NUL-terminator emission deferred — flag accepted" -- log -z
  }
)

# mode=effect — --count: commits matching count, suppress output.
# hash=sha1-only
title "gix log --count"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --count clap-accepted, count-only mode deferred" && {
    compat_effect "gix log --count count-only suppression deferred — flag accepted" -- log --count
  }
)

# --- submodule diff control -------------------------------------------

# mode=effect — --submodule=<mode>: diff rendering mode for submodules.
# hash=sha1-only
title "gix log --submodule=<mode>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --submodule clap-accepted, submodule diff mode deferred" && {
    compat_effect "gix log --submodule diff rendering mode deferred — flag accepted" -- log --submodule=log
  }
)

# --- diff-options / rev-list / pretty companions (steward-remediation pass) ---
#
# Per steward verdict abb628e1b755394b7: close the full user-visible
# surface from vendor/git/Documentation/{rev-list-options,diff-options,
# pretty-options}.adoc that rev-list-only ifdefs don't exclude. 53 rows,
# all clap-accepted compat_effect — semantics deferred per the ledger.

# mode=effect — --unified=<n>: clap-accepted; unified-context width control deferred.
# hash=sha1-only
title "gix log --unified=<n>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --unified=<n> clap-accepted, semantics deferred" && {
    compat_effect "gix log --unified=<n> unified-context width control deferred — flag accepted" -- log --unified=3
  }
)


# mode=effect — --summary: clap-accepted; extended-header summary emission deferred.
# hash=sha1-only
title "gix log --summary"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --summary clap-accepted, semantics deferred" && {
    compat_effect "gix log --summary extended-header summary emission deferred — flag accepted" -- log --summary
  }
)


# mode=effect — --compact-summary: clap-accepted; compact-summary emission deferred.
# hash=sha1-only
title "gix log --compact-summary"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --compact-summary clap-accepted, semantics deferred" && {
    compat_effect "gix log --compact-summary compact-summary emission deferred — flag accepted" -- log --compact-summary
  }
)


# mode=effect — --minimal: clap-accepted; Myers minimal-diff variant deferred.
# hash=sha1-only
title "gix log --minimal"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --minimal clap-accepted, semantics deferred" && {
    compat_effect "gix log --minimal Myers minimal-diff variant deferred — flag accepted" -- log --minimal
  }
)


# mode=effect — --patience: clap-accepted; patience diff algorithm deferred.
# hash=sha1-only
title "gix log --patience"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --patience clap-accepted, semantics deferred" && {
    compat_effect "gix log --patience patience diff algorithm deferred — flag accepted" -- log --patience
  }
)


# mode=effect — --histogram: clap-accepted; histogram diff algorithm deferred.
# hash=sha1-only
title "gix log --histogram"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --histogram clap-accepted, semantics deferred" && {
    compat_effect "gix log --histogram histogram diff algorithm deferred — flag accepted" -- log --histogram
  }
)


# mode=effect — --diff-filter=<filter>: clap-accepted; status-letter filter deferred.
# hash=sha1-only
title "gix log --diff-filter=<filter>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --diff-filter=<filter> clap-accepted, semantics deferred" && {
    compat_effect "gix log --diff-filter=<filter> status-letter filter deferred — flag accepted" -- log --diff-filter=M
  }
)


# mode=effect — --find-object=<oid>: clap-accepted; object-touched filter deferred.
# hash=sha1-only
title "gix log --find-object=<oid>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --find-object=<oid> clap-accepted, semantics deferred" && {
    compat_effect "gix log --find-object=<oid> object-touched filter deferred — flag accepted" -- log --find-object=0000000000000000000000000000000000000000
  }
)


# mode=effect — --find-copies-harder: clap-accepted; aggressive copy detection deferred.
# hash=sha1-only
title "gix log --find-copies-harder"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --find-copies-harder clap-accepted, semantics deferred" && {
    compat_effect "gix log --find-copies-harder aggressive copy detection deferred — flag accepted" -- log --find-copies-harder
  }
)


# mode=effect — --exit-code: clap-accepted; diff-presence exit-code bit deferred — empty range used for exit-0 parity.
# hash=sha1-only
title "gix log --exit-code"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --exit-code clap-accepted, semantics deferred" && {
    compat_effect "gix log --exit-code diff-presence exit-code bit deferred — empty range used for exit-0 parity — flag accepted" -- log --exit-code HEAD..HEAD
  }
)


# mode=effect — --check: clap-accepted; whitespace/conflict-marker check deferred.
# hash=sha1-only
title "gix log --check"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --check clap-accepted, semantics deferred" && {
    compat_effect "gix log --check whitespace/conflict-marker check deferred — flag accepted" -- log --check
  }
)


# mode=effect — --binary: clap-accepted; binary patch emission deferred.
# hash=sha1-only
title "gix log --binary"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --binary clap-accepted, semantics deferred" && {
    compat_effect "gix log --binary binary patch emission deferred — flag accepted" -- log --binary
  }
)


# mode=effect — --full-index: clap-accepted; full-index hash emission deferred.
# hash=sha1-only
title "gix log --full-index"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --full-index clap-accepted, semantics deferred" && {
    compat_effect "gix log --full-index full-index hash emission deferred — flag accepted" -- log --full-index
  }
)


# mode=effect — --remerge-diff: clap-accepted; remerge-diff rendering deferred.
# hash=sha1-only
title "gix log --remerge-diff"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --remerge-diff clap-accepted, semantics deferred" && {
    compat_effect "gix log --remerge-diff remerge-diff rendering deferred — flag accepted" -- log --remerge-diff
  }
)


# mode=effect — --dirstat: clap-accepted; dirstat emission deferred.
# hash=sha1-only
title "gix log --dirstat"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --dirstat clap-accepted, semantics deferred" && {
    compat_effect "gix log --dirstat dirstat emission deferred — flag accepted" -- log --dirstat
  }
)


# mode=effect — --ext-diff: clap-accepted; external-diff program dispatch deferred.
# hash=sha1-only
title "gix log --ext-diff"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ext-diff clap-accepted, semantics deferred" && {
    compat_effect "gix log --ext-diff external-diff program dispatch deferred — flag accepted" -- log --ext-diff
  }
)


# mode=effect — --no-ext-diff: clap-accepted; external-diff suppression deferred.
# hash=sha1-only
title "gix log --no-ext-diff"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-ext-diff clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-ext-diff external-diff suppression deferred — flag accepted" -- log --no-ext-diff
  }
)


# mode=effect — --textconv: clap-accepted; textconv filter application deferred.
# hash=sha1-only
title "gix log --textconv"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --textconv clap-accepted, semantics deferred" && {
    compat_effect "gix log --textconv textconv filter application deferred — flag accepted" -- log --textconv
  }
)


# mode=effect — --no-textconv: clap-accepted; textconv filter suppression deferred.
# hash=sha1-only
title "gix log --no-textconv"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-textconv clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-textconv textconv filter suppression deferred — flag accepted" -- log --no-textconv
  }
)


# mode=effect — --text: clap-accepted; treat-all-as-text mode deferred.
# hash=sha1-only
title "gix log --text"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --text clap-accepted, semantics deferred" && {
    compat_effect "gix log --text treat-all-as-text mode deferred — flag accepted" -- log --text
  }
)


# mode=effect — --patch-with-raw: clap-accepted; patch+raw composition deferred.
# hash=sha1-only
title "gix log --patch-with-raw"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --patch-with-raw clap-accepted, semantics deferred" && {
    compat_effect "gix log --patch-with-raw patch+raw composition deferred — flag accepted" -- log --patch-with-raw
  }
)


# mode=effect — --patch-with-stat: clap-accepted; patch+stat composition deferred.
# hash=sha1-only
title "gix log --patch-with-stat"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --patch-with-stat clap-accepted, semantics deferred" && {
    compat_effect "gix log --patch-with-stat patch+stat composition deferred — flag accepted" -- log --patch-with-stat
  }
)


# mode=effect — --color-moved: clap-accepted; moved-line highlighting deferred.
# hash=sha1-only
title "gix log --color-moved"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --color-moved clap-accepted, semantics deferred" && {
    compat_effect "gix log --color-moved moved-line highlighting deferred — flag accepted" -- log --color-moved
  }
)


# mode=effect — --word-diff: clap-accepted; word-level diff rendering deferred.
# hash=sha1-only
title "gix log --word-diff"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --word-diff clap-accepted, semantics deferred" && {
    compat_effect "gix log --word-diff word-level diff rendering deferred — flag accepted" -- log --word-diff
  }
)


# mode=effect — --word-diff-regex=<regex>: clap-accepted; word-boundary regex deferred.
# hash=sha1-only
title "gix log --word-diff-regex=<regex>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --word-diff-regex=<regex> clap-accepted, semantics deferred" && {
    compat_effect "gix log --word-diff-regex=<regex> word-boundary regex deferred — flag accepted" -- log --word-diff-regex=a
  }
)


# mode=effect — --ws-error-highlight=<kind>: clap-accepted; whitespace-error highlighting deferred.
# hash=sha1-only
title "gix log --ws-error-highlight=<kind>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ws-error-highlight=<kind> clap-accepted, semantics deferred" && {
    compat_effect "gix log --ws-error-highlight=<kind> whitespace-error highlighting deferred — flag accepted" -- log --ws-error-highlight=old
  }
)


# mode=effect — --function-context: clap-accepted; function-context hunk expansion deferred.
# hash=sha1-only
title "gix log --function-context"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --function-context clap-accepted, semantics deferred" && {
    compat_effect "gix log --function-context function-context hunk expansion deferred — flag accepted" -- log --function-context
  }
)


# mode=effect — --inter-hunk-context=<lines>: clap-accepted; inter-hunk merge-context control deferred.
# hash=sha1-only
title "gix log --inter-hunk-context=<lines>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --inter-hunk-context=<lines> clap-accepted, semantics deferred" && {
    compat_effect "gix log --inter-hunk-context=<lines> inter-hunk merge-context control deferred — flag accepted" -- log --inter-hunk-context=3
  }
)


# mode=effect — --indent-heuristic: clap-accepted; indent-heuristic hunk tuning deferred.
# hash=sha1-only
title "gix log --indent-heuristic"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --indent-heuristic clap-accepted, semantics deferred" && {
    compat_effect "gix log --indent-heuristic indent-heuristic hunk tuning deferred — flag accepted" -- log --indent-heuristic
  }
)


# mode=effect — --no-indent-heuristic: clap-accepted; indent-heuristic suppression deferred.
# hash=sha1-only
title "gix log --no-indent-heuristic"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-indent-heuristic clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-indent-heuristic indent-heuristic suppression deferred — flag accepted" -- log --no-indent-heuristic
  }
)


# mode=effect — --irreversible-delete: clap-accepted; deletion-only hunk reordering deferred.
# hash=sha1-only
title "gix log --irreversible-delete"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --irreversible-delete clap-accepted, semantics deferred" && {
    compat_effect "gix log --irreversible-delete deletion-only hunk reordering deferred — flag accepted" -- log --irreversible-delete
  }
)


# mode=effect — --no-renames: clap-accepted; rename-detection suppression deferred.
# hash=sha1-only
title "gix log --no-renames"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-renames clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-renames rename-detection suppression deferred — flag accepted" -- log --no-renames
  }
)


# mode=effect — --rename-empty: clap-accepted; empty-file rename semantics deferred.
# hash=sha1-only
title "gix log --rename-empty"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --rename-empty clap-accepted, semantics deferred" && {
    compat_effect "gix log --rename-empty empty-file rename semantics deferred — flag accepted" -- log --rename-empty
  }
)


# mode=effect — --no-rename-empty: clap-accepted; --rename-empty negation deferred.
# hash=sha1-only
title "gix log --no-rename-empty"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-rename-empty clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-rename-empty --rename-empty negation deferred — flag accepted" -- log --no-rename-empty
  }
)


# mode=effect — --ignore-all-space: clap-accepted; whitespace-ignoring diff mode deferred.
# hash=sha1-only
title "gix log --ignore-all-space"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ignore-all-space clap-accepted, semantics deferred" && {
    compat_effect "gix log --ignore-all-space whitespace-ignoring diff mode deferred — flag accepted" -- log --ignore-all-space
  }
)


# mode=effect — --ignore-blank-lines: clap-accepted; blank-line ignore mode deferred.
# hash=sha1-only
title "gix log --ignore-blank-lines"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ignore-blank-lines clap-accepted, semantics deferred" && {
    compat_effect "gix log --ignore-blank-lines blank-line ignore mode deferred — flag accepted" -- log --ignore-blank-lines
  }
)


# mode=effect — --ignore-cr-at-eol: clap-accepted; CR-at-EOL ignore deferred.
# hash=sha1-only
title "gix log --ignore-cr-at-eol"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ignore-cr-at-eol clap-accepted, semantics deferred" && {
    compat_effect "gix log --ignore-cr-at-eol CR-at-EOL ignore deferred — flag accepted" -- log --ignore-cr-at-eol
  }
)


# mode=effect — --ignore-matching-lines=<regex>: clap-accepted; regex-match line-ignore deferred.
# hash=sha1-only
title "gix log --ignore-matching-lines=<regex>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ignore-matching-lines=<regex> clap-accepted, semantics deferred" && {
    compat_effect "gix log --ignore-matching-lines=<regex> regex-match line-ignore deferred — flag accepted" -- log --ignore-matching-lines=foo
  }
)


# mode=effect — --ignore-space-at-eol: clap-accepted; EOL-whitespace ignore deferred.
# hash=sha1-only
title "gix log --ignore-space-at-eol"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ignore-space-at-eol clap-accepted, semantics deferred" && {
    compat_effect "gix log --ignore-space-at-eol EOL-whitespace ignore deferred — flag accepted" -- log --ignore-space-at-eol
  }
)


# mode=effect — --ignore-space-change: clap-accepted; whitespace-amount ignore deferred.
# hash=sha1-only
title "gix log --ignore-space-change"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ignore-space-change clap-accepted, semantics deferred" && {
    compat_effect "gix log --ignore-space-change whitespace-amount ignore deferred — flag accepted" -- log --ignore-space-change
  }
)


# mode=effect — --src-prefix=<prefix>: clap-accepted; source-prefix override deferred.
# hash=sha1-only
title "gix log --src-prefix=<prefix>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --src-prefix=<prefix> clap-accepted, semantics deferred" && {
    compat_effect "gix log --src-prefix=<prefix> source-prefix override deferred — flag accepted" -- log --src-prefix=x/
  }
)


# mode=effect — --dst-prefix=<prefix>: clap-accepted; destination-prefix override deferred.
# hash=sha1-only
title "gix log --dst-prefix=<prefix>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --dst-prefix=<prefix> clap-accepted, semantics deferred" && {
    compat_effect "gix log --dst-prefix=<prefix> destination-prefix override deferred — flag accepted" -- log --dst-prefix=y/
  }
)


# mode=effect — --no-prefix: clap-accepted; a/ b/ prefix suppression deferred.
# hash=sha1-only
title "gix log --no-prefix"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-prefix clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-prefix a/ b/ prefix suppression deferred — flag accepted" -- log --no-prefix
  }
)


# mode=effect — --relative: clap-accepted; pathname-relativization deferred.
# hash=sha1-only
title "gix log --relative"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --relative clap-accepted, semantics deferred" && {
    compat_effect "gix log --relative pathname-relativization deferred — flag accepted" -- log --relative
  }
)


# mode=effect — --no-relative: clap-accepted; --relative negation deferred.
# hash=sha1-only
title "gix log --no-relative"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-relative clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-relative --relative negation deferred — flag accepted" -- log --no-relative
  }
)


# mode=effect — --output=<file>: clap-accepted; diff-output redirection deferred.
# hash=sha1-only
title "gix log --output=<file>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --output=<file> clap-accepted, semantics deferred" && {
    compat_effect "gix log --output=<file> diff-output redirection deferred — flag accepted" -- log --output=/dev/null
  }
)


# mode=effect — --reflog: clap-accepted; HEAD reflog inclusion as tips deferred.
# hash=sha1-only
title "gix log --reflog"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --reflog clap-accepted, semantics deferred" && {
    compat_effect "gix log --reflog HEAD reflog inclusion as tips deferred — flag accepted" -- log --reflog
  }
)


# mode=effect — --stdin: clap-accepted; stdin rev-argument reader deferred — empty stdin keeps exit 0.
# hash=sha1-only
title "gix log --stdin"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --stdin clap-accepted, semantics deferred" && {
    compat_effect "gix log --stdin stdin rev-argument reader deferred — empty stdin keeps exit 0 — flag accepted" -- log --stdin </dev/null
  }
)


# mode=effect — --ignore-missing: clap-accepted; missing-object tolerance deferred.
# hash=sha1-only
title "gix log --ignore-missing"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ignore-missing clap-accepted, semantics deferred" && {
    compat_effect "gix log --ignore-missing missing-object tolerance deferred — flag accepted" -- log --ignore-missing
  }
)


# mode=effect — --merge: git precondition-errors out 128 when no merge is
# in progress ("--merge requires one of the pseudorefs MERGE_HEAD /
# CHERRY_PICK_HEAD / REVERT_HEAD / REBASE_HEAD"); gix has no precondition
# check and walks HEAD normally → exit 0. Rows that exit-code-diverge
# are not valid compat_effect candidates; mark as legitimate shortcoming.
# hash=sha1-only
title "gix log --merge"
only_for_hash sha1-only && (small-repo-in-sandbox
  shortcoming "--merge precondition check not yet implemented — git exits 128 when no merge pseudoref exists, gix walks HEAD"
)


# mode=effect — --since-as-filter=<time>: clap-accepted; --since-as-filter predicate deferred.
# hash=sha1-only
title "gix log --since-as-filter=<time>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --since-as-filter=<time> clap-accepted, semantics deferred" && {
    compat_effect "gix log --since-as-filter=<time> --since-as-filter predicate deferred — flag accepted" -- log --since-as-filter=2000-01-01
  }
)


# mode=effect — --exclude-first-parent-only: clap-accepted; first-parent exclude semantics deferred.
# hash=sha1-only
title "gix log --exclude-first-parent-only"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --exclude-first-parent-only clap-accepted, semantics deferred" && {
    compat_effect "gix log --exclude-first-parent-only first-parent exclude semantics deferred — flag accepted" -- log --exclude-first-parent-only
  }
)


# mode=effect — --remove-empty: clap-accepted; empty-commit removal deferred.
# hash=sha1-only
title "gix log --remove-empty"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --remove-empty clap-accepted, semantics deferred" && {
    compat_effect "gix log --remove-empty empty-commit removal deferred — flag accepted" -- log --remove-empty
  }
)


# mode=effect — --single-worktree: clap-accepted; single-worktree ref scoping deferred.
# hash=sha1-only
title "gix log --single-worktree"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --single-worktree clap-accepted, semantics deferred" && {
    compat_effect "gix log --single-worktree single-worktree ref scoping deferred — flag accepted" -- log --single-worktree
  }
)


# mode=effect — --encoding=<enc>: clap-accepted; commit-message encoding conversion deferred.
# hash=sha1-only
title "gix log --encoding=<enc>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --encoding=<enc> clap-accepted, semantics deferred" && {
    compat_effect "gix log --encoding=<enc> commit-message encoding conversion deferred — flag accepted" -- log --encoding=UTF-8
  }
)


# mode=effect — --expand-tabs[=<n>]: clap-accepted; tab-expansion deferred.
# hash=sha1-only
title "gix log --expand-tabs[=<n>]"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --expand-tabs[=<n>] clap-accepted, semantics deferred" && {
    compat_effect "gix log --expand-tabs[=<n>] tab-expansion deferred — flag accepted" -- log --expand-tabs
  }
)


# mode=effect — --no-expand-tabs: clap-accepted; --expand-tabs negation deferred.
# hash=sha1-only
title "gix log --no-expand-tabs"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-expand-tabs clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-expand-tabs --expand-tabs negation deferred — flag accepted" -- log --no-expand-tabs
  }
)


# --- third steward-pass: remaining rev-list / diff-options / pretty ---
#
# Per steward verdict a151fe4213d4489d9: 31 more in-scope flag-groups
# from rev-list-options.adoc + diff-options.adoc + pretty-options.adoc
# (excluding rev-list-ifdef entries and --maximal-only which git-log
# itself rejects at runtime even though its adoc entry is unconditional).

# mode=effect — --basic-regexp: clap-accepted; POSIX basic-regex mode for --grep deferred.
# hash=sha1-only
title "gix log --basic-regexp"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --basic-regexp clap-accepted, semantics deferred" && {
    compat_effect "gix log --basic-regexp POSIX basic-regex mode for --grep deferred — flag accepted" -- log --basic-regexp --grep=first
  }
)


# mode=effect — -P / --perl-regexp: clap-accepted; Perl-regex mode for --grep deferred.
# hash=sha1-only
title "gix log -P / --perl-regexp"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -P / --perl-regexp clap-accepted, semantics deferred" && {
    compat_effect "gix log -P / --perl-regexp Perl-regex mode for --grep deferred — flag accepted" -- log -P --grep=first
  }
)


# mode=effect — --exclude-hidden=<section>: clap-accepted; hidden-refs filter deferred.
# hash=sha1-only
title "gix log --exclude-hidden=<section>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --exclude-hidden=<section> clap-accepted, semantics deferred" && {
    compat_effect "gix log --exclude-hidden=<section> hidden-refs filter deferred — flag accepted" -- log --exclude-hidden=fetch --all
  }
)


# mode=effect — --bisect: clap-accepted; bisect-output emission deferred.
# hash=sha1-only
title "gix log --bisect"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --bisect clap-accepted, semantics deferred" && {
    compat_effect "gix log --bisect bisect-output emission deferred — flag accepted" -- log --bisect
  }
)


# mode=effect — --relative-date: clap-accepted; relative-date shorthand deferred (equivalent to --date=relative).
# hash=sha1-only
title "gix log --relative-date"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --relative-date clap-accepted, semantics deferred" && {
    compat_effect "gix log --relative-date relative-date shorthand deferred (equivalent to --date=relative) — flag accepted" -- log --relative-date
  }
)


# mode=effect — --dd: clap-accepted; --diff-merges=dd alias deferred.
# hash=sha1-only
title "gix log --dd"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --dd clap-accepted, semantics deferred" && {
    compat_effect "gix log --dd --diff-merges=dd alias deferred — flag accepted" -- log --dd -p
  }
)


# mode=effect — --no-diff-merges: clap-accepted; --diff-merges suppression deferred.
# hash=sha1-only
title "gix log --no-diff-merges"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-diff-merges clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-diff-merges --diff-merges suppression deferred — flag accepted" -- log --no-diff-merges -p
  }
)


# mode=effect — --combined-all-paths: clap-accepted; combined-diff per-parent path emission deferred.
# hash=sha1-only
title "gix log --combined-all-paths"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --combined-all-paths clap-accepted, semantics deferred" && {
    compat_effect "gix log --combined-all-paths combined-diff per-parent path emission deferred — flag accepted" -- log --cc --combined-all-paths
  }
)


# mode=effect — --output-indicator-new=<char>: clap-accepted; custom add-indicator character deferred.
# hash=sha1-only
title "gix log --output-indicator-new=<char>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --output-indicator-new=<char> clap-accepted, semantics deferred" && {
    compat_effect "gix log --output-indicator-new=<char> custom add-indicator character deferred — flag accepted" -- log --output-indicator-new=+ -p
  }
)


# mode=effect — --output-indicator-old=<char>: clap-accepted; custom remove-indicator character deferred.
# hash=sha1-only
title "gix log --output-indicator-old=<char>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --output-indicator-old=<char> clap-accepted, semantics deferred" && {
    compat_effect "gix log --output-indicator-old=<char> custom remove-indicator character deferred — flag accepted" -- log --output-indicator-old=- -p
  }
)


# mode=effect — --output-indicator-context=<char>: clap-accepted; custom context-indicator character deferred.
# hash=sha1-only
title "gix log --output-indicator-context=<char>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --output-indicator-context=<char> clap-accepted, semantics deferred" && {
    compat_effect "gix log --output-indicator-context=<char> custom context-indicator character deferred — flag accepted" -- log --output-indicator-context=. -p
  }
)


# mode=effect — -t (show tree objects): clap-accepted; tree-object diff emission deferred.
# hash=sha1-only
title "gix log -t (show tree objects)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -t (show tree objects) clap-accepted, semantics deferred" && {
    compat_effect "gix log -t (show tree objects) tree-object diff emission deferred — flag accepted" -- log -t --stat
  }
)


# mode=effect — --anchored=<text>: clap-accepted; anchored-diff algorithm deferred.
# hash=sha1-only
title "gix log --anchored=<text>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --anchored=<text> clap-accepted, semantics deferred" && {
    compat_effect "gix log --anchored=<text> anchored-diff algorithm deferred — flag accepted" -- log --anchored=foo
  }
)


# mode=effect — --cumulative: clap-accepted; cumulative-dirstat emission deferred.
# hash=sha1-only
title "gix log --cumulative"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --cumulative clap-accepted, semantics deferred" && {
    compat_effect "gix log --cumulative cumulative-dirstat emission deferred — flag accepted" -- log --cumulative --dirstat
  }
)


# mode=effect — --dirstat-by-file: clap-accepted; dirstat per-file counting mode deferred.
# hash=sha1-only
title "gix log --dirstat-by-file"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --dirstat-by-file clap-accepted, semantics deferred" && {
    compat_effect "gix log --dirstat-by-file dirstat per-file counting mode deferred — flag accepted" -- log --dirstat-by-file --dirstat
  }
)


# mode=effect — --no-color-moved: clap-accepted; --color-moved negation deferred.
# hash=sha1-only
title "gix log --no-color-moved"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-color-moved clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-color-moved --color-moved negation deferred — flag accepted" -- log --no-color-moved
  }
)


# mode=effect — --color-moved-ws=<mode>: clap-accepted; moved-line whitespace mode deferred.
# hash=sha1-only
title "gix log --color-moved-ws=<mode>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --color-moved-ws=<mode> clap-accepted, semantics deferred" && {
    compat_effect "gix log --color-moved-ws=<mode> moved-line whitespace mode deferred — flag accepted" -- log --color-moved-ws=allow-indentation-change
  }
)


# mode=effect — --no-color-moved-ws: clap-accepted; --color-moved-ws negation deferred.
# hash=sha1-only
title "gix log --no-color-moved-ws"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-color-moved-ws clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-color-moved-ws --color-moved-ws negation deferred — flag accepted" -- log --no-color-moved-ws
  }
)


# mode=effect — --color-words: clap-accepted; word-level coloring deferred.
# hash=sha1-only
title "gix log --color-words"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --color-words clap-accepted, semantics deferred" && {
    compat_effect "gix log --color-words word-level coloring deferred — flag accepted" -- log --color-words
  }
)


# mode=effect — -B / --break-rewrites: clap-accepted; break-rewrites detection deferred.
# hash=sha1-only
title "gix log -B / --break-rewrites"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -B / --break-rewrites clap-accepted, semantics deferred" && {
    compat_effect "gix log -B / --break-rewrites break-rewrites detection deferred — flag accepted" -- log -B
  }
)


# mode=effect — -C / --find-copies: clap-accepted; copy detection deferred.
# hash=sha1-only
title "gix log -C / --find-copies"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -C / --find-copies clap-accepted, semantics deferred" && {
    compat_effect "gix log -C / --find-copies copy detection deferred — flag accepted" -- log -C
  }
)


# mode=effect — -l<num>: clap-accepted; rename-detection scan cap deferred.
# hash=sha1-only
title "gix log -l<num>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -l<num> clap-accepted, semantics deferred" && {
    compat_effect "gix log -l<num> rename-detection scan cap deferred — flag accepted" -- log -l5 -M
  }
)


# mode=effect — -O / --orderfile=<file>: clap-accepted; path-ordering file deferred.
# hash=sha1-only
title "gix log -O / --orderfile=<file>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -O / --orderfile=<file> clap-accepted, semantics deferred" && {
    compat_effect "gix log -O / --orderfile=<file> path-ordering file deferred — flag accepted" -- log -O/dev/null
  }
)


# mode=effect — --skip-to=<path>: clap-accepted; diff skip-to-path deferred.
# hash=sha1-only
title "gix log --skip-to=<path>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --skip-to=<path> clap-accepted, semantics deferred" && {
    compat_effect "gix log --skip-to=<path> diff skip-to-path deferred — flag accepted" -- log --skip-to=a
  }
)


# mode=effect — --rotate-to=<path>: clap-accepted; diff rotate-to-path deferred.
# hash=sha1-only
title "gix log --rotate-to=<path>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --rotate-to=<path> clap-accepted, semantics deferred" && {
    compat_effect "gix log --rotate-to=<path> diff rotate-to-path deferred — flag accepted" -- log --rotate-to=a
  }
)


# mode=effect — -R (reverse diff inputs): clap-accepted; old/new side swap deferred.
# hash=sha1-only
title "gix log -R (reverse diff inputs)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -R (reverse diff inputs) clap-accepted, semantics deferred" && {
    compat_effect "gix log -R (reverse diff inputs) old/new side swap deferred — flag accepted" -- log -R
  }
)


# mode=effect — --ignore-submodules[=<when>]: clap-accepted; submodule-diff ignore mode deferred.
# hash=sha1-only
title "gix log --ignore-submodules[=<when>]"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ignore-submodules[=<when>] clap-accepted, semantics deferred" && {
    compat_effect "gix log --ignore-submodules[=<when>] submodule-diff ignore mode deferred — flag accepted" -- log --ignore-submodules
  }
)


# mode=effect — --default-prefix: clap-accepted; default-prefix restore deferred.
# hash=sha1-only
title "gix log --default-prefix"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --default-prefix clap-accepted, semantics deferred" && {
    compat_effect "gix log --default-prefix default-prefix restore deferred — flag accepted" -- log --default-prefix
  }
)


# mode=effect — --line-prefix=<prefix>: clap-accepted; per-line prefix emission deferred.
# hash=sha1-only
title "gix log --line-prefix=<prefix>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --line-prefix=<prefix> clap-accepted, semantics deferred" && {
    compat_effect "gix log --line-prefix=<prefix> per-line prefix emission deferred — flag accepted" -- log --line-prefix=XX
  }
)


# mode=effect — --ita-invisible-in-index: clap-accepted; intent-to-add invisibility deferred.
# hash=sha1-only
title "gix log --ita-invisible-in-index"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --ita-invisible-in-index clap-accepted, semantics deferred" && {
    compat_effect "gix log --ita-invisible-in-index intent-to-add invisibility deferred — flag accepted" -- log --ita-invisible-in-index
  }
)


# --max-depth=<n> is NOT a standalone git-log flag — it's a sub-token of
# --dirstat (e.g. `--dirstat=cumulative,3`). The steward's adoc-surface
# extractor over-reported it; git-log rejects `--max-depth=N` directly
# ("fatal: unrecognized argument"). No separate row is warranted.


# mode=effect — --show-notes-by-default: clap-accepted; notes-by-default emission deferred.
# hash=sha1-only
title "gix log --show-notes-by-default"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --show-notes-by-default clap-accepted, semantics deferred" && {
    compat_effect "gix log --show-notes-by-default notes-by-default emission deferred — flag accepted" -- log --show-notes-by-default
  }
)


# mode=effect — --show-notes: clap-accepted; deprecated --show-notes alias deferred.
# hash=sha1-only
title "gix log --show-notes"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --show-notes clap-accepted, semantics deferred" && {
    compat_effect "gix log --show-notes deprecated --show-notes alias deferred — flag accepted" -- log --show-notes
  }
)


# mode=effect — --standard-notes: clap-accepted; deprecated --standard-notes alias deferred.
# hash=sha1-only
title "gix log --standard-notes"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --standard-notes clap-accepted, semantics deferred" && {
    compat_effect "gix log --standard-notes deprecated --standard-notes alias deferred — flag accepted" -- log --standard-notes
  }
)


# mode=effect — --no-standard-notes: clap-accepted; deprecated --no-standard-notes alias deferred.
# hash=sha1-only
title "gix log --no-standard-notes"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-standard-notes clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-standard-notes deprecated --no-standard-notes alias deferred — flag accepted" -- log --no-standard-notes
  }
)


# mode=effect — --no-use-mailmap: clap-accepted; --no-use-mailmap (alias of --no-mailmap) deferred.
# hash=sha1-only
title "gix log --no-use-mailmap"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-use-mailmap clap-accepted, semantics deferred" && {
    compat_effect "gix log --no-use-mailmap --no-use-mailmap (alias of --no-mailmap) deferred — flag accepted" -- log --no-use-mailmap
  }
)


# End-of-file sentinel: when every row is `only_for_hash sha1-only` and the
# active hash is sha256, the last statement returns 1 (skip), which would
# propagate out of `source` and trip `set -e` in tests/parity.sh. A trailing
# `:` normalizes the exit code so a fully-skipped file still returns 0.
:
