use gix_hash::ObjectId;
use gix_testtools::fixture_path;

pub use gix_testtools::{scripted_fixture_read_only, scripted_fixture_read_only_with_args, scripted_fixture_writable};

pub fn hex_to_id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

pub type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn db() -> gix_odb::Handle {
    gix_odb::at(fixture_path("objects")).expect("valid object path")
}

fn db_small_packs() -> gix_odb::Handle {
    gix_odb::at(fixture_path("repos/small-packs.git/objects")).unwrap()
}

pub mod alternate;
pub mod find;
pub mod header;
pub mod memory;
pub mod regression;
pub mod sink;
pub mod store;
