# Must be sourced into tests/parity.sh or tests/journey.sh — see tests/parity.sh.
#
# Parity file for `git fetch` ↔ `gix fetch`.
#
# One `title` + `it` block per flag derived from
# vendor/git/builtin/fetch.c (`builtin_fetch_options`) and
# vendor/git/Documentation/fetch-options.adoc.
#
# Verdict modes (comment above each block):
#   bytes  — scriptable output; byte-exact match required (e.g. --porcelain)
#   effect — exit-code + UX; output diff reported but not fatal
#
# ─── PARITY STATE ──────────────────────────────────────────────────────────
# Closed rows: 89 green `it` blocks across 61 sections, covering happy
# paths (zero-arg, named remote, <url> anonymous remote, <remote>
# <refspec>) plus the full error-path surface cmd_fetch validates
# pre-transport:
#   * pre-transport die-128s: --all+<repo>, --all+<refspec>,
#     --negotiate-only without --negotiation-tip, --negotiate-only with
#     non-off --recurse-submodules (three sub-rows), --porcelain with
#     non-off --recurse-submodules (two sub-rows), --deepen + --depth,
#     --depth + --unshallow, --depth=0, --deepen=-1, --unshallow on a
#     complete repository, --recurse-submodules=<bogus> (two sub-rows
#     including the parse-order check against --negotiate-only);
#   * config-layer die-128s: -c fetch.prune/pruneTags/writeCommitGraph/
#     showForcedUpdates=bogus, -c fetch.recurseSubmodules=bogus;
#   * no-remote / empty-positional silent exit-0 (two rows);
#   * parse-only flags with exit-0 parity against a bare empty upstream:
#     --verbose/-v, --quiet/-q, --progress, -4/-6/--ipv4/--ipv6,
#     --tags/-t, --no-tags/-n, --keep/-k, --update-head-ok/-u,
#     --upload-pack, --set-upstream, --update-shallow, --prefetch,
#     --append/-a, --atomic, --force/-f, --prune/-p, --prune-tags/-P,
#     --recurse-submodules[=MODE], --no-recurse-submodules,
#     --auto-maintenance/--no-auto-maintenance/--auto-gc/--no-auto-gc,
#     --write-commit-graph/--no-write-commit-graph, --show-forced-updates
#     /--no-show-forced-updates, --write-fetch-head/--no-write-fetch-head,
#     --filter=<spec>, --server-option=<opt>/-o, --negotiation-tip=<rev>,
#     --jobs=<n>/-j, --stdin, --help, --all (no-op),
#     --depth=<n>/--deepen=<n>/--shallow-since=<date> against a real
#     upstream, --unshallow (error path), --refetch, --refmap=<spec>.
#
# Known follow-ups (explicit shortcoming notes in the rows below):
#   * --shallow-exclude — gix-protocol deepen-not opcode alignment;
#   * --unshallow (happy path) — expect_parity needs per-binary fixture
#     reset for stateful ops (individual runs DO match, just not
#     back-to-back);
#   * --negotiate-only (happy path) — gix-protocol ack-only-capability
#     enforcement;
#   * --multiple — positional re-dispatch when the flag is set.
#
# Every row is `# hash=sha1-only` — `gix fetch` cannot open sha256 remotes
# yet (gix/src/clone/fetch/mod.rs:278 still `unimplemented!()`s hash-change
# reconfiguration). Once that lands, rows become `dual`.
# ──────────────────────────────────────────────────────────────────────────

# Local helper: build a bare empty upstream + a non-bare clone whose
# `origin` remote points at it, then cd into the clone. Used as the common
# happy-path fixture for rows that just exercise exit-code parity against
# an empty round-trip.
function bare-empty-upstream-with-origin() {
  git init -q --bare upstream.git
  git init -q clone
  (cd clone
    git remote add origin "$(cd .. && pwd)/upstream.git"
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  ) &>/dev/null
  cd clone
}

# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch"

# --- meta / help ----------------------------------------------------------

