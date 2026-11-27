use gix_error::Result;
use std::num::NonZeroU32;

use gix_diff::{blob::TokenSource, tree::Visit};
use gix_error::{ErrorExt, OptionExt, ResultExt, bail, message};
use gix_hash::ObjectId;
use gix_object::{
    FindExt,
    bstr::{BStr, BString, ByteSlice},
};
use gix_revision::{Graph, PriorityQueue, graph::LazyCommit};
use smallvec::SmallVec;

use super::{Change, UnblamedHunk, process_changes};
use crate::{
    BlameEntry, Options, Outcome, Statistics,
    types::{BlamePathEntry, Start},
};

/// Commit metadata retained in the revision graph by [`file()`].
#[derive(Default)]
pub struct CommitMetadata {
    root_tree_id: Option<ObjectId>,
    commit_time: Option<gix_date::SecondsSinceUnixEpoch>,
    parents: Option<SmallVec<[ObjectId; 1]>>,
}

/// Produce a list of consecutive [`BlameEntry`] instances to indicate in which commits the ranges of the file
/// at `suspect:<file_path>` originated in.
///
/// ## Parameters
///
/// * `odb`
///    - Access to database objects, also for used for diffing.
///    - Should have an object cache for good diff performance.
/// * `start`
///    - Where to start the blame. Can be either a commit or contents of a worktree file that contains
///      untracked changes.
/// * `graph`
///    - Looks up commits, using its optional on-disk commit-graph when available. Reuse it across calls
///      to retain decoded commit metadata. It must describe the same history as `odb`.
/// * `resource_cache`
///    - Used for diffing trees.
/// * `file_path`
///    - A *slash-separated* worktree-relative path to the file to blame.
/// * `options`
///    - An instance of [`Options`].
///
/// ## Errors
///
/// Comparisons between historical blobs require both diff resources to exist and be text. If the
/// configured pipeline produces missing or binary resources, blame returns an error instead of
/// treating them as empty. Text-converted resources are also rejected because blame ranges and
/// returned content use the original blobs, whose line numbers may differ from converted text.
///
/// ## The algorithm
///
/// *For brevity, `HEAD` denotes the starting point of the blame operation. It could be any commit, or even commits that
/// represent the worktree state.
///
/// We begin with one or more *Unblamed Hunks* and a single suspect, usually the `HEAD` commit as the commit containing the
/// *Blamed File*, so that it contains the entire file, with the first commit being a candidate for the entire *Blamed File*.
/// We traverse the commit graph starting at the first suspect, and see if there have been changes to `file_path`.
/// If so, we have found a *Source File* and a *Suspect* commit, and have hunks that represent these changes.
/// Now the *Unblamed Hunk* is split at the boundaries of each matching change, creating a new *Unblamed Hunk* on each side,
/// along with a [`BlameEntry`] to represent the match.
/// This is repeated until there are no non-empty *Unblamed Hunk*s left.
///
/// At a high level, what we want to do is the following:
///
/// - get the commit
/// - walk through its parents
///   - for each parent, do a diff and mark lines that don’t have a suspect yet (this is the term
///     used in `libgit2`), but that have been changed in this commit
///
/// The algorithm in `libgit2` works by going through parents and keeping a linked list of blame
/// suspects. It can be visualized as follows:
///
/// <---------------------------------------->
/// <---------------><----------------------->
/// <---><----------><----------------------->
/// <---><----------><-------><-----><------->
/// <---><---><-----><-------><-----><------->
/// <---><---><-----><-------><-----><-><-><->
pub fn file(
    odb: impl gix_object::Find + gix_object::FindHeader,
    start: Start,
    graph: &mut Graph<'_, '_, CommitMetadata>,
    resource_cache: &mut gix_diff::blob::Platform,
    file_path: &BStr,
    options: Options,
) -> Result<Outcome> {
    let _span = gix_trace::coarse!("gix_blame::file()", ?file_path, ?start);

    let mut stats = Statistics::default();
    let (mut buf, mut buf2, mut buf3) = (Vec::new(), Vec::new(), Vec::new());

    let InitialState {
        blamed_file_blob,
        mut hunks_to_blame,
        mut out,
        first_suspect,
    } = initial_state(&odb, start, graph, file_path, &options, &mut buf, &mut buf2, &mut stats)?;

    if hunks_to_blame.is_empty() {
        out.sort_by_key(|entry| entry.start_in_blamed_file);
        return Ok(Outcome {
            entries: coalesce_blame_entries(out),
            blob: blamed_file_blob,
            statistics: stats,
            blame_path: None,
        });
    }

    let mut queue = PriorityQueue::new();

    if let Some(first_suspect) = first_suspect {
        queue.insert(commit_time(graph, &first_suspect)?, first_suspect);
    }

    let mut diff_state = gix_diff::tree::State::default();
    let mut previous_entry: Option<(ObjectId, ObjectId)> = None;
    let mut blame_path = if options.debug_track_path {
        Some(Vec::new())
    } else {
        None
    };

    'outer: while let Some(suspect) = queue.pop_value() {
        stats.commits_traversed += 1;
        if hunks_to_blame.is_empty() {
            break;
        }

        let first_hunk_for_suspect = hunks_to_blame.iter().find(|hunk| hunk.has_suspect(&suspect));
        let Some(first_hunk_for_suspect) = first_hunk_for_suspect else {
            // There are no `UnblamedHunk`s associated with this `suspect`, so we can continue with
            // the next one.
            continue 'outer;
        };

        let current_file_path = first_hunk_for_suspect
            .source_file_name
            .clone()
            .unwrap_or_else(|| file_path.to_owned());

        let commit_time = commit_time(graph, &suspect)?;

        if let Some(since) = options.since
            && commit_time < since.seconds
        {
            if unblamed_to_out_is_done(&mut hunks_to_blame, &mut out, suspect) {
                break 'outer;
            }

            continue;
        }

        let parent_ids = collect_parents(graph, &suspect)?;

        if parent_ids.is_empty() {
            if queue.is_empty() {
                // I’m not entirely sure if this is correct yet. `suspect`, at this point, is the
                // `id` of the last `item` that was yielded by `queue`, so it makes sense to assign
                // the remaining lines to it, even though we don’t explicitly check whether that is
                // true here. We could perhaps use diff-tree-to-tree to compare `suspect` against
                // an empty tree to validate this assumption.
                if unblamed_to_out_is_done(&mut hunks_to_blame, &mut out, suspect) {
                    if let Some(ref mut blame_path) = blame_path {
                        let entry = previous_entry
                            .take()
                            .filter(|(id, _)| *id == suspect)
                            .map(|(_, entry)| entry);

                        let blame_path_entry = BlamePathEntry {
                            source_file_path: current_file_path.clone(),
                            previous_source_file_path: None,
                            commit_id: suspect,
                            blob_id: entry.unwrap_or(gix_hash::Kind::shortest().null()),
                            previous_blob_id: gix_hash::Kind::shortest().null(),
                            parent_index: 0,
                        };
                        blame_path.push(blame_path_entry);
                    }

                    break 'outer;
                }
            }
            // There is more, keep looking.
            continue;
        }

        let mut entry = previous_entry
            .take()
            .filter(|(id, _)| *id == suspect)
            .map(|(_, entry)| entry);
        if entry.is_none() {
            entry = find_path_entry_in_commit(
                &odb,
                &suspect,
                current_file_path.as_ref(),
                graph,
                &mut buf,
                &mut buf2,
                &mut stats,
            )?;
        }

        let Some(entry_id) = entry else {
            continue;
        };

        // This block asserts that, for every `UnblamedHunk`, all lines in the *Blamed File* are
        // identical to the corresponding lines in the *Source File*.
        #[cfg(debug_assertions)]
        {
            let source_blob = odb.find_blob(&entry_id, &mut buf)?.data.to_vec();
            let mut source_interner = gix_diff::blob::Interner::new(source_blob.len() / 100);
            let source_lines_as_tokens: Vec<_> = tokens_for_diffing(&source_blob)
                .tokenize()
                .map(|token| source_interner.intern(token))
                .collect();

            let mut blamed_interner = gix_diff::blob::Interner::new(blamed_file_blob.len() / 100);
            let blamed_lines_as_tokens: Vec<_> = tokens_for_diffing(&blamed_file_blob)
                .tokenize()
                .map(|token| blamed_interner.intern(token))
                .collect();

            for hunk in hunks_to_blame.iter() {
                if let Some(range_in_suspect) = hunk.get_range(&suspect) {
                    let range_in_blamed_file = hunk.range_in_blamed_file.clone();

                    let source_lines = range_in_suspect
                        .clone()
                        .map(|i| BString::new(source_interner[source_lines_as_tokens[i as usize]].into()))
                        .collect::<Vec<_>>();
                    let blamed_lines = range_in_blamed_file
                        .clone()
                        .map(|i| BString::new(blamed_interner[blamed_lines_as_tokens[i as usize]].into()))
                        .collect::<Vec<_>>();

                    assert_eq!(source_lines, blamed_lines);
                }
            }
        }

        for (pid, (parent_id, parent_commit_time)) in parent_ids.iter().enumerate() {
            if let Some(parent_entry_id) = find_path_entry_in_commit(
                &odb,
                parent_id,
                current_file_path.as_ref(),
                graph,
                &mut buf,
                &mut buf2,
                &mut stats,
            )? {
                let no_change_in_entry = entry_id == parent_entry_id;
                if pid == 0 {
                    previous_entry = Some((*parent_id, parent_entry_id));
                }
                if no_change_in_entry {
                    pass_blame_from_to(suspect, *parent_id, &mut hunks_to_blame);
                    queue.insert(*parent_commit_time, *parent_id);
                    continue 'outer;
                }
            }
        }

        let more_than_one_parent = parent_ids.len() > 1;
        for (index, (parent_id, parent_commit_time)) in parent_ids.iter().enumerate() {
            queue.insert(*parent_commit_time, *parent_id);
            let changes_for_file_path = tree_diff_at_file_path(
                &odb,
                current_file_path.as_ref(),
                suspect,
                *parent_id,
                graph,
                &mut stats,
                &mut diff_state,
                resource_cache,
                &mut buf2,
                &mut buf3,
                options.rewrites,
            )?;
            let Some(modification) = changes_for_file_path else {
                if more_than_one_parent {
                    // None of the changes affected the file we’re currently blaming.
                    // Copy blame to parent.
                    for unblamed_hunk in &mut hunks_to_blame {
                        unblamed_hunk.clone_blame(suspect, *parent_id);
                    }
                } else {
                    pass_blame_from_to(suspect, *parent_id, &mut hunks_to_blame);
                }
                continue;
            };

            match modification {
                TreeDiffChange::Addition { id } => {
                    if more_than_one_parent {
                        // Do nothing under the assumption that this always (or almost always)
                        // implies that the file comes from a different parent, compared to which
                        // it was modified, not added.
                    } else if unblamed_to_out_is_done(&mut hunks_to_blame, &mut out, suspect) {
                        if let Some(ref mut blame_path) = blame_path {
                            let blame_path_entry = BlamePathEntry {
                                source_file_path: current_file_path.clone(),
                                previous_source_file_path: None,
                                commit_id: suspect,
                                blob_id: id,
                                previous_blob_id: gix_hash::Kind::shortest().null(),
                                parent_index: index,
                            };
                            blame_path.push(blame_path_entry);
                        }

                        break 'outer;
                    }
                }
                TreeDiffChange::Deletion => {
                    unreachable!("We already found file_path in suspect^{{tree}}, so it can't be deleted")
                }
                TreeDiffChange::Modification { previous_id, id } => {
                    let changes = blob_changes(
                        &odb,
                        resource_cache,
                        id,
                        previous_id,
                        file_path,
                        file_path,
                        options.diff_algorithm,
                        &mut stats,
                    )?;
                    hunks_to_blame = process_changes(hunks_to_blame, changes.clone(), suspect, *parent_id);
                    if let Some(ref mut blame_path) = blame_path {
                        let has_blame_been_passed = hunks_to_blame.iter().any(|hunk| hunk.has_suspect(parent_id));

                        if has_blame_been_passed {
                            let blame_path_entry = BlamePathEntry {
                                source_file_path: current_file_path.clone(),
                                previous_source_file_path: Some(current_file_path.clone()),
                                commit_id: suspect,
                                blob_id: id,
                                previous_blob_id: previous_id,
                                parent_index: index,
                            };
                            blame_path.push(blame_path_entry);
                        }
                    }
                }
                TreeDiffChange::Rewrite {
                    source_location,
                    source_id,
                    id,
                } => {
                    let changes = blob_changes(
                        &odb,
                        resource_cache,
                        id,
                        source_id,
                        file_path,
                        source_location.as_ref(),
                        options.diff_algorithm,
                        &mut stats,
                    )?;
                    hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, *parent_id);

                    let mut has_blame_been_passed = false;

                    for hunk in hunks_to_blame.iter_mut() {
                        if hunk.has_suspect(parent_id) {
                            hunk.source_file_name = Some(source_location.clone());

                            has_blame_been_passed = true;
                        }
                    }

                    if has_blame_been_passed && let Some(ref mut blame_path) = blame_path {
                        let blame_path_entry = BlamePathEntry {
                            source_file_path: current_file_path.clone(),
                            previous_source_file_path: Some(source_location.clone()),
                            commit_id: suspect,
                            blob_id: id,
                            previous_blob_id: source_id,
                            parent_index: index,
                        };
                        blame_path.push(blame_path_entry);
                    }
                }
            }
        }

        hunks_to_blame.retain_mut(|unblamed_hunk| {
            if unblamed_hunk.suspects.len() == 1
                && let Some(entry) = BlameEntry::from_unblamed_hunk(unblamed_hunk, suspect)
            {
                // At this point, we have copied blame for every hunk to a parent. Hunks
                // that have only `suspect` left in `suspects` have not passed blame to any
                // parent, and so they can be converted to a `BlameEntry` and moved to
                // `out`.
                out.push(entry);
                return false;
            }
            unblamed_hunk.remove_blame(suspect);
            true
        });
    }

    debug_assert_eq!(
        hunks_to_blame,
        vec![],
        "only if there is no portion of the file left we have completed the blame"
    );

    // I don’t know yet whether it would make sense to use a data structure instead that preserves
    // order on insertion.
    out.sort_by_key(|a| a.start_in_blamed_file);
    Ok(Outcome {
        entries: coalesce_blame_entries(out),
        blob: blamed_file_blob,
        statistics: stats,
        blame_path,
    })
}

