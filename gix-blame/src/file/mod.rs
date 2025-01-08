//! A module with low-level types and functions.

use std::num::NonZeroU32;
use std::ops::Range;

use gix_hash::ObjectId;

use crate::types::{BlameEntry, Either, LineRange};
use crate::types::{Change, Offset, UnblamedHunk};

pub(super) mod function;

/// Compare a section from the *Blamed File* (`hunk`) with a change from a diff and see if there
/// is an intersection with `change`. Based on that intersection, we may generate a [`BlameEntry`] for `out`
/// and/or split the `hunk` into multiple.
///
/// This is the core of the blame implementation as it matches regions in *Source File* to the *Blamed File*.
fn process_change(
    new_hunks_to_blame: &mut Vec<UnblamedHunk>,
    offset: &mut Offset,
    suspect: ObjectId,
    parent: ObjectId,
    hunk: Option<UnblamedHunk>,
    change: Option<Change>,
) -> (Option<UnblamedHunk>, Option<Change>) {
    /// Since `range_with_end` is a range that is not inclusive at the end,
    /// `range_with_end.end` is not part of `range_with_end`.
    /// The first line that is `range_with_end.end - 1`.
    fn actual_end_in_range(test: &Range<u32>, containing_range: &Range<u32>) -> bool {
        (test.end - 1) >= containing_range.start && test.end <= containing_range.end
    }

    // # General Rules
    // 1. If there is no suspect, immediately reschedule `hunk` and redo processing of `change`.
    //
    // # Detailed Rules
    // 1. whenever we do *not* return `hunk`, it must be added to `new_hunks_to_blame`, shifted with `offset`
    // 2. return `hunk` if it is not fully covered by changes yet.
    // 3. `change` *must* be returned if it is not fully included in `hunk`.
    match (hunk, change) {
        (Some(hunk), Some(Change::Unchanged(unchanged))) => {
            let Some(range_in_suspect) = hunk.suspects.get(&suspect) else {
                // We don’t clone blame to `parent` as `suspect` has nothing to do with this
                // `hunk`.
                new_hunks_to_blame.push(hunk);
                return (None, Some(Change::Unchanged(unchanged)));
            };

            match (
                range_in_suspect.contains(&unchanged.start),
                actual_end_in_range(&unchanged, range_in_suspect),
            ) {
                (_, true) => {
                    //     <------>  (hunk)
                    // <------->     (unchanged)
                    //
                    // <---------->  (hunk)
                    //     <--->     (unchanged)

                    // skip over unchanged - there will be changes right after.
                    (Some(hunk), None)
                }
                (true, false) => {
                    // <-------->     (hunk)
                    //     <------->  (unchanged)

                    // Nothing to do with `hunk` except shifting it,
                    // but `unchanged` needs to be checked against the next hunk to catch up.
                    new_hunks_to_blame.push(hunk.clone_blame(suspect, parent).shift_by(parent, *offset));
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

                        // Let changes catch up with us.
                        (Some(hunk), None)
                    } else {
                        // <-->          (hunk)
                        //       <---->  (unchanged)
                        //
                        //    <--->      (hunk)
                        // <---------->  (unchanged)

                        // Nothing to do with `hunk` except shifting it,
                        // but `unchanged` needs to be checked against the next hunk to catch up.
                        new_hunks_to_blame.push(hunk.clone_blame(suspect, parent).shift_by(parent, *offset));
                        (None, Some(Change::Unchanged(unchanged)))
                    }
                }
            }
        }
        (Some(hunk), Some(Change::AddedOrReplaced(added, number_of_lines_deleted))) => {
            let Some(range_in_suspect) = hunk.suspects.get(&suspect).cloned() else {
                new_hunks_to_blame.push(hunk);
                return (None, Some(Change::AddedOrReplaced(added, number_of_lines_deleted)));
            };

            let suspect_contains_added_start = range_in_suspect.contains(&added.start);
            let suspect_contains_added_end = actual_end_in_range(&added, &range_in_suspect);
            match (suspect_contains_added_start, suspect_contains_added_end) {
                (true, true) => {
                    // A perfect match of lines to take out of the unblamed portion.
                    // <---------->  (hunk)
                    //     <--->     (added)
                    //     <--->     (blamed)
                    // <-->     <->  (new hunk)

                    // Split hunk at the start of added.
                    let hunk_starting_at_added = match hunk.split_at(suspect, added.start) {
                        Either::Left(hunk) => {
                            // `added` starts with `hunk`, nothing to split.
                            hunk
                        }
                        Either::Right((before, after)) => {
                            // requeue the left side `before` after offsetting it…
                            new_hunks_to_blame.push(before.clone_blame(suspect, parent).shift_by(parent, *offset));
                            // …and treat `after` as `new_hunk`, which contains the `added` range.
                            after
                        }
                    };

                    *offset += added.end - added.start;
                    *offset -= number_of_lines_deleted;

                    // The overlapping `added` section was successfully located.
                    // Re-split at the end of `added` to continue with what's after.
                    match hunk_starting_at_added.split_at(suspect, added.end) {
                        Either::Left(hunk) => {
                            new_hunks_to_blame.push(hunk);

                            // Nothing to split, so we are done with this hunk.
                            (None, None)
                        }
                        Either::Right((hunk, after)) => {
                            new_hunks_to_blame.push(hunk);

                            // Keep processing the unblamed range after `added`
                            (Some(after), None)
                        }
                    }
                }
                (true, false) => {
                    // Added overlaps towards the end of `hunk`.
                    // <-------->     (hunk)
                    //     <------->  (added)
                    //     <---->     (blamed)
                    // <-->           (new hunk)

                    let hunk_starting_at_added = match hunk.split_at(suspect, added.start) {
                        Either::Left(hunk) => hunk,
                        Either::Right((before, after)) => {
                            // Keep looking for the left side of the unblamed portion.
                            new_hunks_to_blame.push(before.clone_blame(suspect, parent).shift_by(parent, *offset));
                            after
                        }
                    };

                    // We can 'blame' the overlapping area of `added` and `hunk`.
                    new_hunks_to_blame.push(hunk_starting_at_added);
                    // Keep processing `added`, it's portion past `hunk` may still contribute.
                    (None, Some(Change::AddedOrReplaced(added, number_of_lines_deleted)))
                }
                (false, true) => {
                    // Added reaches into the hunk, so we blame only the overlapping portion of it.
                    //    <------->  (hunk)
                    // <------>      (added)
                    //    <--->      (blamed)
                    //         <-->  (new hunk)

                    *offset += added.end - added.start;
                    *offset -= number_of_lines_deleted;

                    match hunk.split_at(suspect, added.end) {
                        Either::Left(hunk) => {
                            new_hunks_to_blame.push(hunk);

                            (None, None)
                        }
                        Either::Right((before, after)) => {
                            new_hunks_to_blame.push(before);

                            (Some(after), None)
                        }
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

                        *offset += added.end - added.start;
                        *offset -= number_of_lines_deleted;

                        // Let changes catchup with `hunk` after letting `added` contribute to the offset.
                        (Some(hunk), None)
                    } else if range_in_suspect.end <= added.start {
                        // <-->          (hunk)
                        //       <---->  (added)

                        // Retry `hunk` once there is overlapping changes to process.
                        new_hunks_to_blame.push(hunk.clone_blame(suspect, parent).shift_by(parent, *offset));

                        // Let hunks catchup with this change.
                        (
                            None,
                            Some(Change::AddedOrReplaced(added.clone(), number_of_lines_deleted)),
                        )
                    } else {
                        // Discard the left side of `added`, keep track of `blamed`, and continue with the
                        // right side of added that is going past `hunk`.
                        //    <--->      (hunk)
                        // <---------->  (added)
                        //    <--->      (blamed)

                        // Successfully blame the whole range.
                        new_hunks_to_blame.push(hunk);

                        // And keep processing `added` with future `hunks` that might be affected by it.
                        (
                            None,
                            Some(Change::AddedOrReplaced(added.clone(), number_of_lines_deleted)),
                        )
                    }
                }
            }
        }
        (Some(hunk), Some(Change::Deleted(line_number_in_destination, number_of_lines_deleted))) => {
            let Some(range_in_suspect) = hunk.suspects.get(&suspect) else {
                new_hunks_to_blame.push(hunk);
                return (
                    None,
                    Some(Change::Deleted(line_number_in_destination, number_of_lines_deleted)),
                );
            };

            if line_number_in_destination < range_in_suspect.start {
                //     <--->  (hunk)
                //  |         (line_number_in_destination)

                // Track the shift to `hunk` as it affects us, and keep catching up with changes.
                *offset -= number_of_lines_deleted;
                (Some(hunk), None)
            } else if line_number_in_destination < range_in_suspect.end {
                //  <----->  (hunk)
                //     |     (line_number_in_destination)

                let new_hunk = match hunk.split_at(suspect, line_number_in_destination) {
                    Either::Left(hunk) => {
                        // Nothing to split as `line_number_in_destination` is directly at start of `hunk`
                        hunk
                    }
                    Either::Right((before, after)) => {
                        // `before` isn't affected by deletion, so keep it for later.
                        new_hunks_to_blame.push(before.clone_blame(suspect, parent).shift_by(parent, *offset));
                        // after will be affected by offset, and we will see if there are more changes affecting it.
                        after
                    }
                };
                *offset -= number_of_lines_deleted;
                (Some(new_hunk), None)
            } else {
                //  <--->     (hunk)
                //         |  (line_number_in_destination)

                // Catchup with changes.
                new_hunks_to_blame.push(hunk.clone_blame(suspect, parent).shift_by(parent, *offset));

                (
                    None,
                    Some(Change::Deleted(line_number_in_destination, number_of_lines_deleted)),
                )
            }
        }
        (Some(hunk), None) => {
            // nothing to do - changes are exhausted, re-evaluate `hunk`.
            new_hunks_to_blame.push(hunk.clone_blame(suspect, parent).shift_by(parent, *offset));
            (None, None)
        }
        (None, Some(Change::Unchanged(_))) => {
            // Nothing changed past the blamed range - do nothing.
            (None, None)
        }
        (None, Some(Change::AddedOrReplaced(added, number_of_lines_deleted))) => {
            // Keep track of the shift to apply to hunks in the future.
            *offset += added.len() as u32;
            *offset -= number_of_lines_deleted;
            (None, None)
        }
        (None, Some(Change::Deleted(_, number_of_lines_deleted))) => {
            // Keep track of the shift to apply to hunks in the future.
            *offset -= number_of_lines_deleted;
            (None, None)
        }
        (None, None) => {
            // Noop, caller shouldn't do that, but not our problem.
            (None, None)
        }
    }
}

