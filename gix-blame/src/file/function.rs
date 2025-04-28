use std::{num::NonZeroU32, ops::Range};

use gix_diff::{blob::intern::TokenSource, tree::Visit};
use gix_hash::ObjectId;
use gix_object::{
    bstr::{BStr, BString},
    FindExt,
};
use gix_traverse::commit::find as find_commit;
use smallvec::SmallVec;

use super::{process_changes, Change, UnblamedHunk};
use crate::file::checkpoint::update_checkpoint_blames_with_changes;
use crate::{BlameCheckpoint, BlameEntry, Error, Options, Outcome, Statistics};

/// Produce a list of consecutive [`BlameEntry`] instances to indicate in which commits the ranges of the file
/// at `suspect:<file_path>` originated in.
///
/// ## Parameters
///
/// * `odb`
///    - Access to database objects, also for used for diffing.
///    - Should have an object cache for good diff performance.
/// * `suspect`
///    - The first commit to be responsible for parts of `file_path`.
/// * `cache`
///    - Optionally, the commitgraph cache.
/// * `file_path`
///    - A *slash-separated* worktree-relative path to the file to blame.
/// * `options`
///    - Options to control the blame operation.
/// * `resource_cache`
///    - Used for diffing trees.
///
/// ## The algorithm
///
/// *For brevity, `HEAD` denotes the starting point of the blame operation. It could be any commit, or even commits that
/// represent the worktree state.
/// We begin with a single *Unblamed Hunk* and a single suspect, usually the `HEAD` commit as the commit containing the
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
///   - for each parent, do a diff and mark lines that don't have a suspect yet (this is the term
///     used in `libgit2`), but that have been changed in this commit
///
/// The algorithm in `libgit2` works by going through parents and keeping a linked list of blame
/// suspects. It can be visualized as follows:
//
// <---------------------------------------->
// <---------------><----------------------->
// <---><----------><----------------------->
// <---><----------><-------><-----><------->
// <---><---><-----><-------><-----><------->
// <---><---><-----><-------><-----><-><-><->
pub fn file(
    odb: impl gix_object::Find + gix_object::FindHeader,
    suspect: ObjectId,
    cache: Option<gix_commitgraph::Graph>,
    resource_cache: &mut gix_diff::blob::Platform,
    file_path: &BStr,
    options: Options,
) -> Result<Outcome, Error> {
    let _span = gix_trace::coarse!("gix_blame::file()", ?file_path, ?suspect);

    let mut processor = BlameProcessor::new(&odb, cache, resource_cache, file_path, options)?;
    processor.set_blame_state(suspect)?;
    processor.process_blame()
}

