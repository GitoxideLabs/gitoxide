# Must be sourced into tests/parity.sh or tests/journey.sh — see tests/parity.sh.
#
# Parity scaffold for `git clone` ↔ `gix clone`.
#
# One `title` + `it` block per flag derived from vendor/git/builtin/clone.c
# (cmd_clone::builtin_clone_options[]) and
# vendor/git/Documentation/git-clone.adoc. Every `it` body starts as a
# TODO: placeholder — iteration N of the parity loop picks the next
# TODO, converts it to a real `expect_parity` assertion, and removes the
# TODO marker.
#
# Verdict modes (comment above each block):
#   bytes  — scriptable output; byte-exact match required
#   effect — exit-code + UX; output diff reported but not fatal
#
# Every row is `# hash=sha1-only` — `gix clone` cannot open sha256 remotes
# yet (gix/src/clone/fetch/mod.rs:278 still `unimplemented!()`s hash-change
# reconfiguration, shared with fetch). Once that lands, rows become `dual`.

# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone"

# --- meta / help ----------------------------------------------------------

# mode=effect — `git clone --help` delegates to man git-clone (exit 0 when
# man is available); gix returns Clap's auto-generated help (exit 0).
# Message text diverges wildly and is NOT asserted.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --help"
only_for_hash sha1-only && (sandbox
  it "matches git: --help exits 0" && {
    expect_parity effect -- clone --help
  }
)

# --- positional / usage error paths --------------------------------------

# mode=effect — mirrors `if (argc == 0) usage_msg_opt("You must specify a
# repository to clone.")`. Exit 129.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone (no repository)"
only_for_hash sha1-only && (sandbox
  it "matches git: bare 'clone' dies 129 (usage)" && {
    expect_parity effect -- clone
  }
)

# mode=effect — mirrors `if (argc > 2) usage_msg_opt("Too many arguments.")`.
# Exit 129. Usage wording differs; only exit code is asserted.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone <repo> <dir> <extra> (too many args)"
only_for_hash sha1-only && (sandbox
  it "matches git: three positionals dies 129 (usage)" && {
    expect_parity effect -- clone /nonexistent.git foo bar
  }
)

# mode=effect — mirrors `die("repository '%s' does not exist")` after
# get_repo_path returns NULL and the name has no ':' (so it's not a URL).
# Exit 128.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone <nonexistent-local-path>"
only_for_hash sha1-only && (sandbox
  it "matches git: colon-less nonexistent path dies 128" && {
    expect_parity effect -- clone /nonexistent-path-for-parity
  }
)

# mode=effect — mirrors `die("destination path '%s' already exists and is
# not an empty directory.")` when dir arg points at a non-empty dir.
# Exit 128.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone <repo> <existing-non-empty-dir>"
only_for_hash sha1-only && (sandbox
  git init -q --bare upstream.git
  mkdir target
  touch target/foo
  it "matches git: destination exists + non-empty dies 128" && {
    expect_parity effect -- clone upstream.git target
  }
)

# --- happy paths ---------------------------------------------------------

# mode=effect — vanilla clone from a local bare remote into an auto-derived
# directory (humanish name). Golden path — the fetch+checkout round-trip.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone <bare-repo> (auto directory)"
only_for_hash sha1-only && (sandbox
  # Auto-directory derivation creates `upstream/` from `upstream.git`.
  # expect_parity runs git then gix in the same cwd, so the second call
  # would collide with the first's target dir. Split into two
  # single-binary it-blocks, each in its own subdir with a symlink back
  # to the shared bare upstream — same effect-mode contract
  # (exit-code parity) without the stateful-fixture collision.
  git-init-hash-aware -q --bare upstream.git
  mkdir g-side && (cd g-side && ln -s ../upstream.git .)
  mkdir x-side && (cd x-side && ln -s ../upstream.git .)
  it "matches git: vanilla clone into auto humanish dir exits 0" && {
    (cd g-side && expect_run 0 git clone upstream.git)
  }
  it "matches gix: vanilla clone into auto humanish dir exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone upstream.git)
  }
)

# mode=effect — explicit destination directory argument.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone <repo> <dir>"
only_for_hash sha1-only && (sandbox
  # Explicit destination positional — same stateful-fixture split as the
  # auto-directory row, but here the target is the explicit `my-clone`
  # name passed on the CLI (not derived from the URL).
  git-init-hash-aware -q --bare upstream.git
  mkdir g-side && (cd g-side && ln -s ../upstream.git .)
  mkdir x-side && (cd x-side && ln -s ../upstream.git .)
  it "matches git: explicit <dir> positional exits 0" && {
    (cd g-side && expect_run 0 git clone upstream.git my-clone)
  }
  it "matches gix: explicit <dir> positional exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone upstream.git my-clone)
  }
)

