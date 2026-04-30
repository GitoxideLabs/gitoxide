use bstr::BString;
use smallvec::SmallVec;

/// The size of the smallest possible extension, which is no more than a signature and a 0 indicating its size.
pub const MIN_SIZE: usize = 4 /* signature */ + 4 /* size */;

/// The kind of index extension.
pub type Signature = [u8; 4];

/// An iterator over the data of index extensions.
pub struct Iter<'a> {
    data: &'a [u8],
    /// The amount of consumed bytes as seen from our internal data pointer. Useful to continue where the iterator left off.
    pub consumed: usize,
}

/// A structure to associate object ids of a tree with sections in the index entries list.
///
/// It allows to more quickly build trees by avoiding as it can quickly reuse portions of the index and its associated tree ids
/// if there was no change to them. Portions of this tree are invalidated as the index is changed.
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Tree {
    /// The name of the tree/directory, or empty if it's the root tree.
    pub name: SmallVec<[u8; 23]>,
    /// The id of the directory tree of the associated tree object.
    pub id: gix_hash::ObjectId,
    /// The amount of non-tree items in this directory tree, including sub-trees, recursively.
    /// The value of the top-level tree is thus equal to the value of the total amount of entries.
    /// If `None`, the tree is considered invalid and needs to be refreshed
    ///
    /// Note that this value can't be trusted, and it may exceed the amount of entries or mismatch.
    pub num_entries: Option<u32>,
    /// The child-trees below the current tree.
    pub children: Vec<Tree>,
}

impl Tree {
    /// Return true if this tree and all child trees are valid and their tree objects exist in `objects`.
    pub fn is_fully_valid(&self, objects: &impl gix_object::Exists) -> bool {
        self.num_entries.is_some()
            && objects.exists(&self.id)
            && self.children.iter().all(|child| child.is_fully_valid(objects))
    }

    /// Invalidate this tree and all child trees.
    pub(crate) fn invalidate_recursively(&mut self) {
        self.num_entries = None;
        for child in &mut self.children {
            child.invalidate_recursively();
        }
    }
}

/// The link extension to track a shared index.
#[derive(Clone)]
pub struct Link {
    /// The checksum of the shared index as last seen.
    pub shared_index_checksum: gix_hash::ObjectId,
    /// Bitmaps to tell us which entries to delete or replace.
    pub bitmaps: Option<link::Bitmaps>,
}

/// The extension for untracked files.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct UntrackedCache {
    /// Something identifying the location and machine that this cache is for.
    /// Should the repository be copied to a different machine, the entire cache can immediately be invalidated.
    identifier: BString,
    /// Stat for the .git/info/exclude file
    info_exclude: Option<untracked_cache::OidStat>,
    /// Stat for the `core.excludesfile`
    excludes_file: Option<untracked_cache::OidStat>,
    /// Usually `.gitignore`
    exclude_filename_per_dir: BString,
    dir_flags: u32,

    /// A list of directories and sub-directories, with `directories[0]` being the root.
    directories: Vec<untracked_cache::Directory>,
}

/// The extension for keeping state on recent information provided by the filesystem monitor.
#[allow(dead_code)]
#[derive(Clone)]
pub struct FsMonitor {
    token: fs_monitor::Token,
    /// if a bit is true, the respective entry is NOT valid as per the fs monitor.
    entry_dirty: gix_bitmap::ewah::Vec,
}

mod iter;

pub(crate) mod fs_monitor;

///
pub mod decode;

///
pub mod tree;

///
pub mod end_of_index_entry;

pub(crate) mod index_entry_offset_table;

///
pub mod link;

pub(crate) mod resolve_undo;

///
pub mod untracked_cache;

///
pub mod sparse;
