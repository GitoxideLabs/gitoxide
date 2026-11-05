use gix_error::{Result, bail, message};
use std::{io::Read, sync::atomic::AtomicBool};

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

        /// Prepare to read an unconverted worktree file again from its start, with the size it has now.
        pub(crate) fn restart_with_current_size(&mut self) -> std::io::Result<()> {
            let ToGitOutcome::Unchanged(file) = &mut self.inner else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "only an unconverted worktree file can be read again",
                ));
            };
            self.len = Some(file.metadata()?.len());
            std::io::Seek::rewind(file)
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
        // Like Git, a file that shrank while it was read changed, so there is no need to read it again.
        Ok(match hash_worktree_stream(entry.id.kind(), &mut stream, buf)? {
            Some(file_hash) => (entry.id != file_hash).then_some(()),
            None => Some(()),
        })
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
    ) -> Result<Option<Self::Output>> {
        let mut stream = data.stream_worktree_file()?;
        let mut attempts_left = HASH_ATTEMPTS_OF_SHRINKING_FILE;
        loop {
            if let Some(file_hash) = hash_worktree_stream(entry.id.kind(), &mut stream, buf)? {
                return Ok((entry.id != file_hash).then_some(file_hash));
            }
            attempts_left -= 1;
            if attempts_left == 0 {
                bail!(message("The worktree file kept shrinking while it was hashed"));
            }
            // An object id has to describe actual content, so stream the file again with the size it has now.
            stream.restart_with_current_size().map_err(gix_hash::io::from_std_io)?;
        }
    }
}

/// How often [`HashEq`] hashes a file that keeps shrinking while it is read before giving up.
const HASH_ATTEMPTS_OF_SHRINKING_FILE: usize = 3;

/// Hash `stream` as a blob, or return `None` if it yielded fewer bytes than its size announced,
/// which means the file shrank while it was read.
fn hash_worktree_stream(
    kind: gix_hash::Kind,
    stream: &mut read_data::Stream<'_>,
    buf: &mut Vec<u8>,
) -> Result<Option<ObjectId>> {
    if let Some(buffer) = stream.as_bytes() {
        return gix_object::compute_hash(kind, gix_object::Kind::Blob, buffer).map(Some);
    }
    match stream.size() {
        None => {
            stream.read_to_end(buf).map_err(gix_hash::io::from_std_io)?;
            gix_object::compute_hash(kind, gix_object::Kind::Blob, buf).map(Some)
        }
        Some(len) => match gix_object::compute_stream_hash(
            kind,
            gix_object::Kind::Blob,
            stream,
            len,
            &mut gix_utils::progress::Discard,
            &AtomicBool::default(),
        ) {
            Err(err) if is_premature_eof(&err) => Ok(None),
            file_hash => file_hash.map(Some),
        },
    }
}

fn is_premature_eof(err: &gix_error::Error) -> bool {
    err.downcast_any_ref::<std::io::Error>()
        .is_some_and(|err| err.kind() == std::io::ErrorKind::UnexpectedEof)
}
