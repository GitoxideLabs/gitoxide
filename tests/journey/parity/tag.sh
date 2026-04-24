# Must be sourced into tests/parity.sh or tests/journey.sh — see tests/parity.sh.
#
# Parity scaffold for `git tag` ↔ `gix tag`.
#
# One `title` + `it` block per flag derived from
# vendor/git/Documentation/git-tag.adoc (OPTIONS section) and
# vendor/git/builtin/tag.c (cmd_tag options[] array, lines ~481-537) plus
# the four synopsis forms (create, delete, list, verify).
#
# Verdict modes (comment above each block):
#   bytes  — scriptable output consumed by tooling: list-mode output
#            (patterns, --format, --sort, --column, -n<num>), query
#            responses. Most list-mode rows want byte parity.
#   effect — UX-level parity (exit-code match + optional prose check).
#            Used for usage/error paths, help, create-mode (where the
#            observable effect is a ref written to refs/tags/ rather
#            than stdout bytes), delete-mode, and GPG-signing paths
#            that depend on external backends.
#
# Coverage on gix's current Clap surface (src/plumbing/options/mod.rs,
# `pub mod tag`):
#   Subcommands::Tag(tag::Platform) with only `Subcommands::List`.
# There are no top-level flags and no create/delete/verify subcommands.
# Every flag below (other than bare `gix tag` listing, which is already
# wired through core::repository::tag::list) will fail its first parity
# attempt by tripping Clap's UnknownArgument / unknown-subcommand path.
# Closing a row generally requires:
#   (1) replace `Subcommands::List` with a flag-bearing `Platform` that
#       mirrors git's cmdmode ('l'/'d'/'v'/create) + modifier flags,
#   (2) wire the flag to gitoxide_core::repository::tag in a new
#       subroutine (or extend `list`), using gix_ref / gix_refspec /
#       gix::refs::transaction for the mutation path,
#   (3) translate C-side invariants — filter.with_commit /
#       filter.no_commit reachability, OPT_REF_SORT semantics,
#       for-each-ref %(fieldname) atom set — to Rust.
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

title "gix tag"

# --- meta / help --------------------------------------------------------

# mode=effect — clap --help short-circuits before repo load, exits 0.
# git's --help delegates to `man git-tag`; gix returns clap's auto-
# generated help. Message text diverges wildly; only the exit-code
# match is asserted.
# hash=dual
title "gix tag --help"
only_for_hash dual && (sandbox
  it "matches git behavior" && {
    expect_parity effect -- tag --help
  }
)

# --- argument-parsing error paths --------------------------------------

# mode=effect — unknown flag: git exits 129 (usage_msg_opt in
# vendor/git/parse-options.c). gix's Clap layer maps UnknownArgument to
# 129 via src/plumbing/main.rs. Tested inside a repo so the arg-parse
# path runs after repo setup.
# hash=sha1-only
title "gix tag --bogus-flag"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- tag --bogus-flag
  }
)

# mode=bytes — `git tag` outside any repo dies 128 with
# "fatal: not a git repository (or any of the parent directories): .git".
# gix's plumbing repository() closure remaps the
# gix_discover::upwards::Error::NoGitRepository* variants to git's exact
# wording + exit 128. Byte-exact match confirmed: the one-line error
# matches character-for-character (see src/plumbing/main.rs handler).
# hash=dual
# mode=bytes
title "gix tag (outside a repository)"
only_for_hash dual && (sandbox
  it "matches git behavior" && {
    expect_parity bytes -- tag
  }
)

# --- default (no-arg) list mode -----------------------------------------

# mode=bytes — bare `git tag` with tags present prints each tag on its
# own line, sorted lexicographically by refname (`%(refname:lstrip=2)`
# is the default format; see cmd_tag list_tags fallback).
# gitoxide-core/src/repository/tag.rs::list was rewritten to emit this
# format; the `Version`-struct numeric sort and `[tag name: *]`
# decoration from Sebastian's original listing are gone. Progress
# rendering was also silenced for this arm (src/plumbing/main.rs tag
# arm uses `verbose` instead of `auto_verbose`) so stderr doesn't bleed
# `\x1b[2K\r` spinner frames into the merged-stream comparison.
# hash=sha1-only
title "gix tag (no args, tags present)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity bytes -- tag
  }
)

