use bstr::ByteSlice;
use winnow::{error::StrContext, prelude::*};

use crate::{signature::decode, Identity, IdentityRef};

impl<'a> IdentityRef<'a> {
    /// Deserialize an identity from the given `data`.
    pub fn from_bytes<E>(mut data: &'a [u8]) -> Result<Self, winnow::error::ErrMode<E>>
    where
        E: winnow::error::ParserError<&'a [u8]> + winnow::error::AddContext<&'a [u8], StrContext>,
    {
        decode::identity.parse_next(&mut data)
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
    use crate::{signature::write::validated_token, Identity, IdentityRef};

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
