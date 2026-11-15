//! Discover rename identities from the two base-to-side diffs before resolving paths.

use bstr::ByteSlice;
use gix_diff::{
    tree::recorder::Location,
    tree_with_rewrites::{Change, ChangeRef},
};
use gix_error::{Result, ResultExt, message};
use gix_object::FindExt;

/// Like Git, only use a directory rename when the other side adds a leaf directly inside it or an ancestor.
pub(super) fn retain_relevant_directory_renames(changes: &mut Vec<Change>, other: &[Change]) {
    if !changes
        .iter()
        .any(|change| matches!(&change, Change::Rewrite { entry_mode, .. } if entry_mode.is_tree()))
    {
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
        let mut path = source_location.as_bstr();
        loop {
            if relevant_directories.contains(path) {
                return false;
            }
            let Some(slash) = path.rfind_byte(b'/') else {
                return true;
            };
            path = path[..slash].as_bstr();
        }
    };
    if !changes.iter().any(&is_irrelevant) {
        return;
    }
    let mut retained = Vec::with_capacity(changes.len());
    for change in changes.drain(..) {
        if is_irrelevant(&change) {
            track_without_directory_rename(change.to_ref(), &mut retained);
        } else {
            retained.push(change);
        }
    }
    *changes = retained;
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
) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    if base_tree != side_tree {
        let side_tree = if side_tree == side_tree.kind().empty_tree() {
            gix_object::TreeRefIter::from_bytes(&[], side_tree.kind())
        } else {
            objects
                .find_tree_iter(side_tree, side_buf)
                .or_raise(|| message("Tree merge failed"))?
        };
        gix_diff::tree_with_rewrites(
            gix_object::TreeRefIter::from_bytes(base_buf, base_tree.kind()),
            side_tree,
            diff_resource_cache,
            diff_state,
            objects,
            |change| {
                if directory_renames == crate::tree::DirectoryRenames::Disabled {
                    track_without_directory_rename(change, &mut changes);
                } else {
                    track(change, &mut changes);
                }
                Ok(std::ops::ControlFlow::Continue(()))
            },
            gix_diff::tree_with_rewrites::Options {
                location: Some(Location::Path),
                rewrites,
            },
        )
        .or_raise(|| message("Tree merge failed"))?;
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
        )?;
        assert_eq!(changes.len(), 1, "the only base entry is deleted");
        assert!(
            matches!(&changes[0], super::Change::Deletion { location, id, .. } if location == "file" && *id == blob_id),
            "the implicit empty tree is diffed without consulting the object database or stale buffer"
        );
        Ok(())
    }
}