# mode=effect — `git fetch --help` delegates to man git-fetch (exit 0 when man
# is available); gix returns Clap's auto-generated help (exit 0). Message
# text diverges wildly and is NOT asserted — this row guards only the
# exit-code contract that `--help` is a benign, zero-exit operation.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --help"
only_for_hash sha1-only && (sandbox
  it "matches git: --help exits 0" && {
    expect_parity effect -- fetch --help
  }
)

# --- error paths (pre-transport validation) -------------------------------

# mode=effect — mirrors the `if (!remote) die("No remote repository specified")`
# path at the bottom of cmd_fetch. When no remote is configured and no
# positional repository is given, both binaries die 128.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch (no configured remote)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  it "matches git: bare 'fetch' in a repo with no remotes silently exits 0" && {
    expect_parity effect -- fetch
  }
)

# mode=effect — `remote_get("")` returns NULL, falling through to the same
# silent fetch_multiple(empty) path as no-arg fetch. Both binaries exit 0,
# no output. This is an empirical git behavior (arguably surprising), not
# a documented error path.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch '' (empty-string positional)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  it "matches git: positional '' silently exits 0" && {
    expect_parity effect -- fetch ''
  }
)

# mode=effect — mirrors `die("fetch --all does not take a repository argument")`
# in cmd_fetch after parse_options. Exit 128.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --all <repository> (conflict: repo arg)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --all with a positional repository" && {
    expect_parity effect -- fetch --all origin
  }
)

# mode=effect — mirrors `die("fetch --all does not make sense with refspecs")`
# in cmd_fetch. Exit 128. git checks for refspecs (argc > 1) *before* the
# repository-only check, so --all with repository + refspecs hits this
# message rather than the "does not take a repository argument" one.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --all <repo> <refspec> (conflict: refspecs)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --all with refspecs" && {
    expect_parity effect -- fetch --all origin main
  }
)

# mode=effect — mirrors `die("--negotiate-only needs one or more --negotiation-tip=*")`
# in cmd_fetch. Exit 128 before remote resolution.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --negotiate-only (without --negotiation-tip)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --negotiate-only without any tips" && {
    expect_parity effect -- fetch --negotiate-only origin
  }
)

# mode=effect — mirrors `die("options '--negotiate-only' and '--recurse-submodules' cannot be used together")`
# when --negotiate-only is combined with --recurse-submodules=yes/on-demand.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --negotiate-only --recurse-submodules=<mode>"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --recurse-submodules=yes → conflict dies 128" && {
    expect_parity effect -- fetch --negotiate-only --recurse-submodules=yes origin
  }
  it "matches git: --recurse-submodules=on-demand → conflict dies 128" && {
    expect_parity effect -- fetch --negotiate-only --recurse-submodules=on-demand origin
  }
  it "matches git: --recurse-submodules=no → conflict does NOT fire (falls through to --negotiation-tip check, dies 128 with different message)" && {
    expect_parity effect -- fetch --negotiate-only --recurse-submodules=no origin
  }
)

# mode=effect — mirrors `die("options '--porcelain' and '--recurse-submodules' cannot be used together")`
# for non-off recurse-submodules values.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --porcelain --recurse-submodules=<mode>"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --recurse-submodules=yes → conflict dies 128" && {
    expect_parity effect -- fetch --porcelain --recurse-submodules=yes origin
  }
  it "matches git: --recurse-submodules=on-demand → conflict dies 128" && {
    expect_parity effect -- fetch --porcelain --recurse-submodules=on-demand origin
  }
)

# mode=effect — mirrors `die("options '--deepen' and '--depth' cannot be used together")`
# in cmd_fetch post-parse validation.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --deepen N --depth M (conflict)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --deepen combined with --depth dies 128" && {
    expect_parity effect -- fetch --deepen=1 --depth=2 origin
  }
)

# mode=effect — mirrors `die("options '--depth' and '--unshallow' cannot be used together")`
# in cmd_fetch post-parse validation.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --depth N --unshallow (conflict)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --depth combined with --unshallow dies 128" && {
    expect_parity effect -- fetch --depth=1 --unshallow origin
  }
)

