//! Discover rename identities from the collected three-way inputs before resolving paths.

use std::collections::HashMap;

use bstr::{BStr, BString, ByteSlice};
use gix_diff::{rewrites::tracker, tree::visit::Change as RawChange, tree_with_rewrites::Change};
use gix_error::{Result, ResultExt, message};
use gix_object::FindExt;

use super::{directories, path::Collected};

#[derive(Default)]
pub(super) struct Changes {
    pub entries: Vec<Change>,
    pub directories: Vec<directories::DirectoryRename>,
}

/// Retain ORT's destination discovery order using the already-collected three inputs.
/// Directories added on one side, or unchanged on the opposite side, are deferred
/// until after the initial traversal. Directory-rename relevance requires immediate
/// traversal instead, including in their nested directories.
pub(super) fn destination_orders(collected: &super::path::Collected) -> [Vec<&BStr>; 2] {
    #[derive(Clone, Copy, Default)]
    struct Directory {
        // Surviving side below a removed directory, or all bits once additions
        // make its rename destinations relevant. Bits are base/current/other.
        rename_mask: u8,
        // Zero denotes the initial traversal; descendants share their deferred root's order.
        deferred: [usize; 2],
    }
    fn parent(path: &BStr) -> &BStr {
        path.rfind_byte(b'/').map_or("".into(), |slash| path[..slash].as_bstr())
    }

    let traversal = || {
        collected
            .entries
            .iter()
            .map(|(name, inputs)| (collected.names[name.clone()].as_bstr(), inputs))
    };
    let mut additions = HashMap::<&BStr, u8>::new();
    for (path, inputs) in traversal() {
        let files = inputs.iter().enumerate().fold(0, |mask, (side, entry)| {
            mask | if entry.is_some_and(|entry| entry.mode.is_no_tree()) {
                1 << side
            } else {
                0
            }
        });
        if matches!(files, 2 | 4) {
            *additions.entry(parent(path.as_bstr())).or_default() |= files;
        }
    }
    let mut directories = HashMap::<&BStr, Directory>::new();
    let mut deferred = [Vec::new(), Vec::new()];
    // Collection already established the three-input callback order.
    for (path, inputs) in traversal() {
        let directory_mask = inputs.iter().enumerate().fold(0, |mask, (side, entry)| {
            mask | if entry.is_some_and(|entry| entry.mode.is_tree()) {
                1 << side
            } else {
                0
            }
        });
        if directory_mask == 0 {
            continue;
        }
        let mut directory = directories.get(parent(path.as_bstr())).copied().unwrap_or_default();
        if directory.rename_mask != 7 && matches!(directory_mask, 3 | 5) {
            directory.rename_mask = directory_mask & 6;
        }
        for side in 1..=2 {
            let unchanged_opposite = inputs[0].is_some() && inputs[0] == inputs[3 - side];
            let new_directory =
                directory_mask == 1 << side && inputs.iter().flatten().all(|entry| entry.mode.is_tree());
            if directory.deferred[side - 1] == 0 && directory.rename_mask != 7 && (unchanged_opposite || new_directory)
            {
                deferred[side - 1].push(path.as_bstr());
                directory.deferred[side - 1] = deferred[side - 1].len();
            }
        }
        if matches!(directory.rename_mask, 2 | 4)
            && additions
                .get(path.as_bstr())
                .is_some_and(|mask| mask & directory.rename_mask != 0)
        {
            directory.rename_mask = 7;
        }
        directories.insert(path.as_bstr(), directory);
    }
    let deferred_orders = deferred.map(|paths| deferred_directory_order(&paths));
    std::array::from_fn(|index| {
        let side = index + 1;
        let mut destinations: Vec<_> = traversal()
            .filter_map(|(path, inputs)| {
                (inputs[side].is_some_and(|entry| entry.mode.is_no_tree())
                    && inputs[0].is_none_or(|entry| entry.mode.is_tree()))
                .then_some(path)
            })
            .collect();
        destinations.sort_by_cached_key(|path| {
            let group = directories
                .get(parent(path))
                .map_or(0, |directory| directory.deferred[index]);
            if group == 0 {
                0
            } else {
                deferred_orders[index][group - 1] + 1
            }
        });
        if destinations.is_sorted() {
            destinations.clear();
        }
        destinations
    })
}