/// Pass ownership of each unblamed hunk of `from` to `to`.
///
/// This happens when `from` didn't actually change anything in the blamed file.
fn pass_blame_from_to(from: ObjectId, to: ObjectId, hunks_to_blame: &mut Vec<UnblamedHunk>) {
    for unblamed_hunk in hunks_to_blame {
        unblamed_hunk.pass_blame(from, to);
    }
}

/// Convert each of the unblamed hunk in `hunks_to_blame` into a [`BlameEntry`], consuming them in the process.
///
/// Return `true` if we are done because `hunks_to_blame` is empty.
fn unblamed_to_out_is_done(
    hunks_to_blame: &mut Vec<UnblamedHunk>,
    out: &mut Vec<BlameEntry>,
    suspect: ObjectId,
) -> bool {
    let mut without_suspect = Vec::new();
    out.extend(hunks_to_blame.drain(..).filter_map(|hunk| {
        BlameEntry::from_unblamed_hunk(&hunk, suspect).or_else(|| {
            without_suspect.push(hunk);
            None
        })
    }));
    *hunks_to_blame = without_suspect;
    hunks_to_blame.is_empty()
}

/// This function merges adjacent blame entries. It merges entries that are adjacent both in the
/// blamed file and in the source file that introduced them. This follows `git`’s
/// behaviour. `libgit2`, as of 2024-09-19, only checks whether two entries are adjacent in the
/// blamed file which can result in different blames in certain edge cases. See [the commit][1]
/// that introduced the extra check into `git` for context. See [this commit][2] for a way to test
/// for this behaviour in `git`.
///
/// [1]: https://github.com/git/git/commit/c2ebaa27d63bfb7c50cbbdaba90aee4efdd45d0a
/// [2]: https://github.com/git/git/commit/6dbf0c7bebd1c71c44d786ebac0f2b3f226a0131
fn coalesce_blame_entries(lines_blamed: Vec<BlameEntry>) -> Vec<BlameEntry> {
    let len = lines_blamed.len();
    lines_blamed
        .into_iter()
        .fold(Vec::with_capacity(len), |mut acc, entry| {
            let previous_entry = acc.last();

            if let Some(previous_entry) = previous_entry {
                let previous_blamed_range = previous_entry.range_in_blamed_file();
                let current_blamed_range = entry.range_in_blamed_file();
                let previous_source_range = previous_entry.range_in_source_file();
                let current_source_range = entry.range_in_source_file();
                if previous_entry.commit_id == entry.commit_id
                    && previous_blamed_range.end == current_blamed_range.start
                    // As of 2024-09-19, the check below only is in `git`, but not in `libgit2`.
                    && previous_source_range.end == current_source_range.start
                {
                    let coalesced_entry = BlameEntry {
                        start_in_blamed_file: previous_blamed_range.start as u32,
                        start_in_source_file: previous_source_range.start as u32,
                        len: NonZeroU32::new((current_source_range.end - previous_source_range.start) as u32)
                            .expect("BUG: hunks are never zero-sized"),
                        commit_id: previous_entry.commit_id,
                        source_file_name: previous_entry.source_file_name.clone(),
                    };

                    acc.pop();
                    acc.push(coalesced_entry);
                } else {
                    acc.push(entry);
                }

                acc
            } else {
                acc.push(entry);

                acc
            }
        })
}

