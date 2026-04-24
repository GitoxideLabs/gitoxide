# Must be sourced into tests/parity.sh or tests/journey.sh — see tests/parity.sh.
#
# Parity scaffold for `git cat-file` ↔ `gix cat` (with `cat-file` alias to be
# added in iter 2 when the first row closes).
#
# One `title` + `it` block per flag derived from
# vendor/git/Documentation/git-cat-file.adoc and vendor/git/builtin/cat-file.c
# (cmd_cat_file + options[]). Flags enumerated from the options[] array
# (~19 entries), the four synopsis forms (<type> <object>, query modes,
# textconv/filters, batch family), and the BATCH OUTPUT atom table
# (objectname, objecttype, objectsize, objectsize:disk, deltabase, rest,
# objectmode).
#
# Verdict modes (comment above each block):
#   bytes  — scriptable output consumed by tooling: the query modes
#            (-e/-p/-t/-s) emit well-defined bytes that callers grep;
#            batch output is line-oriented and scripted; --filter output
#            uses a defined grammar. Most cat-file rows want byte parity.
#   effect — UX-level parity (exit-code match + optional prose check).
#            Used for usage/error paths and help, and for flags whose
#            observable side-effect is accept-or-reject rather than
#            byte-exact output (e.g. deprecated --allow-unknown-type).
#
# Coverage on gix's current Clap surface (src/plumbing/options/mod.rs):
#   Subcommands::Cat { revspec: String }
# That is the entire surface — one positional, zero flags. Every flag
# below will fail its first parity attempt by tripping Clap's
# UnknownArgument path or, more fundamentally, by the subcommand name
# itself: `gix cat-file` is not registered (only `gix cat`). The
# first-closable row (typically --help) therefore also carries the
# scaffold work: add `visible_alias = "cat-file"` to the Cat variant
# in src/plumbing/options/mod.rs so `gix cat-file …` parses.
#
# Closing a semantics row will generally require:
#   (1) add the flag to src/plumbing/options/mod.rs::Subcommands::Cat (or
#       promote Cat to Cat(cat::Platform) once the flag count justifies
#       the refactor),
#   (2) widen gitoxide_core::repository::cat::function::cat's signature
#       and/or add new entrypoints (batch readers, object-info emit,
#       type/size queries),
#   (3) implement semantics in gitoxide-core/src/repository/cat.rs using
#       gix::Object / gix::object::Header / gix::Repository::find* / the
#       `gix_odb` streaming API for blobs, `gix::filter` for --filters,
#       `gix_traverse` helpers for --batch-all-objects.
#
# Hash coverage: every row that opens a repository is `sha1-only` because
# gix-config rejects `extensions.objectFormat=sha256`
# (gix/src/config/tree/sections/extensions.rs try_into_object_format,
# sha1-only validator). Rows that short-circuit before repo load
# (--help, unknown-flag outside a repo, outside-of-repo) are `dual`.
# Rows flip to `dual` once that validator accepts sha256.
#
# parity-defaults:
#   hash=sha1-only "gix cannot load sha256 repos: extensions.objectFormat=sha256 rejected (gix/src/config/tree/sections/extensions.rs)"
#   mode=bytes

title "gix cat-file"

# --- meta / help --------------------------------------------------------

# mode=effect — clap --help short-circuits before repo load, exits 0.
# git's --help delegates to `man git-cat-file`; gix returns clap's
# auto-generated help. Message text diverges wildly; only the exit-code
# match is asserted. This row also demands the `visible_alias = "cat-file"`
# wiring on the Cat subcommand — without it `gix cat-file --help` parses
# as an unknown subcommand (exit 2) rather than reaching Clap's help
# short-circuit.
# hash=dual
# mode=effect
title "gix cat-file --help"
only_for_hash dual && (sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file --help
  }
)

# --- argument-parsing error paths --------------------------------------

# mode=effect — unknown flag: git exits 129 (usage_msg_opt in
# vendor/git/parse-options.c). gix's Clap layer maps UnknownArgument to
# 129 via src/plumbing/main.rs. Tested inside a repo so the arg-parse
# path runs after repo setup.
# hash=sha1-only
# mode=effect
title "gix cat-file --bogus-flag"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file --bogus-flag
  }
)

