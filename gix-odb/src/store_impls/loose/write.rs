use std::{
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

use gix_error::{ErrorExt, Exn, Metadata, ResultExt};
use gix_object::WriteTo;
use gix_zlib::stream::deflate;
use tempfile::NamedTempFile;

use super::Store;
use crate::store_impls::loose;

impl gix_object::Write for Store {
    /// Write failures include metadata `path` (native path), the temporary object directory.
    fn write(&self, object: &dyn WriteTo) -> Result<gix_hash::ObjectId, Exn> {
        let mut to = self.dest()?;
        to.write_all(&object.loose_header())
            .or_raise_erased(|| write_header_error(&self.path))?;
        object
            .write_to(&mut to)
            .or_raise_erased(|| stream_data_error(&self.path))?;
        to.flush().or_erased()?;
        self.finalize_object(to)
    }

    /// Write the given buffer in `from` to disk in one syscall at best.
    ///
    /// This will cost at least 4 IO operations.
    /// Write failures include metadata `path` (native path), the temporary object directory.
    fn write_buf(&self, kind: gix_object::Kind, from: &[u8]) -> Result<gix_hash::ObjectId, Exn> {
        let mut to = self.dest()?;
        to.write_all(&gix_object::encode::loose_header(kind, from.len() as u64))
            .or_raise_erased(|| write_header_error(&self.path))?;

        to.write_all(from).or_raise_erased(|| stream_data_error(&self.path))?;
        to.flush().or_erased()?;
        self.finalize_object(to)
    }

    /// Write failures include metadata `path` (native path), the temporary object directory.
    fn write_buf_with_known_id(
        &self,
        kind: gix_object::Kind,
        from: &[u8],
        id: gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, Exn> {
        let mut to = self.compressed_tempfile()?;
        to.write_all(&gix_object::encode::loose_header(kind, from.len() as u64))
            .or_raise_erased(|| write_header_error(&self.path))?;

        to.write_all(from).or_raise_erased(|| stream_data_error(&self.path))?;
        to.flush().or_erased()?;
        self.finalize_object_at(id, to)
    }

    /// Write the given stream in `from` to disk with at least one syscall.
    ///
    /// This will cost at least 4 IO operations.
    /// Write failures include metadata `path` (native path), the temporary object directory.
    fn write_stream(
        &self,
        kind: gix_object::Kind,
        size: u64,
        mut from: &mut dyn io::Read,
    ) -> Result<gix_hash::ObjectId, Exn> {
        let mut to = self.dest()?;
        to.write_all(&gix_object::encode::loose_header(kind, size))
            .or_raise_erased(|| write_header_error(&self.path))?;

        io::copy(&mut from, &mut to).or_raise_erased(|| stream_data_error(&self.path))?;
        to.flush().or_erased()?;
        self.finalize_object(to)
    }

    /// Write failures include metadata `path` (native path), the temporary object directory.
    fn write_stream_with_known_id(
        &self,
        kind: gix_object::Kind,
        size: u64,
        mut from: &mut dyn io::Read,
        id: gix_hash::ObjectId,
    ) -> Result<gix_hash::ObjectId, Exn> {
        let mut to = self.compressed_tempfile()?;
        to.write_all(&gix_object::encode::loose_header(kind, size))
            .or_raise_erased(|| write_header_error(&self.path))?;

        io::copy(&mut from, &mut to).or_raise_erased(|| stream_data_error(&self.path))?;
        to.flush().or_erased()?;
        self.finalize_object_at(id, to)
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
    fn dest(&self) -> Result<gix_hash::io::Write<CompressedTempfile>, Exn> {
        Ok(gix_hash::io::Write::new(self.compressed_tempfile()?, self.object_hash))
    }

    /// A compressed tempfile, without hasher.
    /// Creation failures include metadata `path` (native path), the temporary object directory.
    fn compressed_tempfile(&self) -> Result<CompressedTempfile, Exn> {
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut builder = tempfile::Builder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o444);
            builder.permissions(perms);
        }
        Ok(deflate::Write::new(
            builder.tempfile_in(&self.path).or_raise_erased(|| {
                Metadata::new("Could not create temporary object file").with("path", self.path.as_path())
            })?,
            self.compression,
        ))
    }

    /// Hashing failures include metadata `path` (native path), the temporary object directory.
    fn finalize_object(
        &self,
        gix_hash::io::Write { hash, inner: file }: gix_hash::io::Write<CompressedTempfile>,
    ) -> Result<gix_hash::ObjectId, Exn> {
        let id = hash
            .try_finalize()
            .map_err(gix_hash::io::from_hasher)
            .or_raise_erased(|| {
                Metadata::new("Could not hash temporary object file").with("path", self.path.as_path())
            })?;
        self.finalize_object_at(id, file)
    }

    /// Publication failures include metadata `path` (native path), the object directory or destination file.
    fn finalize_object_at(&self, id: gix_hash::ObjectId, file: CompressedTempfile) -> Result<gix_hash::ObjectId, Exn> {
        let object_path = loose::hash_path(&id, self.path.clone());
        let object_dir = object_path
            .parent()
            .expect("each object path has a 1 hex-bytes directory");
        if let Err(err) = fs::create_dir(object_dir) {
            match err.kind() {
                io::ErrorKind::AlreadyExists => {}
                _ => {
                    return Err(err
                        .and_raise(Metadata::new("Could not create object directory").with("path", object_dir))
                        .erased());
                }
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
        res.or_raise_erased(|| Metadata::new("Could not persist loose object").with("path", object_path))?;
        Ok(id)
    }
}

/// Metadata `path` (native path) identifies the temporary object directory for a header-write failure.
fn write_header_error(path: &Path) -> Metadata {
    Metadata::new("Could not write loose object header").with("path", path)
}

/// Metadata `path` (native path) identifies the temporary object directory for a data-streaming failure.
fn stream_data_error(path: &Path) -> Metadata {
    Metadata::new("Could not stream loose object data").with("path", path)
}