# mode=bytes — bare `git tag` in a repo with no tags prints nothing,
# exits 0.
# hash=sha1-only
title "gix tag (no args, no tags)"
only_for_hash sha1-only && (sandbox
  git init -q
  git -c commit.gpgsign=false commit -q --allow-empty -m "seed"
  it "matches git behavior" && {
    expect_parity bytes -- tag
  }
)

# --- list mode (-l / --list) --------------------------------------------

# mode=bytes — `-l` / `--list` explicit forms with no pattern. Same
# output as bare `git tag`. Two rows so the Clap wiring for both
# spellings is exercised.
# hash=sha1-only
title "gix tag -l / --list"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with -l" && {
    expect_parity bytes -- tag -l
  }
  it "matches git behavior with --list" && {
    expect_parity bytes -- tag --list
  }
)

# mode=bytes — `-l <pattern>` / `--list <pattern>`: fnmatch(3) shell
# wildcard filter (see git-tag.adoc: "The pattern is a shell wildcard
# (i.e., matched using `fnmatch`(3))"). Multiple patterns OR together.
# hash=sha1-only
title "gix tag -l <pattern>"
only_for_hash sha1-only && (small-repo-in-sandbox
  git tag v1.0 && git tag v2.0 && git tag other
  it "matches git behavior with -l 'v*'" && {
    expect_parity bytes -- tag -l 'v*'
  }
  it "matches git behavior with multiple patterns" && {
    expect_parity bytes -- tag -l 'v1.*' 'other'
  }
  it "matches git behavior with non-matching pattern" && {
    expect_parity bytes -- tag -l 'nope-*'
  }
)

# mode=bytes — `--contains [<commit>]`: only list tags whose commit
# has `<commit>` in its ancestry. Default `<commit>` is HEAD. Implies
# `--list`.
# hash=sha1-only
title "gix tag --contains"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with --contains HEAD" && {
    expect_parity bytes -- tag --contains HEAD
  }
  it "matches git behavior with --contains HEAD~" && {
    expect_parity bytes -- tag --contains HEAD~
  }
  it "matches git behavior with --contains (no arg, defaults to HEAD)" && {
    expect_parity bytes -- tag --contains
  }
)

# mode=bytes — `--no-contains [<commit>]`: inverse of `--contains`.
# Implies `--list`.
# hash=sha1-only
title "gix tag --no-contains"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with --no-contains HEAD" && {
    expect_parity bytes -- tag --no-contains HEAD
  }
  it "matches git behavior with --no-contains HEAD~" && {
    expect_parity bytes -- tag --no-contains HEAD~
  }
)

# mode=bytes — `--merged [<commit>]`: only list tags reachable from
# `<commit>` (HEAD default). Implies `--list`.
# hash=sha1-only
title "gix tag --merged"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with --merged HEAD" && {
    expect_parity bytes -- tag --merged HEAD
  }
  it "matches git behavior with --merged HEAD~" && {
    expect_parity bytes -- tag --merged HEAD~
  }
  it "matches git behavior with --merged (no arg)" && {
    expect_parity bytes -- tag --merged
  }
)

# mode=bytes — `--no-merged [<commit>]`: inverse of `--merged`. Tags
# whose commit is NOT reachable from `<commit>`.
# hash=sha1-only
title "gix tag --no-merged"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with --no-merged HEAD" && {
    expect_parity bytes -- tag --no-merged HEAD
  }
  it "matches git behavior with --no-merged HEAD~" && {
    expect_parity bytes -- tag --no-merged HEAD~
  }
)

