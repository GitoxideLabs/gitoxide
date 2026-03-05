use gix::bstr::{BStr, BString, ByteSlice};

pub fn log(mut repo: gix::Repository, out: &mut dyn std::io::Write, path: Option<BString>) -> anyhow::Result<()> {
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&**repo.index_or_empty()?));

    if let Some(path) = path {
        log_file(repo, out, path)
    } else {
        log_all(repo, out)
    }
}

fn log_all(repo: gix::Repository, out: &mut dyn std::io::Write) -> Result<(), anyhow::Error> {
    let head = repo.head()?.peel_to_commit()?;
    let topo = gix::traverse::commit::topo::Builder::from_iters(&repo.objects, [head.id], None::<Vec<gix::ObjectId>>)
        .build()?;

    for info in topo {
        let info = info?;

        write_info(&repo, &mut *out, &info)?;
    }

    Ok(())
}

fn log_file(repo: gix::Repository, out: &mut dyn std::io::Write, path: BString) -> anyhow::Result<()> {
    let path = gix::path::to_unix_separators_on_windows(path.as_bstr()).into_owned();
    let head = repo.head()?.peel_to_commit()?;
    let cache = repo.commit_graph_if_enabled()?;
    let topo = gix::traverse::commit::topo::Builder::from_iters(&repo.objects, [head.id], None::<Vec<gix::ObjectId>>)
        .build()?;

    for info in topo {
        let info = info?;
        if commit_changes_path(&repo, cache.as_ref(), &info, path.as_ref())? {
            write_info(&repo, &mut *out, &info)?;
        }
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

fn commit_changes_path(
    repo: &gix::Repository,
    cache: Option<&gix::commitgraph::Graph>,
    info: &gix::traverse::commit::Info,
    path: &BStr,
) -> anyhow::Result<bool> {
    let commit = repo.find_commit(info.id)?;
    let commit_tree = commit.tree()?;
    let commit_entry = lookup_path_entry(&commit_tree, path)?;

    if info.parent_ids.is_empty() {
        return Ok(commit_entry.is_some());
    }

    for (index, parent_id) in info.parent_ids.iter().enumerate() {
        if index == 0 && cache.and_then(|graph| graph.maybe_contains_path_by_id(info.id, path)) == Some(false) {
            continue;
        }

        let parent = repo.find_commit(*parent_id)?;
        let parent_tree = parent.tree()?;
        let parent_entry = lookup_path_entry(&parent_tree, path)?;
        if commit_entry != parent_entry {
            return Ok(true);
        }
    }

    Ok(false)
}

fn lookup_path_entry(
    tree: &gix::Tree<'_>,
    path: &BStr,
) -> anyhow::Result<Option<(gix::objs::tree::EntryMode, gix::ObjectId)>> {
    let entry = tree.lookup_entry(path.split(|b| *b == b'/'))?;
    Ok(entry.map(|entry| (entry.mode(), entry.object_id())))
}
