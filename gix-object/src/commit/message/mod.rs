use std::borrow::Cow;

use crate::{
    CommitRef,
    bstr::{BStr, BString, ByteSlice, ByteVec},
    commit::MessageRef,
};

///
pub mod body;
mod decode;

impl<'a> CommitRef<'a> {
    /// Return exactly the same message as [`MessageRef::summary()`].
    pub fn message_summary(&self) -> Cow<'a, BStr> {
        summary(self.message)
    }

    /// Return an iterator over message trailers as obtained from the last paragraph of the commit message.
    /// Maybe empty.
    pub fn message_trailers(&self) -> body::Trailers<'a> {
        MessageRef::from_bytes(self.message)
            .body()
            .map_or(body::Trailers { cursor: &[] }, |body| body.trailers())
    }
}

/// Convenience methods
impl<'a> CommitRef<'a> {
    /// Get an iterator over all `Signed-off-by` trailers in the commit message.
    /// This is useful for finding who signed off on the commit.
    pub fn signed_off_by_trailers(&self) -> impl Iterator<Item = body::TrailerRef<'a>> {
        self.message_trailers().signed_off_by()
    }

    /// Get an iterator over `Co-authored-by` trailers in the commit message.
    /// This is useful for squashed commits that contain multiple authors.
    pub fn co_authored_by_trailers(&self) -> impl Iterator<Item = body::TrailerRef<'a>> {
        self.message_trailers().co_authored_by()
    }

    /// Get all authors mentioned in `Signed-off-by` and `Co-authored-by` trailers.
    /// This is useful for squashed commits that contain multiple authors.
    /// Returns a Vec of author strings that can include both signers and co-authors.
    pub fn author_trailers(&self) -> impl Iterator<Item = body::TrailerRef<'a>> {
        self.message_trailers().authors()
    }

    /// Get an iterator over all attribution-related trailers
    /// (`Signed-off-by,` `Co-authored-by`, `Acked-by`, `Reviewed-by`, `Tested-by`).
    /// This provides a comprehensive view of everyone who contributed to or reviewed the commit.
    /// Note that the same name may occur multiple times, it's not a unified list.
    pub fn attribution_trailers(&self) -> impl Iterator<Item = body::TrailerRef<'a>> {
        self.message_trailers().attributions()
    }
}

impl<'a> MessageRef<'a> {
    /// Parse the given `input` as a message.
    ///
    /// Note that this cannot fail as everything will be interpreted as title if there is no body separator.
    pub fn from_bytes(input: &'a [u8]) -> Self {
        let (title, body) = decode::message_title_and_body(input);
        MessageRef { title, body }
    }

    /// Produce a short commit summary for the message title.
    ///
    /// This means the following
    ///
    /// * Take the subject line which is delimited by two newlines (\n\n)
    /// * transform intermediate consecutive whitespace including \r into one space
    ///
    /// The resulting summary will have folded whitespace before a newline into spaces and stopped that process
    /// once two consecutive newlines are encountered.
    pub fn summary(&self) -> Cow<'a, BStr> {
        summary(self.title)
    }

    /// Further parse the body into non-trailer and trailers, which can be iterated from the returned [`BodyRef`].
    pub fn body(&self) -> Option<BodyRef<'a>> {
        self.body.map(|b| BodyRef::from_bytes(b))
    }
}

pub(crate) fn summary(message: &BStr) -> Cow<'_, BStr> {
    let message = message.trim();
    match message.find_byte(b'\n') {
        Some(mut pos) => {
            let mut out = BString::default();
            let mut previous_pos = None;
            loop {
                if let Some(previous_pos) = previous_pos {
                    if previous_pos + 1 == pos {
                        let len_after_trim = out.trim_end().len();
                        out.resize(len_after_trim, 0);
                        break out.into();
                    }
                }
                let message_to_newline = &message[previous_pos.map_or(0, |p| p + 1)..pos];

                if let Some(pos_before_whitespace) = message_to_newline.rfind_not_byteset(b"\t\n\x0C\r ") {
                    out.extend_from_slice(&message_to_newline[..=pos_before_whitespace]);
                }
                out.push_byte(b' ');
                previous_pos = Some(pos);
                match message.get(pos + 1..).and_then(|i| i.find_byte(b'\n')) {
                    Some(next_nl_pos) => pos += next_nl_pos + 1,
                    None => {
                        if let Some(slice) = message.get((pos + 1)..) {
                            out.extend_from_slice(slice);
                        }
                        break out.into();
                    }
                }
            }
        }
        None => message.as_bstr().into(),
    }
}

/// A reference to a message body, further parsed to only contain the non-trailer parts.
///
/// See [git-interpret-trailers](https://git-scm.com/docs/git-interpret-trailers) for more information
/// on what constitutes trailers and not that this implementation is only good for typical sign-off footer or key-value parsing.
///
/// Note that we only parse trailers from the bottom of the body.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
pub struct BodyRef<'a> {
    body_without_trailer: &'a BStr,
    start_of_trailer: &'a [u8],
}
