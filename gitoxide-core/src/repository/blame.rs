use std::ffi::OsStr;

use gix::{blame::Start, bstr::BStr, config::tree};

pub fn blame_file(
    mut repo: gix::Repository,
    file: &OsStr,
    options: gix::blame::Options,
    out: impl std::io::Write,
    err: Option<&mut dyn std::io::Write>,
) -> anyhow::Result<()> {
    {
        let mut config = repo.config_snapshot_mut();
        if config.string(tree::Core::DELTA_BASE_CACHE_LIMIT).is_none() {
            config.set_value(&tree::Core::DELTA_BASE_CACHE_LIMIT, "100m")?;
        }
    }
    let index = repo.index_or_empty()?;
    repo.object_cache_size_if_unset(repo.compute_object_cache_size_for_tree_diffs(&index));

    let file = gix::path::os_str_into_bstr(file)?;
    let file = repo.normalize_path(file)?;

    let cache: Option<gix::commitgraph::Graph> = repo.commit_graph_if_enabled()?;
    let mut resource_cache = repo.diff_resource_cache_for_tree_diff()?;
    let outcome = gix::blame::file(
        &repo.objects,
        start_for_blame(&repo, file.as_bstr())?,
        cache,
        &mut resource_cache,
        file.as_ref(),
        options,
    )?;
    let statistics = outcome.statistics;
    show_blame_entries(out, outcome, file.as_ref())?;

    if let Some(err) = err {
        writeln!(err, "{statistics:#?}")?;
    }
    Ok(())
}

fn start_for_blame<'a>(repo: &'a gix::Repository, file: &'a gix::bstr::BStr) -> anyhow::Result<gix::blame::Start<'a>> {
    let worktree_roots = gix::diff::blob::pipeline::WorktreeRoots {
        old_root: repo.workdir().map(ToOwned::to_owned),
        new_root: None,
    };
    let mut filter = gix::diff::blob::Pipeline::new(worktree_roots, Default::default(), vec![], Default::default());
    let mut buf = Vec::new();
    let outcome = filter.convert_to_diffable(
        &repo.object_hash().null(),
        gix::objs::tree::EntryKind::Blob,
        file,
        gix::diff::blob::ResourceKind::OldOrSource,
        &mut |_, _| {},
        &repo.objects,
        gix::diff::blob::pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
        &mut buf,
    )?;

    let first_suspect: gix::ObjectId = repo.head()?.into_peeled_id()?.into();

    Ok(outcome
        .data
        .and_then(|data| match data {
            gix::diff::blob::pipeline::Data::Buffer { .. } => Some(Start::Contents {
                first_suspect,
                contents: buf.into(),
            }),
            gix::diff::blob::pipeline::Data::Binary { .. } => None,
        })
        .unwrap_or(Start::Commit(first_suspect)))
}

fn show_blame_entries(
    mut out: impl std::io::Write,
    outcome: gix::blame::Outcome,
    source_file_name: &BStr,
) -> Result<(), std::io::Error> {
    let last_line_no = outcome
        .entries
        .last()
        .map_or(0, |entry| entry.range_in_blamed_file().end);
    let number_of_digits = (last_line_no.ilog10() + 1) as usize;

    for (entry, lines_in_hunk) in outcome.entries_with_lines() {
        for ((actual_lno, source_lno), line) in entry
            .range_in_blamed_file()
            .zip(entry.range_in_source_file())
            .zip(lines_in_hunk)
        {
            write!(
                out,
                "{short_id} {line_no:>number_of_digits$} ",
                short_id = entry.commit_id.to_hex_with_len(8),
                line_no = actual_lno + 1,
            )?;

            let source_file_name = entry.source_file_name.as_ref().map_or(source_file_name, BStr::new);
            write!(out, "{source_file_name} ")?;

            write!(
                out,
                "{src_line_no:>number_of_digits$} {line}",
                src_line_no = source_lno + 1
            )?;
        }
    }

    Ok(())
}