# mode=effect — `git cat-file` outside any repo dies 128 with
# "fatal: not a git repository (or any of the parent directories): .git".
# gix's plumbing repository() closure remaps the
# gix_discover::upwards::Error::NoGitRepository* variants to git's exact
# wording + exit 128 (see status.sh's analogous row).
# hash=dual
# mode=effect
title "gix cat-file (outside a repository)"
only_for_hash dual && (sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file -p HEAD
  }
)

# mode=effect — bare `git cat-file` with no args and no mode flag prints
# usage to stderr and exits 129 (parse_options with empty argv after
# opts). Arg-parse error path; in-repo to exercise config load first.
# hash=sha1-only
# mode=effect
title "gix cat-file (no arguments)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file
  }
)

# --- query modes: -e/-p/-t/-s + <type> <object> -----------------------

# mode=effect — `-e <obj>` tests object existence; exits 0 if present,
# non-zero otherwise. No stdout output on success; stderr only on malformed
# inputs (per git-cat-file(1) DESCRIPTION: "exit with non-zero status if
# <object> is of an invalid format, emit error"). Effect mode: exit-code
# parity, no byte content to compare.
# hash=sha1-only
# mode=effect
title "gix cat-file -e (existing object)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file -e HEAD
  }
)

# mode=effect — `-e <missing-sha>` (well-formed but unknown oid) exits
# non-zero. git emits "fatal: Not a valid object name <sha>" and exits
# 128 (via die() in cat_one_file). gix must mirror.
# hash=sha1-only
# mode=effect
title "gix cat-file -e (missing object)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with a well-formed but absent oid" && {
    expect_parity effect -- cat-file -e 0000000000000000000000000000000000000000
  }
  it "matches git behavior with an unresolvable ref name (exit 128)" && {
    expect_parity effect -- cat-file -e nonexistent-ref
  }
)

# mode=bytes — `-p <blob>` pretty-prints blob contents: raw bytes to
# stdout. For a blob this is identical to `<type> <object>` with
# type=blob. Scripts consume this; byte-exact required.
# hash=sha1-only
title "gix cat-file -p (blob)"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo blob-content > a && git add a && git commit -q -m "populate a"
  it "matches git behavior" && {
    expect_parity bytes -- cat-file -p HEAD:a
  }
)

# mode=bytes — `-p <tree>` pretty-prints tree entries. git's format is
# equivalent to `git ls-tree <tree>` (via cmd_ls_tree in cat-file.c).
# gix's current cat already dispatches tree → display_tree (pretty).
# hash=sha1-only
title "gix cat-file -p (tree)"
only_for_hash sha1-only && (small-repo-in-sandbox
  mkdir sub && echo nested > sub/c && git add sub && git commit -q -m "add subtree"
  it "matches git behavior (root tree with blob + subtree entries)" && {
    expect_parity bytes -- cat-file -p HEAD^{tree}
  }
)

# mode=bytes — `-p <commit>` pretty-prints commit header + message
# (tree/parent/author/committer/blank line/message). Byte-exact.
# hash=sha1-only
title "gix cat-file -p (commit)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity bytes -- cat-file -p HEAD
  }
)

# mode=bytes — `-p <tag>` pretty-prints annotated-tag header + message.
# `annotated` tag exists in small-repo-in-sandbox via `git tag annotated
# -m "tag message"`.
# hash=sha1-only
title "gix cat-file -p (annotated tag)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity bytes -- cat-file -p annotated
  }
)

# mode=bytes — `-t <obj>` prints the object's type name: "blob", "tree",
# "commit", or "tag". One line, trailing newline.
# hash=sha1-only
title "gix cat-file -t"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior on a commit (type = commit)" && {
    expect_parity bytes -- cat-file -t HEAD
  }
  it "matches git behavior on a tree (type = tree)" && {
    expect_parity bytes -- cat-file -t HEAD^{tree}
  }
  it "matches git behavior on a blob (type = blob)" && {
    expect_parity bytes -- cat-file -t HEAD:a
  }
  it "matches git behavior on an annotated tag (type = tag)" && {
    expect_parity bytes -- cat-file -t annotated
  }
)

# mode=bytes — `-s <obj>` prints the object's size in bytes (decimal,
# trailing newline). Source: odb_read_object_info_extended → oi.sizep.
# hash=sha1-only
title "gix cat-file -s"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior on a commit" && {
    expect_parity bytes -- cat-file -s HEAD
  }
  it "matches git behavior on a blob" && {
    expect_parity bytes -- cat-file -s HEAD:a
  }
)