/// The union of [`gix_diff::tree::recorder::Change`] and [`gix_diff::tree_with_rewrites::Change`],
/// keeping only the blame-relevant information.
enum TreeDiffChange {
    Addition {
        id: ObjectId,
    },
    Deletion,
    Modification {
        previous_id: ObjectId,
        id: ObjectId,
    },
    Rewrite {
        source_location: BString,
        source_id: ObjectId,
        id: ObjectId,
    },
}

impl From<gix_diff::tree::recorder::Change> for TreeDiffChange {
    fn from(value: gix_diff::tree::recorder::Change) -> Self {
        use gix_diff::tree::recorder::Change;

        match value {
            Change::Addition { oid, .. } => Self::Addition { id: oid },
            Change::Deletion { .. } => Self::Deletion,
            Change::Modification { previous_oid, oid, .. } => Self::Modification {
                previous_id: previous_oid,
                id: oid,
            },
        }
    }
}

impl From<gix_diff::tree_with_rewrites::Change> for TreeDiffChange {
    fn from(value: gix_diff::tree_with_rewrites::Change) -> Self {
        use gix_diff::tree_with_rewrites::Change;

        match value {
            Change::Addition { id, .. } => Self::Addition { id },
            Change::Deletion { .. } => Self::Deletion,
            Change::Modification { previous_id, id, .. } => Self::Modification { previous_id, id },
            Change::Rewrite {
                source_location,
                source_id,
                id,
                ..
            } => Self::Rewrite {
                source_location,
                source_id,
                id,
            },
        }
    }
}

