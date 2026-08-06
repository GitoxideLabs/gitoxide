use std::ffi::OsStr;

use anyhow::Context;
use gix::{blame::Start, bstr::BStr, config::tree, utils::AsBStr};

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
    let mut resource_cache = repo.diff_resource_cache(
        // TODO(blame): Git uses something akin to `ToGitUnlessBinaryToTextIsPresent` here, but with a specialty: textconv output is converted
        //              to Git, which isn't happening for normal diffing. In theory, this shouldn't be a problem as it's always apples to apples,
        //              at least in theory.
        gix::diff::blob::pipeline::Mode::ToGit,
        gix::diff::blob::pipeline::WorktreeRoots {
            old_root: repo.workdir().map(ToOwned::to_owned),
            new_root: None,
        },
    )?;
    let start = start_for_blame(&repo, file.as_bstr(), &mut resource_cache)?;
    // The worktree root is only for constructing `start`; historical `OldOrSource` resources must be loaded by
    // object ID, and we make sure that worktree contents can't possibly be used.
    resource_cache.filter.roots = Default::default();
    resource_cache.clear_resource_cache_keep_allocation();
    let outcome = gix::blame::file(&repo.objects, start, cache, &mut resource_cache, file.as_ref(), options)?;
    let statistics = outcome.statistics;
    show_blame_entries(out, outcome, file.as_ref())?;

    if let Some(err) = err {
        writeln!(err, "{statistics:#?}")?;
    }
    Ok(())
}

/// Start at `HEAD`, overlaying diffable worktree contents so uncommitted changes are included.
/// Missing or binary worktree files fall back to blaming `HEAD` directly.
fn start_for_blame<'a>(
    repo: &'a gix::Repository,
    file: &'a gix::bstr::BStr,
    resources: &mut gix::diff::blob::Platform,
) -> anyhow::Result<gix::blame::Start<'a>> {
    let first_suspect: gix::ObjectId = repo.head()?.into_peeled_id()?.into();
    let Some(workdir) = repo.workdir() else {
        return Ok(Start::Commit(first_suspect));
    };
    let path = workdir.join(gix::path::from_bstr(file));
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(err) if gix::fs::io_err::is_not_found(err.kind(), err.raw_os_error()) => {
            return Ok(Start::Commit(first_suspect));
        }
        Err(err) => return Err(err).with_context(|| format!("Could not read metadata of '{}'", path.display())),
    };
    // State the correct type here so that the resource cache and its possibly converted bytes match the actual type, i.e.
    // - read the file for blobs
    // - read the symlink bytes themselves, the target path for symlinks.
    let entry_kind = if metadata.file_type().is_symlink() {
        gix::objs::tree::EntryKind::Link
    } else {
        // executable bits don't matter.
        gix::objs::tree::EntryKind::Blob
    };
    resources.set_resource(
        repo.object_hash().null(),
        entry_kind,
        file,
        gix::diff::blob::ResourceKind::OldOrSource,
        &repo.objects,
    )?;
    let contents = resources
        .resource(gix::diff::blob::ResourceKind::OldOrSource)
        .and_then(|resource| match resource.data {
            gix::diff::blob::platform::resource::Data::Buffer { buf, .. } => Some(buf.to_owned()),
            gix::diff::blob::platform::resource::Data::Binary { .. }
            | gix::diff::blob::platform::resource::Data::Missing => None,
        });

    Ok(contents
        .map(|contents| Start::Contents {
            first_suspect,
            contents: contents.into(),
        })
        .unwrap_or(Start::Commit(first_suspect)))
}

fn show_blame_entries(
    mut out: impl std::io::Write,
    outcome: gix::blame::Outcome,
    source_file_name: &BStr,
) -> Result<(), std::io::Error> {
    let num_digits_for_line_number = {
        let largest_line_number = outcome
            .entries
            .last()
            .map_or(0, |entry| entry.range_in_blamed_file().end);
        (largest_line_number.checked_ilog10().unwrap_or(0) + 1) as usize
    };

    for (entry, lines_in_hunk) in outcome.entries_with_lines() {
        for ((actual_lno, source_lno), line) in entry
            .range_in_blamed_file()
            .zip(entry.range_in_source_file())
            .zip(lines_in_hunk)
        {
            write!(
                out,
                "{short_id} {line_no:>num_digits_for_line_number$} ",
                short_id = entry.commit_id.to_hex_with_len(8),
                line_no = actual_lno + 1,
            )?;

            let source_file_name = entry.source_file_name.as_ref().map_or(source_file_name, BStr::new);
            write!(out, "{source_file_name} ")?;

            write!(
                out,
                "{src_line_no:>num_digits_for_line_number$} {line}",
                src_line_no = source_lno + 1
            )?;
        }
    }

    Ok(())
}