/// Process blame information starting from a checkpoint.
///
/// Uses the same core blame algorithm as `file()` but continues from a previously provided checkpoint,
/// allowing for incremental blame operations by reusing previously computed information.
///
/// # Parameters
///
/// * `odb` - Access to database objects, used for diffing and object lookup
/// * `suspect` - The commit to start blame processing from
/// * `cache` - Optional commitgraph cache for performance
/// * `blame_checkpoint` - Previous blame information to start from
/// * `resource_cache` - Cache used for diffing trees
/// * `file_path` - Path to the file being blamed (slash-separated, worktree-relative)
/// * `options` - Options controlling the blame operation
///
/// # Returns
///
/// Returns an `Outcome` containing the blame entries and statistics, or an error if the operation fails.
pub fn file_checkpoint(
    odb: impl gix_object::Find + gix_object::FindHeader,
    suspect: ObjectId,
    cache: Option<gix_commitgraph::Graph>,
    blame_checkpoint: BlameCheckpoint,
    resource_cache: &mut gix_diff::blob::Platform,
    file_path: &BStr,
    options: Options,
) -> Result<Outcome, Error> {
    let _span = gix_trace::coarse!("gix_blame::file_checkpoint()", ?file_path, ?suspect);

    let mut processor = BlameProcessor::new(&odb, cache, resource_cache, file_path, options)?;
    processor.set_blame_state_from_checkpoint(blame_checkpoint, suspect)?;
    processor.process_blame()
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
                    // let combined_range =
                    let coalesced_entry = BlameEntry {
                        start_in_blamed_file: previous_blamed_range.start as u32,
                        start_in_source_file: previous_source_range.start as u32,
                        len: NonZeroU32::new((current_source_range.end - previous_source_range.start) as u32)
                            .expect("BUG: hunks are never zero-sized"),
                        commit_id: previous_entry.commit_id,
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

#[allow(clippy::too_many_arguments)]
fn tree_diff_at_file_path(
    odb: impl gix_object::Find + gix_object::FindHeader,
    file_path: &BStr,
    id: ObjectId,
    parent_id: ObjectId,
    cache: Option<&gix_commitgraph::Graph>,
    stats: &mut Statistics,
    state: &mut gix_diff::tree::State,
    commit_buf: &mut Vec<u8>,
    lhs_tree_buf: &mut Vec<u8>,
    rhs_tree_buf: &mut Vec<u8>,
) -> Result<Option<gix_diff::tree::recorder::Change>, Error> {
    let parent_tree_id = find_commit(cache, &odb, &parent_id, commit_buf)?.tree_id()?;

    let parent_tree_iter = odb.find_tree_iter(&parent_tree_id, lhs_tree_buf)?;
    stats.trees_decoded += 1;

    let tree_id = find_commit(cache, &odb, &id, commit_buf)?.tree_id()?;

    let tree_iter = odb.find_tree_iter(&tree_id, rhs_tree_buf)?;
    stats.trees_decoded += 1;

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
            use gix_diff::tree::{visit, visit::Change::*};

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

                visit::Action::Cancel
            } else {
                visit::Action::Continue
            }
        }
    }

    let mut recorder = FindChangeToPath::new(file_path.into());
    let result = gix_diff::tree(parent_tree_iter, tree_iter, state, &odb, &mut recorder);
    stats.trees_diffed += 1;

    match result {
        Ok(_) | Err(gix_diff::tree::Error::Cancelled) => Ok(recorder.change),
        Err(error) => Err(Error::DiffTree(error)),
    }
}

fn blob_changes(
    odb: impl gix_object::Find + gix_object::FindHeader,
    resource_cache: &mut gix_diff::blob::Platform,
    oid: ObjectId,
    previous_oid: ObjectId,
    file_path: &BStr,
    diff_algorithm: gix_diff::blob::Algorithm,
    stats: &mut Statistics,
) -> Result<Vec<Change>, Error> {
    /// Record all [`Change`]s to learn about additions, deletions and unchanged portions of a *Source File*.
    struct ChangeRecorder {
        last_seen_after_end: u32,
        hunks: Vec<Change>,
        total_number_of_lines: u32,
    }

    impl ChangeRecorder {
        /// `total_number_of_lines` is used to fill in the last unchanged hunk if needed
        /// so that the entire file is represented by [`Change`].
        fn new(total_number_of_lines: u32) -> Self {
            ChangeRecorder {
                last_seen_after_end: 0,
                hunks: Vec::new(),
                total_number_of_lines,
            }
        }
    }

    impl gix_diff::blob::Sink for ChangeRecorder {
        type Out = Vec<Change>;

        fn process_change(&mut self, before: Range<u32>, after: Range<u32>) {
            // This checks for unchanged hunks.
            if after.start > self.last_seen_after_end {
                self.hunks
                    .push(Change::Unchanged(self.last_seen_after_end..after.start));
            }

            match (!before.is_empty(), !after.is_empty()) {
                (_, true) => {
                    self.hunks.push(Change::AddedOrReplaced(
                        after.start..after.end,
                        before.end - before.start,
                    ));
                }
                (true, false) => {
                    self.hunks.push(Change::Deleted(after.start, before.end - before.start));
                }
                (false, false) => unreachable!("BUG: imara-diff provided a non-change"),
            }
            self.last_seen_after_end = after.end;
        }

        fn finish(mut self) -> Self::Out {
            if self.total_number_of_lines > self.last_seen_after_end {
                self.hunks
                    .push(Change::Unchanged(self.last_seen_after_end..self.total_number_of_lines));
            }
            self.hunks
        }
    }

    resource_cache.set_resource(
        previous_oid,
        gix_object::tree::EntryKind::Blob,
        file_path,
        gix_diff::blob::ResourceKind::OldOrSource,
        &odb,
    )?;
    resource_cache.set_resource(
        oid,
        gix_object::tree::EntryKind::Blob,
        file_path,
        gix_diff::blob::ResourceKind::NewOrDestination,
        &odb,
    )?;

    let outcome = resource_cache.prepare_diff()?;
    let input = gix_diff::blob::intern::InternedInput::new(
        tokens_for_diffing(outcome.old.data.as_slice().unwrap_or_default()),
        tokens_for_diffing(outcome.new.data.as_slice().unwrap_or_default()),
    );
    let number_of_lines_in_destination = input.after.len();
    let change_recorder = ChangeRecorder::new(number_of_lines_in_destination as u32);

    let res = gix_diff::blob::diff(diff_algorithm, &input, change_recorder);
    stats.blobs_diffed += 1;
    Ok(res)
}

