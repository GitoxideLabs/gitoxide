use gix_hash::ObjectId;
use gix_testtools::fixture_path;

pub use gix_testtools::{scripted_fixture_read_only, scripted_fixture_read_only_with_args, scripted_fixture_writable};

pub fn hex_to_id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("valid hex object id")
}

pub fn hex_to_id_for_hash(sha1: &str, sha256: &str) -> ObjectId {
    hex_to_id(match gix_testtools::object_hash() {
        gix_hash::Kind::Sha256 => sha256,
        _ => sha1,
    })
}

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[path = "../tools/odb.rs"]
pub mod odb_fixture;

/// Open an object store at `objects_dir`.
/// The static SHA-1 fixtures keep using [`db()`]/[`db_small_packs()`] instead.
pub fn odb_at(objects_dir: impl Into<std::path::PathBuf>) -> std::io::Result<gix_odb::Handle> {
    gix_odb::at(objects_dir, gix_testtools::object_hash())
}

fn db() -> gix_odb::Handle {
    gix_odb::at(fixture_path("objects"), gix_hash::Kind::Sha1).expect("valid object path")
}

fn db_small_packs() -> gix_odb::Handle {
    gix_odb::at(fixture_path("repos/small-packs.git/objects"), gix_hash::Kind::Sha1).unwrap()
}

pub mod alternate;
pub mod find;
pub mod header;
pub mod memory;
pub mod regression;
pub mod sink;
pub mod store;