#[expect(clippy::too_many_arguments)]
fn tree_diff_at_file_path(
    odb: impl gix_object::Find + gix_object::FindHeader,
    file_path: &BStr,
    id: ObjectId,
    parent_id: ObjectId,
    graph: &mut Graph<'_, '_, CommitMetadata>,
    stats: &mut Statistics,
    state: &mut gix_diff::tree::State,
    resource_cache: &mut gix_diff::blob::Platform,
    lhs_tree_buf: &mut Vec<u8>,
    rhs_tree_buf: &mut Vec<u8>,
    rewrites: Option<gix_diff::Rewrites>,
) -> Result<Option<TreeDiffChange>> {
    let parent_tree_id = root_tree_id(graph, &parent_id)?;

    let parent_tree_iter = odb
        .find_tree_iter(&parent_tree_id, lhs_tree_buf)
        .or_raise(|| message("Could not find existing iterator over a tree"))?;
    stats.trees_decoded += 1;

    let tree_id = root_tree_id(graph, &id)?;

    let tree_iter = odb
        .find_tree_iter(&tree_id, rhs_tree_buf)
        .or_raise(|| message("Could not find existing iterator over a tree"))?;
    stats.trees_decoded += 1;

    let result = tree_diff_without_rewrites_at_file_path(&odb, file_path, stats, state, parent_tree_iter, tree_iter)?;

    // Here, we follow git’s behaviour. We return when we’ve found a `Modification`. We try a
    // second time with rename tracking when the change is either an `Addition` or a `Deletion`
    // because those can turn out to have been a `Rewrite`.
    // TODO(perf): renames are usually rare enough to not care about the work duplication done here.
    //             But in theory, a rename tracker could be used by us, on demand, and we could stuff the
    //             changes in there and have it find renames, without repeating the diff.
    if matches!(result, Some(TreeDiffChange::Modification { .. })) {
        return Ok(result);
    }
    let Some(rewrites) = rewrites else {
        return Ok(result);
    };

    let result = tree_diff_with_rewrites_at_file_path(
        &odb,
        file_path,
        stats,
        state,
        resource_cache,
        parent_tree_iter,
        tree_iter,
        rewrites,
    )?;

    Ok(result)
}

