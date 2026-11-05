use gix_error::Result;

use bstr::BStr;
use gix_hash::ObjectId;
use gix_index as index;
use index::Entry;

/// Compares the content of two blobs in some way.
pub trait CompareBlobs {
    /// Output data produced by [`compare_blobs()`][CompareBlobs::compare_blobs()].
    type Output;

    /// Providing the underlying index `entry`, allow comparing a file in the worktree of size `worktree_blob_size`
    /// and allow streaming its bytes using `data`.
    /// If this function returns `None` the `entry` and the worktree blob are assumed to be identical.
    /// Use `data` to obtain the data for the blob referred to by `entry`, allowing comparisons of the data itself.
    /// `buf` can be used to store additional data, and it can be assumed to be a cleared buffer.
    fn compare_blobs<'a, 'b>(
        &mut self,
        entry: &gix_index::Entry,
        worktree_blob_size: u64,
        data: impl ReadData<'a>,
        buf: &mut Vec<u8>,
    ) -> Result<Option<Self::Output>>;
}

/// Determine the status of a submodule, which always indicates that it changed if present.
pub trait SubmoduleStatus {
    /// The status result, describing in which way the submodule changed.
    type Output;

    /// Compute the status of the submodule at `entry` and `rela_path`, or return `None` if no change was detected.
    fn status(&mut self, entry: &gix_index::Entry, rela_path: &BStr) -> Result<Option<Self::Output>>;
}

/// Lazy borrowed access to worktree or blob data, with streaming support for worktree files.
pub trait ReadData<'a> {
    /// Returns the contents of this blob.
    ///
    /// This potentially performs IO and other expensive operations
    /// and should only be called when necessary.
    fn read_blob(self) -> Result<&'a [u8]>;

    /// Stream a worktree file in such a manner that its content matches what would be put into git.
    fn stream_worktree_file(self) -> Result<read_data::Stream<'a>>;
}

///
pub mod read_data {
    use std::{
        io::Read,
        sync::atomic::{AtomicBool, Ordering},
    };

    use gix_error::Result;
    use gix_filter::pipeline::convert::ToGitOutcome;
    use gix_hash::ObjectId;

    use crate::AtomicU64;

    /// A stream with worktree file data.
    pub struct Stream<'a> {
        pub(crate) inner: ToGitOutcome<'a, std::fs::File>,
        /// Counter of consumed worktree bytes.
        pub(crate) bytes: Option<&'a AtomicU64>,
        /// Expected length of an unconverted file, from metadata that may already be stale.
        /// `None` for filtered streams or buffered content.
        pub(crate) len: Option<u64>,
    }

    impl<'a> Stream<'a> {
        /// Return the underlying byte-buffer if there is one.
        ///
        /// If `None`, read from this instance like a stream.
        /// Note that this method should only be called once to assure proper accounting of the amount of bytes read.
        pub fn as_bytes(&self) -> Option<&'a [u8]> {
            self.inner.as_bytes().inspect(|v| {
                if let Some(bytes) = self.bytes {
                    bytes.fetch_add(v.len() as u64, Ordering::Relaxed);
                }
            })
        }

        /// Return the size of the stream in bytes if it is known in advance.
        pub fn size(&self) -> Option<u64> {
            self.len
        }

        /// Hash this stream as a blob, or return `None` if it yielded fewer bytes than its size announced,
        /// which means the file shrank while it was read.
        pub(crate) fn hash(&mut self, kind: gix_hash::Kind, buf: &mut Vec<u8>) -> Result<Option<ObjectId>> {
            fn is_premature_eof(err: &gix_error::Error) -> bool {
                err.downcast_any_ref::<std::io::Error>()
                    .is_some_and(|err| err.kind() == std::io::ErrorKind::UnexpectedEof)
            }
            if let Some(buffer) = self.as_bytes() {
                return gix_object::compute_hash(kind, gix_object::Kind::Blob, buffer).map(Some);
            }
            match self.size() {
                None => {
                    self.read_to_end(buf).map_err(gix_hash::io::from_std_io)?;
                    gix_object::compute_hash(kind, gix_object::Kind::Blob, buf).map(Some)
                }
                Some(len) => match gix_object::compute_stream_hash(
                    kind,
                    gix_object::Kind::Blob,
                    self,
                    len,
                    &mut gix_utils::progress::Discard,
                    &AtomicBool::default(),
                ) {
                    Err(err) if is_premature_eof(&err) => Ok(None),
                    file_hash => file_hash.map(Some),
                },
            }
        }
    }

    impl std::io::Read for Stream<'_> {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let n = self.inner.read(buf)?;
            if let Some(bytes) = self.bytes {
                bytes.fetch_add(n as u64, Ordering::Relaxed);
            }
            Ok(n)
        }
    }
}

/// Compares to blobs by comparing their size and oid, and only looks at the file if
/// the size matches, therefore it's very fast.
#[derive(Clone)]
pub struct FastEq;

impl CompareBlobs for FastEq {
    type Output = ();

    // TODO: make all streaming IOPs interruptible.
    fn compare_blobs<'a, 'b>(
        &mut self,
        entry: &Entry,
        worktree_file_size: u64,
        data: impl ReadData<'a>,
        buf: &mut Vec<u8>,
    ) -> Result<Option<Self::Output>> {
        // make sure to account for racily smudged entries here so that they don't always keep
        // showing up as modified even after their contents have changed again, to a potentially
        // unmodified state. That means that we want to ignore stat.size == 0 for non_empty_blobs.
        if u64::from(entry.stat.size) != worktree_file_size && (entry.id.is_empty_blob() || entry.stat.size != 0) {
            return Ok(Some(()));
        }
        let mut stream = data.stream_worktree_file()?;
        Ok(match stream.hash(entry.id.kind(), buf)? {
            Some(file_hash) => (entry.id != file_hash).then_some(()),
            None => Some(()),
        })
    }
}

/// Compares files to blobs by hashing their contents, without the size-mismatch shortcut of [`FastEq`].
///
/// The result of [`CompareBlobs::compare_blobs()`] distinguishes:
/// * `None`: the content matches the index.
/// * `Some(Some(id))`: the content changed, and its blob hash is available for reuse.
/// * `Some(None)`: the file shrank while being read, so it changed but its blob hash is unavailable.
///
/// Like Git, a short read is reported as a modification without retrying, even if the remaining content
/// matches the index, as the original metadata is stale.
///
/// Like [`FastEq`] and Git, growth of unconverted files is not detected, as checking their size again would
/// require additional I/O. Appends beyond the announced size while streaming can therefore result in a
/// false-equal comparison.
#[derive(Clone)]
pub struct HashEq;

impl CompareBlobs for HashEq {
    type Output = Option<ObjectId>;

    fn compare_blobs<'a, 'b>(
        &mut self,
        entry: &Entry,
        _worktree_blob_size: u64,
        data: impl ReadData<'a>,
        buf: &mut Vec<u8>,
    ) -> Result<Option<Self::Output>> {
        let mut stream = data.stream_worktree_file()?;
        Ok(match stream.hash(entry.id.kind(), buf)? {
            Some(file_hash) => (entry.id != file_hash).then_some(Some(file_hash)),
            None => Some(None),
        })
    }
}
