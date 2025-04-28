use crate::types::{Change, UnblamedHunk};
use crate::{BlameCheckpoint, BlameEntry};
use gix_hash::ObjectId;
use std::num::NonZeroU32;
use std::ops::Range;

/// Updates blame entries from a checkpoint with newly detected changes.
///
/// # Arguments
/// * `checkpoint` - Previous checkpoint with blame entries
/// * `changes` - Changes since the checkpoint
/// * `suspect` - Commit ID being investigated
///
/// # Returns
/// Updated blame entries and new hunks to blame
pub(crate) fn update_checkpoint_blames_with_changes(
    checkpoint: BlameCheckpoint,
    changes: Vec<Change>,
    suspect: ObjectId,
) -> (Vec<BlameEntry>, Vec<UnblamedHunk>) {
    fn blame_fully_contained_by_change(
        blame_lines: &BlameLines,
        blame: &BlameEntry,
        change_lines: &ChangeLines,
        change: &Change,
    ) -> bool {
        blame_lines.get_remaining(blame) < change_lines.get_remaining(change)
    }

    let mut updated_blames = Vec::new();
    let mut new_hunks_to_blame = Vec::new();

    let mut blame_iter = checkpoint.entries.into_iter().peekable();

    // This nested loop iterates through changes and blame entries in parallel, tracking how many
    // lines have been processed in each. For each change type:
    // - Unchanged: Keep the original blame but adjust line numbers
    // - Deleted: Remove blame entries for deleted lines
    // - Added/Replaced: Create new hunks to be blamed later
    // The tracking ensures we correctly handle partial overlaps between changes and blame entries.
    'change: for change in changes {
        let mut change_assigned = ChangeLines::default();
        while let Some(blame) = blame_iter.peek_mut() {
            let mut blame_assigned = BlameLines::default();

            // For each of the three cases we have to check if the blame is fully contained by the change.
            // If so we can update the blame with the remaining length of the blame.
            // If not we have to update the blame with the remaining length of the change.
            match change {
                Change::Unchanged(ref range) => {
                    match blame_fully_contained_by_change(&blame_assigned, blame, &change_assigned, &change) {
                        true => {
                            updated_blames.push(BlameEntry {
                                start_in_blamed_file: range.start + change_assigned.assigned.get_assigned(),
                                start_in_source_file: blame.start_in_source_file,
                                len: blame.len,
                                commit_id: blame.commit_id,
                            });

                            change_assigned.assigned.add_assigned(blame.len.get());
                            blame_assigned.assigned.add_assigned(blame.len.get());
                        }
                        false => {
                            updated_blames.push(BlameEntry {
                                start_in_blamed_file: range.start + change_assigned.assigned.get_assigned(),
                                start_in_source_file: blame.start_in_source_file,
                                len: NonZeroU32::new(change_assigned.get_remaining(&change)).unwrap(),
                                commit_id: blame.commit_id,
                            });

                            blame_assigned
                                .assigned
                                .add_assigned(change_assigned.get_remaining(&change));
                            change_assigned
                                .assigned
                                .add_assigned(change_assigned.get_remaining(&change));
                        }
                    }
                }
                Change::Deleted(_start_deletion, _lines_deleted) => {
                    match blame_fully_contained_by_change(&blame_assigned, blame, &change_assigned, &change) {
                        true => {
                            blame_assigned.assigned.add_assigned(blame.len.get());
                            change_assigned.assigned.add_assigned(blame.len.get());
                        }
                        false => {
                            blame_assigned
                                .assigned
                                .add_assigned(change_assigned.get_remaining(&change));
                            change_assigned
                                .assigned
                                .add_assigned(change_assigned.get_remaining(&change));
                        }
                    }
                }
                Change::AddedOrReplaced(ref range, lines_deleted) => {
                    let new_unblamed_hunk = |range: &Range<u32>, suspect: ObjectId| UnblamedHunk {
                        range_in_blamed_file: range.clone(),
                        suspects: [(suspect, range.clone())].into(),
                    };
                    match blame_fully_contained_by_change(&blame_assigned, blame, &change_assigned, &change) {
                        true => {
                            if lines_deleted == 0 {
                                new_hunks_to_blame.push(new_unblamed_hunk(range, suspect));
                            }

                            change_assigned.assigned.add_assigned(blame.len.get());
                            blame_assigned.assigned.add_assigned(blame.len.get());
                        }
                        false => {
                            new_hunks_to_blame.push(new_unblamed_hunk(range, suspect));

                            blame_assigned
                                .assigned
                                .add_assigned(change_assigned.get_remaining(&change));
                            change_assigned
                                .assigned
                                .add_assigned(change_assigned.get_remaining(&change));
                        }
                    }
                }
            }

            // Check if the blame or the change is fully assigned.
            // If the blame is fully assigned we can continue with the next blame.
            // If the change is fully assigned we can continue with the next change.
            // Since we have a mutable reference to the blame we can update it and reset the assigned blame lines.
            // If both are fully assigned we can continue with the next blame and change.
            match (
                blame_assigned.has_remaining(blame),
                change_assigned.has_remaining(&change),
            ) {
                (true, true) => {
                    // Both have remaining
                    blame.update_blame(&blame_assigned.assigned);
                }
                (true, false) => {
                    // Change is fully assigned
                    blame.update_blame(&blame_assigned.assigned);
                    continue 'change;
                }
                (false, true) => {
                    // Blame is fully assigned
                    blame_iter.next();
                }
                (false, false) => {
                    // Both are fully assigned
                    blame_iter.next();
                    continue 'change;
                }
            }
        }
    }
    (updated_blames, new_hunks_to_blame)
}

