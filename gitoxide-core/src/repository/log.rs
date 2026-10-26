use gix::{
    Result,
    bstr::{BString, ByteSlice},
    error::{ResultExt, bail, message, unsupported},
    revision::{plumbing::Spec, walk::Sorting},
    traverse::commit::Parents,
};

/// Configure the starting commits, hidden ancestry, and order of a log traversal.
pub struct Options {
    /// Revision expressions to walk, including exclusions and ranges, or `HEAD` if empty.
    pub revisions: Vec<BString>,
    /// The order in which to return commits, defaulting to [`Sorting::DateOrder`].
    pub sorting: Sorting,
    /// Which parents to follow when walking visible commits.
    pub parents: Parents,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            revisions: Vec::new(),
            sorting: Sorting::DateOrder,
            parents: Parents::default(),
        }
    }
}

pub fn log(
    mut repo: gix::Repository,
    out: &mut dyn std::io::Write,
    pathspecs: Vec<BString>,
    options: Options,
) -> Result<()> {
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));

    if !pathspecs.is_empty() {
        bail!(unsupported(
            "Pathspec-limited history is not implemented yet; use `git log` for path filtering or omit the pathspecs"
        ));
    }
    log_all(repo, out, options)
}

fn log_all(repo: gix::Repository, out: &mut dyn std::io::Write, options: Options) -> Result<()> {
    let mut tips = Vec::new();
    if options.revisions.is_empty() {
        tips.push(repo.head()?.peel_to_commit()?.id());
    }
    let mut hidden = Vec::new();
    for revision in &options.revisions {
        add_revision(&repo, revision.as_bstr(), &mut tips, &mut hidden)
            .or_raise(|| message!("Could not resolve revision {revision}"))?;
    }
    let _ = repo.config_snapshot().try_boolean("core.commitGraph")?;
    let mut walk = repo.rev_walk(tips).with_hidden(hidden).sorting(options.sorting);
    if matches!(options.parents, Parents::First) {
        walk = walk.first_parent_only();
    }

    for info in walk.all()? {
        let info = info?;

        write_info(&mut *out, &info)?;
    }

    Ok(())
}

fn add_revision<'repo>(
    repo: &'repo gix::Repository,
    revision: &gix::bstr::BStr,
    tips: &mut Vec<gix::Id<'repo>>,
    hidden: &mut Vec<gix::Id<'repo>>,
) -> Result<()> {
    match *repo.rev_parse(revision)? {
        Spec::Include(commit_id) => tips.push(commit(repo, commit_id)?.id()),
        Spec::Exclude(commit_id) => hidden.push(commit(repo, commit_id)?.id()),
        Spec::Range { from, to } => {
            hidden.push(commit(repo, from)?.id());
            tips.push(commit(repo, to)?.id());
        }
        Spec::Merge { theirs, ours } => {
            let theirs = commit(repo, theirs)?.id();
            let ours = commit(repo, ours)?.id();
            hidden.extend(repo.merge_bases_many(theirs, &[ours.into()])?);
            tips.extend([theirs, ours]);
        }
        Spec::IncludeOnlyParents(commit_id) => tips.extend(commit(repo, commit_id)?.parent_ids()),
        Spec::ExcludeParents(commit_id) => {
            let commit = commit(repo, commit_id)?;
            tips.push(commit.id());
            hidden.extend(commit.parent_ids());
        }
    }
    Ok(())
}

fn commit(repo: &gix::Repository, object_id: gix::ObjectId) -> Result<gix::Commit<'_>> {
    Ok(repo
        .find_object(object_id)?
        .peel_to_kind(gix::object::Kind::Commit)
        .or_raise(|| message!("Object {object_id} must refer to a commit"))?
        .into_commit())
}

fn write_info(mut out: impl std::io::Write, info: &gix::revision::walk::Info<'_>) -> Result<()> {
    let commit = info.object()?;

    let message = commit.message_raw_sloppy();
    let title = message.lines().next();

    writeln!(
        out,
        "{} {}",
        info.id.to_hex_with_len(8),
        title.map_or_else(|| "<no message>".into(), BString::from)
    )
    .or_error()?;

    Ok(())
}
