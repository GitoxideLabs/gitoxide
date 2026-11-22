use gix_error::{ErrorExt, OptionExt, Result, ensure};
use std::{borrow::Cow, ops::Range};

use bstr::BStr;
use gix_hash::{ObjectId, oid};

use crate::{
    CommitRefIter,
    bstr::ByteSlice,
    commit::{decode, signature_field_name},
    parse,
    signature::SignedData,
};

#[derive(Copy, Clone)]
pub(crate) enum SignatureKind {
    Author,
    Committer,
}

#[derive(Default, Copy, Clone)]
pub(crate) enum State {
    #[default]
    Tree,
    Parents,
    Signature {
        of: SignatureKind,
    },
    Encoding,
    ExtraHeaders,
    Message,
}

/// Lifecycle
impl<'a> CommitRefIter<'a> {
    /// Create a commit iterator from the given `data`, using `object_hash` to know
    /// what kind of hash to expect for validation.
    pub fn from_bytes(data: &'a [u8], hash_kind: gix_hash::Kind) -> CommitRefIter<'a> {
        CommitRefIter {
            data,
            state: State::default(),
            hash_kind,
        }
    }
}

/// Access
impl<'a> CommitRefIter<'a> {
    /// Parse `data` as commit and return its PGP signature, along with *all non-signature* data as [`SignedData`], or `None`
    /// if the commit isn't signed. All hashes in `data` are parsed as `object_hash`.
    ///
    /// This allows the caller to validate the signature by passing the signed data along with the signature back to the program
    /// that created it.
    pub fn signature(data: &'a [u8], hash_kind: gix_hash::Kind) -> Result<Option<(Cow<'a, BStr>, SignedData<'a>)>> {
        let mut signature_and_range = None;

        let raw_tokens = CommitRefIterRaw {
            data,
            state: State::default(),
            offset: 0,
            hash_kind,
        };
        for token in raw_tokens {
            let token = token?;
            if let Token::ExtraHeader((name, value)) = &token.token
                && *name == signature_field_name(hash_kind)
            {
                // keep track of the signature range alongside the signature data,
                // because all but the signature is the signed data.
                signature_and_range = Some((value.clone(), token.token_range));
                break;
            }
        }

        Ok(signature_and_range.map(|(sig, signature_range)| (sig, SignedData::new(data, signature_range))))
    }

    /// Returns the object id of this commits tree if it is the first function called and if there is no error in decoding
    /// the data.
    ///
    /// Note that this method must only be called once or else will always return None while consuming a single token.
    /// Errors are coerced into options, hiding whether there was an error or not. The caller should assume an error if they
    /// call the method as intended. Such a squelched error cannot be recovered unless the objects data is retrieved and parsed again.
    /// `next()`.
    pub fn tree_id(&mut self) -> Result<ObjectId> {
        let tree_id = self.next().ok_or_raise(missing_field)??;
        Token::try_into_id(tree_id).ok_or_raise(missing_field)
    }

    /// Return all `parent_ids` as iterator.
    ///
    /// Parsing errors are ignored quietly.
    pub fn parent_ids(self) -> impl Iterator<Item = gix_hash::ObjectId> + 'a {
        self.filter_map(|t| match t {
            Ok(Token::Parent { id }) => Some(id),
            _ => None,
        })
    }

    /// Returns all signatures, first the author, then the committer, if there is no decoding error.
    ///
    /// Errors are coerced into options, hiding whether there was an error or not. The caller knows if there was an error or not
    /// if not exactly two signatures were iterable.
    /// Errors are not the common case - if an error needs to be detectable, use this instance as iterator.
    pub fn signatures(self) -> impl Iterator<Item = gix_actor::SignatureRef<'a>> + 'a {
        self.filter_map(|t| match t {
            Ok(Token::Author { signature } | Token::Committer { signature }) => Some(signature),
            _ => None,
        })
    }

    /// Returns the committer signature if there is no decoding error.
    pub fn committer(mut self) -> Result<gix_actor::SignatureRef<'a>> {
        self.find_map(|t| match t {
            Ok(Token::Committer { signature }) => Some(Ok(signature)),
            Err(err) => Some(Err(err)),
            _ => None,
        })
        .ok_or_raise(missing_field)?
    }

    /// Returns the author signature if there is no decoding error.
    ///
    /// It may contain white space surrounding it, and is exactly as parsed.
    pub fn author(mut self) -> Result<gix_actor::SignatureRef<'a>> {
        self.find_map(|t| match t {
            Ok(Token::Author { signature }) => Some(Ok(signature)),
            Err(err) => Some(Err(err)),
            _ => None,
        })
        .ok_or_raise(missing_field)?
    }

    /// Returns the message if there is no decoding error.
    ///
    /// The message borrows all bytes after the blank line that separates it from the headers.
    /// The separator is excluded, but any leading or trailing whitespace in the message is preserved.
    ///
    /// To reach the message, this method checks the syntax of any remaining extra headers, including
    /// their continuation lines. It skips their values without interpreting them or allocating buffers
    /// to unfold multi-line values.
    pub fn message(mut self) -> Result<&'a BStr> {
        while !self.data.is_empty() {
            if let Token::Message(message) = Self::parse_token::<true>(&mut self.data, &mut self.state, self.hash_kind)?
            {
                return Ok(message);
            }
        }
        Ok(Default::default())
    }
}

fn missing_field() -> gix_error::Message {
    crate::decode::empty_error()
}

