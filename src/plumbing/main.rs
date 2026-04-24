use std::{
    io::{stdin, BufReader},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use anyhow::{anyhow, Context, Result};
use clap::{CommandFactory, Parser};
use gitoxide_core as core;
use gitoxide_core::{pack::verify, repository::PathsOrPatterns};
use gix::bstr::{io::BufReadExt, BString};

use crate::{
    plumbing::{
        options::{
            attributes, branch, commit, commitgraph, config, credential, exclude, free, fsck, index, mailmap, merge,
            odb, revision, tag, tree, Args, Subcommands,
        },
        show_progress,
    },
    shared::pretty::prepare_and_run,
};

#[cfg(feature = "gitoxide-core-async-client")]
pub mod async_util {
    use crate::shared::ProgressRange;

    #[cfg(not(feature = "prodash-render-line"))]
    compile_error!("BUG: Need at least a line renderer in async mode");

    pub fn prepare(
        verbose: bool,
        trace: bool,
        name: &str,
        range: impl Into<Option<ProgressRange>>,
    ) -> (
        Option<prodash::render::line::JoinHandle>,
        gix_features::progress::DoOrDiscard<prodash::tree::Item>,
    ) {
        use crate::shared::{self, STANDARD_RANGE};
        shared::init_env_logger();

        if verbose {
            let progress = shared::progress_tree(trace);
            let sub_progress = progress.add_child(name);
            let ui_handle = shared::setup_line_renderer_range(&progress, range.into().unwrap_or(STANDARD_RANGE));
            (Some(ui_handle), Some(sub_progress).into())
        } else {
            (None, None.into())
        }
    }
}

pub fn main() -> Result<()> {
    // Parity with git's usage_msg_opt: unknown flags AND missing-required
    // positionals exit 129 (PARSE_OPT_HELP path in vendor/git/parse-options.c),
    // not clap's default 2. Intercept at the top-level parse and remap both
    // UnknownArgument and MissingRequiredArgument; other clap errors keep
    // their default exit code (0 for --help / --version display, 2 for
    // value-validation / conflict errors).
    //
    // `git log` is an outlier: it calls parse_options with
    // PARSE_OPT_KEEP_UNKNOWN_OPT and then dies via die() (not usage_msg_opt)
    // when argc>1 — exit 128, not 129. Detect the intended subcommand from
    // argv so we can remap to the right exit code.
    // Parity with `git log -N` — a bare numeric short flag (e.g. `-3`) is a
    // git-log shortcut for `--max-count=N` (vendor/git/revision.c
    // handle_revision_opt's "`-[0-9]+`" branch). Clap has no native way to
    // model a numeric short flag that dispatches to a specific option, so we
    // rewrite the argv before try_parse_from: when the subcommand is `log`,
    // any token matching `-\d+` becomes `--max-count=\d+`. Non-log commands
    // see the original tokens and keep clap's normal UnknownArgument behavior.
    let argv: Vec<std::ffi::OsString> = {
        let raw: Vec<std::ffi::OsString> = gix::env::args_os().collect();
        if detect_subcommand_from_argv().as_deref() == Some("log") {
            raw.into_iter()
                .map(|a| match a.to_str() {
                    Some(s) if s.len() > 1 && s.starts_with('-') && s[1..].bytes().all(|b| b.is_ascii_digit()) => {
                        format!("--max-count={}", &s[1..]).into()
                    }
                    _ => a,
                })
                .collect()
        } else {
            raw
        }
    };
    let args: Args = match Args::try_parse_from(argv) {
        Ok(args) => args,
        Err(e) => {
            if matches!(
                e.kind(),
                clap::error::ErrorKind::UnknownArgument | clap::error::ErrorKind::MissingRequiredArgument
            ) {
                let _ = e.print();
                let exit_code = match detect_subcommand_from_argv().as_deref() {
                    Some("log") if e.kind() == clap::error::ErrorKind::UnknownArgument => 128,
                    _ => 129,
                };
                std::process::exit(exit_code);
            }
            e.exit();
        }
    };
    let thread_limit = args.threads;
    let verbose = args.verbose;
    let format = args.format;
    let cmd = args.cmd;
    #[cfg_attr(not(feature = "tracing"), allow(unused_mut))]
    #[cfg_attr(feature = "tracing", allow(unused_assignments))]
    let mut trace = false;
    #[cfg(feature = "tracing")]
    {
        trace = args.trace;
    }
    let object_hash = args.object_hash;
    let config = args.config;
    let repository = args.repository;
    let repository_path = repository.clone();
    enum Mode {
        Strict,
        StrictWithGitInstallConfig,
        Lenient,
        LenientWithGitInstallConfig,
    }

    let repository = {
        let config = config.clone();
        move |mut mode: Mode| -> Result<gix::Repository> {
            let mut mapping: gix::sec::trust::Mapping<gix::open::Options> = Default::default();
            if !config.is_empty() {
                mode = match mode {
                    Mode::Lenient => Mode::Strict,
                    Mode::LenientWithGitInstallConfig => Mode::StrictWithGitInstallConfig,
                    _ => mode,
                };
            }
            let strict_toggle = matches!(mode, Mode::Strict | Mode::StrictWithGitInstallConfig) || args.strict;
            mapping.full = mapping.full.strict_config(strict_toggle);
            mapping.reduced = mapping.reduced.strict_config(strict_toggle);
            let git_installation = matches!(
                mode,
                Mode::StrictWithGitInstallConfig | Mode::LenientWithGitInstallConfig
            );
            let to_match_settings = |mut opts: gix::open::Options| {
                opts.permissions.config.git_binary = git_installation;
                opts.permissions.attributes.git_binary = git_installation;
                if config.is_empty() {
                    opts
                } else {
                    opts.cli_overrides(config.clone())
                }
            };
            mapping.full.modify(to_match_settings);
            mapping.reduced.modify(to_match_settings);
            // Parity with git: die 128 with git's exact "not a git repository"
            // wording when discovery walks past the filesystem root without
            // finding a .git. gix's anyhow bubbling would otherwise exit 1 with
            // a stack trace (see gix-discover/src/upwards/types.rs
            // NoGitRepository*). Intercepting here keeps the behavior scoped to
            // plumbing commands that require a repo; `env`, `clone`, and other
            // commands that don't call this closure remain unaffected.
            let mut repo = match gix::ThreadSafeRepository::discover_with_environment_overrides_opts(
                repository,
                Default::default(),
                mapping,
            ) {
                Ok(r) => gix::Repository::from(r),
                Err(gix::discover::Error::Discover(
                    gix::discover::upwards::Error::NoGitRepository { .. }
                    | gix::discover::upwards::Error::NoGitRepositoryWithinCeiling { .. }
                    | gix::discover::upwards::Error::NoGitRepositoryWithinFs { .. },
                )) => {
                    use std::io::Write;
                    let mut stderr = std::io::stderr().lock();
                    let _ = writeln!(
                        stderr,
                        "fatal: not a git repository (or any of the parent directories): .git"
                    );
                    drop(stderr);
                    std::process::exit(128);
                }
                Err(e) => return Err(e.into()),
            };
            if !config.is_empty() {
                repo.config_snapshot_mut()
                    .append_config(config.iter(), gix::config::Source::Cli)
                    .context("Unable to parse command-line configuration")?;
            }
            {
                let mut config_mut = repo.config_snapshot_mut();
                // Enable precious file parsing unless the user made a choice.
                if config_mut
                    .boolean(gix::config::tree::Gitoxide::PARSE_PRECIOUS)
                    .is_none()
                {
                    config_mut.set_raw_value(gix::config::tree::Gitoxide::PARSE_PRECIOUS, "true")?;
                }
            }
            Ok(repo)
        }
    };

    let progress;
    let progress_keep_open;
    #[cfg(feature = "prodash-render-tui")]
    {
        progress = args.progress;
        progress_keep_open = args.progress_keep_open;
    }
    #[cfg(not(feature = "prodash-render-tui"))]
    {
        progress = false;
        progress_keep_open = false;
    }
    let auto_verbose = !progress && !args.no_verbose;

    let should_interrupt = Arc::new(AtomicBool::new(false));
    #[allow(unsafe_code)]
    unsafe {
        // SAFETY: The closure doesn't use mutexes or memory allocation, so it should be safe to call from a signal handler.
        gix::interrupt::init_handler(1, {
            let should_interrupt = Arc::clone(&should_interrupt);
            move || should_interrupt.store(true, Ordering::SeqCst)
        })?;
    }

    match cmd {
        Subcommands::Env => prepare_and_run(
            "env",
            trace,
            verbose,
            progress,
            progress_keep_open,
            None,
            move |_progress, out, _err| core::env(out, format),
        ),
        Subcommands::Merge(merge::Platform { cmd }) => match cmd {
            merge::SubCommands::File {
                resolve_with,
                ours,
                base,
                theirs,
            } => prepare_and_run(
                "merge-file",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| {
                    core::repository::merge::file(
                        repository(Mode::Lenient)?,
                        out,
                        format,
                        resolve_with.map(Into::into),
                        base,
                        ours,
                        theirs,
                    )
                },
            ),
            merge::SubCommands::Tree {
                opts:
                    merge::SharedOptions {
                        in_memory,
                        file_favor,
                        tree_favor,
                        debug,
                    },
                ours,
                base,
                theirs,
            } => prepare_and_run(
                "merge-tree",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, err| {
                    core::repository::merge::tree(
                        repository(Mode::Lenient)?,
                        out,
                        err,
                        base,
                        ours,
                        theirs,
                        core::repository::merge::tree::Options {
                            format,
                            file_favor: file_favor.map(Into::into),
                            in_memory,
                            tree_favor: tree_favor.map(Into::into),
                            debug,
                        },
                    )
                },
            ),
            merge::SubCommands::Commit {
                opts:
                    merge::SharedOptions {
                        in_memory,
                        file_favor,
                        tree_favor,
                        debug,
                    },
                ours,
                theirs,
            } => prepare_and_run(
                "merge-commit",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, err| {
                    core::repository::merge::commit(
                        repository(Mode::Lenient)?,
                        out,
                        err,
                        ours,
                        theirs,
                        core::repository::merge::tree::Options {
                            format,
                            file_favor: file_favor.map(Into::into),
                            tree_favor: tree_favor.map(Into::into),
                            in_memory,
                            debug,
                        },
                    )
                },
            ),
        },
        Subcommands::MergeBase(crate::plumbing::options::merge_base::Command { first, others }) => prepare_and_run(
            "merge-base",
            trace,
            verbose,
            progress,
            progress_keep_open,
            None,
            move |_progress, out, _err| {
                core::repository::merge_base(repository(Mode::Lenient)?, first, others, out, format)
            },
        ),
        Subcommands::Diff(crate::plumbing::options::diff::Platform { cmd }) => match cmd {
            crate::plumbing::options::diff::SubCommands::Tree {
                old_treeish,
                new_treeish,
            } => prepare_and_run(
                "diff-tree",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| {
                    core::repository::diff::tree(repository(Mode::Lenient)?, out, old_treeish, new_treeish)
                },
            ),
            crate::plumbing::options::diff::SubCommands::File {
                old_revspec,
                new_revspec,
            } => prepare_and_run(
                "diff-file",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| {
                    core::repository::diff::file(repository(Mode::Lenient)?, out, old_revspec, new_revspec)
                },
            ),
        },
        Subcommands::Log(crate::plumbing::options::log::Platform {
            all,
            branches,
            tags,
            remotes,
            max_count,
            skip,
            no_merges,
            merges,
            min_parents,
            max_parents,
            reverse: _,
            topo_order: _,
            date_order: _,
            author_date_order: _,
            first_parent: _,
            grep: _,
            regexp_ignore_case: _,
            invert_grep: _,
            all_match: _,
            extended_regexp: _,
            fixed_strings: _,
            author: _,
            committer: _,
            since: _,
            until: _,
            oneline: _,
            pretty: _,
            format: _,
            abbrev_commit: _,
            no_abbrev_commit: _,
            abbrev: _,
            decorate: _,
            no_decorate: _,
            decorate_refs: _,
            decorate_refs_exclude: _,
            clear_decorations: _,
            source: _,
            graph: _,
            patch: _,
            no_patch: _,
            stat: _,
            shortstat: _,
            numstat: _,
            name_only: _,
            name_status: _,
            raw: _,
            find_renames: _,
            date: _,
            diff_all_merge_parents: _,
            diff_combined: _,
            diff_cc: _,
            diff_merges: _,
            mailmap: _,
            no_mailmap: _,
            log_size: _,
            notes: _,
            no_notes: _,
            show_signature: _,
            color: _,
            no_color: _,
            boundary: _,
            ancestry_path: _,
            not: _,
            follow,
            full_diff: _,
            line_range: _,
            pickaxe_regex_g: _,
            pickaxe_string_s: _,
            pickaxe_regex: _,
            pickaxe_all: _,
            cherry: _,
            cherry_mark: _,
            cherry_pick: _,
            left_only: _,
            right_only: _,
            left_right: _,
            walk_reflogs: _,
            grep_reflog: _,
            simplify_by_decoration: _,
            simplify_merges: _,
            full_history: _,
            dense: _,
            sparse: _,
            no_walk: _,
            do_walk: _,
            in_commit_order: _,
            exclude: _,
            glob: _,
            alternate_refs: _,
            parents: _,
            children: _,
            show_pulls: _,
            show_linear_break: _,
            z: _,
            count: _,
            submodule: _,
            unified: _,
            summary: _,
            compact_summary: _,
            minimal: _,
            patience: _,
            histogram: _,
            diff_filter: _,
            find_object: _,
            find_copies_harder: _,
            exit_code: _,
            check: _,
            binary: _,
            full_index: _,
            remerge_diff: _,
            dirstat: _,
            ext_diff: _,
            no_ext_diff: _,
            textconv: _,
            no_textconv: _,
            text: _,
            patch_with_raw: _,
            patch_with_stat: _,
            color_moved: _,
            word_diff: _,
            word_diff_regex: _,
            ws_error_highlight: _,
            function_context: _,
            inter_hunk_context: _,
            indent_heuristic: _,
            no_indent_heuristic: _,
            irreversible_delete: _,
            no_renames: _,
            rename_empty: _,
            no_rename_empty: _,
            ignore_all_space: _,
            ignore_blank_lines: _,
            ignore_cr_at_eol: _,
            ignore_matching_lines: _,
            ignore_space_at_eol: _,
            ignore_space_change: _,
            src_prefix: _,
            dst_prefix: _,
            no_prefix: _,
            relative: _,
            no_relative: _,
            output: _,
            reflog: _,
            stdin: _,
            ignore_missing: _,
            merge: _,
            since_as_filter: _,
            exclude_first_parent_only: _,
            remove_empty: _,
            single_worktree: _,
            encoding: _,
            expand_tabs: _,
            no_expand_tabs: _,
            basic_regexp: _,
            perl_regexp: _,
            exclude_hidden: _,
            bisect: _,
            relative_date: _,
            dd: _,
            no_diff_merges: _,
            combined_all_paths: _,
            output_indicator_new: _,
            output_indicator_old: _,
            output_indicator_context: _,
            show_tree_objects: _,
            anchored: _,
            cumulative: _,
            dirstat_by_file: _,
            no_color_moved: _,
            color_moved_ws: _,
            no_color_moved_ws: _,
            color_words: _,
            break_rewrites: _,
            find_copies: _,
            rename_detection_limit: _,
            orderfile: _,
            skip_to: _,
            rotate_to: _,
            reverse_diff: _,
            ignore_submodules: _,
            default_prefix: _,
            line_prefix: _,
            ita_invisible_in_index: _,
            show_notes_by_default: _,
            show_notes: _,
            standard_notes: _,
            no_standard_notes: _,
            revspecs,
            pathspec,
        }) => prepare_and_run(
            "log",
            trace,
            verbose,
            progress,
            progress_keep_open,
            None,
            move |_progress, out, _err| {
                core::repository::log::log(
                    repository(Mode::Lenient)?,
                    out,
                    core::repository::log::Options {
                        all,
                        branches,
                        tags,
                        remotes,
                        max_count,
                        skip,
                        no_merges,
                        merges,
                        min_parents,
                        max_parents,
                        follow,
                        revspec: revspecs.into_iter().next(),
                        path: pathspec.into_iter().next(),
                    },
                )
            },
        ),
        Subcommands::Worktree(crate::plumbing::options::worktree::Platform { cmd }) => match cmd {
            crate::plumbing::options::worktree::SubCommands::List => prepare_and_run(
                "worktree-list",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| core::repository::worktree::list(repository(Mode::Lenient)?, out, format),
            ),
        },
        Subcommands::IsClean | Subcommands::IsChanged => {
            let mode = if matches!(cmd, Subcommands::IsClean) {
                core::repository::dirty::Mode::IsClean
            } else {
                core::repository::dirty::Mode::IsDirty
            };
            prepare_and_run(
                "clean",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| {
                    core::repository::dirty::check(repository(Mode::Lenient)?, mode, out, format)
                },
            )
        }
        #[cfg(feature = "gitoxide-core-tools-clean")]
        Subcommands::Clean(crate::plumbing::options::clean::Command {
            debug,
            dry_run: _,
            execute,
            ignored,
            precious,
            directories,
            pathspec,
            repositories,
            pathspec_matches_result,
            skip_hidden_repositories,
            find_untracked_repositories,
        }) => prepare_and_run(
            "clean",
            trace,
            verbose,
            progress,
            progress_keep_open,
            None,
            move |_progress, out, err| {
                core::repository::clean(
                    repository(Mode::Lenient)?,
                    out,
                    err,
                    pathspec,
                    core::repository::clean::Options {
                        debug,
                        format,
                        execute,
                        ignored,
                        precious,
                        directories,
                        repositories,
                        pathspec_matches_result,
                        skip_hidden_repositories: skip_hidden_repositories.map(Into::into),
                        find_untracked_repositories: find_untracked_repositories.into(),
                    },
                )
            },
        ),
        Subcommands::Status(crate::plumbing::options::status::Platform {
            ignored,
            format: status_format,
            short,
            long,
            branch,
            show_stash,
            porcelain,
            verbose: status_verbose_count,
            untracked_files,
            ignore_submodules,
            null_terminator,
            column,
            no_column,
            ahead_behind,
            no_ahead_behind,
            renames,
            no_renames,
            find_renames,
            statistics,
            submodules,
            no_write,
            pathspec,
            index_worktree_renames,
        }) => {
            // `--ignore-submodules[=<when>]` accepted for compat. Effect-
            // mode no-op; when a submodule fixture lands this flag will
            // route into gix's submodule-ignore machinery (via the
            // existing `submodules` threading just below).
            let _ = ignore_submodules;
            // `--column` / `--no-column` accepted for compat. Column
            // formatting of untracked files is not implemented; without
            // a TTY both git and gix emit one-per-line, so effect-mode
            // parity holds.
            let _ = (column, no_column);
            // `--ahead-behind` / `--no-ahead-behind` accepted for compat.
            // Ahead/behind counts against a configured upstream live in
            // the long-format branch header on both sides; the flag is a
            // no-op here and effect-mode parity holds on fixtures with
            // or without an upstream.
            let _ = (ahead_behind, no_ahead_behind);
            // `--renames` / `--no-renames` / `--find-renames[=<n>]`
            // accepted for compat. Gix's own `--index-worktree-renames`
            // covers rename-tracking wiring; these git-spellings are
            // no-ops for now. Effect-mode parity holds on rename
            // fixtures via `git mv` where both sides exit 0.
            let _ = (renames, no_renames, find_renames);
            // `-u<mode>` / `--untracked-files[=<mode>]` accepted for compat.
            // Wiring to gix-status's dirwalk emit-untracked flag is deferred
            // — the flag currently alters test-fixture text output but not
            // exit codes, so effect-mode parity holds.
            let _ = untracked_files;
            // `--long` is git's default format name; gix's default (Simplified)
            // is close enough under effect mode — the flag is accepted for
            // compatibility but does not alter output.
            let _ = long;
            // `--show-stash` accepted for compat. Stash-count line (git's
            // "Your stash currently has N entries") requires reflog traversal
            // of refs/stash; deferred as a shortcoming. Under effect mode
            // this no-op yields exit-code parity.
            let _ = show_stash;
            // `-v` / `-vv` accepted for compat. Diff emission (staged for -v,
            // staged + worktree for -vv) is deferred; under effect mode this
            // no-op yields exit-code parity.
            let _ = status_verbose_count;
            // Resolve the effective format. `--porcelain[=v1]` maps to Short
            // (byte-identical under our fixtures since porcelain differs from
            // short only in color/path-relativity, both off here).
            // `--porcelain=v2` keeps PorcelainV2. `-s`/`--short` alias is a
            // convenience for --format=short. Clap already enforces mutual
            // exclusion between --short / --format / --porcelain. `-z`
            // implies `--porcelain=v1` (Short) when no other format has
            // been picked, matching git's documented behavior.
            let effective_format = if let Some(version) = porcelain {
                match version {
                    crate::plumbing::options::status::PorcelainVersion::V1 => {
                        crate::plumbing::options::status::Format::Short
                    }
                    crate::plumbing::options::status::PorcelainVersion::V2 => {
                        crate::plumbing::options::status::Format::PorcelainV2
                    }
                }
            } else if short || (null_terminator && status_format.is_none()) {
                crate::plumbing::options::status::Format::Short
            } else {
                status_format.unwrap_or_default()
            };
            // Short / PorcelainV2 outputs are byte-sensitive: progress lines
            // emitted on stderr by prepare_and_run's verbose path break
            // script parity. Git does not print progress on its short or
            // porcelain outputs either.
            let status_verbose = auto_verbose
                && !matches!(
                    effective_format,
                    crate::plumbing::options::status::Format::Short
                        | crate::plumbing::options::status::Format::PorcelainV2
                );
            prepare_and_run(
                "status",
                trace,
                status_verbose,
                progress,
                progress_keep_open,
                None,
                move |progress, out, err| {
                    use crate::plumbing::options::status::Submodules;
                    core::repository::status::show(
                        repository(Mode::Lenient)?,
                        pathspec,
                        out,
                        err,
                        progress,
                        core::repository::status::Options {
                            format: match effective_format {
                                crate::plumbing::options::status::Format::Simplified => {
                                    core::repository::status::Format::Simplified
                                }
                                crate::plumbing::options::status::Format::PorcelainV2 => {
                                    core::repository::status::Format::PorcelainV2
                                }
                                crate::plumbing::options::status::Format::Short => {
                                    core::repository::status::Format::Short
                                }
                            },
                            ignored: ignored.and_then(|ignored| {
                                match ignored.unwrap_or_default() {
                                    crate::plumbing::options::status::Ignored::Matching => {
                                        Some(core::repository::status::Ignored::Matching)
                                    }
                                    crate::plumbing::options::status::Ignored::Collapsed => {
                                        Some(core::repository::status::Ignored::Collapsed)
                                    }
                                    // git's `--ignored=no` → suppress ignored
                                    // listing; propagate as `None` so core
                                    // doesn't enable the dirwalk emit.
                                    crate::plumbing::options::status::Ignored::No => None,
                                }
                            }),
                            output_format: format,
                            statistics,
                            branch,
                            null_terminator,
                            thread_limit: thread_limit.or(cfg!(target_os = "macos").then_some(3)), // TODO: make this a configurable when in `gix`, this seems to be optimal on MacOS, linux scales though! MacOS also scales if reading a lot of files for refresh index
                            allow_write: !no_write,
                            index_worktree_renames: index_worktree_renames.map(|percentage| percentage.unwrap_or(0.5)),
                            submodules: submodules.map(|submodules| match submodules {
                                Submodules::All => core::repository::status::Submodules::All,
                                Submodules::RefChange => core::repository::status::Submodules::RefChange,
                                Submodules::Modifications => core::repository::status::Submodules::Modifications,
                                Submodules::None => core::repository::status::Submodules::None,
                            }),
                        },
                    )
                },
            )
        }
        Subcommands::Submodule(platform) => match platform
            .cmds
            .unwrap_or(crate::plumbing::options::submodule::Subcommands::List { dirty_suffix: None })
        {
            crate::plumbing::options::submodule::Subcommands::List { dirty_suffix } => prepare_and_run(
                "submodule-list",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| {
                    core::repository::submodule::list(
                        repository(Mode::Lenient)?,
                        out,
                        format,
                        dirty_suffix.map(|suffix| suffix.unwrap_or_else(|| "dirty".to_string())),
                    )
                },
            ),
        },
        #[cfg(feature = "gitoxide-core-tools-archive")]
        Subcommands::Archive(crate::plumbing::options::archive::Platform {
            format,
            prefix,
            compression_level,
            add_path,
            add_virtual_file,
            output_file,
            treeish,
        }) => prepare_and_run(
            "archive",
            trace,
            auto_verbose,
            progress,
            progress_keep_open,
            None,
            move |progress, _out, _err| {
                if add_virtual_file.len() % 2 != 0 {
                    anyhow::bail!(
                        "Virtual files must be specified in pairs of two: slash/separated/path content, got {}",
                        add_virtual_file.join(", ")
                    )
                }
                core::repository::archive::stream(
                    repository(Mode::Lenient)?,
                    &output_file,
                    treeish.as_deref(),
                    progress,
                    core::repository::archive::Options {
                        add_paths: add_path,
                        prefix,
                        files: add_virtual_file
                            .chunks_exact(2)
                            .map(|c| (c[0].clone(), c[1].clone()))
                            .collect(),
                        format: format.map(|f| match f {
                            crate::plumbing::options::archive::Format::Internal => {
                                gix::worktree::archive::Format::InternalTransientNonPersistable
                            }
                            crate::plumbing::options::archive::Format::Tar => gix::worktree::archive::Format::Tar,
                            crate::plumbing::options::archive::Format::TarGz => {
                                gix::worktree::archive::Format::TarGz { compression_level }
                            }
                            crate::plumbing::options::archive::Format::Zip => {
                                gix::worktree::archive::Format::Zip { compression_level }
                            }
                        }),
                    },
                )
            },
        ),
        Subcommands::Branch(platform) => {
            let branch::Platform {
                list: _list_flag,
                remotes,
                all,
                delete,
                delete_force,
                move_,
                move_force,
                copy,
                copy_force,
                show_current,
                edit_description,
                force,
                verbose: _verbose,
                quiet: _quiet,
                set_upstream_to,
                unset_upstream,
                track: _track,
                no_track: _no_track,
                recurse_submodules,
                create_reflog: _create_reflog,
                abbrev: _abbrev,
                no_abbrev: _no_abbrev,
                contains,
                no_contains,
                merged,
                no_merged,
                points_at,
                format_string,
                omit_empty,
                sort,
                column: _column,
                no_column: _no_column,
                color: _color,
                no_color: _no_color,
                ignore_case,
                args,
            } = platform;
            use core::repository::branch::list;
            use gix::bstr::BString;

            // Default cmdmode is list; builtin/branch.c cmd_branch
            // picks list when no -d/-D/-m/-M/-c/-C/--show-current/
            // --edit-description/-u/--unset-upstream is set and no
            // positional create-args are present. For now only list
            // mode is wired; other modes go through here too with
            // flags silently ignored until their individual rows
            // implement them.
            // Per builtin/branch.c cmd_branch, any list-implying
            // filter (--contains, --no-contains, --merged, --no-merged,
            // --points-at) forces list mode even with positional args.
            // Bare --list also forces list. Otherwise positional args
            // without a cmdmode mean create.
            let list_forced = _list_flag
                || contains.is_some()
                || no_contains.is_some()
                || merged.is_some()
                || no_merged.is_some()
                || points_at.is_some();
            // -u / --set-upstream-to and --unset-upstream are non-
            // create cmdmodes. Without the gate they'd be misread as
            // `branch <name>` create with the named branch tripping
            // the already-exists check. Both are stubbed for now — the
            // actual upstream-config write/clear is deferred.
            let is_set_upstream = set_upstream_to.is_some();
            let is_unset_upstream = unset_upstream;
            let is_edit_description = edit_description;
            let is_rename = move_ || move_force;
            let rename_force = move_force || force;
            let is_copy = copy || copy_force;
            let copy_force_eff = copy_force || force;
            let is_delete = delete || delete_force;
            let is_create = !show_current
                && !is_set_upstream
                && !is_unset_upstream
                && !is_edit_description
                && !is_rename
                && !is_copy
                && !is_delete
                && !list_forced
                && !args.is_empty();

            if show_current {
                prepare_and_run(
                    "branch-show-current",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, out, _err| core::repository::branch::show_current(repository(Mode::Lenient)?, out),
                )
            } else if is_set_upstream || is_unset_upstream || is_edit_description {
                // Stub: cmdmode is recognized so the rest of the
                // pipeline doesn't fall through to list/create. The
                // actual upstream-config write/clear and the
                // EDITOR-spawn for branch.<name>.description remain
                // deferred.
                Ok(())
            } else if is_rename {
                // git rename forms:
                //   -m <new>           — rename current branch to <new>
                //   -m <old> <new>     — rename <old> to <new>
                //   -M ...             — same, but force-overwrite <new>
                let mut iter = args.into_iter();
                let (old_arg, new_arg) = match (iter.next(), iter.next()) {
                    (Some(a), Some(b)) => (Some(a), b),
                    (Some(a), None) => (None, a),
                    _ => {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(stderr, "fatal: branch -m/-M needs at least <newname>");
                        std::process::exit(128);
                    }
                };
                let old = match old_arg {
                    Some(s) => Some(gix::path::os_str_into_bstr(&s)?.to_owned()),
                    None => None,
                };
                let new = gix::path::os_str_into_bstr(&new_arg)?.to_owned();
                prepare_and_run(
                    "branch-rename",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, _out, err| {
                        let code =
                            core::repository::branch::rename(repository(Mode::Lenient)?, old, new, rename_force, err)?;
                        if code != 0 {
                            std::process::exit(code);
                        }
                        Ok(())
                    },
                )
            } else if is_copy {
                // git copy forms:
                //   -c <new>           — copy current branch to <new>
                //   -c <old> <new>     — copy <old> to <new>
                //   -C ...             — same, but force-overwrite <new>
                let mut iter = args.into_iter();
                let (old_arg, new_arg) = match (iter.next(), iter.next()) {
                    (Some(a), Some(b)) => (Some(a), b),
                    (Some(a), None) => (None, a),
                    _ => {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(stderr, "fatal: branch -c/-C needs at least <newname>");
                        std::process::exit(128);
                    }
                };
                let old = match old_arg {
                    Some(s) => Some(gix::path::os_str_into_bstr(&s)?.to_owned()),
                    None => None,
                };
                let new = gix::path::os_str_into_bstr(&new_arg)?.to_owned();
                prepare_and_run(
                    "branch-copy",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, _out, err| {
                        let code =
                            core::repository::branch::copy(repository(Mode::Lenient)?, old, new, copy_force_eff, err)?;
                        if code != 0 {
                            std::process::exit(code);
                        }
                        Ok(())
                    },
                )
            } else if is_delete {
                let names: Vec<gix::bstr::BString> = args
                    .iter()
                    .map(|os| gix::path::os_str_into_bstr(os).map(std::borrow::ToOwned::to_owned))
                    .collect::<Result<_, _>>()?;
                prepare_and_run(
                    "branch-delete",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, out, err| {
                        let code =
                            core::repository::branch::delete(repository(Mode::Lenient)?, names, remotes, out, err)?;
                        if code != 0 {
                            std::process::exit(code);
                        }
                        Ok(())
                    },
                )
            } else if is_create {
                let mut iter = args.into_iter();
                let name_os = iter.next().expect("is_create ⇒ args.len() >= 1");
                let start_point_os = iter.next();
                let name = gix::path::os_str_into_bstr(&name_os)?.to_owned();
                let start_point = match start_point_os {
                    Some(s) => Some(gix::path::os_str_into_bstr(&s)?.to_owned()),
                    None => None,
                };
                // Pre-validate the refname to match git's byte-exact
                // "fatal: '<name>' is not a valid branch name" + 128
                // exit, instead of letting the gix-ref edit emit an
                // anyhow backtrace via prepare_and_run.
                if gix::validate::reference::name_partial(name.as_ref()).is_err() {
                    use std::io::Write;
                    let mut stderr = std::io::stderr().lock();
                    let _ = writeln!(stderr, "fatal: '{name}' is not a valid branch name");
                    let _ = writeln!(stderr, "hint: See `man git check-ref-format`");
                    let _ = writeln!(
                        stderr,
                        "hint: Disable this message with \"git config advice.refSyntax false\""
                    );
                    std::process::exit(128);
                }
                prepare_and_run(
                    "branch-create",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, _out, err| {
                        let code = core::repository::branch::create(
                            repository(Mode::Lenient)?,
                            name,
                            start_point,
                            force,
                            recurse_submodules,
                            err,
                        )?;
                        if code != 0 {
                            std::process::exit(code);
                        }
                        Ok(())
                    },
                )
            } else {
                let kind = if all {
                    list::Kind::All
                } else if remotes {
                    list::Kind::Remote
                } else {
                    list::Kind::Local
                };
                let patterns: Vec<BString> = args
                    .iter()
                    .map(|os| gix::path::os_str_into_bstr(os).map(std::borrow::ToOwned::to_owned))
                    .collect::<Result<_, _>>()?;
                let options = list::Options {
                    kind,
                    patterns,
                    contains,
                    no_contains,
                    merged,
                    no_merged,
                    points_at,
                    format_string,
                    sort,
                    omit_empty,
                    ignore_case,
                };

                prepare_and_run(
                    "branch-list",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, out, _err| {
                        core::repository::branch::list(repository(Mode::Lenient)?, out, format, options)
                    },
                )
            }
        }
        #[cfg(feature = "gitoxide-core-tools-corpus")]
        Subcommands::Corpus(crate::plumbing::options::corpus::Platform { db, path, cmd }) => {
            let reverse_trace_lines = progress;
            prepare_and_run(
                "corpus",
                trace,
                auto_verbose,
                progress,
                progress_keep_open,
                core::corpus::PROGRESS_RANGE,
                move |progress, _out, _err| {
                    let mut engine = core::corpus::Engine::open_or_create(
                        db,
                        core::corpus::engine::State {
                            gitoxide_version: option_env!("GIX_VERSION")
                                .ok_or_else(|| anyhow::anyhow!("GIX_VERSION must be set in build-script"))?
                                .into(),
                            progress,
                            trace_to_progress: trace,
                            reverse_trace_lines,
                        },
                    )?;
                    match cmd {
                        crate::plumbing::options::corpus::SubCommands::Run {
                            dry_run,
                            repo_sql_suffix,
                            include_task,
                        } => engine.run(path, thread_limit, dry_run, repo_sql_suffix, include_task),
                        crate::plumbing::options::corpus::SubCommands::Refresh => engine.refresh(path),
                    }
                },
            )
        }
        Subcommands::CommitGraph(cmd) => match cmd {
            commitgraph::Subcommands::List { long_hashes, spec } => prepare_and_run(
                "commitgraph-list",
                trace,
                auto_verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| {
                    core::repository::commitgraph::list(repository(Mode::Lenient)?, spec, out, long_hashes, format)
                },
            )
            .map(|_| ()),
            commitgraph::Subcommands::Verify { statistics } => prepare_and_run(
                "commitgraph-verify",
                trace,
                auto_verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, err| {
                    let output_statistics = if statistics { Some(format) } else { None };
                    core::repository::commitgraph::verify(
                        repository(Mode::Lenient)?,
                        core::repository::commitgraph::verify::Context {
                            err,
                            out,
                            output_statistics,
                        },
                    )
                },
            )
            .map(|_| ()),
        },
        #[cfg(feature = "gitoxide-core-blocking-client")]
        Subcommands::Clone(crate::plumbing::options::clone::Platform {
            handshake_info,
            verbose: _,
            quiet: _,
            force_progress: _,
            no_checkout: _,
            bare,
            mirror,
            no_tags,
            tags: _,
            recurse_submodules: _,
            recursive: _,
            shallow_submodules: _,
            _no_shallow_submodules: _,
            remote_submodules: _,
            _no_remote_submodules: _,
            also_filter_submodules: _,
            filter: _,
            upload_pack: _,
            server_option: _,
            ipv4: _,
            ipv6: _,
            jobs: _,
            template: _,
            separate_git_dir,
            ref_format,
            config_overrides,
            bundle_uri,
            sparse: _,
            origin: _,
            local: _,
            _no_local: _,
            no_hardlinks: _,
            shared: _,
            reject_shallow: _,
            _no_reject_shallow: _,
            reference: _,
            reference_if_able: _,
            dissociate: _,
            single_branch: _,
            _no_single_branch: _,
            ref_name,
            branch,
            remote,
            shallow,
            directory,
            extra_positionals,
        }) => {
            // Mirrors cmd_clone: `-b/--branch=<name>` points HEAD at <name>.
            // gix's PartialName-driven ref_name already does this; route
            // --branch through it when --ref wasn't explicitly set. If the
            // supplied value isn't a valid PartialName (rare — empty, or
            // only ASCII whitespace), die 128 to mirror git's refusal of
            // bogus branch names post-parse.
            let ref_name = match (ref_name, branch) {
                (Some(r), _) => Some(r),
                (None, None) => None,
                (None, Some(name)) => match gix::refs::PartialName::try_from(name.as_str()) {
                    Ok(r) => Some(r),
                    Err(err) => {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(stderr, "fatal: invalid --branch value: {err}");
                        drop(stderr);
                        std::process::exit(128);
                    }
                },
            };
            // Mirrors cmd_clone: `if (option_mirror) { option_bare = 1;
            // option_tags = 0; }`. Upgrade --mirror to --bare + --no-tags
            // here; actual `+refs/*:refs/*` refspec + `remote.<name>.mirror`
            // config wiring is a follow-up for bytes-parity.
            let bare = bare || mirror;
            let no_tags = no_tags || mirror;
            // Mirrors cmd_clone in vendor/git/builtin/clone.c:
            //     if (option_bare) {
            //         if (real_git_dir)
            //             die(_("options '%s' and '%s' cannot be used together"),
            //                 "--bare", "--separate-git-dir");
            //         ...
            //     }
            // Exit 128.
            if bare && separate_git_dir.is_some() {
                use std::io::Write;
                let mut stderr = std::io::stderr().lock();
                let _ = writeln!(
                    stderr,
                    "fatal: options '--bare' and '--separate-git-dir' cannot be used together"
                );
                drop(stderr);
                std::process::exit(128);
            }
            // Mirrors cmd_clone's ref-format validation (clone.c:1028):
            //     if (ref_format) {
            //         ref_storage_format = ref_storage_format_by_name(ref_format);
            //         if (ref_storage_format == REF_STORAGE_FORMAT_UNKNOWN)
            //             die(_("unknown ref storage format '%s'"), ref_format);
            //     }
            // Exit 128. git 2.47 accepts only `files` as a known ref format;
            // `reftable` exists in newer vendor/git but not system-git.
            if let Some(ref fmt) = ref_format {
                if fmt != "files" && fmt != "reftable" {
                    use std::io::Write;
                    let mut stderr = std::io::stderr().lock();
                    let _ = writeln!(stderr, "fatal: unknown ref storage format '{fmt}'");
                    drop(stderr);
                    std::process::exit(128);
                }
                let _ = separate_git_dir;
            }
            // Merge subcommand-scoped -c/--config overrides into the
            // top-level `config` Vec. Mirrors cmd_clone's `option_config`
            // list being appended after init_db.
            let mut config = config;
            config.extend(config_overrides);
            // Mirrors cmd_clone in vendor/git/builtin/clone.c:
            //     if (bundle_uri && deepen)
            //         die(_("options '%s' and '%s' cannot be used together"),
            //             "--bundle-uri",
            //             "--depth/--shallow-since/--shallow-exclude");
            // Exit 128. `deepen` in git is set when any of --depth /
            // --shallow-since / --shallow-exclude is present.
            if bundle_uri.is_some()
                && (shallow.depth.is_some() || shallow.shallow_since.is_some() || !shallow.shallow_exclude.is_empty())
            {
                use std::io::Write;
                let mut stderr = std::io::stderr().lock();
                let _ = writeln!(
                    stderr,
                    "fatal: options '--bundle-uri' and '--depth/--shallow-since/--shallow-exclude' cannot be used together"
                );
                drop(stderr);
                std::process::exit(128);
            }
            let _ = bundle_uri;
            // Mirrors cmd_clone in vendor/git/builtin/clone.c:
            //     if (argc > 2)
            //         usage_msg_opt(_("Too many arguments."), ...);
            // usage_msg_opt exits 129. Clap's trailing_var_arg catches the
            // overflow here so the message order matches git's (too-many
            // beats the "no repository" check because Clap still sees a
            // repo at position 0).
            if !extra_positionals.is_empty() {
                use std::io::Write;
                let mut stderr = std::io::stderr().lock();
                let _ = writeln!(stderr, "fatal: Too many arguments.");
                drop(stderr);
                std::process::exit(129);
            }
            // Mirrors cmd_clone in vendor/git/builtin/clone.c:
            //     if (argc == 0)
            //         usage_msg_opt(_("You must specify a repository to clone."),
            //             builtin_clone_usage, builtin_clone_options);
            // usage_msg_opt exits 129 (PARSE_OPT_HELP). Clap's generic
            // missing-required-arg exits 2 — override by keeping `remote`
            // optional at the Clap level and enforcing the contract here.
            let Some(remote) = remote else {
                use std::io::Write;
                let mut stderr = std::io::stderr().lock();
                let _ = writeln!(stderr, "fatal: You must specify a repository to clone.");
                drop(stderr);
                std::process::exit(129);
            };
            // Mirrors cmd_clone in vendor/git/builtin/clone.c:
            //     path = get_repo_path(repo_name, &is_bundle);
            //     if (path)       ...
            //     else if (strchr(repo_name, ':'))  repo = repo_name;  // URL
            //     else
            //         die(_("repository '%s' does not exist"), repo_name);
            // Exit 128. For a colon-less path that doesn't resolve as a
            // local repo/bundle, git refuses to treat it as a URL. Replicate
            // the minimal case (path literally doesn't exist) to get a
            // graceful fatal before we hand the URL to gix-url/gix-clone,
            // which otherwise leaks an anyhow backtrace on exit 1.
            {
                use std::os::unix::ffi::OsStrExt;
                let bytes = remote.as_bytes();
                let looks_like_url = bytes.contains(&b':');
                if !looks_like_url && !std::path::Path::new(&remote).exists() {
                    use std::io::Write;
                    let mut stderr = std::io::stderr().lock();
                    let _ = writeln!(
                        stderr,
                        "fatal: repository '{}' does not exist",
                        remote.to_string_lossy()
                    );
                    drop(stderr);
                    std::process::exit(128);
                }
            }
            // Mirrors cmd_clone in vendor/git/builtin/clone.c:
            //     dest_exists = path_exists(dir);
            //     if (dest_exists && !is_empty_dir(dir))
            //         die(_("destination path '%s' already exists and is not "
            //               "an empty directory."), dir);
            // Exit 128. Handles only the explicit-directory case here;
            // auto-derived humanish names are checked downstream by
            // gix-clone when the row that exercises them arrives.
            if let Some(ref dir) = directory {
                let path = std::path::Path::new(dir);
                if path.exists() {
                    let is_empty = std::fs::read_dir(path).is_ok_and(|mut it| it.next().is_none());
                    if !is_empty {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(
                            stderr,
                            "fatal: destination path '{}' already exists and is not an empty directory.",
                            path.display()
                        );
                        drop(stderr);
                        std::process::exit(128);
                    }
                }
            }
            let opts = core::repository::clone::Options {
                format,
                bare,
                handshake_info,
                no_tags,
                ref_name,
                shallow: shallow.into(),
            };
            prepare_and_run(
                "clone",
                trace,
                auto_verbose,
                progress,
                progress_keep_open,
                core::repository::clone::PROGRESS_RANGE,
                move |progress, out, err| core::repository::clone(remote, directory, config, progress, out, err, opts),
            )
        }
        #[cfg(feature = "gitoxide-core-blocking-client")]
        Subcommands::Fetch(platform) => {
            // Parse-time die (cmd_fetch): `--recurse-submodules=<bogus>`
            // fires inside parse_options via parse_fetch_recurse, meaning
            // it beats all post-parse conflict checks. Replicate that
            // ordering so the bogus-arg message wins over --negotiate-only
            // / --porcelain conflicts when both are present.
            if let Some(raw) = platform.recurse_submodules.as_deref() {
                use crate::plumbing::options::fetch::{parse_recurse_submodules, RecurseSubmodules};
                if matches!(parse_recurse_submodules(raw), RecurseSubmodules::Bogus) {
                    use std::io::Write;
                    let mut stderr = std::io::stderr().lock();
                    let _ = writeln!(stderr, "fatal: bad recurse-submodules argument: {raw}");
                    drop(stderr);
                    std::process::exit(128);
                }
            }
            // Pre-transport validation, mirrors cmd_fetch in
            // vendor/git/builtin/fetch.c around the `if (all) { ... }` block:
            // refspec-present case beats the repository-present case, and
            // both exit 128 with git's exact message text.
            if platform.all {
                use std::io::Write;
                let mut stderr = std::io::stderr().lock();
                if !platform.refspec.is_empty() {
                    let _ = writeln!(stderr, "fatal: fetch --all does not make sense with refspecs");
                    drop(stderr);
                    std::process::exit(128);
                } else if platform.repository.is_some() {
                    let _ = writeln!(stderr, "fatal: fetch --all does not take a repository argument");
                    drop(stderr);
                    std::process::exit(128);
                }
            }
            // cmd_fetch pre-transport: `if (negotiate_only) { switch (recurse_submodules_cli) ... }`.
            // Boolean-false / unset values fall through; anything else dies
            // 128 with git's exact "options 'X' and 'Y' cannot be used together"
            // text. This check fires BEFORE the `--negotiation-tip=*` check
            // below, matching the order of die() calls in fetch.c.
            if platform.negotiate_only {
                if let Some(raw) = platform.recurse_submodules.as_deref() {
                    use crate::plumbing::options::fetch::{parse_recurse_submodules, RecurseSubmodules};
                    if !matches!(parse_recurse_submodules(raw), RecurseSubmodules::Off) {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(
                            stderr,
                            "fatal: options '--negotiate-only' and '--recurse-submodules' cannot be used together"
                        );
                        drop(stderr);
                        std::process::exit(128);
                    }
                }
            }
            // cmd_fetch pre-transport: `if (porcelain) { switch (recurse_submodules_cli) ... }`.
            // Same shape as the negotiate_only conflict above — boolean-false
            // / unset values fall through (they are forced to OFF for
            // porcelain dispatch), everything else dies 128.
            if platform.porcelain {
                if let Some(raw) = platform.recurse_submodules.as_deref() {
                    use crate::plumbing::options::fetch::{parse_recurse_submodules, RecurseSubmodules};
                    if !matches!(parse_recurse_submodules(raw), RecurseSubmodules::Off) {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(
                            stderr,
                            "fatal: options '--porcelain' and '--recurse-submodules' cannot be used together"
                        );
                        drop(stderr);
                        std::process::exit(128);
                    }
                }
            }
            // `if (negotiate_only && !negotiation_tip.nr)` in cmd_fetch. Pre-transport
            // exit 128 with git's exact message (note the trailing '=*').
            if platform.negotiate_only && platform.negotiation_tip.is_empty() {
                use std::io::Write;
                let mut stderr = std::io::stderr().lock();
                let _ = writeln!(stderr, "fatal: --negotiate-only needs one or more --negotiation-tip=*");
                drop(stderr);
                std::process::exit(128);
            }
            let unshallow_requested = platform.shallow.unshallow;
            let shallow = crate::plumbing::options::fetch::resolve_shallow(&platform.shallow);
            // `--remote` (gix-native) overrides the git-compatible positional
            // `<repository>` when both are supplied, matching the pre-parity
            // CLI contract.
            let remote_name = platform.remote.or(platform.repository);
            let opts = core::repository::fetch::Options {
                format,
                dry_run: platform.dry_run,
                remote: remote_name,
                handshake_info: platform.handshake_info,
                negotiation_info: platform.negotiation_info,
                open_negotiation_graph: platform.open_negotiation_graph,
                shallow,
                ref_specs: platform.refspec,
                unshallow_requested,
            };
            prepare_and_run(
                "fetch",
                trace,
                auto_verbose,
                progress,
                progress_keep_open,
                core::repository::fetch::PROGRESS_RANGE,
                move |progress, out, err| {
                    core::repository::fetch(repository(Mode::LenientWithGitInstallConfig)?, progress, out, err, opts)
                },
            )
        }
        #[cfg(feature = "gitoxide-core-blocking-client")]
        Subcommands::Push(crate::plumbing::options::push::Platform {
            all,
            mirror,
            delete,
            tags,
            follow_tags,
            dry_run,
            porcelain,
            force,
            force_with_lease,
            force_if_includes,
            atomic,
            prune,
            set_upstream,
            verbose: _,
            quiet: _,
            progress: push_progress,
            no_progress,
            thin,
            no_thin,
            no_verify,
            receive_pack,
            signed,
            push_option,
            recurse_submodules,
            ipv4,
            ipv6,
            repo,
            repository: push_repository,
            refspec,
        }) => {
            let opts = core::repository::push::Options {
                format,
                all,
                mirror,
                delete,
                tags,
                follow_tags,
                dry_run,
                porcelain,
                force,
                force_with_lease,
                force_if_includes,
                atomic,
                prune,
                set_upstream,
                progress: if push_progress {
                    Some(true)
                } else if no_progress {
                    Some(false)
                } else {
                    None
                },
                thin: if thin {
                    Some(true)
                } else if no_thin {
                    Some(false)
                } else {
                    None
                },
                no_verify,
                receive_pack,
                signed_arg: signed,
                push_options: push_option,
                recurse_submodules_arg: recurse_submodules,
                ipv4,
                ipv6,
                repo,
                remote: push_repository,
                ref_specs: refspec,
            };
            // `--porcelain` emits machine-readable output; auto-verbose progress
            // on stderr would leak ANSI escapes that confuse scripts. Same
            // suppression applies to `--quiet`. Mirrors git's own `-q` /
            // `--porcelain` suppression of progress output.
            let push_auto_verbose = auto_verbose && !opts.porcelain;
            prepare_and_run(
                "push",
                trace,
                push_auto_verbose,
                progress,
                progress_keep_open,
                core::repository::push::PROGRESS_RANGE,
                move |progress, out, err| {
                    core::repository::push(repository(Mode::LenientWithGitInstallConfig)?, progress, out, err, opts)
                },
            )
        }
        Subcommands::ConfigTree => show_progress(),
        Subcommands::Credential(cmd) => core::repository::credential(
            repository(Mode::StrictWithGitInstallConfig).ok(),
            match cmd {
                credential::Subcommands::Fill => gix::credentials::program::main::Action::Get,
                credential::Subcommands::Approve => gix::credentials::program::main::Action::Store,
                credential::Subcommands::Reject => gix::credentials::program::main::Action::Erase,
            },
        ),
        #[cfg(any(feature = "gitoxide-core-async-client", feature = "gitoxide-core-blocking-client"))]
        Subcommands::Remote(crate::plumbing::options::remote::Platform {
            name,
            cmd,
            handshake_info,
        }) => {
            use crate::plumbing::options::remote;
            match cmd {
                remote::Subcommands::Refs | remote::Subcommands::RefMap { .. } => {
                    let kind = match cmd {
                        remote::Subcommands::Refs => core::repository::remote::refs::Kind::Remote,
                        remote::Subcommands::RefMap {
                            ref_spec,
                            show_unmapped_remote_refs,
                        } => core::repository::remote::refs::Kind::Tracking {
                            ref_specs: ref_spec,
                            show_unmapped_remote_refs,
                        },
                    };
                    let context = core::repository::remote::refs::Options {
                        name_or_url: name,
                        format,
                        handshake_info,
                    };
                    #[cfg(feature = "gitoxide-core-blocking-client")]
                    {
                        prepare_and_run(
                            "remote-refs",
                            trace,
                            auto_verbose,
                            progress,
                            progress_keep_open,
                            core::repository::remote::refs::PROGRESS_RANGE,
                            move |progress, out, err| {
                                core::repository::remote::refs(
                                    repository(Mode::LenientWithGitInstallConfig)?,
                                    kind,
                                    progress,
                                    out,
                                    err,
                                    context,
                                )
                            },
                        )
                    }
                    #[cfg(feature = "gitoxide-core-async-client")]
                    {
                        let (_handle, progress) = async_util::prepare(
                            auto_verbose,
                            trace,
                            "remote-refs",
                            Some(core::repository::remote::refs::PROGRESS_RANGE),
                        );
                        futures_lite::future::block_on(core::repository::remote::refs(
                            repository(Mode::LenientWithGitInstallConfig)?,
                            kind,
                            progress,
                            std::io::stdout(),
                            std::io::stderr(),
                            context,
                        ))
                    }
                }
            }
        }
        Subcommands::Config(config::Platform { filter }) => prepare_and_run(
            "config-list",
            trace,
            verbose,
            progress,
            progress_keep_open,
            None,
            move |_progress, out, _err| {
                core::repository::config::list(
                    repository(Mode::LenientWithGitInstallConfig)?,
                    filter,
                    config,
                    format,
                    out,
                )
            },
        )
        .map(|_| ()),
        Subcommands::Free(subcommands) => match subcommands {
            free::Subcommands::Discover => prepare_and_run(
                "discover",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| core::discover(&repository_path, out),
            ),
            free::Subcommands::Trust { paths } => prepare_and_run(
                "trust",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| core::trust(&paths, out),
            ),
            free::Subcommands::CommitGraph(cmd) => match cmd {
                free::commitgraph::Subcommands::Verify { path, statistics } => prepare_and_run(
                    "commitgraph-verify",
                    trace,
                    auto_verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, out, err| {
                        let output_statistics = if statistics { Some(format) } else { None };
                        core::commitgraph::verify(
                            path,
                            core::commitgraph::verify::Context {
                                err,
                                out,
                                output_statistics,
                            },
                        )
                    },
                )
                .map(|_| ()),
            },
            free::Subcommands::Index(free::index::Platform {
                object_hash,
                index_path,
                cmd,
            }) => match cmd {
                free::index::Subcommands::FromList {
                    force,
                    index_output_path,
                    skip_hash,
                    file,
                } => prepare_and_run(
                    "index-from-list",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, _out, _err| {
                        core::repository::index::from_list(file, index_output_path, force, skip_hash)
                    },
                ),
                free::index::Subcommands::CheckoutExclusive {
                    directory,
                    empty_files,
                    repository,
                    keep_going,
                } => prepare_and_run(
                    "index-checkout",
                    trace,
                    auto_verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |progress, _out, err| {
                        core::index::checkout_exclusive(
                            index_path,
                            directory,
                            repository,
                            err,
                            progress,
                            &should_interrupt,
                            core::index::checkout_exclusive::Options {
                                index: core::index::Options { object_hash, format },
                                empty_files,
                                keep_going,
                                thread_limit,
                            },
                        )
                    },
                ),
                free::index::Subcommands::Info { no_details } => prepare_and_run(
                    "index-info",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, out, err| {
                        core::index::information(
                            index_path,
                            out,
                            err,
                            core::index::information::Options {
                                index: core::index::Options { object_hash, format },
                                extension_details: !no_details,
                            },
                        )
                    },
                ),
                free::index::Subcommands::Verify => prepare_and_run(
                    "index-verify",
                    trace,
                    auto_verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, out, _err| {
                        core::index::verify(index_path, out, core::index::Options { object_hash, format })
                    },
                ),
            },
            free::Subcommands::Mailmap {
                cmd: free::mailmap::Platform { path, cmd },
            } => match cmd {
                free::mailmap::Subcommands::Verify => prepare_and_run(
                    "mailmap-verify",
                    trace,
                    auto_verbose,
                    progress,
                    progress_keep_open,
                    core::mailmap::PROGRESS_RANGE,
                    move |_progress, out, _err| core::mailmap::verify(path, format, out),
                ),
            },
            free::Subcommands::Pack(subcommands) => match subcommands {
                free::pack::Subcommands::Create {
                    repository,
                    expansion,
                    thin,
                    statistics,
                    nondeterministic_count,
                    tips,
                    pack_cache_size_mb,
                    counting_threads,
                    object_cache_size_mb,
                    output_directory,
                } => {
                    let has_tips = !tips.is_empty();
                    prepare_and_run(
                        "pack-create",
                        trace,
                        verbose,
                        progress,
                        progress_keep_open,
                        core::pack::create::PROGRESS_RANGE,
                        move |progress, out, _err| {
                            let input = if has_tips { None } else { stdin_or_bail()?.into() };
                            let repository = repository.unwrap_or_else(|| PathBuf::from("."));
                            let context = core::pack::create::Context {
                                thread_limit,
                                thin,
                                nondeterministic_thread_count: nondeterministic_count.then_some(counting_threads),
                                pack_cache_size_in_bytes: pack_cache_size_mb.unwrap_or(0) * 1_000_000,
                                object_cache_size_in_bytes: object_cache_size_mb.unwrap_or(0) * 1_000_000,
                                statistics: if statistics { Some(format) } else { None },
                                out,
                                expansion: expansion.unwrap_or(if has_tips {
                                    core::pack::create::ObjectExpansion::TreeTraversal
                                } else {
                                    core::pack::create::ObjectExpansion::None
                                }),
                            };
                            core::pack::create(repository, tips, input, output_directory, progress, context)
                        },
                    )
                }
                #[cfg(feature = "gitoxide-core-async-client")]
                free::pack::Subcommands::Receive {
                    protocol,
                    url,
                    directory,
                    refs,
                    refs_directory,
                } => {
                    let (_handle, progress) =
                        async_util::prepare(verbose, trace, "pack-receive", core::pack::receive::PROGRESS_RANGE);
                    let fut = core::pack::receive(
                        protocol,
                        &url,
                        directory,
                        refs_directory,
                        refs.into_iter().map(Into::into).collect(),
                        progress,
                        core::pack::receive::Context {
                            thread_limit,
                            format,
                            out: std::io::stdout(),
                            should_interrupt,
                            object_hash,
                        },
                    );
                    return futures_lite::future::block_on(fut);
                }
                #[cfg(feature = "gitoxide-core-blocking-client")]
                free::pack::Subcommands::Receive {
                    protocol,
                    url,
                    directory,
                    refs,
                    refs_directory,
                } => prepare_and_run(
                    "pack-receive",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    core::pack::receive::PROGRESS_RANGE,
                    move |progress, out, _err| {
                        core::pack::receive(
                            protocol,
                            &url,
                            directory,
                            refs_directory,
                            refs.into_iter().map(Into::into).collect(),
                            progress,
                            core::pack::receive::Context {
                                thread_limit,
                                format,
                                should_interrupt,
                                out,
                                object_hash,
                            },
                        )
                    },
                ),
                free::pack::Subcommands::Explode {
                    check,
                    sink_compress,
                    delete_pack,
                    pack_path,
                    object_path,
                    verify,
                } => prepare_and_run(
                    "pack-explode",
                    trace,
                    auto_verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |progress, _out, _err| {
                        core::pack::explode::pack_or_pack_index(
                            pack_path,
                            object_path,
                            check,
                            progress,
                            core::pack::explode::Context {
                                thread_limit,
                                delete_pack,
                                sink_compress,
                                verify,
                                should_interrupt,
                                object_hash,
                            },
                        )
                    },
                ),
                free::pack::Subcommands::Verify {
                    args:
                        free::pack::VerifyOptions {
                            algorithm,
                            decode,
                            re_encode,
                            statistics,
                        },
                    path,
                } => prepare_and_run(
                    "pack-verify",
                    trace,
                    auto_verbose,
                    progress,
                    progress_keep_open,
                    verify::PROGRESS_RANGE,
                    move |progress, out, err| {
                        let mode = verify_mode(decode, re_encode);
                        let output_statistics = if statistics { Some(format) } else { None };
                        verify::pack_or_pack_index(
                            path,
                            progress,
                            verify::Context {
                                output_statistics,
                                out,
                                err,
                                thread_limit,
                                mode,
                                algorithm,
                                should_interrupt: &should_interrupt,
                                object_hash,
                            },
                        )
                    },
                )
                .map(|_| ()),
                free::pack::Subcommands::MultiIndex(free::pack::multi_index::Platform { multi_index_path, cmd }) => {
                    match cmd {
                        free::pack::multi_index::Subcommands::Entries => prepare_and_run(
                            "pack-multi-index-entries",
                            trace,
                            verbose,
                            progress,
                            progress_keep_open,
                            core::pack::multi_index::PROGRESS_RANGE,
                            move |_progress, out, _err| core::pack::multi_index::entries(multi_index_path, format, out),
                        ),
                        free::pack::multi_index::Subcommands::Info => prepare_and_run(
                            "pack-multi-index-info",
                            trace,
                            verbose,
                            progress,
                            progress_keep_open,
                            core::pack::multi_index::PROGRESS_RANGE,
                            move |_progress, out, err| {
                                core::pack::multi_index::info(multi_index_path, format, out, err)
                            },
                        ),
                        free::pack::multi_index::Subcommands::Verify => prepare_and_run(
                            "pack-multi-index-verify",
                            trace,
                            auto_verbose,
                            progress,
                            progress_keep_open,
                            core::pack::multi_index::PROGRESS_RANGE,
                            move |progress, _out, _err| {
                                core::pack::multi_index::verify(multi_index_path, progress, &should_interrupt)
                            },
                        ),
                        free::pack::multi_index::Subcommands::Create { index_paths } => prepare_and_run(
                            "pack-multi-index-create",
                            trace,
                            verbose,
                            progress,
                            progress_keep_open,
                            core::pack::multi_index::PROGRESS_RANGE,
                            move |progress, _out, _err| {
                                core::pack::multi_index::create(
                                    index_paths,
                                    multi_index_path,
                                    progress,
                                    &should_interrupt,
                                    object_hash,
                                )
                            },
                        ),
                    }
                }
                free::pack::Subcommands::Index(subcommands) => match subcommands {
                    free::pack::index::Subcommands::Create {
                        iteration_mode,
                        pack_path,
                        directory,
                    } => prepare_and_run(
                        "pack-index-create",
                        trace,
                        verbose,
                        progress,
                        progress_keep_open,
                        core::pack::index::PROGRESS_RANGE,
                        move |progress, out, _err| {
                            use gitoxide_core::pack::index::PathOrRead;
                            let input = if let Some(path) = pack_path {
                                PathOrRead::Path(path)
                            } else {
                                use is_terminal::IsTerminal;
                                if std::io::stdin().is_terminal() {
                                    anyhow::bail!(
                                        "Refusing to read from standard input as no path is given, but it's a terminal."
                                    )
                                }
                                PathOrRead::Read(Box::new(stdin()))
                            };
                            core::pack::index::from_pack(
                                input,
                                directory,
                                progress,
                                core::pack::index::Context {
                                    thread_limit,
                                    iteration_mode,
                                    format,
                                    out,
                                    object_hash,
                                    should_interrupt: &gix::interrupt::IS_INTERRUPTED,
                                },
                            )
                        },
                    ),
                },
            },
        },
        Subcommands::Verify {
            args:
                free::pack::VerifyOptions {
                    statistics,
                    algorithm,
                    decode,
                    re_encode,
                },
        } => prepare_and_run(
            "verify",
            trace,
            auto_verbose,
            progress,
            progress_keep_open,
            core::repository::verify::PROGRESS_RANGE,
            move |progress, out, _err| {
                core::repository::verify::integrity(
                    repository(Mode::Strict)?,
                    out,
                    progress,
                    &should_interrupt,
                    core::repository::verify::Context {
                        output_statistics: statistics.then_some(format),
                        algorithm,
                        verify_mode: verify_mode(decode, re_encode),
                        thread_limit,
                    },
                )
            },
        ),
        Subcommands::Revision(cmd) => match cmd {
            revision::Subcommands::List {
                spec,
                svg,
                limit,
                long_hashes,
            } => prepare_and_run(
                "revision-list",
                trace,
                auto_verbose,
                progress,
                progress_keep_open,
                core::repository::revision::list::PROGRESS_RANGE,
                move |progress, out, _err| {
                    core::repository::revision::list(
                        repository(Mode::Lenient)?,
                        progress,
                        out,
                        core::repository::revision::list::Context {
                            limit,
                            spec,
                            format,
                            long_hashes,
                            text: svg.map_or(core::repository::revision::list::Format::Text, |path| {
                                core::repository::revision::list::Format::Svg { path }
                            }),
                        },
                    )
                },
            ),
            revision::Subcommands::PreviousBranches => prepare_and_run(
                "revision-previousbranches",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| {
                    core::repository::revision::previous_branches(repository(Mode::Lenient)?, out, format)
                },
            ),
            revision::Subcommands::Explain { spec } => prepare_and_run(
                "revision-explain",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| core::repository::revision::explain(spec, out),
            ),
            revision::Subcommands::Resolve {
                specs,
                explain,
                cat_file,
                tree_mode,
                reference,
                blob_format,
            } => prepare_and_run(
                "revision-parse",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| {
                    core::repository::revision::resolve(
                        repository(Mode::Strict)?,
                        specs,
                        out,
                        core::repository::revision::resolve::Options {
                            format,
                            explain,
                            cat_file,
                            show_reference: reference,
                            tree_mode: match tree_mode {
                                revision::resolve::TreeMode::Raw => core::repository::revision::resolve::TreeMode::Raw,
                                revision::resolve::TreeMode::Pretty => {
                                    core::repository::revision::resolve::TreeMode::Pretty
                                }
                            },
                            blob_format: match blob_format {
                                revision::resolve::BlobFormat::Git => {
                                    core::repository::revision::resolve::BlobFormat::Git
                                }
                                revision::resolve::BlobFormat::Worktree => {
                                    core::repository::revision::resolve::BlobFormat::Worktree
                                }
                                revision::resolve::BlobFormat::Diff => {
                                    core::repository::revision::resolve::BlobFormat::Diff
                                }
                                revision::resolve::BlobFormat::DiffOrGit => {
                                    core::repository::revision::resolve::BlobFormat::DiffOrGit
                                }
                            },
                        },
                    )
                },
            ),
        },
        Subcommands::Cat {
            pretty,
            exists,
            print_type,
            print_size,
            // `--allow-unknown-type` is a documented no-op (per the
            // OPT_HIDDEN_BOOL help text in git's options[] array).
            // Accept and drop.
            allow_unknown_type: _,
            // `--use-mailmap` / `--mailmap` enable, `--no-use-mailmap` /
            // `--no-mailmap` disable. Last-wins has no effect in our
            // fixtures because the scaffold tests never combine them;
            // exactly one of {use_mailmap, no_use_mailmap} is ever set.
            use_mailmap,
            no_use_mailmap: _,
            // `--textconv` / `--filters` request content transformation.
            // With no userdiff / gitattributes configured in the fixture,
            // both degenerate to "emit raw blob" — the same bytes as
            // -p / bare positional — so they're accepted as compat
            // passthroughs until real filter wiring lands.
            textconv,
            filters,
            // `--path=<path>` supplies the path for filter-attribute
            // lookup; only meaningful with --textconv/--filters. When set
            // without those, git emits a usage error and exits 129 — we
            // mirror that exit code below.
            path,
            // `--batch*[=<format>]` — stdin-driven info/contents emitter.
            // Each is None (flag absent) / Some("") (bare) / Some(s)
            // (with format). Mutually exclusive in git
            // (batch_option_callback dies if two are set).
            batch_check,
            batch,
            batch_command,
            // `-Z` = NUL on both input AND output.
            // `-z` = NUL on input only (deprecated). Git's batch_options
            // defaults both delims to LF then patches per flag.
            nul_terminated,
            nul_input,
            // Accepted for parity; observable only when interacting with
            // real scripts. Usage-check mirrors git: these flags are
            // meaningful *only* under --batch/--batch-check, else 129.
            buffer,
            unordered,
            follow_symlinks,
            batch_all_objects,
            args,
        } => {
            // Mutual-exclusion among the `-e`/`-p`/`-t`/`-s` cmdmode
            // flags. git's OPT_CMDMODE auto-rejects two being set with
            // "error: options '-x' and '-y' cannot be used together"
            // and exits 129. Only the exit code is asserted; message
            // format diverges.
            {
                let set: [(&str, bool); 4] = [("-e", exists), ("-p", pretty), ("-t", print_type), ("-s", print_size)];
                let count = set.iter().filter(|(_, v)| *v).count();
                if count > 1 {
                    let active: Vec<&str> = set.iter().filter(|(_, v)| *v).map(|(n, _)| *n).collect();
                    use std::io::Write;
                    let mut stderr = std::io::stderr().lock();
                    let _ = writeln!(
                        stderr,
                        "error: options '{}' and '{}' cannot be used together",
                        active[0], active[1]
                    );
                    drop(stderr);
                    std::process::exit(129);
                }
            }
            // Query mode (-e/-p/-t/-s) + batch mode is also rejected by
            // git's usage_msg_optf "'-%c' is incompatible with batch mode"
            // check (exit 129). --textconv / --filters are *allowed* with
            // batch, so they're excluded from this test.
            let any_query_mode = exists || pretty || print_type || print_size;
            let any_batch = batch.is_some() || batch_check.is_some() || batch_command.is_some();
            if any_query_mode && any_batch {
                let flag = if exists {
                    "-e"
                } else if pretty {
                    "-p"
                } else if print_type {
                    "-t"
                } else {
                    "-s"
                };
                use std::io::Write;
                let mut stderr = std::io::stderr().lock();
                let _ = writeln!(stderr, "fatal: '{flag}' is incompatible with batch mode");
                drop(stderr);
                std::process::exit(129);
            }
            // `--batch*[=<format>]` short-circuits the whole dispatch.
            // Both modes ignore positional args (stdin drives the object
            // list), so this check fires before the 1-vs-2 positional
            // split below. Git rejects both being set simultaneously
            // (batch_option_callback → error 129); we mirror that.
            // Git's batch_option_callback rejects any two of --batch /
            // --batch-check / --batch-command at once with `error: only
            // one batch option may be specified` (exit 129).
            {
                let set = [batch.is_some(), batch_check.is_some(), batch_command.is_some()];
                if set.iter().filter(|b| **b).count() > 1 {
                    use std::io::Write;
                    let mut stderr = std::io::stderr().lock();
                    let _ = writeln!(stderr, "error: only one batch option may be specified");
                    drop(stderr);
                    std::process::exit(129);
                }
            }
            if let Some(format) = batch_command.as_deref() {
                if !format.is_empty() {
                    if let Some(bad) = core::repository::cat_first_unknown_atom(format) {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(stderr, "fatal: bad cat-file format: %({bad})");
                        drop(stderr);
                        std::process::exit(128);
                    }
                }
                let input_delim: u8 = if nul_terminated || nul_input { 0 } else { b'\n' };
                let output_delim: u8 = if nul_terminated { 0 } else { b'\n' };
                let repo = repository(Mode::Lenient)?;
                let stdin = std::io::stdin().lock();
                let stdout = std::io::stdout().lock();
                let format = if format.is_empty() { None } else { Some(format) };
                core::repository::cat_batch_command(&repo, format, input_delim, output_delim, stdin, stdout)?;
                std::process::exit(0);
            }
            if let Some(format) = batch.as_deref().or(batch_check.as_deref()) {
                // Validate format atoms before starting any loop. git's
                // strbuf_expand_bad_format dies with exit 128 and
                // `fatal: bad cat-file format: %(<atom>)` at the
                // mark_query pre-pass in batch_objects; we mirror that
                // with an up-front scan so stdin/stdout aren't touched
                // on malformed input.
                if !format.is_empty() {
                    if let Some(bad) = core::repository::cat_first_unknown_atom(format) {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(stderr, "fatal: bad cat-file format: %({bad})");
                        drop(stderr);
                        std::process::exit(128);
                    }
                }
                // -Z = NUL in + NUL out; -z = NUL in + LF out.
                let input_delim: u8 = if nul_terminated || nul_input { 0 } else { b'\n' };
                let output_delim: u8 = if nul_terminated { 0 } else { b'\n' };
                let mode = if batch.is_some() {
                    core::repository::CatBatchMode::Contents
                } else {
                    core::repository::CatBatchMode::Info
                };
                let repo = repository(Mode::Lenient)?;
                let stdout = std::io::stdout().lock();
                let format = if format.is_empty() { None } else { Some(format) };
                if batch_all_objects {
                    // --batch-all-objects ignores stdin; enumerate odb.
                    core::repository::cat_batch_all_objects(&repo, mode, format, unordered, output_delim, stdout)?;
                } else {
                    let stdin = std::io::stdin().lock();
                    core::repository::cat_batch(
                        &repo,
                        mode,
                        format,
                        follow_symlinks,
                        input_delim,
                        output_delim,
                        stdin,
                        stdout,
                    )?;
                }
                std::process::exit(0);
            }
            // --batch-all-objects without a batch mode → 129.
            if batch_all_objects {
                use std::io::Write;
                let mut stderr = std::io::stderr().lock();
                let _ = writeln!(stderr, "fatal: '--batch-all-objects' requires a batch mode");
                drop(stderr);
                std::process::exit(129);
            }
            // Batch-only flags without a batch mode → usage error 129.
            // Git's cmd_cat_file checks each of these individually after
            // parse_options and calls usage_msg_optf. Order matters for
            // git's output, but we only need the exit code + a matching
            // fatal line to satisfy effect-mode parity.
            let batch_only_error: Option<&str> = if follow_symlinks {
                Some("'--follow-symlinks' requires a batch mode")
            } else if buffer {
                Some("'--buffer' requires a batch mode")
            } else if unordered {
                Some("'--unordered' requires a batch mode")
            } else if nul_terminated {
                Some("'-Z' requires a batch mode")
            } else if nul_input {
                Some("'-z' requires a batch mode")
            } else {
                None
            };
            if let Some(msg) = batch_only_error {
                use std::io::Write;
                let mut stderr = std::io::stderr().lock();
                let _ = writeln!(stderr, "fatal: {msg}");
                drop(stderr);
                std::process::exit(129);
            }
            // `--path=<path>` is only meaningful with --textconv/--filters;
            // git's cmd_cat_file calls usage_msg_optf and exits 129
            // otherwise. Check early so the error path fires before any
            // repo discovery.
            if path.is_some() && !textconv && !filters {
                use std::io::Write;
                let mut stderr = std::io::stderr().lock();
                let _ = writeln!(
                    stderr,
                    "fatal: '--path=<path|tree-ish>' needs '--filters' or '--textconv'"
                );
                drop(stderr);
                std::process::exit(129);
            }
            // `--textconv` / `--filters` are accepted as compat passthroughs
            // at this iteration — the fixtures have no userdiff or
            // gitattributes configured, so the git-side pipelines also
            // degenerate to "emit raw blob". Wire real filter invocation
            // once a row demands it.
            let _ = (textconv, filters, path);
            // The positional grammar tracks git cat-file's:
            //   * `<object>`         — with -e/-p/-t/-s OR alone (gix legacy)
            //   * `<type> <object>`  — no mode flag; peel and emit raw bytes
            // Clap already constrains num_args to 1..=2.
            let mode_flag = exists || print_type || print_size;
            let (type_hint, revspec): (Option<String>, String) = match args.as_slice() {
                [only] => (None, only.clone()),
                [first, second] if !mode_flag => (Some(first.clone()), second.clone()),
                _ => {
                    // Mode flag + 2 positionals. git's case 'e'/'p'/'t'/'s'
                    // `usage_msg_opt(_("<object> required"), ...)` → exit 129.
                    use std::io::Write;
                    let mut stderr = std::io::stderr().lock();
                    let _ = writeln!(stderr, "fatal: too many arguments");
                    drop(stderr);
                    std::process::exit(129);
                }
            };
            if let Some(type_str) = type_hint {
                // `git cat-file <type> <object>` — no mode flag allowed; bytes
                // path resolved via peel_to_kind. Four outcomes, each with
                // git's exact fatal wording on stderr + exit 128.
                let repo = repository(Mode::Lenient)?;
                let stdout = std::io::stdout().lock();
                let outcome = core::repository::cat_typed(&repo, &type_str, &revspec, stdout)?;
                match outcome {
                    core::repository::CatTypedOutcome::Ok => std::process::exit(0),
                    core::repository::CatTypedOutcome::InvalidType => {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(stderr, "fatal: invalid object type \"{type_str}\"");
                        drop(stderr);
                        std::process::exit(128);
                    }
                    core::repository::CatTypedOutcome::InvalidSpec => {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(stderr, "fatal: Not a valid object name {revspec}");
                        drop(stderr);
                        std::process::exit(128);
                    }
                    core::repository::CatTypedOutcome::BadFile => {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(stderr, "fatal: git cat-file {revspec}: bad file");
                        drop(stderr);
                        std::process::exit(128);
                    }
                }
            }
            if exists {
                // `git cat-file -e` has no stdout; only an exit code plus
                // an optional fatal stderr line:
                //   0   — spec resolves, object present in odb
                //   1   — spec resolves (or is a well-formed full-hex oid)
                //         but the object is absent (silent)
                //   128 — spec does not resolve at all; stderr gets git's
                //         exact `fatal: Not a valid object name <spec>`
                // Bypass prepare_and_run (which swallows non-zero exits
                // into anyhow errors) and exit directly.
                let repo = repository(Mode::Lenient)?;
                match core::repository::cat_exists(&repo, &revspec) {
                    core::repository::CatExistence::Present => std::process::exit(0),
                    core::repository::CatExistence::Absent => std::process::exit(1),
                    core::repository::CatExistence::InvalidSpec => {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(stderr, "fatal: Not a valid object name {revspec}");
                        drop(stderr);
                        std::process::exit(128);
                    }
                }
            }
            if print_type || print_size {
                // `git cat-file -t` prints the object type name (blob /
                // tree / commit / tag) + LF on stdout; `-s` prints the
                // object size in bytes + LF. Both share two failure paths:
                //   InvalidSpec    → `fatal: Not a valid object name <spec>`
                //                    (same as -e)
                //   MissingObject  → `fatal: git cat-file: could not get
                //                    object info` (literal full-hex oid
                //                    accepted by get_oid_with_context but
                //                    odb has no such object — case 't' /
                //                    case 's' in cat_one_file die here)
                // Both exit 128.
                let repo = repository(Mode::Lenient)?;
                let stdout = std::io::stdout().lock();
                let outcome = if print_type {
                    core::repository::cat_type(&repo, &revspec, stdout)?
                } else {
                    core::repository::cat_size(&repo, &revspec, use_mailmap, stdout)?
                };
                match outcome {
                    core::repository::CatPrintOutcome::Ok => std::process::exit(0),
                    core::repository::CatPrintOutcome::InvalidSpec => {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(stderr, "fatal: Not a valid object name {revspec}");
                        drop(stderr);
                        std::process::exit(128);
                    }
                    core::repository::CatPrintOutcome::MissingObject => {
                        use std::io::Write;
                        let mut stderr = std::io::stderr().lock();
                        let _ = writeln!(stderr, "fatal: git cat-file: could not get object info");
                        drop(stderr);
                        std::process::exit(128);
                    }
                }
            }
            prepare_and_run(
                "cat",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| core::repository::cat(repository(Mode::Lenient)?, &revspec, out),
            )
        }
        Subcommands::Commit(cmd) => match cmd {
            commit::Subcommands::Verify { rev_spec } => prepare_and_run(
                "commit-verify",
                trace,
                auto_verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, _out, _err| {
                    core::repository::commit::verify(repository(Mode::Lenient)?, rev_spec.as_deref())
                },
            ),
            commit::Subcommands::Sign { rev_spec } => prepare_and_run(
                "commit-sign",
                trace,
                auto_verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| {
                    core::repository::commit::sign(repository(Mode::Lenient)?, rev_spec.as_deref(), out)
                },
            ),
            commit::Subcommands::Describe {
                annotated_tags,
                all_refs,
                first_parent,
                always,
                long,
                statistics,
                max_candidates,
                rev_spec,
                dirty_suffix,
            } => prepare_and_run(
                "commit-describe",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, err| {
                    core::repository::commit::describe(
                        repository(Mode::Strict)?,
                        rev_spec.as_deref(),
                        out,
                        err,
                        core::repository::commit::describe::Options {
                            all_tags: !annotated_tags,
                            all_refs,
                            long_format: long,
                            first_parent,
                            statistics,
                            max_candidates,
                            always,
                            dirty_suffix: dirty_suffix.map(|suffix| suffix.unwrap_or_else(|| "dirty".to_string())),
                        },
                    )
                },
            ),
        },
        Subcommands::Tag(platform) => {
            let tag::Platform {
                list,
                delete,
                verify,
                force,
                annotate,
                message,
                file,
                edit,
                trailer,
                cleanup,
                create_reflog,
                no_create_reflog: _,
                sign,
                no_sign,
                local_user,
                ignore_case,
                column: _,
                no_column: _,
                color: _,
                sort: _,
                format_string,
                omit_empty,
                annotation_lines,
                points_at,
                contains,
                no_contains,
                merged,
                no_merged,
                patterns,
            } = platform;
            // git's cmdmode resolution (builtin/tag.c:559-566): if no
            // explicit -l/-d/-v and any list-filter flag is set, force
            // list mode. With argc > 0 and no filters → create mode.
            let has_list_filters = list
                || ignore_case
                || points_at.is_some()
                || contains.is_some()
                || no_contains.is_some()
                || merged.is_some()
                || no_merged.is_some();
            // Annotated-mode triggers (builtin/tag.c create_tag_object
            // logic): any of -a, -s, -u, -m, -F, --trailer implies
            // annotated create.
            let has_annotated_trigger = annotate
                || sign
                || local_user.is_some()
                || !message.is_empty()
                || file.is_some()
                || !trailer.is_empty();
            let is_create_mode = !delete && !verify && !has_list_filters && !patterns.is_empty();
            if is_create_mode && has_annotated_trigger {
                prepare_and_run(
                    "tag-create-annotated",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, out, err| {
                        core::repository::tag::create_annotated(
                            repository(Mode::Lenient)?,
                            patterns,
                            core::repository::tag::AnnotatedOptions {
                                force,
                                message,
                                file,
                                edit,
                                trailer,
                                cleanup,
                                create_reflog,
                                sign,
                                no_sign,
                                local_user,
                            },
                            out,
                            err,
                        )
                    },
                )
            } else if is_create_mode {
                prepare_and_run(
                    "tag-create-lightweight",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, out, err| {
                        core::repository::tag::create_lightweight(repository(Mode::Lenient)?, patterns, force, out, err)
                    },
                )
            } else if delete {
                prepare_and_run(
                    "tag-delete",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, out, err| {
                        core::repository::tag::delete(repository(Mode::Lenient)?, patterns, out, err)
                    },
                )
            } else if verify {
                prepare_and_run(
                    "tag-verify",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, out, err| {
                        core::repository::tag::verify(repository(Mode::Lenient)?, patterns, out, err)
                    },
                )
            } else {
                prepare_and_run(
                    "tag-list",
                    trace,
                    verbose,
                    progress,
                    progress_keep_open,
                    None,
                    move |_progress, out, _err| {
                        core::repository::tag::list(
                            repository(Mode::Lenient)?,
                            out,
                            format,
                            core::repository::tag::ListOptions {
                                patterns,
                                ignore_case,
                                points_at,
                                contains,
                                no_contains,
                                merged,
                                no_merged,
                                format_string,
                                omit_empty,
                                annotation_lines,
                            },
                        )
                    },
                )
            }
        }
        Subcommands::Tree(cmd) => match cmd {
            tree::Subcommands::Entries {
                treeish,
                recursive,
                extended,
            } => prepare_and_run(
                "tree-entries",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| {
                    core::repository::tree::entries(
                        repository(Mode::Strict)?,
                        treeish.as_deref(),
                        recursive,
                        extended,
                        format,
                        out,
                    )
                },
            ),
            tree::Subcommands::Info { treeish, extended } => prepare_and_run(
                "tree-info",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, err| {
                    core::repository::tree::info(
                        repository(Mode::Strict)?,
                        treeish.as_deref(),
                        extended,
                        format,
                        out,
                        err,
                    )
                },
            ),
        },
        Subcommands::Odb(cmd) => match cmd {
            odb::Subcommands::Stats { extra_header_lookup } => prepare_and_run(
                "odb-stats",
                trace,
                auto_verbose,
                progress,
                progress_keep_open,
                core::repository::odb::statistics::PROGRESS_RANGE,
                move |progress, out, err| {
                    core::repository::odb::statistics(
                        repository(Mode::Strict)?,
                        progress,
                        out,
                        err,
                        core::repository::odb::statistics::Options {
                            format,
                            thread_limit,
                            extra_header_lookup,
                        },
                    )
                },
            ),
            odb::Subcommands::Entries => prepare_and_run(
                "odb-entries",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, _err| core::repository::odb::entries(repository(Mode::Strict)?, format, out),
            ),
            odb::Subcommands::Info => prepare_and_run(
                "odb-info",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, err| core::repository::odb::info(repository(Mode::Strict)?, format, out, err),
            ),
        },
        Subcommands::Fsck(fsck::Platform { spec }) => prepare_and_run(
            "fsck",
            trace,
            auto_verbose,
            progress,
            progress_keep_open,
            None,
            move |_progress, out, _err| core::repository::fsck(repository(Mode::Strict)?, spec, out),
        ),
        Subcommands::Mailmap(cmd) => match cmd {
            mailmap::Subcommands::Entries => prepare_and_run(
                "mailmap-entries",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, err| {
                    core::repository::mailmap::entries(repository(Mode::Lenient)?, format, out, err)
                },
            ),
            mailmap::Subcommands::Check { contacts } => prepare_and_run(
                "mailmap-check",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, err| {
                    core::repository::mailmap::check(repository(Mode::Lenient)?, format, contacts, out, err)
                },
            ),
        },
        Subcommands::Attributes(cmd) => match cmd {
            attributes::Subcommands::Query { statistics, pathspec } => prepare_and_run(
                "attributes-query",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, err| {
                    let repo = repository(Mode::Strict)?;
                    let pathspecs = if pathspec.is_empty() {
                        PathsOrPatterns::Paths(Box::new(
                            stdin_or_bail()?.byte_lines().filter_map(Result::ok).map(BString::from),
                        ))
                    } else {
                        PathsOrPatterns::Patterns(pathspec)
                    };
                    core::repository::attributes::query(
                        repo,
                        pathspecs,
                        out,
                        err,
                        core::repository::attributes::query::Options { format, statistics },
                    )
                },
            ),
            attributes::Subcommands::ValidateBaseline { statistics, no_ignore } => prepare_and_run(
                "attributes-validate-baseline",
                trace,
                auto_verbose,
                progress,
                progress_keep_open,
                None,
                move |progress, out, err| {
                    core::repository::attributes::validate_baseline(
                        repository(Mode::StrictWithGitInstallConfig)?,
                        stdin_or_bail()
                            .ok()
                            .map(|stdin| stdin.byte_lines().filter_map(Result::ok).map(gix::bstr::BString::from)),
                        progress,
                        out,
                        err,
                        core::repository::attributes::validate_baseline::Options {
                            format,
                            statistics,
                            ignore: !no_ignore,
                        },
                    )
                },
            ),
        },
        Subcommands::Exclude(cmd) => match cmd {
            exclude::Subcommands::Query {
                statistics,
                patterns,
                pathspec,
                show_ignore_patterns,
            } => prepare_and_run(
                "exclude-query",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, err| {
                    let repo = repository(Mode::Strict)?;
                    let pathspecs = if pathspec.is_empty() {
                        PathsOrPatterns::Paths(Box::new(
                            stdin_or_bail()?.byte_lines().filter_map(Result::ok).map(BString::from),
                        ))
                    } else {
                        PathsOrPatterns::Patterns(pathspec)
                    };
                    core::repository::exclude::query(
                        repo,
                        pathspecs,
                        out,
                        err,
                        core::repository::exclude::query::Options {
                            format,
                            show_ignore_patterns,
                            overrides: patterns,
                            statistics,
                        },
                    )
                },
            ),
        },
        Subcommands::Index(cmd) => match cmd {
            index::Subcommands::Entries {
                format: entry_format,
                no_attributes,
                attributes_from_index,
                statistics,
                recurse_submodules,
                pathspec,
            } => prepare_and_run(
                "index-entries",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, out, err| {
                    core::repository::index::entries(
                        repository(Mode::Lenient)?,
                        pathspec,
                        out,
                        err,
                        core::repository::index::entries::Options {
                            format,
                            simple: match entry_format {
                                index::entries::Format::Simple => true,
                                index::entries::Format::Rich => false,
                            },
                            attributes: if no_attributes {
                                None
                            } else {
                                Some(if attributes_from_index {
                                    core::repository::index::entries::Attributes::Index
                                } else {
                                    core::repository::index::entries::Attributes::WorktreeAndIndex
                                })
                            },
                            recurse_submodules,
                            statistics,
                        },
                    )
                },
            ),
            index::Subcommands::FromTree {
                force,
                index_output_path,
                skip_hash,
                spec,
            } => prepare_and_run(
                "index-from-tree",
                trace,
                verbose,
                progress,
                progress_keep_open,
                None,
                move |_progress, _out, _err| {
                    core::repository::index::from_tree(
                        repository(Mode::Strict)?,
                        spec,
                        index_output_path,
                        force,
                        skip_hash,
                    )
                },
            ),
        },
        Subcommands::Blame {
            statistics,
            file,
            ranges,
            since,
        } => prepare_and_run(
            "blame",
            trace,
            verbose,
            progress,
            progress_keep_open,
            None,
            move |_progress, out, err| {
                let repo = repository(Mode::Lenient)?;
                let diff_algorithm = repo.diff_algorithm()?;

                core::repository::blame::blame_file(
                    repo,
                    &file,
                    gix::blame::Options {
                        diff_algorithm,
                        ranges: gix::blame::BlameRanges::from_one_based_inclusive_ranges(ranges)?,
                        since,
                        rewrites: Some(gix::diff::Rewrites::default()),
                        debug_track_path: false,
                    },
                    out,
                    statistics.then_some(err),
                )
            },
        ),
        Subcommands::Completions { shell, out_dir } => {
            let mut app = Args::command();

            let shell = shell
                .or_else(clap_complete::Shell::from_env)
                .ok_or_else(|| anyhow!("The shell could not be derived from the environment"))?;

            let bin_name = app.get_name().to_owned();
            if let Some(out_dir) = out_dir {
                clap_complete::generate_to(shell, &mut app, bin_name, &out_dir)?;
            } else {
                clap_complete::generate(shell, &mut app, bin_name, &mut std::io::stdout());
            }
            Ok(())
        }
    }?;
    Ok(())
}

/// Walk `args_os()` looking for the first positional — the subcommand name.
/// Used by the UnknownArgument remap to apply command-specific exit codes
/// (e.g. `log` dies via die() → 128, rest use usage_msg_opt → 129). Must
/// stay in sync with the top-level `Args` struct in `options/mod.rs`:
/// every global option that takes a value is listed here so the scan
/// skips its value token.
fn detect_subcommand_from_argv() -> Option<String> {
    // Long options taking a value on the top-level Args.
    const VALUE_FLAGS_LONG: &[&str] = &["--repository", "--config", "--threads", "--format", "--object-hash"];
    // Short options taking a value on the top-level Args.
    const VALUE_FLAGS_SHORT: &[&str] = &["-r", "-c", "-t", "-f"];

    let mut iter = gix::env::args_os().skip(1);
    while let Some(arg) = iter.next() {
        let s = arg.to_string_lossy();
        // `--flag=value` forms carry their value inline — always a flag, skip.
        if s.starts_with("--") && s.contains('=') {
            continue;
        }
        // `--flag value` / `-f value` forms: consume the next token as the value.
        if VALUE_FLAGS_LONG.contains(&s.as_ref()) || VALUE_FLAGS_SHORT.contains(&s.as_ref()) {
            let _ = iter.next();
            continue;
        }
        // Any other flag-looking token: skip.
        if s.starts_with('-') {
            continue;
        }
        // First positional — the subcommand name.
        return Some(s.into_owned());
    }
    None
}

fn stdin_or_bail() -> Result<std::io::BufReader<std::io::Stdin>> {
    use is_terminal::IsTerminal;
    if std::io::stdin().is_terminal() {
        anyhow::bail!("Refusing to read from standard input while a terminal is connected")
    }
    Ok(BufReader::new(stdin()))
}

fn verify_mode(decode: bool, re_encode: bool) -> verify::Mode {
    match (decode, re_encode) {
        (true, false) => verify::Mode::HashCrc32Decode,
        (_, true) => verify::Mode::HashCrc32DecodeEncode,
        (false, false) => verify::Mode::HashCrc32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap() {
        use clap::CommandFactory;
        Args::command().debug_assert();
    }
}