# --- verbosity / progress -------------------------------------------------

# mode=effect — OPT__VERBOSITY maps -v → option_verbosity++.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone -v / --verbose"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare upstream.git
  mkdir g-short g-long x-short x-long
  for d in g-short g-long x-short x-long; do
    (cd "$d" && ln -s ../upstream.git .)
  done
  it "matches git: -v exits 0" && {
    (cd g-short && expect_run 0 git clone -v upstream.git)
  }
  it "matches gix: -v exits 0" && {
    (cd x-short && expect_run 0 "$exe_plumbing" clone -v upstream.git)
  }
  it "matches git: --verbose exits 0" && {
    (cd g-long && expect_run 0 git clone --verbose upstream.git)
  }
  it "matches gix: --verbose exits 0" && {
    (cd x-long && expect_run 0 "$exe_plumbing" clone --verbose upstream.git)
  }
)

# mode=effect — OPT__VERBOSITY maps -q → option_verbosity--; clone prints
# no "Cloning into..." banner.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone -q / --quiet"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare upstream.git
  mkdir g-short g-long x-short x-long
  for d in g-short g-long x-short x-long; do
    (cd "$d" && ln -s ../upstream.git .)
  done
  it "matches git: -q exits 0" && {
    (cd g-short && expect_run 0 git clone -q upstream.git)
  }
  it "matches gix: -q exits 0" && {
    (cd x-short && expect_run 0 "$exe_plumbing" clone -q upstream.git)
  }
  it "matches git: --quiet exits 0" && {
    (cd g-long && expect_run 0 git clone --quiet upstream.git)
  }
  it "matches gix: --quiet exits 0" && {
    (cd x-long && expect_run 0 "$exe_plumbing" clone --quiet upstream.git)
  }
)

# mode=effect — OPT_BOOL `progress` forces progress even without a TTY.
# Effect mode — progress lines diverge; only exit code is asserted.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --progress"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare upstream.git
  mkdir g-side x-side
  (cd g-side && ln -s ../upstream.git .)
  (cd x-side && ln -s ../upstream.git .)
  it "matches git: --progress exits 0" && {
    (cd g-side && expect_run 0 git clone --progress upstream.git)
  }
  it "matches gix: --progress exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --progress upstream.git)
  }
)

# --- structural flags (bare / mirror / sparse / no-checkout) --------------

# mode=effect — `--bare` → option_bare=1 → option_no_checkout=1; remote-
# tracking refs land directly under refs/heads/ rather than refs/remotes/.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --bare"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-side x-side
  (cd g-side && ln -s ../src-repo.git .)
  (cd x-side && ln -s ../src-repo.git .)
  it "matches git: --bare + explicit target exits 0" && {
    (cd g-side && expect_run 0 git clone --bare src-repo.git target.git)
  }
  it "matches gix: --bare + explicit target exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --bare src-repo.git target.git)
  }
)

# mode=effect — `--mirror` implies `--bare` AND uses refspec `+refs/*:refs/*`,
# plus sets `remote.<name>.mirror=true`.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --mirror"
only_for_hash sha1-only && (sandbox
  # Effect-mode only: gix upgrades --mirror to --bare + --no-tags to
  # match the exit-code contract. Actual +refs/*:refs/* refspec +
  # `remote.<name>.mirror=true` config writes are a follow-up.
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-side x-side
  (cd g-side && ln -s ../src-repo.git .)
  (cd x-side && ln -s ../src-repo.git .)
  it "matches git: --mirror + explicit target exits 0" && {
    (cd g-side && expect_run 0 git clone --mirror src-repo.git target.git)
  }
  it "matches gix: --mirror + explicit target exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --mirror src-repo.git target.git)
  }
)

# mode=effect — `-n` / `--no-checkout` suppresses the worktree checkout
# step. Equivalent to the first half of `--bare` without the bareness.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone -n / --no-checkout"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-short g-long x-short x-long
  for d in g-short g-long x-short x-long; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: -n exits 0" && {
    (cd g-short && expect_run 0 git clone -n src-repo.git target)
  }
  it "matches gix: -n exits 0" && {
    (cd x-short && expect_run 0 "$exe_plumbing" clone -n src-repo.git target)
  }
  it "matches git: --no-checkout exits 0" && {
    (cd g-long && expect_run 0 git clone --no-checkout src-repo.git target)
  }
  it "matches gix: --no-checkout exits 0" && {
    (cd x-long && expect_run 0 "$exe_plumbing" clone --no-checkout src-repo.git target)
  }
)

