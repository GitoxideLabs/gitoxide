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

/// [`init::Options`](gix_odb::store::init::Options) respecting [`gix_testtools::object_hash()`]
/// that regenerate under `GIX_TEST_FIXTURE_HASH` are opened with a matching object hash.
pub fn fixture_options() -> gix_odb::store::init::Options {
    gix_odb::store::init::Options {
        object_hash: gix_testtools::object_hash(),
        ..Default::default()
    }
}

/// Open an object store at `objects_dir`.
/// The static SHA-1 fixtures keep using [`db()`]/[`db_small_packs()`] instead.
pub fn odb_at(objects_dir: impl Into<std::path::PathBuf>) -> std::io::Result<gix_odb::Handle> {
    gix_odb::at_opts(objects_dir, Vec::new(), fixture_options())
}

fn db() -> gix_odb::Handle {
    gix_odb::at_opts(
        fixture_path("objects"),
        None,
        gix_odb::store::init::Options {
            object_hash: gix_hash::Kind::Sha1,
            ..Default::default()
        },
    )
    .expect("valid object path")
}

fn db_small_packs() -> gix_odb::Handle {
    gix_odb::at_opts(
        fixture_path("repos/small-packs.git/objects"),
        None,
        gix_odb::store::init::Options {
            object_hash: gix_hash::Kind::Sha1,
            ..Default::default()
        },
    )
    .unwrap()
}

pub mod alternate;
pub mod find;
pub mod header;
pub mod memory;
pub mod regression;
pub mod sink;
pub mod store;
