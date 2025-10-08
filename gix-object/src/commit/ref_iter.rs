use std::{borrow::Cow, ops::Range};

use bstr::BStr;
use gix_hash::{oid, ObjectId};
use winnow::{
    combinator::{alt, eof, opt, terminated},
    error::StrContext,
    prelude::*,
    token::take_till,
};

use crate::{
    bstr::ByteSlice,
    commit::{decode, SignedData, SIGNATURE_FIELD_NAME},
    parse::{self, NL},
    CommitRefIter,
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
    /// Create a commit iterator from data.
    pub fn from_bytes(data: &'a [u8]) -> CommitRefIter<'a> {
        CommitRefIter {
            data,
            state: State::default(),
        }
    }
}

/// Access
impl<'a> CommitRefIter<'a> {
    /// Parse `data` as commit and return its PGP signature, along with *all non-signature* data as [`SignedData`], or `None`
    /// if the commit isn't signed.
    ///
    /// This allows the caller to validate the signature by passing the signed data along with the signature back to the program
    /// that created it.
    pub fn signature(data: &'a [u8]) -> Result<Option<(Cow<'a, BStr>, SignedData<'a>)>, crate::decode::Error> {
        let mut signature_and_range = None;

        let raw_tokens = CommitRefIterRaw {
            data,
            state: State::default(),
            offset: 0,
        };
        for token in raw_tokens {
            let token = token?;
            if let Token::ExtraHeader((name, value)) = &token.token {
                if *name == SIGNATURE_FIELD_NAME {
                    // keep track of the signature range alongside the signature data,
                    // because all but the signature is the signed data.
                    signature_and_range = Some((value.clone(), token.token_range));
                    break;
                }
            }
        }

        Ok(signature_and_range.map(|(sig, signature_range)| (sig, SignedData { data, signature_range })))
    }

    /// Returns the object id of this commits tree if it is the first function called and if there is no error in decoding
    /// the data.
    ///
    /// Note that this method must only be called once or else will always return None while consuming a single token.
    /// Errors are coerced into options, hiding whether there was an error or not. The caller should assume an error if they
    /// call the method as intended. Such a squelched error cannot be recovered unless the objects data is retrieved and parsed again.
    /// `next()`.
    pub fn tree_id(&mut self) -> Result<ObjectId, crate::decode::Error> {
        let tree_id = self.next().ok_or_else(missing_field)??;
        Token::try_into_id(tree_id).ok_or_else(missing_field)
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
    pub fn committer(mut self) -> Result<gix_actor::SignatureRef<'a>, crate::decode::Error> {
        self.find_map(|t| match t {
            Ok(Token::Committer { signature }) => Some(Ok(signature)),
            Err(err) => Some(Err(err)),
            _ => None,
        })
        .ok_or_else(missing_field)?
    }

    /// Returns the author signature if there is no decoding error.
    ///
    /// It may contain white space surrounding it, and is exactly as parsed.
    pub fn author(mut self) -> Result<gix_actor::SignatureRef<'a>, crate::decode::Error> {
        self.find_map(|t| match t {
            Ok(Token::Author { signature }) => Some(Ok(signature)),
            Err(err) => Some(Err(err)),
            _ => None,
        })
        .ok_or_else(missing_field)?
    }

    /// Returns the message if there is no decoding error.
    ///
    /// It may contain white space surrounding it, and is exactly as
    //  parsed.
    pub fn message(mut self) -> Result<&'a BStr, crate::decode::Error> {
        self.find_map(|t| match t {
            Ok(Token::Message(msg)) => Some(Ok(msg)),
            Err(err) => Some(Err(err)),
            _ => None,
        })
        .transpose()
        .map(Option::unwrap_or_default)
    }
}

fn missing_field() -> crate::decode::Error {
    crate::decode::empty_error()
}

impl<'a> CommitRefIter<'a> {
    #[inline]
    fn next_inner(mut i: &'a [u8], state: &mut State) -> Result<(&'a [u8], Token<'a>), crate::decode::Error> {
        let input = &mut i;
        match Self::next_inner_(input, state) {
            Ok(token) => Ok((*input, token)),
            Err(err) => Err(crate::decode::Error::with_err(err, input)),
        }
    }

    fn next_inner_(
        input: &mut &'a [u8],
        state: &mut State,
    ) -> Result<Token<'a>, winnow::error::ErrMode<crate::decode::ParseError>> {
        use State::*;
        Ok(match state {
            Tree => {
                let tree = (|i: &mut _| parse::header_field(i, b"tree", parse::hex_hash))
                    .context(StrContext::Expected("tree <40 lowercase hex char>".into()))
                    .parse_next(input)?;
                *state = State::Parents;
                Token::Tree {
                    id: ObjectId::from_hex(tree).expect("parsing validation"),
                }
            }
            Parents => {
                let parent = opt(|i: &mut _| parse::header_field(i, b"parent", parse::hex_hash))
                    .context(StrContext::Expected("commit <40 lowercase hex char>".into()))
                    .parse_next(input)?;
                match parent {
                    Some(parent) => Token::Parent {
                        id: ObjectId::from_hex(parent).expect("parsing validation"),
                    },
                    None => {
                        *state = State::Signature {
                            of: SignatureKind::Author,
                        };
                        Self::next_inner_(input, state)?
                    }
                }
            }
            Signature { ref mut of } => {
                let who = *of;
                let (field_name, err_msg) = match of {
                    SignatureKind::Author => {
                        *of = SignatureKind::Committer;
                        (&b"author"[..], "author <signature>")
                    }
                    SignatureKind::Committer => {
                        *state = State::Encoding;
                        (&b"committer"[..], "committer <signature>")
                    }
                };
                let signature = (|i: &mut _| parse::header_field(i, field_name, parse::signature))
                    .context(StrContext::Expected(err_msg.into()))
                    .parse_next(input)?;
                match who {
                    SignatureKind::Author => Token::Author { signature },
                    SignatureKind::Committer => Token::Committer { signature },
                }
            }
            Encoding => {
                let encoding = opt(|i: &mut _| parse::header_field(i, b"encoding", take_till(0.., NL)))
                    .context(StrContext::Expected("encoding <encoding>".into()))
                    .parse_next(input)?;
                *state = State::ExtraHeaders;
                match encoding {
                    Some(encoding) => Token::Encoding(encoding.as_bstr()),
                    None => Self::next_inner_(input, state)?,
                }
            }
            ExtraHeaders => {
                let extra_header = opt(alt((
                    |i: &mut _| parse::any_header_field_multi_line(i).map(|(k, o)| (k.as_bstr(), Cow::Owned(o))),
                    |i: &mut _| {
                        parse::any_header_field(i, take_till(0.., NL))
                            .map(|(k, o)| (k.as_bstr(), Cow::Borrowed(o.as_bstr())))
                    },
                )))
                .context(StrContext::Expected("<field> <single-line|multi-line>".into()))
                .parse_next(input)?;
                match extra_header {
                    Some(extra_header) => Token::ExtraHeader(extra_header),
                    None => {
                        *state = State::Message;
                        Self::next_inner_(input, state)?
                    }
                }
            }
            Message => {
                let message = terminated(decode::message, eof).parse_next(input)?;
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
    type Item = Result<Token<'a>, crate::decode::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }
        match Self::next_inner(self.data, &mut self.state) {
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
}

impl<'a> Iterator for CommitRefIterRaw<'a> {
    type Item = Result<RawToken<'a>, crate::decode::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.data.is_empty() {
            return None;
        }
        match CommitRefIter::next_inner(self.data, &mut self.state) {
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
#[allow(missing_docs)]
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