/// ORT drains pending directories in Git's `hashmap` iteration order. This order
/// determines which exact rename consumes an ambiguous source, so retain its
/// FNV hashing, head insertion and growth behavior independently of Rust hashing.
fn deferred_directory_order(paths: &[&BStr]) -> Vec<usize> {
    if paths.len() < 2 {
        return vec![0; paths.len()];
    }
    let hashes: Vec<_> = paths
        .iter()
        .map(|path| {
            path.iter().fold(0x811c9dc5_u32, |hash, byte| {
                hash.wrapping_mul(0x01000193) ^ u32::from(*byte)
            }) as usize
        })
        .collect();
    let mut buckets = vec![Vec::new(); 64];
    for index in 0..paths.len() {
        let bucket = hashes[index] & (buckets.len() - 1);
        buckets[bucket].push(index);
        if index + 1 > buckets.len() * 80 / 100 {
            let size = buckets.len() * 4;
            let old = std::mem::replace(&mut buckets, vec![Vec::new(); size]);
            for chain in old {
                for index in chain.into_iter().rev() {
                    buckets[hashes[index] & (size - 1)].push(index);
                }
            }
        }
    }
    let mut order = vec![0; paths.len()];
    for (position, index) in buckets
        .into_iter()
        .flat_map(|chain| chain.into_iter().rev())
        .enumerate()
    {
        order[index] = position;
    }
    order
}

/// Like Git, only use a directory rename when the other side adds a leaf directly inside it or an ancestor.
pub(super) fn retain_relevant_directory_renames(changes: &mut Changes, other: &[Change]) {
    if changes.directories.is_empty() {
        return;
    }
    let relevant_directories: std::collections::HashSet<_> = other
        .iter()
        .filter(|change| {
            !change.entry_mode().is_tree() && matches!(change, Change::Addition { .. } | Change::Rewrite { .. })
        })
        .filter_map(|change| {
            let path = change.location();
            path.rfind_byte(b'/').map(|slash| path[..slash].as_bstr())
        })
        .collect();
    let is_relevant = |source_location: &bstr::BStr| {
        let mut path = source_location;
        loop {
            if relevant_directories.contains(path) {
                return true;
            }
            let Some(slash) = path.rfind_byte(b'/') else {
                return false;
            };
            path = path[..slash].as_bstr();
        }
    };
    changes
        .directories
        .retain(|directory| is_relevant(directory.source.as_bstr()));
}

#[expect(clippy::too_many_arguments)]
pub(super) fn collect(
    collected: &Collected,
    base_tree: &gix_hash::oid,
    side: usize,
    objects: &impl gix_object::FindObjectOrHeader,
    diff_resource_cache: &mut gix_diff::blob::Platform,
    rewrites: gix_diff::Rewrites,
    directory_renames: crate::tree::DirectoryRenames,
    destination_order: &[&BStr],
) -> Result<Changes> {
    let mut changes = Changes::default();
    let mut tracked = gix_diff::rewrites::Tracker::new(rewrites);
    for (name, inputs) in &collected.entries {
        let location = collected.names[name.clone()].as_bstr();
        let mut push = |change: RawChange| {
            if let Some(change) = tracked.try_push_change(change, location) {
                changes.entries.push(from_raw(&change, location));
            }
        };
        match (inputs[0], inputs[side]) {
            (Some(base), Some(current)) if base == current => {}
            (Some(base), Some(current)) if base.mode.is_tree() && current.mode.is_tree() => {}
            (Some(base), Some(current)) if base.mode.is_tree() != current.mode.is_tree() => {
                push(RawChange::Deletion {
                    entry_mode: base.mode,
                    oid: base.id,
                    relation: None,
                });
                push(RawChange::Addition {
                    entry_mode: current.mode,
                    oid: current.id,
                    relation: None,
                });
            }
            (Some(base), Some(current)) => push(RawChange::Modification {
                previous_entry_mode: base.mode,
                previous_oid: base.id,
                entry_mode: current.mode,
                oid: current.id,
            }),
            (Some(base), None) => push(RawChange::Deletion {
                entry_mode: base.mode,
                oid: base.id,
                relation: None,
            }),
            (None, Some(current)) => push(RawChange::Addition {
                entry_mode: current.mode,
                oid: current.id,
                relation: None,
            }),
            (None, None) => {}
        }
    }
    tracked
        .emit(
            |destination, source| {
                let change = match source {
                    Some(source) if source.kind == tracker::visit::SourceKind::Rename => {
                        let (id, entry_mode) = destination.change.oid_and_entry_mode();
                        Change::Rewrite {
                            source_location: source.location.to_owned(),
                            source_entry_mode: source.entry_mode,
                            source_id: match source.change {
                                RawChange::Modification { previous_oid, .. } => *previous_oid,
                                change => change.oid().to_owned(),
                            },
                            source_relation: source.change.relation(),
                            entry_mode,
                            id: id.to_owned(),
                            location: destination.location.to_owned(),
                            relation: destination.change.relation(),
                            similarity: source.similarity,
                            copy: false,
                        }
                    }
                    // Copies retain their sources and therefore participate in merges as additions.
                    _ => from_raw(&destination.change, destination.location),
                };
                changes.entries.push(change);
                std::ops::ControlFlow::Continue(())
            },
            diff_resource_cache,
            objects,
            |push| push_copy_sources(base_tree, objects, push),
            tracker::Context { destination_order },
        )
        .or_raise(|| message("Tree merge rename discovery failed"))?;
    if directory_renames != crate::tree::DirectoryRenames::Disabled {
        changes.directories = directories::infer(collected, &changes.entries, side);
    }
    Ok(changes)
}

fn from_raw(change: &RawChange, location: &BStr) -> Change {
    match *change {
        RawChange::Addition {
            entry_mode,
            oid,
            relation,
        } => Change::Addition {
            location: location.to_owned(),
            entry_mode,
            id: oid,
            relation,
        },
        RawChange::Deletion {
            entry_mode,
            oid,
            relation,
        } => Change::Deletion {
            location: location.to_owned(),
            entry_mode,
            id: oid,
            relation,
        },
        RawChange::Modification {
            previous_entry_mode,
            previous_oid,
            entry_mode,
            oid,
        } => Change::Modification {
            location: location.to_owned(),
            previous_entry_mode,
            previous_id: previous_oid,
            entry_mode,
            id: oid,
        },
    }
}

fn push_copy_sources(
    base_tree: &gix_hash::oid,
    objects: &impl gix_object::Find,
    push: &mut dyn FnMut(RawChange, &BStr),
) -> Result<()> {
    let mut pending = vec![(BString::default(), base_tree.to_owned())];
    let mut buf = Vec::new();
    while let Some((mut path, tree_id)) = pending.pop() {
        if tree_id == tree_id.kind().empty_tree() {
            continue;
        }
        let prefix_len = path.len();
        for entry in objects.find_tree_iter(&tree_id, &mut buf)? {
            let entry = entry?;
            path.extend_from_slice(entry.filename);
            if entry.mode.is_tree() {
                let mut directory = path.clone();
                directory.push(b'/');
                pending.push((directory, entry.oid.to_owned()));
            } else {
                push(
                    RawChange::Modification {
                        previous_entry_mode: entry.mode,
                        previous_oid: entry.oid.to_owned(),
                        entry_mode: entry.mode,
                        oid: entry.oid.to_owned(),
                    },
                    path.as_bstr(),
                );
            }
            path.truncate(prefix_len);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use gix_error::TestResult;

    #[test]
    fn an_empty_side_tree_need_not_exist_in_the_object_database() -> TestResult {
        let hash = gix_testtools::object_hash();
        let blob_id = gix_object::compute_hash(hash, gix_object::Kind::Blob, b"deleted")?;
        let entry = super::super::path::Entry {
            mode: gix_object::tree::EntryKind::Blob.into(),
            id: blob_id,
        };
        let collected = super::Collected {
            names: b"file".to_vec(),
            entries: vec![(0..4, [Some(entry), None, Some(entry)])],
            ..Default::default()
        };
        let worktree = gix_testtools::tempfile::tempdir()?;
        let mut resources = gix_diff::blob::Platform::new(
            Default::default(),
            gix_diff::blob::Pipeline::new(
                Default::default(),
                gix_filter::Pipeline::new(Default::default(), hash, Default::default()),
                Vec::new(),
                Default::default(),
            ),
            Default::default(),
            gix_worktree::Stack::new(
                worktree.path(),
                gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::default()),
                Default::default(),
                Vec::new(),
                Vec::new(),
            ),
        );
        let changes = super::collect(
            &collected,
            &hash.empty_tree(),
            1,
            &gix_object::find::Never::panic_on_access(),
            &mut resources,
            gix_diff::Rewrites::default(),
            crate::tree::DirectoryRenames::Disabled,
            &[],
        )?;
        assert_eq!(changes.entries.len(), 1, "the only base entry is deleted");
        assert!(
            matches!(&changes.entries[0], super::Change::Deletion { location, id, .. } if location == "file" && *id == blob_id),
            "collected deletions need no second tree traversal or object lookup"
        );
        Ok(())
    }
}
