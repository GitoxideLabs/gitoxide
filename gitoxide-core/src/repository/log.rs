use gix::{
    Result,
    bstr::{BString, ByteSlice},
    error::{ResultExt, bail, message, unsupported},
    traverse::commit::{Parents, topo},
};

/// Configure the starting commits, hidden ancestry, and order of a log traversal.
#[derive(Default)]
pub struct Options {
    /// Revisions to start from, or `HEAD` if empty.
    pub tips: Vec<BString>,
    /// Revisions to exclude along with all of their ancestors.
    pub hidden: Vec<BString>,
    /// The order in which to return commits.
    pub sorting: topo::Sorting,
    /// Which parents to follow when walking visible commits.
    pub parents: Parents,
}

pub fn log(
    mut repo: gix::Repository,
    out: &mut dyn std::io::Write,
    path: Option<BString>,
    options: Options,
) -> Result<()> {
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));

    if let Some(path) = path {
        log_file(repo, out, path)
    } else {
        log_all(repo, out, options)
    }
}

fn log_all(repo: gix::Repository, out: &mut dyn std::io::Write, options: Options) -> Result<()> {
    let tips = if options.tips.is_empty() {
        vec![repo.head()?.peel_to_commit()?.id()]
    } else {
        options
            .tips
            .iter()
            .map(|spec| commit_id(&repo, spec.as_bstr()))
            .collect::<Result<Vec<_>>>()?
    };
    let hidden = options
        .hidden
        .iter()
        .map(|spec| commit_id(&repo, spec.as_bstr()))
        .collect::<Result<Vec<_>>>()?;
    // Invalid configuration is actionable, but a broken acceleration cache must not prevent reading intact commits.
    let _ = repo.config_snapshot().try_boolean("core.commitGraph")?;
    let commit_graph = repo.commit_graph_if_enabled().unwrap_or_default();
    let topo = topo::Builder::from_iters(&repo.objects, tips, Some(hidden))
        .sorting(options.sorting)
        .parents(options.parents)
        .with_commit_graph(commit_graph)
        .build()?;

    for info in topo {
        let info = info?;

        write_info(&repo, &mut *out, &info)?;
    }

    Ok(())
}

fn commit_id<'repo>(repo: &'repo gix::Repository, spec: &gix::bstr::BStr) -> Result<gix::Id<'repo>> {
    Ok(repo
        .rev_parse_single(spec)
        .or_raise(|| message!("Could not resolve commit {spec}"))?
        .object()?
        .peel_to_kind(gix::object::Kind::Commit)
        .or_raise(|| message!("Revision {spec} must refer to a commit"))?
        .id())
}

fn log_file(_repo: gix::Repository, _out: &mut dyn std::io::Write, _path: BString) -> Result<()> {
    bail!(unsupported(
        "File-based lookup isn't yet implemented in a way that is competitively fast"
    ));
}

fn write_info(repo: &gix::Repository, mut out: impl std::io::Write, info: &gix::traverse::commit::Info) -> Result<()> {
    let commit = repo.find_commit(info.id)?;

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
