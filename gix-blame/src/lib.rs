//! A crate to implement an algorithm to annotate lines in tracked files with the commits that changed them.
#![deny(rust_2018_idioms)]
#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    ops::{Add, AddAssign, Range, SubAssign},
    path::PathBuf,
};

use gix_hash::ObjectId;
use gix_object::bstr::BStr;
use gix_object::FindExt;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Offset {
    Added(u32),
    Deleted(u32),
}

impl Add<u32> for Offset {
    type Output = Offset;

    fn add(self, rhs: u32) -> Self::Output {
        let Self::Added(added) = self else { todo!() };

        Self::Added(added + rhs)
    }
}

impl Add<Offset> for Offset {
    type Output = Offset;

    fn add(self, rhs: Offset) -> Self::Output {
        match (self, rhs) {
            (Self::Added(added), Offset::Added(added_rhs)) => Self::Added(added + added_rhs),
            (Self::Added(added), Offset::Deleted(deleted_rhs)) => {
                if deleted_rhs > added {
                    Self::Deleted(deleted_rhs - added)
                } else {
                    Self::Added(added - deleted_rhs)
                }
            }
            (Self::Deleted(deleted), Offset::Added(added_rhs)) => {
                if added_rhs > deleted {
                    Self::Added(added_rhs - deleted)
                } else {
                    Self::Deleted(deleted - added_rhs)
                }
            }
            (Self::Deleted(deleted), Offset::Deleted(deleted_rhs)) => Self::Deleted(deleted + deleted_rhs),
        }
    }
}

impl AddAssign<u32> for Offset {
    fn add_assign(&mut self, rhs: u32) {
        match self {
            Self::Added(added) => *self = Self::Added(*added + rhs),
            Self::Deleted(deleted) => {
                if rhs > *deleted {
                    *self = Self::Added(rhs - *deleted);
                } else {
                    *self = Self::Deleted(*deleted - rhs);
                }
            }
        }
    }
}

