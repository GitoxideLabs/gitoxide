/// Whether or not to perform round-trip checks.
#[derive(Debug, Copy, Clone)]
pub enum RoundTripCheck {
    /// Assure that we can losslessly convert the UTF-8 result back to the original encoding or fail with an error.
    Fail,
    /// Do not check if the encoding is round-trippable.
    Skip,
}

/// The error returned by [`encode_to_git()][super::encode_to_git()].
pub type Error = gix_error::ValidationError;

pub(crate) mod function {
    use encoding_rs::DecoderResult;

    use super::{Error, RoundTripCheck};

    /// Decode `src` according to `src_encoding` to `UTF-8` for storage in git and place it in `buf`.
    /// Note that the encoding is always applied, there is no conditional even if `src_encoding` already is `UTF-8`.
    pub fn encode_to_git(
        src: &[u8],
        src_encoding: &'static encoding_rs::Encoding,
        buf: &mut Vec<u8>,
        round_trip: RoundTripCheck,
    ) -> Result<(), Error> {
        let mut decoder = src_encoding.new_decoder_with_bom_removal();
        let buf_len = decoder
            .max_utf8_buffer_length_without_replacement(src.len())
            .ok_or_else(|| {
                gix_error::ValidationError::new(format!(
                    "Cannot convert input of {} bytes to UTF-8 without overflowing",
                    src.len()
                ))
            })?;
        buf.clear();
        buf.resize(buf_len, 0);
        let (res, read, written) = decoder.decode_to_utf8_without_replacement(src, buf, true);
        match res {
            DecoderResult::InputEmpty => {
                assert!(
                    buf_len >= written,
                    "encoding_rs estimates the maximum amount of bytes written correctly"
                );
                assert_eq!(read, src.len(), "input buffer should be fully consumed");
                buf.truncate(written);
            }
            DecoderResult::OutputFull => {
                unreachable!("we assure that the output buffer is big enough as per the encoder's estimate")
            }
            DecoderResult::Malformed(_, _) => {
                return Err(gix_error::ValidationError::new(format!(
                    "The input was malformed and could not be decoded as '{}'",
                    src_encoding.name()
                )));
            }
        }

        match round_trip {
            RoundTripCheck::Fail => {
                // SAFETY: we trust `encoding_rs` to output valid UTF-8 only if we ask it to.
                #[expect(unsafe_code)]
                let str = unsafe { std::str::from_utf8_unchecked(buf) };
                let (should_equal_src, _actual_encoding, _had_errors) = src_encoding.encode(str);
                if should_equal_src != src {
                    return Err(gix_error::ValidationError::new(format!(
                        "Encoding from '{}' to 'UTF-8' and back is not the same",
                        src_encoding.name()
                    )));
                }
            }
            RoundTripCheck::Skip => {}
        }
        Ok(())
    }
}
