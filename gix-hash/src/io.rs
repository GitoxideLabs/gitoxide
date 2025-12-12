use crate::hasher;

/// The error type for I/O operations that compute hashes.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("Failed to hash data")]
    Hasher(#[from] hasher::Error),
}

pub(super) mod _impl {
    use crate::{hasher, io::Error, Hasher};

    /// Compute the hash of `kind` for the bytes in the file at `path`, hashing only the first `num_bytes_from_start`
    /// while initializing and calling `progress`.
    ///
    /// `num_bytes_from_start` is useful to avoid reading trailing hashes, which are never part of the hash itself,
    /// denoting the amount of bytes to hash starting from the beginning of the file.
    ///
    /// # Note
    ///
    /// * [Interrupts][gix_features::interrupt] are supported.
    pub fn bytes_of_file(
        path: &std::path::Path,
        num_bytes_from_start: u64,
        kind: crate::Kind,
        progress: &mut dyn gix_features::progress::Progress,
        should_interrupt: &std::sync::atomic::AtomicBool,
    ) -> Result<crate::ObjectId, Error> {
        bytes(
            &mut std::fs::File::open(path)?,
            num_bytes_from_start,
            kind,
            progress,
            should_interrupt,
        )
    }

    /// Similar to [`bytes_of_file`], but operates on a stream of bytes.
    pub fn bytes(
        read: &mut dyn std::io::Read,
        num_bytes_from_start: u64,
        kind: crate::Kind,
        progress: &mut dyn gix_features::progress::Progress,
        should_interrupt: &std::sync::atomic::AtomicBool,
    ) -> Result<crate::ObjectId, Error> {
        bytes_with_hasher(read, num_bytes_from_start, hasher(kind), progress, should_interrupt)
    }

    /// Similar to [`bytes()`], but takes a `hasher` instead of a hash kind.
    pub fn bytes_with_hasher(
        read: &mut dyn std::io::Read,
        num_bytes_from_start: u64,
        mut hasher: Hasher,
        progress: &mut dyn gix_features::progress::Progress,
        should_interrupt: &std::sync::atomic::AtomicBool,
    ) -> Result<crate::ObjectId, Error> {
        let start = std::time::Instant::now();
        // init progress before the possibility for failure, as convenience in case people want to recover
        progress.init(
            Some(num_bytes_from_start as gix_features::progress::prodash::progress::Step),
            gix_features::progress::bytes(),
        );

        const BUF_SIZE: usize = u16::MAX as usize;
        let mut buf = [0u8; BUF_SIZE];
        let mut bytes_left = num_bytes_from_start;

        while bytes_left > 0 {
            let out = &mut buf[..BUF_SIZE.min(bytes_left as usize)];
            read.read_exact(out)?;
            bytes_left -= out.len() as u64;
            progress.inc_by(out.len());
            hasher.update(out);
            if should_interrupt.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(std::io::Error::other("Interrupted").into());
            }
        }

        let id = hasher.try_finalize()?;
        progress.show_throughput(start);
        Ok(id)
    }

    /// A utility to automatically generate a hash while writing into an inner writer.
    pub struct Write<T> {
        /// The hash implementation.
        pub hash: Hasher,
        /// The inner writer.
        pub inner: T,
    }

    impl<T> std::io::Write for Write<T>
    where
        T: std::io::Write,
    {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let written = self.inner.write(buf)?;
            self.hash.update(&buf[..written]);
            Ok(written)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }

    impl<T> Write<T>
    where
        T: std::io::Write,
    {
        /// Create a new hash writer which hashes all bytes written to `inner` with a hash of `kind`.
        pub fn new(inner: T, object_hash: crate::Kind) -> Self {
            match object_hash {
                crate::Kind::Sha1 => Write {
                    inner,
                    hash: crate::hasher(object_hash),
                },
                #[cfg(feature = "sha256")]
                crate::Kind::Sha256 => Write {
                    inner,
                    hash: crate::hasher(object_hash),
                },
            }
        }
    }
}
pub use _impl::Write;
