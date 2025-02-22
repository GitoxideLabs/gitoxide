use std::io::Write;

use gix_hash::ObjectId;

use crate::{data, data::output, find};

///
pub mod iter_from_counts;
pub use iter_from_counts::function::iter_from_counts;

/// The kind of pack entry to be written
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Kind {
    /// A complete base object, including its kind
    Base(gix_object::Kind),
    /// A delta against the object with the given index. It's always an index that was already encountered to refer only
    /// to object we have written already.
    DeltaRef {
        /// The absolute index to the object to serve as base. It's up to the writer to maintain enough state to allow producing
        /// a packed delta object from it.
        object_index: usize,
    },
    /// A delta against the given object as identified by its `ObjectId`.
    /// This is the case for thin packs only, i.e. those that are sent over the wire.
    /// Note that there is the option of the `ObjectId` being used to refer to an object within
    /// the same pack, but it's a discontinued practice which won't be encountered here.
    DeltaOid {
        /// The object serving as base for this delta
        id: ObjectId,
    },
}

/// The error returned by [`output::Entry::from_data()`].
#[allow(missing_docs)]
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    ZlibDeflate(#[from] std::io::Error),
    #[error(transparent)]
    EntryType(#[from] crate::data::entry::decode::Error),
}

impl output::Entry {
    /// An object which can be identified as invalid easily which happens if objects didn't exist even if they were referred to.
    pub fn invalid() -> output::Entry {
        output::Entry {
            id: gix_hash::Kind::Sha1.null(), // NOTE: the actual object hash used in the repo doesn't matter here, this is a sentinel value.
            kind: Kind::Base(gix_object::Kind::Blob),
            decompressed_size: 0,
            compressed_data: vec![],
        }
    }

    /// Returns true if this object doesn't really exist but still has to be handled responsibly
    ///
    /// Note that this is true for tree entries that are commits/git submodules, or for objects which aren't present in our local clone
    /// due to shallow clones.
    pub fn is_invalid(&self) -> bool {
        self.id.is_null()
    }

    /// Create an Entry from a previously counted object which is located in a pack. It's `entry` is provided here.
    /// The `version` specifies what kind of target `Entry` version the caller desires.
    pub fn from_pack_entry(
        mut entry: find::Entry,
        count: &output::Count,
        potential_bases: &[output::Count],
        bases_index_offset: usize,
        pack_offset_to_oid: Option<impl FnMut(u32, u64) -> Option<ObjectId>>,
        target_version: data::Version,
    ) -> Option<Result<Self, Error>> {
        if entry.version != target_version {
            return None;
        }

        let pack_offset_must_be_zero = 0;
        let pack_entry = match data::Entry::from_bytes(&entry.data, pack_offset_must_be_zero, count.id.as_slice().len())
        {
            Ok(e) => e,
            Err(err) => return Some(Err(err.into())),
        };

        use crate::data::entry::Header::*;
        match pack_entry.header {
            Commit => Some(output::entry::Kind::Base(gix_object::Kind::Commit)),
            Tree => Some(output::entry::Kind::Base(gix_object::Kind::Tree)),
            Blob => Some(output::entry::Kind::Base(gix_object::Kind::Blob)),
            Tag => Some(output::entry::Kind::Base(gix_object::Kind::Tag)),
            OfsDelta { base_distance } => {
                let pack_location = count.entry_pack_location.as_ref().expect("packed");
                let base_offset = pack_location
                    .pack_offset
                    .checked_sub(base_distance)
                    .expect("pack-offset - distance is firmly within the pack");
                potential_bases
                    .binary_search_by(|e| {
                        e.entry_pack_location
                            .as_ref()
                            .expect("packed")
                            .pack_offset
                            .cmp(&base_offset)
                    })
                    .ok()
                    .map(|idx| output::entry::Kind::DeltaRef {
                        object_index: idx + bases_index_offset,
                    })
                    .or_else(|| {
                        pack_offset_to_oid
                            .and_then(|mut f| f(pack_location.pack_id, base_offset))
                            .map(|id| output::entry::Kind::DeltaOid { id })
                    })
            }
            RefDelta { base_id: _ } => None, // ref deltas are for thin packs or legacy, repack them as base objects
        }
        .map(|kind| {
            Ok(output::Entry {
                id: count.id.to_owned(),
                kind,
                decompressed_size: pack_entry.decompressed_size as usize,
                compressed_data: {
                    entry.data.copy_within(pack_entry.data_offset as usize.., 0);
                    entry.data.resize(
                        entry.data.len()
                            - usize::try_from(pack_entry.data_offset).expect("offset representable as usize"),
                        0,
                    );
                    entry.data
                },
            })
        })
    }

    /// Create a new instance from the given `oid` and its corresponding git object data `obj`.
    pub fn from_data(count: &output::Count, obj: &gix_object::Data<'_>) -> Result<Self, Error> {
        Ok(output::Entry {
            id: count.id.to_owned(),
            kind: Kind::Base(obj.kind),
            decompressed_size: obj.data.len(),
            compressed_data: {
                let mut out = gix_features::zlib::stream::deflate::Write::new(Vec::new());
                if let Err(err) = std::io::copy(&mut &*obj.data, &mut out) {
                    match err.kind() {
                        std::io::ErrorKind::Other => return Err(Error::ZlibDeflate(err)),
                        err => unreachable!("Should never see other errors than zlib, but got {:?}", err,),
                    }
                }
                out.flush()?;
                out.into_inner()
            },
        })
    }

    /// Transform ourselves into pack entry header of `version` which can be written into a pack.
    ///
    /// `index_to_pack(object_index) -> pack_offset` is a function to convert the base object's index into
    /// the input object array (if each object is numbered) to an offset into the pack.
    /// This information is known to the one calling the method.
    pub fn to_entry_header(
        &self,
        version: data::Version,
        index_to_base_distance: impl FnOnce(usize) -> u64,
    ) -> data::entry::Header {
        assert!(
            matches!(version, data::Version::V2),
            "we can only write V2 pack entries for now"
        );

        use Kind::*;
        match self.kind {
            Base(kind) => {
                use gix_object::Kind::*;
                match kind {
                    Tree => data::entry::Header::Tree,
                    Blob => data::entry::Header::Blob,
                    Commit => data::entry::Header::Commit,
                    Tag => data::entry::Header::Tag,
                }
            }
            DeltaOid { id } => data::entry::Header::RefDelta { base_id: id.to_owned() },
            DeltaRef { object_index } => data::entry::Header::OfsDelta {
                base_distance: index_to_base_distance(object_index),
            },
        }
    }
}