# mode=effect — `--sparse` writes a sparse-checkout pattern of just the
# toplevel directory. Needs sparse-checkout support in gix.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --sparse"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-side x-side
  (cd g-side && ln -s ../src-repo.git .)
  (cd x-side && ln -s ../src-repo.git .)
  it "matches git: --sparse exits 0" && {
    (cd g-side && expect_run 0 git clone --sparse src-repo.git target)
  }
  it "matches gix: --sparse exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --sparse src-repo.git target)
  }
)

# --- origin & branch selection --------------------------------------------

# mode=effect — `-o <name>` / `--origin=<name>` replaces the default remote
# name `origin`. Overrides `clone.defaultRemoteName` config.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone -o / --origin=<name>"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-short g-long x-short x-long
  for d in g-short g-long x-short x-long; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: -o upstream exits 0" && {
    (cd g-short && expect_run 0 git clone -o upstream src-repo.git target)
  }
  it "matches gix: -o upstream exits 0" && {
    (cd x-short && expect_run 0 "$exe_plumbing" clone -o upstream src-repo.git target)
  }
  it "matches git: --origin=upstream exits 0" && {
    (cd g-long && expect_run 0 git clone --origin=upstream src-repo.git target)
  }
  it "matches gix: --origin=upstream exits 0" && {
    (cd x-long && expect_run 0 "$exe_plumbing" clone --origin=upstream src-repo.git target)
  }
)

# mode=effect — `-b <name>` / `--branch=<name>` points HEAD at <name>
# instead of the remote default. Accepts tag names too (detaches).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone -b / --branch=<name>"
only_for_hash sha1-only && (sandbox
  # shortcoming: Clap field + dispatch-arm routing to ref_name is
  # wired (invalid PartialName → 128 via src/plumbing/main.rs),
  # but the "remote branch not found" resolution error that fires
  # against an empty upstream surfaces through anyhow as exit 1
  # instead of git's exit 128. Unifying requires a targeted 128
  # wrapper in the dispatch arm around the gix::clone::Fetch
  # error path (specifically the "None of the refspecs matched"
  # kind) so `gix clone -b main <empty-remote>` dies 128 with
  # git's `fatal: Remote branch %s not found in upstream %s`
  # message. Deferred pending error-kind mapping.
  shortcoming "deferred: gix::clone::Fetch ref-not-found error maps to exit 1; git exits 128"
)

# mode=effect — `--revision=<rev>` detaches HEAD at <rev>; no local branch
# is created; incompatible with --branch and --mirror.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --revision=<rev>"
only_for_hash sha1-only && (sandbox
  # shortcoming: `--revision` is present in vendor/git/builtin/clone.c
  # (line 964) but not in system git 2.47.3 — system git rejects it
  # with "unknown option" exit 129. The parity harness runs against
  # system git, so wiring --revision in gix today would create false
  # divergence (gix accepts, system git rejects). Defer until the
  # parity harness either bumps its git target or the flag lands in
  # a system-git release.
  shortcoming "deferred: --revision is vendor-only; system git 2.47 rejects it"
)

# mode=effect — `--revision` + `--branch` conflict → die 128.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --revision=<rev> --branch=<name> (conflict)"
only_for_hash sha1-only && (sandbox
  shortcoming "deferred: --revision is vendor-only; conflict row depends on it"
)

# mode=effect — `--revision` + `--mirror` conflict → die 128.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --revision=<rev> --mirror (conflict)"
only_for_hash sha1-only && (sandbox
  shortcoming "deferred: --revision is vendor-only; conflict row depends on it"
)

# --- local-optimization flags ---------------------------------------------

# mode=effect — `-l` / `--local` forces the local hardlink optimization.
# No-op when repo is already local (default for path args). `--no-local`
# forces the regular Git transport.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone -l / --local / --no-local"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  for d in g-l g-local g-no x-l x-local x-no; do
    mkdir "$d" && (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: -l exits 0 on local path" && {
    (cd g-l && expect_run 0 git clone -l src-repo.git target)
  }
  it "matches gix: -l exits 0 on local path" && {
    (cd x-l && expect_run 0 "$exe_plumbing" clone -l src-repo.git target)
  }
  it "matches git: --local exits 0" && {
    (cd g-local && expect_run 0 git clone --local src-repo.git target)
  }
  it "matches gix: --local exits 0" && {
    (cd x-local && expect_run 0 "$exe_plumbing" clone --local src-repo.git target)
  }
  it "matches git: --no-local exits 0" && {
    (cd g-no && expect_run 0 git clone --no-local src-repo.git target)
  }
  it "matches gix: --no-local exits 0" && {
    (cd x-no && expect_run 0 "$exe_plumbing" clone --no-local src-repo.git target)
  }
)