fn find_path_entry_in_commit(
    odb: &impl gix_object::Find,
    commit: &gix_hash::oid,
    file_path: &BStr,
    cache: Option<&gix_commitgraph::Graph>,
    buf: &mut Vec<u8>,
    buf2: &mut Vec<u8>,
    stats: &mut Statistics,
) -> Result<Option<ObjectId>, Error> {
    let tree_id = find_commit(cache, odb, commit, buf)?.tree_id()?;
    let tree_iter = odb.find_tree_iter(&tree_id, buf)?;
    stats.trees_decoded += 1;

    let res = tree_iter.lookup_entry(
        odb,
        buf2,
        file_path.split(|b| *b == b'/').inspect(|_| stats.trees_decoded += 1),
    )?;
    stats.trees_decoded -= 1;
    Ok(res.map(|e| e.oid))
}

type CommitTime = i64;

fn commit_time(commit: gix_traverse::commit::Either<'_, '_>) -> Result<CommitTime, gix_object::decode::Error> {
    match commit {
        gix_traverse::commit::Either::CommitRefIter(commit_ref_iter) => {
            commit_ref_iter.committer().map(|c| c.seconds())
        }
        gix_traverse::commit::Either::CachedCommit(commit) => Ok(commit.committer_timestamp() as i64),
    }
}

type ParentIds = SmallVec<[(ObjectId, i64); 2]>;

fn collect_parents(
    commit: gix_traverse::commit::Either<'_, '_>,
    odb: &impl gix_object::Find,
    cache: Option<&gix_commitgraph::Graph>,
    buf: &mut Vec<u8>,
) -> Result<ParentIds, Error> {
    let mut parent_ids: ParentIds = Default::default();
    match commit {
        gix_traverse::commit::Either::CachedCommit(commit) => {
            let cache = cache
                .as_ref()
                .expect("find returned a cached commit, so we expect cache to be present");
            for parent_pos in commit.iter_parents() {
                let parent = cache.commit_at(parent_pos?);
                parent_ids.push((parent.id().to_owned(), parent.committer_timestamp() as i64));
            }
        }
        gix_traverse::commit::Either::CommitRefIter(commit_ref_iter) => {
            for id in commit_ref_iter.parent_ids() {
                let parent = odb.find_commit_iter(id.as_ref(), buf).ok();
                let parent_commit_time = parent
                    .and_then(|parent| parent.committer().ok().map(|committer| committer.seconds()))
                    .unwrap_or_default();
                parent_ids.push((id, parent_commit_time));
            }
        }
    }
    Ok(parent_ids)
}

