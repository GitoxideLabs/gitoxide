use crate::{Converter, Detection, Encoding, convert::Status, detect::Evidence};

/// Detect an encoding for the **complete** `input`, without imposing a fallback on ambiguity.
///
/// A selected encoding or ambiguous candidate always strictly decodes all of its input,
/// including the final sequence. The sole exception is [`Detection::InvalidBom`], which
/// reports a signature without endorsing its malformed payload.
///
/// Detection applies these policies in order:
///
/// * Recognize UTF-8/16/32 BOMs at offset zero, longest first, and validate their payloads.
///   In particular, `FF FE 00 00` is treated as UTF-32LE, not UTF-16LE followed by NUL.
/// * Recognize BOM-less UTF-16/32 only if at least four code units are present, at least
///   half are ASCII text in a consistent byte order, the complete decoded text has no
///   unexpected controls, and only one byte-order variant meets those conditions.
/// * With the `asian` feature, recognize ISO-2022-JP designations before considering
///   ASCII-compatible input.
/// * Prefer ASCII for seven-bit text and UTF-8 for other valid UTF-8 text.
/// * Otherwise return compatible encodings, without language guessing or merging legacy
///   encoding families. Validity alone cannot distinguish, for example, Latin-1 from
///   Windows-1252 on bytes they share.
///
/// Except for BOM-marked input, a preferred interpretation must contain no control characters
/// other than tab, LF, FF, and CR. This deliberately leaves some real text ambiguous, and is
/// not proof that other input is binary. BOM-less, non-ASCII-heavy UTF-16/32 may also remain
/// ambiguous. There are no calibrated probabilities or statistical language models here.
///
/// Validation uses bounded scratch space rather than allocating decoded copies. Detection probes
/// only encodings enabled by the active features (see [`Encoding`]). Probing an encoding may
/// allocate on first use to lazily decompress and cache its mapping tables. The ambiguous path
/// additionally allocates the returned candidate vector and any codec error diagnostics; its time
/// is proportional to input length times the number of enabled encodings.
///
/// ```
/// use gix_encoding::{Detection, Encoding, convert, detect, detect::Evidence};
///
/// # fn main() -> gix_error::TestResult {
/// let input = b"\xff\xfeh\0i\0";
/// let Detection::Detected { encoding, evidence, bom_len } = detect(input) else {
///     panic!("the complete UTF-16LE BOM and valid payload identify an interpretation");
/// };
/// assert_eq!(encoding, Encoding::Utf16Le);
/// assert_eq!(evidence, Evidence::ByteOrderMark);
/// assert_eq!(convert(&input[bom_len..], encoding, Encoding::Utf8)?, b"hi");
/// assert!(matches!(detect(b"caf\xe9"), Detection::Ambiguous { .. }));
/// # Ok(())
/// # }
/// ```
pub fn detect(input: &[u8]) -> Detection {
    use Encoding::*;
    if input.is_empty() {
        return Detection::Empty;
    }
    for (bom, encoding) in [
        (&b"\xff\xfe\0\0"[..], Utf32Le),
        (&b"\0\0\xfe\xff"[..], Utf32Be),
        (&b"\xef\xbb\xbf"[..], Utf8),
        (&b"\xff\xfe"[..], Utf16Le),
        (&b"\xfe\xff"[..], Utf16Be),
    ] {
        if let Some(payload) = input.strip_prefix(bom) {
            return if probe(payload, encoding).is_some() {
                Detection::Detected {
                    encoding,
                    evidence: Evidence::ByteOrderMark,
                    bom_len: bom.len(),
                }
            } else {
                Detection::InvalidBom {
                    encoding,
                    bom_len: bom.len(),
                }
            };
        }
    }
    if input.contains(&0) {
        let mut unicode = [Utf16Le, Utf16Be, Utf32Le, Utf32Be]
            .into_iter()
            .filter(|&encoding| unicode_pattern(input, encoding) && probe(input, encoding) == Some(true));
        if let (Some(encoding), None) = (unicode.next(), unicode.next()) {
            return preferred(encoding, Evidence::UnicodeBytePattern);
        }
    }
    #[cfg(feature = "asian")]
    if input
        .windows(3)
        .any(|bytes| matches!(bytes, b"\x1b$@" | b"\x1b$B" | b"\x1b(J"))
        && probe(input, Iso2022Jp) == Some(true)
    {
        return preferred(Iso2022Jp, Evidence::Iso2022JpEscape);
    }
    if let Ok(text) = std::str::from_utf8(input)
        && text.chars().all(is_text)
    {
        return if input.is_ascii() {
            preferred(Ascii, Evidence::Ascii)
        } else {
            preferred(Utf8, Evidence::Utf8)
        };
    }
    Detection::Ambiguous {
        candidates: Encoding::ALL
            .iter()
            .copied()
            .filter(|encoding| !matches!(encoding, Utf16 | Utf32 | Utf16LeBom | Utf16BeBom))
            .filter(|&encoding| probe(input, encoding).is_some())
            .collect(),
    }
}

fn preferred(encoding: Encoding, evidence: Evidence) -> Detection {
    Detection::Detected {
        encoding,
        evidence,
        bom_len: 0,
    }
}

fn is_text(ch: char) -> bool {
    !ch.is_control() || matches!(ch, '\t' | '\n' | '\x0c' | '\r')
}

fn unicode_pattern(input: &[u8], encoding: Encoding) -> bool {
    let width = if matches!(encoding, Encoding::Utf16Le | Encoding::Utf16Be) {
        2
    } else {
        4
    };
    let units = input.len() / width;
    if units < 4 || !input.len().is_multiple_of(width) {
        return false;
    }
    let low = if matches!(encoding, Encoding::Utf16Le | Encoding::Utf32Le) {
        0
    } else {
        width - 1
    };
    let ascii = input
        .chunks_exact(width)
        .filter(|unit| {
            unit[low].is_ascii()
                && is_text(char::from(unit[low]))
                && unit.iter().enumerate().all(|(index, &byte)| index == low || byte == 0)
        })
        .count();
    ascii >= units - units / 2
}

/// `None` means invalid; otherwise the boolean indicates whether all decoded scalars are text-like.
fn probe(input: &[u8], encoding: Encoding) -> Option<bool> {
    let mut converter = Converter::new(encoding, Encoding::Utf8);
    let mut output = [0; 4096];
    let mut read = 0;
    let mut text_like = true;
    loop {
        let progress = converter.convert(&input[read..], &mut output, true).ok()?;
        // Converters emit whole destination characters, even across scratch-buffer boundaries.
        let text = std::str::from_utf8(&output[..progress.written]).expect("strict conversion emits complete UTF-8");
        text_like &= text.chars().all(is_text);
        read += progress.read;
        if progress.status == Status::Finished {
            return Some(text_like);
        }
    }
}
