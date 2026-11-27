//! [Read](read()) and [write](write()) shallow files, while performing typical operations on them.
//!
//! ## Examples
//!
//! ```
//! # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! let first = gix_hash::ObjectId::from_hex(b"1111111111111111111111111111111111111111")?;
//! let second = gix_hash::ObjectId::from_hex(b"2222222222222222222222222222222222222222")?;
//! # let dir = tempfile::tempdir()?;
//! # let shallow_file = dir.path().join("shallow");
//! # std::fs::write(&shallow_file, format!("{first}\n"))?;
//!
//! let shallow = gix_shallow::read(&shallow_file, first.kind())
//!     ?
//!     .expect("a shallow boundary");
//! let lock = gix_lock::File::acquire_to_update_resource(
//!     &shallow_file,
//!     gix_lock::acquire::Fail::Immediately,
//!     None,
//!     Default::default(),
//! )
//! ?;
//! gix_shallow::write(lock, Some(shallow), &[gix_shallow::Update::Shallow(second)])
//!     ?;
//!
//! let ids = gix_shallow::read(&shallow_file, first.kind())
//!     ?
//!     .expect("a shallow boundary")
//!     .into_iter()
//!     .collect::<Vec<_>>();
//! assert_eq!(ids, vec![first, second]);
//! # Ok(()) }
//! ```
#![deny(missing_docs)]
#![forbid(unsafe_code)]

use gix_error::Result;
/// An instruction on how to
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Update {
    /// Shallow the given `id`.
    Shallow(gix_hash::ObjectId),
    /// Don't shallow the given `id` anymore.
    Unshallow(gix_hash::ObjectId),
}

/// Return a sorted list of shallow commits as unconditionally read from `shallow_file`.
///
/// Like Git, stream records of up to 1023 bytes, stopping at and including a newline if it fits.
/// Decode the first [`object_hash.len_in_hex()`][gix_hash::Kind::len_in_hex] bytes of each record
/// and ignore trailing bytes. `object_hash` is the repository's object format.
///
/// The list of shallow commits represents the shallow boundary, beyond which we are lacking all (parent) commits.
/// Note that the list is never empty, as `Ok(None)` is returned in that case indicating the repository
/// isn't a shallow clone.
pub fn read(
    shallow_file: &std::path::Path,
    object_hash: gix_hash::Kind,
) -> Result<Option<nonempty::NonEmpty<gix_hash::ObjectId>>> {
    use std::io::{BufRead, BufReader, Read};

    use gix_error::{ErrorExt, OptionExt, ResultExt, bail, corruption, message};

    let file = match std::fs::File::open(shallow_file) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            bail!(err.and_raise(message("Could not open shallow file for reading")));
        }
    };

    // Git's 1024-byte fgets() buffer reserves one byte for the NUL terminator.
    const MAX_RECORD_LEN: usize = 1023;
    let mut reader = BufReader::new(file);
    let mut record = Vec::with_capacity(MAX_RECORD_LEN);
    let hex_len = object_hash.len_in_hex();
    let mut commits = Vec::new();
    loop {
        record.clear();
        if (&mut reader)
            .take(MAX_RECORD_LEN as u64)
            .read_until(b'\n', &mut record)
            .or_raise(|| message("Could not read shallow file"))?
            == 0
        {
            break;
        }
        let commit_id = record
            .get(..hex_len)
            .ok_or_raise(|| message!("Expected {hex_len} hexadecimal characters in the shallow record"))
            .and_then(gix_hash::ObjectId::from_hex)
            .or_raise(|| corruption("Could not decode a line in shallow file as hex-encoded object hash"))?;
        commits.push(commit_id);
    }

    commits.sort();
    Ok(nonempty::NonEmpty::from_vec(commits))
}

///
pub mod write {
    pub(crate) mod function {
        use gix_error::Result;
        use std::io::Write;

        use gix_error::{ErrorExt, ResultExt, bail, message};

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
        ) -> Result {
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
                if let Err(err) = std::fs::remove_file(file.resource_path())
                    && err.kind() != std::io::ErrorKind::NotFound
                {
                    bail!(err.and_raise(message("Could not remove an empty shallow file")));
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
            file.commit().or_raise(|| message("Could not commit shallow file"))?;
            Ok(())
        }
    }
}
pub use write::function::write;
