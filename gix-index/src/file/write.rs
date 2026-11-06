use crate::{File, Version, write};
use gix_error::Result;

impl File {
    /// Write the index to `out` with `options`, to be readable by [`File::at()`], returning the version that was actually written
    /// to retain all information of this index.
    /// Also returns the checksum written to the trailer, which is null if [`write::Options::skip_hash`] is set.
    /// Unlike [`File::checksum()`], this preserves the on-disk representation of an omitted checksum.
    ///
    /// The `tree` (tree-cache) extension is written as-is. Mutable entry or path access removes it;
    /// callers restoring it with [`State::set_tree()`](crate::State::set_tree()) must ensure it is current.
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
    ///
    /// The `tree` (tree-cache) extension is written as-is, without recomputing it. Mutable entry or
    /// path access removes the cache so staged changes cannot retain outdated tree IDs. Callers
    /// restoring it with [`State::set_tree()`](crate::State::set_tree()) must ensure it is current.
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
        self.checksum = (!digest.is_null()).then_some(digest);
        Ok(())
    }
}