/// Return an iterator over tokens for use in diffing. These are usually lines, but it's important
/// to unify them so the later access shows the right thing.
pub(crate) fn tokens_for_diffing(data: &[u8]) -> impl TokenSource<Token = &[u8]> {
    gix_diff::blob::sources::byte_lines_with_terminator(data)
}

/// State maintained during the blame operation
///
/// Tracks both the current processing state and final results, including unblamed hunks,
/// completed entries, file content, and statistics. Can be created either from scratch
/// or from a checkpoint.
struct BlameState {
    /// The current state of unblamed hunks that still need processing
    hunks_to_blame: Vec<UnblamedHunk>,
    /// The final blame entries that have been processed
    out: Vec<BlameEntry>,
    /// The content of the file being blamed
    blamed_file_blob: Vec<u8>,
    /// Statistics gathered during the blame operation
    stats: Statistics,
    /// Priority queue for processing commits in chronological order
    queue: gix_revwalk::PriorityQueue<CommitTime, ObjectId>,
    /// Cache for the previous entry to avoid redundant lookups
    previous_entry: Option<(ObjectId, ObjectId)>,
}

/// Processor for handling blame operations
///
/// Encapsulates the logic for processing blame operations, whether starting
/// from scratch or continuing from a checkpoint.
struct BlameProcessor<'a, T: gix_object::Find + gix_object::FindHeader> {
    odb: &'a T,
    commit_graph_cache: Option<gix_commitgraph::Graph>,
    resource_cache: &'a mut gix_diff::blob::Platform,
    file_path: &'a BStr,
    options: Options,
    blame_state: Option<BlameState>,
}

impl<'a, T: gix_object::Find + gix_object::FindHeader> BlameProcessor<'a, T> {
    /// Creates a new blame processor with the given parameters
    fn new(
        odb: &'a T,
        commit_graph_cache: Option<gix_commitgraph::Graph>,
        resource_cache: &'a mut gix_diff::blob::Platform,
        file_path: &'a BStr,
        options: Options,
    ) -> Result<Self, Error> {
        Ok(Self {
            odb,
            commit_graph_cache,
            resource_cache,
            file_path,
            blame_state: None,
            options,
        })
    }

    /// Sets up the initial blame state for processing from scratch
    fn set_blame_state(&mut self, suspect: ObjectId) -> Result<(), Error> {
        let blame_state = self.create_blame_state(suspect)?;
        self.blame_state = Some(blame_state);
        Ok(())
    }

    /// Sets up the initial blame state using information from a checkpoint
    fn set_blame_state_from_checkpoint(
        &mut self,
        blame_checkpoint: BlameCheckpoint,
        suspect: ObjectId,
    ) -> Result<(), Error> {
        let blame_state = self.create_blame_state_from_checkpoint(blame_checkpoint, suspect)?;
        self.blame_state = Some(blame_state);
        Ok(())
    }

    /// Helper function to find a file entry in a commit and return its blob data
    fn find_file_entry_and_blob(
        &self,
        commit_id: &ObjectId,
        stats: &mut Statistics,
        buf: &mut Vec<u8>,
        buf2: &mut Vec<u8>,
    ) -> Result<(ObjectId, Vec<u8>), Error> {
        let entry_id = find_path_entry_in_commit(
            self.odb,
            commit_id,
            self.file_path,
            self.commit_graph_cache.as_ref(),
            buf,
            buf2,
            stats,
        )?
        .ok_or_else(|| Error::FileMissing {
            file_path: self.file_path.to_owned(),
            commit_id: *commit_id,
        })?;

        let blob_data = self.odb.find_blob(&entry_id, buf)?.data.to_vec();
        Ok((entry_id, blob_data))
    }

