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

    let suspect: gix::ObjectId = repo.head()?.into_peeled_id()?.into();
    let cache: Option<gix::commitgraph::Graph> = repo.commit_graph_if_enabled()?;
    let mut resource_cache = repo.diff_resource_cache_for_tree_diff()?;
    let outcome = gix::blame::file(
        &repo.objects,
        Start::Commit(suspect),
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

fn show_blame_entries(
    mut out: impl std::io::Write,
    outcome: gix::blame::Outcome,
    source_file_name: &BStr,
) -> Result<(), std::io::Error> {
    for (entry, lines_in_hunk) in outcome.entries_with_lines() {
        for ((actual_lno, source_lno), line) in entry
            .range_in_blamed_file()
            .zip(entry.range_in_source_file())
            .zip(lines_in_hunk)
        {
            write!(
                out,
                "{short_id} {line_no} ",
                short_id = entry.commit_id.to_hex_with_len(8),
                line_no = actual_lno + 1,
            )?;

            let source_file_name = entry.source_file_name.as_ref().map_or(source_file_name, BStr::new);
            write!(out, "{source_file_name} ")?;

            write!(out, "{src_line_no} {line}", src_line_no = source_lno + 1)?;
        }
    }

    Ok(())
}