# mode=bytes — `--points-at [<object>]`: only list tags that point
# directly at `<object>` (HEAD default). Implies `--list`. Lightweight
# tags match their referent; annotated tags match the tagged object
# (not the tag object itself).
# hash=sha1-only
title "gix tag --points-at"
only_for_hash sha1-only && (small-repo-in-sandbox
  # small-repo-in-sandbox puts `unannotated` on HEAD~~ (first commit)
  # and `annotated` on HEAD~ (second commit). HEAD itself (third) has
  # no tag, so --points-at HEAD lists nothing.
  it "matches git behavior with --points-at HEAD~" && {
    expect_parity bytes -- tag --points-at HEAD~
  }
  it "matches git behavior with --points-at HEAD~~" && {
    expect_parity bytes -- tag --points-at HEAD~~
  }
  it "matches git behavior with --points-at HEAD (no tags)" && {
    expect_parity bytes -- tag --points-at HEAD
  }
  it "matches git behavior with --points-at (no arg, defaults to HEAD)" && {
    expect_parity bytes -- tag --points-at
  }
)

# mode=bytes — `-n[<num>]`: list tags with <num> lines of annotation
# (or commit message for lightweight). `-n` with no number prints
# only the first line. Implies `--list`. Per git-tag.adoc: "If the
# tag is not annotated, the commit message is displayed instead."
# hash=sha1-only
title "gix tag -n"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with -n" && {
    expect_parity bytes -- tag -n
  }
  it "matches git behavior with -n1" && {
    expect_parity bytes -- tag -n1
  }
  it "matches git behavior with -n5" && {
    expect_parity bytes -- tag -n5
  }
)

# mode=bytes — `--sort=<key>`: key-based sort. git supports for-each-ref
# key syntax — `refname`, `taggerdate`, `version:refname` / `v:refname`,
# `-<key>` for descending. Default is `refname` (or `tag.sort` config).
# hash=sha1-only
title "gix tag --sort"
only_for_hash sha1-only && (small-repo-in-sandbox
  git tag v1.10 && git tag v1.2 && git tag v1.9
  it "matches git behavior with --sort=refname (default order)" && {
    expect_parity bytes -- tag --sort=refname
  }
  it "matches git behavior with --sort=-refname (descending)" && {
    compat_effect "tag --sort=<key> interpreter deferred (key-based sort, descending/version)" -- tag --sort=-refname
  }
  it "matches git behavior with --sort=version:refname" && {
    compat_effect "tag --sort=<key> interpreter deferred (key-based sort, descending/version)" -- tag --sort=version:refname
  }
  it "matches git behavior with --sort=v:refname (alias)" && {
    compat_effect "tag --sort=<key> interpreter deferred (key-based sort, descending/version)" -- tag --sort=v:refname
  }
)

# mode=bytes — `--format=<format>`: for-each-ref-style `%(fieldname)`
# interpolation. Default is `%(refname:strip=2)`. Atoms used by callers:
# %(refname), %(objectname), %(objecttype), %(subject), %(contents).
# hash=sha1-only
title "gix tag --format"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with --format='%(refname)'" && {
    expect_parity bytes -- tag --format=%\(refname\)
  }
  it "matches git behavior with --format='%(refname:short)'" && {
    expect_parity bytes -- tag --format=%\(refname:short\)
  }
  it "matches git behavior with --format='%(objectname) %(refname:strip=2)'" && {
    expect_parity bytes -- tag '--format=%(objectname) %(refname:strip=2)'
  }
)

# mode=effect — `--column[=<options>]` / `--no-column`: multi-column
# output for tag names (see `column.tag` config). git spells options
# like `always`, `never`, `auto`, `column=5`, `dense`/`nodense`, etc.
# Under effect mode the row checks exit-code parity only; byte output
# through column-folding is a later upgrade to `bytes` mode.
# hash=sha1-only
title "gix tag --column / --no-column"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with --column" && {
    compat_effect "tag --column packing deferred; Clap accepts, one-per-line output" -- tag --column
  }
  it "matches git behavior with --column=always" && {
    compat_effect "tag --column=always packing deferred; Clap accepts, one-per-line output" -- tag --column=always
  }
  it "matches git behavior with --no-column" && {
    expect_parity bytes -- tag --no-column
  }
)

