use bstr::BString;
use gix_hash::ObjectId;
use gix_object::{Find, FindExt};
use imara_diff::{intern::InternedInput, Algorithm, Sink};

use crate::error::{Result, SdkError};
use crate::pool::RepoHandle;
use crate::types::{
    BlobDiff, ChangeKind, DiffEntry, DiffHunk, DiffLine, DiffLineKind, DiffStats, FileStats,
};

pub fn diff_trees(
    repo: &RepoHandle,
    old_tree_id: ObjectId,
    new_tree_id: ObjectId,
) -> Result<Vec<DiffEntry>> {
    let local = repo.to_local();
    let mut old_buf = Vec::new();
    let mut new_buf = Vec::new();

    let old_tree_iter = local
        .objects
        .find_tree_iter(&old_tree_id, &mut old_buf)
        .map_err(|e| SdkError::Git(Box::new(e)))?;
    let new_tree_iter = local
        .objects
        .find_tree_iter(&new_tree_id, &mut new_buf)
        .map_err(|e| SdkError::Git(Box::new(e)))?;

    let mut state = gix_diff::tree::State::default();
    let mut recorder = gix_diff::tree::Recorder::default();

    gix_diff::tree(old_tree_iter, new_tree_iter, &mut state, &local.objects, &mut recorder)?;

    let entries = recorder
        .records
        .into_iter()
        .filter_map(|change| {
            use gix_diff::tree::recorder::Change;
            match change {
                Change::Addition {
                    entry_mode,
                    oid,
                    path,
                    ..
                } => {
                    if entry_mode.is_tree() {
                        return None;
                    }
                    Some(DiffEntry {
                        path,
                        change: ChangeKind::Added,
                        old_mode: None,
                        new_mode: Some(entry_mode.into()),
                        old_id: None,
                        new_id: Some(oid),
                    })
                }
                Change::Deletion {
                    entry_mode,
                    oid,
                    path,
                    ..
                } => {
                    if entry_mode.is_tree() {
                        return None;
                    }
                    Some(DiffEntry {
                        path,
                        change: ChangeKind::Deleted,
                        old_mode: Some(entry_mode.into()),
                        new_mode: None,
                        old_id: Some(oid),
                        new_id: None,
                    })
                }
                Change::Modification {
                    previous_entry_mode,
                    previous_oid,
                    entry_mode,
                    oid,
                    path,
                } => {
                    if entry_mode.is_tree() {
                        return None;
                    }
                    Some(DiffEntry {
                        path,
                        change: ChangeKind::Modified,
                        old_mode: Some(previous_entry_mode.into()),
                        new_mode: Some(entry_mode.into()),
                        old_id: Some(previous_oid),
                        new_id: Some(oid),
                    })
                }
            }
        })
        .collect();

    Ok(entries)
}

pub fn diff_commits(
    repo: &RepoHandle,
    old_commit_id: ObjectId,
    new_commit_id: ObjectId,
) -> Result<Vec<DiffEntry>> {
    let local = repo.to_local();
    let mut buf = Vec::new();

    let old_commit = local
        .objects
        .find_commit(&old_commit_id, &mut buf)
        .map_err(|e| SdkError::Git(Box::new(e)))?;
    let old_tree_id = old_commit.tree();

    let mut buf2 = Vec::new();
    let new_commit = local
        .objects
        .find_commit(&new_commit_id, &mut buf2)
        .map_err(|e| SdkError::Git(Box::new(e)))?;
    let new_tree_id = new_commit.tree();

    diff_trees(repo, old_tree_id, new_tree_id)
}

