use std::path::{Path, PathBuf};

use crate::{DOT_LOCK_SUFFIX, File, Marker};

fn strip_lock_suffix(lock_path: &Path) -> PathBuf {
    let ext = lock_path
        .extension()
        .expect("at least our own extension")
        .to_str()
        .expect("no illegal UTF8 in extension");
    lock_path.with_extension(ext.split_at(ext.len().saturating_sub(DOT_LOCK_SUFFIX.len())).0)
}

impl File {
    /// Obtain a mutable reference to the write handle and call `f(out)` with it.
    pub fn with_mut<T>(&mut self, f: impl FnOnce(&mut std::fs::File) -> std::io::Result<T>) -> std::io::Result<T> {
        self.inner.with_mut(|tf| f(tf.as_file_mut())).and_then(|res| res)
    }
    /// Close the lock file to prevent further writes and to save system resources.
    /// A call to [`Marker::commit()`] is allowed on the [`Marker`] to write changes back to the resource.
    pub fn close(self) -> std::io::Result<Marker> {
        Ok(Marker {
            inner: self.inner.close()?,
            created_from_file: true,
            lock_path: self.lock_path,
        })
    }

    /// Return the path at which the lock file resides
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }

    /// Return the path at which the locked resource resides
    pub fn resource_path(&self) -> PathBuf {
        strip_lock_suffix(&self.lock_path)
    }
}

mod io_impls {
    use std::{io, io::SeekFrom};

    use super::File;

    impl io::Write for File {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.inner.with_mut(|f| f.write(buf))?
        }

        fn flush(&mut self) -> io::Result<()> {
            self.inner.with_mut(io::Write::flush)?
        }
    }

    impl io::Seek for File {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.inner.with_mut(|f| f.seek(pos))?
        }
    }

    impl io::Read for File {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.inner.with_mut(|f| f.read(buf))?
        }
    }
}

impl Marker {
    /// Return the path at which the lock file resides
    pub fn lock_path(&self) -> &Path {
        &self.lock_path
    }

    /// Return the path at which the locked resource resides
    pub fn resource_path(&self) -> PathBuf {
        strip_lock_suffix(&self.lock_path)
    }
}
