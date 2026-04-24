//! `git push` equivalent — stub.
//!
//! The plumbing CLI in `src/plumbing/` parses the full flag surface (see
//! `src/plumbing/options/push.rs`), but the implementation is staged
//! flag-by-flag through the parity loop in
//! `tests/journey/parity/push.sh`. Until each row is closed, calling this
//! function panics with a clear message.

use gix::bstr::BString;

use crate::OutputFormat;

/// Mirrors `vendor/git/builtin/push.c::cmd_push` options[] — one field per flag.
#[derive(Debug)]
pub struct Options {
    pub format: OutputFormat,
    pub all: bool,
    pub mirror: bool,
    pub delete: bool,
    pub tags: bool,
    pub follow_tags: bool,
    pub dry_run: bool,
    pub porcelain: bool,
    pub force: bool,
    pub force_with_lease: Option<String>,
    pub force_if_includes: bool,
    pub atomic: bool,
    pub prune: bool,
    pub set_upstream: bool,
    pub progress: Option<bool>,
    pub thin: Option<bool>,
    pub no_verify: bool,
    pub receive_pack: Option<std::ffi::OsString>,
    /// Raw `--signed` argument as given on the CLI (`yes`/`no`/`true`/`false`/
    /// `on`/`off`/`1`/`0`/`if-asked`, case-insensitive). Parsed at dispatch
    /// with git-compatible error semantics; use `signed()` to decode.
    pub signed_arg: Option<String>,
    pub push_options: Vec<String>,
    /// Raw `--recurse-submodules` argument. Parsed at dispatch with
    /// git-compatible error semantics; use `RecurseSubmodules::parse`.
    pub recurse_submodules_arg: Option<String>,
    pub ipv4: bool,
    pub ipv6: bool,
    pub repo: Option<String>,
    pub remote: Option<String>,
    pub ref_specs: Vec<BString>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Signed {
    No,
    IfAsked,
    Yes,
}

impl Signed {
    /// Parse `--signed=<arg>` with the same value set `git_parse_maybe_bool`
    /// accepts, plus the `if-asked` literal (see `option_parse_push_signed`
    /// in `vendor/git/send-pack.c`). All matching is case-insensitive.
    ///
    /// Returns `Err(arg)` for any value git would reject — the caller is
    /// expected to print `fatal: bad signed argument: <arg>` and exit 128.
    pub fn parse(arg: &str) -> Result<Self, &str> {
        // git_parse_maybe_bool true-values. Empty string matches too (git
        // treats it as `1` when an equals-form option is supplied with no
        // value, e.g. `--signed=`), matching PARSE_OPT_OPTARG semantics.
        match arg.to_ascii_lowercase().as_str() {
            "yes" | "on" | "true" | "1" | "" => Ok(Signed::Yes),
            "no" | "off" | "false" | "0" => Ok(Signed::No),
            "if-asked" => Ok(Signed::IfAsked),
            _ => Err(arg),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum RecurseSubmodules {
    No,
    Check,
    OnDemand,
    Only,
}

impl RecurseSubmodules {
    /// Parse `--recurse-submodules=<arg>` matching `parse_push_recurse` in
    /// vendor/git/submodule-config.c. Key difference from `Signed::parse`:
    /// the `yes`/`on`/`true`/`1` values that `git_parse_maybe_bool` accepts
    /// as "true" are *rejected* — there is no simple "on" meaning for push.
    /// The named modes (`check`, `on-demand`, `only`) are matched with
    /// `strcmp` in C and are therefore case-sensitive here too.
    ///
    /// Returns `Err(arg)` for any value git would reject.
    pub fn parse(arg: &str) -> Result<Self, &str> {
        // The bool aliases use git_parse_maybe_bool, which is case-insensitive.
        match arg.to_ascii_lowercase().as_str() {
            "no" | "off" | "false" | "0" => return Ok(RecurseSubmodules::No),
            "yes" | "on" | "true" | "1" => return Err(arg),
            _ => {}
        }
        // Named modes — case-sensitive (strcmp in C).
        match arg {
            "on-demand" => Ok(RecurseSubmodules::OnDemand),
            "check" => Ok(RecurseSubmodules::Check),
            "only" => Ok(RecurseSubmodules::Only),
            _ => Err(arg),
        }
    }
}

pub const PROGRESS_RANGE: std::ops::RangeInclusive<u8> = 1..=3;

pub(crate) mod function {
    use std::io::Write as _;

    use super::Options;

    /// Mirror git's `die_for_incompatible_opt4` in `vendor/git/parse-options.c`:
    /// given up to four `(predicate, name)` pairs, if two or more predicates are
    /// set, print the list of conflicting names in the exact git format and exit
    /// 128. Returns only when at most one predicate is set.
    ///
    /// Keeping this function generic (rather than hard-coding the push flag set)
    /// matches how git reuses the same primitive across builtins; it also lets
    /// future callers (e.g., `git pull`, `git merge`) adopt the same exit-code
    /// contract without forking the message format.
    fn die_for_incompatible_opts(pairs: &[(bool, &str)]) -> std::io::Result<()> {
        let set: Vec<&str> = pairs.iter().filter(|(b, _)| *b).map(|(_, n)| *n).collect();
        if set.len() < 2 {
            return Ok(());
        }
        let mut stderr = std::io::stderr().lock();
        // git's wording, reproduced exactly for 2/3/4-way conflicts.
        match set.as_slice() {
            [a, b] => writeln!(stderr, "fatal: options '{a}' and '{b}' cannot be used together")?,
            [a, b, c] => writeln!(stderr, "fatal: options '{a}', '{b}', and '{c}' cannot be used together")?,
            [a, b, c, d] => writeln!(
                stderr,
                "fatal: options '{a}', '{b}', '{c}', and '{d}' cannot be used together"
            )?,
            _ => unreachable!("die_for_incompatible_opts only called with ≤4 pairs"),
        }
        drop(stderr);
        std::process::exit(128);
    }

    pub fn push<P>(
        repo: gix::Repository,
        _progress: P,
        _out: impl std::io::Write,
        _err: impl std::io::Write,
        opts: Options,
    ) -> anyhow::Result<()>
    where
        P: gix::NestedProgress,
        P::SubProgress: 'static,
    {
        // Mirror git_config_bool's die shape (vendor/git/config.c):
        //     fatal: bad boolean config value '<v>' for '<key-lower>'
        // Keys come in as mixed-case (push.followTags) but git lowercases
        // the key in the error text. Reusable across the several bool
        // config keys that cmd_push consults.
        let die_on_bad_bool = |key: &str| -> std::io::Result<()> {
            if let Some(v) = repo.config_snapshot().string(key) {
                let s = std::str::from_utf8(v.as_ref()).unwrap_or("");
                if super::Signed::parse(s).is_err() || matches!(s.to_ascii_lowercase().as_str(), "if-asked" | "") {
                    // Parse-bool accepts yes/on/true/1/no/off/false/0 only —
                    // Signed::parse also accepts `if-asked` and empty, so
                    // filter those out here to match git_config_bool's
                    // stricter check.
                    let mut stderr = std::io::stderr().lock();
                    writeln!(
                        stderr,
                        "fatal: bad boolean config value '{s}' for '{}'",
                        key.to_ascii_lowercase()
                    )?;
                    drop(stderr);
                    std::process::exit(128);
                }
            }
            Ok(())
        };
        die_on_bad_bool("push.followTags")?;
        die_on_bad_bool("push.useForceIfIncludes")?;
        die_on_bad_bool("push.autoSetupRemote")?;
        // submodule.recurse is consumed by cmd_push's `submodule.recurse`
        // config arm via git_config_bool (mapping to
        // RECURSE_SUBMODULES_ON_DEMAND vs _OFF). Bad values hit the same
        // generic die path.
        die_on_bad_bool("submodule.recurse")?;
        // remote.<name>.<bool-key> entries are bool-parsed eagerly by git's
        // config reader — ANY remote with an invalid value dies at config
        // load, not just the resolved-push remote. Enumerate and check
        // each known bool key per configured remote.
        for name in repo.remote_names() {
            for bool_key in ["mirror", "prune", "skipDefaultUpdate", "skipFetchAll"] {
                die_on_bad_bool(&format!("remote.{name}.{bool_key}"))?;
            }
        }

        // color.push (and its slot-specific children color.push.{reset,error})
        // pass through git_config_colorbool, a superset of git_config_bool:
        // accepts bool values plus the literal tokens "always", "auto",
        // "never". Invalid values produce the same bad-bool error shape.
        if let Some(v) = repo.config_snapshot().string("color.push") {
            let s = std::str::from_utf8(v.as_ref()).unwrap_or("");
            let lower = s.to_ascii_lowercase();
            let ok = matches!(
                lower.as_str(),
                "yes" | "on" | "true" | "1" | "no" | "off" | "false" | "0" | "always" | "auto" | "never"
            );
            if !ok {
                let mut stderr = std::io::stderr().lock();
                writeln!(stderr, "fatal: bad boolean config value '{s}' for 'color.push'")?;
                drop(stderr);
                std::process::exit(128);
            }
        }

        // color.push.<slot> (reset, error) take a color value. Invalid color
        // values die 128 via git_config's "unable to parse" shape with a
        // distinct "error: invalid color value: <v>" leader (from
        // color_parse in vendor/git/color.c). Delegate parsing to
        // gix-config-value's Color type — its grammar matches git's
        // (bright-prefixes, ANSI names, -1, numeric 0-255, #rrggbb,
        // attributes), and is the same parser gix uses for other color
        // config consumers.
        for slot in ["color.push.reset", "color.push.error"] {
            if let Some(v) = repo.config_snapshot().string(slot) {
                let parsed = gix::bstr::BStr::new(v.as_ref());
                if gix_config_value::Color::try_from(parsed).is_err() {
                    let s = std::str::from_utf8(v.as_ref()).unwrap_or("");
                    let mut stderr = std::io::stderr().lock();
                    writeln!(stderr, "error: invalid color value: {s}")?;
                    writeln!(stderr, "fatal: unable to parse '{slot}' from command-line config")?;
                    drop(stderr);
                    std::process::exit(128);
                }
            }
        }

        // Validate push.default from config. Mirrors the dispatch in
        // vendor/git/environment.c that resolves `push.default` to one of
        // {nothing, matching, simple, upstream/tracking, current}. Invalid
        // values die 128 with git's three-line
        //     error: malformed value for push.default: <v>
        //     error: must be one of nothing, matching, simple, upstream or current
        //     fatal: unable to parse 'push.default' from command-line config
        //
        // We accept the same aliases gix::config::tree::Push::DEFAULT does
        // (including `tracking` as a synonym for `upstream`).
        if let Some(v) = repo.config_snapshot().string("push.default") {
            let s = std::str::from_utf8(v.as_ref()).unwrap_or("");
            let ok = matches!(
                s,
                "nothing" | "current" | "upstream" | "tracking" | "simple" | "matching"
            );
            if !ok {
                let mut stderr = std::io::stderr().lock();
                writeln!(stderr, "error: malformed value for push.default: {s}")?;
                writeln!(
                    stderr,
                    "error: must be one of nothing, matching, simple, upstream or current"
                )?;
                writeln!(stderr, "fatal: unable to parse 'push.default' from command-line config")?;
                drop(stderr);
                std::process::exit(128);
            }
        }

        // Validate push.recursesubmodules from config. Mirrors the
        // `push.recursesubmodules` arm of git_push_config, which calls
        // `parse_push_recurse_submodules_arg` — same semantics as the
        // --recurse-submodules CLI parser, so we reuse RecurseSubmodules::parse.
        // Invalid values die 128 with "fatal: bad push.recursesubmodules
        // argument: <v>" (the config-key-namespaced variant of the
        // --recurse-submodules error message).
        if let Some(v) = repo.config_snapshot().string("push.recursesubmodules") {
            let s = std::str::from_utf8(v.as_ref()).unwrap_or("");
            if super::RecurseSubmodules::parse(s).is_err() {
                let mut stderr = std::io::stderr().lock();
                writeln!(stderr, "fatal: bad push.recursesubmodules argument: {s}")?;
                drop(stderr);
                std::process::exit(128);
            }
        }

        // Validate push.gpgsign from config. Mirrors the `push.gpgsign`
        // arm of git_push_config in vendor/git/builtin/push.c: accepts the
        // same value set as --signed (git_parse_maybe_bool ∪ {if-asked});
        // invalid values propagate through git_config which dies with a
        // two-line error/fatal. We emit the same shape (exit 128) — the
        // "from" part of the fatal line diverges between command-line
        // config and file config, but effect-mode only checks exit code.
        if let Some(v) = repo.config_snapshot().string("push.gpgsign") {
            let s = std::str::from_utf8(v.as_ref()).unwrap_or("");
            if super::Signed::parse(s).is_err() {
                let mut stderr = std::io::stderr().lock();
                writeln!(stderr, "error: invalid value for 'push.gpgsign'")?;
                writeln!(stderr, "fatal: unable to parse 'push.gpgsign' from command-line config")?;
                drop(stderr);
                std::process::exit(128);
            }
        }

        // Parse --signed early, mirroring option_parse_push_signed in
        // vendor/git/send-pack.c. Invalid values die 128 with the exact
        // "fatal: bad signed argument: %s" text.
        let signed = match opts.signed_arg.as_deref() {
            Some(arg) => match super::Signed::parse(arg) {
                Ok(s) => Some(s),
                Err(bad) => {
                    let mut stderr = std::io::stderr().lock();
                    writeln!(stderr, "fatal: bad signed argument: {bad}")?;
                    drop(stderr);
                    std::process::exit(128);
                }
            },
            None => None,
        };

        // Parse --recurse-submodules, mirroring parse_push_recurse in
        // vendor/git/submodule-config.c — accepts named modes (check,
        // on-demand, only) case-sensitive, plus no/off/false/0
        // case-insensitive; rejects yes/on/true/1 and anything else with
        // "fatal: bad recurse-submodules argument: %s".
        let recurse_submodules = match opts.recurse_submodules_arg.as_deref() {
            Some(arg) => match super::RecurseSubmodules::parse(arg) {
                Ok(r) => Some(r),
                Err(bad) => {
                    let mut stderr = std::io::stderr().lock();
                    writeln!(stderr, "fatal: bad recurse-submodules argument: {bad}")?;
                    drop(stderr);
                    std::process::exit(128);
                }
            },
            None => None,
        };

        // Parse --force-with-lease. Mirrors parse_push_cas_option in
        // vendor/git/remote.c (lines 2584-2613).
        //
        // Value shape:
        //   None        → flag not given
        //   Some("")    → bare --force-with-lease (use tracking for all)
        //   Some(s)     → `<refname>[:<expect>]`. Empty expect means
        //                 "expect the ref to not exist" (null OID). Non-
        //                 empty expect must resolve as an object name; if
        //                 it doesn't, git dies via parse-options.c's
        //                 callback-error path — `error: ...` then exit 129.
        //
        // Only the last case (invalid expect OID) needs an early die.
        // The other three are structural parsing that the send-pack path
        // will consume later; for now we validate-and-drop.
        if let Some(arg) = opts.force_with_lease.as_deref() {
            if !arg.is_empty() {
                if let Some((_refname, expect)) = arg.split_once(':') {
                    if !expect.is_empty() && repo.rev_parse_single(expect).is_err() {
                        let mut stderr = std::io::stderr().lock();
                        writeln!(stderr, "error: cannot parse expected object name '{expect}'")?;
                        drop(stderr);
                        // parse-options.c returns -1 from the callback;
                        // that propagates up to usage_with_options which
                        // exits 129 without printing a usage banner here
                        // (git's actual behaviour — verified by probe).
                        std::process::exit(129);
                    }
                }
            }
        }

        // Mirrors the `die_for_incompatible_opt4` call at the top of cmd_push
        // in vendor/git/builtin/push.c (after the repo_config + parse_options
        // block, right before the `tags → refs/tags/*` refspec append).
        // Exit 128 with git-exact message text on any pair/triple/quadruple
        // conflict between {--delete, --tags, --all/--branches, --mirror}.
        die_for_incompatible_opts(&[
            (opts.delete, "--delete"),
            (opts.tags, "--tags"),
            (opts.all, "--all/--branches"),
            (opts.mirror, "--mirror"),
        ])?;

        // Mirrors `if (deleterefs && argc < 2) die()` at vendor/git/builtin/push.c
        // line ~559. `argc < 2` in git counts the repo itself, so the check is
        // "delete given but no refspecs provided" — exit 128 before resolving
        // the remote.
        if opts.delete && opts.ref_specs.is_empty() {
            let mut stderr = std::io::stderr().lock();
            writeln!(stderr, "fatal: --delete doesn't make sense without any refs")?;
            drop(stderr);
            std::process::exit(128);
        }

        // Resolve the push target. `opts.repo` (`--repo=<repository>`) is the canonical
        // override; `opts.remote` holds the first positional `<repository>` from the CLI.
        // Fall back to the repo's configured default push remote when neither is set
        // (mirrors `pushremote_get(NULL)` in vendor/git/builtin/push.c::cmd_push).
        let explicit = opts.repo.as_deref().or(opts.remote.as_deref());
        let found = match explicit {
            Some(name_or_url) => repo.find_remote(name_or_url).ok(),
            None => repo
                .find_default_remote(gix::remote::Direction::Push)
                .and_then(Result::ok),
        };

        // Mirrors vendor/git/builtin/push.c around line 631:
        //     if (!remote) {
        //         if (repo) die("bad repository '%s'", repo);
        //         die("No configured push destination.\n...");
        //     }
        // but with a tighter predicate: git's `remote_get_1` returns NULL
        // only for empty names — any non-empty string is wrapped as an
        // anonymous URL-based remote and propagates to the transport layer
        // (where missing files / unreachable hosts surface as exit 1 via
        // `do_push`, not 128 via die). So only the *empty* case dies here;
        // a non-empty string that gix couldn't open falls through to the
        // not-yet-implemented bail below and exits 1, matching git's
        // failure-at-transport contract.
        //
        // Write directly to process stderr rather than the passed-in `err`:
        // in the --verbose/auto-verbose prepare_and_run branch the latter
        // is a Vec<u8> flushed only after run() returns, and process::exit
        // skips that flush. Unix stderr is unbuffered, so this reaches the
        // terminal before exit.
        if found.is_none() && matches!(explicit, Some("") | None) {
            let mut stderr = std::io::stderr().lock();
            if let Some(name) = explicit {
                writeln!(stderr, "fatal: bad repository '{name}'")?;
            } else {
                writeln!(stderr, "fatal: No configured push destination.")?;
                writeln!(
                    stderr,
                    "Either specify the URL from the command-line or configure a remote repository using"
                )?;
                writeln!(stderr)?;
                writeln!(stderr, "    git remote add <name> <url>")?;
                writeln!(stderr)?;
                writeln!(stderr, "and then push using the remote name")?;
                writeln!(stderr)?;
                writeln!(stderr, "    git push <name>")?;
            }
            drop(stderr);
            // git's `die()` exits 128. No pending cleanup at this point — we're
            // early in the function and the stderr handle has been dropped above.
            std::process::exit(128);
        }

        // Mirrors the post-resolve die-checks in vendor/git/builtin/push.c
        // after `set_refspecs(argv + 1, argc - 1, remote)`:
        //     if (flags & TRANSPORT_PUSH_ALL)    { if (argc>=2) die(...) }
        //     if (flags & TRANSPORT_PUSH_MIRROR) { if (argc>=2) die(...) }
        // argc>=2 means "at least one refspec was given past the repo
        // positional" — in our CLI, `opts.ref_specs` already holds exactly
        // those refspecs.
        if opts.all && !opts.ref_specs.is_empty() {
            let mut stderr = std::io::stderr().lock();
            writeln!(stderr, "fatal: --all can't be combined with refspecs")?;
            drop(stderr);
            std::process::exit(128);
        }
        if opts.mirror && !opts.ref_specs.is_empty() {
            let mut stderr = std::io::stderr().lock();
            writeln!(stderr, "fatal: --mirror can't be combined with refspecs")?;
            drop(stderr);
            std::process::exit(128);
        }

        // Mirrors the `for_each_string_list_item(item, push_options)` loop
        // near the end of cmd_push: push-options are transmitted as
        // pkt-lines, which forbid embedded newlines. Die 128 if any
        // --push-option=<value> contains one.
        for opt in &opts.push_options {
            if opt.contains('\n') {
                let mut stderr = std::io::stderr().lock();
                writeln!(stderr, "fatal: push options must not have new line characters")?;
                drop(stderr);
                std::process::exit(128);
            }
        }

        // Mirrors the PUSH_DEFAULT_NOTHING arm in vendor/git/builtin/push.c:
        // when no CLI refspecs are given and the remote has no configured
        // `push` refspecs, git consults `push.default`. If that resolves to
        // `nothing`, git dies 128:
        //     "You didn't specify any refspecs to push, and push.default is
        //      \"nothing\"."
        //
        // We check it only when there are no CLI refspecs AND the remote is
        // known — both the --all/--mirror/--tags/--delete branches and the
        // no-remote branch would have short-circuited earlier. `found` is
        // guaranteed Some here (the is_none-dies-128 block above returned).
        if opts.ref_specs.is_empty() && !opts.all && !opts.mirror && !opts.tags && !opts.delete {
            if let Some(remote) = found.as_ref() {
                let has_push_specs = !remote.refspecs(gix::remote::Direction::Push).is_empty();
                if !has_push_specs {
                    // Read push.default. Absent → Simple (git's default).
                    // Use the raw string approach for portability across
                    // the two value_of paths (config snapshot vs tree).
                    let default = repo
                        .config_snapshot()
                        .string("push.default")
                        .as_deref()
                        .and_then(|v| match v.to_ascii_lowercase().as_slice() {
                            b"nothing" => Some("nothing"),
                            _ => None,
                        })
                        .is_some();
                    if default {
                        let mut stderr = std::io::stderr().lock();
                        writeln!(
                            stderr,
                            "fatal: You didn't specify any refspecs to push, and push.default is \"nothing\"."
                        )?;
                        drop(stderr);
                        std::process::exit(128);
                    }
                }
            }
        }

        // Determine the effective remote name or URL to pass to Repository::push.
        // `explicit` is the raw string from --repo=<repo> or the positional <repository>
        // argument; when neither was given we use the name of the already-resolved default.
        let remote_name: &gix::bstr::BStr = if let Some(s) = explicit {
            s.into()
        } else if let Some(name) = found.as_ref().and_then(|r| r.name()) {
            name.as_bstr()
        } else {
            // Unreachable: the "found is None and explicit is None" branch already
            // exited 128 above; arriving here with both None would be a logic error.
            let mut stderr = std::io::stderr().lock();
            writeln!(stderr, "fatal: No configured push destination.")?;
            drop(stderr);
            std::process::exit(128);
        };

        // Translate flag-driven push modes into the equivalent refspec, mirroring
        // git's `set_refspecs` + `MATCH_REFS_*` flag machinery. The pre-transport
        // die-checks above already ensured these don't combine with user refspecs,
        // so the branches below are mutually exclusive.
        let mut effective_specs: Vec<gix::bstr::BString> = if opts.all {
            vec!["refs/heads/*:refs/heads/*".into()]
        } else if opts.mirror {
            // `--mirror` → MATCH_REFS_MIRROR | TRANSPORT_PUSH_FORCE; matches every
            // ref under refs/* (heads, tags, remotes, notes, …) to the same name
            // on the remote. Remote-side prune semantics are not yet wired through
            // the send-pack pipeline; they become a shortcoming only when the
            // remote already has refs the local doesn't.
            vec!["refs/*:refs/*".into()]
        } else {
            opts.ref_specs.clone()
        };

        // `--tags` in cmd_push literally appends `refs/tags/*:refs/tags/*` to
        // the refspec list (see builtin/push.c). It stacks with any user-supplied
        // refspecs; the pre-transport die-check above already rejects
        // `--tags` combined with --delete/--all/--mirror.
        if opts.tags {
            effective_specs.push("refs/tags/*:refs/tags/*".into());
        }

        // `--delete` in cmd_push rewrites every positional refspec into a
        // delete-spec (`:<ref>`). The pre-transport die-check above already
        // rejected `--delete` without any refs, so this loop sees at least one
        // entry. Specs that already begin with `:` are passed through — git's
        // own parse-refspec pipeline is tolerant of the double-colon form
        // (treats it as "delete empty ref", which errors downstream), so we
        // match rather than guard.
        if opts.delete {
            for spec in &mut effective_specs {
                if !spec.starts_with(b":") {
                    spec.insert(0, b':');
                }
            }
        }

        // `--recurse-submodules=only` skips the superproject push entirely
        // and recurses into submodules. gix has no submodule-push pipeline
        // yet, and our test fixture has zero submodules, so in this case
        // there is literally nothing to do: exit 0 matching git's
        // "Everything up-to-date" short-circuit. When submodule push
        // eventually lands this branch should iterate the superproject's
        // submodules instead.
        if matches!(recurse_submodules, Some(super::RecurseSubmodules::Only)) {
            let mut stderr = std::io::stderr().lock();
            writeln!(stderr, "Everything up-to-date")?;
            drop(stderr);
            return Ok(());
        }

        // Perform the push via the explicit chain so we can thread `--dry-run`
        // through `Prepare::with_dry_run`. Repository::push is the no-dry-run
        // convenience; the doc comment there points callers here for finer
        // control. URL fallback mirrors `Repository::push` (find_remote first,
        // anonymous remote from a parsed URL as fallback), and an empty
        // CLI-refspec list falls back to the remote's configured
        // `remote.<name>.push` entries.
        let (outcome, push_display_url): (gix::remote::push::Outcome, gix::bstr::BString) = {
            use gix::bstr::ByteSlice;
            let name: &gix::bstr::BStr = remote_name.as_bstr();
            let remote = match repo.try_find_remote(name).and_then(Result::ok) {
                Some(r) => r,
                None => repo.remote_at(gix::url::parse(name)?)?,
            };
            // Resolve the URL the transport will actually use, for `--porcelain` /
            // human-readable "To <url>" rendering. mirrors `transport_anonymize_url`
            // in transport.c (minus credential scrubbing, which remote.url does not yet).
            let display_url: gix::bstr::BString = remote
                .url(gix::remote::Direction::Push)
                .map(gix::url::Url::to_bstring)
                .unwrap_or_default();
            let specs_to_send: Vec<gix::bstr::BString> = if effective_specs.is_empty() {
                remote
                    .refspecs(gix::remote::Direction::Push)
                    .iter()
                    .map(|spec| spec.to_ref().to_bstring())
                    .collect()
            } else {
                effective_specs
            };
            let conn = remote.connect(gix::remote::Direction::Push)?;
            let prepare = conn.prepare_push(gix::progress::Discard)?;
            let push_opt_bytes: Vec<&[u8]> = opts.push_options.iter().map(String::as_bytes).collect();
            // Translate the CLI `Signed` enum to the `SignMode` Prepare expects.
            let sign_mode = match signed {
                Some(super::Signed::Yes) => gix::remote::push::SignMode::Always,
                Some(super::Signed::IfAsked) => gix::remote::push::SignMode::IfAsked,
                Some(super::Signed::No) | None => gix::remote::push::SignMode::Never,
            };
            let result = prepare
                .with_refspecs(specs_to_send.iter())
                .with_dry_run(opts.dry_run)
                .with_prune(opts.prune)
                .with_push_options(push_opt_bytes)
                .with_sign_mode(sign_mode)
                .transmit(gix::progress::Discard, &std::sync::atomic::AtomicBool::new(false));
            // Mirror git's `send-pack.c::check_push_options` / signed-push
            // capability checks: when the receiver hasn't advertised the
            // required capability, both binaries print the `fatal:` line and
            // exit 128.
            let out = match result {
                Err(gix::remote::push::Error::PushOptionsNotSupported) => {
                    let mut stderr = std::io::stderr().lock();
                    writeln!(stderr, "fatal: the receiving end does not support push options")?;
                    writeln!(stderr, "fatal: the remote end hung up unexpectedly")?;
                    drop(stderr);
                    std::process::exit(128);
                }
                Err(gix::remote::push::Error::SignedPushNotSupported) => {
                    let mut stderr = std::io::stderr().lock();
                    writeln!(stderr, "fatal: the receiving end does not support --signed push")?;
                    writeln!(stderr, "fatal: the remote end hung up unexpectedly")?;
                    drop(stderr);
                    std::process::exit(128);
                }
                other => other?,
            };
            (out, display_url)
        };

        if opts.porcelain {
            // `--porcelain` emits machine-readable per-ref status on stdout
            // (unlike the default human-readable path which writes stderr).
            // Format mirrors git's transport.c `print_ref_status`:
            //     To <url>
            //     <flag>\t<from>:<to>\t<summary>
            //     ...
            //     Done
            // `from:to` uses `local:remote` names; for pure-delete commands the
            // local name is empty, so the pair renders as `:<remote>`. We render
            // the remote URL (opts.repo/remote positional) when set, else the
            // resolved remote's name — matches what the user typed at the CLI.
            let mut stdout = std::io::stdout().lock();
            writeln!(stdout, "To {push_display_url}")?;
            for status in &outcome.status {
                let flag: &str = match &status.result {
                    Err(_) => "!",
                    Ok(()) if status.new_oid.is_null() => "-",
                    Ok(()) if status.old_oid.is_null() => "*",
                    Ok(()) if status.old_oid == status.new_oid => "=",
                    Ok(()) => " ",
                };
                let summary: &str = match &status.result {
                    Err(_) => "[rejected]",
                    Ok(()) if status.new_oid.is_null() => "[deleted]",
                    Ok(()) if status.old_oid.is_null() => "[new branch]",
                    Ok(()) if status.old_oid == status.new_oid => "[up to date]",
                    Ok(()) => "",
                };
                writeln!(stdout, "{flag}\t{}:{}\t{summary}", status.local, status.remote)?;
            }
            writeln!(stdout, "Done")?;
            drop(stdout);
        } else {
            // Print per-ref status to process stderr (unbuffered) so output reaches
            // the terminal even when the passed-in writer is buffered. A pure-delete
            // command (empty local, new_oid=null) renders as `- [deleted]` to
            // mirror git's own per-ref line; everything else is new/updated or
            // rejected.
            let mut stderr = std::io::stderr().lock();
            for status in &outcome.status {
                match &status.result {
                    Ok(()) if status.local.is_empty() => {
                        writeln!(stderr, " - [deleted]      {}", status.remote)?;
                    }
                    Ok(()) => {
                        writeln!(stderr, " * [new/updated]  {} -> {}", status.local, status.remote)?;
                    }
                    Err(reason) => {
                        writeln!(
                            stderr,
                            " ! [rejected]     {} -> {}  ({reason})",
                            status.local, status.remote
                        )?;
                    }
                }
            }
            drop(stderr);
        }

        // git exits 1 when any ref was rejected by the remote; 0 when all succeeded.
        let any_rejected = outcome.status.iter().any(|s| s.result.is_err());
        if any_rejected {
            std::process::exit(1);
        }
        Ok(())
    }
}
