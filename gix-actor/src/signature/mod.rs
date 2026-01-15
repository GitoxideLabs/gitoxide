mod _ref {
    use bstr::ByteSlice;
    use winnow::{error::StrContext, prelude::*};

    use crate::{signature::decode, IdentityRef, Signature, SignatureRef};

    /// Lifecycle
    impl<'a> SignatureRef<'a> {
        /// Deserialize a signature from the given `data`.
        pub fn from_bytes<E>(mut data: &'a [u8]) -> Result<SignatureRef<'a>, winnow::error::ErrMode<E>>
        where
            E: winnow::error::ParserError<&'a [u8]> + winnow::error::AddContext<&'a [u8], StrContext>,
        {
            decode.parse_next(&mut data)
        }

        /// Try to parse the timestamp and create an owned instance from this shared one.
        pub fn to_owned(&self) -> Result<Signature, gix_date::Error> {
            Ok(Signature {
                name: self.name.to_owned(),
                email: self.email.to_owned(),
                time: self.time()?,
            })
        }
    }

    /// Access
    impl<'a> SignatureRef<'a> {
        /// Trim the whitespace surrounding the `name`, `email` and `time` and return a new signature.
        pub fn trim(&self) -> SignatureRef<'a> {
            SignatureRef {
                name: self.name.trim().as_bstr(),
                email: self.email.trim().as_bstr(),
                time: self.time.trim(),
            }
        }

        /// Return the actor's name and email, effectively excluding the timestamp of this signature.
        pub fn actor(&self) -> IdentityRef<'a> {
            IdentityRef {
                name: self.name,
                email: self.email,
            }
        }

        /// Parse only the seconds since unix epoch from the `time` field, or silently default to 0
        /// if parsing fails. Note that this ignores the timezone, so it can parse otherwise broken dates.
        ///
        /// For a fallible and more complete, but slower version, use [`time()`](Self::time).
        pub fn seconds(&self) -> gix_date::SecondsSinceUnixEpoch {
            self.time
                .trim()
                .split(' ')
                .next()
                .and_then(|i| i.parse().ok())
                .unwrap_or_default()
        }

        /// Parse the `time` field for access to the passed time since unix epoch, and the time offset.
        /// The format is expected to be [raw](gix_date::parse_header()).
        pub fn time(&self) -> Result<gix_date::Time, gix_date::Error> {
            self.time.parse()
        }
    }
}

mod convert {
    use gix_date::parse::TimeBuf;

    use crate::{Signature, SignatureRef};

    impl Signature {
        /// Borrow this instance as immutable, serializing the `time` field into `buf`.
        ///
        /// Commonly used as [`signature.to_ref(&mut TimeBuf::default())`](TimeBuf::default).
        pub fn to_ref<'a>(&'a self, time_buf: &'a mut TimeBuf) -> SignatureRef<'a> {
            SignatureRef {
                name: self.name.as_ref(),
                email: self.email.as_ref(),
                time: self.time.to_str(time_buf),
            }
        }
    }

    /// Note that this conversion is lossy due to the lenient parsing of the [`time`](SignatureRef::time) field.
    impl From<SignatureRef<'_>> for Signature {
        fn from(other: SignatureRef<'_>) -> Signature {
            Signature {
                name: other.name.to_owned(),
                email: other.email.to_owned(),
                time: other.time().unwrap_or_default(),
            }
        }
    }
}

pub(crate) mod write {
    use bstr::{BStr, ByteSlice};
    use gix_date::parse::TimeBuf;

    use crate::{Signature, SignatureRef};

    /// The Error produced by [`Signature::write_to()`].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub(crate) enum Error {
        #[error(r"Signature name or email must not contain '<', '>' or \n")]
        IllegalCharacter,
    }

    impl From<Error> for std::io::Error {
        fn from(err: Error) -> Self {
            std::io::Error::other(err)
        }
    }

    /// Output
    impl Signature {
        /// Serialize this instance to `out` in the git serialization format for actors.
        pub fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
            let mut buf = TimeBuf::default();
            self.to_ref(&mut buf).write_to(out)
        }
        /// Computes the number of bytes necessary to serialize this signature
        pub fn size(&self) -> usize {
            self.name.len() + 2 /* space <*/ + self.email.len() +  2 /* > space */ + self.time.size()
        }
    }

    impl SignatureRef<'_> {
        /// Serialize this instance to `out` in the git serialization format for actors.
        pub fn write_to(&self, out: &mut dyn std::io::Write) -> std::io::Result<()> {
            out.write_all(validated_token(self.name)?)?;
            out.write_all(b" ")?;
            out.write_all(b"<")?;
            out.write_all(validated_token(self.email)?)?;
            out.write_all(b"> ")?;
            out.write_all(validated_token(self.time.into())?)
        }
        /// Computes the number of bytes necessary to serialize this signature
        pub fn size(&self) -> usize {
            self.name.len() + 2 /* space <*/ + self.email.len() +  2 /* > space */ + self.time.len()
        }
    }

    pub(crate) fn validated_token(name: &BStr) -> Result<&BStr, Error> {
        if name.find_byteset(b"<>\n").is_some() {
            return Err(Error::IllegalCharacter);
        }
        Ok(name)
    }
}

///
pub mod decode;
pub use decode::function::decode;