impl BlameEntry {
    /// Updates the blame entry's line numbers by the given offset.
    ///
    /// This is used when processing changes to adjust the line numbers in the blame entry
    /// to account for lines that have already been processed. It updates:
    /// * The starting line in the blamed file
    /// * The starting line in the source file
    /// * The length of the blame entry
    pub(crate) fn update_blame(&mut self, offset: &LinesAssigned) {
        self.start_in_blamed_file += offset.get_assigned();
        self.start_in_source_file += offset.get_assigned();
        self.len = NonZeroU32::new(u32::from(self.len) - offset.get_assigned()).unwrap();
    }
}

/// Tracks the number of lines processed during blame updates
#[derive(Debug, Default)]
pub(crate) struct LinesAssigned {
    lines_assigned: u32,
}

impl LinesAssigned {
    /// Add lines to the count
    fn add_assigned(&mut self, lines: u32) {
        self.lines_assigned += lines;
    }

    /// Get current count
    fn get_assigned(&self) -> u32 {
        self.lines_assigned
    }
}

/// Tracks line assignments for blame entries
#[derive(Debug, Default)]
struct BlameLines {
    assigned: LinesAssigned,
}

impl BlameLines {
    /// Calculate remaining lines in a blame entry
    fn get_remaining(&self, blame: &BlameEntry) -> u32 {
        blame.len.get() - self.assigned.get_assigned()
    }

    /// Check if any lines remain
    fn has_remaining(&self, blame: &BlameEntry) -> bool {
        self.get_remaining(blame) > 0
    }
}

/// Tracks line assignments for changes
#[derive(Debug, Default)]
struct ChangeLines {
    assigned: LinesAssigned,
}

impl ChangeLines {
    /// Calculate remaining lines in a change
    fn get_remaining(&self, change: &Change) -> u32 {
        match &change {
            Change::Unchanged(range) => range.len() as u32 - self.assigned.get_assigned(),
            Change::AddedOrReplaced(_, deleted_in_before) => *deleted_in_before - self.assigned.get_assigned(),
            Change::Deleted(_, deleted_in_before) => *deleted_in_before - self.assigned.get_assigned(),
        }
    }

    /// Check if any lines remain
    fn has_remaining(&self, change: &Change) -> bool {
        self.get_remaining(change) > 0
    }
}
