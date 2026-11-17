use crate::extension::Signature;

/// The signature for tree extensions
pub const SIGNATURE: Signature = *b"TREE";

impl crate::extension::Tree {
    pub(crate) fn invalidate_path(&mut self, path: &bstr::BStr) {
        use bstr::ByteSlice;

        self.num_entries = None;
        let (component, rest) = match path.find_byte(b'/') {
            Some(slash) => (&path[..slash], Some(path[slash + 1..].as_bstr())),
            None => (path, None),
        };
        if let Ok(index) = self
            .children
            .binary_search_by(|child| child.name.as_slice().cmp(component))
        {
            match rest {
                Some(rest) => self.children[index].invalidate_path(rest),
                None => {
                    self.children.remove(index);
                }
            }
        }
    }
}

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
        let sha1 = 88;
        let sha256_extra = 16;
        let expected = sha1 + sha256_extra;
        assert!(
            size_ok(actual, expected),
            "the size of this structure should not change unexpectedly: {actual} <~ {expected}"
        );
    }
}