# mode=effect — mirrors `die("--unshallow on a complete repository does not make sense")`
# when the repo is not shallow.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --unshallow (on a complete repository)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --unshallow on non-shallow repo dies 128" && {
    expect_parity effect -- fetch --unshallow origin
  }
)

# mode=effect — mirrors `die("negative depth in --deepen is not supported")`.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --deepen=-1 (negative)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --deepen negative dies 128" && {
    expect_parity effect -- fetch --deepen=-1 origin
  }
)

# mode=effect — mirrors `die("depth %s is not a positive number")` when
# --depth is 0 or negative.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --depth=0 (non-positive)"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --depth=0 dies 128" && {
    expect_parity effect -- fetch --depth=0 origin
  }
)

# --- happy path: named-remote round trip ----------------------------------

# mode=effect — baseline happy-path. A named `origin` remote pointing at a
# bare file:// sibling, fetching all refs; exit 0 from both binaries.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch origin (named remote, bare file:// upstream)"
only_for_hash sha1-only && (sandbox
  # Construct a bare empty upstream + a non-bare clone whose `origin` points
  # at it. Fetching against the empty upstream is a trivial round-trip: both
  # binaries exit 0 with no ref updates.
  git init -q --bare upstream.git
  git init -q clone
  (cd clone
    git remote add origin "$(cd .. && pwd)/upstream.git"
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  )
  it "matches git: fetch from named origin (empty upstream, exit 0)" && {
    cd clone
    expect_parity effect -- fetch origin
  }
)

# mode=effect — the zero-arg form: no positional, remote.<branch>.remote or
# origin is chosen implicitly.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch (zero-arg, implicit origin)"
only_for_hash sha1-only && (sandbox
  git init -q --bare upstream.git
  git init -q clone
  (cd clone
    git remote add origin "$(cd .. && pwd)/upstream.git"
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  )
  it "matches git: bare 'fetch' uses implicit remote, exit 0" && {
    cd clone
    expect_parity effect -- fetch
  }
)

# mode=effect — URL-as-repository: anonymous remote (no stored config).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch <url> (anonymous URL remote)"
only_for_hash sha1-only && (sandbox
  # Anonymous URL remote: no named 'origin' in the clone. gitoxide-core's
  # fetch function now injects a default 'HEAD' refspec when the effective
  # remote has no configured fetch refspecs AND no explicit refspec on the
  # command line, mirroring cmd_fetch's implicit HEAD:FETCH_HEAD path.
  git init -q --bare upstream.git
  git clone -q upstream.git work &>/dev/null
  (cd work
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
    git push -q origin master
  )
  git init -q clone
  (cd clone
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  )
  upstream_abs="$(pwd)/upstream.git"
  cd clone
  it "matches git: fetch from URL path directly (no named remote), exit 0" && {
    expect_parity effect -- fetch "$upstream_abs"
  }
)

# mode=effect — explicit refspec narrowing.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch <remote> <refspec>"
only_for_hash sha1-only && (sandbox
  # Upstream with a real branch; clone has `origin` pointing at it.
  git init -q --bare upstream.git
  git clone -q upstream.git work &>/dev/null
  (cd work
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
    git push -q origin master
  )
  git init -q clone
  (cd clone
    git remote add origin "$(cd .. && pwd)/upstream.git"
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  )
  cd clone
  it "matches git: fetch origin master (short refspec)" && {
    expect_parity effect -- fetch origin master
  }
  it "matches git: fetch origin refs/heads/master:refs/remotes/origin/master (full refspec)" && {
    expect_parity effect -- fetch origin refs/heads/master:refs/remotes/origin/master
  }
)

# --- flags ----------------------------------------------------------------

