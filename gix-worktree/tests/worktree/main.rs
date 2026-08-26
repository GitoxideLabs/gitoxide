use gix_hash::ObjectId;

#[cfg(feature = "create")]
mod create;
#[cfg(feature = "remove")]
mod remove;
mod stack;

pub use gix_testtools::Result;
pub use gix_testtools::scripted_fixture_read_only;

static SHA1_TO_SHA256_HASHES: std::sync::LazyLock<std::collections::HashMap<&str, &str>> =
    std::sync::LazyLock::new(|| {
        [(
            "5c7e0ed672d3d31d83a3df61f13cc8f7b22d5bfd",
            "7b23c65fe2c72939324bccab5501c8a5be3608e5b2892cc92d96064f630ddef0",
        )]
        .into()
    });

/// Convert a hexadecimal SHA-1 hash or the corresponding SHA-256 hash into an `ObjectId` or
/// _panic_.
pub fn hex_to_id(hex: &str) -> ObjectId {
    match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex"),
        gix_hash::Kind::Sha256 => ObjectId::from_hex(
            SHA1_TO_SHA256_HASHES
                .get(hex)
                .unwrap_or_else(|| panic!("SHA-1 {hex} wasn't mapped to SHA-256 yet"))
                .as_bytes(),
        )
        .expect("64 bytes hex"),
        _ => unimplemented!(),
    }
}
