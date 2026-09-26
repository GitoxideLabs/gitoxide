use bstr::BStr;
use encoding_rs::Encoding;
use gix_error::{OptionExt, Result};

/// Try to produce a new `Encoding` for `label` or report an error if it is not known.
///
/// ### Deviation
///
/// * There is no special handling of UTF-16LE/BE with checks if data contains a BOM or not, like `git` as we don't expect to have
///   data available here.
/// * Special `-BOM` suffixed versions of `UTF-16` encodings are not supported.
pub fn for_label<'a>(label: impl Into<&'a BStr>) -> Result<&'static Encoding> {
    let mut label = label.into();
    if label == "latin-1" {
        label = "ISO-8859-1".into();
    }
    let enc = Encoding::for_label(label.as_ref())
        .ok_or_raise(|| gix_error::validation(format!("An encoding named '{label}' is not known")))?;
    Ok(enc)
}