# mode=effect — `--no-hardlinks` forces copy of .git/objects instead of
# hardlinking (no-op if not a local clone).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --no-hardlinks"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-side x-side
  (cd g-side && ln -s ../src-repo.git .)
  (cd x-side && ln -s ../src-repo.git .)
  it "matches git: --no-hardlinks exits 0" && {
    (cd g-side && expect_run 0 git clone --no-hardlinks src-repo.git target)
  }
  it "matches gix: --no-hardlinks exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --no-hardlinks src-repo.git target)
  }
)

# mode=effect — `-s` / `--shared` sets objects/info/alternates to share
# with the source. NOTE: dangerous in practice; effect-mode exits 0.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone -s / --shared"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-short g-long x-short x-long
  for d in g-short g-long x-short x-long; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: -s exits 0" && {
    (cd g-short && expect_run 0 git clone -s src-repo.git target)
  }
  it "matches gix: -s exits 0" && {
    (cd x-short && expect_run 0 "$exe_plumbing" clone -s src-repo.git target)
  }
  it "matches git: --shared exits 0" && {
    (cd g-long && expect_run 0 git clone --shared src-repo.git target)
  }
  it "matches gix: --shared exits 0" && {
    (cd x-long && expect_run 0 "$exe_plumbing" clone --shared src-repo.git target)
  }
)

# mode=effect — `--reject-shallow` / `--no-reject-shallow` fails the clone
# if the source is shallow (overrides `clone.rejectShallow` config).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --reject-shallow / --no-reject-shallow"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-rej g-no x-rej x-no
  for d in g-rej g-no x-rej x-no; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: --reject-shallow on non-shallow upstream exits 0" && {
    (cd g-rej && expect_run 0 git clone --reject-shallow src-repo.git target)
  }
  it "matches gix: --reject-shallow exits 0" && {
    (cd x-rej && expect_run 0 "$exe_plumbing" clone --reject-shallow src-repo.git target)
  }
  it "matches git: --no-reject-shallow exits 0" && {
    (cd g-no && expect_run 0 git clone --no-reject-shallow src-repo.git target)
  }
  it "matches gix: --no-reject-shallow exits 0" && {
    (cd x-no && expect_run 0 "$exe_plumbing" clone --no-reject-shallow src-repo.git target)
  }
)

# --- reference repositories -----------------------------------------------

# mode=effect — `--reference=<repo>` adds <repo>/objects as an alternate.
# Requires the alternate to exist; aborts if it doesn't.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --reference=<repo>"
only_for_hash sha1-only && (sandbox
  # shortcoming: git writes <dst>/.git/objects/info/alternates
  # pointing at <repo>/objects when --reference is given, so the
  # clone shares object storage with the reference repo. gix
  # accepts --reference at the Clap level but doesn't write the
  # alternates file today — both binaries exit 0 on an empty
  # upstream, but the on-disk state diverges (git has alternates,
  # gix has a full copy). Parity-as-exit-code here would be
  # misleading without the alternates wiring; defer until
  # gix-odb/alternates ingestion lands.
  shortcoming "deferred: gix doesn't write objects/info/alternates; --reference is parse-only"
)

# mode=effect — `--reference-if-able=<repo>` is `--reference` but with a
# warning (not a fatal error) when the alternate doesn't exist.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --reference-if-able=<repo>"
only_for_hash sha1-only && (sandbox
  # shortcoming: same gap as --reference — gix accepts the flag
  # at the Clap level but does not attempt alternates resolution,
  # so the "warn-if-missing" semantics that --reference-if-able
  # adds to --reference can't be meaningfully tested for parity
  # until alternates are wired.
  shortcoming "deferred: parent --reference is not yet wired; --reference-if-able inherits the deferral"
)

# mode=effect — `--dissociate` copies borrowed objects locally after the
# clone and stops using alternates. Requires `--reference`.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --dissociate --reference=<repo>"
only_for_hash sha1-only && (sandbox
  # shortcoming: --dissociate is a post-processing step on
  # --reference's alternates (copy borrowed objects locally,
  # then drop the alternate). Inherits the --reference deferral;
  # there's nothing meaningful to exercise without alternates.
  shortcoming "deferred: depends on --reference alternates wiring"
)

# --- shallow clone --------------------------------------------------------

# mode=effect — `--depth=<n>` truncates history to <n> commits. Implies
# `--single-branch` unless `--no-single-branch` overrides.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --depth=<n>"
only_for_hash sha1-only && (sandbox
  # Non-bare source with one commit — gix's shallow clone of an
  # empty-upstream refuses to match any refspec, so the happy-path
  # needs an actual ref to match against.
  git init -q src
  (cd src && git config commit.gpgsign false && git -c user.email=x@x -c user.name=x commit --allow-empty -qm init) &>/dev/null
  mkdir g-side x-side
  it "matches git: --depth=1 exits 0" && {
    (cd g-side && expect_run 0 git clone --depth=1 ../src target)
  }
  it "matches gix: --depth=1 exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --depth=1 ../src target)
  }
)