    /// Creates the initial blame state from a checkpoint
    ///
    /// Sets up blame state by:
    /// 1. Getting file contents from checkpoint and suspect commits
    /// 2. Computing and applying changes
    /// 3. Setting up remaining hunks for processing
    fn create_blame_state_from_checkpoint(
        &mut self,
        blame_checkpoint: BlameCheckpoint,
        suspect: ObjectId,
    ) -> Result<BlameState, Error> {
        let mut stats = Statistics::default();
        let (mut buf, mut buf2) = (Vec::new(), Vec::new());

        let (checkpoint_file_entry_id, _) =
            self.find_file_entry_and_blob(&blame_checkpoint.checkpoint_commit_id, &mut stats, &mut buf, &mut buf2)?;

        let (blamed_file_entry_id, blamed_file_blob) =
            self.find_file_entry_and_blob(&suspect, &mut stats, &mut buf, &mut buf2)?;

        let changes_with_checkpoint = blob_changes(
            self.odb,
            self.resource_cache,
            checkpoint_file_entry_id,
            blamed_file_entry_id,
            self.file_path,
            self.options.diff_algorithm,
            &mut stats,
        )?;

        let mut blame_state = BlameState::empty(blamed_file_blob, suspect, stats);

        // If there are no changes, we update the blame_state with the checkpoint entries.
        // Because the range stays 0..0 we are returning early during processing
        if changes_with_checkpoint
            .iter()
            .all(|change| matches!(change, Change::Unchanged(_)))
        {
            blame_state.set_out(blame_checkpoint.entries);
            return Ok(blame_state);
        }

        let (blame_entries, hunks_to_blame) =
            update_checkpoint_blames_with_changes(blame_checkpoint, changes_with_checkpoint, suspect);

        blame_state.set_out(blame_entries);
        if !hunks_to_blame.is_empty() {
            blame_state.set_hunks(hunks_to_blame);
        }
        Ok(blame_state)
    }

    /// Set a new BlameState for the given suspect commit
    fn create_blame_state(&self, suspect: ObjectId) -> Result<BlameState, Error> {
        let mut stats = Statistics::default();
        let (mut buf, mut buf2) = (Vec::new(), Vec::new());

        let (_, blamed_file_blob) = self.find_file_entry_and_blob(&suspect, &mut stats, &mut buf, &mut buf2)?;

        let num_lines_in_blamed = tokens_for_diffing(&blamed_file_blob).tokenize().count() as u32;

        // Binary or otherwise empty?
        if num_lines_in_blamed == 0 {
            return Ok(BlameState::empty(blamed_file_blob, suspect, stats));
        }

        let ranges_in_blamed_file = self
            .options
            .range
            .to_zero_based_exclusive(num_lines_in_blamed)
            .unwrap_or_default();

        let commit = find_commit(self.commit_graph_cache.as_ref(), self.odb, &suspect, &mut buf)?;
        let initial_commit_time = commit_time(commit)?;

        Ok(BlameState::new(
            blamed_file_blob,
            ranges_in_blamed_file,
            suspect,
            initial_commit_time,
            Some(stats),
        ))
    }

    fn process_blame(self) -> Result<Outcome, Error> {
        let mut state = match self.blame_state {
            Some(state) => state,
            None => {
                return Err(Error::BlameStateNotSet);
            }
        };

        let mut diff_state = gix_diff::tree::State::default();
        let (mut buf, mut buf2, mut buf3) = (Vec::new(), Vec::new(), Vec::new());

        'outer: while let Some(suspect) = state.queue.pop_value() {
            state.stats.commits_traversed += 1;
            if !state.has_unblamed_hunks() {
                break;
            }

            let is_still_suspect = state.hunks_to_blame.iter().any(|hunk| hunk.has_suspect(&suspect));
            if !is_still_suspect {
                // There are no `UnblamedHunk`s associated with this `suspect`, so we can continue with
                // the next one.
                continue 'outer;
            }

            let commit = find_commit(self.commit_graph_cache.as_ref(), self.odb, &suspect, &mut buf)?;
            let commit_time = commit_time(commit)?;

            if let Some(since) = self.options.since {
                if commit_time < since.seconds {
                    if state.has_unblamed_hunks() && state.unblamed_to_out_is_done(suspect) {
                        break 'outer;
                    }
                    continue;
                }
            }

            let parent_ids: ParentIds = collect_parents(commit, self.odb, self.commit_graph_cache.as_ref(), &mut buf2)?;

            if parent_ids.is_empty() {
                if state.queue.is_empty() {
                    // TODO I'm not entirely sure if this is correct yet. `suspect`, at this point, is the
                    // `id` of the last `item` that was yielded by `queue`, so it makes sense to assign
                    // the remaining lines to it, even though we don't explicitly check whether that is
                    // true here. We could perhaps use diff-tree-to-tree to compare `suspect` against
                    // an empty tree to validate this assumption.
                    if state.unblamed_to_out_is_done(suspect) {
                        break 'outer;
                    }
                }
                // There is more, keep looking.
                continue;
            }

            let mut entry = state
                .previous_entry
                .take()
                .filter(|(id, _)| *id == suspect)
                .map(|(_, entry)| entry);
            if entry.is_none() {
                entry = find_path_entry_in_commit(
                    self.odb,
                    &suspect,
                    self.file_path,
                    self.commit_graph_cache.as_ref(),
                    &mut buf,
                    &mut buf2,
                    &mut state.stats,
                )?;
            }

            let Some(entry_id) = entry else {
                continue;
            };

            #[cfg(debug_assertions)]
            state.assert_hunk_and_blame_ranges_identical(self.odb, entry_id, &mut buf)?;

            if state.check_for_unchanged_parents(
                self.odb,
                self.commit_graph_cache.as_ref(),
                self.file_path,
                entry_id,
                suspect,
                &parent_ids,
                &mut buf,
                &mut buf2,
            )? {
                continue 'outer;
            }

            let more_than_one_parent = parent_ids.len() > 1;
            for (parent_id, parent_commit_time) in parent_ids {
                state.queue.insert(parent_commit_time, parent_id);
                let changes_for_file_path = tree_diff_at_file_path(
                    self.odb,
                    self.file_path,
                    suspect,
                    parent_id,
                    self.commit_graph_cache.as_ref(),
                    &mut state.stats,
                    &mut diff_state,
                    &mut buf,
                    &mut buf2,
                    &mut buf3,
                )?;

                let Some(modification) = changes_for_file_path else {
                    if more_than_one_parent {
                        // None of the changes affected the file we're currently blaming.
                        // Copy blame to parent
                        state.clone_blame(suspect, parent_id);
                    } else {
                        state.pass_blame(suspect, parent_id);
                    }
                    continue;
                };

                match modification {
                    gix_diff::tree::recorder::Change::Addition { .. } => {
                        if more_than_one_parent {
                            // Do nothing under the assumption that this always (or almost always)
                            // implies that the file comes from a different parent, compared to which
                            // it was modified, not added.
                        } else if state.unblamed_to_out_is_done(suspect) {
                            break 'outer;
                        }
                    }
                    gix_diff::tree::recorder::Change::Deletion { .. } => {
                        unreachable!("We already found file_path in suspect^{{tree}}, so it can't be deleted")
                    }
                    gix_diff::tree::recorder::Change::Modification { previous_oid, oid, .. } => {
                        let changes = blob_changes(
                            self.odb,
                            self.resource_cache,
                            oid,
                            previous_oid,
                            self.file_path,
                            self.options.diff_algorithm,
                            &mut state.stats,
                        )?;
                        state.hunks_to_blame = process_changes(state.hunks_to_blame, changes, suspect, parent_id);
                    }
                }
            }

            state.process_completed_hunks(suspect);
        }

        #[cfg(debug_assertions)]
        state.assert_no_overlapping_ranges();

        Ok(state.finalize())
    }
}

impl BlameState {
    /// Create a new BlameState with initial values
    fn new(
        blamed_file_blob: Vec<u8>,
        initial_ranges: Vec<Range<u32>>,
        initial_suspect: ObjectId,
        initial_commit_time: CommitTime,
        statistics: Option<Statistics>,
    ) -> Self {
        let mut queue = gix_revwalk::PriorityQueue::new();
        queue.insert(initial_commit_time, initial_suspect);

        let mut hunks_to_blame = Vec::with_capacity(initial_ranges.len());

        for range in initial_ranges {
            hunks_to_blame.push(UnblamedHunk {
                range_in_blamed_file: range.clone(),
                suspects: [(initial_suspect, range)].into(),
            });
        }

        Self {
            hunks_to_blame,
            out: Vec::new(),
            blamed_file_blob,
            stats: statistics.unwrap_or_default(),
            queue,
            previous_entry: None,
        }
    }

    /// Create a BlameState with an empty range, used when there are no lines to blame
    /// or when starting from a checkpoint
    fn empty(blamed_file_blob: Vec<u8>, suspect: ObjectId, stats: Statistics) -> Self {
        let empty_range = 0..0;
        Self::new(blamed_file_blob, vec![empty_range], suspect, 0, Some(stats))
    }

    /// Check if there are any remaining hunks to process
    fn has_unblamed_hunks(&self) -> bool {
        !self.hunks_to_blame.is_empty()
    }

    /// Convert each of the unblamed hunk in `hunks_to_blame` into a [`BlameEntry`], consuming them in the process.
    ///
    /// Return `true` if we are done because `hunks_to_blame` is empty.
    fn unblamed_to_out_is_done(&mut self, suspect: ObjectId) -> bool {
        let mut without_suspect = Vec::new();
        self.out.extend(self.hunks_to_blame.drain(..).filter_map(|hunk| {
            BlameEntry::from_unblamed_hunk(&hunk, suspect).or_else(|| {
                without_suspect.push(hunk);
                None
            })
        }));
        self.hunks_to_blame = without_suspect;
        self.hunks_to_blame.is_empty()
    }

    fn set_out(&mut self, out: Vec<BlameEntry>) {
        self.out = out;
    }

    fn set_hunks(&mut self, unblamed_hunks: Vec<UnblamedHunk>) {
        self.hunks_to_blame = unblamed_hunks;
    }

    /// Process completed hunks for a suspect, moving them to the output
    fn process_completed_hunks(&mut self, suspect: ObjectId) {
        self.hunks_to_blame.retain_mut(|unblamed_hunk| {
            if unblamed_hunk.suspects.len() == 1 {
                if let Some(entry) = BlameEntry::from_unblamed_hunk(unblamed_hunk, suspect) {
                    // At this point, we have copied blame for every hunk to a parent. Hunks
                    // that have only `suspect` left in `suspects` have not passed blame to any
                    // parent, and so they can be converted to a `BlameEntry` and moved to
                    // `out`.
                    self.out.push(entry);
                    return false;
                }
            }
            unblamed_hunk.remove_blame(suspect);
            true
        });
    }

    /// Pass blame from one commit to another for all hunks
    fn pass_blame(&mut self, from: ObjectId, to: ObjectId) {
        for unblamed_hunk in &mut self.hunks_to_blame {
            unblamed_hunk.pass_blame(from, to);
        }
    }

    /// Clone blame from one commit to another for all hunks
    fn clone_blame(&mut self, from: ObjectId, to: ObjectId) {
        for unblamed_hunk in &mut self.hunks_to_blame {
            unblamed_hunk.clone_blame(from, to);
        }
    }

    /// Finalize the blame state and return the outcome
    fn finalize(mut self) -> Outcome {
        debug_assert_eq!(
            self.hunks_to_blame,
            vec![],
            "only if there is no portion of the file left we have completed the blame"
        );

        // TODO I don't know yet whether it would make sense to use a data structure instead that preserves
        // order on insertion.
        self.out
            .sort_by(|a, b| a.start_in_blamed_file.cmp(&b.start_in_blamed_file));
        Outcome {
            entries: coalesce_blame_entries(self.out),
            blob: self.blamed_file_blob,
            statistics: self.stats,
        }
    }

