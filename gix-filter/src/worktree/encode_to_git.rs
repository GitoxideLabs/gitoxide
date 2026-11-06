/// Whether or not to perform round-trip checks.
#[derive(Debug, Copy, Clone)]
pub enum RoundTripCheck {
    /// Assure that we can losslessly convert the UTF-8 result back to the original encoding or fail with an error.
    Fail,
    /// Do not check if the encoding is round-trippable.
    Skip,
}

pub(crate) mod function {
    use gix_encoding::Encoding;
    use gix_error::{Result, ResultExt, bail, validation};

    use super::RoundTripCheck;

    /// Decode `src` according to `src_encoding` to UTF-8 for storage in Git, replacing `buf`.
    ///
    /// Git's worktree rules require BOMs for UTF-16/32 and prohibit them for explicit endian variants.
    /// Empty input stays empty, including for encodings which normally require or produce a BOM.
    /// Unlike the filter pipeline, this function validates UTF-8 even if `src_encoding` is UTF-8.
    pub fn encode_to_git(src: &[u8], src_encoding: Encoding, buf: &mut Vec<u8>, round_trip: RoundTripCheck) -> Result {
        buf.clear();
        if src.is_empty() {
            return Ok(());
        }
        let utf16_bom = src.starts_with(b"\xff\xfe") || src.starts_with(b"\xfe\xff");
        let utf32_bom = src.starts_with(b"\xff\xfe\0\0") || src.starts_with(b"\0\0\xfe\xff");
        if matches!(src_encoding, Encoding::Utf16Le | Encoding::Utf16Be) && utf16_bom
            || matches!(src_encoding, Encoding::Utf32Le | Encoding::Utf32Be) && utf32_bom
        {
            bail!(validation(format!("BOM is prohibited for '{}'", src_encoding.name())));
        }
        if src_encoding == Encoding::Utf16 && !utf16_bom || src_encoding == Encoding::Utf32 && !utf32_bom {
            bail!(validation(format!("BOM is required for '{}'", src_encoding.name())));
        }
        gix_encoding::convert_to(src, src_encoding, Encoding::Utf8, buf)
            .or_raise(|| validation(format!("Could not decode input as '{}'", src_encoding.name())))?;
        if matches!(round_trip, RoundTripCheck::Fail) {
            let original = gix_encoding::convert(buf, Encoding::Utf8, src_encoding)
                .or_raise(|| validation(format!("Could not re-encode input as '{}'", src_encoding.name())))?;
            if original != src {
                bail!(validation(format!(
                    "Encoding from '{}' to 'UTF-8' and back is not the same",
                    src_encoding.name()
                )));
            }
        }
        Ok(())
    }
}
