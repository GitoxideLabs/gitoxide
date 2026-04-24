use gix::bstr::{BString, ByteSlice};

/// CLI-level options for `gix log`. Mirrors the user-visible flag surface
/// from `src/plumbing/options/mod.rs::log::Platform`.
#[derive(Debug, Default)]
pub struct Options {
    pub all: bool,
    pub branches: bool,
    pub tags: bool,
    pub remotes: bool,
    pub max_count: Option<usize>,
    pub skip: Option<usize>,
    pub no_merges: bool,
    pub merges: bool,
    pub min_parents: Option<usize>,
    pub max_parents: Option<usize>,
    /// When true, the single positional argument is treated as a pathspec
    /// rather than a revspec (git's `--follow <file>` convention).
    pub follow: bool,
    pub revspec: Option<BString>,
    pub path: Option<BString>,
}

pub fn log(mut repo: gix::Repository, out: &mut dyn std::io::Write, opts: Options) -> anyhow::Result<()> {
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));

    // Pathspec filtering is not yet wired into log. git's `-- <path>` is
    // always well-formed regardless of whether the path matches an existing
    // file (setup_revisions emits an empty walk for non-matching paths, not
    // a fatal — the fatal is only for BEFORE-`--` args that name neither a
    // rev nor a path). Today gix ignores the pathspec entirely; both sides
    // exit 0 with output divergence, which compat_effect rows capture.
    let _path = opts.path.as_ref();
    log_all(repo, out, opts)
}