    /// Check if any parent has the exact same file content and if so, pass blame to that parent.
    /// This is an optimization to avoid unnecessary diffing.
    /// Returns true if an unchanged parent was found and handled.
    #[allow(clippy::too_many_arguments)]
    fn check_for_unchanged_parents(
        &mut self,
        odb: &impl gix_object::Find,
        commit_graph_cache: Option<&gix_commitgraph::Graph>,
        file_path: &BStr,
        entry_id: ObjectId,
        suspect: ObjectId,
        parent_ids: &[(ObjectId, i64)],
        buf: &mut Vec<u8>,
        buf2: &mut Vec<u8>,
    ) -> Result<bool, Error> {
        for (pid, (parent_id, parent_commit_time)) in parent_ids.iter().enumerate() {
            if let Some(parent_entry_id) = find_path_entry_in_commit(
                odb,
                parent_id,
                file_path,
                commit_graph_cache,
                buf,
                buf2,
                &mut self.stats,
            )? {
                let no_change_in_entry = entry_id == parent_entry_id;
                if pid == 0 {
                    self.previous_entry = Some((*parent_id, parent_entry_id));
                }
                if no_change_in_entry {
                    self.pass_blame(suspect, *parent_id);
                    self.queue.insert(*parent_commit_time, *parent_id);
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Assert that line ranges for each suspect never overlap
    /// This block asserts that line ranges for each suspect never overlap. If they did overlap
    //  this would mean that the same line in a *Source File* would map to more than one line in
    //  the *Blamed File* and this is not possible.
    #[cfg(debug_assertions)]
    fn assert_no_overlapping_ranges(&self) {
        let ranges = self.hunks_to_blame.iter().fold(
            std::collections::BTreeMap::<ObjectId, Vec<Range<u32>>>::new(),
            |mut acc, hunk| {
                for (suspect, range) in hunk.suspects.clone() {
                    acc.entry(suspect).or_default().push(range);
                }
                acc
            },
        );

        for (_, mut ranges) in ranges {
            ranges.sort_by(|a, b| a.start.cmp(&b.start));
            for window in ranges.windows(2) {
                if let [a, b] = window {
                    assert!(a.end <= b.start, "#{:?}", self.hunks_to_blame);
                }
            }
        }
    }

    /// Assert that for every UnblamedHunk, all lines in the Blamed File are identical
    /// to the corresponding lines in the Source File.
    #[cfg(debug_assertions)]
    fn assert_hunk_and_blame_ranges_identical(
        &self,
        odb: &impl gix_object::Find,
        entry_id: ObjectId,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        let source_blob = odb.find_blob(&entry_id, buf)?.data.to_vec();
        let mut source_interner = gix_diff::blob::intern::Interner::new(source_blob.len() / 100);
        let source_lines_as_tokens: Vec<_> = tokens_for_diffing(&source_blob)
            .tokenize()
            .map(|token| source_interner.intern(token))
            .collect();

        let mut blamed_interner = gix_diff::blob::intern::Interner::new(self.blamed_file_blob.len() / 100);
        let blamed_lines_as_tokens: Vec<_> = tokens_for_diffing(&self.blamed_file_blob)
            .tokenize()
            .map(|token| blamed_interner.intern(token))
            .collect();

        for hunk in self.hunks_to_blame.iter() {
            if let Some(range_in_suspect) = hunk.get_range(&entry_id) {
                let range_in_blamed_file = hunk.range_in_blamed_file.clone();

                for (blamed_line_number, source_line_number) in range_in_blamed_file.zip(range_in_suspect.clone()) {
                    let source_token = source_lines_as_tokens[source_line_number as usize];
                    let blame_token = blamed_lines_as_tokens[blamed_line_number as usize];

                    let source_line = BString::new(source_interner[source_token].into());
                    let blamed_line = BString::new(blamed_interner[blame_token].into());

                    assert_eq!(source_line, blamed_line);
                }
            }
        }
        Ok(())
    }
}
