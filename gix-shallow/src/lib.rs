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
//! let shallow = gix_shallow::read(&shallow_file)?.expect("a shallow boundary");
//! let lock = gix_lock::File::acquire_to_update_resource(
//!     &shallow_file,
//!     gix_lock::acquire::Fail::Immediately,
//!     None,
//! )?;
//! gix_shallow::write(lock, Some(shallow), &[gix_shallow::Update::Shallow(second)])?;
//!
//! let ids = gix_shallow::read(&shallow_file)?.unwrap().into_iter().collect::<Vec<_>>();
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
pub fn read(shallow_file: &std::path::Path) -> Result<Option<nonempty::NonEmpty<gix_hash::ObjectId>>, read::Error> {
    use bstr::ByteSlice;
    let buf = match std::fs::read(shallow_file) {
        Ok(buf) => buf,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    let mut commits = buf
        .lines()
        .map(gix_hash::ObjectId::from_hex)
        .collect::<Result<Vec<_>, _>>()?;

    commits.sort();
    Ok(nonempty::NonEmpty::from_vec(commits))
}

///
pub mod write {
    pub(crate) mod function {
        use std::io::Write;

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
                        return Err(err.into());
                    }
                }
                drop(file);
                return Ok(());
            }
            shallow_commits.sort();
            let mut buf = Vec::<u8>::new();
            for commit in shallow_commits {
                commit.write_hex_to(&mut buf).map_err(Error::Io)?;
                buf.push(b'\n');
            }
            file.write_all(&buf).map_err(Error::Io)?;
            file.flush().map_err(Error::Io)?;
            file.commit()?;
            Ok(())
        }
    }

    /// The error returned by [`write()`](crate::write()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Commit(#[from] gix_lock::commit::Error<gix_lock::File>),
        #[error("Could not remove an empty shallow file")]
        RemoveEmpty(#[from] std::io::Error),
        #[error("Failed to write object id to shallow file")]
        Io(std::io::Error),
    }
}
pub use write::function::write;

///
pub mod read {
    /// The error returned by [`read`](crate::read()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Could not open shallow file for reading")]
        Io(#[from] std::io::Error),
        #[error("Could not decode a line in shallow file as hex-encoded object hash")]
        DecodeHash(#[from] gix_hash::decode::Error),
    }
}