impl SubAssign<u32> for Offset {
    fn sub_assign(&mut self, rhs: u32) {
        match self {
            Self::Added(added) => {
                if rhs > *added {
                    *self = Self::Deleted(rhs - *added);
                } else {
                    *self = Self::Added(*added - rhs);
                }
            }
            Self::Deleted(deleted) => *self = Self::Deleted(*deleted + rhs),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct BlameEntry {
    pub range_in_blamed_file: Range<u32>,
    pub range_in_original_file: Range<u32>,
    pub commit_id: ObjectId,
}

impl BlameEntry {
    pub fn new(range_in_blamed_file: Range<u32>, range_in_original_file: Range<u32>, commit_id: ObjectId) -> Self {
        assert!(
            range_in_blamed_file.end > range_in_blamed_file.start,
            "{range_in_blamed_file:?}"
        );
        assert!(
            range_in_original_file.end > range_in_original_file.start,
            "{range_in_original_file:?}"
        );

        Self {
            range_in_blamed_file: range_in_blamed_file.clone(),
            range_in_original_file: range_in_original_file.clone(),
            commit_id,
        }
    }

    fn with_offset(range_in_original_file: Range<u32>, commit_id: ObjectId, offset: Offset) -> Self {
        assert!(
            range_in_original_file.end > range_in_original_file.start,
            "{range_in_original_file:?}"
        );

        match offset {
            Offset::Added(added) => Self {
                range_in_blamed_file: (range_in_original_file.start + added)..(range_in_original_file.end + added),
                range_in_original_file,
                commit_id,
            },
            Offset::Deleted(deleted) => {
                assert!(
                    range_in_original_file.start >= deleted,
                    "{range_in_original_file:?} {offset:?}"
                );

                Self {
                    range_in_blamed_file: (range_in_original_file.start - deleted)
                        ..(range_in_original_file.end - deleted),
                    range_in_original_file,
                    commit_id,
                }
            }
        }
    }

    fn from_unblamed_hunk(unblamed_hunk: &UnblamedHunk, commit_id: ObjectId) -> Self {
        let range_in_original_file = unblamed_hunk.suspects.get(&commit_id).expect("TODO");

        Self {
            range_in_blamed_file: unblamed_hunk.range_in_blamed_file.clone(),
            range_in_original_file: range_in_original_file.clone(),
            commit_id,
        }
    }
}

trait LineRange {
    fn shift_by(&self, offset: Offset) -> Self;
}

impl LineRange for Range<u32> {
    fn shift_by(&self, offset: Offset) -> Self {
        match offset {
            Offset::Added(added) => {
                assert!(self.start >= added, "{self:?} {offset:?}");

                Self {
                    start: self.start - added,
                    end: self.end - added,
                }
            }
            Offset::Deleted(deleted) => Self {
                start: self.start + deleted,
                end: self.end + deleted,
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct UnblamedHunk {
    pub range_in_blamed_file: Range<u32>,
    pub suspects: BTreeMap<ObjectId, Range<u32>>,
}

#[derive(Debug)]
enum Either<T, U> {
    Left(T),
    Right(U),
}

impl UnblamedHunk {
    pub fn new(range_in_blamed_file: Range<u32>, suspect: ObjectId, offset: Offset) -> Self {
        assert!(
            range_in_blamed_file.end > range_in_blamed_file.start,
            "{range_in_blamed_file:?}"
        );

        let range_in_destination = range_in_blamed_file.shift_by(offset);

        Self {
            range_in_blamed_file,
            suspects: [(suspect, range_in_destination)].into(),
        }
    }

    fn shift_by(mut self, suspect: ObjectId, offset: Offset) -> Self {
        self.suspects.entry(suspect).and_modify(|e| *e = e.shift_by(offset));

        self
    }

    fn split_at(self, suspect: ObjectId, line_number_in_destination: u32) -> Either<Self, (Self, Self)> {
        match self.suspects.get(&suspect) {
            None => Either::Left(self),
            Some(range_in_suspect) => {
                if line_number_in_destination > range_in_suspect.start
                    && line_number_in_destination < range_in_suspect.end
                {
                    let split_at_from_start = line_number_in_destination - range_in_suspect.start;

                    if split_at_from_start > 0 {
                        let new_suspects_before = self
                            .suspects
                            .iter()
                            .map(|(suspect, range)| (*suspect, range.start..(range.start + split_at_from_start)))
                            .collect();

                        let new_suspects_after = self
                            .suspects
                            .iter()
                            .map(|(suspect, range)| (*suspect, (range.start + split_at_from_start)..range.end))
                            .collect();

                        let new_hunk_before = Self {
                            range_in_blamed_file: self.range_in_blamed_file.start
                                ..(self.range_in_blamed_file.start + split_at_from_start),
                            suspects: new_suspects_before,
                        };
                        let new_hunk_after = Self {
                            range_in_blamed_file: (self.range_in_blamed_file.start + split_at_from_start)
                                ..(self.range_in_blamed_file.end),
                            suspects: new_suspects_after,
                        };

                        Either::Right((new_hunk_before, new_hunk_after))
                    } else {
                        Either::Left(self)
                    }
                } else {
                    Either::Left(self)
                }
            }
        }
    }

    fn offset_for(&self, suspect: ObjectId) -> Offset {
        let range_in_suspect = self.suspects.get(&suspect).expect("TODO");

        if self.range_in_blamed_file.start > range_in_suspect.start {
            Offset::Added(self.range_in_blamed_file.start - range_in_suspect.start)
        } else {
            Offset::Deleted(range_in_suspect.start - self.range_in_blamed_file.start)
        }
    }

    fn pass_blame(&mut self, from: ObjectId, to: ObjectId) {
        if let Some(range_in_suspect) = self.suspects.remove(&from) {
            self.suspects.insert(to, range_in_suspect);
        }
    }

    fn clone_blame(&mut self, from: ObjectId, to: ObjectId) {
        if let Some(range_in_suspect) = self.suspects.get(&from) {
            self.suspects.insert(to, range_in_suspect.clone());
        }
    }

    fn remove_blame(&mut self, suspect: ObjectId) {
        let _ = self.suspects.remove(&suspect);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Unchanged(Range<u32>),
    Added(Range<u32>, u32),
    Deleted(u32, u32),
}

struct ChangeRecorder {
    previous_after_end: u32,
    changes: Vec<Change>,
    total_number_of_lines: u32,
}

impl ChangeRecorder {
    fn new(total_number_of_lines: u32) -> Self {
        ChangeRecorder {
            previous_after_end: 0,
            changes: vec![],
            total_number_of_lines,
        }
    }
}

impl gix_diff::blob::Sink for ChangeRecorder {
    type Out = Vec<Change>;

    // “imara-diff will compute a line diff by default”, so each `start` and `end` represents a
    // line in a file.
    fn process_change(&mut self, before: Range<u32>, after: Range<u32>) {
        // This checks for unchanged hunks.
        //
        // https://docs.rs/imara-diff/latest/imara_diff/sink/trait.Sink.html#notes
        if after.start > self.previous_after_end {
            self.changes
                .push(Change::Unchanged(self.previous_after_end..after.start));
        }

        match (before.end > before.start, after.end > after.start) {
            (_, true) => {
                self.changes
                    .push(Change::Added(after.start..after.end, before.end - before.start));
            }
            (true, false) => {
                self.changes
                    .push(Change::Deleted(after.start, before.end - before.start));
            }
            (false, false) => unimplemented!(),
        }

        self.previous_after_end = after.end;
    }

    fn finish(mut self) -> Self::Out {
        if self.total_number_of_lines > self.previous_after_end {
            self.changes
                .push(Change::Unchanged(self.previous_after_end..self.total_number_of_lines));
        }

        self.changes
    }
}

pub fn process_change(
    out: &mut Vec<BlameEntry>,
    new_hunks_to_blame: &mut Vec<UnblamedHunk>,
    offset_in_destination: &mut Offset,
    suspect: ObjectId,
    hunk: Option<UnblamedHunk>,
    change: Option<Change>,
) -> (Option<UnblamedHunk>, Option<Change>) {
    match (hunk, change) {
        (Some(hunk), Some(Change::Unchanged(unchanged))) => {
            let Some(range_in_suspect) = hunk.suspects.get(&suspect) else {
                new_hunks_to_blame.push(hunk);

                return (None, Some(Change::Unchanged(unchanged)));
            };

            match (
                // Since `unchanged` is a range that is not inclusive at the end,
                // `unchanged.end` is not part of `unchanged`. The first line that is
                // `unchanged.end - 1`.
                range_in_suspect.contains(&unchanged.start),
                (unchanged.end - 1) >= range_in_suspect.start && unchanged.end <= range_in_suspect.end,
            ) {
                (_, true) => {
                    //     <------>  (hunk)
                    // <------->     (unchanged)
                    //
                    // <---------->  (hunk)
                    //     <--->     (unchanged)

                    (Some(hunk), None)
                }
                (true, false) => {
                    // <-------->     (hunk)
                    //     <------->  (unchanged)

                    new_hunks_to_blame.push(hunk.shift_by(suspect, *offset_in_destination));

                    (None, Some(Change::Unchanged(unchanged)))
                }
                (false, false) => {
                    // Any of the following cases are handled by this branch:
                    //    <--->      (hunk)
                    // <---------->  (unchanged)
                    //
                    //       <---->  (hunk)
                    // <-->          (unchanged)
                    //
                    // <-->          (hunk)
                    //       <---->  (unchanged)

                    if unchanged.end <= range_in_suspect.start {
                        //       <---->  (hunk)
                        // <-->          (unchanged)

                        (Some(hunk.clone()), None)
                    } else {
                        // <-->          (hunk)
                        //       <---->  (unchanged)
                        //
                        //    <--->      (hunk)
                        // <---------->  (unchanged)

                        new_hunks_to_blame.push(hunk.shift_by(suspect, *offset_in_destination));

                        (None, Some(Change::Unchanged(unchanged.clone())))
                    }
                }
            }
        }
        (Some(hunk), Some(Change::Added(added, number_of_lines_deleted))) => {
            let Some(range_in_suspect) = hunk.suspects.get(&suspect) else {
                new_hunks_to_blame.push(hunk);

                return (None, Some(Change::Added(added, number_of_lines_deleted)));
            };

            let range_in_suspect = range_in_suspect.clone();

            match (
                range_in_suspect.contains(&added.start),
                // Since `added` is a range that is not inclusive at the end, `added.end` is
                // not part of `added`. The first line that is is `added.end - 1`.
                (added.end - 1) >= range_in_suspect.start && added.end <= range_in_suspect.end,
            ) {
                (true, true) => {
                    // <---------->  (hunk)
                    //     <--->     (added)
                    //     <--->     (blamed)
                    // <-->     <->  (new hunk)

                    let new_hunk = match hunk.split_at(suspect, added.start) {
                        Either::Left(hunk) => hunk,
                        Either::Right((before, after)) => {
                            new_hunks_to_blame.push(before.shift_by(suspect, *offset_in_destination));

                            after
                        }
                    };

                    *offset_in_destination += added.end - added.start;
                    *offset_in_destination -= number_of_lines_deleted;

                    out.push(BlameEntry::with_offset(
                        added.clone(),
                        suspect,
                        new_hunk.offset_for(suspect),
                    ));

                    match new_hunk.split_at(suspect, added.end) {
                        Either::Left(_) => (None, None),
                        Either::Right((_, after)) => (Some(after), None),
                    }
                }
                (true, false) => {
                    // <-------->     (hunk)
                    //     <------->  (added)
                    //     <---->     (blamed)
                    // <-->           (new hunk)

                    let new_hunk = match hunk.split_at(suspect, added.start) {
                        Either::Left(hunk) => hunk,
                        Either::Right((before, after)) => {
                            new_hunks_to_blame.push(before.shift_by(suspect, *offset_in_destination));

                            after
                        }
                    };

                    out.push(BlameEntry::with_offset(
                        added.start..range_in_suspect.end,
                        suspect,
                        new_hunk.offset_for(suspect),
                    ));

                    if added.end > range_in_suspect.end {
                        (None, Some(Change::Added(added, number_of_lines_deleted)))
                    } else {
                        todo!();
                    }
                }
                (false, true) => {
                    //    <------->  (hunk)
                    // <------>      (added)
                    //    <--->      (blamed)
                    //         <-->  (new hunk)

                    out.push(BlameEntry::with_offset(
                        range_in_suspect.start..added.end,
                        suspect,
                        hunk.offset_for(suspect),
                    ));

                    *offset_in_destination += added.end - added.start;
                    *offset_in_destination -= number_of_lines_deleted;

                    match hunk.split_at(suspect, added.end) {
                        Either::Left(_) => (None, None),
                        Either::Right((_, after)) => (Some(after), None),
                    }
                }
                (false, false) => {
                    // Any of the following cases are handled by this branch:
                    //    <--->      (hunk)
                    // <---------->  (added)
                    //
                    //       <---->  (hunk)
                    // <-->          (added)
                    //
                    // <-->          (hunk)
                    //       <---->  (added)

                    if added.end <= range_in_suspect.start {
                        //       <---->  (hunk)
                        // <-->          (added)

                        *offset_in_destination += added.end - added.start;
                        *offset_in_destination -= number_of_lines_deleted;

                        (Some(hunk.clone()), None)
                    } else if range_in_suspect.end <= added.start {
                        // <-->          (hunk)
                        //       <---->  (added)

                        new_hunks_to_blame.push(hunk.shift_by(suspect, *offset_in_destination));

                        (None, Some(Change::Added(added.clone(), number_of_lines_deleted)))
                    } else {
                        //    <--->      (hunk)
                        // <---------->  (added)
                        //    <--->      (blamed)

                        out.push(BlameEntry::with_offset(
                            range_in_suspect.clone(),
                            suspect,
                            hunk.offset_for(suspect),
                        ));

                        (None, Some(Change::Added(added.clone(), number_of_lines_deleted)))
                    }
                }
            }
        }
        (Some(hunk), Some(Change::Deleted(line_number_in_destination, number_of_lines_deleted))) => {
            let range_in_suspect = hunk.suspects.get(&suspect).expect("TODO");

            if line_number_in_destination < range_in_suspect.start {
                //     <--->  (hunk)
                //  |         (line_number_in_destination)

                *offset_in_destination -= number_of_lines_deleted;

                (Some(hunk), None)
            } else if line_number_in_destination < range_in_suspect.end {
                //  <----->  (hunk)
                //     |     (line_number_in_destination)

                let new_hunk = match hunk.split_at(suspect, line_number_in_destination) {
                    Either::Left(hunk) => hunk,
                    Either::Right((before, after)) => {
                        new_hunks_to_blame.push(before.shift_by(suspect, *offset_in_destination));

                        after
                    }
                };

                *offset_in_destination -= number_of_lines_deleted;

                (Some(new_hunk), None)
            } else {
                //  <--->     (hunk)
                //         |  (line_number_in_destination)

                new_hunks_to_blame.push(hunk.shift_by(suspect, *offset_in_destination));

                (
                    None,
                    Some(Change::Deleted(line_number_in_destination, number_of_lines_deleted)),
                )
            }
        }
        (Some(hunk), None) => {
            new_hunks_to_blame.push(hunk.shift_by(suspect, *offset_in_destination));

            (None, None)
        }
        (None, Some(Change::Unchanged(_))) => (None, None),
        (None, Some(Change::Added(added, number_of_lines_deleted))) => {
            *offset_in_destination += added.end - added.start;
            *offset_in_destination -= number_of_lines_deleted;

            (None, None)
        }
        (None, Some(Change::Deleted(_, number_of_lines_deleted))) => {
            *offset_in_destination -= number_of_lines_deleted;

            (None, None)
        }
        (None, None) => (None, None),
    }
}

pub fn process_changes(
    out: &mut Vec<BlameEntry>,
    hunks_to_blame: &[UnblamedHunk],
    changes: &[Change],
    suspect: ObjectId,
) -> Vec<UnblamedHunk> {
    let mut hunks_iter = hunks_to_blame.iter().cloned();
    let mut changes_iter = changes.iter().cloned();

    let mut hunk: Option<UnblamedHunk> = hunks_iter.next();
    let mut change: Option<Change> = changes_iter.next();

    let mut new_hunks_to_blame: Vec<UnblamedHunk> = vec![];
    let mut offset_in_destination: Offset = Offset::Added(0);

    loop {
        (hunk, change) = process_change(
            out,
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            hunk,
            change,
        );

        hunk = hunk.or_else(|| hunks_iter.next());
        change = change.or_else(|| changes_iter.next());

        if hunk.is_none() && change.is_none() {
            break;
        }
    }

    new_hunks_to_blame
}

fn get_changes_for_file_path(
    odb: impl gix_object::Find + gix_object::FindHeader,
    file_path: &BStr,
    id: ObjectId,
    parent_id: ObjectId,
) -> Vec<gix_diff::tree::recorder::Change> {
    let mut buffer = Vec::new();

    let parent = odb.find_commit(&parent_id, &mut buffer).unwrap();

    let mut buffer = Vec::new();
    let parent_tree_iter = odb
        .find(&parent.tree(), &mut buffer)
        .unwrap()
        .try_into_tree_iter()
        .unwrap();

    let mut buffer = Vec::new();
    let commit = odb.find_commit(&id, &mut buffer).unwrap();

    let mut buffer = Vec::new();
    let tree_iter = odb
        .find(&commit.tree(), &mut buffer)
        .unwrap()
        .try_into_tree_iter()
        .unwrap();

    let mut recorder = gix_diff::tree::Recorder::default();
    gix_diff::tree(
        parent_tree_iter,
        tree_iter,
        gix_diff::tree::State::default(),
        &odb,
        &mut recorder,
    )
    .unwrap();

    recorder
        .records
        .iter()
        .filter(|change| match change {
            gix_diff::tree::recorder::Change::Modification { path, .. } => path == file_path,
            gix_diff::tree::recorder::Change::Addition { path, .. } => path == file_path,
            gix_diff::tree::recorder::Change::Deletion { path, .. } => path == file_path,
        })
        .cloned()
        .collect()
}

fn get_changes(
    odb: impl gix_object::Find + gix_object::FindHeader,
    resource_cache: &mut gix_diff::blob::Platform,
    oid: ObjectId,
    previous_oid: ObjectId,
    file_path: &BStr,
) -> Vec<Change> {
    resource_cache
        .set_resource(
            previous_oid,
            gix_object::tree::EntryKind::Blob,
            file_path,
            gix_diff::blob::ResourceKind::OldOrSource,
            &odb,
        )
        .unwrap();
    resource_cache
        .set_resource(
            oid,
            gix_object::tree::EntryKind::Blob,
            file_path,
            gix_diff::blob::ResourceKind::NewOrDestination,
            &odb,
        )
        .unwrap();

    let outcome = resource_cache.prepare_diff().unwrap();
    let input = outcome.interned_input();
    let number_of_lines_in_destination = input.after.len();
    let change_recorder = ChangeRecorder::new(number_of_lines_in_destination.try_into().unwrap());

    gix_diff::blob::diff(gix_diff::blob::Algorithm::Histogram, &input, change_recorder)
}

/// This function merges adjacent blame entries. It merges entries that are adjacent both in the
/// blamed file and in the original file that introduced them. This follows `git`’s
/// behaviour. `libgit2`, as of 2024-09-19, only checks whether two entries are adjacent in the
/// blamed file which can result in different blames in certain edge cases. See [the commit][1]
/// that introduced the extra check into `git` for context. See [this commit][2] for a way to test
/// for this behaviour in `git`.
///
/// [1]: https://github.com/git/git/commit/c2ebaa27d63bfb7c50cbbdaba90aee4efdd45d0a
/// [2]: https://github.com/git/git/commit/6dbf0c7bebd1c71c44d786ebac0f2b3f226a0131
fn coalesce_blame_entries(lines_blamed: Vec<BlameEntry>) -> Vec<BlameEntry> {
    // TODO
    // It’s possible this could better be done on insertion into `lines_blamed`.
    lines_blamed.into_iter().fold(vec![], |mut acc, entry| {
        let previous_entry = acc.last();

        if let Some(previous_entry) = previous_entry {
            if previous_entry.commit_id == entry.commit_id
                && previous_entry.range_in_blamed_file.end == entry.range_in_blamed_file.start
                // As of 2024-09-19, the check below only is in `git`, but not in `libgit2`.
                && previous_entry.range_in_original_file.end == entry.range_in_original_file.start
            {
                let coalesced_entry = BlameEntry {
                    range_in_blamed_file: previous_entry.range_in_blamed_file.start..entry.range_in_blamed_file.end,
                    range_in_original_file: previous_entry.range_in_original_file.start
                        ..entry.range_in_original_file.end,
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

// TODO: do not instantiate anything, get everything passed as argument.
pub fn blame_file<E>(
    odb: impl gix_object::Find + gix_object::FindHeader,
    traverse: impl IntoIterator<Item = Result<gix_traverse::commit::Info, E>>,
    resource_cache: &mut gix_diff::blob::Platform,
    suspect: ObjectId,
    worktree_path: PathBuf,
    file_path: &BStr,
) -> Result<Vec<BlameEntry>, E> {
    // TODO
    // At a high level, what we want to do is the following:
    //
    // - get the commit that belongs to a commit id
    // - walk through parents
    //   - for each parent, do a diff and mark lines that don’t have a suspect (this is the term
    //     used in `libgit2`) yet, but that have been changed in this commit
    //
    // The algorithm in `libgit2` works by going through parents and keeping a linked list of blame
    // suspects. It can be visualized as follows:
    //
    // <---------------------------------------->
    // <---------------><----------------------->
    // <---><----------><----------------------->
    // <---><----------><-------><-----><------->
    // <---><---><-----><-------><-----><------->
    // <---><---><-----><-------><-----><-><-><->

    // Needed for `to_str`.
    use gix_object::bstr::ByteSlice;

    let absolute_path = worktree_path.join(file_path.to_str().unwrap());

    // TODO Verify that `imara-diff` tokenizes lines the same way `lines` does.
    let number_of_lines = std::fs::read_to_string(absolute_path).unwrap().lines().count();

    let mut hunks_to_blame: Vec<UnblamedHunk> = vec![UnblamedHunk::new(
        0..number_of_lines.try_into().unwrap(),
        suspect,
        Offset::Added(0),
    )];
    let mut out: Vec<BlameEntry> = vec![];

    for item in traverse {
        let item = item?;
        let suspect = item.id;

        let parent_ids = item.parent_ids;
        if parent_ids.is_empty() {
            // I’m not entirely sure if this is correct yet. `suspect`, at this point, is the `id` of
            // the last `item` that was yielded by `traverse`, so it makes sense to assign the
            // remaining lines to it, even though we don’t explicitly check whether that is true
            // here. We could perhaps use `needed_to_obtain` to compare `suspect` against an empty
            // tree to validate this assumption.
            out.extend(
                hunks_to_blame
                    .iter()
                    .map(|hunk| BlameEntry::from_unblamed_hunk(hunk, suspect)),
            );

            hunks_to_blame = vec![];

            break;
        }

        if parent_ids.len() == 1 {
            let parent_id: ObjectId = *parent_ids.last().unwrap();

            let changes_for_file_path = get_changes_for_file_path(&odb, file_path, item.id, parent_id);

            let [ref modification]: [gix_diff::tree::recorder::Change] = changes_for_file_path[..] else {
                // None of the changes affected the file we’re currently blaming. Pass blame to parent.
                hunks_to_blame
                    .iter_mut()
                    .for_each(|unblamed_hunk| unblamed_hunk.pass_blame(suspect, parent_id));

                continue;
            };

            match modification {
                gix_diff::tree::recorder::Change::Addition { .. } => {
                    // Every line that has not been blamed yet on a commit, is expected to have been
                    // added when the file was added to the repository.
                    out.extend(
                        hunks_to_blame
                            .iter()
                            .map(|hunk| BlameEntry::from_unblamed_hunk(hunk, suspect)),
                    );

                    hunks_to_blame = vec![];

                    break;
                }
                gix_diff::tree::recorder::Change::Deletion { .. } => todo!(),
                gix_diff::tree::recorder::Change::Modification { previous_oid, oid, .. } => {
                    let changes = get_changes(&odb, resource_cache, *oid, *previous_oid, file_path);

                    hunks_to_blame = process_changes(&mut out, &hunks_to_blame, &changes, suspect);
                    hunks_to_blame
                        .iter_mut()
                        .for_each(|unblamed_hunk| unblamed_hunk.pass_blame(suspect, parent_id));
                }
            }
        } else {
            for parent_id in parent_ids {
                let changes_for_file_path = get_changes_for_file_path(&odb, file_path, item.id, parent_id);

                let [ref modification]: [gix_diff::tree::recorder::Change] = changes_for_file_path[..] else {
                    // None of the changes affected the file we’re currently blaming. Pass blame
                    // to parent.
                    hunks_to_blame
                        .iter_mut()
                        .for_each(|unblamed_hunk| unblamed_hunk.clone_blame(suspect, parent_id));

                    continue;
                };

                match modification {
                    gix_diff::tree::recorder::Change::Addition { .. } => {
                        // Do nothing under the assumption that this always (or almost always)
                        // implies that the file comes from a different parent, compared to which
                        // it was modified, not added.
                        //
                        // TODO: I still have to figure out whether this is correct in all cases.
                    }
                    gix_diff::tree::recorder::Change::Deletion { .. } => todo!(),
                    gix_diff::tree::recorder::Change::Modification { previous_oid, oid, .. } => {
                        let changes = get_changes(&odb, resource_cache, *oid, *previous_oid, file_path);

                        hunks_to_blame = process_changes(&mut out, &hunks_to_blame, &changes, suspect);

                        hunks_to_blame
                            .iter_mut()
                            .for_each(|unblamed_hunk| unblamed_hunk.pass_blame(suspect, parent_id));
                    }
                }
            }

            hunks_to_blame
                .iter_mut()
                .for_each(|unblamed_hunk| unblamed_hunk.remove_blame(suspect));
        }
    }

    assert_eq!(hunks_to_blame, vec![]);

    // I don’t know yet whether it would make sense to use a data structure instead that preserves
    // order on insertion.
    out.sort_by(|a, b| a.range_in_blamed_file.start.cmp(&b.range_in_blamed_file.start));

    Ok(coalesce_blame_entries(out))
}