impl<'a> CommitRefIter<'a> {
    #[inline]
    fn next_inner(mut i: &'a [u8], state: &mut State, hash_kind: gix_hash::Kind) -> Result<(&'a [u8], Token<'a>)> {
        let token = Self::parse_token::<false>(&mut i, state, hash_kind)?;
        Ok((i, token))
    }

    fn parse_token<const SKIP_EXTRA_HEADERS: bool>(
        input: &mut &'a [u8],
        state: &mut State,
        hash_kind: gix_hash::Kind,
    ) -> Result<Token<'a>> {
        use State::*;
        let parse_id = |hex: &[u8]| {
            ensure!(hex.len() == hash_kind.len_in_hex(), crate::decode::empty_error());
            ObjectId::from_hex(hex).map_err(|_| crate::decode::empty_error().raise())
        };
        Ok(match state {
            Tree => {
                let id = parse::header_field(input, b"tree", parse_id)?;
                *state = State::Parents;
                Token::Tree { id }
            }
            Parents => {
                if input.starts_with(b"parent ") {
                    let id = parse::header_field(input, b"parent", parse_id)?;
                    Token::Parent { id }
                } else {
                    *state = State::Signature {
                        of: SignatureKind::Author,
                    };
                    Self::parse_token::<SKIP_EXTRA_HEADERS>(input, state, hash_kind)?
                }
            }
            Signature { of } => {
                let who = *of;
                let field_name = match of {
                    SignatureKind::Author => {
                        *of = SignatureKind::Committer;
                        &b"author"[..]
                    }
                    SignatureKind::Committer => {
                        *state = State::Encoding;
                        &b"committer"[..]
                    }
                };
                let signature = parse::header_field(input, field_name, parse::signature)?;
                match who {
                    SignatureKind::Author => Token::Author { signature },
                    SignatureKind::Committer => Token::Committer { signature },
                }
            }
            Encoding => {
                *state = State::ExtraHeaders;
                if input.starts_with(b"encoding ") {
                    let encoding = parse::header_field(input, b"encoding", Ok)?;
                    Token::Encoding(encoding.as_bstr())
                } else {
                    Self::parse_token::<SKIP_EXTRA_HEADERS>(input, state, hash_kind)?
                }
            }
            ExtraHeaders => {
                if SKIP_EXTRA_HEADERS {
                    while !input.is_empty() && !input.starts_with(parse::NL) {
                        parse::any_header_field(input)?;
                    }
                    if input.is_empty() {
                        return Ok(Token::Message(Default::default()));
                    }
                }
                if input.starts_with(parse::NL) {
                    *state = State::Message;
                    Self::parse_token::<SKIP_EXTRA_HEADERS>(input, state, hash_kind)?
                } else {
                    let (name, value) = parse::any_header_field(input)?;
                    Token::ExtraHeader((name, value.unfold()))
                }
            }
            Message => {
                let message = decode::message(input)?;
                debug_assert!(
                    input.is_empty(),
                    "we should have consumed all data - otherwise iter may go forever"
                );
                Token::Message(message)
            }
        })
    }
}

impl<'a> Iterator for CommitRefIter<'a> {
    type Item = Result<Token<'a>>;

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

/// A variation of [`CommitRefIter`] that return's [`RawToken`]s instead.
struct CommitRefIterRaw<'a> {
    data: &'a [u8],
    state: State,
    offset: usize,
    hash_kind: gix_hash::Kind,
}

impl<'a> Iterator for CommitRefIterRaw<'a> {
    type Item = Result<RawToken<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }
        match CommitRefIter::next_inner(self.data, &mut self.state, self.hash_kind) {
            Ok((remaining, token)) => {
                let consumed = self.data.len() - remaining.len();
                let start = self.offset;
                let end = start + consumed;
                self.offset = end;

                self.data = remaining;
                Some(Ok(RawToken {
                    token,
                    token_range: start..end,
                }))
            }
            Err(err) => {
                self.data = &[];
                Some(Err(err))
            }
        }
    }
}

/// A combination of a parsed [`Token`] as well as the range of bytes that were consumed to parse it.
struct RawToken<'a> {
    /// The parsed token.
    token: Token<'a>,
    token_range: Range<usize>,
}

/// A token returned by the [commit iterator][CommitRefIter].
#[expect(missing_docs)]
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
pub enum Token<'a> {
    Tree {
        id: ObjectId,
    },
    Parent {
        id: ObjectId,
    },
    /// A person who authored the content of the commit.
    Author {
        signature: gix_actor::SignatureRef<'a>,
    },
    /// A person who committed the authors work to the repository.
    Committer {
        signature: gix_actor::SignatureRef<'a>,
    },
    Encoding(&'a BStr),
    ExtraHeader((&'a BStr, Cow<'a, BStr>)),
    Message(&'a BStr),
}

impl Token<'_> {
    /// Return the object id of this token if it's a [tree][Token::Tree] or a [parent commit][Token::Parent].
    pub fn id(&self) -> Option<&oid> {
        match self {
            Token::Tree { id } | Token::Parent { id } => Some(id.as_ref()),
            _ => None,
        }
    }

    /// Return the owned object id of this token if it's a [tree][Token::Tree] or a [parent commit][Token::Parent].
    pub fn try_into_id(self) -> Option<ObjectId> {
        match self {
            Token::Tree { id } | Token::Parent { id } => Some(id),
            _ => None,
        }
    }
}