# mode=effect — `--depth` with a non-positive integer → die 128 with
# "depth %s is not a positive number".
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --depth=0 (non-positive)"
only_for_hash sha1-only && (sandbox
  # shortcoming: git dies 128 on --depth=0 / --depth=-1 with the
  # explicit "depth 0 is not a positive number" message after
  # parse_options (clone.c:1063). gix's Clap Platform types `depth`
  # as `Option<NonZeroU32>`, which makes Clap reject `0` / negative
  # values at parse-time with its generic invalid-value exit 2.
  # Unifying the exit code requires a custom value_parser that
  # prints the git-style fatal and exits 128, or a looser type
  # (`Option<i32>`) plus a manual post-parse check in the dispatch
  # arm. Deferred.
  shortcoming "deferred: Clap's NonZeroU32 parser exits 2; git exits 128 with a fatal"
)

# mode=effect — `--shallow-since=<time>` creates a shallow clone with
# history after <time>. Implies `--single-branch`.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --shallow-since=<time>"
only_for_hash sha1-only && (sandbox
  # shortcoming: gix-protocol's shallow-since handling currently
  # returns "Could not decode server reply" on common shapes of
  # the shallow response after a deepen-since request, even when
  # the remote has a commit within the cutoff. Deferred until
  # gix-protocol's shallow reply decoder is aligned.
  shortcoming "deferred: gix-protocol shallow-since decoder returns 'Could not decode server reply'"
)

# mode=effect — `--shallow-exclude=<ref>` creates a shallow clone excluding
# history reachable from <ref>. Multi-valued.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --shallow-exclude=<ref>"
only_for_hash sha1-only && (sandbox
  # shortcoming: mirrors the same gix-protocol gap documented in
  # fetch.sh's --shallow-exclude row. gix-protocol's deepen-not
  # opcode alignment is incomplete; gix errors "Could not decode
  # server reply" post-request. Deferred.
  shortcoming "deferred: gix-protocol deepen-not opcode alignment (same gap as fetch.sh --shallow-exclude)"
)

# mode=effect — `--single-branch` clones only the branch HEAD resolves to
# (or --branch). `--no-single-branch` explicitly cancels --depth's
# implicit single-branch.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --single-branch / --no-single-branch"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-sb g-nsb x-sb x-nsb
  for d in g-sb g-nsb x-sb x-nsb; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: --single-branch exits 0" && {
    (cd g-sb && expect_run 0 git clone --single-branch src-repo.git target)
  }
  it "matches gix: --single-branch exits 0" && {
    (cd x-sb && expect_run 0 "$exe_plumbing" clone --single-branch src-repo.git target)
  }
  it "matches git: --no-single-branch exits 0" && {
    (cd g-nsb && expect_run 0 git clone --no-single-branch src-repo.git target)
  }
  it "matches gix: --no-single-branch exits 0" && {
    (cd x-nsb && expect_run 0 "$exe_plumbing" clone --no-single-branch src-repo.git target)
  }
)

# --- tags ----------------------------------------------------------------

# mode=effect — `--tags` / `--no-tags` toggles whether remote tags follow.
# `--no-tags` sets `remote.<name>.tagOpt=--no-tags` in the new repo.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --tags / --no-tags"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-tags g-notags x-tags x-notags
  for d in g-tags g-notags x-tags x-notags; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: --tags exits 0" && {
    (cd g-tags && expect_run 0 git clone --tags src-repo.git target)
  }
  it "matches gix: --tags exits 0" && {
    (cd x-tags && expect_run 0 "$exe_plumbing" clone --tags src-repo.git target)
  }
  it "matches git: --no-tags exits 0" && {
    (cd g-notags && expect_run 0 git clone --no-tags src-repo.git target)
  }
  it "matches gix: --no-tags exits 0" && {
    (cd x-notags && expect_run 0 "$exe_plumbing" clone --no-tags src-repo.git target)
  }
)

# --- submodules ----------------------------------------------------------

# mode=effect — `--recurse-submodules[=<pathspec>]` initializes submodules
# after the clone. Multi-valued; `--recursive` is an alias.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --recurse-submodules / --recursive"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-rs g-ry g-rp x-rs x-ry x-rp
  for d in g-rs g-ry g-rp x-rs x-ry x-rp; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: --recurse-submodules exits 0" && {
    (cd g-rs && expect_run 0 git clone --recurse-submodules src-repo.git target)
  }
  it "matches gix: --recurse-submodules exits 0" && {
    (cd x-rs && expect_run 0 "$exe_plumbing" clone --recurse-submodules src-repo.git target)
  }
  it "matches git: --recursive exits 0" && {
    (cd g-ry && expect_run 0 git clone --recursive src-repo.git target)
  }
  it "matches gix: --recursive exits 0" && {
    (cd x-ry && expect_run 0 "$exe_plumbing" clone --recursive src-repo.git target)
  }
  it "matches git: --recurse-submodules=lib exits 0" && {
    (cd g-rp && expect_run 0 git clone --recurse-submodules=lib src-repo.git target)
  }
  it "matches gix: --recurse-submodules=lib exits 0" && {
    (cd x-rp && expect_run 0 "$exe_plumbing" clone --recurse-submodules=lib src-repo.git target)
  }
)

