use bstr::BStr;
use gix_hash::{ObjectId, oid};

use crate::{Kind, TagRefIter, bstr::ByteSlice, tag::decode};

#[derive(Default, Copy, Clone)]
pub(crate) enum State {
    #[default]
    Target,
    TargetKind,
    Name,
    Tagger,
    Message,
}

impl<'a> TagRefIter<'a> {
    /// Create a tag iterator from `data`, parsing hashes as `object_hash`.
    pub fn from_bytes(data: &'a [u8], hash_kind: gix_hash::Kind) -> TagRefIter<'a> {
        TagRefIter {
            data,
            state: State::default(),
            hash_kind,
        }
    }

    /// Returns the target id of this tag if it is the first function called and if there is no error in decoding
    /// the data.
    ///
    /// Note that this method must only be called once or else will always return None while consuming a single token.
    /// Errors are coerced into options, hiding whether there was an error or not. The caller should assume an error if they
    /// call the method as intended. Such a squelched error cannot be recovered unless the objects data is retrieved and parsed again.
    /// `next()`.
    pub fn target_id(mut self) -> Result<ObjectId, crate::decode::Error> {
        let token = self.next().ok_or_else(missing_field)??;
        Token::into_id(token).ok_or_else(missing_field)
    }

    /// Returns the taggers signature if there is no decoding error, and if this field exists.
    /// Errors are coerced into options, hiding whether there was an error or not. The caller knows if there was an error or not.
    pub fn tagger(mut self) -> Result<Option<gix_actor::SignatureRef<'a>>, crate::decode::Error> {
        self.find_map(|t| match t {
            Ok(Token::Tagger(signature)) => Some(Ok(signature)),
            Err(err) => Some(Err(err)),
            _ => None,
        })
        .ok_or_else(missing_field)?
    }
}

fn missing_field() -> crate::decode::Error {
    crate::decode::empty_error()
}

impl<'a> TagRefIter<'a> {
    #[inline]
    fn next_inner(
        mut i: &'a [u8],
        state: &mut State,
        hash_kind: gix_hash::Kind,
    ) -> Result<(&'a [u8], Token<'a>), crate::decode::Error> {
        let input = &mut i;
        match Self::next_inner_(input, state, hash_kind) {
            Ok(token) => Ok((*input, token)),
            Err(err) => Err(err),
        }
    }

    fn next_inner_(
        input: &mut &'a [u8],
        state: &mut State,
        hash_kind: gix_hash::Kind,
    ) -> Result<Token<'a>, crate::decode::Error> {
        use State::*;
        Ok(match state {
            Target => {
                let target = decode::target(input, hash_kind)?;
                *state = TargetKind;
                Token::Target {
                    id: ObjectId::from_hex(target).expect("parsing validation"),
                }
            }
            TargetKind => {
                let kind = decode::kind(input)?;
                *state = Name;
                Token::TargetKind(kind)
            }
            Name => {
                let tag_version = decode::name(input)?;
                *state = Tagger;
                Token::Name(tag_version.as_bstr())
            }
            Tagger => {
                *state = Message;
                let signature = decode::tagger(input)?;
                Token::Tagger(signature)
            }
            Message => {
                let (message, pgp_signature) = decode::message(input)?;
                debug_assert!(
                    input.is_empty(),
                    "we should have consumed all data - otherwise iter may go forever"
                );
                Token::Body { message, pgp_signature }
            }
        })
    }
}

impl<'a> Iterator for TagRefIter<'a> {
    type Item = Result<Token<'a>, crate::decode::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }
        match Self::next_inner(self.data, &mut self.state, self.hash_kind) {
            Ok((data, token)) => {
                self.data = data;
                Some(Ok(token))
            }
            Err(err) => {
                self.data = &[];
                Some(Err(err))
            }
        }
    }
}

/// A token returned by the [tag iterator][TagRefIter].
#[allow(missing_docs)]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub enum Token<'a> {
    Target {
        id: ObjectId,
    },
    TargetKind(Kind),
    Name(&'a BStr),
    Tagger(Option<gix_actor::SignatureRef<'a>>),
    Body {
        message: &'a BStr,
        pgp_signature: Option<&'a BStr>,
    },
}

impl Token<'_> {
    /// Return the object id of this token if its a [Target][Token::Target].
    pub fn id(&self) -> Option<&oid> {
        match self {
            Token::Target { id } => Some(id.as_ref()),
            _ => None,
        }
    }

    /// Return the owned object id of this token if its a [Target][Token::Target].
    pub fn into_id(self) -> Option<ObjectId> {
        match self {
            Token::Target { id } => Some(id),
            _ => None,
        }
    }
}