fn tree_diff_without_rewrites_at_file_path(
    odb: impl gix_object::Find + gix_object::FindHeader,
    file_path: &BStr,
    stats: &mut Statistics,
    state: &mut gix_diff::tree::State,
    parent_tree_iter: gix_object::TreeRefIter<'_>,
    tree_iter: gix_object::TreeRefIter<'_>,
) -> Result<Option<TreeDiffChange>> {
    struct FindChangeToPath {
        inner: gix_diff::tree::Recorder,
        interesting_path: BString,
        change: Option<gix_diff::tree::recorder::Change>,
    }

    impl FindChangeToPath {
        fn new(interesting_path: BString) -> Self {
            let inner =
                gix_diff::tree::Recorder::default().track_location(Some(gix_diff::tree::recorder::Location::Path));

            FindChangeToPath {
                inner,
                interesting_path,
                change: None,
            }
        }
    }

    impl Visit for FindChangeToPath {
        fn pop_front_tracked_path_and_set_current(&mut self) {
            self.inner.pop_front_tracked_path_and_set_current();
        }

        fn push_back_tracked_path_component(&mut self, component: &BStr) {
            self.inner.push_back_tracked_path_component(component);
        }

        fn push_path_component(&mut self, component: &BStr) {
            self.inner.push_path_component(component);
        }

        fn pop_path_component(&mut self) {
            self.inner.pop_path_component();
        }

        fn visit(&mut self, change: gix_diff::tree::visit::Change) -> gix_diff::tree::visit::Action {
            use gix_diff::tree::visit::Change::*;

            if self.inner.path() == self.interesting_path {
                self.change = Some(match change {
                    Deletion {
                        entry_mode,
                        oid,
                        relation,
                    } => gix_diff::tree::recorder::Change::Deletion {
                        entry_mode,
                        oid,
                        path: self.inner.path_clone(),
                        relation,
                    },
                    Addition {
                        entry_mode,
                        oid,
                        relation,
                    } => gix_diff::tree::recorder::Change::Addition {
                        entry_mode,
                        oid,
                        path: self.inner.path_clone(),
                        relation,
                    },
                    Modification {
                        previous_entry_mode,
                        previous_oid,
                        entry_mode,
                        oid,
                    } => gix_diff::tree::recorder::Change::Modification {
                        previous_entry_mode,
                        previous_oid,
                        entry_mode,
                        oid,
                        path: self.inner.path_clone(),
                    },
                });

                std::ops::ControlFlow::Break(())
            } else {
                std::ops::ControlFlow::Continue(())
            }
        }
    }

    let mut recorder = FindChangeToPath::new(file_path.into());
    let result = gix_diff::tree(parent_tree_iter, tree_iter, state, &odb, &mut recorder);
    stats.trees_diffed += 1;

    match result {
        Ok(_) | Err(gix_diff::tree::Error::Cancelled) => Ok(recorder.change.map(Into::into)),
        Err(error) => Err(error.raise()),
    }
}

