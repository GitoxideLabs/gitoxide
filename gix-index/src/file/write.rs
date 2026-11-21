use crate::{File, Version, write};
use gix_error::{Result, ResultExt, message};
use gix_fs::SharedPermissions;

impl File {
    /// Write the index to `out` with `options`, to be readable by [`File::at()`], returning the version that was actually written
    /// to retain all information of this index.
    /// Also returns the checksum written to the trailer, which is null if [`write::Options::skip_hash`] is set.
    /// Unlike [`File::checksum()`], this preserves the on-disk representation of an omitted checksum.
    ///
    /// Cache extensions are invalidated by default, without mutating the in-memory caches.
    /// See [`File::write()`] for how to retain valid caches after changing entries.
    pub fn write_to(
        &self,
        mut out: impl std::io::Write,
        options: write::Options,
    ) -> Result<(Version, gix_hash::ObjectId)> {
        let _span = gix_trace::detail!("gix_index::File::write_to()", skip_hash = options.skip_hash);
        let (version, hash) = if options.skip_hash {
            let out: &mut dyn std::io::Write = &mut out;
            let version = self.state.write_to(out, options)?;
            (version, self.state.object_hash.null())
        } else {
            let mut hasher = gix_hash::io::Write::new(&mut out, self.state.object_hash);
            let out: &mut dyn std::io::Write = &mut hasher;
            let version = self.state.write_to(out, options)?;
            (version, hasher.hash.try_finalize()?)
        };
        out.write_all(hash.as_slice()).map_err(gix_hash::io::from_std_io)?;
        Ok((version, hash))
    }

    /// Write ourselves to the path we were read from after acquiring a lock, using `options`.
    ///
    /// The checksum is updated after a successful write, or cleared if [`write::Options::skip_hash`] is set.
    /// The sharing policy `shared_repository_permissions` applies to the newly written index.
    ///
    /// ### Cache extensions
    ///
    /// By default, [`write::Extensions::All`] writes all supported optional extensions, but conservatively invalidates
    /// the tree-cache and untracked cache so Git recomputes them. The in-memory caches are left untouched.
    ///
    /// [`write::Extensions::AllUnconditionally`] and [`write::Extensions::Given`] write selected caches
    /// as-is. They must already match the current entries and worktree: a stale tree-cache can cause
    /// `git commit` or `git write-tree` to capture outdated subtree content, and a stale untracked cache
    /// can hide files whose tracked status changed.
    ///
    /// A newly constructed index can use [`State::copy_metadata_from()`](crate::State::copy_metadata_from())
    /// with its original state to retain and selectively invalidate both caches before writing with
    /// [`write::Extensions::AllUnconditionally`].
    /// Mutable entry or path access removes the tree-cache. Callers restoring it with
    /// [`State::set_tree()`](crate::State::set_tree()) must ensure it is current.
    pub fn write(&mut self, options: write::Options, shared_repository_permissions: SharedPermissions) -> Result {
        let _span = gix_trace::detail!("gix_index::File::write()", path = ?self.path);
        let lock = self.acquire_lock(shared_repository_permissions)?;
        self.write_locked(lock, options)
    }

    /// Like [`write()`](Self::write), but only replace the index if its on-disk snapshot is still current.
    /// Return `false` if the lock cannot be acquired immediately or the snapshot cannot be verified,
    /// including for a newly constructed index. Errors writing or committing the locked index are propagated.
    /// If the index has no checksum, verify the complete file against the original snapshot.
    pub fn write_if_unchanged(
        &mut self,
        options: write::Options,
        shared_repository_permissions: SharedPermissions,
    ) -> Result<bool> {
        let Some(snapshot) = &self.snapshot else {
            return Ok(false);
        };
        let Ok(lock) = self.acquire_lock(shared_repository_permissions) else {
            return Ok(false);
        };
        if !snapshot.matches(&self.path).unwrap_or(false) {
            return Ok(false);
        }
        self.write_locked(lock, options)?;
        Ok(true)
    }

    fn acquire_lock(&self, permissions: SharedPermissions) -> Result<gix_lock::File> {
        gix_lock::File::acquire_to_update_resource(&self.path, gix_lock::acquire::Fail::Immediately, None, permissions)
            .or_raise(|| message("Could not acquire lock for index file"))
    }

    fn write_locked(&mut self, lock: gix_lock::File, options: write::Options) -> Result {
        let mut lock = std::io::BufWriter::with_capacity(64 * 1024, lock);
        let (version, digest) = self
            .write_to(&mut lock, options)
            .or_raise(|| message("Could not write index"))?;
        let (_, file) = lock
            .into_inner()
            .map_err(std::io::IntoInnerError::into_error)
            .or_raise(|| message("Could not flush buffered index data"))?
            .commit()
            .or_raise(|| message("Could not commit lock for index file"))?;
        self.state.version = version;
        self.snapshot = super::Snapshot::new(digest, file);
        Ok(())
    }
}
