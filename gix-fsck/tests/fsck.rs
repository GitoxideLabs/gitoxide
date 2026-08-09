use gix_hash::ObjectId;

static SHA1_TO_SHA256_HASHES: std::sync::LazyLock<std::collections::HashMap<&str, &str>> =
    std::sync::LazyLock::new(|| {
        [
            (
                "20317ffa7614f49b2702a057bf2833918ea9fd24",
                "b0f75640686e83fa924630c38cfa9ea6b7d620ab2d25256abea132a85c4bc9a4",
            ),
            (
                "4cdeaab5b01f9a9fbbb2fb6c08404cf12b7bdab1",
                "11024c008d9f84812ae424262f0e64afe301179095fa55d97dc86239db40b4ad",
            ),
            (
                "734c926856a328d1168ffd7088532e0d1ad19bbe",
                "e220624b63997f59795adfcb6d0089e86c1e5a1ec371ab8aee59028d99063e29",
            ),
            (
                "8ff6d0f8891c3cb22827be142cc64606121d47b3",
                "f51926ed3d442ae8b72a8e7b6b7f50a024a6d3d4cddd0b8ff459738e756303e4",
            ),
            (
                "c18147dc648481eeb65dc5e66628429a64843327",
                "a17ef66bc682c044fa7adaf0b5d560198e57a61effd6f68bf5d5f64f92097115",
            ),
            (
                "ebed23648b19484cb1f340c4ee04dda08479188a",
                "1020c865922a803fad1e39f001f41e2d89dded76ded3e2cd331230310a21c7b5",
            ),
            (
                "fc264b3b6875a46e9031483aeb9994a1b897ffd3",
                "ee4ba7f091756b415601c00353451b39d29bb5eb82e35e9e99386ccdc0df79a1",
            ),
        ]
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

mod connectivity;
