use std::{fs, io, io::Write, path::PathBuf};

use gix_object::WriteTo;
use gix_zlib::stream::deflate;
use tempfile::NamedTempFile;

use super::Store;
use crate::store_impls::loose;

/// Returned by the [`gix_object::Write`] trait implementation of [`Store`]
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    Io {
        source: gix_hash::io::Error,
        message: &'static str,
        path: PathBuf,
    },
    IoRaw(io::Error),
    Persist {
        source: tempfile::PersistError,
        target: PathBuf,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io { message, path, .. } => write!(f, "Could not {message} '{}'", path.display()),
            Error::IoRaw(_) => f.write_str("An IO error occurred while writing an object"),
            Error::Persist { target, .. } => {
                write!(
                    f,
                    "Could not turn temporary file into persisted file at '{}'",
                    target.display()
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io { source, .. } => Some(source),
            Error::IoRaw(err) => Some(err),
            Error::Persist { source, .. } => Some(source),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoRaw(err)
    }
}

impl gix_object::Write for Store {
    fn write(&self, object: &dyn WriteTo) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        let mut to = self.dest()?;
        to.write_all(&object.loose_header()).map_err(|err| Error::Io {
            source: err.into(),
            message: "write header to tempfile in",
            path: self.path.to_owned(),
        })?;
        object.write_to(&mut to).map_err(|err| Error::Io {
            source: err.into(),
            message: "stream all data into tempfile in",
            path: self.path.to_owned(),
        })?;
        to.flush().map_err(Box::new)?;
        Ok(self.finalize_object(to).map_err(Box::new)?)
    }

    /// Write the given buffer in `from` to disk in one syscall at best.
    ///
    /// This will cost at least 4 IO operations.
    fn write_buf(&self, kind: gix_object::Kind, from: &[u8]) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        let mut to = self.dest().map_err(Box::new)?;
        to.write_all(&gix_object::encode::loose_header(kind, from.len() as u64))
            .map_err(|err| Error::Io {
                source: err.into(),
                message: "write header to tempfile in",
                path: self.path.to_owned(),
            })?;

        to.write_all(from).map_err(|err| Error::Io {
            source: err.into(),
            message: "stream all data into tempfile in",
            path: self.path.to_owned(),
        })?;
        to.flush()?;
        Ok(self.finalize_object(to)?)
    }

    fn write_buf_with_known_id(
        &self,
        kind: gix_object::Kind,
        from: &[u8],
        id: gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        let mut to = self.compressed_tempfile().map_err(Box::new)?;
        to.write_all(&gix_object::encode::loose_header(kind, from.len() as u64))
            .map_err(|err| Error::Io {
                source: err.into(),
                message: "write header to tempfile in",
                path: self.path.to_owned(),
            })?;

        to.write_all(from).map_err(|err| Error::Io {
            source: err.into(),
            message: "stream all data into tempfile in",
            path: self.path.to_owned(),
        })?;
        to.flush()?;
        Ok(self.finalize_object_at(id, to)?)
    }

    /// Write the given stream in `from` to disk with at least one syscall.
    ///
    /// This will cost at least 4 IO operations.
    fn write_stream(
        &self,
        kind: gix_object::Kind,
        size: u64,
        mut from: &mut dyn io::Read,
    ) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        let mut to = self.dest().map_err(Box::new)?;
        to.write_all(&gix_object::encode::loose_header(kind, size))
            .map_err(|err| Error::Io {
                source: err.into(),
                message: "write header to tempfile in",
                path: self.path.to_owned(),
            })?;

        io::copy(&mut from, &mut to)
            .map_err(|err| Error::Io {
                source: err.into(),
                message: "stream all data into tempfile in",
                path: self.path.to_owned(),
            })
            .map_err(Box::new)?;
        to.flush().map_err(Box::new)?;
        Ok(self.finalize_object(to)?)
    }

    fn write_stream_with_known_id(
        &self,
        kind: gix_object::Kind,
        size: u64,
        mut from: &mut dyn io::Read,
        id: gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, gix_object::write::Error> {
        let mut to = self.compressed_tempfile().map_err(Box::new)?;
        to.write_all(&gix_object::encode::loose_header(kind, size))
            .map_err(|err| Error::Io {
                source: err.into(),
                message: "write header to tempfile in",
                path: self.path.to_owned(),
            })?;

        io::copy(&mut from, &mut to)
            .map_err(|err| Error::Io {
                source: err.into(),
                message: "stream all data into tempfile in",
                path: self.path.to_owned(),
            })
            .map_err(Box::new)?;
        to.flush().map_err(Box::new)?;
        Ok(self.finalize_object_at(id, to)?)
    }
}

type CompressedTempfile = deflate::Write<NamedTempFile>;

/// Access
impl Store {
    /// Return the path to the object with `id`.
    ///
    /// Note that is may not exist yet.
    pub fn object_path(&self, id: &gix_hash::oid) -> PathBuf {
        loose::hash_path(id, self.path.clone())
    }
}

impl Store {
    /// A compressed tempfile, with auto-hashing.
    fn dest(&self) -> Result<gix_hash::io::Write<CompressedTempfile>, Error> {
        Ok(gix_hash::io::Write::new(self.compressed_tempfile()?, self.object_hash))
    }

    /// A compressed tempfile, without hasher.
    fn compressed_tempfile(&self) -> Result<CompressedTempfile, Error> {
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut builder = tempfile::Builder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o444);
            builder.permissions(perms);
        }
        Ok(deflate::Write::new(
            builder.tempfile_in(&self.path).map_err(|err| Error::Io {
                source: err.into(),
                message: "create named temp file in",
                path: self.path.to_owned(),
            })?,
            self.compression,
        ))
    }

    fn finalize_object(
        &self,
        gix_hash::io::Write { hash, inner: file }: gix_hash::io::Write<CompressedTempfile>,
    ) -> Result<gix_hash::ObjectId, Error> {
        let id = hash.try_finalize().map_err(|err| Error::Io {
            source: err.into(),
            message: "hash tempfile in",
            path: self.path.to_owned(),
        })?;
        self.finalize_object_at(id, file)
    }

    fn finalize_object_at(
        &self,
        id: gix_hash::ObjectId,
        file: CompressedTempfile,
    ) -> Result<gix_hash::ObjectId, Error> {
        let object_path = loose::hash_path(&id, self.path.clone());
        let object_dir = object_path
            .parent()
            .expect("each object path has a 1 hex-bytes directory");
        if let Err(err) = fs::create_dir(object_dir) {
            match err.kind() {
                io::ErrorKind::AlreadyExists => {}
                _ => return Err(err.into()),
            }
        }
        let file = file.into_inner();
        let res = file.persist(&object_path);
        // On windows, we assume that such errors are due to its special filesystem semantics,
        // on any other platform that would be a legitimate error though.
        #[cfg(windows)]
        if let Err(err) = &res {
            if err.error.kind() == std::io::ErrorKind::PermissionDenied
                || err.error.kind() == std::io::ErrorKind::AlreadyExists
            {
                return Ok(id);
            }
        }
        res.map_err(|err| Error::Persist {
            source: err,
            target: object_path,
        })?;
        Ok(id)
    }
}