#[expect(clippy::too_many_arguments)]
fn tree_diff_with_rewrites_at_file_path(
    odb: impl gix_object::Find + gix_object::FindHeader,
    file_path: &BStr,
    stats: &mut Statistics,
    state: &mut gix_diff::tree::State,
    resource_cache: &mut gix_diff::blob::Platform,
    parent_tree_iter: gix_object::TreeRefIter<'_>,
    tree_iter: gix_object::TreeRefIter<'_>,
    rewrites: gix_diff::Rewrites,
) -> Result<Option<TreeDiffChange>> {
    let mut change: Option<gix_diff::tree_with_rewrites::Change> = None;

    let options: gix_diff::tree_with_rewrites::Options = gix_diff::tree_with_rewrites::Options {
        location: Some(gix_diff::tree::recorder::Location::Path),
        rewrites: Some(rewrites),
    };
    let result = gix_diff::tree_with_rewrites(
        parent_tree_iter,
        tree_iter,
        resource_cache,
        state,
        &odb,
        |change_ref| {
            if change_ref.location() == file_path {
                change = Some(change_ref.into_owned());
                Ok(std::ops::ControlFlow::Break(()))
            } else {
                Ok(std::ops::ControlFlow::Continue(()))
            }
        },
        options,
    );
    stats.trees_diffed_with_rewrites += 1;

    match result {
        Ok(_) | Err(gix_diff::tree::Error::Cancelled) => Ok(change.map(Into::into)),
        Err(error) => Err(error.raise()),
    }
}

#[expect(clippy::too_many_arguments)]
fn blob_changes(
    odb: impl gix_object::Find + gix_object::FindHeader,
    resource_cache: &mut gix_diff::blob::Platform,
    oid: ObjectId,
    previous_oid: ObjectId,
    file_path: &BStr,
    previous_file_path: &BStr,
    diff_algorithm: gix_diff::blob::Algorithm,
    stats: &mut Statistics,
) -> Result<Vec<Change>> {
    use gix_diff::blob::platform::prepare_diff::{Operation, TextChange};

    resource_cache.set_resource(
        Some(previous_oid),
        // TODO(blame): add a test to show of symlink blaming works.
        gix_object::tree::EntryKind::Blob,
        previous_file_path,
        gix_diff::blob::ResourceKind::OldOrSource,
        &odb,
    )?;
    resource_cache.set_resource(
        Some(oid),
        gix_object::tree::EntryKind::Blob,
        file_path,
        gix_diff::blob::ResourceKind::NewOrDestination,
        &odb,
    )?;

    let outcome = resource_cache.prepare_diff()?;
    let pair = match outcome.operation {
        Operation::InternalDiff {
            change: TextChange::Modification(pair),
            ..
        }
        | Operation::ExternalCommand {
            change: TextChange::Modification(pair),
            ..
        } => pair,
        Operation::SourceOrDestinationIsBinary => {
            bail!(message!(
                "Cannot blame \"{file_path}\" against \"{previous_file_path}\": a diff resource is binary; force text diffing to blame its original bytes"
            ).validation());
        }
        Operation::InternalDiff { .. } | Operation::ExternalCommand { .. } | Operation::NeitherExists => {
            bail!(message!(
                "Cannot blame \"{file_path}\" against \"{previous_file_path}\": both diff resources must exist; check the configured worktree roots"
            ).validation());
        }
    };

    // shortcut: reject text conversion until blame ranges and output use converted content too.
    if pair.before().is_derived() || pair.after().is_derived() {
        bail!(message!(
            "Cannot blame \"{file_path}\" against \"{previous_file_path}\": a diff resource uses text conversion, which can change line numbers; disable text conversion"
        ).validation());
    }

    // Blame selects its own algorithm, even when a driver requests an external diff.
    Ok(blob_changes_from_data(
        pair.before().as_slice(),
        pair.after().as_slice(),
        diff_algorithm,
        stats,
    ))
}