# mode=bytes — `--omit-empty`: when `--format` expands to the empty
# string for a ref, suppress the trailing newline entirely (no blank
# line). Only meaningful in combination with `--format`.
# hash=sha1-only
title "gix tag --omit-empty"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with --omit-empty + empty-format" && {
    expect_parity bytes -- tag --omit-empty --format=
  }
)

# mode=effect — `--color[=<when>]`: respect colors in `--format`.
# `<when>` ∈ {always, never, auto}. Default (omitted) behaves as
# `always`. Without any `%(color:...)` atom in `--format`, this flag
# is a no-op content-wise; exit-code parity is the observable.
# hash=sha1-only
title "gix tag --color"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with --color" && {
    expect_parity effect -- tag --color
  }
  it "matches git behavior with --color=never" && {
    expect_parity effect -- tag --color=never
  }
  it "matches git behavior with --color=always" && {
    expect_parity effect -- tag --color=always
  }
  it "matches git behavior with --color=auto" && {
    expect_parity effect -- tag --color=auto
  }
)

# mode=bytes — `-i` / `--ignore-case`: sorting and pattern filtering
# are case-insensitive.
# hash=sha1-only
title "gix tag -i / --ignore-case"
only_for_hash sha1-only && (small-repo-in-sandbox
  git tag Alpha && git tag beta && git tag GAMMA
  it "matches git behavior with -i -l 'a*'" && {
    expect_parity bytes -- tag -i -l 'a*'
  }
  it "matches git behavior with --ignore-case -l 'a*'" && {
    expect_parity bytes -- tag --ignore-case -l 'a*'
  }
  it "matches git behavior with --ignore-case folding refname sort" && {
    expect_parity bytes -- tag --ignore-case
  }
)

# --- delete mode (-d / --delete) ----------------------------------------

# mode=effect — `-d <tagname>`: delete the named tag. On success git
# prints "Deleted tag '<tagname>' (was <short-sha>)" to stdout and
# exits 0. Effect mode: exit-code + ref-removal side-effect; byte
# parity of the "was <sha>" line is a later upgrade (sha dependency).
# hash=sha1-only
title "gix tag -d (existing tag)"
only_for_hash sha1-only && (sandbox
  # Defined inside the subshell so it's not picked up by other files.
  # expect_parity_reset needs a setup fn that initializes a repo
  # in-place (no pushd), since it provides its own per-binary workdir.
  # small-repo-in-sandbox uses `sandbox` internally, which double-
  # nests and leaves the repo in a tempdir expect_parity_reset's git
  # call never cds into → both sides then hit "not a git repository"
  # and a bogus parity green emerges. This function mirrors
  # small-repo-in-sandbox's seed but without the pushd.
  function _tag-parity-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    touch a
    git add a
    git commit -m "first"
    git tag unannotated
    touch b
    git add b
    git commit -m "second"
    git tag annotated -m "tag message"
  }
  it "matches git behavior with -d unannotated" && {
    expect_parity_reset _tag-parity-fixture effect -- tag -d unannotated
  }
  it "matches git behavior with --delete unannotated" && {
    expect_parity_reset _tag-parity-fixture effect -- tag --delete unannotated
  }
)

# mode=effect — `-d <nonexistent>`: git prints "error: tag '<name>'
# not found." to stderr and exits 1 (see for_each_tag_name in
# builtin/tag.c:85-108).
# hash=sha1-only
title "gix tag -d (nonexistent tag)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- tag -d nonexistent
  }
)

# mode=effect — `-d <t1> <t2> ...`: multiple tags in one invocation.
# git attempts each, prints per-tag "Deleted" / "error" lines, and
# exits with 1 if any failed, 0 if all succeeded.
# hash=sha1-only
title "gix tag -d (multiple)"
only_for_hash sha1-only && (sandbox
  function _tag-parity-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    touch a
    git add a
    git commit -m "first"
    git tag unannotated
    touch b
    git add b
    git commit -m "second"
    git tag annotated -m "tag message"
  }
  it "matches git behavior with multiple existing tags" && {
    expect_parity_reset _tag-parity-fixture effect -- tag -d unannotated annotated
  }
  it "matches git behavior with mix of existing + nonexistent" && {
    expect_parity_reset _tag-parity-fixture effect -- tag -d unannotated nonexistent
  }
)

