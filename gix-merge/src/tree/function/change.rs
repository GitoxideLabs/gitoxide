//! Discover rename identities from the two base-to-side diffs before resolving paths.

use std::collections::HashMap;

use bstr::{BStr, ByteSlice};
use gix_diff::{
    tree::recorder::Location,
    tree_with_rewrites::{Change, ChangeRef},
};
use gix_error::{Result, ResultExt, message};
use gix_object::FindExt;

#[derive(Default)]
pub(super) struct Changes {
    pub entries: Vec<Change>,
    pub directories: Vec<gix_diff::tree_with_rewrites::DirectoryRename>,
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
                (inputs[side].is_some_and(|entry| entry.mode.is_no_tree()) && inputs[side] != inputs[0]).then_some(path)
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
    let is_irrelevant = |change: &Change| {
        let Change::Rewrite {
            source_location,
            entry_mode,
            ..
        } = change
        else {
            return false;
        };
        if !entry_mode.is_tree() {
            return false;
        }
        !is_relevant(source_location.as_bstr())
    };
    if !changes.entries.iter().any(&is_irrelevant) {
        return;
    }
    let mut retained = Vec::with_capacity(changes.entries.len());
    for change in changes.entries.drain(..) {
        if is_irrelevant(&change) {
            track_without_directory_rename(change.to_ref(), &mut retained);
        } else {
            retained.push(change);
        }
    }
    changes.entries = retained;
}

#[expect(clippy::too_many_arguments)]
pub(super) fn collect(
    base_tree: &gix_hash::oid,
    side_tree: &gix_hash::oid,
    base_buf: &[u8],
    side_buf: &mut Vec<u8>,
    objects: &impl gix_object::FindObjectOrHeader,
    diff_resource_cache: &mut gix_diff::blob::Platform,
    diff_state: &mut gix_diff::tree::State,
    rewrites: Option<gix_diff::Rewrites>,
    directory_renames: crate::tree::DirectoryRenames,
    destination_order: &[&BStr],
) -> Result<Changes> {
    let mut changes = Changes::default();
    if base_tree != side_tree {
        let side_tree = if side_tree == side_tree.kind().empty_tree() {
            gix_object::TreeRefIter::from_bytes(&[], side_tree.kind())
        } else {
            objects
                .find_tree_iter(side_tree, side_buf)
                .or_raise(|| message("Tree merge failed"))?
        };
        let outcome = gix_diff::tree_with_rewrites(
            gix_object::TreeRefIter::from_bytes(base_buf, base_tree.kind()),
            side_tree,
            diff_resource_cache,
            diff_state,
            objects,
            |change| {
                if directory_renames == crate::tree::DirectoryRenames::Disabled {
                    track_without_directory_rename(change, &mut changes.entries);
                } else {
                    track(change, &mut changes.entries);
                }
                Ok(std::ops::ControlFlow::Continue(()))
            },
            gix_diff::tree_with_rewrites::Options {
                location: Some(Location::Path),
                rewrites,
            },
            gix_diff::tree_with_rewrites::Context { destination_order },
        )
        .or_raise(|| message("Tree merge failed"))?;
        if directory_renames != crate::tree::DirectoryRenames::Disabled {
            changes.directories = outcome.map_or_else(Vec::new, |outcome| outcome.directory_renames);
        }
    }
    Ok(changes)
}

fn track_without_directory_rename(change: ChangeRef<'_>, changes: &mut Vec<Change>) {
    if let ChangeRef::Rewrite {
        source_location,
        source_entry_mode,
        source_id,
        source_relation,
        entry_mode,
        id,
        location,
        relation,
        ..
    } = change
        && entry_mode.is_tree()
    {
        // Keep structural boundaries for file/directory conflicts, but not the inferred relocation.
        track(
            ChangeRef::Deletion {
                location: source_location,
                entry_mode: source_entry_mode,
                id: source_id,
                relation: source_relation,
            },
            changes,
        );
        track(
            ChangeRef::Addition {
                location,
                entry_mode,
                id,
                relation,
            },
            changes,
        );
    } else {
        track(change, changes);
    }
}

/// Copy sources remain in place and therefore contribute ordinary additions to a merge.
fn track(change: ChangeRef<'_>, changes: &mut Vec<Change>) {
    if change.entry_mode().is_tree() && matches!(change, ChangeRef::Modification { .. }) {
        return;
    }
    changes.push(match change.into_owned() {
        Change::Rewrite {
            id,
            entry_mode,
            location,
            relation,
            copy: true,
            ..
        } => Change::Addition {
            location,
            relation,
            entry_mode,
            id,
        },
        other => other,
    });
}

#[cfg(test)]
mod tests {
    use gix_error::TestResult;
    use gix_object::WriteTo;

    #[test]
    fn an_empty_side_tree_need_not_exist_in_the_object_database() -> TestResult {
        let hash = gix_testtools::object_hash();
        let blob_id = gix_object::compute_hash(hash, gix_object::Kind::Blob, b"deleted")?;
        let mut base_buf = Vec::new();
        gix_object::Tree {
            entries: vec![gix_object::tree::Entry {
                mode: gix_object::tree::EntryKind::Blob.into(),
                filename: "file".into(),
                oid: blob_id,
            }],
        }
        .write_to(&mut base_buf)?;
        let base_tree_id = gix_object::compute_hash(hash, gix_object::Kind::Tree, &base_buf)?;
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
            &base_tree_id,
            &hash.empty_tree(),
            &base_buf,
            &mut b"previous side tree bytes".to_vec(),
            &gix_object::find::Never::panic_on_access(),
            &mut resources,
            &mut gix_diff::tree::State::default(),
            None,
            crate::tree::DirectoryRenames::Disabled,
            &[],
        )?;
        assert_eq!(changes.entries.len(), 1, "the only base entry is deleted");
        assert!(
            matches!(&changes.entries[0], super::Change::Deletion { location, id, .. } if location == "file" && *id == blob_id),
            "the implicit empty tree is diffed without consulting the object database or stale buffer"
        );
        Ok(())
    }
}
