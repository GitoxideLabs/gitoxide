use gix_error::{ExnResult, ResultExt, message};

use std::path::PathBuf;

use crate::store_impl::{file, packed};

impl file::Store {
    /// Return a packed transaction ready to receive updates. Use this to create or update `packed-refs`.
    /// Note that if you already have a [`packed::Buffer`] then use its [`packed::Buffer::into_transaction()`] method instead.
    pub(crate) fn packed_transaction(&self, lock_mode: gix_lock::acquire::Fail) -> ExnResult<packed::Transaction> {
        let lock = gix_lock::File::acquire_to_update_resource(self.packed_refs_path(), lock_mode, None)
            .or_raise_erased(|| message("Could not lock packed refs"))?;
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
    pub fn open_packed_buffer(&self) -> ExnResult<Option<packed::Buffer>> {
        match packed::Buffer::open(
            self.packed_refs_path(),
            self.packed_buffer_mmap_threshold,
            self.object_hash,
        ) {
            Ok(buf) => Ok(Some(buf)),
            Err(err) if err.is_not_found() => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Return a possibly cached packed buffer with shared ownership. At retrieval it will assure it's up to date, but
    /// after that it can be considered a snapshot as it cannot change anymore.
    ///
    /// Use this to make successive calls to [`file::Store::try_find_packed()`]
    /// or obtain iterators using [`file::Store::iter_packed()`] in a way that assures the packed-refs content won't change.
    pub fn cached_packed_buffer(&self) -> ExnResult<Option<file::packed::SharedBufferSnapshot>> {
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

/// An up-to-date snapshot of the packed refs buffer.
pub type SharedBufferSnapshot = gix_fs::SharedFileSnapshot<packed::Buffer>;

pub(crate) mod modifiable {
    use gix_features::threading::OwnShared;

    use crate::{file, packed};
    use gix_error::{ExnResult, Message, ResultExt};

    pub(crate) type MutableSharedBuffer = OwnShared<gix_fs::SharedFileSnapshotMut<packed::Buffer>>;

    impl file::Store {
        /// Forcefully reload the packed refs buffer.
        ///
        /// This method should be used if it's clear that the buffer on disk has changed, to
        /// make the latest changes visible before other operations are done on this instance.
        ///
        /// As some filesystems don't have nanosecond granularity, changes are likely to be missed
        /// if they happen within one second otherwise.
        ///
        /// [Metadata](gix_error::Exn::metadata()) `path` (native path) identifies a packed-refs file whose modification
        /// time could not be read.
        pub fn force_refresh_packed_buffer(&self) -> ExnResult {
            self.packed.force_refresh(|| {
                let path = self.packed_refs_path();
                let modified = path
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .or_raise_erased(|| {
                        Message::new("Could not read packed refs modification time").with("path", path)
                    })?;
                self.open_packed_buffer().map(|packed| Some(modified).zip(packed))
            })
        }
        pub(crate) fn assure_packed_refs_uptodate(&self) -> ExnResult<Option<super::SharedBufferSnapshot>> {
            self.packed.recent_snapshot(
                || self.packed_refs_path().metadata().and_then(|m| m.modified()).ok(),
                || self.open_packed_buffer(),
            )
        }
    }
}