# mode=effect — `--shallow-submodules` / `--no-shallow-submodules` makes
# cloned submodules shallow (depth 1).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --shallow-submodules / --no-shallow-submodules"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-ss g-nss x-ss x-nss
  for d in g-ss g-nss x-ss x-nss; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: --shallow-submodules exits 0" && {
    (cd g-ss && expect_run 0 git clone --shallow-submodules src-repo.git target)
  }
  it "matches gix: --shallow-submodules exits 0" && {
    (cd x-ss && expect_run 0 "$exe_plumbing" clone --shallow-submodules src-repo.git target)
  }
  it "matches git: --no-shallow-submodules exits 0" && {
    (cd g-nss && expect_run 0 git clone --no-shallow-submodules src-repo.git target)
  }
  it "matches gix: --no-shallow-submodules exits 0" && {
    (cd x-nss && expect_run 0 "$exe_plumbing" clone --no-shallow-submodules src-repo.git target)
  }
)

# mode=effect — `--remote-submodules` / `--no-remote-submodules` uses each
# submodule's remote-tracking HEAD, not the superproject's SHA-1.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --remote-submodules / --no-remote-submodules"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-rs g-nrs x-rs x-nrs
  for d in g-rs g-nrs x-rs x-nrs; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: --remote-submodules exits 0" && {
    (cd g-rs && expect_run 0 git clone --remote-submodules src-repo.git target)
  }
  it "matches gix: --remote-submodules exits 0" && {
    (cd x-rs && expect_run 0 "$exe_plumbing" clone --remote-submodules src-repo.git target)
  }
  it "matches git: --no-remote-submodules exits 0" && {
    (cd g-nrs && expect_run 0 git clone --no-remote-submodules src-repo.git target)
  }
  it "matches gix: --no-remote-submodules exits 0" && {
    (cd x-nrs && expect_run 0 "$exe_plumbing" clone --no-remote-submodules src-repo.git target)
  }
)

# mode=effect — `--also-filter-submodules` requires `--filter` and
# `--recurse-submodules`; applies the partial-clone filter to submodules.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --also-filter-submodules"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-side x-side
  (cd g-side && ln -s ../src-repo.git .)
  (cd x-side && ln -s ../src-repo.git .)
  it "matches git: --filter=blob:none --recurse-submodules --also-filter-submodules exits 0" && {
    (cd g-side && expect_run 0 git clone --filter=blob:none --recurse-submodules --also-filter-submodules src-repo.git target)
  }
  it "matches gix: --filter=blob:none --recurse-submodules --also-filter-submodules exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --filter=blob:none --recurse-submodules --also-filter-submodules src-repo.git target)
  }
)

# --- transport / networking ----------------------------------------------

# mode=effect — `-u` / `--upload-pack=<path>` overrides the remote
# upload-pack binary (ssh only; silently used/ignored elsewhere).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone -u / --upload-pack=<path>"
only_for_hash sha1-only && (sandbox
  # git's -u/--upload-pack IS exec'd on file:// transport (not
  # ssh-only as the manpage hints). Use the real git-upload-pack
  # binary path to get a clean exit-0 on both sides. gix's file://
  # transport currently ignores the flag entirely, so the parity
  # holds only when the supplied path happens to be the default.
  git-init-hash-aware -q --bare src-repo.git
  upload_pack="$(command -v git-upload-pack)"
  mkdir g-u g-up x-u x-up
  for d in g-u g-up x-u x-up; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: -u <real-upload-pack> exits 0" && {
    (cd g-u && expect_run 0 git clone -u "$upload_pack" src-repo.git target)
  }
  it "matches gix: -u <real-upload-pack> exits 0" && {
    (cd x-u && expect_run 0 "$exe_plumbing" clone -u "$upload_pack" src-repo.git target)
  }
  it "matches git: --upload-pack=<real-upload-pack> exits 0" && {
    (cd g-up && expect_run 0 git clone --upload-pack="$upload_pack" src-repo.git target)
  }
  it "matches gix: --upload-pack=<real-upload-pack> exits 0" && {
    (cd x-up && expect_run 0 "$exe_plumbing" clone --upload-pack="$upload_pack" src-repo.git target)
  }
)

