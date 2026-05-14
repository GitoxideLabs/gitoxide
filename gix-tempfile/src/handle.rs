//!
#![allow(clippy::empty_docs)]
use std::{io, path::Path};

use tempfile::{NamedTempFile, TempPath};

use crate::{AutoRemove, ContainingDirectory, ForksafeTempfile, Handle, NEXT_MAP_INDEX, REGISTRY};

/// Marker to signal the Registration is an open file able to be written to.
#[derive(Debug)]
pub struct Writable;

/// Marker to signal the Registration is a closed file that consumes no additional process resources.
///
/// It can't ever be written to unless reopened after persisting it.
#[derive(Debug)]
pub struct Closed;

pub(crate) enum Mode {
    Writable,
    Closed,
}

/// Utilities
impl Handle<()> {
    fn at_path(
        path: &Path,
        directory: ContainingDirectory,
        cleanup: AutoRemove,
        mode: Mode,
        permissions: Option<std::fs::Permissions>,
    ) -> io::Result<usize> {
        let tempfile = {
            let mut builder = tempfile::Builder::new();
            let dot_ext_storage;
            match path.file_stem() {
                Some(stem) => builder.prefix(stem),
                None => builder.prefix(""),
            };
            if let Some(ext) = path.extension() {
                dot_ext_storage = format!(".{}", ext.to_string_lossy());
                builder.suffix(&dot_ext_storage);
            }
            if let Some(permissions) = permissions {
                builder.permissions(permissions);
            }
            let parent_dir = path.parent().expect("parent directory is present");
            let parent_dir = directory.resolve(parent_dir)?;
            ForksafeTempfile::new(builder.rand_bytes(0).tempfile_in(parent_dir)?, cleanup, mode)
        };
        let id = NEXT_MAP_INDEX.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        expect_none(REGISTRY.insert(id, Some(tempfile)));
        Ok(id)
    }

    fn new_writable_inner(
        containing_directory: &Path,
        directory: ContainingDirectory,
        cleanup: AutoRemove,
        mode: Mode,
    ) -> io::Result<usize> {
        let containing_directory = directory.resolve(containing_directory)?;
        let id = NEXT_MAP_INDEX.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        expect_none(REGISTRY.insert(
            id,
            Some(ForksafeTempfile::new(
                NamedTempFile::new_in(containing_directory)?,
                cleanup,
                mode,
            )),
        ));
        Ok(id)
    }
}

/// Creation and ownership transfer
impl Handle<Closed> {
    /// Create a registered tempfile at the given `path`, where `path` includes the desired filename and close it immediately.
    ///
    /// Depending on the `directory` configuration, intermediate directories will be created, and depending on `cleanup` empty
    /// intermediate directories will be removed.
    ///
    /// ### Warning of potential leaks
    ///
    /// Without [signal handlers](crate::signal) installed, tempfiles will remain once a termination
    /// signal is encountered as destructors won't run. See [the top-level documentation](crate) for more.
    pub fn at(path: impl AsRef<Path>, directory: ContainingDirectory, cleanup: AutoRemove) -> io::Result<Self> {
        Ok(Handle {
            id: Handle::<()>::at_path(path.as_ref(), directory, cleanup, Mode::Closed, None)?,
            _marker: Default::default(),
        })
    }

    /// Like [`at`](Self::at()), but with support for filesystem `permissions`.
    pub fn at_with_permissions(
        path: impl AsRef<Path>,
        directory: ContainingDirectory,
        cleanup: AutoRemove,
        permissions: std::fs::Permissions,
    ) -> io::Result<Self> {
        Ok(Handle {
            id: Handle::<()>::at_path(path.as_ref(), directory, cleanup, Mode::Closed, Some(permissions))?,
            _marker: Default::default(),
        })
    }

    /// Take ownership of the temporary file path, which deletes it when dropped without persisting it beforehand.
    ///
    /// It's a theoretical possibility that the file isn't present anymore if signals interfere, hence the `Option`
    pub fn take(self) -> Option<TempPath> {
        let res = REGISTRY.remove(&self.id);
        std::mem::forget(self);
        res.and_then(|(_k, v)| v.map(ForksafeTempfile::into_temppath))
    }
}

