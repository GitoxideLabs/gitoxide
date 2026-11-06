#![deny(missing_docs)]
#![forbid(unsafe_code)]
//! Strict character conversion using Git's encoding names and `iconv` character mappings.
//!
//! [`Converter`] streams without per-conversion allocations after first-use table initialization.
//! [`read`] and [`mod@write`] adapt
//! standard I/O, retaining incomplete characters across calls with bounded memory. [`convert()`]
//! and [`convert_to()`] provide whole-buffer convenience with the same conversion semantics.
//! [`detect()`] inspects a complete byte slice, returning an encoding with supporting [`detect::Evidence`]
//! or explicit ambiguity instead of an uncalibrated probability. Prefer declared encodings when available.
//!
//! Conversion never inserts replacement characters or HTML entities. Invalid source sequences
//! and unrepresentable destination characters are errors. UTF-8 BOMs are ordinary characters;
//! UTF-16/32 consume BOMs only for their unsuffixed, byte-order-detecting variants. The explicit
//! `UTF-16LE-BOM` and `UTF-16BE-BOM` variants implement Git's output convention. Worktree-specific
//! BOM requirements, empty-file shortcuts, and roundtrip checking belong to `gix-filter`.
//!
//! Like Git's `reencode_string_len()`, finishing an ISO-2022-JP conversion does not append a
//! shift back to ASCII. Conversions are independent streams; do not concatenate their output
//! without understanding the destination encoding's state and BOM rules.
//!
//! See [`Encoding`] for the default repertoire and opt-in encoding features. Mapping tables are
//! embedded as zlib streams and decompressed lazily with `gix-zlib`. Each table is initialized
//! once, shared across threads, and retained for the rest of the process.
//!
//! ## Feature Flags
#![cfg_attr(all(doc, feature = "document-features"), doc = ::document_features::document_features!())]

mod encoding;
pub use encoding::Encoding;
/// Streaming conversion and supporting types for [`Converter`].
pub mod convert;
/// Types supporting encoding [`Detection`].
pub mod detect;
pub use detect::function::detect;
mod fast;
mod legacy;
mod table;
mod unicode;

/// Converting [`std::io::Read`] adapters.
pub mod read;
/// Converting [`std::io::Write`] adapters.
pub mod write;

use gix_error::{ErrorExt, Result, message};

/// A strict converter between two encodings, with no per-stream heap allocations.
///
/// Mapping tables are decompressed and cached on first use, which allocates shared storage.
/// Subsequent conversions reuse those tables without allocating, except for error diagnostics.
///
/// Incomplete source characters remain in the caller's input. Output characters, BOMs and
/// escape sequences are emitted atomically: provide at least 32 bytes of output space to
/// guarantee progress. The I/O adapters handle these buffering requirements automatically.
#[derive(Clone, Debug)]
pub struct Converter {
    from: Encoding,
    to: Encoding,
    source: legacy::State,
    destination: legacy::State,
    unicode_source: unicode::State,
    output_started: bool,
    offset: u64,
    finished: bool,
    failed: bool,
}

/// The outcome of detecting the encoding of a complete byte slice.
///
/// Detection is not a binary-file classifier and cannot recover an original encoding in general.
/// Use declared metadata (such as Git's `working-tree-encoding`) instead whenever available.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Detection {
    /// A preferred, strictly valid interpretation, with its supporting evidence.
    ///
    /// This is not a claim that other encodings are incompatible with the input.
    Detected {
        /// The source encoding, always using explicit byte order for UTF-16/32.
        encoding: Encoding,
        /// Why this interpretation was preferred over merely compatible ones.
        evidence: detect::Evidence,
        /// The leading BOM's length, or zero when no BOM was recognized.
        ///
        /// Convert `&input[bom_len..]` to omit the signature. Converting the entire input
        /// with `encoding` instead preserves it as U+FEFF, consistent with the raw codecs.
        bom_len: usize,
    },
    /// No interpretation has sufficient distinguishing evidence.
    Ambiguous {
        /// All enabled source encodings that strictly decode the entire input.
        ///
        /// These are in [`Encoding::ALL`] order, **not** ranked by likelihood. They need not
        /// produce the same text, or text at all. Latin-1, for example, accepts any bytes.
        /// Native-endian UTF-16/32 and Git's BOM aliases are omitted in favor of explicit
        /// byte-order variants; the output-only UTF-16BE-BOM variant is also excluded.
        candidates: Vec<Encoding>,
    },
    /// There are no bytes from which to infer an encoding; no fallback is imposed.
    Empty,
    /// A complete leading BOM was found, but its payload is invalid or truncated.
    ///
    /// No legacy fallback is attempted. Use [`crate::convert()`] on the payload for detailed
    /// conversion errors. As with valid BOMs, the longest signature takes precedence.
    InvalidBom {
        /// The BOM-indicated encoding, with explicit byte order for UTF-16/32.
        encoding: Encoding,
        /// The recognized BOM's length; the invalid payload follows it.
        bom_len: usize,
    },
}

/// Convert `input` from `from` to `to`, returning a newly allocated buffer.
pub fn convert(input: &[u8], from: Encoding, to: Encoding) -> Result<Vec<u8>> {
    let mut output = Vec::new();
    convert_to(input, from, to, &mut output)?;
    Ok(output)
}

/// Convert `input`, replacing `output` and reusing its allocation.
///
/// On error, `output` is empty. Use [`Converter`] to retain conversion state between chunks.
pub fn convert_to(input: &[u8], from: Encoding, to: Encoding, output: &mut Vec<u8>) -> Result<()> {
    output.clear();
    let mut converter = Converter::new(from, to);
    let mut read = 0;
    loop {
        let start = output.len();
        // Leave enough room for a character, BOM, and a buffered legacy character even at EOF.
        let additional = (input.len() - read).max(64);
        if let Err(err) = output.try_reserve(additional) {
            output.clear();
            return Err(err.and_raise(message("Could not allocate the encoded output")));
        }
        output.resize(start + additional, 0);
        let progress = match converter.convert(&input[read..], &mut output[start..], true) {
            Ok(progress) => progress,
            Err(err) => {
                output.clear();
                return Err(err);
            }
        };
        output.truncate(start + progress.written);
        read += progress.read;
        if progress.status == convert::Status::Finished {
            return Ok(());
        }
    }
}
