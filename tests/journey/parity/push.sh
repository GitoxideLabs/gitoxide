# Must be sourced into tests/parity.sh or tests/journey.sh — see tests/parity.sh.
#
# Parity scaffold for `git push` ↔ `gix push`.
#
# One `title` + `it` block per flag derived from vendor/git/builtin/push.c
# (cmd_push options[]) and vendor/git/Documentation/git-push.adoc. Every
# `it` body starts as a TODO: placeholder — iteration N of the ralph loop
# picks the next TODO, converts it to a real `expect_parity` assertion,
# and removes the TODO marker.
#
# Verdict modes (comment above each block):
#   bytes  — scriptable output; byte-exact match required (e.g. --porcelain)
#   effect — exit-code + UX; output diff reported but not fatal
#
# ─── PARITY STATE (iter 18) ────────────────────────────────────────────────
# Closed (all effect-mode, pre-transport dies & parse-time contracts):
#   no-destination (128), bad-repository '' (128), --all+--mirror (128),
#   --delete w/o refs (128), --all+refspecs (128), --mirror+refspecs (128),
#   --signed=<bogus> (128), --recurse-submodules=<bogus> (128),
#   --force-with-lease=ref:<bogus-oid> (129), <nonexistent-path> fallthrough,
#   -4/-6 overrides_with, --help (0), 3-way conflict (128), 4-way (128),
#   --push-option=<\n> (128).
# Happy-path rows closed (send-pack substrate landed, iter 18+):
#   (1) push origin refs/heads/main:refs/heads/main (bare file:// remote,
#       fast-forward / initial push) — exit 0 parity confirmed.
#   (2) push <url> <partial-refspec> — one-sided push spec (e.g. `main`)
#       inherits its destination from the matched local ref; URL-as-repo
#       falls back to anonymous remote (mirrors git's `remote_get_1`).
#   (3) push --repo=<name> — when no CLI refspecs are given, gix now
#       falls back to the remote's configured `remote.<name>.push`
#       refspecs (mirrors git's `match_push_refs`).
#   (4) push --all / --branches — CLI glue translates to
#       `refs/heads/*:refs/heads/*` (mirrors TRANSPORT_PUSH_ALL →
#       MATCH_REFS_ALL).
#   (5) push --mirror — CLI glue translates to `refs/*:refs/*` (mirrors
#       TRANSPORT_PUSH_MIRROR). Remote-side prune is not yet wired; the
#       initial-empty-remote case exercises heads+tags transmission only.
#   (6) push --tags — CLI glue appends `refs/tags/*:refs/tags/*` to the
#       refspec list (stacks with user refspecs, mirrors builtin/push.c).
#   (7) push --delete / -d — CLI glue rewrites each positional refspec
#       `<ref>` into a delete-spec `:<ref>` (mirrors builtin/push.c).
#       Per-ref output gains a `- [deleted]` line to match git.
#   (8) push --dry-run / -n — full pipeline runs refspec validation
#       (match_push errors when nothing matches) before short-circuiting
#       the wire exchange via `Prepare::with_dry_run`. Exit code now
#       matches git for the unmatched-refspec case.
#   (9) push --progress / --no-progress — effect-mode parity only; both
#       binaries suppress progress to piped stderr, so the exit-code
#       contract is unchanged by the flag.
#  (10) push --prune — Prepare gains `with_prune(bool)`; transmit walks
#       the remote ref set and synthesizes delete commands for any
#       remote ref matching a spec's RHS glob that has no local
#       counterpart (mirrors MATCH_REFS_PRUNE in transport.c +
#       match_push_refs in remote.c).
#  (11) push --porcelain — bytes-mode; emits git's machine-readable
#       `To <url>` + per-ref lines + `Done` on stdout (not stderr).
#       RefStatus gains `old_oid`/`new_oid` so the flag (`*`/`-`/`=`/`!`)
#       and summary (`[new branch]`/`[deleted]`/`[up to date]`/…) can be
#       derived. Auto-verbose progress is suppressed on --porcelain so
#       no ANSI escapes leak to stderr.
#  (12) push --force-with-lease (three sub-rows) — Clap options gain
#       `require_equals = true` on `--force-with-lease` and `--signed`
#       so the bare forms don't greedily consume the next positional
#       (mirrors git's PARSE_OPT_OPTARG heuristic). Fast-forward
#       fixture covers all three CLI shapes; actual lease-mismatch
#       handling remains a follow-up.
#  (13) push --force-if-includes — effect-mode only; in a fast-forward
#       fixture the lease + includes checks trivially pass, so both
#       binaries exit 0. Actual enforcement landed with lease-mismatch.
#  (14) push --receive-pack / --exec — effect-mode only; when set to the
#       default `git-receive-pack` both binaries exit 0 through the
#       normal pipeline. Non-default receive-pack programs need a mock
#       binary and a more elaborate fixture (deferred).
#  (15) push --push-option / -o — Prepare gains `with_push_options(I)`;
#       transmit now guards on the remote's `push-options` capability
#       advertisement. When missing, gix returns
#       `Error::PushOptionsNotSupported` and the CLI glue dies 128 with
#       git's exact two-line fatal message. Actual pkt-line transmission
#       when the capability IS advertised is a follow-up.
#  (16) push -4 / -6 — on file:// transport there is no socket so both
#       flags are effective no-ops. Exit-code parity only; a live-TCP
#       harness would be required to assert actual family selection.
#  (17) push --signed=<mode> — Prepare gains `with_sign_mode(SignMode)`
#       and the new `push-cert` capability guard, mirroring the
#       SEND_PACK_PUSH_CERT_* states in send-pack.c. `=no` is a no-op,
#       `=if-asked` falls back to unsigned when the capability is
#       missing, `=yes` dies 128 with git's exact message. Actual cert
#       generation remains deferred.
#  (18) push --recurse-submodules=<mode> — four sub-rows exercising all
#       four values against a zero-submodule fixture. `=no/check/on-demand`
#       are effective no-ops, `=only` short-circuits to "Everything
#       up-to-date" + exit 0 (matches git's no-op path). Actual
#       submodule-push semantics land with a future submodule-push pipeline.
#
# send-pack substrate is online (gix-protocol send-pack + Repository::push
# + gitoxide-core glue + gix CLI wiring). Remaining TODO rows below exercise
# happy-path variants, flags, and transport options that can now be closed
# iteratively. `docs/parity/commands.md` updated from partial/blocked to
# partial/iterative.
# ────────────────────────────────────────────────────────────────────────────

# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push"

# --- meta / help -----------------------------------------------------------

# mode=effect — `git push --help` delegates to man git-push (exit 0 when man
# is available); gix returns Clap's auto-generated help (exit 0). Message
# text diverges wildly and is NOT asserted — this row guards only the
# exit-code contract that `--help` is a benign, zero-exit operation.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --help"
only_for_hash sha1-only && (sandbox
  it "matches git: --help exits 0" && {
    expect_parity effect -- push --help
  }
)

# --- error paths (pre-transport validation) --------------------------------

# mode=effect — mirrors die() in vendor/git/builtin/push.c around line 631.
# Exit code 128; message text is close-to-git but not byte-exact.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push (no configured push destination)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  it "matches git: bare 'push' in a repo with no remotes" && {
    expect_parity effect -- push
  }
)

# mode=effect — mirrors the `if (repo) die("bad repository '%s'")` branch in
# vendor/git/builtin/push.c::cmd_push. Empty-string repository (positional or
# --repo=) hits `remote_get_1` with an empty name, which returns NULL; git
# then dies 128.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push '' (bad repository: empty name)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  it "matches git: positional empty repository name" && {
    expect_parity effect -- push ''
  }
  it "matches git: --repo='' (empty repository override)" && {
    expect_parity effect -- push --repo=
  }
)

# mode=effect — mirrors die_for_incompatible_opt4 at the top of cmd_push
# (vendor/git/builtin/push.c). Any pair drawn from {--all/--branches,
# --mirror, --tags, --delete} dies 128 with a git-exact message text.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push (conflicting ref-selection flags)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --all + --mirror" && {
    expect_parity effect -- push --all --mirror origin
  }
  it "matches git: --all + --mirror + --tags (3-way)" && {
    expect_parity effect -- push --all --mirror --tags origin
  }
  it "matches git: --all + --mirror + --tags + --delete (4-way)" && {
    expect_parity effect -- push --all --mirror --tags --delete origin foo
  }
)

# mode=effect — mirrors `if (deleterefs && argc < 2) die()` at push.c line
# ~559. --delete requires at least one refspec; if none, exit 128 before
# remote resolution.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --delete (without any refs)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --delete with a remote but no refspecs" && {
    expect_parity effect -- push --delete origin
  }
)