# --- verify mode (-v / --verify) ----------------------------------------

# mode=effect — `-v <tagname>` on a lightweight tag: git dies 128
# ("<name> cannot verify a non-tag object"). On an annotated-but-
# unsigned tag, git prints the tag body and exits 1 with
# "error: no signature found". GPG verification itself is out of
# scope here (requires backend). The no-sig error path is observable
# without a GPG binary.
# hash=sha1-only
title "gix tag -v (annotated unsigned)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- tag -v annotated
  }
)

# mode=effect — `-v <lightweight>`: git prints
# `error: <name>: cannot verify a non-tag object of type commit.`
# on stderr and exits 1. Matches the ref-filter.c path that refuses
# to proceed when the resolved object isn't itself a tag.
# hash=sha1-only
title "gix tag -v (lightweight tag)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- tag -v unannotated
  }
)

# mode=effect — `-v <nonexistent>`: git prints "error: tag '<name>'
# not found." and exits 1.
# hash=sha1-only
title "gix tag -v (nonexistent)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity effect -- tag -v nonexistent
  }
)

# --- create mode: lightweight -------------------------------------------

# mode=effect — bare `git tag <name>` creates a lightweight tag
# pointing at HEAD. No stdout on success; side effect is
# refs/tags/<name> written. Exit 0.
# hash=sha1-only
title "gix tag <name> (lightweight)"
only_for_hash sha1-only && (sandbox
  function _tag-parity-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
  }
  it "matches git behavior" && {
    expect_parity_reset _tag-parity-fixture effect -- tag newtag
  }
)

# mode=effect — `git tag <name> <commit>` creates a lightweight tag
# at `<commit>` (resolved as revspec, defaults to HEAD when absent).
# hash=sha1-only
title "gix tag <name> <commit>"
only_for_hash sha1-only && (sandbox
  function _two-commit-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "c1"
    git commit -q --allow-empty -m "c2"
  }
  it "matches git behavior with HEAD~" && {
    expect_parity_reset _two-commit-fixture effect -- tag newtag HEAD~
  }
)

# mode=effect — `git tag <existing>` without `-f` dies 128 with
# "fatal: tag '<name>' already exists". See builtin/tag.c near the
# `if (!force && ...)` check around line 600.
# hash=sha1-only
title "gix tag <name> (already exists)"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior" && {
    expect_parity bytes -- tag unannotated
  }
)

# mode=effect — `-f <name>` / `--force <name>` replaces an existing
# tag without error.
# hash=sha1-only
title "gix tag -f / --force"
only_for_hash sha1-only && (sandbox
  function _tag-exists-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "c1"
    git commit -q --allow-empty -m "c2"
    git tag unannotated HEAD
  }
  it "matches git behavior with -f replacing existing" && {
    expect_parity_reset _tag-exists-fixture effect -- tag -f unannotated HEAD~
  }
  it "matches git behavior with --force replacing existing" && {
    expect_parity_reset _tag-exists-fixture effect -- tag --force unannotated HEAD~
  }
)

# mode=effect — invalid tagname (fails check_ref_format). git dies 128
# with "fatal: '<name>' is not a valid tag name." Examples of
# rejected forms: two consecutive dots, ends in `.lock`, contains
# control chars. See refs.c check_refname_format.
# hash=sha1-only
title "gix tag <invalid-name>"
only_for_hash sha1-only && (small-repo-in-sandbox
  it "matches git behavior with bad..name" && {
    expect_parity bytes -- tag 'bad..name'
  }
  it "matches git behavior with ends-in-.lock" && {
    expect_parity bytes -- tag 'foo.lock'
  }
)

# --- create mode: annotated ---------------------------------------------

# mode=effect — `-a -m "<msg>" <name>`: creates an annotated tag
# object in the object database + ref. No stdout on success.
# hash=sha1-only
title "gix tag -a -m"
only_for_hash sha1-only && (sandbox
  function _anno-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
  }
  it "matches git behavior" && {
    expect_parity_reset _anno-fixture effect -- tag -a -m "annotated" anno1
  }
)

