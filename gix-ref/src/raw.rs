use gix_hash::ObjectId;

use crate::{FullName, Target};

/// A fully owned backend agnostic reference
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Reference {
    /// The path to uniquely identify this ref within its store.
    pub name: FullName,
    /// The target of the reference, either a symbolic reference by full name or a possibly intermediate object by its id.
    pub target: Target,
    /// The fully peeled object to which this reference ultimately points to after following all symbolic refs and all annotated
    /// tags. Only guaranteed to be set after
    /// [`Reference::peel_to_id()`](crate::file::ReferenceExt::peel_to_id) was called or if this reference originated
    /// from a packed ref.
    pub peeled: Option<ObjectId>,
}

mod convert {
    use gix_hash::ObjectId;

    use crate::{
        raw::Reference,
        store_impl::{file::loose, packed},
        Target,
    };

    impl From<Reference> for loose::Reference {
        fn from(value: Reference) -> Self {
            loose::Reference {
                name: value.name,
                target: value.target,
            }
        }
    }

    impl From<loose::Reference> for Reference {
        fn from(value: loose::Reference) -> Self {
            Reference {
                name: value.name,
                target: value.target,
                peeled: None,
            }
        }
    }

    impl<'p> From<packed::Reference<'p>> for Reference {
        fn from(value: packed::Reference<'p>) -> Self {
            Reference {
                name: value.name.into(),
                target: Target::Object(value.target()),
                peeled: value
                    .object
                    .map(|hex| ObjectId::from_hex(hex).expect("parser validation")),
            }
        }
    }
}

mod access {
    use gix_object::bstr::ByteSlice;

    use crate::{raw::Reference, FullNameRef, Namespace, Target};

    impl Reference {
        /// Returns the kind of reference based on its target
        pub fn kind(&self) -> crate::Kind {
            self.target.kind()
        }

        /// Return the full validated name of the reference, with the given namespace stripped if possible.
        ///
        /// If the reference name wasn't prefixed with `namespace`, `None` is returned instead.
        pub fn name_without_namespace(&self, namespace: &Namespace) -> Option<&FullNameRef> {
            self.name
                .0
                .as_bstr()
                .strip_prefix(namespace.0.as_bytes())
                .map(|stripped| FullNameRef::new_unchecked(stripped.as_bstr()))
        }

        /// Strip the given namespace from our name as well as the name, but not the reference we point to.
        pub fn strip_namespace(&mut self, namespace: &Namespace) -> &mut Self {
            self.name.strip_namespace(namespace);
            if let Target::Symbolic(name) = &mut self.target {
                name.strip_namespace(namespace);
            }
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use gix_testtools::size_ok;

    use super::*;

    #[test]
    fn size_of_reference() {
        let actual = std::mem::size_of::<Reference>();
        let sha1 = 80;
        let sha256_extra = 24;
        let expected = sha1 + sha256_extra;
        assert!(
            size_ok(actual, expected),
            "let's not let it change size undetected: {actual} <~ {expected}"
        );
    }
}
