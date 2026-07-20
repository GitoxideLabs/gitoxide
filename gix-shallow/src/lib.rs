//! [Read](read()) and [write](write()) shallow files, while performing typical operations on them.
//!
//! ## Examples
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let first = gix_hash::ObjectId::from_hex(b"1111111111111111111111111111111111111111")?;
//! let second = gix_hash::ObjectId::from_hex(b"2222222222222222222222222222222222222222")?;
//! # let dir = tempfile::tempdir()?;
//! # let shallow_file = dir.path().join("shallow");
//! # std::fs::write(&shallow_file, format!("{first}\n"))?;
//!
//! let shallow = gix_shallow::read(&shallow_file)
//!     .map_err(|err| err.into_error())?
//!     .expect("a shallow boundary");
//! let lock = gix_lock::File::acquire_to_update_resource(
//!     &shallow_file,
//!     gix_lock::acquire::Fail::Immediately,
//!     None,
//! )
//! .map_err(|err| err.into_error())?;
//! gix_shallow::write(lock, Some(shallow), &[gix_shallow::Update::Shallow(second)]).map_err(|err| err.into_error())?;
//!
//! let ids = gix_shallow::read(&shallow_file)
//!     .map_err(|err| err.into_error())?
//!     .unwrap()
//!     .into_iter()
//!     .collect::<Vec<_>>();
//! assert_eq!(ids, vec![first, second]);
//! # Ok(()) }
//! ```
#![deny(missing_docs)]
#![forbid(unsafe_code)]

/// An instruction on how to
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Update {
    /// Shallow the given `id`.
    Shallow(gix_hash::ObjectId),
    /// Don't shallow the given `id` anymore.
    Unshallow(gix_hash::ObjectId),
}

/// Return a list of shallow commits as unconditionally read from `shallow_file`.
///
/// The list of shallow commits represents the shallow boundary, beyond which we are lacking all (parent) commits.
/// Note that the list is never empty, as `Ok(None)` is returned in that case indicating the repository
/// isn't a shallow clone.
// TODO(review): through still-unconverted `thiserror` wrappers (e.g. `gix_protocol::fetch::Error`),
//                `source()` of these errors reaches the `ValidationError`/`Message` whose source is
//                `None`, so the underlying io/decode causes are missing from `std` error chains on
//                that path until consumers are converted. They remain visible in the `Exn` tree and
//                at erased boundaries.
pub fn read(shallow_file: &std::path::Path) -> Result<Option<nonempty::NonEmpty<gix_hash::ObjectId>>, read::Error> {
    use bstr::ByteSlice;
    use gix_error::{ResultExt, ValidationError};

    let buf = match std::fs::read(shallow_file) {
        Ok(buf) => buf,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => Err(err).or_raise(|| ValidationError::new("Could not open shallow file for reading"))?,
    };

    let mut commits = buf
        .lines()
        .map(|line| {
            gix_hash::ObjectId::from_hex(line).or_raise(|| {
                ValidationError::new_with_input(
                    "Could not decode a line in shallow file as hex-encoded object hash",
                    line,
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    commits.sort();
    Ok(nonempty::NonEmpty::from_vec(commits))
}

///
pub mod write {
    pub(crate) mod function {
        use std::io::Write;

        use gix_error::{ResultExt, message};

        use super::Error;
        use crate::Update;

        /// Write the [previously obtained](crate::read()) (possibly non-existing) `shallow_commits` to the shallow `file`
        /// after applying all `updates`.
        ///
        /// If this leaves the list of shallow commits empty, the file is removed.
        ///
        /// ### Deviation
        ///
        /// Git also prunes the set of shallow commits while writing, we don't until we support some sort of pruning.
        // TODO(review): the same `std` error chain gap as noted on `read()` applies here, for the
        //                io and lock-commit causes.
        pub fn write(
            mut file: gix_lock::File,
            shallow_commits: Option<nonempty::NonEmpty<gix_hash::ObjectId>>,
            updates: &[Update],
        ) -> Result<(), Error> {
            let mut shallow_commits = shallow_commits.map(Vec::from).unwrap_or_default();
            for update in updates {
                match update {
                    Update::Shallow(id) => {
                        shallow_commits.push(*id);
                    }
                    Update::Unshallow(id) => shallow_commits.retain(|oid| oid != id),
                }
            }
            if shallow_commits.is_empty() {
                if let Err(err) = std::fs::remove_file(file.resource_path()) {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        return Err(err).or_raise(|| message("Could not remove an empty shallow file"));
                    }
                }
                drop(file);
                return Ok(());
            }
            shallow_commits.sort();
            let mut buf = Vec::<u8>::new();
            for commit in shallow_commits {
                commit
                    .write_hex_to(&mut buf)
                    .or_raise(|| message("Failed to write object id to shallow file"))?;
                buf.push(b'\n');
            }
            file.write_all(&buf)
                .or_raise(|| message("Failed to write object id to shallow file"))?;
            file.flush()
                .or_raise(|| message("Failed to write object id to shallow file"))?;
            file.commit()
                .or_raise(|| message("Could not commit changes to the shallow file"))?;
            Ok(())
        }
    }

    /// The error returned by [`write()`](crate::write()).
    // TODO(review): as an `Exn`, this no longer implements `std::error::Error` — out-of-tree callers
    //               that propagated it into `Box<dyn Error>` or `anyhow` need `.into_error()` now.
    pub type Error = gix_error::Exn<gix_error::Message>;
}
pub use write::function::write;

///
pub mod read {
    /// The error returned by [`read`](crate::read()).
    // TODO(review): as an `Exn`, this no longer implements `std::error::Error` — out-of-tree callers
    //               that propagated it into `Box<dyn Error>` or `anyhow` need `.into_error()` now.
    pub type Error = gix_error::Exn<gix_error::ValidationError>;
}
