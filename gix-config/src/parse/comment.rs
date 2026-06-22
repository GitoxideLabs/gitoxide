use std::fmt::Display;

use bstr::BString;

use crate::parse::Comment;

impl Comment {
    pub(crate) fn to_owned_in(&self, backing: &[u8]) -> Comment {
        Comment {
            tag: self.tag,
            text: self.text.to_bstring_in(backing).into(),
        }
    }

    pub(crate) fn copy_to_backing_in(&self, source: &[u8], target: &mut Vec<u8>) -> Comment {
        Comment {
            tag: self.tag,
            text: self.text.copy_to_backing_in(source, target),
        }
    }

    /// Serialize this type into a `BString` for convenience.
    ///
    /// Note that `to_string()` can also be used, but might not be lossless.
    #[must_use]
    pub fn to_bstring(&self) -> BString {
        let mut buf = Vec::new();
        self.write_to(&mut buf).expect("io error impossible");
        buf.into()
    }

    /// Stream ourselves to the given `out`, in order to reproduce this comment losslessly.
    pub fn write_to(&self, mut out: impl std::io::Write) -> std::io::Result<()> {
        out.write_all(&[self.tag])?;
        out.write_all(self.text.as_slice())
    }

    pub(crate) fn write_to_in(&self, backing: &[u8], mut out: impl std::io::Write) -> std::io::Result<()> {
        out.write_all(&[self.tag])?;
        out.write_all(self.text.as_slice_in(backing))
    }
}

impl Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.to_bstring(), f)
    }
}

impl From<Comment> for BString {
    fn from(c: Comment) -> Self {
        c.to_bstring()
    }
}

impl From<&Comment> for BString {
    fn from(c: &Comment) -> Self {
        c.to_bstring()
    }
}
