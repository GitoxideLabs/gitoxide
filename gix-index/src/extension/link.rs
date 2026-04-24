use crate::extension::{Link, Signature};

/// The signature of the link extension.
pub const SIGNATURE: Signature = *b"link";

/// Bitmaps to know which entries to delete or replace, even though details are still unknown.
#[derive(Clone)]
pub struct Bitmaps {
    /// A bitmap to signal which entries to delete, maybe.
    pub delete: gix_bitmap::ewah::Vec,
    /// A bitmap to signal which entries to replace, maybe.
    pub replace: gix_bitmap::ewah::Vec,
}

///
pub mod decode {

    /// The error returned when decoding link extensions.
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("{0}")]
        Corrupt(&'static str),
        #[error("{kind} bitmap corrupt")]
        BitmapDecode {
            err: gix_bitmap::ewah::decode::Error,
            kind: &'static str,
        },
    }

    impl From<std::num::TryFromIntError> for Error {
        fn from(_: std::num::TryFromIntError) -> Self {
            Self::Corrupt("error in bitmap iteration trying to convert from u64 to usize")
        }
    }
}

pub(crate) fn decode(data: &[u8], object_hash: gix_hash::Kind) -> Result<Link, decode::Error> {
    let (id, data) = data
        .split_at_checked(object_hash.len_in_bytes())
        .ok_or(decode::Error::Corrupt(
            "link extension too short to read share index checksum",
        ))
        .map(|(id, d)| (gix_hash::ObjectId::from_bytes_or_panic(id), d))?;

    if data.is_empty() {
        return Ok(Link {
            shared_index_checksum: id,
            bitmaps: None,
        });
    }

    let (delete, data) =
        gix_bitmap::ewah::decode(data).map_err(|err| decode::Error::BitmapDecode { kind: "delete", err })?;
    let (replace, data) =
        gix_bitmap::ewah::decode(data).map_err(|err| decode::Error::BitmapDecode { kind: "replace", err })?;

    if !data.is_empty() {
        return Err(decode::Error::Corrupt("garbage trailing link extension"));
    }

    Ok(Link {
        shared_index_checksum: id,
        bitmaps: Some(Bitmaps { delete, replace }),
    })
}

impl Link {
    pub(crate) fn dissolve_into(
        self,
        split_index: &mut crate::File,
        object_hash: gix_hash::Kind,
        skip_hash: bool,
        options: crate::decode::Options,
    ) -> Result<(), crate::file::init::Error> {
        let shared_index_path = split_index
            .path
            .parent()
            .expect("split index file in .git folder")
            .join(format!("sharedindex.{}", self.shared_index_checksum));
        let mut shared_index = crate::File::at(
            shared_index_path,
            object_hash,
            skip_hash,
            crate::decode::Options {
                expected_checksum: self.shared_index_checksum.into(),
                ..options
            },
        )?;

        if let Some(bitmaps) = self.bitmaps {
            let mut split_entry_index = 0;

            let mut err = None;
            if bitmaps.replace.for_each_set_bit(|replace_index| {
                let shared_entry = match shared_index.entries.get_mut(replace_index) {
                    Some(e) => e,
                    None => {
                        err = decode::Error::Corrupt("replace bitmap length exceeds shared index length - more entries in bitmap than found in shared index").into();
                        return None
                    }
                };

                if shared_entry.flags.contains(crate::entry::Flags::REMOVE) {
                    err = decode::Error::Corrupt("entry is marked as both replace and delete").into();
                    return None
                }

                let split_entry = match split_index.entries.get(split_entry_index) {
                    Some(e) => e,
                    None => {
                        err = decode::Error::Corrupt("replace bitmap length exceeds split index length - more entries in bitmap than found in split index").into();
                        return None
                    }
                };
                if !split_entry.path.is_empty() {
                    err = decode::Error::Corrupt("paths in split index entries that are for replacement should be empty").into();
                    return None
                }
                if shared_entry.path.is_empty() {
                    err = decode::Error::Corrupt("paths in shared index entries that are replaced should not be empty").into();
                    return None
                }
                shared_entry.stat = split_entry.stat;
                shared_entry.id = split_entry.id;
                shared_entry.flags = split_entry.flags;
                shared_entry.mode = split_entry.mode;

                split_entry_index += 1;
                Some(())
            }).is_none() && err.is_none() {
                err = decode::Error::Corrupt("replace bitmap is malformed").into();
            }
            if let Some(err) = err {
                return Err(err.into());
            }

            let split_index_path_backing = std::mem::take(&mut split_index.path_backing);
            for mut split_entry in split_index.entries.drain(split_entry_index..) {
                let start = shared_index.path_backing.len();
                let split_index_path = split_entry.path.clone();

                split_entry.path = start..start + split_entry.path.len();
                shared_index.entries.push(split_entry);

                shared_index
                    .path_backing
                    .extend_from_slice(&split_index_path_backing[split_index_path]);
            }

            if bitmaps.delete.for_each_set_bit(|delete_index| {
                let shared_entry = match shared_index.entries.get_mut(delete_index) {
                    Some(e) => e,
                    None => {
                        err = decode::Error::Corrupt("delete bitmap length exceeds shared index length - more entries in bitmap than found in shared index").into();
                        return None
                    }
                };
                shared_entry.flags.insert(crate::entry::Flags::REMOVE);
                Some(())
            }).is_none() && err.is_none() {
                err = decode::Error::Corrupt("delete bitmap is malformed").into();
            }
            if let Some(err) = err {
                return Err(err.into());
            }

            shared_index
                .entries
                .retain(|e| !e.flags.contains(crate::entry::Flags::REMOVE));

            let mut shared_entries = std::mem::take(&mut shared_index.entries);
            shared_entries.sort_by(|a, b| a.cmp(b, &shared_index.state));

            split_index.entries = shared_entries;
            split_index.path_backing = std::mem::take(&mut shared_index.path_backing);
        }

        Ok(())
    }
}