# mode=effect — --all fetches all configured remotes.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --all"
only_for_hash sha1-only && (sandbox
  # --all in git iterates every configured remote; gix's --all flag is
  # currently parse-only and silently no-ops. Both binaries exit 0 with
  # two configured remotes pointing at empty upstreams — effect-mode
  # parity holds at the exit code, but gix does not (yet) emit the
  # 'Fetching <name>' lines git prints per remote. Actual multi-remote
  # iteration in gix is a follow-up.
  git init -q --bare upstream1.git
  git init -q --bare upstream2.git
  git init -q clone
  (cd clone
    git remote add up1 "$(cd .. && pwd)/upstream1.git"
    git remote add up2 "$(cd .. && pwd)/upstream2.git"
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  )
  cd clone
  it "matches git: --all with two configured remotes, exit 0 (no-op parity only)" && {
    expect_parity effect -- fetch --all
  }
)

# mode=effect — --append appends to .git/FETCH_HEAD rather than overwriting.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --append / -a"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --append accepted, exit 0" && {
    expect_parity effect -- fetch --append origin
  }
  it "matches git: -a accepted, exit 0" && {
    expect_parity effect -- fetch -a origin
  }
)

# mode=effect — --atomic asks the ref transaction to succeed or fail as a
# whole.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --atomic"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --atomic accepted, exit 0" && {
    expect_parity effect -- fetch --atomic origin
  }
)

# mode=effect — --depth limits history depth from the tip of each remote
# branch.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --depth=<n>"
only_for_hash sha1-only && (sandbox
  # Upstream with a real branch so --depth has something to narrow.
  git init -q --bare upstream.git
  git clone -q upstream.git work &>/dev/null
  (cd work
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
    git push -q origin master
  )
  git init -q clone
  (cd clone
    git remote add origin "$(cd .. && pwd)/upstream.git"
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  )
  cd clone
  it "matches git: --depth=1 accepted with a real refspec, exit 0" && {
    expect_parity effect -- fetch --depth=1 origin master
  }
)

# mode=effect — --deepen extends the shallow boundary by N commits.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --deepen=<n>"
only_for_hash sha1-only && (sandbox
  git init -q --bare upstream.git
  git clone -q upstream.git work &>/dev/null
  (cd work
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
    git push -q origin master
  )
  git init -q clone
  (cd clone
    git remote add origin "$(cd .. && pwd)/upstream.git"
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  )
  cd clone
  it "matches git: --deepen=1 accepted with a real refspec, exit 0" && {
    expect_parity effect -- fetch --deepen=1 origin master
  }
)

# mode=effect — --shallow-since cuts off history past a date.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --shallow-since=<date>"
only_for_hash sha1-only && (sandbox
  git init -q --bare upstream.git
  git clone -q upstream.git work &>/dev/null
  (cd work
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
    git push -q origin master
  )
  git init -q clone
  (cd clone
    git remote add origin "$(cd .. && pwd)/upstream.git"
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  )
  cd clone
  it "matches git: --shallow-since=<date> accepted with a real refspec, exit 0" && {
    expect_parity effect -- fetch --shallow-since=2020-01-01 origin master
  }
)

# mode=effect — --shallow-exclude excludes a named ref/tag from the history.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --shallow-exclude=<ref>"
only_for_hash sha1-only && (sandbox
  # Both binaries fail on an unresolvable exclude ref, but on file://
  # transport git surfaces 'ambiguous deepen-not' (exit 128) while gix
  # surfaces a decode error from the upload-pack handshake (exit 1). The
  # divergence is a protocol-layer issue: gix-protocol does not yet emit
  # the deepen-not opcode the same way git does, and the error propagation
  # path differs. Closed as a deferred follow-up; the CLI flag is
  # accepted on both sides.
  shortcoming "--shallow-exclude semantic parity needs gix-protocol deepen-not alignment"
)

# mode=effect — --unshallow promotes a shallow clone to complete.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --unshallow"
only_for_hash sha1-only && (sandbox
  # --unshallow is stateful: after git's call the repo is no longer shallow,
  # so gix's call against the same fixture dies 128 with "--unshallow on a
  # complete repository". expect_parity runs both binaries against a
  # shared fixture and has no per-binary reset hook, so the happy-path
  # test can't be expressed cleanly with the current helper. Individual
  # runs DO match (both exit 0 on an actually shallow clone); the
  # error-path row ('--unshallow on a complete repository') is closed and
  # green.
  shortcoming "happy-path parity needs expect_parity to reset fixtures between the git and gix invocations for stateful ops"
)

