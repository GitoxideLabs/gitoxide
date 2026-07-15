#[cfg(feature = "describe")]
mod describe;
#[cfg(feature = "merge_base")]
mod merge_base;
#[cfg(feature = "name_rev")]
mod name_rev;
mod spec;

pub use gix_testtools::Result;

fn hex_to_id(hex: &str) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}
