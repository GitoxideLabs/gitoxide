use bstr::BStr;
use gix_encoding::Encoding;
use gix_error::{OptionExt, Result};

/// Try to produce a new `Encoding` for `label` or report an error if it is not known.
///
/// Byte-order mark requirements are checked when converting worktree content to Git.
pub fn for_label<'a>(label: impl Into<&'a BStr>) -> Result<Encoding> {
    let label = label.into();
    let enc = Encoding::for_label(label.as_ref())
        .ok_or_raise(|| gix_error::message!("An encoding named '{label}' is not known").validation())?;
    Ok(enc)
}