# mode=effect — --update-shallow accepts refs that require updating
# .git/shallow.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --update-shallow"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --update-shallow accepted, exit 0" && {
    expect_parity effect -- fetch --update-shallow origin
  }
)

# mode=effect — --negotiation-tip narrows the set of commits reported to the
# server during negotiation.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --negotiation-tip=<rev>"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --negotiation-tip=HEAD accepted, exit 0" && {
    expect_parity effect -- fetch --negotiation-tip=HEAD origin
  }
)

# mode=effect — --negotiate-only (needs --negotiation-tip, no packfile).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --negotiate-only"
only_for_hash sha1-only && (sandbox
  # --negotiate-only requires the ack-only protocol v2 extension. git dies
  # 128 when the transport (file://) does not advertise it; gix silently
  # falls through to a normal fetch and exits 0. Semantic parity needs
  # gix-protocol to emit an ack-only wire-exchange and surface the
  # capability-missing error, which is a chunk of protocol work.
  shortcoming "--negotiate-only needs ack-only-capability enforcement in gix-protocol"
)

# mode=effect — --dry-run shows what would be fetched without changing state.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --dry-run"
only_for_hash sha1-only && (sandbox
  git init -q --bare upstream.git
  git init -q clone
  (cd clone
    git remote add origin "$(cd .. && pwd)/upstream.git"
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  )
  it "matches git: --dry-run against an empty upstream, exit 0" && {
    cd clone
    expect_parity effect -- fetch --dry-run origin
  }
)

# mode=effect — --porcelain is a bytes-mode row on the push side; on the
# fetch side gix currently does not emit git's machine-readable per-ref
# lines on stdout, so bytes-mode is still a follow-up. Exit-code parity
# against an empty upstream is already in place (parse-only).
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --porcelain"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --porcelain accepted, exit 0 (byte-exact output is a follow-up)" && {
    expect_parity effect -- fetch --porcelain origin
  }
)

# mode=effect — --filter=<spec> requests a partial-clone filter.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --filter=<spec>"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --filter=blob:none accepted, exit 0 against empty upstream" && {
    expect_parity effect -- fetch --filter=blob:none origin
  }
)

# mode=effect — --write-fetch-head is the default; --no-write-fetch-head
# suppresses FETCH_HEAD.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --no-write-fetch-head / --write-fetch-head"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --no-write-fetch-head accepted, exit 0" && {
    expect_parity effect -- fetch --no-write-fetch-head origin
  }
  it "matches git: --write-fetch-head is a no-op default, exit 0" && {
    expect_parity effect -- fetch --write-fetch-head origin
  }
)

# mode=effect — --force / -f overrides the non-fast-forward guard when a
# refspec lacks a leading '+'.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --force / -f"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --force accepted, exit 0 against empty upstream" && {
    expect_parity effect -- fetch --force origin
  }
  it "matches git: -f accepted, exit 0" && {
    expect_parity effect -- fetch -f origin
  }
)

# mode=effect — --keep retains the downloaded pack rather than exploding
# loose objects / discarding.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --keep / -k"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --keep accepted, exit 0" && {
    expect_parity effect -- fetch --keep origin
  }
  it "matches git: -k accepted, exit 0" && {
    expect_parity effect -- fetch -k origin
  }
)

# mode=effect — --multiple lets multiple repositories/groups follow.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --multiple"
only_for_hash sha1-only && (sandbox
  # --multiple changes positional parsing: git expects ALL positionals to
  # be remote names (not <repository> [<refspec>...]), and iterates each.
  # gix's Clap still parses positionals as repository + refspecs, so
  # 'gix fetch --multiple up1 up2' tries to use up2 as a refspec against
  # up1 and errors. Proper --multiple parity needs a runtime re-dispatch
  # when --multiple is set.
  shortcoming "--multiple needs Clap-level or dispatch-level remapping of positionals to remote-names"
)

