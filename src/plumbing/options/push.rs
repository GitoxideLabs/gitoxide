//! `gix push` plumbing CLI.
//!
//! Flag surface mirrors `vendor/git/builtin/push.c::cmd_push` options[] and
//! `vendor/git/Documentation/git-push.adoc`. Parity coverage lives in
//! `tests/journey/parity/push.sh`.

use std::ffi::OsString;

#[derive(Debug, clap::Parser)]
pub struct Platform {
    /// Push all branches (equivalent to refspec `refs/heads/*`).
    ///
    /// Incompatible with `--mirror`, `--tags`, and `--delete` — validated at
    /// dispatch to match git's exit code (128) rather than Clap's default (2).
    #[clap(long, visible_alias = "branches")]
    pub all: bool,

    /// Mirror all refs to the remote (implies `--force`).
    ///
    /// Incompatible with `--all`, `--tags`, and `--delete` — see `--all`.
    #[clap(long)]
    pub mirror: bool,

    /// Delete the given refs from the remote.
    ///
    /// Incompatible with `--all`, `--mirror`, and `--tags` — see `--all`.
    #[clap(long, short = 'd')]
    pub delete: bool,

    /// Push all refs under `refs/tags`.
    ///
    /// Incompatible with `--all`, `--mirror`, and `--delete` — see `--all`.
    #[clap(long)]
    pub tags: bool,

    /// Push all missing but reachable tags after normal refs.
    #[clap(long)]
    pub follow_tags: bool,

    /// Show what would be pushed without actually pushing.
    #[clap(long, short = 'n')]
    pub dry_run: bool,

    /// Produce machine-readable output (`<flag> <from>:<to> <summary> (<reason>)`).
    #[clap(long)]
    pub porcelain: bool,

    /// Force-update refs even when the update is not a fast-forward.
    #[clap(long, short = 'f')]
    pub force: bool,

    /// Require the remote ref's current value to match before updating.
    ///
    /// Accepts an optional `[<refname>[:<expect>]]`. Passing just the flag
    /// compares against the locally-recorded remote-tracking branch.
    ///
    /// `require_equals = true` so that `--force-with-lease` on its own
    /// doesn't greedily consume the next positional argument — git's
    /// `PARSE_OPT_OPTARG` heuristic treats the next token as a positional
    /// when it doesn't look like an option, and Clap needs the explicit
    /// `=` form to match that behavior.
    #[clap(
        long,
        value_name = "REFNAME[:EXPECT]",
        num_args = 0..=1,
        default_missing_value = "",
        require_equals = true,
    )]
    pub force_with_lease: Option<String>,

    /// Require remote refs to include our locally-known commits before force-update.
    #[clap(long)]
    pub force_if_includes: bool,

    /// Request an atomic transaction on the remote side (all-or-nothing updates).
    #[clap(long)]
    pub atomic: bool,

    /// Remove remote-tracking refs that no longer exist on the remote.
    #[clap(long)]
    pub prune: bool,

    /// Set upstream (tracking) reference for the pushed branch.
    #[clap(long, short = 'u')]
    pub set_upstream: bool,

    /// Produce more verbose output (inherited from git's OPT__VERBOSITY).
    ///
    /// Push-scoped, separate from the top-level `gix -v`. Currently parsed
    /// but not consumed — the flag exists so `gix push -v` mirrors
    /// `git push -v`'s exit contract at parse time; UX wiring arrives
    /// with the happy-path send-pack.
    #[clap(short = 'v', long)]
    pub verbose: bool,

    /// Suppress non-error output (inherited from git's OPT__VERBOSITY).
    #[clap(short = 'q', long)]
    pub quiet: bool,

    /// Force progress reporting.
    #[clap(long, overrides_with = "no_progress")]
    pub progress: bool,

    /// Disable progress reporting.
    #[clap(long, overrides_with = "progress")]
    pub no_progress: bool,

    /// Produce a thin pack (the default).
    #[clap(long, overrides_with = "no_thin")]
    pub thin: bool,

    /// Disable thin-pack generation.
    #[clap(long, overrides_with = "thin")]
    pub no_thin: bool,

    /// Bypass the `pre-push` hook.
    #[clap(long)]
    pub no_verify: bool,

    /// Path (or name on PATH) of the receive-pack program to invoke remotely.
    #[clap(long, visible_alias = "exec", value_name = "PROGRAM")]
    pub receive_pack: Option<OsString>,

    /// GPG-sign the push.
    ///
    /// Accepts the same value set as `git --signed`:
    /// `yes`/`true`/`on`/`1` (always sign), `no`/`false`/`off`/`0` (never
    /// sign), or `if-asked` (sign only when the server requests it). Invalid
    /// values fail at dispatch with git's exact `fatal: bad signed argument`
    /// message, not Clap's default enum-validation error, to match git's
    /// exit code (128).
    #[clap(
        long,
        value_name = "MODE",
        num_args = 0..=1,
        default_missing_value = "yes",
        require_equals = true,
    )]
    pub signed: Option<String>,

    /// Transmit the given option to the receive-pack on the other side.
    #[clap(long, short = 'o', value_name = "OPTION")]
    pub push_option: Vec<String>,

    /// Recursion strategy for submodules.
    ///
    /// Accepts the same values as `git --recurse-submodules`: `check`,
    /// `on-demand`, `only`, or `no`/`false`/`off`/`0` (case-sensitive for
    /// the named modes; case-insensitive for the bool aliases, matching
    /// git's `parse_push_recurse` in submodule-config.c). Unlike `--signed`,
    /// `yes`/`on`/`true`/`1` are *rejected* — pushing submodules has no
    /// simple "on" meaning. Invalid values fail at dispatch with git's
    /// exact `fatal: bad recurse-submodules argument` message.
    #[clap(long, value_name = "MODE")]
    pub recurse_submodules: Option<String>,

    /// Force IPv4 connections to the remote.
    ///
    /// Mutually overrides `-6`/`--ipv6` (last wins), matching git's
    /// OPT_IPVERSION — both flags write to the same `transport_family`
    /// variable, so `-4 -6` silently becomes IPv6 rather than erroring.
    #[clap(short = '4', long, overrides_with = "ipv6")]
    pub ipv4: bool,

    /// Force IPv6 connections to the remote.
    ///
    /// Mutually overrides `-4`/`--ipv4` (last wins) — see `-4`.
    #[clap(short = '6', long, overrides_with = "ipv4")]
    pub ipv6: bool,

    /// Repository override (equivalent to the first positional `<repository>`).
    #[clap(long, value_name = "REPOSITORY")]
    pub repo: Option<String>,

    /// The remote to push to; either a named remote or a URL.
    ///
    /// If unset, the upstream of the current branch is used.
    pub repository: Option<String>,

    /// Refspecs to push (e.g. `main`, `main:main`, `+main:upstream`).
    #[clap(value_parser = crate::shared::AsBString)]
    pub refspec: Vec<gix::bstr::BString>,
}
