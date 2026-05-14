use gix_object::bstr::BStr;

pub use super::loose::reflog::{Error, create_or_update};

///
pub mod iter;
mod line;

/// A parsed ref log line.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct LineRef<'a> {
    /// The previous object id in hexadecimal. Use [`LineRef::previous_oid()`] to get a more usable form.
    pub previous_oid: &'a BStr,
    /// The new object id in hexadecimal. Use [`LineRef::new_oid()`] to get a more usable form.
    pub new_oid: &'a BStr,
    /// The signature of the currently configured committer.
    #[cfg_attr(feature = "serde", serde(borrow))]
    pub signature: gix_actor::SignatureRef<'a>,
    /// The message providing details about the operation performed in this log line.
    pub message: &'a BStr,
}
