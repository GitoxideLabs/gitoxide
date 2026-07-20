use std::cmp::Ordering;

use bstr::{BString, ByteSlice};
use gix_object::FindExt;

use crate::extension::Tree;

/// The error returned by [`Tree::verify()`][crate::extension::Tree::verify()].
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    MissingTreeDirectory {
        parent_id: gix_hash::ObjectId,
        entry_id: gix_hash::ObjectId,
        name: BString,
    },
    TreeNodeNotFound(gix_object::find::existing_iter::Error),
    TreeNodeChildcountMismatch {
        oid: gix_hash::ObjectId,
        expected_childcount: usize,
        actual_childcount: usize,
    },
    RootWithName {
        name: BString,
    },
    EntriesCount {
        actual: u32,
        expected: u32,
    },
    EntriesCountExceedsIndex {
        name: BString,
        actual: u32,
        expected: usize,
    },
    OutOfOrder {
        parent_id: gix_hash::ObjectId,
        current_path: BString,
        previous_path: BString,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MissingTreeDirectory {
                parent_id,
                entry_id,
                name,
            } => write!(
                f,
                "The entry {entry_id} at path '{name}' in parent tree {parent_id} wasn't found in the nodes children, making it incomplete"
            ),
            Error::TreeNodeNotFound(err) => std::fmt::Display::fmt(err, f),
            Error::TreeNodeChildcountMismatch {
                oid,
                expected_childcount,
                actual_childcount,
            } => write!(
                f,
                "The tree with id {oid} should have {expected_childcount} children, but its cached representation had {actual_childcount} of them"
            ),
            Error::RootWithName { name } => {
                write!(f, "The root tree was named '{name}', even though it should be empty")
            }
            Error::EntriesCount { actual, expected } => write!(
                f,
                "Expected not more than {expected} entries to be reachable from the top-level, but actual count was {actual}"
            ),
            Error::EntriesCountExceedsIndex { name, actual, expected } => write!(
                f,
                "TREE entry '{name}' declared {actual} entries, but the index only contains {expected} entries"
            ),
            Error::OutOfOrder {
                parent_id,
                current_path,
                previous_path,
            } => write!(
                f,
                "Parent tree '{parent_id}' contained out-of order trees prev = '{previous_path}' and next = '{current_path}'"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::TreeNodeNotFound(err) => err.source(),
            Error::MissingTreeDirectory { .. }
            | Error::TreeNodeChildcountMismatch { .. }
            | Error::RootWithName { .. }
            | Error::EntriesCount { .. }
            | Error::EntriesCountExceedsIndex { .. }
            | Error::OutOfOrder { .. } => None,
        }
    }
}

impl From<gix_object::find::existing_iter::Error> for Error {
    fn from(err: gix_object::find::existing_iter::Error) -> Self {
        Error::TreeNodeNotFound(err)
    }
}

impl Tree {
    /// Validate the correctness of this instance. If `use_objects` is true, then `objects` will be used to access all objects.
    pub fn verify(&self, use_objects: bool, objects: impl gix_object::Find) -> Result<(), Error> {
        fn verify_recursive(
            parent_id: gix_hash::ObjectId,
            children: &[Tree],
            mut object_buf: Option<&mut Vec<u8>>,
            objects: &impl gix_object::Find,
        ) -> Result<Option<u32>, Error> {
            if children.is_empty() {
                return Ok(None);
            }
            let mut entries = 0;
            let mut prev = None::<&Tree>;
            for child in children {
                entries += child.num_entries.unwrap_or(0);
                if let Some(prev) = prev {
                    if prev.name.cmp(&child.name) != Ordering::Less {
                        return Err(Error::OutOfOrder {
                            parent_id,
                            previous_path: prev.name.as_bstr().into(),
                            current_path: child.name.as_bstr().into(),
                        });
                    }
                }
                prev = Some(child);
            }
            if let Some(buf) = object_buf.as_mut() {
                let tree_entries = objects.find_tree_iter(&parent_id, buf)?;
                let mut num_entries = 0;
                for entry in tree_entries.filter_map(Result::ok).filter(|e| e.mode.is_tree()) {
                    children
                        .binary_search_by(|e| e.name.as_bstr().cmp(entry.filename))
                        .map_err(|_| Error::MissingTreeDirectory {
                            parent_id,
                            entry_id: entry.oid.to_owned(),
                            name: entry.filename.to_owned(),
                        })?;
                    num_entries += 1;
                }

                if num_entries != children.len() {
                    return Err(Error::TreeNodeChildcountMismatch {
                        oid: parent_id,
                        expected_childcount: num_entries,
                        actual_childcount: children.len(),
                    });
                }
            }
            for child in children {
                // This is actually needed here as it's a mut ref, which isn't copy. We do a re-borrow here.
                #[allow(clippy::needless_option_as_deref)]
                let actual_num_entries =
                    verify_recursive(child.id, &child.children, object_buf.as_deref_mut(), objects)?;
                if let Some((actual, num_entries)) = actual_num_entries.zip(child.num_entries) {
                    if actual > num_entries {
                        return Err(Error::EntriesCount {
                            actual,
                            expected: num_entries,
                        });
                    }
                }
            }
            Ok(entries.into())
        }
        let _span = gix_features::trace::coarse!("gix_index::extension::Tree::verify()");

        if !self.name.is_empty() {
            return Err(Error::RootWithName {
                name: self.name.as_bstr().into(),
            });
        }

        let mut buf = Vec::new();
        let declared_entries = verify_recursive(self.id, &self.children, use_objects.then_some(&mut buf), &objects)?;
        if let Some((actual, num_entries)) = declared_entries.zip(self.num_entries) {
            if actual > num_entries {
                return Err(Error::EntriesCount {
                    actual,
                    expected: num_entries,
                });
            }
        }

        Ok(())
    }

    /// Reject impossible cached entry counts using the total number of index entries as an upper bound.
    ///
    /// This is a cheap heuristic: it doesn't prove each cached subtree count matches its actual path range,
    /// but no TREE node can describe more entries than the entire index contains.
    pub(crate) fn verify_entries_count(&self, num_index_entries: usize) -> Result<(), Error> {
        if let Some(actual) = self.num_entries {
            if actual as usize > num_index_entries {
                return Err(Error::EntriesCountExceedsIndex {
                    name: self.name.as_bstr().into(),
                    actual,
                    expected: num_index_entries,
                });
            }
        }

        for child in &self.children {
            child.verify_entries_count(num_index_entries)?;
        }

        Ok(())
    }
}
