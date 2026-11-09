use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use bstr::{BStr, BString, ByteSlice};

use super::{Action, ChangeRef};
use crate::tree::visit::Change;

/// Buffers directory changes to infer directory renames from finalized file renames.
///
/// Inference runs after file matching and does not influence it. Only finalized file renames contribute,
/// not copies; directory object identity and similarity are not compared.
///
/// # Algorithm
///
/// 1. **Collect candidates.** Use [`push()`](Self::push) to buffer directory changes, including metadata
///    for modifications already emitted to the visitor. [`index_paths()`](Self::index_paths)
///    builds a path-sorted index for binary search, preserving the changes' insertion order for emission.
/// 2. **Count votes.** For each file rename, [`record_rename()`](Self::record_rename) looks up the source
///    and destination's immediate parent directories. If the source directory was deleted, increment
///    that directory pair's count by one, including existing destinations and the repository root.
///    Then ascend both paths together, but only if the current directory names match, and repeat.
///    Stop at the root, a mismatching name, or a source that wasn't deleted; do not skip missing
///    candidates to search higher ancestors. File names need not match.
///
///    For example, `old/sub/a` renamed to `new/sub/b` votes for `old/sub -> new/sub` and, because `sub`
///    matches, `old -> new`, provided both source directories were deleted. In contrast, `old/left/a` renamed to
///    `new/right/a` votes only for `old/left -> new/right`: differing directory names prevent ascent.
///
/// 3. **Choose destinations.** In [`emit()`](Self::emit), each source independently chooses the
///    destination with the highest vote count, provided it is the unique maximum. No absolute majority
///    is required. A tie, no votes, or a winner without directory metadata leaves the source unpaired.
///    Multiple sources may choose the same destination; there is no global one-to-one matching.
/// 4. **Emit changes.** Walk the buffered changes in insertion order. Replace each paired deletion with
///    a [`ChangeRef::Rewrite`] using the source and destination directory metadata, with `copy: false`
///    and no similarity score. Suppress the selected destinations' additions, since the rewrites
///    represent them, and emit all remaining additions and deletions unchanged. Modifications were
///    already emitted and are not repeated. Stop if the visitor requests cancellation.
#[derive(Default)]
pub(super) struct Directories {
    /// Directory changes with their ranges in `path_backing`, in insertion order.
    changes: Vec<(Change, Range<usize>)>,
    /// All directory paths, stored together to avoid allocating for each change.
    path_backing: Vec<u8>,
    /// Indices into `changes`, sorted by path for binary search without reordering emission.
    paths: Vec<usize>,
    /// Source index to destination path to supporting file-rename count, including unrepresented destinations.
    renames: BTreeMap<usize, BTreeMap<BString, usize>>,
}

impl Directories {
    pub fn push(&mut self, change: Change, location: &BStr) {
        let start = self.path_backing.len();
        self.path_backing.extend_from_slice(location);
        self.changes.push((change, start..self.path_backing.len()));
    }

    pub fn index_paths(&mut self) {
        self.paths = (0..self.changes.len()).collect();
        self.paths.sort_by(|&a, &b| {
            self.path_backing[self.changes[a].1.clone()].cmp(&self.path_backing[self.changes[b].1.clone()])
        });
    }

    fn find(&self, path: &BStr, deletion: bool) -> Option<usize> {
        let index = self
            .paths
            .binary_search_by(|&index| self.path_backing[self.changes[index].1.clone()].as_bstr().cmp(path))
            .ok()?;
        let index = self.paths[index];
        (matches!(self.changes[index].0, Change::Deletion { .. }) == deletion).then_some(index)
    }

    pub fn record_rename(&mut self, source_path: &BStr, dest_path: &BStr) {
        let (Some((mut source, _)), Some((mut dest, _))) = (split_parent(source_path), split_parent(dest_path)) else {
            return;
        };
        while let Some(source_index) = self.find(source, true) {
            let candidates = self.renames.entry(source_index).or_default();
            if let Some(count) = candidates.get_mut(dest) {
                *count += 1;
            } else {
                candidates.insert(dest.to_owned(), 1);
            }

            // Like Git, infer ancestors only while their relative directory suffixes agree.
            let (Some((source_parent, source_name)), Some((dest_parent, dest_name))) =
                (split_parent(source), split_parent(dest))
            else {
                break;
            };
            if source_name != dest_name {
                break;
            }
            (source, dest) = (source_parent, dest_parent);
        }
    }

    pub fn emit(&self, mut visit: impl FnMut(ChangeRef<'_>) -> Action) -> Action {
        let mut renames = BTreeMap::new();
        let mut destinations = BTreeSet::new();
        for (&source, candidates) in &self.renames {
            let max_count = candidates.values().copied().max().unwrap_or(0);
            let mut winners = candidates.iter().filter(|(_, count)| **count == max_count);
            let Some((dest, _)) = winners.next() else {
                continue;
            };
            // shortcut: ties stay unpaired; expose them when merge gains directory-split diagnostics.
            if winners.next().is_some() {
                continue;
            }
            let Some(dest) = self.find(dest.as_bstr(), false) else {
                // shortcut: root destinations stay unpaired until tree changes can represent the root.
                continue;
            };
            renames.insert(source, dest);
            destinations.insert(dest);
        }

        for (index, (change, location)) in self.changes.iter().enumerate() {
            if destinations.contains(&index) || matches!(change, Change::Modification { .. }) {
                continue;
            }
            let change = match renames.get(&index) {
                Some(&dest) => {
                    let (dest, dest_location) = &self.changes[dest];
                    ChangeRef::Rewrite {
                        source_location: self.path_backing[location.clone()].as_bstr(),
                        source_entry_mode: change.entry_mode(),
                        source_relation: change.relation(),
                        source_id: change.oid().to_owned(),
                        similarity: None,
                        entry_mode: dest.entry_mode(),
                        id: dest.oid().to_owned(),
                        location: self.path_backing[dest_location.clone()].as_bstr(),
                        relation: dest.relation(),
                        copy: false,
                    }
                }
                None => ChangeRef::from_tree_change(change, self.path_backing[location.clone()].as_bstr()),
            };
            visit(change)?;
        }
        std::ops::ControlFlow::Continue(())
    }
}

fn split_parent(path: &BStr) -> Option<(&BStr, &BStr)> {
    if path.is_empty() {
        return None;
    }
    Some(match path.rfind_byte(b'/') {
        Some(slash) => (path[..slash].as_bstr(), path[slash + 1..].as_bstr()),
        None => (b"".as_bstr(), path),
    })
}
