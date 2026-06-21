use crate::{File, Version, write};

/// The error produced by [`File::write()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] gix_hash::io::Error),
    #[error("Could not acquire lock for index file")]
    AcquireLock(#[from] gix_lock::acquire::Error),
    #[error("Could not commit lock for index file")]
    CommitLock(#[from] gix_lock::commit::Error<gix_lock::File>),
}

impl File {
    /// Write the index to `out` with `options`, to be readable by [`File::at()`], returning the version that was actually written
    /// to retain all information of this index.
    ///
    /// Note that the `tree` (tree-cache) extension is written as-is and is **not** recomputed from the
    /// current entries; see [`File::write()`] for the implications and the recommended workaround.
    pub fn write_to(
        &self,
        mut out: impl std::io::Write,
        options: write::Options,
    ) -> Result<(Version, gix_hash::ObjectId), gix_hash::io::Error> {
        let _span = gix_features::trace::detail!("gix_index::File::write_to()", skip_hash = options.skip_hash);
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
        out.write_all(hash.as_slice())?;
        Ok((version, hash))
    }

    /// Write ourselves to the path we were read from after acquiring a lock, using `options`.
    ///
    /// Note that the hash produced will be stored which is why we need to be mutable.
    ///
    /// ### The `tree` (tree-cache) extension is written as-is
    ///
    /// The `tree` extension (tree-cache) is serialized from its current in-memory state; it is
    /// **not** recomputed to match the entries. If entries were modified since the index was read,
    /// the now-stale tree-cache is written out, which can leave the index in a state that other Git
    /// implementations misread: they consult the tree-cache to decide what changed, so modifications
    /// may appear invisible to `git status`, or be committed incorrectly.
    ///
    /// Until updating the tree-cache on write is implemented (see [issue #2421]), remove it with
    /// [`State::remove_tree()`](crate::State::remove_tree()) before writing whenever entries were
    /// changed:
    ///
    /// ```ignore
    /// index.remove_tree();
    /// index.write(gix_index::write::Options::default())?;
    /// ```
    ///
    /// [issue #2421]: https://github.com/GitoxideLabs/gitoxide/issues/2421
    pub fn write(&mut self, options: write::Options) -> Result<(), Error> {
        let _span = gix_features::trace::detail!("gix_index::File::write()", path = ?self.path);
        let mut lock = std::io::BufWriter::with_capacity(
            64 * 1024,
            gix_lock::File::acquire_to_update_resource(&self.path, gix_lock::acquire::Fail::Immediately, None)?,
        );
        let (version, digest) = self.write_to(&mut lock, options)?;
        match lock.into_inner() {
            Ok(lock) => lock.commit()?,
            Err(err) => return Err(Error::Io(err.into_error().into())),
        };
        self.state.version = version;
        self.checksum = Some(digest);
        Ok(())
    }
}
