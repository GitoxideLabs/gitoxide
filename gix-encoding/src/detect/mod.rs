pub(crate) mod function;

/// The evidence for a preferred interpretation, not a probability or certainty about its origin.
///
/// Even a BOM can occur coincidentally in another encoding. These categories are deliberately
/// not numerically ranked: ASCII compatibility, for example, is exact, but does not distinguish
/// ASCII from UTF-8 or many legacy encodings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Evidence {
    /// A complete, leading Unicode byte-order mark and a strictly valid payload.
    ByteOrderMark,
    /// Non-ASCII, strictly valid UTF-8 without unexpected control characters.
    ///
    /// UTF-8 is preferred by convention; other encodings may also decode these bytes.
    Utf8,
    /// Seven-bit text without unexpected control characters.
    ///
    /// This identifies an ASCII-compatible interpretation, not the original encoding.
    Ascii,
    /// Repeated ASCII code units in a strictly valid, BOM-less UTF-16/32 stream.
    ///
    /// This is a heuristic, not evidence that excludes every other interpretation.
    UnicodeBytePattern,
    /// A recognized ISO-2022-JP designation in a strictly valid text stream.
    ///
    /// Produced only when the `asian` feature is enabled.
    Iso2022JpEscape,
}
