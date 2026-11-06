use bstr::BString;
use gix_hash::ObjectId;

use crate::{extension::Signature, util::split_at_byte_exclusive};

/// Paths with merge stages recorded by the resolve-undo extension.
pub type Paths = Vec<ResolvePath>;

/// A path with merge stages recorded by the resolve-undo extension.
#[expect(
    dead_code,
    reason = "REUC is decoded but not yet exposed or written; retain its fields for planned round-trip support"
)]
#[derive(Clone)]
pub struct ResolvePath {
    /// relative to the root of the repository, or what would be stored in the index
    name: BString,

    /// 0 = ancestor/common, 1 = ours, 2 = theirs
    stages: [Option<Stage>; 3],
}

/// The file mode and object ID of a recorded merge stage.
#[expect(
    dead_code,
    reason = "REUC is decoded but not yet exposed or written; retain its fields for planned round-trip support"
)]
#[derive(Clone, Copy)]
pub struct Stage {
    mode: u32,
    id: ObjectId,
}

/// The signature of the resolve-undo extension.
pub const SIGNATURE: Signature = *b"REUC";

/// Decode resolve-undo extension `data` using `object_hash`, returning `None` if it is malformed.
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