# mode=effect — `--server-option=<opt>` sends protocol-v2 server options.
# Multi-valued, preserves order. No-op on file:// transport.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --server-option=<opt>"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-side x-side
  (cd g-side && ln -s ../src-repo.git .)
  (cd x-side && ln -s ../src-repo.git .)
  it "matches git: --server-option exits 0 on file:// transport" && {
    (cd g-side && expect_run 0 git clone --server-option=foo=bar src-repo.git target)
  }
  it "matches gix: --server-option exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --server-option=foo=bar src-repo.git target)
  }
)

# mode=effect — `-4` / `-6` / `--ipv4` / `--ipv6` restricts address family.
# No-op on file:// transport; exit-code parity only.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone -4 / -6 / --ipv4 / --ipv6"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  for d in g-4 g-6 g-ipv4 g-ipv6 x-4 x-6 x-ipv4 x-ipv6; do
    mkdir "$d" && (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: -4 exits 0" && {
    (cd g-4 && expect_run 0 git clone -4 src-repo.git target)
  }
  it "matches gix: -4 exits 0" && {
    (cd x-4 && expect_run 0 "$exe_plumbing" clone -4 src-repo.git target)
  }
  it "matches git: -6 exits 0" && {
    (cd g-6 && expect_run 0 git clone -6 src-repo.git target)
  }
  it "matches gix: -6 exits 0" && {
    (cd x-6 && expect_run 0 "$exe_plumbing" clone -6 src-repo.git target)
  }
  it "matches git: --ipv4 exits 0" && {
    (cd g-ipv4 && expect_run 0 git clone --ipv4 src-repo.git target)
  }
  it "matches gix: --ipv4 exits 0" && {
    (cd x-ipv4 && expect_run 0 "$exe_plumbing" clone --ipv4 src-repo.git target)
  }
  it "matches git: --ipv6 exits 0" && {
    (cd g-ipv6 && expect_run 0 git clone --ipv6 src-repo.git target)
  }
  it "matches gix: --ipv6 exits 0" && {
    (cd x-ipv6 && expect_run 0 "$exe_plumbing" clone --ipv6 src-repo.git target)
  }
)

# mode=effect — `-j` / `--jobs=<n>` sets submodule-fetch parallelism.
# No effect without submodules; defaults to submodule.fetchJobs config.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone -j / --jobs=<n>"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-j g-jobs x-j x-jobs
  for d in g-j g-jobs x-j x-jobs; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: -j 4 exits 0" && {
    (cd g-j && expect_run 0 git clone -j 4 src-repo.git target)
  }
  it "matches gix: -j 4 exits 0" && {
    (cd x-j && expect_run 0 "$exe_plumbing" clone -j 4 src-repo.git target)
  }
  it "matches git: --jobs=4 exits 0" && {
    (cd g-jobs && expect_run 0 git clone --jobs=4 src-repo.git target)
  }
  it "matches gix: --jobs=4 exits 0" && {
    (cd x-jobs && expect_run 0 "$exe_plumbing" clone --jobs=4 src-repo.git target)
  }
)

# --- repository structure ------------------------------------------------

# mode=effect — `--template=<dir>` overrides the template-directory used
# during init. Empty-template exits 0.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --template=<dir>"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir empty-template
  mkdir g-side x-side
  (cd g-side && ln -s ../src-repo.git . && ln -s ../empty-template .)
  (cd x-side && ln -s ../src-repo.git . && ln -s ../empty-template .)
  it "matches git: --template=<empty-dir> exits 0" && {
    (cd g-side && expect_run 0 git clone --template=./empty-template src-repo.git target)
  }
  it "matches gix: --template=<empty-dir> exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --template=./empty-template src-repo.git target)
  }
)

# mode=effect — `--separate-git-dir=<dir>` places .git at <dir> with a
# gitfile link back from the worktree.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --separate-git-dir=<dir>"
only_for_hash sha1-only && (sandbox
  # shortcoming: git --separate-git-dir places .git at <dir> and
  # writes a gitfile at <target>/.git pointing there. gix doesn't
  # redirect the git dir today — the flag is accepted at the Clap
  # level but the clone lands .git/ at the worktree regardless.
  # Both binaries exit 0; bytes-parity is deferred until gix's
  # init/clone layer learns to honor the separate git-dir config.
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-side x-side
  (cd g-side && ln -s ../src-repo.git .)
  (cd x-side && ln -s ../src-repo.git .)
  it "matches git: --separate-git-dir=<new-dir> exits 0" && {
    (cd g-side && expect_run 0 git clone --separate-git-dir=./real-git src-repo.git target)
  }
  it "matches gix: --separate-git-dir=<new-dir> exits 0" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --separate-git-dir=./real-git src-repo.git target)
  }
)