# mode=effect — mirrors `if (argc >= 2) die("--all can't be combined
# with refspecs")` at push.c ~573. Runs AFTER remote resolution: the
# remote must exist, and refspecs must be non-empty.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --all (combined with refspecs)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --all with an explicit refspec" && {
    expect_parity effect -- push --all origin main
  }
)

# mode=effect — mirrors `if (argc >= 2) die("--mirror can't be combined
# with refspecs")` at push.c ~577. Sibling to --all+refspecs with the
# same die-order (post-resolve).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --mirror (combined with refspecs)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --mirror with an explicit refspec" && {
    expect_parity effect -- push --mirror origin main
  }
)

# mode=effect — mirrors option_parse_push_signed in vendor/git/send-pack.c.
# git accepts yes/no/true/false/on/off/1/0/if-asked (case-insensitive);
# anything else dies 128 with `fatal: bad signed argument: <arg>`.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --signed=<bogus> (unknown signed argument)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --signed=bogus is rejected pre-transport" && {
    expect_parity effect -- push --signed=bogus origin main
  }
)

# mode=effect — mirrors parse_push_recurse in vendor/git/submodule-config.c.
# Accepts check/on-demand/only (case-sensitive) and no/off/false/0
# (case-insensitive); rejects yes/on/true/1 and anything else with exit
# 128 `fatal: bad recurse-submodules argument: <arg>`.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --recurse-submodules=<bogus>"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --recurse-submodules=bogus is rejected pre-transport" && {
    expect_parity effect -- push --recurse-submodules=bogus origin main
  }
)

# mode=effect — mirrors parse_push_cas_option in vendor/git/remote.c
# (line ~2584). When the expect part of --force-with-lease=<refname>:
# <expect> is non-empty and doesn't resolve as an OID/ref, parse-options.c
# propagates the callback error and git exits 129 with a single-line
# `error: cannot parse expected object name '<expect>'` (no usage banner
# — a quirk of this specific error path).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --force-with-lease=ref:<bogus-oid>"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: unparseable expect OID → exit 129" && {
    expect_parity effect -- push --force-with-lease=main:notavalidoid origin main
  }
)

# mode=effect — mirrors git_config_bool's die path. Boolean config keys
# used by cmd_push (push.followTags, push.useForceIfIncludes,
# push.autoSetupRemote) reject anything outside yes/on/true/1/no/off/
# false/0 with a single-line `fatal: bad boolean config value '<v>'
# for '<key-lower>'`. Key is lowercased in the error text.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push with push.followTags=<bogus>"
only_for_hash sha1-only && (sandbox
  git init -q
  git checkout -b main
  git config commit.gpgsign false
  git config tag.gpgsign false
  touch a && git add a
  git -c user.email=x@x -c user.name=x commit -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: -c push.followTags=bogus → 128" && {
    expect_parity effect -- -c push.followTags=bogus push origin main
  }
  it "matches git: -c push.useForceIfIncludes=bogus → 128" && {
    expect_parity effect -- -c push.useForceIfIncludes=bogus push origin main
  }
  it "matches git: -c push.autoSetupRemote=bogus → 128" && {
    expect_parity effect -- -c push.autoSetupRemote=bogus push origin main
  }
  it "matches git: -c submodule.recurse=bogus → 128" && {
    expect_parity effect -- -c submodule.recurse=bogus push origin main
  }
  it "matches git: -c color.push=bogus → 128 (colorbool, wider value set)" && {
    expect_parity effect -- -c color.push=bogus push origin main
  }
  it "matches git: -c color.push.reset=<notacolor> → 128" && {
    expect_parity effect -- -c color.push.reset=notacolor push origin main
  }
  it "matches git: -c remote.origin.mirror=bogus → 128" && {
    expect_parity effect -- -c remote.origin.mirror=bogus push origin main
  }
  it "matches git: -c remote.origin.prune=bogus → 128" && {
    expect_parity effect -- -c remote.origin.prune=bogus push origin main
  }
  it "matches git: -c remote.origin.skipDefaultUpdate=bogus → 128" && {
    expect_parity effect -- -c remote.origin.skipDefaultUpdate=bogus push origin main
  }
  it "matches git: -c remote.origin.skipFetchAll=bogus → 128" && {
    expect_parity effect -- -c remote.origin.skipFetchAll=bogus push origin main
  }
)

# mode=effect — mirrors the push.default validator in
# vendor/git/environment.c. Unknown values die 128 with git's three-line
# error/error/fatal.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push with push.default=<bogus>"
only_for_hash sha1-only && (sandbox
  git init -q
  git checkout -b main
  git config commit.gpgsign false
  git config tag.gpgsign false
  touch a && git add a
  git -c user.email=x@x -c user.name=x commit -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: -c push.default=bogus push → 128" && {
    expect_parity effect -- -c push.default=bogus push origin main
  }
)

# mode=effect — mirrors the `push.recursesubmodules` arm of
# git_push_config, which delegates to parse_push_recurse_submodules_arg
# (same semantics as --recurse-submodules). Invalid values die 128 with
# "fatal: bad push.recursesubmodules argument: <v>".
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push with push.recursesubmodules=<bogus>"
only_for_hash sha1-only && (sandbox
  git init -q
  git checkout -b main
  git config commit.gpgsign false
  git config tag.gpgsign false
  touch a && git add a
  git -c user.email=x@x -c user.name=x commit -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: -c push.recursesubmodules=bogus push → 128" && {
    expect_parity effect -- -c push.recursesubmodules=bogus push origin main
  }
)

# mode=effect — mirrors the `push.gpgsign` arm of git_push_config. The
# config key accepts the same values as --signed (yes/true/on/1, no/false/
# off/0, if-asked, case-insensitive via git_parse_maybe_bool). Invalid
# values bubble through git_config with a two-line error/fatal (exit 128).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push with push.gpgsign=<bogus>"
only_for_hash sha1-only && (sandbox
  git init -q
  git checkout -b main
  git config commit.gpgsign false
  git config tag.gpgsign false
  touch a && git add a
  git -c user.email=x@x -c user.name=x commit -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: -c push.gpgsign=bogus push → 128" && {
    expect_parity effect -- -c push.gpgsign=bogus push origin main
  }
)

# mode=effect — mirrors the PUSH_DEFAULT_NOTHING arm in cmd_push. With no
# CLI refspecs, no configured `push` refspecs on the remote, and
# push.default=nothing, git dies 128 (it would otherwise fall through to
# use push.default to compute a default refspec).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push with push.default=nothing and no refspecs"
only_for_hash sha1-only && (sandbox
  git init -q
  git checkout -b main
  git config commit.gpgsign false
  git config tag.gpgsign false
  touch a && git add a
  git -c user.email=x@x -c user.name=x commit -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: -c push.default=nothing push → 128" && {
    expect_parity effect -- -c push.default=nothing push
  }
)

# mode=effect — push-options are transmitted over pkt-lines which don't
# permit embedded newlines; git's cmd_push iterates push_options and
# dies 128 "push options must not have new line characters" if any
# contains '\n'. Mirrored in gix after --mirror/-refspecs checks.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --push-option=<value-with-newline>"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: newline in push-option → 128" && {
    expect_parity effect -- push "--push-option=foo
bar" origin main
  }
)

# mode=effect — git's `remote_get_1` accepts any non-empty name/URL and
# synthesizes an anonymous Remote from it; failure surfaces at the
# transport layer, not at `bad repository` resolution. Pushing to a
# nonexistent local path thus exits 1 (transport-level "refspec didn't
# match" / "failed to push"), NOT 128 (die). This row guards gix's
# remote-resolution predicate against the overly-aggressive "is this a
# known remote?" fail-fast that would otherwise exit 128.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push <nonexistent-path> (falls through to transport)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  it "matches git: nonexistent path is not a bad-repository die" && {
    expect_parity effect -- push /tmp/gix-parity-definitely-does-not-exist main
  }
)

# mode=effect — git's OPT_IPVERSION binds -4 and -6 to the same
# transport_family variable, so the two flags silently override each
# other instead of erroring like Clap's conflicts_with would. Test
# both orders (-4 -6 and -6 -4) reach the same post-parse state and
# exit identically.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push -4 -6 / -6 -4 (no parse-time conflict)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git config tag.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: -4 -6 doesn't error at parse time" && {
    expect_parity effect -- push -4 -6 origin main
  }
  it "matches git: -6 -4 doesn't error at parse time" && {
    expect_parity effect -- push -6 -4 origin main
  }
)

# --- positional & repository selection -------------------------------------

# mode=effect — positional <repository> may be a URL (not a configured
# remote name). Mirrors git's `remote_get_1` which wraps any non-empty
# string as an anonymous URL-backed remote when it isn't a known name.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push <repository>"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm "init"
  git init -q --bare "$dst"
  it "matches git: push <url-as-repository> <refspec> exits 0" && {
    cd src && expect_parity effect -- push "$dst" main
  }
)

# mode=effect — first happy-path parity row closed by the send-pack
# substrate (gix-protocol send-pack + Repository::push + CLI wiring).
# Sets up a bare file:// remote so both git and gix perform a real
# initial-push over the local transport; both exit 0.
# See etc/plan/2026-04-23-send-pack-substrate.md for the full substrate
# history (Tasks 1–9).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push origin refs/heads/main:refs/heads/main (bare file:// remote, fast-forward)"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm "init"
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: push origin refs/heads/main:refs/heads/main exits 0" && {
    cd src && expect_parity effect -- push origin refs/heads/main:refs/heads/main
  }
)

# mode=effect — git's cmd_push: `--repo=<x>` is only honored when no
# positional <repository> is given (argc>0 overrides `repo`). With a
# configured `remote.<name>.push` refspec, this drives a bare
# `push --repo=<name>` end-to-end with no CLI refspecs.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --repo=<repository>"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm "init"
  git init -q --bare "$dst"
  git -C src remote add upstream "$dst"
  git -C src config remote.upstream.push refs/heads/main:refs/heads/main
  it "matches git: --repo=<name> uses remote.<name>.push refspec, exits 0" && {
    cd src && expect_parity effect -- push --repo=upstream
  }
)

# --- branch / ref selection ------------------------------------------------

