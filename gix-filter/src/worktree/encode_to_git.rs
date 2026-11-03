/// Whether or not to perform round-trip checks.
#[derive(Debug, Copy, Clone)]
pub enum RoundTripCheck {
    /// Assure that we can losslessly convert the UTF-8 result back to the original encoding or fail with an error.
    Fail,
    /// Do not check if the encoding is round-trippable.
    Skip,
}

pub(crate) mod function {
    use encoding_rs::DecoderResult;

    use gix_error::{OptionExt, Result, bail, validation};

    use super::RoundTripCheck;

    /// Decode `src` according to `src_encoding` to `UTF-8` for storage in git and place it in `buf`.
    /// Note that the encoding is always applied, there is no conditional even if `src_encoding` already is `UTF-8`.
    pub fn encode_to_git(
        src: &[u8],
        src_encoding: &'static encoding_rs::Encoding,
        buf: &mut Vec<u8>,
        round_trip: RoundTripCheck,
    ) -> Result {
        let mut decoder = src_encoding.new_decoder_with_bom_removal();
        let buf_len = decoder
            .max_utf8_buffer_length_without_replacement(src.len())
            .ok_or_raise(|| {
                validation(format!(
                    "Cannot convert input of {} bytes to UTF-8 without overflowing",
                    src.len()
                ))
            })?;
        buf.clear();
        // Decode into a `String` that reuses the allocation of `buf`, so the round-trip check can use it as `&str`
        // without validating it again.
        let mut decoded = String::from_utf8(std::mem::take(buf)).expect("an empty buffer is valid UTF-8");
        decoded.reserve(buf_len);
        let (res, read) = decoder.decode_to_string_without_replacement(src, &mut decoded, true);
        match res {
            DecoderResult::InputEmpty => {
                assert_eq!(read, src.len(), "input buffer should be fully consumed");
            }
            DecoderResult::OutputFull => {
                unreachable!("we assure that the output buffer is big enough as per the encoder's estimate")
            }
            DecoderResult::Malformed(_, _) => {
                bail!(validation(format!(
                    "The input was malformed and could not be decoded as '{}'",
                    src_encoding.name()
                )));
            }
        }

        match round_trip {
            RoundTripCheck::Fail => {
                let (should_equal_src, _actual_encoding, _had_errors) = src_encoding.encode(&decoded);
                if should_equal_src != src {
                    bail!(validation(format!(
                        "Encoding from '{}' to 'UTF-8' and back is not the same",
                        src_encoding.name()
                    )));
                }
            }
            RoundTripCheck::Skip => {}
        }
        *buf = decoded.into_bytes();
        Ok(())
    }
}