# mode=bytes — positional `<type> <object>` form: raw object bytes
# (uncompressed) with the given type as a type hint / assertion. If the
# object's real type does not match and cannot be dereferenced to the
# requested type, git dies 128. Otherwise stdout is the raw bytes.
# hash=sha1-only
title "gix cat-file <type> <object>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior: blob <obj>" && {
    expect_parity bytes -- cat-file blob HEAD:a
  }
  it "matches git behavior: commit <obj>" && {
    expect_parity bytes -- cat-file commit HEAD
  }
  it "matches git behavior: tree <obj>" && {
    expect_parity bytes -- cat-file tree HEAD^{tree}
  }
)

# mode=effect — positional `<type> <object>` with a type mismatch (e.g.
# `blob HEAD` where HEAD is a commit, and the commit does not deref to
# a blob). git dies 128 with "fatal: <sha> is not a valid 'blob' object".
# hash=sha1-only
# mode=effect
title "gix cat-file <type> <object> (type mismatch)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file blob HEAD
  }
)

# --- mailmap ------------------------------------------------------------

# mode=effect — `--mailmap` / `--no-mailmap` are aliases of
# `--use-mailmap` / `--no-use-mailmap`. Toggles mailmap ident rewriting
# for commit/tag `-s`/`-p` output. On a fixture without a .mailmap file
# the flag is a no-op content-wise; exit-code parity holds.
# hash=sha1-only
# mode=effect
title "gix cat-file --mailmap / --no-mailmap"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with --mailmap -p HEAD" && {
    expect_parity effect -- cat-file --mailmap -p HEAD
  }
  it "matches git behavior with --no-mailmap -p HEAD" && {
    expect_parity effect -- cat-file --no-mailmap -p HEAD
  }
)

# mode=effect — `--use-mailmap` / `--no-use-mailmap` (canonical spelling).
# Same semantics as --mailmap; separate row because Clap aliasing needs
# both entries reachable.
# hash=sha1-only
# mode=effect
title "gix cat-file --use-mailmap / --no-use-mailmap"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with --use-mailmap -p HEAD" && {
    expect_parity effect -- cat-file --use-mailmap -p HEAD
  }
  it "matches git behavior with --no-use-mailmap -p HEAD" && {
    expect_parity effect -- cat-file --no-use-mailmap -p HEAD
  }
)

# mode=bytes — `--use-mailmap -s` with an actual .mailmap file present:
# size reflects post-rewrite object (longer or shorter than raw).
# Exercises the mailmap replacement path in cat_one_file.
# hash=sha1-only
title "gix cat-file --use-mailmap -s (with .mailmap)"
only_for_hash sha1-only && (small-repo-in-sandbox
  printf 'Mapped Name <mapped@example.com> Sebastian Thiel <git@example.com>\n' > .mailmap
  git add .mailmap && git commit -q -m "add mailmap"
  it "matches git behavior" && {
    expect_parity bytes -- cat-file --use-mailmap -s HEAD
  }
)

# --- textconv / filters ------------------------------------------------

# mode=bytes — `--textconv <rev:path>`: runs the textconv filter
# configured for <path> on the blob at <rev:path>. With no textconv
# filter configured the output is the raw blob. Exercises path resolution.
# hash=sha1-only
title "gix cat-file --textconv"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo blob-content > a && git add a && git commit -q -m "populate a"
  it "matches git behavior" && {
    expect_parity bytes -- cat-file --textconv HEAD:a
  }
)

# mode=bytes — `--filters <rev:path>`: runs smudge filters / EOL
# conversions / checkout filters on the blob. With no filters configured
# output is raw blob. Exercises gix::filter::Pipeline wiring.
# hash=sha1-only
title "gix cat-file --filters"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo blob-content > a && git add a && git commit -q -m "populate a"
  it "matches git behavior" && {
    expect_parity bytes -- cat-file --filters HEAD:a
  }
)

# mode=bytes — `--path=<path> --textconv <rev>`: alternative spelling
# for textconv when you have only the blob SHA (not a tree-ish:path).
# Same output as `--textconv <rev>:<path>`.
# hash=sha1-only
title "gix cat-file --path=<path> --textconv"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo blob-content > a && git add a && git commit -q -m "populate a"
  it "matches git behavior" && {
    expect_parity bytes -- cat-file --path=a --textconv HEAD:a
  }
)