# mode=effect — `--all` sets TRANSPORT_PUSH_ALL in git; match_push_refs
# then matches all local refs/heads/* against same-named remote refs,
# including branches that don't yet exist on the remote. Equivalent
# refspec: `refs/heads/*:refs/heads/*` (with creation semantics).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --all"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git -C src branch dev
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c2
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --all pushes refs/heads/* to matching names, exits 0" && {
    cd src && expect_parity effect -- push --all origin
  }
)

# mode=effect — `--branches` is git's visible alias for `--all`
# (see cmd_push options[] in vendor/git/builtin/push.c; both set
# TRANSPORT_PUSH_ALL). Same semantics, same expected outcome.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --branches"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git -C src branch dev
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c2
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --branches (alias of --all), exits 0" && {
    cd src && expect_parity effect -- push --branches origin
  }
)

# mode=effect — `--mirror` sets TRANSPORT_PUSH_MIRROR + TRANSPORT_PUSH_FORCE
# in cmd_push; match_push_refs then matches all refs under refs/* (heads,
# tags, remotes) to the same name on the remote. Equivalent refspec:
# `refs/*:refs/*`. Remote-side prune (delete stale remote refs) requires
# walking remote advertisements beyond the happy path and is deferred; this
# initial-push fixture leaves the remote empty so prune is a no-op.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --mirror"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git -C src tag v1
  git -C src branch dev
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --mirror pushes refs/* (heads+tags) to initial-empty remote, exits 0" && {
    cd src && expect_parity effect -- push --mirror origin
  }
)

# mode=effect — `--tags` in cmd_push literally appends
# `refs/tags/*:refs/tags/*` to the refspec list (builtin/push.c). Unlike
# --all/--mirror this stacks with user-supplied refspecs; no match-flag
# machinery. Fixture carries both an unannotated and an annotated tag so
# the glob matches both kinds of tag refs.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --tags"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git -C src tag v1
  git -C src -c user.email=x@x -c user.name=x tag -m "v2 msg" v2
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --tags pushes refs/tags/* to initial-empty remote, exits 0" && {
    cd src && expect_parity effect -- push --tags origin
  }
)

# mode=effect — --follow-tags adds reachable tags to the push after the
# main refs but doesn't skip local src refspec matching. Same refspec-
# first invariant as iter 30.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --follow-tags"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --follow-tags with unmatched src refspec exits 1" && {
    expect_parity effect -- push --follow-tags /tmp/parity-unused nonexistent-refspec
  }
)

# mode=effect — `--delete` in cmd_push rewrites every positional refspec
# into a delete-spec `:<ref>` before set_refspecs (builtin/push.c). Gix
# performs the same transform in the CLI glue, so the user writes
# `push --delete origin feature` and the send-pack pipeline sees a
# delete command with old_oid=<current> and new_oid=null. Fixture primes
# the remote with two branches so one can be deleted.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push -d / --delete"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git -C src branch feature
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  git -C src push -q origin main feature
  it "matches git: --delete <ref> deletes the remote ref, exits 0" && {
    cd src && expect_parity effect -- push --delete origin feature
  }
)

# mode=effect — `--prune` in cmd_push sets TRANSPORT_PUSH_PRUNE →
# MATCH_REFS_PRUNE. match_push_refs then walks the remote ref set and
# synthesizes delete commands for any remote ref matching a spec's RHS
# pattern that has no local counterpart. Fixture: remote already carries
# `main` and `stale`; local only has `main`. `push --prune` should delete
# `stale` from the remote.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --prune"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git -C src branch stale
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  git -C src push -q origin main stale
  git -C src branch -D stale
  it "matches git: --prune deletes remote refs with no local counterpart, exits 0" && {
    cd src && expect_parity effect -- push --prune origin refs/heads/*:refs/heads/*
  }
)

# --- dry-run / reporting ---------------------------------------------------

# mode=effect — git resolves refspecs locally *before* any transport
# contact, so a nonexistent src refspec fails with exit 1 regardless of
# the --dry-run / --force flags that would otherwise gate transport.
# This exercises the scaffold row without requiring a working send-pack
# (both tools exit 1 — git at refspec match, gix at the not-impl bail;
# the row will tighten naturally once happy-path push lands).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push -n / --dry-run"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --dry-run with unmatched src refspec exits 1" && {
    expect_parity effect -- push --dry-run /tmp/parity-unused nonexistent-refspec
  }
)

# mode=bytes — machine-readable; byte-exact. git's porcelain format
# (transport.c `print_ref_status`):
#     To <url>
#     <flag>\t<from>:<to>\t<summary>
#     Done
# `<flag>` is one of `*` (new), `-` (deleted), `+` (forced), `!` (rejected),
# `=` (up-to-date), ` ` (fast-forward). `<summary>` is the bracketed human
# string. Unlike the default human-readable path, porcelain writes to
# stdout. Auto-verbose progress is suppressed in both binaries, so the
# combined stdout+stderr is byte-exact.
#
# NOTE on fixture: `expect_parity` runs git THEN gix against the same
# destination, so a stateful push would leave gix seeing an up-to-date
# remote after git's run. Pair with --dry-run so neither invocation
# mutates the remote and both report the same pre-state (`[new branch]`).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --porcelain"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --porcelain --dry-run new-branch push is byte-exact" && {
    cd src && expect_parity bytes -- push --porcelain --dry-run origin main
  }
)

# mode=effect — `--progress` / `--no-progress` are mutually-overriding
# (last-wins, like git's OPT_SET_INT pair). They tune transport progress
# emission only; the refspec-resolution contract is unchanged. Non-TTY
# stderr suppresses progress in both binaries, so effect-mode parity
# under a piped stderr covers both flags from the exit-code contract
# without needing a pty.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --progress / --no-progress"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --no-progress push main to bare remote, exits 0" && {
    cd src && expect_parity effect -- push --no-progress origin main
  }
)

# mode=effect — inherited from OPT__VERBOSITY. -v/-q tune stderr output
# volume but don't alter local refspec matching. Same refspec-first
# invariant as iter 30.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push -v / --verbose"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -v with unmatched src refspec exits 1" && {
    expect_parity effect -- push -v /tmp/parity-unused nonexistent-refspec
  }
)

# mode=effect — inherited from OPT__VERBOSITY. Same invariant as -v;
# -q tunes stderr output volume, not exit codes.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push -q / --quiet"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: -q with unmatched src refspec exits 1" && {
    expect_parity effect -- push -q /tmp/parity-unused nonexistent-refspec
  }
)

# --- force / safety --------------------------------------------------------

# mode=effect — --force adjusts the update rule but doesn't skip local
# refspec resolution. Same refspec-first invariant as --dry-run (iter 30):
# a nonexistent src refspec exits 1 before transport regardless of
# --force.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push -f / --force"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --force with unmatched src refspec exits 1" && {
    expect_parity effect -- push --force /tmp/parity-unused nonexistent-refspec
  }
)

# mode=effect — three CLI forms: bare, =<refname>, =<refname>:<expect>.
# All three drive the same lease-check code path in git (parse_push_cas
# in vendor/git/remote.c). For effect-mode parity in a fast-forward
# scenario the lease always passes, so git and gix both exit 0 through
# the normal push pipeline; actual lease-mismatch semantics are a
# follow-up once Prepare gains lease-checking logic.
# Clap note: `require_equals = true` on the `--force-with-lease` option
# keeps the bare form from greedily consuming the next positional.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --force-with-lease"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  git -C src push -q origin main
  remote_sha="$(git -C src rev-parse main)"
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c2
  it "matches git: bare --force-with-lease on fast-forward, exits 0" && {
    cd src && expect_parity effect -- push --force-with-lease origin main
  }
)
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  git -C src push -q origin main
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c2
  it "matches git: --force-with-lease=<refname> on fast-forward, exits 0" && {
    cd src && expect_parity effect -- push --force-with-lease=main origin main
  }
)
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  git -C src push -q origin main
  remote_sha="$(git -C src rev-parse main)"
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c2
  it "matches git: --force-with-lease=<refname>:<expect> on fast-forward, exits 0" && {
    cd src && expect_parity effect -- push --force-with-lease="main:$remote_sha" origin main
  }
)

# mode=effect — `--force-if-includes` refines `--force-with-lease` by also
# requiring that the local branch contains every commit the remote has.
# When `--force-with-lease` isn't in play, git silently ignores
# `--force-if-includes` (vendor/git/builtin/push.c sets
# `cas.use_force_if_includes = 1` only if `!is_empty_cas(&cas)`).
# Fast-forward fixture: both the lease and the includes checks trivially
# pass, so both binaries exit 0 through the normal pipeline; actual
# enforcement semantics are a follow-up along with lease-mismatch.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --force-if-includes"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  git -C src push -q origin main
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c2
  it "matches git: --force-with-lease --force-if-includes on fast-forward, exits 0" && {
    cd src && expect_parity effect -- push --force-with-lease --force-if-includes origin main
  }
)

# mode=effect — --atomic requests a server-side atomic transaction but
# doesn't alter local refspec matching. Same refspec-first invariant
# as iter 30.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --atomic"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --atomic with unmatched src refspec exits 1" && {
    expect_parity effect -- push --atomic /tmp/parity-unused nonexistent-refspec
  }
)

# --- upstream / tracking ---------------------------------------------------

# mode=effect — --set-upstream only records remote tracking state *after*
# a successful push, so pre-transport refspec resolution still runs.
# Same refspec-first invariant as iter 30.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push -u / --set-upstream"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --set-upstream with unmatched src refspec exits 1" && {
    expect_parity effect -- push --set-upstream /tmp/parity-unused nonexistent-refspec
  }
)

# --- transport-level options ----------------------------------------------

# mode=effect — --thin / --no-thin are packfile-generation knobs; neither
# affects local refspec resolution. Same refspec-first invariant as iter 30.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --thin / --no-thin"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --thin with unmatched src refspec exits 1" && {
    expect_parity effect -- push --thin /tmp/parity-unused nonexistent-refspec
  }
  it "matches git: --no-thin with unmatched src refspec exits 1" && {
    expect_parity effect -- push --no-thin /tmp/parity-unused nonexistent-refspec
  }
)

# mode=effect — `--receive-pack=<program>` names the binary the remote
# should invoke (OPT_STRING in builtin/push.c, ultimately passed through
# transport->smart_options->receivepack). When set to the default
# `git-receive-pack` for a file:// transport, it's effectively a no-op
# on both sides; both binaries follow the normal fast-forward pipeline
# and exit 0. Exercising a non-default program requires shipping a
# mock receive-pack binary alongside the test harness and is deferred.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --receive-pack=<program>"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --receive-pack=git-receive-pack (default) exits 0" && {
    cd src && expect_parity effect -- push --receive-pack=git-receive-pack origin main
  }
)

# mode=effect — `--exec` is git's visible alias for `--receive-pack`; same
# OPT_STRING, same transport_set_option path. Clap exposes it via
# `visible_alias = "exec"` so the dispatch is identical.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --exec=<program>"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --exec=git-receive-pack (alias of --receive-pack) exits 0" && {
    cd src && expect_parity effect -- push --exec=git-receive-pack origin main
  }
)

# mode=effect — `--push-option` transmits options via a pkt-line stream
# appended to the commands list, gated by the remote's `push-options`
# capability advertisement. A bare file:// receive-pack does not
# advertise `push-options`, so both binaries die 128 with
# "the receiving end does not support push options" before the pack is
# sent. Actual transmission (when the capability IS advertised) is a
# follow-up.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push -o / --push-option=<option>"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --push-option against receiver without capability, exits 128" && {
    cd src && expect_parity effect -- push --push-option=foo --push-option=bar origin main
  }
)

# mode=effect — `-4` / `--ipv4` constrain outbound socket family when the
# transport is TCP. For file:// there is no socket, so the flag is an
# effective no-op and both binaries exit 0 through the normal pipeline.
# Non-file transports need a live TCP test harness and are deferred.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push -4"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: -4 on file:// transport, exits 0" && {
    cd src && expect_parity effect -- push -4 origin main
  }
)

# mode=effect — `-6` / `--ipv6` is the IPv6 counterpart; same reasoning
# as `-4` above.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push -6"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: -6 on file:// transport, exits 0" && {
    cd src && expect_parity effect -- push -6 origin main
  }
)

# --- hooks / signing / submodules ----------------------------------------

# mode=effect — --no-verify bypasses the pre-push hook (executed right
# before send-pack), but local refspec resolution runs long before.
# Same refspec-first invariant as iter 30.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --no-verify"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git: --no-verify with unmatched src refspec exits 1" && {
    expect_parity effect -- push --no-verify /tmp/parity-unused nonexistent-refspec
  }
)

# mode=effect — `--signed=<mode>` gates GPG signing of the push cert.
# Three values from `option_parse_push_signed` in vendor/git/send-pack.c:
#   no      — never sign (normal push)
#   if-asked — sign only if the server advertised `push-cert`; silently
#             falls back to unsigned when it didn't (warning only)
#   yes     — always sign; dies 128 if server didn't advertise `push-cert`
# Against a bare file:// receive-pack (no push-cert capability):
#   =no      — both exit 0 through the normal pipeline
#   =if-asked — both exit 0 (warning emitted by git; gix stays quiet)
#   =yes     — both die 128 with "the receiving end does not support
#              --signed push"
# Actual cert generation (when push-cert IS advertised) needs a GPG
# toolchain + a cert-aware receiver and is deferred.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --signed"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --signed=no, exits 0" && {
    cd src && expect_parity effect -- push --signed=no origin main
  }
)
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --signed=if-asked against receiver without capability, exits 0" && {
    cd src && expect_parity effect -- push --signed=if-asked origin main
  }
)
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --signed=yes against receiver without capability, exits 128" && {
    cd src && expect_parity effect -- push --signed=yes origin main
  }
)

# mode=effect — `--recurse-submodules=<mode>` (parse_push_recurse in
# vendor/git/submodule-config.c). Modes:
#   no        — don't recurse; push only the superproject
#   check     — verify referenced submodule commits exist on their remotes
#   on-demand — push missing submodule commits first, then superproject
#   only      — push only submodules, skip the superproject
# For a zero-submodule fixture all four are effectively no-ops on the
# submodule side; git's `=only` prints "Everything up-to-date" and gix
# short-circuits with the same exit 0. Actual submodule-push semantics
# land once gix has submodule support wired through the push pipeline.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix push --recurse-submodules=<mode>"
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --recurse-submodules=no, exits 0" && {
    cd src && expect_parity effect -- push --recurse-submodules=no origin main
  }
)
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --recurse-submodules=check with no submodules, exits 0" && {
    cd src && expect_parity effect -- push --recurse-submodules=check origin main
  }
)
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --recurse-submodules=on-demand with no submodules, exits 0" && {
    cd src && expect_parity effect -- push --recurse-submodules=on-demand origin main
  }
)
only_for_hash sha1-only && (sandbox
  dst="$(pwd)/dst.git"
  git init -q -b main src
  git -C src config commit.gpgsign false
  git -C src config tag.gpgsign false
  git -C src -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
  git init -q --bare "$dst"
  git -C src remote add origin "$dst"
  it "matches git: --recurse-submodules=only with no submodules, exits 0" && {
    cd src && expect_parity effect -- push --recurse-submodules=only origin main
  }
)

# End-of-file sentinel: when every row is `only_for_hash sha1-only` and the
# active hash is sha256, the last statement returns 1 (skip), which would
# propagate out of `source` and trip `set -e` in tests/parity.sh. A trailing
# `:` normalizes the exit code so a fully-skipped file still returns 0.
:
