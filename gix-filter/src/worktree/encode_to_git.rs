/// Whether or not to perform round-trip checks.
#[derive(Debug, Copy, Clone)]
pub enum RoundTripCheck {
    /// Assure that we can losslessly convert the UTF-8 result back to the original encoding or fail with an error.
    Fail,
    /// Do not check if the encoding is round-trippable.
    Skip,
}

/// The error returned by [`encode_to_git()][super::encode_to_git()].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Cannot convert input of {input_len} bytes to UTF-8 without overflowing")]
    Overflow { input_len: usize },
    #[error("The input was malformed and could not be decoded as '{encoding}'")]
    Malformed { encoding: &'static str },
    #[error("Encoding from '{src_encoding}' to '{dest_encoding}' and back is not the same")]
    RoundTrip {
        src_encoding: &'static str,
        dest_encoding: &'static str,
    },
}

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
            .ok_or(Error::Overflow { input_len: src.len() })?;
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
                return Err(Error::Malformed {
                    encoding: src_encoding.name(),
                });
            }
        }

        match round_trip {
            RoundTripCheck::Fail => {
                // SAFETY: we trust `encoding_rs` to output valid UTF-8 only if we ask it to.
                #[allow(unsafe_code)]
                let str = unsafe { std::str::from_utf8_unchecked(buf) };
                let (should_equal_src, _actual_encoding, _had_errors) = src_encoding.encode(str);
                if should_equal_src != src {
                    return Err(Error::RoundTrip {
                        src_encoding: src_encoding.name(),
                        dest_encoding: "UTF-8",
                    });
                }
            }
            RoundTripCheck::Skip => {}
        }
        Ok(())
    }
}
