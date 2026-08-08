use bstr::BStr;
use encoding_rs::Encoding;

///
pub mod for_label {
    use bstr::BString;

    /// The error returned by [for_label()][super::for_label()].
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Unknown { name: BString },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Unknown { name } => write!(f, "An encoding named '{name}' is not known"),
            }
        }
    }

    impl std::error::Error for Error {}
}

/// Try to produce a new `Encoding` for `label` or report an error if it is not known.
///
/// ### Deviation
///
/// * There is no special handling of UTF-16LE/BE with checks if data contains a BOM or not, like `git` as we don't expect to have
///   data available here.
/// * Special `-BOM` suffixed versions of `UTF-16` encodings are not supported.
pub fn for_label<'a>(label: impl Into<&'a BStr>) -> Result<&'static Encoding, for_label::Error> {
    let mut label = label.into();
    if label == "latin-1" {
        label = "ISO-8859-1".into();
    }
    let enc = Encoding::for_label(label.as_ref()).ok_or_else(|| for_label::Error::Unknown { name: label.into() })?;
    Ok(enc)
}
