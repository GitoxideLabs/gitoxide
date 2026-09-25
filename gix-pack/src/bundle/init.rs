use gix_error::Result;
use gix_error::ResultExt;
use std::path::Path;

use gix_error::{ErrorExt, ExnResult, OptionExt};

use crate::Bundle;

/// Initialization
impl Bundle {
    /// Create a `Bundle` from `path`, which is either a pack file _(*.pack)_ or an index file _(*.idx)_.
    ///
    /// The corresponding complementary file is expected to be present.
    ///
    /// The `object_hash` is a way to read (and write) the same file format with different hashes, as the hash kind
    /// isn't stored within the file format itself.
    pub fn at(path: impl AsRef<Path>, object_hash: gix_hash::Kind) -> Result<Self> {
        (Self::at_inner(path.as_ref(), object_hash)).map_err(Into::into)
    }

    fn at_inner(path: &Path, object_hash: gix_hash::Kind) -> ExnResult<Self> {
        let ext = path
            .extension()
            .and_then(std::ffi::OsStr::to_str)
            .ok_or_raise_erased(|| {
                gix_error::validation(format!(
                    "An 'idx' extension is expected of an index file: '{}'",
                    path.display()
                ))
            })?;
        Ok(match ext {
            "idx" => Self {
                index: crate::index::File::at(path, object_hash).or_erased()?,
                pack: crate::data::File::at(path.with_extension("pack"), object_hash).or_erased()?,
            },
            "pack" => Self {
                pack: crate::data::File::at(path, object_hash).or_erased()?,
                index: crate::index::File::at(path.with_extension("idx"), object_hash).or_erased()?,
            },
            _ => {
                return Err(gix_error::validation(format!(
                    "An 'idx' extension is expected of an index file: '{}'",
                    path.display()
                ))
                .raise_erased());
            }
        })
    }
}