/// Consume `hunks_to_blame` and `changes` to pair up matches ranges (also overlapping) with each other.
/// Once a match is found, it's pushed onto `out`.
fn process_changes(
    hunks_to_blame: Vec<UnblamedHunk>,
    changes: Vec<Change>,
    suspect: ObjectId,
    parent: ObjectId,
) -> Vec<UnblamedHunk> {
    let mut hunks_iter = hunks_to_blame.into_iter();
    let mut changes_iter = changes.into_iter();

    let mut hunk = hunks_iter.next();
    let mut change = changes_iter.next();

    let mut new_hunks_to_blame = Vec::new();
    let mut offset_in_destination = Offset::Added(0);

    loop {
        (hunk, change) = process_change(
            &mut new_hunks_to_blame,
            &mut offset_in_destination,
            suspect,
            parent,
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

impl UnblamedHunk {
    fn shift_by(mut self, suspect: ObjectId, offset: Offset) -> Self {
        self.suspects.entry(suspect).and_modify(|e| *e = e.shift_by(offset));
        self
    }

    fn split_at(self, suspect: ObjectId, line_number_in_destination: u32) -> Either<Self, (Self, Self)> {
        match self.suspects.get(&suspect) {
            None => Either::Left(self),
            Some(range_in_suspect) => {
                if !range_in_suspect.contains(&line_number_in_destination) {
                    return Either::Left(self);
                }

                let split_at_from_start = line_number_in_destination - range_in_suspect.start;
                if split_at_from_start > 0 {
                    let new_suspects_before = self
                        .suspects
                        .iter()
                        .map(|(suspect, range)| (*suspect, range.start..(range.start + split_at_from_start)));

                    let new_suspects_after = self
                        .suspects
                        .iter()
                        .map(|(suspect, range)| (*suspect, (range.start + split_at_from_start)..range.end));

                    let new_hunk_before = Self {
                        range_in_blamed_file: self.range_in_blamed_file.start
                            ..(self.range_in_blamed_file.start + split_at_from_start),
                        suspects: new_suspects_before.collect(),
                    };
                    let new_hunk_after = Self {
                        range_in_blamed_file: (self.range_in_blamed_file.start + split_at_from_start)
                            ..(self.range_in_blamed_file.end),
                        suspects: new_suspects_after.collect(),
                    };

                    Either::Right((new_hunk_before, new_hunk_after))
                } else {
                    Either::Left(self)
                }
            }
        }
    }

    /// Transfer all ranges from the commit at `from` to the commit at `to`.
    fn pass_blame(&mut self, from: ObjectId, to: ObjectId) {
        if let Some(range_in_suspect) = self.suspects.remove(&from) {
            self.suspects.insert(to, range_in_suspect);
        }
    }

    // TODO
    // Should this also accept `&mut self` as the other functions do?
    fn clone_blame(mut self, from: ObjectId, to: ObjectId) -> Self {
        if let Some(range_in_suspect) = self.suspects.get(&from) {
            self.suspects.insert(to, range_in_suspect.clone());
        }
        self
    }

    fn remove_blame(&mut self, suspect: ObjectId) {
        self.suspects.remove(&suspect);
    }
}

impl BlameEntry {
    /// Create an offset from a portion of the *Blamed File*.
    fn from_unblamed_hunk(unblamed_hunk: &UnblamedHunk, commit_id: ObjectId) -> Option<Self> {
        let range_in_source_file = unblamed_hunk.suspects.get(&commit_id)?;

        Some(Self {
            start_in_blamed_file: unblamed_hunk.range_in_blamed_file.start,
            start_in_source_file: range_in_source_file.start,
            len: force_non_zero(range_in_source_file.len() as u32),
            commit_id,
        })
    }
}

fn force_non_zero(n: u32) -> NonZeroU32 {
    NonZeroU32::new(n).expect("BUG: hunks are never empty")
}

#[cfg(test)]
mod tests;
