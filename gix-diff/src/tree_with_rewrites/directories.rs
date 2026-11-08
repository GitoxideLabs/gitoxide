use std::collections::{BTreeMap, BTreeSet};

use bstr::{BStr, BString, ByteSlice};

use super::{Action, ChangeRef};
use crate::tree::visit::Change;

/// Buffers directory additions and deletions to infer directory renames from finalized file renames.
///
/// Inference runs after file matching and does not influence it. Only finalized file renames contribute,
/// not copies; directory object identity and similarity are not compared.
///
/// # Algorithm
///
/// 1. **Collect candidates.** Use [`push()`](Self::push) to buffer directory additions and deletions,
///    excluding modifications. Once all candidates are collected, [`index_paths()`](Self::index_paths)
///    builds a path-sorted index for binary search, preserving the changes' insertion order for emission.
/// 2. **Count votes.** For each file rename, [`record_rename()`](Self::record_rename) looks up the source
///    and destination's immediate parent directories. If the source directory was deleted and the
///    destination directory was added, increment that directory pair's count by one. Then ascend both
///    paths together, but only if the current directory names match, and repeat. Stop at the root, a
///    mismatching name, or a pair without both a deleted source and an added destination; do not skip
///    missing candidates to search higher ancestors. File names need not match.
///
///    For example, `old/sub/a` renamed to `new/sub/b` votes for `old/sub -> new/sub` and, because `sub`
///    matches, `old -> new`, provided both pairs are candidates. In contrast, `old/left/a` renamed to
///    `new/right/a` votes only for `old/left -> new/right`: differing directory names prevent ascent.
///
/// 3. **Choose destinations.** In [`emit()`](Self::emit), each source independently chooses the
///    destination with the highest vote count, provided it is the unique maximum. No absolute majority
///    is required. A tie or no votes leaves the source unpaired. Multiple sources may choose the same
///    destination; there is no global one-to-one matching.
/// 4. **Emit changes.** Walk the buffered changes in insertion order. Replace each paired deletion with
///    a [`ChangeRef::Rewrite`] using the source and destination directory metadata, with `copy: false`
///    and no similarity score. Suppress the selected destinations' additions, since the rewrites
///    represent them, and emit all remaining additions and deletions unchanged. Stop if the visitor
///    requests cancellation.
#[derive(Default)]
pub(super) struct Directories {
    /// Directory additions and deletions with their full paths, in insertion order.
    changes: Vec<(Change, BString)>,
    /// Indices into `changes`, sorted by path for binary search without reordering emission.
    paths: Vec<usize>,
    /// Source index to destination index to supporting file-rename count, using indices into `changes`.
    renames: BTreeMap<usize, BTreeMap<usize, usize>>,
}

impl Directories {
    pub fn push(&mut self, change: Change, location: &BStr) {
        self.changes.push((change, location.to_owned()));
    }

    pub fn index_paths(&mut self) {
        self.paths = (0..self.changes.len()).collect();
        self.paths.sort_by(|&a, &b| self.changes[a].1.cmp(&self.changes[b].1));
    }

    fn find(&self, path: &BStr, deletion: bool) -> Option<usize> {
        let index = self
            .paths
            .binary_search_by(|&index| self.changes[index].1.as_bstr().cmp(path))
            .ok()?;
        let index = self.paths[index];
        (matches!(self.changes[index].0, Change::Deletion { .. }) == deletion).then_some(index)
    }

    pub fn record_rename(&mut self, source_path: &BStr, dest_path: &BStr) {
        let (Some((mut source, _)), Some((mut dest, _))) = (split_parent(source_path), split_parent(dest_path)) else {
            return;
        };
        while let (Some(source_index), Some(dest_index)) = (self.find(source, true), self.find(dest, false)) {
            *self
                .renames
                .entry(source_index)
                .or_default()
                .entry(dest_index)
                .or_default() += 1;

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
            let Some((&dest, _)) = winners.next() else {
                continue;
            };
            // shortcut: ties stay unpaired; expose them when merge gains directory-split diagnostics.
            if winners.next().is_none() {
                renames.insert(source, dest);
                destinations.insert(dest);
            }
        }

        for (index, (change, location)) in self.changes.iter().enumerate() {
            if destinations.contains(&index) {
                continue;
            }
            let change = match renames.get(&index) {
                Some(&dest) => {
                    let (dest, dest_location) = &self.changes[dest];
                    ChangeRef::Rewrite {
                        source_location: location.as_ref(),
                        source_entry_mode: change.entry_mode(),
                        source_relation: change.relation(),
                        source_id: change.oid().to_owned(),
                        similarity: None,
                        entry_mode: dest.entry_mode(),
                        id: dest.oid().to_owned(),
                        location: dest_location.as_ref(),
                        relation: dest.relation(),
                        copy: false,
                    }
                }
                None => ChangeRef::from_tree_change(change, location.as_ref()),
            };
            visit(change)?;
        }
        std::ops::ControlFlow::Continue(())
    }
}

fn split_parent(path: &BStr) -> Option<(&BStr, &BStr)> {
    let slash = path.rfind_byte(b'/')?;
    Some((path[..slash].as_bstr(), path[slash + 1..].as_bstr()))
}