fn log_all(repo: gix::Repository, out: &mut dyn std::io::Write, opts: Options) -> Result<(), anyhow::Error> {
    let mut tips: Vec<gix::ObjectId> = Vec::new();
    let mut ends: Vec<gix::ObjectId> = Vec::new();

    // --all / --branches / --tags / --remotes pseudo-ref selectors. Each
    // iterates a ref category and appends peeled commit ids as tips.
    // `git log --all` is equivalent to `--branches --tags --remotes` plus
    // HEAD; enabling all four category booleans matches that shape.
    let (want_branches, want_tags, want_remotes) = if opts.all {
        (true, true, true)
    } else {
        (opts.branches, opts.tags, opts.remotes)
    };
    // flatten() silently drops per-ref read errors — matches branch.rs's
    // list listing pattern and git's own behavior of skipping unreadable refs.
    // peel_to_id may still fail per-ref (e.g. corrupt packed-refs entry) and
    // is handled via `if let Ok(...)`.
    if want_branches {
        for mut r in repo.references()?.local_branches()?.flatten() {
            if let Ok(id) = r.peel_to_id() {
                tips.push(id.detach());
            }
        }
    }
    if want_tags {
        for mut r in repo.references()?.tags()?.flatten() {
            if let Ok(id) = r.peel_to_id() {
                tips.push(id.detach());
            }
        }
    }
    if want_remotes {
        for mut r in repo.references()?.remote_branches()?.flatten() {
            if let Ok(id) = r.peel_to_id() {
                tips.push(id.detach());
            }
        }
    }

    // --follow treats its positional argument as a pathspec, not a revspec
    // (vendor/git/builtin/log.c sets rev.follow_rename and setup_revisions
    // re-classifies args under that flag). gix doesn't yet wire pathspec
    // filtering; accept the arg, skip rev_parse, fall through to HEAD.
    let revspec = if opts.follow { None } else { opts.revspec };

    // Explicit revspec — additive with the pseudo-ref selectors above.
    match revspec {
        Some(spec) => match repo.rev_parse(spec.as_bstr()) {
            Ok(parsed) => match parsed.detach() {
                gix::revision::plumbing::Spec::Include(id) | gix::revision::plumbing::Spec::ExcludeParents(id) => {
                    tips.push(id);
                }
                gix::revision::plumbing::Spec::Exclude(_) => {
                    // A bare `^rev` with no included side: git's setup_revisions
                    // emits "fatal: empty revision range" when no pseudo-ref was
                    // supplied either. For now accept the empty walk when the
                    // pseudo-refs supplied zero tips; wording parity is a later
                    // row.
                }
                gix::revision::plumbing::Spec::Range { from, to } => {
                    tips.push(to);
                    ends.push(from);
                }
                gix::revision::plumbing::Spec::Merge { theirs, ours } => {
                    let base = repo
                        .merge_base(theirs, ours)
                        .map(gix::Id::detach)
                        .map_err(|e| anyhow::anyhow!("failed to resolve merge-base for '{spec}': {e}"))?;
                    tips.push(theirs);
                    tips.push(ours);
                    ends.push(base);
                }
                gix::revision::plumbing::Spec::IncludeOnlyParents(id) => {
                    let commit = repo.find_commit(id)?;
                    tips.extend(commit.parent_ids().map(gix::Id::detach));
                }
            },
            Err(_) => {
                // Parity with git's setup_revisions: unknown revision/path dies 128
                // (vendor/git/revision.c::handle_revision_arg → die). Wording is
                // git's exact 3-line stanza.
                eprintln!(
                    "fatal: ambiguous argument '{spec}': unknown revision or path not in the working tree.\n\
                     Use '--' to separate paths from revisions, like this:\n\
                     'git <command> [<revision>...] -- [<file>...]'"
                );
                std::process::exit(128);
            }
        },
        None => {
            // No explicit revspec. If the user didn't pass any pseudo-ref
            // selector either, default to HEAD. If they did (e.g. `--tags`
            // in a repo with no tags), leave tips empty — git prints
            // nothing and exits 0 in that case.
            let pseudo_ref_requested = opts.all || opts.branches || opts.tags || opts.remotes;
            if !pseudo_ref_requested {
                let mut head = repo.head()?;
                // Parity with git: unborn HEAD (fresh `git init`, no commits)
                // dies 128 with "fatal: your current branch '<short>' does not
                // have any commits yet" (vendor/git/builtin/log.c →
                // cmd_log_walk → revision.c::die).
                if let gix::head::Kind::Unborn(name) = &head.kind {
                    eprintln!(
                        "fatal: your current branch '{}' does not have any commits yet",
                        name.shorten()
                    );
                    std::process::exit(128);
                }
                tips.push(head.peel_to_commit()?.id);
            }
        }
    }

    let topo = gix::traverse::commit::topo::Builder::from_iters(&repo.objects, tips, Some(ends)).build()?;

    // Parent-count filter family: --no-merges / --merges / --min-parents /
    // --max-parents. git's vendor/git/revision.c sets rev->min_parents and
    // rev->max_parents (where --merges = min=2, --no-merges = max=1) and
    // commit_rewrite_parents skips mismatching commits. Equivalent predicate
    // on the Info stream here.
    let min_parents = if opts.merges { Some(2) } else { opts.min_parents };
    let max_parents = if opts.no_merges { Some(1) } else { opts.max_parents };
    let parent_filter = move |info: &Result<gix::traverse::commit::Info, _>| -> bool {
        let Ok(info) = info else {
            return true; // surface walker errors unfiltered
        };
        if min_parents.is_none() && max_parents.is_none() {
            return true;
        }
        let count = info.parent_ids.len();
        if let Some(n) = min_parents {
            if count < n {
                return false;
            }
        }
        if let Some(n) = max_parents {
            if count > n {
                return false;
            }
        }
        true
    };

    // --skip=<n> drops the first n commits AFTER filtering; --max-count=<n>
    // caps the output after that. Matches git's rev_list_info.max_count +
    // rev_list_info.skip_count in vendor/git/revision.c::get_revision.
    let mut iter: Box<dyn Iterator<Item = _>> = Box::new(topo.filter(parent_filter));
    if let Some(n) = opts.skip {
        iter = Box::new(iter.skip(n));
    }
    if let Some(n) = opts.max_count {
        iter = Box::new(iter.take(n));
    }
    for info in iter {
        let info = info?;

        write_info(&repo, &mut *out, &info)?;
    }

    Ok(())
}

fn write_info(
    repo: &gix::Repository,
    mut out: impl std::io::Write,
    info: &gix::traverse::commit::Info,
) -> Result<(), std::io::Error> {
    let commit = repo.find_commit(info.id).unwrap();

    let message = commit.message_raw_sloppy();
    let title = message.lines().next();

    writeln!(
        out,
        "{} {}",
        info.id.to_hex_with_len(8),
        title.map_or_else(|| "<no message>".into(), BString::from)
    )?;

    Ok(())
}