# mode=effect — --auto-maintenance / --auto-gc (synonym) runs maintenance
# after fetch. Gix may no-op.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --auto-maintenance / --no-auto-maintenance / --auto-gc / --no-auto-gc"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --auto-maintenance accepted, exit 0" && {
    expect_parity effect -- fetch --auto-maintenance origin
  }
  it "matches git: --no-auto-maintenance accepted, exit 0" && {
    expect_parity effect -- fetch --no-auto-maintenance origin
  }
  it "matches git: --auto-gc accepted (alias), exit 0" && {
    expect_parity effect -- fetch --auto-gc origin
  }
  it "matches git: --no-auto-gc accepted (alias), exit 0" && {
    expect_parity effect -- fetch --no-auto-gc origin
  }
)

# mode=effect — --write-commit-graph / --no-write-commit-graph writes a
# commit-graph file after fetch.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --write-commit-graph / --no-write-commit-graph"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --write-commit-graph accepted, exit 0" && {
    expect_parity effect -- fetch --write-commit-graph origin
  }
  it "matches git: --no-write-commit-graph accepted, exit 0" && {
    expect_parity effect -- fetch --no-write-commit-graph origin
  }
)

# mode=effect — --prefetch rewrites the refspec into refs/prefetch/*.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --prefetch"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --prefetch accepted, exit 0" && {
    expect_parity effect -- fetch --prefetch origin
  }
)

# mode=effect — --prune / -p removes stale remote-tracking refs.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --prune / -p"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --prune accepted, exit 0" && {
    expect_parity effect -- fetch --prune origin
  }
  it "matches git: -p accepted, exit 0" && {
    expect_parity effect -- fetch -p origin
  }
)

# mode=effect — --prune-tags / -P is shorthand for refs/tags/*:refs/tags/*
# + --prune.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --prune-tags / -P"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --prune-tags accepted, exit 0" && {
    expect_parity effect -- fetch --prune-tags origin
  }
  it "matches git: -P accepted, exit 0" && {
    expect_parity effect -- fetch -P origin
  }
)

# mode=effect — --tags / -t fetches refs/tags/* alongside other refs.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --tags / -t"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --tags accepted, exit 0" && {
    expect_parity effect -- fetch --tags origin
  }
  it "matches git: -t accepted, exit 0" && {
    expect_parity effect -- fetch -t origin
  }
)

# mode=effect — --no-tags / -n disables automatic tag following.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --no-tags / -n"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --no-tags suppresses tag auto-follow, exit 0" && {
    expect_parity effect -- fetch --no-tags origin
  }
  it "matches git: -n is the short alias for --no-tags, exit 0" && {
    expect_parity effect -- fetch -n origin
  }
)

# mode=effect — --refetch ignores local objects and refetches a fresh copy.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --refetch"
only_for_hash sha1-only && (sandbox
  # --refetch needs a real refspec AND a real upstream commit — otherwise
  # git hangs in negotiation. With a one-commit upstream and an explicit
  # 'master' refspec both binaries complete and exit 0.
  git init -q --bare upstream.git
  git clone -q upstream.git work &>/dev/null
  (cd work
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
    git push -q origin master
  )
  git init -q clone
  (cd clone
    git remote add origin "$(cd .. && pwd)/upstream.git"
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  )
  cd clone
  it "matches git: --refetch with a real refspec, exit 0" && {
    expect_parity effect -- fetch --refetch origin master
  }
)