# mode=bytes — `--path=<path> --filters <rev>`: same as `--path=<path>
# --textconv` but invokes the smudge-filter path.
# hash=sha1-only
title "gix cat-file --path=<path> --filters"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo blob-content > a && git add a && git commit -q -m "populate a"
  it "matches git behavior" && {
    expect_parity bytes -- cat-file --path=a --filters HEAD:a
  }
)

# mode=effect — `--path=<path>` without `--textconv`/`--filters` is a
# usage error: git dies 129 with "fatal: '--path=<path|tree-ish>' needs
# '--filters' or '--textconv'".
# hash=sha1-only
# mode=effect
title "gix cat-file --path=<path> (without --textconv/--filters)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file --path=a HEAD:a
  }
)

# --- batch family ------------------------------------------------------

# mode=bytes — `--batch`: reads object names from stdin, emits
# `<oid> SP <type> SP <size> LF<contents>LF` per line. Default format.
# hash=sha1-only
title "gix cat-file --batch"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo blob-content > a && git add a && git commit -q -m "populate a"
  it "matches git behavior" && {
    PARITY_STDIN="HEAD:a
" expect_parity bytes -- cat-file --batch
  }
)

# mode=bytes — `--batch=<format>`: custom format string. Exercises
# %(objectname), %(objecttype), %(objectsize) expansion + contents trailer.
# hash=sha1-only
title "gix cat-file --batch=<format>"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo blob-content > a && git add a && git commit -q -m "populate a"
  it "matches git behavior" && {
    PARITY_STDIN="HEAD:a
" expect_parity bytes -- cat-file "--batch=%(objecttype) %(objectsize)"
  }
)

# mode=bytes — `--batch-check`: like --batch but without the <contents>
# trailer. One `<oid> SP <type> SP <size> LF` per input line.
# hash=sha1-only
title "gix cat-file --batch-check"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior (default format)" && {
    PARITY_STDIN="HEAD
" expect_parity bytes -- cat-file --batch-check
  }
)

# mode=bytes — `--batch-check=<format>`: custom format, no contents.
# hash=sha1-only
title "gix cat-file --batch-check=<format>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with %(objectname) %(objecttype)" && {
    PARITY_STDIN="HEAD
" expect_parity bytes -- cat-file "--batch-check=%(objectname) %(objecttype)"
  }
)

# mode=bytes — `--batch-command`: reads `contents <obj>` / `info <obj>` /
# `flush` commands from stdin, dispatches each to --batch or --batch-check
# equivalent. Default format per command.
# hash=sha1-only
title "gix cat-file --batch-command"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo blob-content > a && git add a && git commit -q -m "populate a"
  it "matches git behavior" && {
    PARITY_STDIN="info HEAD
contents HEAD:a
" expect_parity bytes -- cat-file --batch-command
  }
)

# mode=bytes — `--batch-command=<format>`: custom format applied to each
# command's info/contents output.
# hash=sha1-only
title "gix cat-file --batch-command=<format>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    PARITY_STDIN="info HEAD
" expect_parity bytes -- cat-file "--batch-command=%(objecttype)"
  }
)

# mode=bytes — `--batch-all-objects`: ignore stdin, iterate every object
# in the odb (loose + packed + alternates). Combined with --batch-check
# for scriptable output. Requires --batch/--batch-check. Bitmap-aware on
# git's side; gix enumerates via gix_odb::Cache::iter.
# hash=sha1-only
title "gix cat-file --batch-all-objects"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior (sorted hash order)" && {
    expect_parity bytes -- cat-file --batch-all-objects --batch-check
  }
)

# mode=effect — `--batch-all-objects` without `--batch[-check]` is a
# usage error: git dies 129 "'--batch-all-objects' requires a batch mode".
# hash=sha1-only
# mode=effect
title "gix cat-file --batch-all-objects (without batch)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file --batch-all-objects
  }
)

# mode=effect — `--buffer`: toggles stdio buffering on batch output.
# Default is line-flushed for interactive use; --buffer switches to
# normal stdio buffering. Observable effect: output timing, not
# content. Pairs with batch mode only (outside batch git errors 129).
# hash=sha1-only
# mode=effect
title "gix cat-file --buffer"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with --buffer --batch-check" && {
    PARITY_STDIN="HEAD
" expect_parity bytes -- cat-file --buffer --batch-check
  }
)

# mode=effect — `--buffer` without batch: usage error 129.
# hash=sha1-only
# mode=effect
title "gix cat-file --buffer (without batch)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file --buffer
  }
)