fn blob_changes_from_data(
    old_data: &[u8],
    new_data: &[u8],
    diff_algorithm: gix_diff::blob::Algorithm,
    stats: &mut Statistics,
) -> Vec<Change> {
    use gix_diff::blob::{Algorithm, Hunk};

    let trimmed_bytes = if diff_algorithm == Algorithm::Histogram {
        common_tail_bytes_to_trim(old_data, new_data)
    } else {
        0
    };
    let (new_data, unchanged_tail) = new_data.split_at(new_data.len() - trimmed_bytes);
    let input = gix_diff::blob::InternedInput::new(&old_data[..old_data.len() - trimmed_bytes], new_data);

    let mut diff = gix_diff::blob::Diff::compute(diff_algorithm, &input);
    diff.postprocess_lines(&input);

    let mut last_seen_after_end = 0;
    let mut changes = diff.hunks().fold(Vec::new(), |mut hunks, hunk| {
        let Hunk { before, after } = hunk;

        // This checks for unchanged hunks.
        if after.start > last_seen_after_end {
            hunks.push(Change::Unchanged(last_seen_after_end..after.start));
        }

        match (!before.is_empty(), !after.is_empty()) {
            (_, true) => {
                hunks.push(Change::AddedOrReplaced(
                    after.start..after.end,
                    before.end - before.start,
                ));
            }
            (true, false) => {
                hunks.push(Change::Deleted(after.start, before.end - before.start));
            }
            (false, false) => unreachable!("BUG: imara-diff provided a non-change"),
        }

        last_seen_after_end = after.end;

        hunks
    });

    let total_number_of_lines = input.after.len() + tokens_for_diffing(unchanged_tail).tokenize().count();
    if total_number_of_lines > last_seen_after_end as usize {
        changes.push(Change::Unchanged(last_seen_after_end..total_number_of_lines as u32));
    }

    stats.blobs_diffed += 1;
    changes
}

/// Match Git's zero-context `xdi_diff()`, which trims whole common 1 KiB blocks from the end
/// and keeps bytes through the first newline of those blocks to leave a complete line.
fn common_tail_bytes_to_trim(before: &[u8], after: &[u8]) -> usize {
    const BLOCK_SIZE: usize = 1024;
    let common_len = before
        .rchunks_exact(BLOCK_SIZE)
        .zip(after.rchunks_exact(BLOCK_SIZE))
        .take_while(|(before, after)| before == after)
        .count()
        * BLOCK_SIZE;
    let common_tail = &after[after.len() - common_len..];
    common_tail
        .find_byte(b'\n')
        .map_or(0, |newline| common_len - newline - 1)
}

fn find_path_entry_in_commit(
    odb: &impl gix_object::Find,
    commit: &gix_hash::oid,
    file_path: &BStr,
    graph: &mut Graph<'_, '_, CommitMetadata>,
    buf: &mut Vec<u8>,
    buf2: &mut Vec<u8>,
    stats: &mut Statistics,
) -> Result<Option<ObjectId>> {
    let tree_id = root_tree_id(graph, commit)?;
    let tree_iter = odb
        .find_tree_iter(&tree_id, buf)
        .or_raise(|| message("Could not find existing iterator over a tree"))?;
    stats.trees_decoded += 1;

    let res = tree_iter
        .lookup_entry(
            odb,
            buf2,
            file_path.split(|b| *b == b'/').inspect(|_| stats.trees_decoded += 1),
        )
        .or_raise(|| message("Couldn't find commit or tree in the object database"))?;
    stats.trees_decoded -= 1;
    Ok(res.map(|e| e.oid))
}

type ParentIds = SmallVec<[(gix_hash::ObjectId, i64); 2]>;

fn root_tree_id(graph: &mut Graph<'_, '_, CommitMetadata>, commit_id: &gix_hash::oid) -> Result<ObjectId> {
    commit_field(
        graph,
        commit_id,
        |metadata| &mut metadata.root_tree_id,
        |commit| {
            commit
                .tree_id()
                .or_raise(|| message!("Could not decode the root tree of commit {commit_id}"))
        },
    )
}

#[allow(
    clippy::redundant_closure_for_method_calls,
    reason = "the method item cannot satisfy the callback's higher-ranked lifetimes"
)]
fn commit_time(graph: &mut Graph<'_, '_, CommitMetadata>, commit_id: &gix_hash::oid) -> Result<i64> {
    commit_field(
        graph,
        commit_id,
        |metadata| &mut metadata.commit_time,
        |commit| commit.committer_timestamp(),
    )
}

fn collect_parents(graph: &mut Graph<'_, '_, CommitMetadata>, commit_id: &gix_hash::oid) -> Result<ParentIds> {
    let parent_commit_ids = commit_field(
        graph,
        commit_id,
        |metadata| &mut metadata.parents,
        |commit| commit.iter_parents().collect::<Result<_>>(),
    )?;
    Ok(parent_commit_ids
        .into_iter()
        // A parent that cannot be read gets the lowest priority, but its fallback time is not cached.
        .map(|parent_id| (parent_id, commit_time(graph, &parent_id).unwrap_or_default()))
        .collect())
}

fn commit_field<T: Clone>(
    graph: &mut Graph<'_, '_, CommitMetadata>,
    commit_id: &gix_hash::oid,
    field: fn(&mut CommitMetadata) -> &mut Option<T>,
    decode: impl FnOnce(&LazyCommit<'_, '_>) -> Result<T>,
) -> Result<T> {
    if let Some(value) = graph.get_mut(commit_id).and_then(|metadata| field(metadata).clone()) {
        return Ok(value);
    }
    let value = decode(&graph.lookup(commit_id)?)?;
    match graph.get_mut(commit_id) {
        Some(metadata) => *field(metadata) = Some(value.clone()),
        None => {
            let mut metadata = CommitMetadata::default();
            *field(&mut metadata) = Some(value.clone());
            graph.insert(commit_id.to_owned(), metadata);
        }
    }
    Ok(value)
}

