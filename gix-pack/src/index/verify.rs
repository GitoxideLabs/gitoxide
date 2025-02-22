use std::sync::atomic::AtomicBool;

use gix_features::progress::{DynNestedProgress, Progress};
use gix_object::WriteTo;

use crate::index;

///
pub mod integrity {
    use std::marker::PhantomData;

    use gix_object::bstr::BString;

    /// Returned by [`index::File::verify_integrity()`][crate::index::File::verify_integrity()].
    #[derive(thiserror::Error, Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Reserialization of an object failed")]
        Io(#[from] std::io::Error),
        #[error("The fan at index {index} is out of order as it's larger then the following value.")]
        Fan { index: usize },
        #[error("{kind} object {id} could not be decoded")]
        ObjectDecode {
            source: gix_object::decode::Error,
            kind: gix_object::Kind,
            id: gix_hash::ObjectId,
        },
        #[error("{kind} object {id} wasn't re-encoded without change, wanted\n{expected}\n\nGOT\n\n{actual}")]
        ObjectEncodeMismatch {
            kind: gix_object::Kind,
            id: gix_hash::ObjectId,
            expected: BString,
            actual: BString,
        },
    }

    /// Returned by [`index::File::verify_integrity()`][crate::index::File::verify_integrity()].
    pub struct Outcome {
        /// The computed checksum of the index which matched the stored one.
        pub actual_index_checksum: gix_hash::ObjectId,
        /// The packs traversal outcome, if one was provided
        pub pack_traverse_statistics: Option<crate::index::traverse::Statistics>,
    }

    /// Additional options to define how the integrity should be verified.
    #[derive(Clone)]
    pub struct Options<F> {
        /// The thoroughness of the verification
        pub verify_mode: crate::index::verify::Mode,
        /// The way to traverse packs
        pub traversal: crate::index::traverse::Algorithm,
        /// The amount of threads to use of `Some(N)`, with `None|Some(0)` using all available cores are used.
        pub thread_limit: Option<usize>,
        /// A function to create a pack cache
        pub make_pack_lookup_cache: F,
    }

    impl Default for Options<fn() -> crate::cache::Never> {
        fn default() -> Self {
            Options {
                verify_mode: Default::default(),
                traversal: Default::default(),
                thread_limit: None,
                make_pack_lookup_cache: || crate::cache::Never,
            }
        }
    }

    /// The progress ids used in [`index::File::verify_integrity()`][crate::index::File::verify_integrity()].
    ///
    /// Use this information to selectively extract the progress of interest in case the parent application has custom visualization.
    #[derive(Debug, Copy, Clone)]
    pub enum ProgressId {
        /// The amount of bytes read to verify the index checksum.
        ChecksumBytes,
        /// A root progress for traversal which isn't actually used directly, but here to link to the respective `ProgressId` types.
        Traverse(PhantomData<crate::index::verify::index::traverse::ProgressId>),
    }

    impl From<ProgressId> for gix_features::progress::Id {
        fn from(v: ProgressId) -> Self {
            match v {
                ProgressId::ChecksumBytes => *b"PTHI",
                ProgressId::Traverse(_) => gix_features::progress::UNKNOWN,
            }
        }
    }
}

///
pub mod checksum {
    /// Returned by [`index::File::verify_checksum()`][crate::index::File::verify_checksum()].
    pub type Error = crate::verify::checksum::Error;
}

/// Various ways in which a pack and index can be verified
#[derive(Default, Debug, Eq, PartialEq, Hash, Clone, Copy)]
pub enum Mode {
    /// Validate the object hash and CRC32
    HashCrc32,
    /// Validate hash and CRC32, and decode each non-Blob object.
    /// Each object should be valid, i.e. be decodable.
    HashCrc32Decode,
    /// Validate hash and CRC32, and decode and encode each non-Blob object.
    /// Each object should yield exactly the same hash when re-encoded.
    #[default]
    HashCrc32DecodeEncode,
}

/// Information to allow verifying the integrity of an index with the help of its corresponding pack.
pub struct PackContext<'a, F> {
    /// The pack data file itself.
    pub data: &'a crate::data::File,
    /// The options further configuring the pack traversal and verification
    pub options: integrity::Options<F>,
}

/// Verify and validate the content of the index file
impl index::File {
    /// Returns the trailing hash stored at the end of this index file.
    ///
    /// It's a hash over all bytes of the index.
    pub fn index_checksum(&self) -> gix_hash::ObjectId {
        gix_hash::ObjectId::from_bytes_or_panic(&self.data[self.data.len() - self.hash_len..])
    }

    /// Returns the hash of the pack data file that this index file corresponds to.
    ///
    /// It should [`crate::data::File::checksum()`] of the corresponding pack data file.
    pub fn pack_checksum(&self) -> gix_hash::ObjectId {
        let from = self.data.len() - self.hash_len * 2;
        gix_hash::ObjectId::from_bytes_or_panic(&self.data[from..][..self.hash_len])
    }