# mode=effect — --refmap overrides remote.<name>.fetch for cmdline-specified
# refs.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --refmap=<spec>"
only_for_hash sha1-only && (sandbox
  # --refmap needs a real refspec AND a remote-tracking destination; mapping
  # into refs/heads/* (the checked-out branch) would make git die 128 with
  # 'refusing to fetch into branch'. Use the standard refs/remotes/origin/*
  # destination for a clean round-trip.
  git init -q --bare upstream.git
  git clone -q upstream.git work &>/dev/null
  (cd work
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm c1
    git push -q origin master
  )
  git init -q clone
  (cd clone
    git remote add origin "$(cd .. && pwd)/upstream.git"
    git config commit.gpgsign false
    git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  )
  cd clone
  it "matches git: --refmap=refs/heads/*:refs/remotes/origin/* + refspec, exit 0" && {
    expect_parity effect -- fetch --refmap=refs/heads/*:refs/remotes/origin/* origin master
  }
)

# mode=effect — --recurse-submodules controls recursive fetch of submodules.
# Accepts yes, on-demand, no. Bare form defaults to yes.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --recurse-submodules[=<mode>]"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --recurse-submodules=yes accepted, exit 0 (no submodules in fixture)" && {
    expect_parity effect -- fetch --recurse-submodules=yes origin
  }
  it "matches git: --recurse-submodules=on-demand accepted, exit 0" && {
    expect_parity effect -- fetch --recurse-submodules=on-demand origin
  }
  it "matches git: --recurse-submodules=no accepted, exit 0" && {
    expect_parity effect -- fetch --recurse-submodules=no origin
  }
  it "matches git: bare --recurse-submodules defaults to yes, exit 0" && {
    expect_parity effect -- fetch --recurse-submodules origin
  }
)

# mode=effect — --no-recurse-submodules is an alias for --recurse-submodules=no.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --no-recurse-submodules"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --no-recurse-submodules accepted, exit 0" && {
    expect_parity effect -- fetch --no-recurse-submodules origin
  }
)

# mode=effect — --recurse-submodules rejects unknown values with
# `fatal: bad recurse-submodules argument`.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --recurse-submodules=<bogus>"
only_for_hash sha1-only && (sandbox
  git init -q
  git config commit.gpgsign false
  git -c user.email=x@x -c user.name=x commit --allow-empty -qm init
  git remote add origin /tmp/parity-unused
  it "matches git: --recurse-submodules=bogus dies 128" && {
    expect_parity effect -- fetch --recurse-submodules=bogus origin
  }
  it "matches git: bogus beats --negotiate-only conflict (parse-time)" && {
    expect_parity effect -- fetch --negotiate-only --recurse-submodules=bogus origin
  }
)

# mode=effect — --jobs / -j parallelizes submodule and multi-remote fetches.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --jobs=<n> / -j"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --jobs=2 accepted, exit 0" && {
    expect_parity effect -- fetch --jobs=2 origin
  }
  it "matches git: -j 2 accepted, exit 0" && {
    expect_parity effect -- fetch -j 2 origin
  }
)

# mode=effect — --set-upstream wires branch.<name>.{remote,merge}.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --set-upstream"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --set-upstream accepted, exit 0 against empty upstream" && {
    expect_parity effect -- fetch --set-upstream origin
  }
)

# mode=effect — --update-head-ok is internal/pull-only but accepted.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --update-head-ok / -u"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --update-head-ok accepted, exit 0" && {
    expect_parity effect -- fetch --update-head-ok origin
  }
  it "matches git: -u accepted, exit 0" && {
    expect_parity effect -- fetch -u origin
  }
)

# mode=effect — --upload-pack overrides the remote's upload-pack program.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --upload-pack=<path>"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --upload-pack=git-upload-pack is a no-op default" && {
    expect_parity effect -- fetch --upload-pack=git-upload-pack origin
  }
)

# mode=effect — --quiet / -q silences the status output.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --quiet / -q"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --quiet exit 0 against empty upstream" && {
    expect_parity effect -- fetch --quiet origin
  }
  it "matches git: -q exit 0" && {
    expect_parity effect -- fetch -q origin
  }
)

# mode=effect — --verbose / -v requests more output, including up-to-date
# refs. Gix emits the same exit code; output diverges and is not asserted.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --verbose / -v"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --verbose exit 0 against empty upstream" && {
    expect_parity effect -- fetch --verbose origin
  }
  it "matches git: -v exit 0" && {
    expect_parity effect -- fetch -v origin
  }
)

# mode=effect — --progress forces progress even if stderr is not a TTY.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --progress"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --progress accepted, exit 0" && {
    expect_parity effect -- fetch --progress origin
  }
)

# mode=effect — --server-option / -o transmits protocol-v2 server options.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --server-option=<option> / -o"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --server-option=key=val accepted, exit 0" && {
    expect_parity effect -- fetch --server-option=key=val origin
  }
  it "matches git: -o key=val accepted, exit 0" && {
    expect_parity effect -- fetch -o key=val origin
  }
)

# mode=effect — --show-forced-updates / --no-show-forced-updates controls
# the forced-update check.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --show-forced-updates / --no-show-forced-updates"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --show-forced-updates accepted, exit 0" && {
    expect_parity effect -- fetch --show-forced-updates origin
  }
  it "matches git: --no-show-forced-updates accepted, exit 0" && {
    expect_parity effect -- fetch --no-show-forced-updates origin
  }
)

# mode=effect — -4 and -6 force IPv4 / IPv6 resolution at the transport layer.
# Over file:// both are no-ops — exit-code parity only.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch -4 / --ipv4 and -6 / --ipv6"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: -4 accepted, exit 0 against empty upstream" && {
    expect_parity effect -- fetch -4 origin
  }
  it "matches git: -6 accepted, exit 0" && {
    expect_parity effect -- fetch -6 origin
  }
  it "matches git: -4 -6 last-wins (IPv6), exit 0" && {
    expect_parity effect -- fetch -4 -6 origin
  }
  it "matches git: --ipv4 long-form, exit 0" && {
    expect_parity effect -- fetch --ipv4 origin
  }
  it "matches git: --ipv6 long-form, exit 0" && {
    expect_parity effect -- fetch --ipv6 origin
  }
)

# mode=effect — --stdin lets refspecs come from stdin.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch --stdin"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: --stdin accepted with empty stdin, exit 0" && {
    expect_parity effect -- fetch --stdin origin </dev/null
  }
)

# --- config-triggered error paths -----------------------------------------

# mode=effect — bad boolean config value for any fetch.* boolean. git dies
# with `fatal: bad boolean config value '<v>' for '<key-lower>'`.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch with fetch.<bool>=<bogus>"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: -c fetch.prune=bogus → 128" && {
    expect_parity effect -- -c fetch.prune=bogus fetch origin
  }
  it "matches git: -c fetch.pruneTags=bogus → 128" && {
    expect_parity effect -- -c fetch.pruneTags=bogus fetch origin
  }
  it "matches git: -c fetch.writeCommitGraph=bogus → 128" && {
    expect_parity effect -- -c fetch.writeCommitGraph=bogus fetch origin
  }
  it "matches git: -c fetch.showForcedUpdates=bogus → 128" && {
    expect_parity effect -- -c fetch.showForcedUpdates=bogus fetch origin
  }
)

# mode=effect — bad recurse-submodules config value. Note the message
# shape differs from the CLI flag ("bad fetch.recursesubmodules argument"
# vs "bad recurse-submodules argument") — git prints the config key
# verbatim for the config path.
# hash=sha1-only "gix cannot open sha256 remotes, see gix/src/clone/fetch/mod.rs unimplemented!()"
title "gix fetch with fetch.recurseSubmodules=<bogus>"
only_for_hash sha1-only && (sandbox
  bare-empty-upstream-with-origin
  it "matches git: -c fetch.recurseSubmodules=bogus → 128" && {
    expect_parity effect -- -c fetch.recurseSubmodules=bogus fetch origin
  }
)

# End-of-file sentinel: when every row is `only_for_hash sha1-only` and the
# active hash is sha256, the last statement returns 1 (skip), which would
# propagate out of `source` and trip `set -e` in tests/parity.sh. A trailing
# `:` normalizes the exit code so a fully-skipped file still returns 0.
:
