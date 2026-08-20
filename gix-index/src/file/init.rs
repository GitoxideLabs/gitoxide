#![allow(unused)]

use std::path::{Path, PathBuf};

use gix_error::{ResultExt, message};

use crate::{File, State, decode, extension};

/// The error returned by [File::at()][File::at()].
pub type Error = gix_error::Exn;

/// Initialization
impl File {
    /// Try to open the index file at `path` with `options`, assuming `object_hash` is used throughout the file, or create a new
    /// index that merely exists in memory and is empty. `skip_hash` will increase the performance by a factor of 2, at the cost of
    /// possibly not detecting corruption.
    ///
    /// Note that the `path` will not be written if it doesn't exist.
    pub fn at_or_default(
        path: impl Into<PathBuf>,
        object_hash: gix_hash::Kind,
        skip_hash: bool,
        options: decode::Options,
    ) -> Result<Self, Error> {
        let path = path.into();
        Ok(match Self::at(&path, object_hash, skip_hash, options) {
            Ok(f) => f,
            Err(err)
                if err
                    .downcast_any_ref::<std::io::Error>()
                    .is_some_and(|err| err.kind() == std::io::ErrorKind::NotFound) =>
            {
                File::from_state(State::new(object_hash), path)
            }
            Err(err) => return Err(err),
        })
    }

    /// Open an index file at `path` with `options`, assuming `object_hash` is used throughout the file. If `skip_hash` is `true`,
    /// we will not get or compare the checksum of the index at all, which generally increases performance of this method by a factor
    /// of 2 or more.
    ///
    /// Note that the verification of the file hash depends on `options`, and even then it's performed after the file was read and not
    /// before it is read. That way, invalid files would see a more descriptive error message as we try to parse them.
    pub fn at(
        path: impl Into<PathBuf>,
        object_hash: gix_hash::Kind,
        skip_hash: bool,
        options: decode::Options,
    ) -> Result<Self, Error> {
        let _span = gix_features::trace::detail!("gix_index::File::at()");
        let path = path.into();
        let (data, mtime) = {
            let mut file = std::fs::File::open(&path)
                .or_raise_erased(|| message("An IO error occurred while opening the index"))?;
            // SAFETY: we have to take the risk of somebody changing the file underneath. Git never writes into the same file.
            #[expect(unsafe_code)]
            let data = unsafe { memmap2::MmapOptions::new().map_copy_read_only(&file) }
                .or_raise_erased(|| message("An IO error occurred while opening the index"))?;

            if !skip_hash {
                // Note that even though it's trivial to offload this into a thread, which is worth it for all but the smallest
                // index files, we choose more safety here just like git does and don't even try to decode the index if the hashes
                // don't match.
                // Thanks to `skip_hash`, we can get performance and it's under caller control, at the cost of some safety.
                let expected =
                    gix_hash::ObjectId::from_bytes_or_panic(&data[data.len() - object_hash.len_in_bytes()..]);
                if !expected.is_null() {
                    let _span = gix_features::trace::detail!("gix::open_index::hash_index", path = ?path);
                    let meta = file
                        .metadata()
                        .or_raise_erased(|| message("An IO error occurred while opening the index"))?;
                    let num_bytes_to_hash = meta.len() - object_hash.len_in_bytes() as u64;
                    gix_hash::bytes(
                        &mut file,
                        num_bytes_to_hash,
                        object_hash,
                        &mut gix_features::progress::Discard,
                        &Default::default(),
                    )
                    .or_raise_erased(|| message("Could not hash index data"))?
                    .verify(&expected)
                    .or_raise_erased(|| message("Shared index checksum mismatch"))?;
                }
            }

            (
                data,
                filetime::FileTime::from_last_modification_time(
                    &file
                        .metadata()
                        .or_raise_erased(|| message("An IO error occurred while opening the index"))?,
                ),
            )
        };

        let (state, checksum) = State::from_bytes(&data, mtime, object_hash, options)?;
        let mut file = File { state, path, checksum };
        if let Some(mut link) = file.link.take() {
            link.dissolve_into(&mut file, object_hash, skip_hash, options)?;
        }

        Ok(file)
    }

    /// Consume `state` and pretend it was read from `path`, setting our checksum to `null`.
    ///
    /// `File` instances created like that should be written to disk to set the correct checksum via `[File::write()]`.
    pub fn from_state(state: State, path: impl Into<PathBuf>) -> Self {
        File {
            state,
            path: path.into(),
            checksum: None,
        }
    }
}