    /// Validate that our [`index_checksum()`][index::File::index_checksum()] matches the actual contents
    /// of this index file, and return it if it does.
    pub fn verify_checksum(
        &self,
        progress: &mut dyn Progress,
        should_interrupt: &AtomicBool,
    ) -> Result<gix_hash::ObjectId, checksum::Error> {
        crate::verify::checksum_on_disk_or_mmap(
            self.path(),
            &self.data,
            self.index_checksum(),
            self.object_hash,
            progress,
            should_interrupt,
        )
    }

    /// The most thorough validation of integrity of both index file and the corresponding pack data file, if provided.
    /// Returns the checksum of the index file, the traversal outcome and the given progress if the integrity check is successful.
    ///
    /// If `pack` is provided, it is expected (and validated to be) the pack belonging to this index.
    /// It will be used to validate internal integrity of the pack before checking each objects integrity
    /// is indeed as advertised via its SHA1 as stored in this index, as well as the CRC32 hash.
    /// The last member of the Option is a function returning an implementation of [`crate::cache::DecodeEntry`] to be used if
    /// the [`index::traverse::Algorithm`] is `Lookup`.
    /// To set this to `None`, use `None::<(_, _, _, fn() -> crate::cache::Never)>`.
    ///
    /// The `thread_limit` optionally specifies the amount of threads to be used for the [pack traversal][index::File::traverse()].
    /// `make_cache` is only used in case a `pack` is specified, use existing implementations in the [`crate::cache`] module.
    ///
    /// # Tradeoffs
    ///
    /// The given `progress` is inevitably consumed if there is an error, which is a tradeoff chosen to easily allow using `?` in the
    /// error case.
    pub fn verify_integrity<C, F>(
        &self,
        pack: Option<PackContext<'_, F>>,
        progress: &mut dyn DynNestedProgress,
        should_interrupt: &AtomicBool,
    ) -> Result<integrity::Outcome, index::traverse::Error<index::verify::integrity::Error>>
    where
        C: crate::cache::DecodeEntry,
        F: Fn() -> C + Send + Clone,
    {
        if let Some(first_invalid) = crate::verify::fan(&self.fan) {
            return Err(index::traverse::Error::Processor(integrity::Error::Fan {
                index: first_invalid,
            }));
        }

        match pack {
            Some(PackContext {
                data: pack,
                options:
                    integrity::Options {
                        verify_mode,
                        traversal,
                        thread_limit,
                        make_pack_lookup_cache,
                    },
            }) => self
                .traverse(
                    pack,
                    progress,
                    should_interrupt,
                    {
                        let mut encode_buf = Vec::with_capacity(2048);
                        move |kind, data, index_entry, progress| {
                            Self::verify_entry(verify_mode, &mut encode_buf, kind, data, index_entry, progress)
                        }
                    },
                    index::traverse::Options {
                        traversal,
                        thread_limit,
                        check: index::traverse::SafetyCheck::All,
                        make_pack_lookup_cache,
                    },
                )
                .map(|o| integrity::Outcome {
                    actual_index_checksum: o.actual_index_checksum,
                    pack_traverse_statistics: Some(o.statistics),
                }),
            None => self
                .verify_checksum(
                    &mut progress
                        .add_child_with_id("Sha1 of index".into(), integrity::ProgressId::ChecksumBytes.into()),
                    should_interrupt,
                )
                .map_err(Into::into)
                .map(|id| integrity::Outcome {
                    actual_index_checksum: id,
                    pack_traverse_statistics: None,
                }),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn verify_entry(
        verify_mode: Mode,
        encode_buf: &mut Vec<u8>,
        object_kind: gix_object::Kind,
        buf: &[u8],
        index_entry: &index::Entry,
        _progress: &dyn gix_features::progress::Progress,
    ) -> Result<(), integrity::Error> {
        if let Mode::HashCrc32Decode | Mode::HashCrc32DecodeEncode = verify_mode {
            use gix_object::Kind::*;
            match object_kind {
                Tree | Commit | Tag => {
                    let object = gix_object::ObjectRef::from_bytes(object_kind, buf).map_err(|err| {
                        integrity::Error::ObjectDecode {
                            source: err,
                            kind: object_kind,
                            id: index_entry.oid,
                        }
                    })?;
                    if let Mode::HashCrc32DecodeEncode = verify_mode {
                        encode_buf.clear();
                        object.write_to(&mut *encode_buf)?;
                        if encode_buf.as_slice() != buf {
                            return Err(integrity::Error::ObjectEncodeMismatch {
                                kind: object_kind,
                                id: index_entry.oid,
                                expected: buf.into(),
                                actual: encode_buf.clone().into(),
                            });
                        }
                    }
                }
                Blob => {}
            }
        }
        Ok(())
    }
}