/// Creation and ownership transfer
impl Handle<Writable> {
    /// Create a registered tempfile at the given `path`, where `path` includes the desired filename.
    ///
    /// Depending on the `directory` configuration, intermediate directories will be created, and depending on `cleanup` empty
    /// intermediate directories will be removed.
    ///
    /// ### Warning of potential leaks
    ///
    /// Without [signal handlers](crate::signal) installed, tempfiles will remain once a termination
    /// signal is encountered as destructors won't run. See [the top-level documentation](crate) for more.
    pub fn at(path: impl AsRef<Path>, directory: ContainingDirectory, cleanup: AutoRemove) -> io::Result<Self> {
        Ok(Handle {
            id: Handle::<()>::at_path(path.as_ref(), directory, cleanup, Mode::Writable, None)?,
            _marker: Default::default(),
        })
    }

    /// Like [`at`](Self::at()), but with support for filesystem `permissions`.
    pub fn at_with_permissions(
        path: impl AsRef<Path>,
        directory: ContainingDirectory,
        cleanup: AutoRemove,
        permissions: std::fs::Permissions,
    ) -> io::Result<Self> {
        Ok(Handle {
            id: Handle::<()>::at_path(path.as_ref(), directory, cleanup, Mode::Writable, Some(permissions))?,
            _marker: Default::default(),
        })
    }

    /// Create a registered tempfile within `containing_directory` with a name that won't clash, and clean it up as specified with `cleanup`.
    /// Control how to deal with intermediate directories with `directory`.
    /// The temporary file is opened and can be written to using the [`with_mut()`][Handle::with_mut()] method.
    ///
    /// ### Warning of potential leaks
    ///
    /// Without [signal handlers](crate::signal) installed, tempfiles will remain once a termination
    /// signal is encountered as destructors won't run. See [the top-level documentation](crate) for more.
    pub fn new(
        containing_directory: impl AsRef<Path>,
        directory: ContainingDirectory,
        cleanup: AutoRemove,
    ) -> io::Result<Self> {
        Ok(Handle {
            id: Handle::<()>::new_writable_inner(containing_directory.as_ref(), directory, cleanup, Mode::Writable)?,
            _marker: Default::default(),
        })
    }

    /// Take ownership of the temporary file.
    ///
    /// It's a theoretical possibility that the file isn't present anymore if signals interfere, hence the `Option`
    pub fn take(self) -> Option<NamedTempFile> {
        let res = REGISTRY.remove(&self.id);
        std::mem::forget(self);
        res.and_then(|(_k, v)| v.map(|v| v.into_tempfile().expect("correct runtime typing")))
    }

    /// Close the underlying file handle but keep track of the temporary file as before for automatic cleanup.
    ///
    /// This saves system resources in situations where one opens a tempfile file at a time, writes a new value, and closes
    /// it right after to perform more updates of this kind in other tempfiles. When all succeed, they can be renamed one after
    /// another.
    pub fn close(self) -> std::io::Result<Handle<Closed>> {
        match REGISTRY.remove(&self.id) {
            Some((id, Some(t))) => {
                std::mem::forget(self);
                expect_none(REGISTRY.insert(id, Some(t.close())));
                Ok(Handle::<Closed> {
                    id,
                    _marker: Default::default(),
                })
            }
            None | Some((_, None)) => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("The tempfile with id {} wasn't available anymore", self.id),
            )),
        }
    }
}

/// Mutation
impl Handle<Writable> {
    /// Obtain a mutable handler to the underlying named tempfile and call `f(&mut named_tempfile)` on it.
    ///
    /// Note that for the duration of the call, a signal interrupting the operation will cause the tempfile not to be cleaned up
    /// as it is not visible anymore to the signal handler.
    ///
    /// # Assumptions
    /// The caller must assure that the signal handler for cleanup will be followed by an abort call so that
    /// this code won't run again on a removed instance. An error will occur otherwise.
    pub fn with_mut<T>(&mut self, once: impl FnOnce(&mut NamedTempFile) -> T) -> std::io::Result<T> {
        match REGISTRY.remove(&self.id) {
            Some((id, Some(mut t))) => {
                let res = once(t.as_mut_tempfile().expect("correct runtime typing"));
                expect_none(REGISTRY.insert(id, Some(t)));
                Ok(res)
            }
            None | Some((_, None)) => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("The tempfile with id {} wasn't available anymore", self.id),
            )),
        }
    }
}

