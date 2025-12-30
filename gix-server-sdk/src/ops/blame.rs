use bstr::{BStr, BString};
use gix_diff::blob::intern::TokenSource;
use gix_hash::ObjectId;

use crate::error::{Result, SdkError};
use crate::pool::RepoHandle;
use crate::types::{BlameEntry, BlameOptions, BlameResult, BlameStatistics};

pub fn blame_file(
    repo: &RepoHandle,
    commit_id: ObjectId,
    file_path: &BStr,
    options: BlameOptions,
) -> Result<BlameResult> {
    let local = repo.to_local();
    let mut resource_cache = create_resource_cache(&local)?;

    let blame_ranges = if let Some((start, end)) = options.range {
        gix_blame::BlameRanges::from_one_based_inclusive_range(start..=end)
            .map_err(|e| SdkError::Git(Box::new(e)))?
    } else {
        gix_blame::BlameRanges::default()
    };

    let rewrites = if options.follow_renames {
        Some(gix_diff::Rewrites::default())
    } else {
        None
    };

    let blame_options = gix_blame::Options {
        diff_algorithm: gix_diff::blob::Algorithm::Histogram,
        ranges: blame_ranges,
        since: None,
        rewrites,
        debug_track_path: false,
    };

    let cache = local.commit_graph_if_enabled().ok().flatten();

    let outcome = gix_blame::file(
        &local.objects,
        commit_id,
        cache,
        &mut resource_cache,
        file_path,
        blame_options,
    )
    .map_err(|e| SdkError::Git(Box::new(e)))?;

    let lines = extract_lines(&outcome.blob);

    let entries = outcome
        .entries
        .into_iter()
        .map(|e| BlameEntry {
            commit_id: e.commit_id,
            start_line: e.start_in_blamed_file + 1,
            line_count: e.len.get(),
            original_start_line: e.start_in_source_file + 1,
            original_path: e.source_file_name,
        })
        .collect();

    let statistics = BlameStatistics {
        commits_traversed: outcome.statistics.commits_traversed,
        blobs_diffed: outcome.statistics.blobs_diffed,
    };

    Ok(BlameResult {
        entries,
        lines,
        statistics,
    })
}

fn extract_lines(blob: &[u8]) -> Vec<BString> {
    gix_diff::blob::sources::byte_lines_with_terminator(blob)
        .tokenize()
        .map(|line| BString::from(line))
        .collect()
}

fn create_resource_cache(repo: &gix::Repository) -> Result<gix_diff::blob::Platform> {
    let git_dir = repo.git_dir();
    let worktree_path = repo.workdir().unwrap_or(git_dir);

    let index = match gix_index::File::at(
        git_dir.join("index"),
        gix_hash::Kind::Sha1,
        false,
        Default::default(),
    ) {
        Ok(index) => index,
        Err(_) => {
            return Ok(create_minimal_platform(worktree_path));
        }
    };

    let stack = gix_worktree::Stack::from_state_and_ignore_case(
        worktree_path.to_path_buf(),
        false,
        gix_worktree::stack::State::AttributesAndIgnoreStack {
            attributes: Default::default(),
            ignore: Default::default(),
        },
        &index,
        index.path_backing(),
    );

    let capabilities = gix_fs::Capabilities::probe(git_dir);

    let platform = gix_diff::blob::Platform::new(
        Default::default(),
        gix_diff::blob::Pipeline::new(
            gix_diff::blob::pipeline::WorktreeRoots {
                old_root: None,
                new_root: None,
            },
            gix_filter::Pipeline::new(Default::default(), Default::default()),
            vec![],
            gix_diff::blob::pipeline::Options {
                large_file_threshold_bytes: 0,
                fs: capabilities,
            },
        ),
        gix_diff::blob::pipeline::Mode::ToGit,
        stack,
    );

    Ok(platform)
}

fn create_minimal_platform(worktree_path: &std::path::Path) -> gix_diff::blob::Platform {
    let empty_index = gix_index::State::new(gix_hash::Kind::Sha1);

    let stack = gix_worktree::Stack::from_state_and_ignore_case(
        worktree_path.to_path_buf(),
        false,
        gix_worktree::stack::State::AttributesAndIgnoreStack {
            attributes: Default::default(),
            ignore: Default::default(),
        },
        &empty_index,
        &[],
    );

    let capabilities = gix_fs::Capabilities::probe(worktree_path);

    gix_diff::blob::Platform::new(
        Default::default(),
        gix_diff::blob::Pipeline::new(
            gix_diff::blob::pipeline::WorktreeRoots {
                old_root: None,
                new_root: None,
            },
            gix_filter::Pipeline::new(Default::default(), Default::default()),
            vec![],
            gix_diff::blob::pipeline::Options {
                large_file_threshold_bytes: 0,
                fs: capabilities,
            },
        ),
        gix_diff::blob::pipeline::Mode::ToGit,
        stack,
    )
}