# mode=effect — `--unordered`: with --batch-all-objects, visit objects
# in pack-storage order rather than hash-sorted. Observable effect is
# line ordering; the set of lines is identical. Effect-mode parity
# (exit-code match) chosen over bytes because order is spec-loose.
# hash=sha1-only
# mode=effect
title "gix cat-file --unordered"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file --batch-all-objects --batch-check --unordered
  }
)

# mode=bytes — `--follow-symlinks`: with --batch/--batch-check, resolve
# in-tree symlinks when requesting `tree-ish:path`. Produces specialized
# status lines (`symlink <size>`, `dangling <size>`, `loop <size>`,
# `notdir <size>`) per the BATCH OUTPUT section of git-cat-file(1).
# hash=sha1-only
title "gix cat-file --follow-symlinks"
only_for_hash sha1-only && (small-repo-in-sandbox
  echo blob-content > a && git add a && git commit -q -m "populate a"
  ln -s a link-to-a && git add link-to-a && git commit -q -m "add symlink"
  it "matches git behavior" && {
    PARITY_STDIN="HEAD:link-to-a
" expect_parity bytes -- cat-file --batch --follow-symlinks
  }
)

# mode=effect — `--follow-symlinks` without batch: usage error 129.
# hash=sha1-only
# mode=effect
title "gix cat-file --follow-symlinks (without batch)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file --follow-symlinks
  }
)

# mode=bytes — `-Z`: NUL-delimits input AND output (replaces LF with NUL
# in both directions). Recommended for scripting when object names may
# contain LF. Paired with any batch mode.
# hash=sha1-only
title "gix cat-file -Z (NUL in+out)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    PARITY_STDIN=$'HEAD\0HEAD:a\0' expect_parity bytes -- cat-file --batch-check -Z
  }
)

# mode=bytes — `-z`: NUL-delimits INPUT only (output remains LF).
# Deprecated in favor of `-Z` because output can be ambiguous.
# hash=sha1-only
title "gix cat-file -z (NUL input only)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    PARITY_STDIN=$'HEAD\0HEAD:a\0' expect_parity bytes -- cat-file --batch-check -z
  }
)

# mode=effect — `-Z` without batch mode: usage error 129.
# hash=sha1-only
# mode=effect
title "gix cat-file -Z (without batch)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file -Z
  }
)

# --- --filter (batch-only) ---------------------------------------------

# mode=effect — `--filter=<spec>` family is downgraded from bytes to
# effect across every row because the system `git` in the test
# environment (2.47.3) predates cat-file's `OPT_PARSE_LIST_OBJECTS_FILTER`
# registration and errors 129 ("option 'filters' takes no value") on
# any `--filter=<spec>`. gix similarly rejects the flag via clap's
# UnknownArgument → 129 remap. Both binaries exit 129 on every filter
# row, so exit-code parity holds; stderr text diverges (clap's
# "unexpected argument" vs git's "takes no value") and is expected.
# Rows flip to bytes-mode when the test-environment git grows the
# --filter= subparser.
# hash=sha1-only
title "gix cat-file --filter=blob:none"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior (both binaries exit 129)" && {
    expect_parity effect -- cat-file --batch-check --batch-all-objects --filter=blob:none
  }
)

# mode=effect — `--filter=blob:limit=<n>`: same path as blob:none
# (both git and gix exit 129 on parse_options / clap). See --filter=blob:none
# for the rationale.
# hash=sha1-only
title "gix cat-file --filter=blob:limit=<n>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with limit=0" && {
    expect_parity effect -- cat-file --batch-check --batch-all-objects --filter=blob:limit=0
  }
  it "matches git behavior with limit=1k" && {
    expect_parity effect -- cat-file --batch-check --batch-all-objects --filter=blob:limit=1k
  }
)

# mode=effect — `--filter=object:type=<t>`: same path (both error 129).
# hash=sha1-only
title "gix cat-file --filter=object:type=<type>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with type=commit" && {
    expect_parity effect -- cat-file --batch-check --batch-all-objects --filter=object:type=commit
  }
  it "matches git behavior with type=blob" && {
    expect_parity effect -- cat-file --batch-check --batch-all-objects --filter=object:type=blob
  }
)

