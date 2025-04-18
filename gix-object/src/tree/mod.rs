use crate::{
    bstr::{BStr, BString},
    tree, Tree, TreeRef,
};
use std::cell::RefCell;
use std::cmp::Ordering;

///
pub mod editor;

mod ref_iter;
///
pub mod write;

/// The state needed to apply edits instantly to in-memory trees.
///
/// It's made so that each tree is looked at in the object database at most once, and held in memory for
/// all edits until everything is flushed to write all changed trees.
///
/// The editor is optimized to edit existing trees, but can deal with building entirely new trees as well
/// with some penalties.
#[doc(alias = "TreeUpdateBuilder", alias = "git2")]
#[derive(Clone)]
pub struct Editor<'a> {
    /// A way to lookup trees.
    find: &'a dyn crate::FindExt,
    /// The kind of hashes to produce>
    object_hash: gix_hash::Kind,
    /// All trees we currently hold in memory. Each of these may change while adding and removing entries.
    /// null-object-ids mark tree-entries whose value we don't know yet, they are placeholders that will be
    /// dropped when writing at the latest.
    trees: std::collections::HashMap<BString, Tree>,
    /// A buffer to build up paths when finding the tree to edit.
    path_buf: RefCell<BString>,
    /// Our buffer for storing tree-data in, right before decoding it.
    tree_buf: Vec<u8>,
}

/// The mode of items storable in a tree, similar to the file mode on a unix file system.
///
/// Used in [`mutable::Entry`][crate::tree::Entry] and [`EntryRef`].
///
/// Note that even though it can be created from any `u16`, it should be preferable to
/// create it by converting [`EntryKind`] into `EntryMode`.
#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntryMode {
    // Represents the value read from Git, except that "040000" is represented with 0o140000 but
    // "40000" is represented with 0o40000.
    internal: u16,
}

impl TryFrom<u32> for tree::EntryMode {
    type Error = u32;
    fn try_from(mode: u32) -> Result<Self, Self::Error> {
        Ok(match mode {
            0o40000 | 0o120000 | 0o160000 => EntryMode { internal: mode as u16 },
            blob_mode if blob_mode & 0o100000 == 0o100000 => EntryMode { internal: mode as u16 },
            _ => return Err(mode),
        })
    }
}

impl EntryMode {
    /// Expose the value as u16 (lossy, unlike the internal representation that is hidden).
    pub const fn value(self) -> u16 {
        // Demangle the hack: In the case where the second leftmost octet is 4 (Tree), the leftmost bit is
        // there to represent whether the bytes representation should have 5 or 6 octets.
        if self.internal & IFMT == 0o140000 {
            0o040000
        } else {
            self.internal
        }
    }

    /// Return the representation as used in the git internal format, which is octal and written
    /// to the `backing` buffer. The respective sub-slice that was written to is returned.
    pub fn as_bytes<'a>(&self, backing: &'a mut [u8; 6]) -> &'a BStr {
        if self.internal == 0 {
            std::slice::from_ref(&b'0')
        } else {
            for (idx, backing_octet) in backing.iter_mut().enumerate() {
                let bit_pos = 3 /* because base 8 and 2^3 == 8*/ * (6 - idx - 1);
                let oct_mask = 0b111 << bit_pos;
                let digit = (self.internal & oct_mask) >> bit_pos;
                *backing_octet = b'0' + digit as u8;
            }
            // Hack: `0o140000` represents `"040000"`, `0o40000` represents `"40000"`.
            if backing[1] == b'4' {
                if backing[0] == b'1' {
                    backing[0] = b'0';
                    &backing[0..6]
                } else {
                    &backing[1..6]
                }
            } else {
                &backing[0..6]
            }
        }
        .into()
    }

    /// Construct an EntryMode from bytes represented as in the git internal format
    /// Return the mode and the remainder of the bytes.
    pub(crate) fn extract_from_bytes(i: &[u8]) -> Option<(Self, &'_ [u8])> {
        let mut mode = 0;
        let mut idx = 0;
        let mut space_pos = 0;
        if i.is_empty() {
            return None;
        }
        // const fn, this is why we can't have nice things (like `.iter().any()`).
        while idx < i.len() {
            let b = i[idx];
            // Delimiter, return what we got
            if b == b' ' {
                space_pos = idx;
                break;
            }
            // Not a pure octal input.
            // Performance matters here, so `!(b'0'..=b'7').contains(&b)` won't do.
            #[allow(clippy::manual_range_contains)]
            if b < b'0' || b > b'7' {
                return None;
            }
            // More than 6 octal digits we must have hit the delimiter or the input was malformed.
            if idx > 6 {
                return None;
            }
            mode = (mode << 3) + (b - b'0') as u16;
            idx += 1;
        }
        // Hack: `0o140000` represents `"040000"`, `0o40000` represents `"40000"`.
        if mode == 0o040000 && i[0] == b'0' {
            mode += 0o100000;
        }
        Some((Self { internal: mode }, &i[(space_pos + 1)..]))
    }

    /// Construct an EntryMode from bytes represented as in the git internal format.
    pub fn from_bytes(i: &[u8]) -> Option<Self> {
        Self::extract_from_bytes(i).map(|(mode, _rest)| mode)
    }
}

impl std::fmt::Debug for EntryMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EntryMode(0o{})", self.as_bytes(&mut Default::default()))
    }
}

impl std::fmt::Octal for EntryMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_bytes(&mut Default::default()))
    }
}