pub fn diff_blob(
    repo: &RepoHandle,
    old_blob_id: ObjectId,
    new_blob_id: ObjectId,
    context_lines: u32,
) -> Result<BlobDiff> {
    let local = repo.to_local();

    let mut old_buf = Vec::new();
    let old_data = local
        .objects
        .try_find(&old_blob_id, &mut old_buf)
        .map_err(|e| SdkError::Git(e))?
        .ok_or_else(|| SdkError::ObjectNotFound(old_blob_id))?;

    if old_data.kind != gix_object::Kind::Blob {
        return Err(SdkError::InvalidObjectType {
            expected: "blob".to_string(),
            actual: old_data.kind.to_string(),
        });
    }
    let old_content = old_buf.clone();

    let mut new_buf = Vec::new();
    let new_data = local
        .objects
        .try_find(&new_blob_id, &mut new_buf)
        .map_err(|e| SdkError::Git(e))?
        .ok_or_else(|| SdkError::ObjectNotFound(new_blob_id))?;

    if new_data.kind != gix_object::Kind::Blob {
        return Err(SdkError::InvalidObjectType {
            expected: "blob".to_string(),
            actual: new_data.kind.to_string(),
        });
    }
    let new_content = new_buf;

    compute_blob_diff(old_blob_id, new_blob_id, &old_content, &new_content, context_lines)
}

fn compute_blob_diff(
    old_id: ObjectId,
    new_id: ObjectId,
    old_content: &[u8],
    new_content: &[u8],
    context_lines: u32,
) -> Result<BlobDiff> {
    let input = InternedInput::new(old_content, new_content);
    let context_size = gix_diff::blob::unified_diff::ContextSize::symmetrical(context_lines);

    let mut collector = HunkCollector::new();
    let consume_hunk = HunkConsumer {
        collector: &mut collector,
    };

    let sink = gix_diff::blob::UnifiedDiff::new(&input, consume_hunk, context_size);
    let result = imara_diff::diff(Algorithm::Histogram, &input, sink);

    match result {
        Ok(_) => {}
        Err(e) => {
            return Err(SdkError::Operation(format!("diff failed: {}", e)));
        }
    }

    let mut total_additions = 0u32;
    let mut total_deletions = 0u32;

    for hunk in &collector.hunks {
        for line in &hunk.lines {
            match line.kind {
                DiffLineKind::Addition => total_additions += 1,
                DiffLineKind::Deletion => total_deletions += 1,
                DiffLineKind::Context => {}
            }
        }
    }

    Ok(BlobDiff {
        old_id,
        new_id,
        hunks: collector.hunks,
        additions: total_additions,
        deletions: total_deletions,
    })
}

struct HunkCollector {
    hunks: Vec<DiffHunk>,
}

impl HunkCollector {
    fn new() -> Self {
        Self { hunks: Vec::new() }
    }
}

struct HunkConsumer<'a> {
    collector: &'a mut HunkCollector,
}

impl gix_diff::blob::unified_diff::ConsumeHunk for HunkConsumer<'_> {
    type Out = ();

    fn consume_hunk(
        &mut self,
        header: gix_diff::blob::unified_diff::HunkHeader,
        lines: &[(gix_diff::blob::unified_diff::DiffLineKind, &[u8])],
    ) -> std::io::Result<()> {
        let mut diff_lines = Vec::with_capacity(lines.len());

        for &(kind, content) in lines {
            let line_kind = match kind {
                gix_diff::blob::unified_diff::DiffLineKind::Context => DiffLineKind::Context,
                gix_diff::blob::unified_diff::DiffLineKind::Add => DiffLineKind::Addition,
                gix_diff::blob::unified_diff::DiffLineKind::Remove => DiffLineKind::Deletion,
            };

            diff_lines.push(DiffLine {
                kind: line_kind,
                content: BString::from(content),
            });
        }

        self.collector.hunks.push(DiffHunk {
            old_start: header.before_hunk_start,
            old_lines: header.before_hunk_len,
            new_start: header.after_hunk_start,
            new_lines: header.after_hunk_len,
            lines: diff_lines,
        });

        Ok(())
    }

    fn finish(self) -> Self::Out {}
}