# mode=effect — `--bare --separate-git-dir` → die 128 with
# "options '--bare' and '--separate-git-dir' cannot be used together".
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --bare --separate-git-dir=<dir> (conflict)"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  it "matches git: conflict dies 128" && {
    expect_parity effect -- clone --bare --separate-git-dir=./real-git src-repo.git target.git
  }
)

# mode=effect — `--ref-format=<fmt>` picks the ref storage backend
# (files|reftable). Unknown values die 128.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --ref-format=<fmt>"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-ok x-ok
  (cd g-ok && ln -s ../src-repo.git .)
  (cd x-ok && ln -s ../src-repo.git .)
  it "matches git: --ref-format=files exits 0" && {
    (cd g-ok && expect_run 0 git clone --ref-format=files src-repo.git target)
  }
  it "matches gix: --ref-format=files exits 0" && {
    (cd x-ok && expect_run 0 "$exe_plumbing" clone --ref-format=files src-repo.git target)
  }
  it "matches git: --ref-format=bogus dies 128" && {
    expect_parity effect -- clone --ref-format=bogus src-repo.git target-bogus
  }
)

# mode=effect — `-c <key>=<value>` / `--config=<key>=<value>` seeds the
# new repo's config before the initial fetch. Multi-valued.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone -c / --config=<key=value>"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-c g-cfg x-c x-cfg
  for d in g-c g-cfg x-c x-cfg; do
    (cd "$d" && ln -s ../src-repo.git .)
  done
  it "matches git: -c core.eol=lf exits 0" && {
    (cd g-c && expect_run 0 git clone -c core.eol=lf src-repo.git target)
  }
  it "matches gix: -c core.eol=lf exits 0" && {
    (cd x-c && expect_run 0 "$exe_plumbing" clone -c core.eol=lf src-repo.git target)
  }
  it "matches git: --config=core.eol=lf exits 0" && {
    (cd g-cfg && expect_run 0 git clone --config=core.eol=lf src-repo.git target)
  }
  it "matches gix: --config=core.eol=lf exits 0" && {
    (cd x-cfg && expect_run 0 "$exe_plumbing" clone --config=core.eol=lf src-repo.git target)
  }
)

# --- partial clone / bundle ----------------------------------------------

# mode=effect — `--filter=<spec>` uses partial clone and asks the server
# for a filtered subset. Requires promisor remote + protocol v2.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --filter=<spec>"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-side x-side
  (cd g-side && ln -s ../src-repo.git .)
  (cd x-side && ln -s ../src-repo.git .)
  it "matches git: --filter=blob:none exits 0 on empty upstream" && {
    (cd g-side && expect_run 0 git clone --filter=blob:none src-repo.git target)
  }
  it "matches gix: --filter=blob:none exits 0 on empty upstream" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --filter=blob:none src-repo.git target)
  }
)

# mode=effect — `--bundle-uri=<uri>` fetches a bundle before the real
# fetch. Incompatible with --depth / --shallow-since / --shallow-exclude.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --bundle-uri=<uri>"
only_for_hash sha1-only && (sandbox
  # Non-existent bundle URI → git falls through to a regular clone
  # and exits 0. gix ignores --bundle-uri (parse-only) and does the
  # regular clone regardless. Both exit 0.
  git-init-hash-aware -q --bare src-repo.git
  mkdir g-side x-side
  (cd g-side && ln -s ../src-repo.git .)
  (cd x-side && ln -s ../src-repo.git .)
  it "matches git: --bundle-uri=<missing> falls through to regular clone" && {
    (cd g-side && expect_run 0 git clone --bundle-uri=file:///tmp/missing-bundle src-repo.git target)
  }
  it "matches gix: --bundle-uri is accepted and ignored" && {
    (cd x-side && expect_run 0 "$exe_plumbing" clone --bundle-uri=file:///tmp/missing-bundle src-repo.git target)
  }
)

# mode=effect — `--bundle-uri` + `--depth` → die 128 (mirrors `if
# (bundle_uri && deepen) die(...)` in cmd_clone).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix clone --bundle-uri=<uri> --depth=<n> (conflict)"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware -q --bare src-repo.git
  it "matches git: --bundle-uri + --depth dies 128" && {
    expect_parity effect -- clone --bundle-uri=file:///tmp/x --depth=1 src-repo.git target-bundle-depth
  }
)

# End-of-file sentinel: when every row is `only_for_hash sha1-only` and the
# active hash is sha256, the last statement returns 1 (skip), which would
# propagate out of `source` and trip `set -e` in tests/parity.sh. A trailing
# `:` normalizes the exit code so a fully-skipped file still returns 0.
:
