#[cfg(feature = "describe")]
mod describe;
#[cfg(feature = "merge_base")]
mod merge_base;
mod spec;

pub use gix_testtools::Result;

static SHA1_TO_SHA256_HASHES: std::sync::LazyLock<std::collections::HashMap<&str, &str>> =
    std::sync::LazyLock::new(|| {
        [
            (
                "01ec18a3ebf2855708ad3c9d244306bc1fae3e9b",
                "fb6f3cf687f7adc3da7d030935d071b738861741046d030b37e5efcc9cde5131",
            ),
            (
                "3ca3e3dd12585fabbef311d524a5e54678090528",
                "0200e122dc6bbd419e90bdbf342740558c71664cecd0a874cd812433cdfd7c8b",
            ),
            (
                "413d38a3fe7453c68cb7314739d7775f68ab89f5",
                "7e6a85f687dd29fb6fadc71abed1447d49fb389950d97a6ad0698c8c4e2ccb5a",
            ),
            (
                "4ce66b336dff547fdeb6cd86e04c617c8d998ff5",
                "b6319202e8ffdc4e02fafb6e81c1782091ede487cf7319539d26c590d556b5b6",
            ),
            (
                "4fbed377d3eab982d4a465cafaf34b64207da847",
                "ae9afa67fcc27827c25da7d6103a2abf8735b1641787bf6f7108b88c297917cb",
            ),
            (
                "6291f6d7da04208dc4ccbbdf9fda98ac9ae67bc0",
                "f09702849e3702cc4197fa6251460bf2a2e6990409fe5e9f36a4ac588cd09bc4",
            ),
            (
                "8bc2f99c9aacf07568a2bbfe1269f6e543f22d6b",
                "19e73229bd7608ac12638bda3ab889a5b2e4d1c6cd2f8012687060a84afbdecf",
            ),
            (
                "9152eeee2328073cf23dcf8e90c949170b711659",
                "9b1336395000ea1dda99a04bec4ef7d4eeea969312ec4d2fa86b6527bfd8fbfd",
            ),
            (
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            ),
            (
                "b920bbb055e1efb9080592a409d3975738b6efb3",
                "b920bbb055e1efb9080592a409d3975738b6efb3000000000000000000000000",
            ),
            (
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            ),
            (
                "c507d5413da00c32e5de1ea433030e8e4716bc60",
                "7f8591576b32d5d8d1f8f3ebf711e0c2fbf6d702a25424530565ad6767bcfb7d",
            ),
            (
                "d4d01a9b6f6fcb23d57cd560229cd9680ec9bd6e",
                "23ef9211b6ce466ec1c76ff067f6b53418611edf97e1fd29eafcd9bb08cf1f30",
            ),
            (
                "e5d0542bd38431f105a8de8e982b3579647feb9f",
                "ea1f3113553eb24785c1ed961dc7663fe8d029387dc57cd51b254abd322a797e",
            ),
            (
                "efd9a841189668f1bab5b8ebade9cd0a1b139a37",
                "0fc125d0690528eeff91d75edb3da0fa7bf75ed8eca44c0e402d4a6b6975e86a",
            ),
        ]
        .into()
    });

/// Convert a hexadecimal SHA-1 hash or the corresponding SHA-256 hash into an `ObjectId` or
/// _panic_.
fn hex_to_id(hex: &str) -> gix_hash::ObjectId {
    match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex"),
        gix_hash::Kind::Sha256 => gix_hash::ObjectId::from_hex(
            SHA1_TO_SHA256_HASHES
                .get(hex)
                .unwrap_or_else(|| panic!("SHA-1 {hex} wasn't mapped to SHA-256 yet"))
                .as_bytes(),
        )
        .expect("64 bytes hex"),
        _ => unimplemented!(),
    }
}

fn odb_at(objects_dir: impl Into<std::path::PathBuf>) -> Result<gix_odb::Handle> {
    Ok(gix_odb::at(objects_dir, gix_testtools::object_hash())?)
}
