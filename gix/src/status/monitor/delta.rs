use std::{cmp::Ordering, sync::atomic::AtomicBool};

use gix_error::Result;

use crate::status::Item;

/// An exact change to an owned status item, identified by its staged/worktree side, path and origin.
///
/// Worktree modifications originate in the index; directory contents and rewrites originate in
/// the directory walk. Replacing a tracked file with a directory can produce both at the same path.
/// Entry metadata is included. Content edits that keep items equal appear only in refresh coverage.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    /// An item appeared in the snapshot.
    Addition(Item),
    /// An item disappeared from the snapshot.
    Deletion(Item),
    /// An existing item's classification or metadata changed.
    Modification {
        /// The item from the previous successful snapshot.
        previous: Item,
        /// The item in the current snapshot.
        current: Item,
    },
}

pub(super) fn changes(mut previous: &[Item], mut current: &[Item], interrupt: &AtomicBool) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    loop {
        super::check_interrupted(interrupt)?;
        match (previous.first(), current.first()) {
            (Some(before), Some(after)) => match super::item_key(before).cmp(&super::item_key(after)) {
                Ordering::Less => {
                    changes.push(Change::Deletion(before.clone()));
                    previous = &previous[1..];
                }
                Ordering::Greater => {
                    changes.push(Change::Addition(after.clone()));
                    current = &current[1..];
                }
                Ordering::Equal => {
                    if before != after {
                        changes.push(Change::Modification {
                            previous: before.clone(),
                            current: after.clone(),
                        });
                    }
                    previous = &previous[1..];
                    current = &current[1..];
                }
            },
            (Some(before), None) => {
                changes.push(Change::Deletion(before.clone()));
                previous = &previous[1..];
            }
            (None, Some(after)) => {
                changes.push(Change::Addition(after.clone()));
                current = &current[1..];
            }
            (None, None) => return Ok(changes),
        }
    }
}
