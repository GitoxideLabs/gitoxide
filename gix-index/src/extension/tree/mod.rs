use crate::extension::Signature;

/// The signature for tree extensions
pub const SIGNATURE: Signature = *b"TREE";

///
pub mod verify;

mod decode;
pub use decode::decode;

mod write;

#[cfg(test)]
mod tests {
    use gix_testtools::size_ok;

    #[test]
    fn size_of_tree() {
        let actual = std::mem::size_of::<crate::extension::Tree>();
        let expected = 104;
        assert!(
            size_ok(actual, expected),
            "the size of this structure should not change unexpectedly: {actual} <~ {expected}"
        );
    }
}
