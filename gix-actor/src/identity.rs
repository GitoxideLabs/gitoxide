use bstr::ByteSlice;

use crate::{Identity, IdentityRef, signature::decode};

impl<'a> IdentityRef<'a> {
    /// Deserialize an identity from the given `data`.
    ///
    /// Typical input is `Name <name@example.com> 1700000000 +0000`.
    pub fn from_bytes(mut data: &'a [u8]) -> Result<Self, gix_error::ValidationError> {
        Self::from_bytes_consuming(&mut data)
    }

    /// Deserialize an identity from the given `data` and advance it past the identity.
    ///
    /// Typical input is `Name <name@example.com> 1700000000 +0000`; on success,
    /// `data` points to the bytes immediately after the closing `>`.
    pub fn from_bytes_consuming(data: &mut &'a [u8]) -> Result<Self, gix_error::ValidationError> {
        decode::identity(data)
    }

    /// Create an owned instance from this shared one.
    pub fn to_owned(&self) -> Identity {
        Identity {
            name: self.name.to_owned(),
            email: self.email.to_owned(),
        }
    }

    /// Trim whitespace surrounding the name and email and return a new identity.
    pub fn trim(&self) -> IdentityRef<'a> {
        IdentityRef {
            name: self.name.trim().as_bstr(),
            email: self.email.trim().as_bstr(),
        }
    }
}

mod write {
    use crate::{Identity, IdentityRef, signature::write::validated_token};

    /// Output
    impl Identity {
        /// Serialize this instance to `out` in the git serialization format for signatures (but without timestamp).
        pub fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
            self.to_ref().write_to(out)
        }
    }

    impl IdentityRef<'_> {
        /// Serialize this instance to `out` in the git serialization format for signatures (but without timestamp).
        pub fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
            out.write_all(validated_token(self.name).map_err(std::io::Error::other)?)?;
            out.write_all(b" ")?;
            out.write_all(b"<")?;
            out.write_all(validated_token(self.email).map_err(std::io::Error::other)?)?;
            out.write_all(b">")
        }
    }
}

mod impls {
    use crate::{Identity, IdentityRef, Signature, SignatureRef};

    impl Identity {
        /// Borrow this instance as immutable
        pub fn to_ref(&self) -> IdentityRef<'_> {
            IdentityRef {
                name: self.name.as_ref(),
                email: self.email.as_ref(),
            }
        }
    }

    impl From<IdentityRef<'_>> for Identity {
        fn from(other: IdentityRef<'_>) -> Identity {
            let IdentityRef { name, email } = other;
            Identity {
                name: name.to_owned(),
                email: email.to_owned(),
            }
        }
    }

    impl<'a> From<&'a Identity> for IdentityRef<'a> {
        fn from(other: &'a Identity) -> IdentityRef<'a> {
            other.to_ref()
        }
    }

    impl From<Signature> for Identity {
        fn from(Signature { name, email, time: _ }: Signature) -> Self {
            Identity { name, email }
        }
    }

    impl<'a> From<SignatureRef<'a>> for IdentityRef<'a> {
        fn from(SignatureRef { name, email, time: _ }: SignatureRef<'a>) -> Self {
            IdentityRef { name, email }
        }
    }
}
