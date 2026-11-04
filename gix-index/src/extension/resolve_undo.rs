//! Conflict stages retained after a path is resolved, allowing the conflict to be restored.

use bstr::BString;
use gix_hash::ObjectId;

use crate::{extension::Signature, util::split_at_byte_exclusive};

/// Paths with recorded pre-resolution conflict stages.
pub type Paths = Vec<ResolvePath>;

/// A resolved path and its original conflict stages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvePath {
    /// relative to the root of the repository, or what would be stored in the index
    pub name: BString,

    /// 0 = ancestor/common, 1 = ours, 2 = theirs
    pub stages: [Option<Stage>; 3],
}

/// A single original conflict stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stage {
    /// The original Git entry mode, or type and permission bits.
    pub mode: u32,
    /// The object previously recorded at this stage.
    pub id: ObjectId,
}

/// The on-disk extension signature.
pub const SIGNATURE: Signature = *b"REUC";

/// Decode the complete extension payload, returning `None` for malformed data.
pub fn decode(mut data: &[u8], object_hash: gix_hash::Kind) -> Option<Paths> {
    let hash_len = object_hash.len_in_bytes();
    let mut out = Vec::new();

    while !data.is_empty() {
        let (path, rest) = split_at_byte_exclusive(data, 0)?;
        data = rest;

        let mut modes = [0u32; 3];
        for mode in &mut modes {
            let (mode_ascii, rest) = split_at_byte_exclusive(data, 0)?;
            data = rest;
            *mode = u32::from_str_radix(std::str::from_utf8(mode_ascii).ok()?, 8).ok()?;
        }

        let mut stages = [None, None, None];
        for (mode, stage) in modes.iter().zip(stages.iter_mut()) {
            if *mode == 0 {
                continue;
            }
            let (hash, rest) = data.split_at_checked(hash_len)?;
            data = rest;
            *stage = Some(Stage {
                mode: *mode,
                id: ObjectId::from_bytes_or_panic(hash),
            });
        }

        out.push(ResolvePath {
            name: path.into(),
            stages,
        });
    }
    out.into()
}
