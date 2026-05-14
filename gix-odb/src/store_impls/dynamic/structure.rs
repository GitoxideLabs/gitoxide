use std::path::PathBuf;

use crate::{Store, store::load_index, types::IndexAndPacks};

/// A record of a structural element of an object database.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Record {
    /// A loose object database.
    LooseObjectDatabase {
        /// The root of the object database.
        objects_directory: PathBuf,
        /// The amount of object files.
        num_objects: usize,
    },
    /// A pack index file
    Index {
        /// The location of the index file,
        path: PathBuf,
        /// Whether or not the index is mapped into memory.
        state: IndexState,
    },
    /// A multi-index file
    MultiIndex {
        /// The location of the multi-index file,
        path: PathBuf,
        /// Whether or not the index is mapped into memory.
        state: IndexState,
    },
    /// An empty slot was encountered, this is possibly happening as the ODB changes during query with
    /// a file being removed.
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
/// Possible stats of pack indices.
pub enum IndexState {
    /// The index is active in memory because a mapping exists.
    Loaded,
    /// The index couldn't be unloaded as it was still in use, but that can happen another time.
    Disposable,
    /// The index isn't loaded/memory mapped.
    Unloaded,
}

impl Store {
    /// Return information about all files known to us as well as their loading state.
    ///
    /// Note that this call is expensive as it gathers additional information about loose object databases.
    /// Note that it may change as we collect information due to the highly volatile nature of the
    /// implementation. The likelihood of actual changes is low though as these still depend on something
    /// changing on disk and somebody reading at the same time.
    pub fn structure(&self) -> Result<Vec<Record>, load_index::Error> {
        let _span = gix_features::trace::detail!("gix_odb::Store::structure()");
        let index = self.index.load();
        if !index.is_initialized() {
            self.consolidate_with_disk_state(true, false /*load one new index*/)?;
        }
        let index = self.index.load();
        let mut res: Vec<_> = index
            .loose_dbs
            .iter()
            .map(|db| Record::LooseObjectDatabase {
                objects_directory: db.path.clone(),
                num_objects: db.iter().count(),
            })
            .collect();

        for slot in index.slot_indices.iter().map(|idx| &self.files[*idx]) {
            let files = slot.files.load();
            let record = match &**files {
                Some(index) => {
                    let state = if index.is_disposable() {
                        IndexState::Disposable
                    } else if index.index_is_loaded() {
                        IndexState::Loaded
                    } else {
                        IndexState::Unloaded
                    };
                    match index {
                        IndexAndPacks::Index(b) => Record::Index {
                            path: b.index.path().into(),
                            state,
                        },
                        IndexAndPacks::MultiIndex(b) => Record::MultiIndex {
                            path: b.multi_index.path().into(),
                            state,
                        },
                    }
                }
                None => Record::Empty,
            };
            res.push(record);
        }
        Ok(res)
    }

    /// Provide a list of all `objects` directories of `alternate` object database paths.
    /// This list might be empty if there are no alternates.
    ///
    /// Read more about alternates in the documentation of the [`resolve`][crate::alternate::resolve()] function.
    pub fn alternate_db_paths(&self) -> Result<Vec<PathBuf>, load_index::Error> {
        let index = self.index.load();
        if !index.is_initialized() {
            self.consolidate_with_disk_state(true, false /*load one new index*/)?;
        }
        let index = self.index.load();
        Ok(index
            .loose_dbs
            .iter()
            .skip(
                1, /* first odb is always the primary one, all the follows is alternates */
            )
            .map(|db| db.path.clone())
            .collect())
    }
}