# mode=effect — `--annotate --message=<msg>` canonical long forms.
# hash=sha1-only
title "gix tag --annotate --message"
only_for_hash sha1-only && (sandbox
  function _anno-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
  }
  it "matches git behavior" && {
    expect_parity_reset _anno-fixture effect -- tag --annotate --message=annotated anno1
  }
)

# mode=effect — `-m <msg>` without `-a`: per git-tag.adoc, "Implies
# `-a` if none of `-a`, `-s`, or `-u <key-id>` is given." Same
# annotated outcome.
# hash=sha1-only
title "gix tag -m (implies -a)"
only_for_hash sha1-only && (sandbox
  function _anno-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
  }
  it "matches git behavior" && {
    expect_parity_reset _anno-fixture effect -- tag -m "implied-annotate" anno1
  }
)

# mode=effect — multiple `-m` options concatenate as separate
# paragraphs. Per git-tag.adoc: "If multiple `-m` options are given,
# their values are concatenated as separate paragraphs."
# hash=sha1-only
title "gix tag -m -m (multiple messages)"
only_for_hash sha1-only && (sandbox
  function _anno-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
  }
  it "matches git behavior" && {
    expect_parity_reset _anno-fixture effect -- tag -m "first para" -m "second para" anno1
  }
)

# mode=effect — `-F <file>`: read message from file. `-F -` reads
# from stdin.
# hash=sha1-only
title "gix tag -F / --file"
only_for_hash sha1-only && (sandbox
  function _anno-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
    printf 'line one\nline two\n' > msg.txt
  }
  it "matches git behavior with -F file" && {
    expect_parity_reset _anno-fixture effect -- tag -F msg.txt anno1
  }
  it "matches git behavior with --file=file" && {
    expect_parity_reset _anno-fixture effect -- tag --file=msg.txt anno1
  }
  # `-F -` (stdin-sourced message) row is deferred: expect_parity_reset
  # does not forward $PARITY_STDIN to its setup/run pair, so both
  # binaries would read EOF instead of the expected body. Closable by
  # extending expect_parity_reset to plumb stdin through to git/gix.
  it "matches git behavior with -F - (stdin) [deferred]" && {
    shortcoming "tag -F - stdin row blocked on expect_parity_reset PARITY_STDIN plumbing"
  }
)

# mode=effect — `--cleanup=<mode>`: controls message-cleanup rules.
# `<mode>` ∈ {verbatim, whitespace, strip}. Default is `strip`.
# `verbatim` leaves message untouched; `whitespace` trims leading/
# trailing blank lines; `strip` also removes `#`-prefixed comments.
# hash=sha1-only
title "gix tag --cleanup"
only_for_hash sha1-only && (sandbox
  function _anno-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
  }
  it "matches git behavior with --cleanup=verbatim" && {
    expect_parity_reset _anno-fixture effect -- tag --cleanup=verbatim -m "msg" anno1
  }
  it "matches git behavior with --cleanup=whitespace" && {
    expect_parity_reset _anno-fixture effect -- tag --cleanup=whitespace -m "msg" anno1
  }
  it "matches git behavior with --cleanup=strip" && {
    expect_parity_reset _anno-fixture effect -- tag --cleanup=strip -m "msg" anno1
  }
  it "matches git behavior with --cleanup=bogus" && {
    expect_parity effect -- tag --cleanup=bogus -m "msg" anno1
  }
)

# mode=effect — `-e` / `--edit` with `-m` or `-F`: opens EDITOR to
# let user further edit the provided message. Interactive-editor
# paths can be pinned via `GIT_EDITOR=true` which accepts the file
# unchanged — same outcome as without -e in that case.
# hash=sha1-only
title "gix tag -e / --edit"
only_for_hash sha1-only && (sandbox
  function _anno-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
  }
  export GIT_EDITOR=true
  it "matches git behavior with GIT_EDITOR=true -e -m" && {
    expect_parity_reset _anno-fixture effect -- tag -e -m "msg" anno1
  }
)

