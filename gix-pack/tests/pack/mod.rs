const SMALL_PACK_INDEX: &str = "objects/pack/pack-a2bf8e71d8c18879e499335762dd95119d93d9f1.idx";
const SMALL_PACK: &str = "objects/pack/pack-a2bf8e71d8c18879e499335762dd95119d93d9f1.pack";

const INDEX_V1: &str = "objects/pack/pack-c0438c19fb16422b6bbcce24387b3264416d485b.idx";
const PACK_FOR_INDEX_V1: &str = "objects/pack/pack-c0438c19fb16422b6bbcce24387b3264416d485b.pack";

const INDEX_V2: &str = "objects/pack/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.idx";
const PACK_FOR_INDEX_V2: &str = "objects/pack/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.pack";

const PACKS_AND_INDICES: &[(&str, &str)] = &[(SMALL_PACK_INDEX, SMALL_PACK), (INDEX_V1, PACK_FOR_INDEX_V1)];

const V2_PACKS_AND_INDICES: &[(&str, &str)] = &[(SMALL_PACK_INDEX, SMALL_PACK), (INDEX_V2, PACK_FOR_INDEX_V2)];

use std::path::PathBuf;

use gix_hash::ObjectId;
pub use gix_testtools::{
    fixture_path_standalone as fixture_path, scripted_fixture_read_only_standalone as scripted_fixture_read_only,
};

pub fn hex_to_id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

/// Read fixture data into memory and intentionally leak it to obtain a `'static` byte slice.
///
/// This is acceptable in tests because the fixtures are small, loaded only for the duration of the
/// test process, and the process exits immediately after the test run.
pub(crate) fn leaked_fixture_bytes(path: PathBuf) -> (&'static [u8], PathBuf) {
    let data: &'static [u8] = Box::leak(std::fs::read(&path).expect("readable fixture").into_boxed_slice());
    (data, path)
}

/// Load a pack fixture into memory and instantiate a memory-backed pack file from it.
pub(crate) fn pack_from_memory_at(at: &str) -> gix_pack::data::File<&'static [u8]> {
    let (data, path) = leaked_fixture_bytes(fixture_path(at));
    gix_pack::data::File::from_data(data, path, gix_hash::Kind::Sha1).expect("valid pack file")
}

pub(crate) fn fuzz_artifact_paths(target: &str) -> Vec<PathBuf> {
    let mut paths = std::fs::read_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../fuzz/artifacts")
            .join(target),
    )
    .expect("artifact directory exists")
    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
    .collect::<Vec<_>>();
    paths.sort();
    paths
}

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[cfg(not(windows))]
pub fn fixup(v: Vec<u8>) -> Vec<u8> {
    v
}

#[cfg(windows)]
pub fn fixup(v: Vec<u8>) -> Vec<u8> {
    // Git checks out text files with line ending conversions, git itself will of course not put '\r\n' anywhere,
    // so that wouldn't be expected in an object and doesn't have to be parsed.
    use bstr::ByteSlice;
    v.replace(b"\r\n", "\n")
}

mod bundle;
mod data;
mod index;
mod iter;
mod malformed;
mod multi_index;
