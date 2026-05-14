//! The implementation of creating an archive from a git tree, similar to `git archive`, but using an internal format.
//!
//! This crate can effectively be used to manipulate worktrees as streams of bytes, which can be decoded using the [`Stream`] type.
#![deny(missing_docs, unsafe_code)]

use std::{path::Path, sync::Arc};

use gix_object::bstr::BString;

/// A stream of entries that originate from a git tree and optionally from additional entries.
///
/// Note that a git tree is mandatory, but the empty tree can be used to effectively disable it.
pub struct Stream {
    read: utils::Read,
    err: SharedErrorSlot,
    extra_entries: Option<std::sync::mpsc::Sender<AdditionalEntry>>,
    // additional_entries: Vec,
    /// `None` if currently held by an entry.
    path_buf: Option<BString>,
    /// Another buffer to partially act like a buf-reader.
    buf: Vec<u8>,
    /// The offset into `buf` for entries being able to act like a buf reader.
    pos: usize,
    /// The amount of bytes usable from `buf` (even though it always has a fixed size)
    filled: usize,
}

///
pub mod entry;
pub(crate) mod protocol;

mod from_tree;
pub use from_tree::from_tree;

pub(crate) type SharedErrorSlot = Arc<parking_lot::Mutex<Option<entry::Error>>>;

/// An entry in a stream. Note that they must be consumed fully, by reading from them till exhaustion.
///
/// ### Drop behaviour
///
/// If the entry is dropped without reading it till exhaustion, the stream is tainted and
/// [`next_entry()`][Stream::next_entry()] will panic next time it is called.
pub struct Entry<'a> {
    /// The kind of entry at [`relative_path`][Self::relative_path()].
    pub mode: gix_object::tree::EntryMode,
    /// The hash of the object, uniquely identifying it.
    pub id: gix_hash::ObjectId,
    /// Access to our parent
    parent: &'a mut Stream,
    /// The path relative to the repository at which data should be written.
    path_buf: Option<BString>,
    /// The amount of bytes left to read if the size of bytes to read is known.
    /// It's also our marker to say that we are depleted, which is important to signal to the
    /// parent stream that we can proceed reading the next entry.
    remaining: Option<usize>,
}

/// An entry that is [added to the stream][Stream::add_entry()] by the user, verbatim, without additional worktree conversions.
///
/// It may overwrite previously written paths, which may or may not work for the consumer of the stream.
pub struct AdditionalEntry {
    /// The hash of the object, uniquely identifying it.
    /// Note that it can be [`null()`][gix_hash::ObjectId::null()] as the hash is typically ignored by consumers of the stream.
    pub id: gix_hash::ObjectId,
    /// The kind of entry to create.
    pub mode: gix_object::tree::EntryMode,
    /// The path relative to the repository at which content should be located.
    pub relative_path: BString,
    /// Where to get the content of the entry from.
    pub source: entry::Source,
}

/// Lifecycle
impl Stream {
    /// Turn ourselves into the underlying byte stream which is a representation of the underlying git tree.
    ///
    /// Note that the format is unspecified, and its sole use is for transport, not for persistence.
    /// Can be used with [`Self::from_read()`] to decode the contained entries.
    pub fn into_read(self) -> impl std::io::Read {
        self.read
    }

    /// Return our internal byte stream from which entries would be generated.
    ///
    /// Note that the stream must then be consumed in its entirety.
    pub fn as_read_mut(&mut self) -> &mut impl std::io::Read {
        self.extra_entries.take();
        &mut self.read
    }

    /// Create a new instance from a stream of bytes in our format.
    ///
    /// It must have been created from [`Self::into_read()`] to be compatible, and must
    /// not have been persisted.
    pub fn from_read(read: impl std::io::Read + 'static) -> Self {
        Self {
            read: utils::Read::Unknown(Box::new(read)),
            extra_entries: None,
            path_buf: Some(Vec::with_capacity(1024).into()),
            err: Default::default(),
            buf: std::iter::repeat_n(0, u16::MAX as usize).collect(),
            pos: 0,
            filled: 0,
        }
    }
}

/// Entries
impl Stream {
    /// Add `entry` to the list of entries to be returned in calls to [`Self::next_entry()`].
    ///
    /// The entry will be returned after the one contained in the tree, in order of addition.
    /// # Panics
    /// If called after the first call to [`Self::next_entry()`].
    pub fn add_entry(&mut self, entry: AdditionalEntry) -> &mut Self {
        self.extra_entries
            .as_ref()
            .expect("BUG: must not add entries after the start of entries traversal")
            .send(entry)
            .expect("Failure is impossible as thread blocks on the receiving end");
        self
    }

    /// Add the item at `path` as entry to this stream, which is expected to be under `root`.
    ///
    /// Note that the created entries will always have a null hash, and that we access this path
    /// to determine its type, and will access it again when it is requested.
    pub fn add_entry_from_path(
        &mut self,
        root: &Path,
        path: &Path,
        object_hash: gix_hash::Kind,
    ) -> std::io::Result<&mut Self> {
        let rela_path = path.strip_prefix(root).map_err(std::io::Error::other)?;
        let meta = path.symlink_metadata()?;
        let relative_path = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(rela_path)).into_owned();
        let id = object_hash.null();

        let entry = if meta.is_symlink() {
            let content = std::fs::read_link(path)?;
            let content = gix_path::into_bstr(content).into_owned();
            AdditionalEntry {
                id,
                mode: gix_object::tree::EntryKind::Link.into(),
                relative_path,
                source: entry::Source::Memory(content.into()),
            }
        } else if meta.is_dir() {
            AdditionalEntry {
                id,
                mode: gix_object::tree::EntryKind::Tree.into(),
                relative_path,
                source: entry::Source::Null,
            }
        } else {
            let mode = if gix_fs::is_executable(&meta) {
                gix_object::tree::EntryKind::BlobExecutable
            } else {
                gix_object::tree::EntryKind::Blob
            }
            .into();
            AdditionalEntry {
                id,
                mode,
                relative_path,
                source: entry::Source::Path(path.to_owned()),
            }
        };
        Ok(self.add_entry(entry))
    }
}

impl Stream {
    pub(crate) fn new() -> (
        Stream,
        gix_features::io::pipe::Writer,
        std::sync::mpsc::Receiver<AdditionalEntry>,
    ) {
        // 1 write for entry header and 1 for hash, 1 for entry path, + 1 for a buffer, then 32 of these.
        // Giving some buffer, at the expense of memory, is important to allow consumers to take off bytes more quickly,
        // otherwise, both threads effectively run in lock-step and nullify the benefit.
        let in_flight_writes = (2 + 1) * 32;
        let (write, read) = gix_features::io::pipe::unidirectional(in_flight_writes);
        let (tx_entries, rx_entries) = std::sync::mpsc::channel();
        (
            Stream {
                read: utils::Read::Known(read),
                extra_entries: Some(tx_entries),
                path_buf: Some(Vec::with_capacity(1024).into()),
                err: Default::default(),
                buf: std::iter::repeat_n(0, u16::MAX as usize).collect(),
                pos: 0,
                filled: 0,
            },
            write,
            rx_entries,
        )
    }
}

pub(crate) mod utils {
    pub enum Read {
        Known(gix_features::io::pipe::Reader),
        Unknown(Box<dyn std::io::Read>),
    }

    impl std::io::Read for Read {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            match self {
                Read::Known(r) => r.read(buf),
                Read::Unknown(r) => r.read(buf),
            }
        }
    }
}