/// Return an iterator over tokens for use in diffing. These are usually lines, but it's important
/// to unify them so the later access shows the right thing.
pub(crate) fn tokens_for_diffing(data: &[u8]) -> impl TokenSource<Token = &[u8]> {
    gix_diff::blob::sources::byte_lines(data)
}

/// The blame input after resolving [`Start`] and before traversing commit history.
struct InitialState {
    /// The final file contents whose lines are being attributed.
    blamed_file_blob: Vec<u8>,
    /// Requested ranges that still need attribution through commit traversal.
    hunks_to_blame: Vec<UnblamedHunk>,
    /// Entries already resolved while comparing worktree contents to the first suspect.
    out: Vec<BlameEntry>,
    /// The first commit to traverse, or `None` if no history traversal is needed.
    first_suspect: Option<ObjectId>,
}

#[expect(clippy::too_many_arguments)]
fn initial_state(
    odb: &impl gix_object::Find,
    start: Start<'_>,
    graph: &mut Graph<'_, '_, CommitMetadata>,
    file_path: &BStr,
    options: &Options,
    buf: &mut Vec<u8>,
    buf2: &mut Vec<u8>,
    stats: &mut Statistics,
) -> Result<InitialState> {
    match start {
        Start::Commit(suspect) => {
            let blamed_file_entry_id = find_path_entry_in_commit(&odb, &suspect, file_path, graph, buf, buf2, stats)?
                .ok_or_raise(|| {
                message!("The file to blame at \"{file_path}\" wasn't found in the first commit at {suspect}")
                    .not_found()
            })?;
            let blamed_file_blob = odb
                .find_blob(&blamed_file_entry_id, buf)
                .or_raise(|| message("Could not find existing blob or commit"))?
                .data
                .to_vec();
            let num_lines_in_blamed = tokens_for_diffing(&blamed_file_blob).tokenize().count() as u32;

            // Binary or otherwise empty?
            if num_lines_in_blamed == 0 {
                return Ok(InitialState {
                    blamed_file_blob,
                    hunks_to_blame: Vec::new(),
                    out: Vec::new(),
                    first_suspect: None,
                });
            }

            let ranges_to_blame = options.ranges.to_zero_based_exclusive_ranges(num_lines_in_blamed);
            let hunks_to_blame = ranges_to_blame
                .into_iter()
                .map(|range| UnblamedHunk::new(range, suspect))
                .collect::<Vec<_>>();

            Ok(InitialState {
                blamed_file_blob,
                hunks_to_blame,
                out: Vec::new(),
                first_suspect: Some(suspect),
            })
        }
        Start::Contents {
            first_suspect,
            contents,
        } => {
            let null_id = first_suspect.kind().null();
            let blamed_file_blob = contents.into_owned();
            let num_lines_in_blamed = tokens_for_diffing(&blamed_file_blob).tokenize().count() as u32;

            if num_lines_in_blamed == 0 {
                return Ok(InitialState {
                    blamed_file_blob,
                    hunks_to_blame: Vec::new(),
                    out: Vec::new(),
                    first_suspect: None,
                });
            }

            let ranges_to_blame = options.ranges.to_zero_based_exclusive_ranges(num_lines_in_blamed);
            let mut hunks_to_blame = ranges_to_blame
                .into_iter()
                .map(|range| UnblamedHunk::new(range, null_id))
                .collect::<Vec<_>>();

            let mut out = Vec::new();

            let Some(first_suspect_entry_id) =
                find_path_entry_in_commit(odb, &first_suspect, file_path, graph, buf, buf2, stats)?
            else {
                // File does not exist in the starting commit. Treat the requested
                // ranges as entirely "not committed yet".
                out.extend(
                    hunks_to_blame
                        .iter()
                        .filter_map(|hunk| BlameEntry::from_unblamed_hunk(hunk, null_id)),
                );

                return Ok(InitialState {
                    blamed_file_blob,
                    hunks_to_blame: Vec::new(),
                    out,
                    first_suspect: None,
                });
            };

            let first_suspect_blob = odb
                .find_blob(&first_suspect_entry_id, buf)
                .or_raise(|| message("Could not find existing blob or commit"))?
                .data
                .to_vec();

            let changes = blob_changes_from_data(&first_suspect_blob, &blamed_file_blob, options.diff_algorithm, stats);
            hunks_to_blame = process_changes(hunks_to_blame, changes, null_id, first_suspect);

            unblamed_to_out_is_done(&mut hunks_to_blame, &mut out, null_id);

            let first_suspect = hunks_to_blame
                .iter()
                .any(|hunk| hunk.has_suspect(&first_suspect))
                .then_some(first_suspect);

            Ok(InitialState {
                blamed_file_blob,
                hunks_to_blame,
                out,
                first_suspect,
            })
        }
    }
}