/// A discretized version of ideal and valid values for entry modes.
///
/// Note that even though it can represent every valid [mode](EntryMode), it might
/// lose information due to that as well.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Ord, PartialOrd, Hash)]
#[repr(u16)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EntryKind {
    /// A tree, or directory
    Tree = 0o040000u16,
    /// A file that is not executable
    Blob = 0o100644,
    /// A file that is executable
    BlobExecutable = 0o100755,
    /// A symbolic link
    Link = 0o120000,
    /// A commit of a git submodule
    Commit = 0o160000,
}

impl From<EntryKind> for EntryMode {
    fn from(value: EntryKind) -> Self {
        EntryMode { internal: value as u16 }
    }
}

impl From<EntryMode> for EntryKind {
    fn from(value: EntryMode) -> Self {
        value.kind()
    }
}

/// Serialization
impl EntryKind {
    /// Return the representation as used in the git internal format.
    pub fn as_octal_str(&self) -> &'static BStr {
        use EntryKind::*;
        let bytes: &[u8] = match self {
            Tree => b"40000",
            Blob => b"100644",
            BlobExecutable => b"100755",
            Link => b"120000",
            Commit => b"160000",
        };
        bytes.into()
    }
}

const IFMT: u16 = 0o170000;

impl EntryMode {
    /// Discretize the raw mode into an enum with well-known state while dropping unnecessary details.
    pub const fn kind(&self) -> EntryKind {
        let etype = self.value() & IFMT;
        if etype == 0o100000 {
            if self.value() & 0o000100 == 0o000100 {
                EntryKind::BlobExecutable
            } else {
                EntryKind::Blob
            }
        } else if etype == EntryKind::Link as u16 {
            EntryKind::Link
        } else if etype == EntryKind::Tree as u16 {
            EntryKind::Tree
        } else {
            EntryKind::Commit
        }
    }

    /// Return true if this entry mode represents a Tree/directory
    pub const fn is_tree(&self) -> bool {
        self.value() & IFMT == EntryKind::Tree as u16
    }

    /// Return true if this entry mode represents the commit of a submodule.
    pub const fn is_commit(&self) -> bool {
        self.value() & IFMT == EntryKind::Commit as u16
    }

    /// Return true if this entry mode represents a symbolic link
    pub const fn is_link(&self) -> bool {
        self.value() & IFMT == EntryKind::Link as u16
    }

    /// Return true if this entry mode represents anything BUT Tree/directory
    pub const fn is_no_tree(&self) -> bool {
        self.value() & IFMT != EntryKind::Tree as u16
    }

    /// Return true if the entry is any kind of blob.
    pub const fn is_blob(&self) -> bool {
        self.value() & IFMT == 0o100000
    }

    /// Return true if the entry is an executable blob.
    pub const fn is_executable(&self) -> bool {
        matches!(self.kind(), EntryKind::BlobExecutable)
    }

    /// Return true if the entry is any kind of blob or symlink.
    pub const fn is_blob_or_symlink(&self) -> bool {
        matches!(
            self.kind(),
            EntryKind::Blob | EntryKind::BlobExecutable | EntryKind::Link
        )
    }

    /// Represent the mode as descriptive string.
    pub const fn as_str(&self) -> &'static str {
        use EntryKind::*;
        match self.kind() {
            Tree => "tree",
            Blob => "blob",
            BlobExecutable => "exe",
            Link => "link",
            Commit => "commit",
        }
    }
}

impl TreeRef<'_> {
    /// Convert this instance into its own version, creating a copy of all data.
    ///
    /// This will temporarily allocate an extra copy in memory, so at worst three copies of the tree exist
    /// at some intermediate point in time. Use [`Self::into_owned()`] to avoid this.
    pub fn to_owned(&self) -> Tree {
        self.clone().into()
    }

    /// Convert this instance into its own version, creating a copy of all data.
    pub fn into_owned(self) -> Tree {
        self.into()
    }
}

/// An element of a [`TreeRef`][crate::TreeRef::entries].
#[derive(PartialEq, Eq, Debug, Hash, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EntryRef<'a> {
    /// The kind of object to which `oid` is pointing.
    pub mode: tree::EntryMode,
    /// The name of the file in the parent tree.
    pub filename: &'a BStr,
    /// The id of the object representing the entry.
    // TODO: figure out how these should be called. id or oid? It's inconsistent around the codebase.
    //       Answer: make it 'id', as in `git2`
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub oid: &'a gix_hash::oid,
}

impl PartialOrd for EntryRef<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for EntryRef<'_> {
    fn cmp(&self, b: &Self) -> Ordering {
        let a = self;
        let common = a.filename.len().min(b.filename.len());
        a.filename[..common].cmp(&b.filename[..common]).then_with(|| {
            let a = a.filename.get(common).or_else(|| a.mode.is_tree().then_some(&b'/'));
            let b = b.filename.get(common).or_else(|| b.mode.is_tree().then_some(&b'/'));
            a.cmp(&b)
        })
    }
}

/// An entry in a [`Tree`], similar to an entry in a directory.
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Entry {
    /// The kind of object to which `oid` is pointing to.
    pub mode: EntryMode,
    /// The name of the file in the parent tree.
    pub filename: BString,
    /// The id of the object representing the entry.
    pub oid: gix_hash::ObjectId,
}

impl PartialOrd for Entry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Entry {
    fn cmp(&self, b: &Self) -> Ordering {
        let a = self;
        let common = a.filename.len().min(b.filename.len());
        a.filename[..common].cmp(&b.filename[..common]).then_with(|| {
            let a = a.filename.get(common).or_else(|| a.mode.is_tree().then_some(&b'/'));
            let b = b.filename.get(common).or_else(|| b.mode.is_tree().then_some(&b'/'));
            a.cmp(&b)
        })
    }
}