pub fn diff_stats(
    repo: &RepoHandle,
    old_tree_id: ObjectId,
    new_tree_id: ObjectId,
) -> Result<DiffStats> {
    let entries = diff_trees(repo, old_tree_id, new_tree_id)?;
    let local = repo.to_local();

    let mut stats = DiffStats {
        files_changed: entries.len(),
        additions: 0,
        deletions: 0,
        entries: Vec::with_capacity(entries.len()),
    };

    for entry in entries {
        let (additions, deletions) = match entry.change {
            ChangeKind::Added => {
                if let Some(new_id) = entry.new_id {
                    let line_count = count_lines_in_blob(&local.objects, new_id)?;
                    (line_count, 0)
                } else {
                    (0, 0)
                }
            }
            ChangeKind::Deleted => {
                if let Some(old_id) = entry.old_id {
                    let line_count = count_lines_in_blob(&local.objects, old_id)?;
                    (0, line_count)
                } else {
                    (0, 0)
                }
            }
            ChangeKind::Modified => {
                if let (Some(old_id), Some(new_id)) = (entry.old_id, entry.new_id) {
                    compute_line_stats(&local.objects, old_id, new_id)?
                } else {
                    (0, 0)
                }
            }
        };

        stats.additions += additions;
        stats.deletions += deletions;

        stats.entries.push(FileStats {
            path: entry.path,
            additions,
            deletions,
        });
    }

    Ok(stats)
}

fn count_lines_in_blob<O: Find>(objects: &O, blob_id: ObjectId) -> Result<u32> {
    let mut buf = Vec::new();
    let data = objects
        .try_find(&blob_id, &mut buf)
        .map_err(|e| SdkError::Git(e))?
        .ok_or_else(|| SdkError::ObjectNotFound(blob_id))?;

    if data.kind != gix_object::Kind::Blob {
        return Ok(0);
    }

    let line_count = buf.split(|&b| b == b'\n').count() as u32;
    Ok(line_count)
}

fn compute_line_stats<O: Find>(
    objects: &O,
    old_id: ObjectId,
    new_id: ObjectId,
) -> Result<(u32, u32)> {
    let mut old_buf = Vec::new();
    let old_data = objects
        .try_find(&old_id, &mut old_buf)
        .map_err(|e| SdkError::Git(e))?
        .ok_or_else(|| SdkError::ObjectNotFound(old_id))?;

    if old_data.kind != gix_object::Kind::Blob {
        return Ok((0, 0));
    }
    let old_content = old_buf.clone();

    let mut new_buf = Vec::new();
    let new_data = objects
        .try_find(&new_id, &mut new_buf)
        .map_err(|e| SdkError::Git(e))?
        .ok_or_else(|| SdkError::ObjectNotFound(new_id))?;

    if new_data.kind != gix_object::Kind::Blob {
        return Ok((0, 0));
    }
    let new_content = new_buf;

    let input = InternedInput::new(old_content.as_slice(), new_content.as_slice());
    let counter = LineStatsCounter::default();
    let result = imara_diff::diff(Algorithm::Histogram, &input, counter);

    Ok((result.insertions, result.removals))
}

#[derive(Default)]
struct LineStatsCounter {
    insertions: u32,
    removals: u32,
}

impl Sink for LineStatsCounter {
    type Out = LineStatsCounter;

    fn process_change(&mut self, before: std::ops::Range<u32>, after: std::ops::Range<u32>) {
        self.removals += before.end - before.start;
        self.insertions += after.end - after.start;
    }

    fn finish(self) -> Self::Out {
        self
    }
}

pub fn diff_trees_with_filter<F>(
    repo: &RepoHandle,
    old_tree_id: ObjectId,
    new_tree_id: ObjectId,
    path_filter: F,
) -> Result<Vec<DiffEntry>>
where
    F: Fn(&[u8]) -> bool,
{
    let entries = diff_trees(repo, old_tree_id, new_tree_id)?;
    Ok(entries
        .into_iter()
        .filter(|e| path_filter(e.path.as_ref()))
        .collect())
}

pub fn diff_blob_inline(
    old_content: &[u8],
    new_content: &[u8],
    context_lines: u32,
) -> Result<BlobDiff> {
    let null_id = ObjectId::null(gix_hash::Kind::Sha1);
    compute_blob_diff(null_id, null_id, old_content, new_content, context_lines)
}