# mode=effect — `--trailer "<tok>[=<val>]"`: append a
# git-interpret-trailers(1)-style trailer. Implies `-a`. Multiple
# `--trailer` entries accumulate. `--trailer "Key: val"` is the
# common form.
# hash=sha1-only
title "gix tag --trailer"
only_for_hash sha1-only && (sandbox
  function _anno-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
  }
  it "matches git behavior with --trailer 'Key: val'" && {
    expect_parity_reset _anno-fixture effect -- tag -m "body" --trailer "Key: val" anno1
  }
  it "matches git behavior with multiple --trailer" && {
    expect_parity_reset _anno-fixture effect -- tag -m "body" --trailer "K1: v1" --trailer "K2: v2" anno1
  }
)

# mode=effect — `--create-reflog` creates refs/tags/<name> with a
# reflog entry. The negated form `--no-create-reflog` overrides only
# an earlier `--create-reflog` (does not disable core.logAllRefUpdates).
# Side-effect observable via the presence of `.git/logs/refs/tags/<name>`.
# hash=sha1-only
title "gix tag --create-reflog"
only_for_hash sha1-only && (sandbox
  function _anno-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
  }
  it "matches git behavior with --create-reflog" && {
    expect_parity_reset _anno-fixture effect -- tag --create-reflog newtag
  }
  it "matches git behavior with --no-create-reflog" && {
    expect_parity_reset _anno-fixture effect -- tag --no-create-reflog newtag
  }
)

# --- create mode: signed ------------------------------------------------

# mode=effect — `-s <name>`: create a GPG-signed annotated tag.
# Requires a signing backend (gpg.program). Without a configured
# signer the command dies 128 — that error path is what we assert.
# The "successful signature" path depends on a GPG keychain and is
# out of scope for the CI-reproducible parity harness; track as a
# shortcoming once the -s row closes on the error path.
# hash=sha1-only
title "gix tag -s (no signing backend)"
only_for_hash sha1-only && (sandbox
  function _anno-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
  }
  export GNUPGHOME=/nonexistent
  it "matches git behavior without configured signer" && {
    expect_parity_reset _anno-fixture effect -- tag -s -m "signed" anno1
  }
)

# mode=effect — `-u <key-id>` / `--local-user=<key-id>`: sign with a
# specific key. Same signing-backend dependency as `-s`; we assert
# the no-backend error path.
# hash=sha1-only
title "gix tag -u / --local-user"
only_for_hash sha1-only && (sandbox
  function _anno-fixture() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgsign false
    git commit -q --allow-empty -m "seed"
  }
  export GNUPGHOME=/nonexistent
  it "matches git behavior with -u <keyid> no backend" && {
    expect_parity_reset _anno-fixture effect -- tag -u deadbeef -m "signed" anno1
  }
  it "matches git behavior with --local-user=<keyid> no backend" && {
    expect_parity_reset _anno-fixture effect -- tag --local-user=deadbeef -m "signed" anno1
  }
)

# mode=effect — `--no-sign` overrides `tag.gpgSign=true` in config.
# With `tag.gpgSign=true` set, `git tag -a -m "msg" <name>` would
# normally sign; `--no-sign` forces an unsigned annotated tag.
# hash=sha1-only
title "gix tag --no-sign"
only_for_hash sha1-only && (sandbox
  function _anno-fixture-with-gpgsign() {
    git-init-hash-aware
    git checkout -b main
    git config commit.gpgsign false
    git config tag.gpgSign true
    git commit -q --allow-empty -m "seed"
  }
  it "matches git behavior" && {
    expect_parity_reset _anno-fixture-with-gpgsign effect -- tag --no-sign -m "msg" anno1
  }
)

# End-of-file sentinel: when every row is `only_for_hash sha1-only` and the
# active hash is sha256, the last statement returns 1 (skip), which would
# propagate out of `source` and trip `set -e` in tests/parity.sh. A trailing
# `:` normalizes the exit code so a fully-skipped file still returns 0.
:
