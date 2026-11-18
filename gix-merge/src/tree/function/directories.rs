use std::collections::{BTreeMap, HashSet};

use bstr::{BStr, BString, ByteSlice};
use gix_diff::tree_with_rewrites::Change;

use super::path::Collected;

#[derive(Debug, Clone)]
pub(super) struct DirectoryRename {
    pub source: BString,
    pub destinations: nonempty::NonEmpty<BString>,
}

/// Infer removed directories' destinations from finalized file renames, retaining ties and root destinations.
pub(super) fn infer(collected: &Collected, changes: &[Change], side: usize) -> Vec<DirectoryRename> {
    let removed: HashSet<_> = collected
        .entries
        .iter()
        .filter(|(_, inputs)| {
            inputs[0].is_some_and(|entry| entry.mode.is_tree())
                && inputs[side].is_none_or(|entry| entry.mode.is_no_tree())
        })
        .map(|(name, _)| collected.names[name.clone()].as_bstr())
        .collect();
    let mut votes = BTreeMap::<&BStr, BTreeMap<&BStr, usize>>::new();
    for change in changes {
        let Change::Rewrite {
            source_location,
            location,
            entry_mode,
            copy: false,
            ..
        } = change
        else {
            continue;
        };
        if entry_mode.is_tree() {
            continue;
        }
        let (Some((mut source, _)), Some((mut destination, _))) = (
            split_parent(source_location.as_bstr()),
            split_parent(location.as_bstr()),
        ) else {
            continue;
        };
        while removed.contains(source) {
            *votes.entry(source).or_default().entry(destination).or_default() += 1;
            let (Some((source_parent, source_name)), Some((destination_parent, destination_name))) =
                (split_parent(source), split_parent(destination))
            else {
                break;
            };
            // Ancestors receive votes only when the relative directory suffixes agree.
            if source_name != destination_name {
                break;
            }
            (source, destination) = (source_parent, destination_parent);
        }
    }
    votes
        .into_iter()
        .map(|(source, candidates)| {
            let maximum = candidates
                .values()
                .copied()
                .max()
                .expect("a removed directory has at least one rename vote");
            DirectoryRename {
                source: source.to_owned(),
                destinations: nonempty::NonEmpty::collect(
                    candidates
                        .into_iter()
                        .filter(|(_, count)| *count == maximum)
                        .map(|(destination, _)| destination.to_owned()),
                )
                .expect("at least one destination has the maximum vote count"),
            }
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use bstr::{BStr, ByteSlice};
    use gix_error::TestResult;
    use gix_object::tree::EntryKind;

    use super::{Collected, infer};
    use crate::tree::function::path::Entry;

    #[test]
    fn votes_preserve_suffixes_ties_root_and_existing_destinations() -> TestResult {
        let tree = Entry {
            mode: EntryKind::Tree.into(),
            id: gix_testtools::object_hash().null(),
        };
        let leaf = Entry {
            mode: EntryKind::Blob.into(),
            ..tree
        };
        let mut collected = Collected::default();
        for (path, side) in [
            ("old", None),
            ("old/sub", None),
            ("old/left", None),
            ("top", None),
            ("stay", Some(tree)),
            ("stay/sub", None),
            ("replaced", Some(leaf)),
        ] {
            let start = collected.names.len();
            collected.names.extend_from_slice(path.as_bytes());
            collected
                .entries
                .push((start..collected.names.len(), [Some(tree), side, Some(tree)]));
        }
        let rename = |source: &str, destination: &str, copy| super::Change::Rewrite {
            source_location: source.into(),
            source_entry_mode: leaf.mode,
            source_id: leaf.id,
            source_relation: None,
            entry_mode: leaf.mode,
            id: leaf.id,
            location: destination.into(),
            relation: None,
            similarity: None,
            copy,
        };
        let changes = [
            rename("old/sub/a", "new/sub/b", false),
            rename("old/sub/c", "new/sub/d", false),
            rename("old/sub/e", "left/sub/e", false),
            rename("old/left/a", "new/right/a", false),
            rename("top/a", "a", false),
            rename("top/b", "right/b", false),
            rename("top/c", "extra/c", true),
            rename("stay/sub/a", "new/sub/a", false),
            rename("replaced/a", "existing/a", false),
        ];
        let inferred = infer(&collected, &changes, 1);
        let destinations: Vec<_> = inferred
            .iter()
            .map(|directory| {
                (
                    directory.source.as_bstr(),
                    directory
                        .destinations
                        .iter()
                        .map(|path| path.as_bstr())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(
            destinations,
            vec![
                (BStr::new("old"), vec![BStr::new("new")]),
                (BStr::new("old/left"), vec![BStr::new("new/right")]),
                (BStr::new("old/sub"), vec![BStr::new("new/sub")]),
                (BStr::new("replaced"), vec![BStr::new("existing")]),
                (BStr::new("stay/sub"), vec![BStr::new("new/sub")]),
                (BStr::new("top"), vec![BStr::new(""), BStr::new("right")]),
            ],
            "file renames vote for removed ancestors with matching suffixes; copies do not vote and destinations need no directory metadata"
        );
        assert!(
            infer(&collected, &changes, 2).is_empty(),
            "directories retained by this side cannot become rename sources"
        );
        Ok(())
    }
}
