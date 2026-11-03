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
        .ok_or_raise(|| gix_error::message!("An encoding named '{label}' is not known").validation())?;
    Ok(enc)
}

/// Write `text` as UTF-16 to `out` and return `true` if `encoding` is `UTF-16LE` or `UTF-16BE`, or return `false`
/// without touching `out` for all other encodings.
///
/// `encoding_rs` decodes UTF-16, but following the WHATWG Encoding Standard, its encoders for `UTF-16LE` and `UTF-16BE`
/// produce UTF-8. Git writes UTF-16 for these, so we have to do it ourselves.
pub(crate) fn encode_utf16(text: &str, encoding: &'static Encoding, out: &mut Vec<u8>) -> bool {
    let to_bytes: fn(u16) -> [u8; 2] = if encoding == encoding_rs::UTF_16LE {
        u16::to_le_bytes
    } else if encoding == encoding_rs::UTF_16BE {
        u16::to_be_bytes
    } else {
        return false;
    };
    out.clear();
    out.reserve(text.len() * 2);
    out.extend(text.encode_utf16().flat_map(to_bytes));
    true
}