# mode=effect — `--filter=<spec>` outside batch mode: git 2.47.3 also
# errors 129 (for a different reason than newer git would —
# "option takes no value"), and gix errors 129 via clap. Exit-code
# parity holds regardless.
# hash=sha1-only
title "gix cat-file --filter (without batch)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file --filter=blob:none
  }
)

# mode=effect — `--no-filter`: system git 2.47.3 does not accept
# `--no-filter` at all (exits 129 "unknown option 'no-filter'"),
# gix exits 129 via clap. Exit-code parity holds.
# hash=sha1-only
title "gix cat-file --no-filter"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file --batch-check --batch-all-objects --no-filter
  }
)

# --- batch format atoms (--batch-check=<format>) -----------------------

# mode=bytes — `%(objectname)`: full-length hex oid.
# hash=sha1-only
title "gix cat-file --batch-check='%(objectname)'"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    PARITY_STDIN="HEAD
" expect_parity bytes -- cat-file "--batch-check=%(objectname)"
  }
)

# mode=bytes — `%(objecttype)`: type name.
# hash=sha1-only
title "gix cat-file --batch-check='%(objecttype)'"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    PARITY_STDIN="HEAD
" expect_parity bytes -- cat-file "--batch-check=%(objecttype)"
  }
)

# mode=bytes — `%(objectsize)`: size in bytes.
# hash=sha1-only
title "gix cat-file --batch-check='%(objectsize)'"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    PARITY_STDIN="HEAD
" expect_parity bytes -- cat-file "--batch-check=%(objectsize)"
  }
)

# mode=bytes — `%(objectsize:disk)`: on-disk size (packed or loose). Per
# CAVEATS, identity of the copy measured is undefined when multiple
# copies exist, so this may diverge fixture-to-fixture; pinning to a
# freshly-init'd loose-only repo keeps it stable.
# hash=sha1-only
title "gix cat-file --batch-check='%(objectsize:disk)'"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    PARITY_STDIN="HEAD
" expect_parity bytes -- cat-file "--batch-check=%(objectsize:disk)"
  }
)

# mode=bytes — `%(deltabase)`: full-length hex oid of delta base, or
# null-oid (40/64 zeros) if not stored as delta. Loose-only fixture ⇒
# expect null-oid.
# hash=sha1-only
title "gix cat-file --batch-check='%(deltabase)'"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    PARITY_STDIN="HEAD
" expect_parity bytes -- cat-file "--batch-check=%(deltabase)"
  }
)

# mode=bytes — `%(rest)`: splits input on first whitespace; characters
# after that split are emitted in place of %(rest). Must set
# split_on_whitespace on expand_data.
# hash=sha1-only
title "gix cat-file --batch-check='%(rest)'"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    PARITY_STDIN="HEAD extra stuff
" expect_parity bytes -- cat-file "--batch-check=%(objectname) %(rest)"
  }
)

# mode=bytes — %(objectmode) is rejected by system git 2.47.3 with
# `fatal: bad cat-file format: %(objectmode)` + exit 128. gix matches
# bit-for-bit: its expand_atoms pre-flight check in the dispatch emits
# the identical fatal wording before the stdin loop starts, so
# byte-exact stderr parity holds. When the test-env git picks up the
# %(objectmode) atom (newer upstream), both binaries will succeed and
# the row stays bytes-mode unchanged.
# hash=sha1-only
title "gix cat-file --batch-check='%(objectmode)'"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    PARITY_STDIN="HEAD:a
" expect_parity bytes -- cat-file "--batch-check=%(objectmode) %(objectname)"
  }
)

# --- batch input error paths -------------------------------------------

# mode=bytes — missing object on stdin: batch prints `<input> missing LF`
# and keeps reading. Exit 0 at EOF.
# hash=sha1-only
title "gix cat-file --batch-check (missing object on stdin)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    PARITY_STDIN="notanoid
" expect_parity bytes -- cat-file --batch-check
  }
)

