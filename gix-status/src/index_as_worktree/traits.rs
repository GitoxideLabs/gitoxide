use std::{io::Read, sync::atomic::AtomicBool};

use bstr::BStr;
use gix_hash::ObjectId;
use gix_index as index;
use index::Entry;

use crate::index_as_worktree::Error;

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
    ) -> Result<Option<Self::Output>, Error>;
}

/// Determine the status of a submodule, which always indicates that it changed if present.
pub trait SubmoduleStatus {
    /// The status result, describing in which way the submodule changed.
    type Output;
    /// A custom error that may occur while computing the submodule status.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Compute the status of the submodule at `entry` and `rela_path`, or return `None` if no change was detected.
    fn status(&mut self, entry: &gix_index::Entry, rela_path: &BStr) -> Result<Option<Self::Output>, Self::Error>;
}

/// Lazy borrowed access to worktree or blob data, with streaming support for worktree files.
pub trait ReadData<'a> {
    /// Returns the contents of this blob.
    ///
    /// This potentially performs IO and other expensive operations
    /// and should only be called when necessary.
    fn read_blob(self) -> Result<&'a [u8], Error>;

    /// Stream a worktree file in such a manner that its content matches what would be put into git.
    fn stream_worktree_file(self) -> Result<read_data::Stream<'a>, Error>;
}

///
pub mod read_data {
    use std::sync::atomic::Ordering;

    use gix_filter::pipeline::convert::ToGitOutcome;

    use crate::AtomicU64;

    /// A stream with worktree file data.
    pub struct Stream<'a> {
        pub(crate) inner: ToGitOutcome<'a, std::fs::File>,
        pub(crate) bytes: Option<&'a AtomicU64>,
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
    ) -> Result<Option<Self::Output>, Error> {
        // make sure to account for racily smudged entries here so that they don't always keep
        // showing up as modified even after their contents have changed again, to a potentially
        // unmodified state. That means that we want to ignore stat.size == 0 for non_empty_blobs.
        if u64::from(entry.stat.size) != worktree_file_size && (entry.id.is_empty_blob() || entry.stat.size != 0) {
            return Ok(Some(()));
        }
        HashEq
            .compare_blobs(entry, worktree_file_size, data, buf)
            .map(|opt| opt.map(|_| ()))
    }
}

/// Compares files to blobs by *always* comparing their hashes.
///
/// Same as [`FastEq`] but does not contain a fast path for files with mismatched files and
/// therefore always returns an OID that can be reused later.
#[derive(Clone)]
pub struct HashEq;

impl CompareBlobs for HashEq {
    type Output = ObjectId;

    fn compare_blobs<'a, 'b>(
        &mut self,
        entry: &Entry,
        _worktree_blob_size: u64,
        data: impl ReadData<'a>,
        buf: &mut Vec<u8>,
    ) -> Result<Option<Self::Output>, Error> {
        let mut stream = data.stream_worktree_file()?;
        match stream.as_bytes() {
            Some(buffer) => {
                let file_hash = gix_object::compute_hash(entry.id.kind(), gix_object::Kind::Blob, buffer)
                    .map_err(gix_hash::io::from_hasher)?;
                Ok((entry.id != file_hash).then_some(file_hash))
            }
            None => {
                let file_hash = match stream.size() {
                    None => {
                        stream.read_to_end(buf).map_err(gix_hash::io::from_std_io)?;
                        gix_object::compute_hash(entry.id.kind(), gix_object::Kind::Blob, buf)
                            .map_err(gix_hash::io::from_hasher)?
                    }
                    Some(len) => gix_object::compute_stream_hash(
                        entry.id.kind(),
                        gix_object::Kind::Blob,
                        &mut stream,
                        len,
                        &mut gix_features::progress::Discard,
                        &AtomicBool::default(),
                    )?,
                };
                Ok((entry.id != file_hash).then_some(file_hash))
            }
        }
    }
}
