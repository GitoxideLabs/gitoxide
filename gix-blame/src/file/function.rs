use super::{process_changes, Change, UnblamedHunk};
use crate::{BlameEntry, Error, Outcome, Statistics};
use gix_diff::blob::intern::TokenSource;
use gix_diff::tree::Visit;
use gix_hash::ObjectId;
use gix_object::{
    bstr::{BStr, BString},
    FindExt,
};
use std::num::NonZeroU32;
use std::ops::Range;

/// Produce a list of consecutive [`BlameEntry`] instances to indicate in which commits the ranges of the file
/// at `traverse[0]:<file_path>` originated in.
///
/// ## Paramters
///
/// * `odb`
///    - Access to database objects, also for used for diffing.
///    - Should have an object cache for good diff performance.
/// * `traverse`
///    - The list of commits from the most recent to prior ones, following all parents sorted
///      by time.
///    - It's paramount that older commits are returned after newer ones.
///    - The first commit returned here is the first eligible commit to be responsible for parts of `file_path`.
/// * `file_path`
///    - A *slash-separated* worktree-relative path to the file to blame.
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
///   - for each parent, do a diff and mark lines that don’t have a suspect yet (this is the term
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
pub fn file<E>(
    odb: impl gix_object::Find + gix_object::FindHeader,
    traverse: impl IntoIterator<Item = Result<gix_traverse::commit::Info, E>>,
    resource_cache: &mut gix_diff::blob::Platform,
    file_path: &BStr,
) -> Result<Outcome, Error>
where
    E: Into<Box<dyn std::error::Error + Send + Sync + 'static>>,
{
    let mut traverse = traverse.into_iter().peekable();
    let Some(Ok(suspect)) = traverse.peek().map(|res| res.as_ref().map(|item| item.id)) else {
        return Err(Error::EmptyTraversal);
    };
    let _span = gix_trace::coarse!("gix_blame::file()", ?file_path, ?suspect);

    let mut stats = Statistics::default();
    let (mut buf, mut buf2, mut buf3) = (Vec::new(), Vec::new(), Vec::new());
    let blamed_file_entry_id = find_path_entry_in_commit(&odb, &suspect, file_path, &mut buf, &mut buf2, &mut stats)?
        .ok_or_else(|| Error::FileMissing {
        file_path: file_path.to_owned(),
        commit_id: suspect,
    })?;
    let blamed_file_blob = odb.find_blob(&blamed_file_entry_id, &mut buf)?.data.to_vec();
    let num_lines_in_blamed = {
        let mut interner = gix_diff::blob::intern::Interner::new(blamed_file_blob.len() / 100);
        tokens_for_diffing(&blamed_file_blob)
            .tokenize()
            .map(|token| interner.intern(token))
            .count()
    };

    // Binary or otherwise empty?
    if num_lines_in_blamed == 0 {
        return Ok(Outcome::default());
    }

    let mut hunks_to_blame = vec![{
        let range_in_blamed_file = 0..num_lines_in_blamed as u32;
        UnblamedHunk {
            range_in_blamed_file: range_in_blamed_file.clone(),
            suspects: [(suspect, range_in_blamed_file)].into(),
        }
    }];

    let mut out = Vec::new();
    let mut diff_state = gix_diff::tree::State::default();
    let mut previous_entry: Option<(ObjectId, ObjectId)> = None;
    'outer: while let Some(item) = traverse.next() {
        if hunks_to_blame.is_empty() {
            break;
        }
        let commit = item.map_err(|err| Error::Traverse(err.into()))?;
        let suspect = commit.id;
        stats.commits_traversed += 1;

        let parent_ids = commit.parent_ids;
        if parent_ids.is_empty() {
            if traverse.peek().is_none() {
                // I’m not entirely sure if this is correct yet. `suspect`, at this point, is the `id` of
                // the last `item` that was yielded by `traverse`, so it makes sense to assign the
                // remaining lines to it, even though we don’t explicitly check whether that is true
                // here. We could perhaps use diff-tree-to-tree to compare `suspect`
                // against an empty tree to validate this assumption.
                if unblamed_to_out_is_done(&mut hunks_to_blame, &mut out, suspect) {
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
            entry = find_path_entry_in_commit(&odb, &suspect, file_path, &mut buf, &mut buf2, &mut stats)?;
        }

        let Some(entry_id) = entry else {
            continue;
        };

        for (pid, parent_id) in parent_ids.iter().enumerate() {
            if let Some(parent_entry_id) =
                find_path_entry_in_commit(&odb, parent_id, file_path, &mut buf, &mut buf2, &mut stats)?
            {
                let no_change_in_entry = entry_id == parent_entry_id;
                if pid == 0 {
                    previous_entry = Some((*parent_id, parent_entry_id));
                }
                if no_change_in_entry {
                    pass_blame_from_to(suspect, *parent_id, &mut hunks_to_blame);
                    continue 'outer;
                }
            }
        }

        let more_than_one_parent = parent_ids.len() > 1;
        for parent_id in parent_ids {
            let changes_for_file_path = tree_diff_at_file_path(
                &odb,
                file_path,
                commit.id,
                parent_id,
                &mut stats,
                &mut diff_state,
                &mut buf,
                &mut buf2,
                &mut buf3,
            )?;
            let Some(modification) = changes_for_file_path else {
                if more_than_one_parent {
                    // None of the changes affected the file we’re currently blaming.
                    // Copy blame to parent.
                    hunks_to_blame = hunks_to_blame
                        .into_iter()
                        .map(|unblamed_hunk| unblamed_hunk.clone_blame(suspect, parent_id))
                        .collect();
                } else {
                    pass_blame_from_to(suspect, parent_id, &mut hunks_to_blame);
                }
                continue;
            };

            match modification {
                gix_diff::tree::recorder::Change::Addition { .. } => {
                    if more_than_one_parent {
                        // Do nothing under the assumption that this always (or almost always)
                        // implies that the file comes from a different parent, compared to which
                        // it was modified, not added.
                    } else if unblamed_to_out_is_done(&mut hunks_to_blame, &mut out, suspect) {
                        break 'outer;
                    }
                }
                gix_diff::tree::recorder::Change::Deletion { .. } => {
                    unreachable!("We already found file_path in suspect^{{tree}}, so it can't be deleted")
                }
                gix_diff::tree::recorder::Change::Modification { previous_oid, oid, .. } => {
                    let changes = blob_changes(&odb, resource_cache, oid, previous_oid, file_path, &mut stats)?;
                    hunks_to_blame = process_changes(hunks_to_blame, changes, suspect, parent_id);
                }
            }
        }

        hunks_to_blame = hunks_to_blame
            .into_iter()
            .filter_map(|mut unblamed_hunk| {
                if unblamed_hunk.suspects.len() == 1 {
                    if let Some(entry) = BlameEntry::from_unblamed_hunk(&unblamed_hunk, suspect) {
                        // At this point, we have copied blame for every hunk to a parent. Hunks
                        // that have only `suspect` left in `suspects` have not passed blame to any
                        // parent and so they can be converted to a `BlameEntry` and moved to
                        // `out`.
                        out.push(entry);

                        return None;
                    }
                }

                unblamed_hunk.remove_blame(suspect);

                Some(unblamed_hunk)
            })
            .collect();

        // This block asserts that line ranges for each suspect never overlap. If they did overlap
        // this would mean that the same line in a *Source File* would map to more than one line in
        // the *Blamed File* and this is not possible.
        #[cfg(debug_assertions)]
        {
            let ranges = hunks_to_blame.iter().fold(
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
                        assert!(a.end <= b.start, "#{hunks_to_blame:#?}");
                    }
                }
            }
        }
    }

    debug_assert_eq!(
        hunks_to_blame,
        vec![],
        "only if there is no portion of the file left we have completed the blame"
    );

    // I don’t know yet whether it would make sense to use a data structure instead that preserves
    // order on insertion.
    out.sort_by(|a, b| a.start_in_blamed_file.cmp(&b.start_in_blamed_file));
    Ok(Outcome {
        entries: coalesce_blame_entries(out),
        blob: blamed_file_blob,
        statistics: stats,
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
    stats: &mut Statistics,
    state: &mut gix_diff::tree::State,
    commit_buf: &mut Vec<u8>,
    lhs_tree_buf: &mut Vec<u8>,
    rhs_tree_buf: &mut Vec<u8>,
) -> Result<Option<gix_diff::tree::recorder::Change>, Error> {
    let parent_tree = odb.find_commit(&parent_id, commit_buf)?.tree();
    stats.commits_to_tree += 1;

    let parent_tree_iter = odb.find_tree_iter(&parent_tree, lhs_tree_buf)?;
    stats.trees_decoded += 1;

    let tree_id = odb.find_commit(&id, commit_buf)?.tree();
    stats.commits_to_tree += 1;

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
            use gix_diff::tree::visit::Action::*;
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

                // When we return `Cancel`, `gix_diff::tree` will convert this `Cancel` into an
                // `Err(...)`. Keep this in mind when using `FindChangeToPath`.
                Cancel
            } else {
                Continue
            }
        }
    }

    let mut recorder = FindChangeToPath::new(file_path.into());
    let result = gix_diff::tree(parent_tree_iter, tree_iter, state, &odb, &mut recorder);
    stats.trees_diffed += 1;

    match result {
        // `recorder` cancels the traversal by returning `Cancel` when a change to `file_path` is
        // found. `gix_diff::tree` converts `Cancel` into `Err(Cancelled)` which is why we match on
        // `Err(Cancelled)` in addition to `Ok`.
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

    let res = gix_diff::blob::diff(gix_diff::blob::Algorithm::Histogram, &input, change_recorder);
    stats.blobs_diffed += 1;
    Ok(res)
}

fn find_path_entry_in_commit(
    odb: &impl gix_object::Find,
    commit: &gix_hash::oid,
    file_path: &BStr,
    buf: &mut Vec<u8>,
    buf2: &mut Vec<u8>,
    stats: &mut Statistics,
) -> Result<Option<ObjectId>, Error> {
    let commit_id = odb.find_commit(commit, buf)?.tree();
    stats.commits_to_tree += 1;
    let tree_iter = odb.find_tree_iter(&commit_id, buf)?;
    stats.trees_decoded += 1;

    let res = tree_iter.lookup_entry(
        odb,
        buf2,
        file_path.split(|b| *b == b'/').inspect(|_| stats.trees_decoded += 1),
    )?;
    stats.trees_decoded -= 1;
    Ok(res.map(|e| e.oid))
}

/// Return an iterator over tokens for use in diffing. These usually lines, but iit's important to unify them
/// so the later access shows the right thing.
pub(crate) fn tokens_for_diffing(data: &[u8]) -> impl TokenSource<Token = &[u8]> {
    gix_diff::blob::sources::byte_lines_with_terminator(data)
}
