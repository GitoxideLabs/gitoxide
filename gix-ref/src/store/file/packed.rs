use std::path::PathBuf;

use crate::store_impl::{file, packed};

impl file::Store {
    /// Return a packed transaction ready to receive updates. Use this to create or update `packed-refs`.
    /// Note that if you already have a [`packed::Buffer`] then use its [`packed::Buffer::into_transaction()`] method instead.
    pub(crate) fn packed_transaction(
        &self,
        lock_mode: gix_lock::acquire::Fail,
    ) -> Result<packed::Transaction, transaction::Error> {
        let lock = gix_lock::File::acquire_to_update_resource(self.packed_refs_path(), lock_mode, None)
            .map_err(|err| transaction::Error::TransactionLock(std::io::Error::other(err.into_error())))?;
        // We 'steal' the possibly existing packed buffer which may safe time if it's already there and fresh.
        // If nothing else is happening, nobody will get to see the soon stale buffer either, but if so, they will pay
        // for reloading it. That seems preferred over always loading up a new one.
        Ok(packed::Transaction::new_from_pack_and_lock(
            self.assure_packed_refs_uptodate()?,
            lock,
            self.precompose_unicode,
            self.namespace.clone(),
        ))
    }

    /// Try to open a new packed buffer. It's not an error if it doesn't exist, but yields `Ok(None)`.
    ///
    /// Note that it will automatically be memory mapped if it exceeds the default threshold of 32KB.
    /// Change the threshold with [file::Store::set_packed_buffer_mmap_threshold()].
    pub fn open_packed_buffer(&self) -> Result<Option<packed::Buffer>, packed::buffer::open::Error> {
        match packed::Buffer::open(
            self.packed_refs_path(),
            self.packed_buffer_mmap_threshold,
            self.object_hash,
        ) {
            Ok(buf) => Ok(Some(buf)),
            Err(packed::buffer::open::Error::Io(err)) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Return a possibly cached packed buffer with shared ownership. At retrieval it will assure it's up to date, but
    /// after that it can be considered a snapshot as it cannot change anymore.
    ///
    /// Use this to make successive calls to [`file::Store::try_find_packed()`]
    /// or obtain iterators using [`file::Store::iter_packed()`] in a way that assures the packed-refs content won't change.
    pub fn cached_packed_buffer(
        &self,
    ) -> Result<Option<file::packed::SharedBufferSnapshot>, packed::buffer::open::Error> {
        self.assure_packed_refs_uptodate()
    }

    /// Return the path at which packed-refs would usually be stored
    pub fn packed_refs_path(&self) -> PathBuf {
        self.common_dir_resolved().join("packed-refs")
    }

    pub(crate) fn packed_refs_lock_path(&self) -> PathBuf {
        let mut p = self.packed_refs_path();
        p.set_extension("lock");
        p
    }
}

///
pub mod transaction {

    use crate::store_impl::packed;

    /// The error returned by [`file::Transaction::prepare()`][crate::file::Transaction::prepare()].
    #[derive(Debug, thiserror::Error)]
    #[expect(missing_docs)]
    pub enum Error {
        #[error("An existing pack couldn't be opened or read when preparing a transaction")]
        BufferOpen(#[from] packed::buffer::open::Error),
        #[error("The lock for a packed transaction could not be obtained")]
        TransactionLock(#[source] std::io::Error),
    }
}

/// An up-to-date snapshot of the packed refs buffer.
pub type SharedBufferSnapshot = gix_fs::SharedFileSnapshot<packed::Buffer>;

pub(crate) mod modifiable {
    use gix_features::threading::OwnShared;

    use crate::{file, packed};

    pub(crate) type MutableSharedBuffer = OwnShared<gix_fs::SharedFileSnapshotMut<packed::Buffer>>;

    impl file::Store {
        /// Forcefully reload the packed refs buffer.
        ///
        /// This method should be used if it's clear that the buffer on disk has changed, to
        /// make the latest changes visible before other operations are done on this instance.
        ///
        /// As some filesystems don't have nanosecond granularity, changes are likely to be missed
        /// if they happen within one second otherwise.
        pub fn force_refresh_packed_buffer(&self) -> Result<(), packed::buffer::open::Error> {
            self.packed.force_refresh(|| {
                let modified = self.packed_refs_path().metadata()?.modified()?;
                self.open_packed_buffer().map(|packed| Some(modified).zip(packed))
            })
        }
        pub(crate) fn assure_packed_refs_uptodate(
            &self,
        ) -> Result<Option<super::SharedBufferSnapshot>, packed::buffer::open::Error> {
            self.packed.recent_snapshot(
                || self.packed_refs_path().metadata().and_then(|m| m.modified()).ok(),
                || self.open_packed_buffer(),
            )
        }
    }
}