mod io_impls {
    use std::{io, io::SeekFrom};

    use super::{Handle, Writable};

    impl io::Write for Handle<Writable> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.with_mut(|f| f.write(buf))?
        }

        fn flush(&mut self) -> io::Result<()> {
            self.with_mut(io::Write::flush)?
        }
    }

    impl io::Seek for Handle<Writable> {
        fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
            self.with_mut(|f| f.seek(pos))?
        }
    }

    impl io::Read for Handle<Writable> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.with_mut(|f| f.read(buf))?
        }
    }
}

///
pub mod persist {
    use std::path::Path;

    use crate::{
        Handle, REGISTRY,
        handle::{Closed, Writable, expect_none},
    };

    mod error {
        use std::fmt::{self, Debug, Display};

        use crate::Handle;

        /// The error returned by various [`persist(…)`][Handle<crate::handle::Writable>::persist()] methods
        #[derive(Debug)]
        pub struct Error<T: Debug> {
            /// The io error that prevented the attempt to succeed
            pub error: std::io::Error,
            /// The registered handle to the tempfile which couldn't be persisted.
            pub handle: Handle<T>,
        }

        impl<T: Debug> Display for Error<T> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                Display::fmt(&self.error, f)
            }
        }

        impl<T: Debug> std::error::Error for Error<T> {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                self.error.source()
            }
        }
    }
    pub use error::Error;

    impl Handle<Writable> {
        /// Persist this tempfile to replace the file at the given `path` if necessary, in a way that recovers the original instance
        /// on error or returns the open now persisted former tempfile.
        /// Note that it might not exist anymore if an interrupt handler managed to steal it and allowed the program to return to
        /// its normal flow.
        pub fn persist(self, path: impl AsRef<Path>) -> Result<Option<std::fs::File>, Error<Writable>> {
            let res = REGISTRY.remove(&self.id);

            match res.and_then(|(_k, v)| v.map(|v| v.persist(path))) {
                Some(Ok(Some(file))) => {
                    std::mem::forget(self);
                    Ok(Some(file))
                }
                None => {
                    std::mem::forget(self);
                    Ok(None)
                }
                Some(Err((err, tempfile))) => {
                    expect_none(REGISTRY.insert(self.id, Some(tempfile)));
                    Err(Error::<Writable> {
                        error: err,
                        handle: self,
                    })
                }
                Some(Ok(None)) => unreachable!("no open files in an open handle"),
            }
        }
    }

    impl Handle<Closed> {
        /// Persist this tempfile to replace the file at the given `path` if necessary, in a way that recovers the original instance
        /// on error.
        pub fn persist(self, path: impl AsRef<Path>) -> Result<(), Error<Closed>> {
            let res = REGISTRY.remove(&self.id);

            match res.and_then(|(_k, v)| v.map(|v| v.persist(path))) {
                None | Some(Ok(None)) => {
                    std::mem::forget(self);
                    Ok(())
                }
                Some(Err((err, tempfile))) => {
                    expect_none(REGISTRY.insert(self.id, Some(tempfile)));
                    Err(Error::<Closed> {
                        error: err,
                        handle: self,
                    })
                }
                Some(Ok(Some(_file))) => unreachable!("no open files in a closed handle"),
            }
        }
    }
}

impl ContainingDirectory {
    fn resolve(self, dir: &Path) -> std::io::Result<&Path> {
        match self {
            ContainingDirectory::Exists => Ok(dir),
            ContainingDirectory::CreateAllRaceProof(retries) => crate::create_dir::all(dir, retries),
        }
    }
}

fn expect_none<T>(v: Option<T>) {
    assert!(
        v.is_none(),
        "there should never be conflicts or old values as ids are never reused."
    );
}

impl<T: std::fmt::Debug> Drop for Handle<T> {
    fn drop(&mut self) {
        if let Some((_id, Some(tempfile))) = REGISTRY.remove(&self.id) {
            tempfile.drop_impl();
        }
    }
}