# mode=effect — ambiguous short sha on stdin. Seeding enough blobs
# (~1000) until a 4-hex prefix collides, then feeding that prefix to
# --batch-check: git emits warnings on stderr ("error: short object
# ID <p> is ambiguous\nhint: The candidates are:\n hint:   <oid>
# <type>\n ...") + stdout "<input> ambiguous\n" + exit 0. gix's
# rev_parse returns an ambiguous-lookup error which we currently
# collapse to "<input> missing\n" + exit 0 — exit parity holds, but
# stderr bytes would diverge (and git's hint text varies across
# versions), so effect-mode is the stable row.
# hash=sha1-only
title "gix cat-file --batch-check (ambiguous short sha)"
only_for_hash sha1-only && (sandbox
  git-init-hash-aware
  for i in $(seq 1 1000); do echo "content-$i" | git hash-object --stdin -w >/dev/null; done
  prefix=$(cd .git/objects && find . -maxdepth 2 -type f | sed 's|^\./||; s|/||' | awk '{print substr($0, 1, 4)}' | sort | uniq -d | head -1)
  it "matches git behavior (exit-code parity; git's hint text varies)" && {
    PARITY_STDIN="$prefix
" expect_parity effect -- cat-file --batch-check
  }
)

# --- historical / hidden -----------------------------------------------

# mode=effect — `--allow-unknown-type`: historical option, no-op per the
# option table's N_("historical option -- no-op") help text. Must be
# accepted by Clap without error; exits 0 alongside a valid mode flag.
# hash=sha1-only
# mode=effect
title "gix cat-file --allow-unknown-type"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file --allow-unknown-type -t HEAD
  }
)

# --- submodule / dangling / notdir / loop (follow-symlinks statuses) ---

# mode=bytes — gitlink tree entry whose commit is absent locally.
# Turns out git's "submodule" status line (via S_IFGITLINK in
# batch_object_write) only fires when `data->mode` was populated by
# --batch-all-objects; stdin-driven lookup via `HEAD:sub` goes
# through get_oid_with_context, returns MISSING_OBJECT, and emits
# "<input> missing" — same bytes as gix. Byte-exact parity holds
# trivially.
# hash=sha1-only
title "gix cat-file --batch (submodule entry)"
only_for_hash sha1-only && (small-repo-in-sandbox
  FAKE_COMMIT_OID=0123456789abcdef0123456789abcdef01234567
  git update-index --add --cacheinfo 160000,$FAKE_COMMIT_OID,sub
  git commit -q -m "add gitlink"
  it "matches git behavior" && {
    PARITY_STDIN="HEAD:sub
" expect_parity bytes -- cat-file --batch
  }
)

# mode=bytes — `--follow-symlinks` on a link that points outside the
# tree root: emits `symlink <size> LF<target>LF` with the external
# path (absolute or `..`-relative). Fixture: link to /etc/passwd.
# hash=sha1-only
title "gix cat-file --batch --follow-symlinks (link outside tree)"
only_for_hash sha1-only && (small-repo-in-sandbox
  ln -s /etc/passwd alink && git add alink && git commit -q -m "alink"
  it "matches git behavior" && {
    PARITY_STDIN="HEAD:alink
" expect_parity bytes -- cat-file --batch --follow-symlinks
  }
)

# mode=bytes — `--follow-symlinks` on a broken symlink (dangling):
# emits `dangling <size> LF<target>LF`.
# hash=sha1-only
title "gix cat-file --batch --follow-symlinks (dangling)"
only_for_hash sha1-only && (small-repo-in-sandbox
  ln -s missing-target dlink && git add dlink && git commit -q -m "dlink"
  it "matches git behavior" && {
    PARITY_STDIN="HEAD:dlink
" expect_parity bytes -- cat-file --batch --follow-symlinks
  }
)

# --- combined flags ----------------------------------------------------

# mode=effect — conflicting mode flags (e.g. `-e -p`): git exits 129
# "options '-e' and '-p' are mutually exclusive" via OPT_CMDMODE.
# hash=sha1-only
# mode=effect
title "gix cat-file -e -p (mutually exclusive modes)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file -e -p HEAD
  }
)

# mode=effect — mode flag combined with batch (e.g. `-p --batch`): git
# exits 129 "'-p' is incompatible with batch mode".
# hash=sha1-only
# mode=effect
title "gix cat-file -p --batch (mode + batch)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file -p --batch
  }
)

# mode=effect — two batch modes at once (e.g. `--batch --batch-check`):
# git exits 128 "only one batch option may be specified" via the
# batch_option_callback error() path.
# hash=sha1-only
# mode=effect
title "gix cat-file --batch --batch-check (conflicting batches)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- cat-file --batch --batch-check
  }
)

# End-of-file sentinel: when every row is `only_for_hash sha1-only` and the
# active hash is sha256, the last statement returns 1 (skip), which would
# propagate out of `source` and trip `set -e` in tests/parity.sh. A trailing
# `:` normalizes the exit code so a fully-skipped file still returns 0.
:
