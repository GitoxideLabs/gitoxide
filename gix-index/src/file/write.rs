use crate::{File, Version, write};
use gix_error::Result;

impl File {
    /// Write the index to `out` with `options`, to be readable by [`File::at()`], returning the version that was actually written
    /// to retain all information of this index.
    ///
    /// The tree cache is written as-is. Mutable entry access invalidates it; writing
    /// does not recompute it. See [`File::write()`].
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
    /// Note that the hash produced will be stored which is why we need to be mutable.
    ///
    /// The tree cache is written as-is, without recomputing it. The mutation APIs
    /// on [`State`](crate::State) discard it when exposing mutable entries or
    /// changing paths. Callers that modify the cache directly through
    /// [`State::tree_mut()`](crate::State::tree_mut()) must keep it consistent with
    /// the entries, or discard it with [`State::remove_tree()`](crate::State::remove_tree()).
    pub fn write(&mut self, options: write::Options) -> Result {
        use gix_error::{ErrorExt, ResultExt, bail, message};

        let _span = gix_trace::detail!("gix_index::File::write()", path = ?self.path);
        let mut lock = std::io::BufWriter::with_capacity(
            64 * 1024,
            gix_lock::File::acquire_to_update_resource(&self.path, gix_lock::acquire::Fail::Immediately, None, 0)
                .or_raise(|| message("Could not acquire lock for index file"))?,
        );
        let (version, digest) = self
            .write_to(&mut lock, options)
            .or_raise(|| message("Could not write index"))?;
        match lock.into_inner() {
            Ok(lock) => lock
                .commit()
                .or_raise(|| message("Could not commit lock for index file"))?,
            Err(err) => {
                bail!(
                    err.into_error()
                        .and_raise(message("Could not flush buffered index data"))
                );
            }
        };
        self.state.version = version;
        self.checksum = Some(digest);
        Ok(())
    }
}
