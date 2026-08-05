use std::path::{Path, PathBuf};

use crate::Bundle;

/// Returned by [`Bundle::at()`]
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    InvalidPath(PathBuf),
    Pack(crate::data::header::decode::Error),
    Index(crate::index::init::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidPath(path) => {
                write!(
                    f,
                    "An 'idx' extension is expected of an index file: '{}'",
                    path.display()
                )
            }
            Error::Pack(err) => std::fmt::Display::fmt(err, f),
            Error::Index(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Pack(err) => err.source(),
            Error::Index(err) => err.source(),
            Error::InvalidPath(_) => None,
        }
    }
}

impl From<crate::data::header::decode::Error> for Error {
    fn from(err: crate::data::header::decode::Error) -> Self {
        Error::Pack(err)
    }
}

impl From<crate::index::init::Error> for Error {
    fn from(err: crate::index::init::Error) -> Self {
        Error::Index(err)
    }
}

/// Initialization
impl Bundle {
    /// Create a `Bundle` from `path`, which is either a pack file _(*.pack)_ or an index file _(*.idx)_.
    ///
    /// The corresponding complementary file is expected to be present.
    ///
    /// The `object_hash` is a way to read (and write) the same file format with different hashes, as the hash kind
    /// isn't stored within the file format itself.
    pub fn at(path: impl AsRef<Path>, object_hash: gix_hash::Kind) -> Result<Self, Error> {
        Self::at_inner(path.as_ref(), object_hash)
    }

    fn at_inner(path: &Path, object_hash: gix_hash::Kind) -> Result<Self, Error> {
        let ext = path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .ok_or_else(|| Error::InvalidPath(path.to_owned()))?;
        Ok(match ext {
            "idx" => Self {
                index: crate::index::File::at(path, object_hash)?,
                pack: crate::data::File::at(path.with_extension("pack"), object_hash)?,
            },
            "pack" => Self {
                pack: crate::data::File::at(path, object_hash)?,
                index: crate::index::File::at(path.with_extension("idx"), object_hash)?,
            },
            _ => return Err(Error::InvalidPath(path.to_owned())),
        })
    }
}
