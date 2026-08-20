/// The error returned by [`encode_to_worktree()][super::encode_to_worktree()].
pub type Error = gix_error::Exn<gix_error::ValidationError>;

pub(crate) mod function {
    use encoding_rs::EncoderResult;

    use super::Error;

    /// Encode `src_utf8`, which is assumed to be UTF-8 encoded, according to `worktree_encoding` for placement in the working directory,
    /// and write it to `buf`, possibly resizing it.
    /// Note that the encoding is always applied, there is no conditional even if `worktree_encoding` and the `src` encoding are the same.
    pub fn encode_to_worktree(
        src_utf8: &[u8],
        worktree_encoding: &'static encoding_rs::Encoding,
        buf: &mut Vec<u8>,
    ) -> Result<(), Error> {
        use gix_error::{ErrorExt, ResultExt};

        let mut encoder = worktree_encoding.new_encoder();
        let buf_len = encoder
            .max_buffer_length_from_utf8_if_no_unmappables(src_utf8.len())
            .ok_or_else(|| {
                gix_error::ValidationError::new(format!(
                    "Cannot convert input of {} UTF-8 bytes to target encoding without overflowing",
                    src_utf8.len()
                ))
                .raise()
            })?;
        buf.clear();
        buf.resize(buf_len, 0);
        let src = std::str::from_utf8(src_utf8)
            .or_raise(|| gix_error::ValidationError::new("Input was not UTF-8 encoded"))?;
        let (res, read, written) = encoder.encode_from_utf8_without_replacement(src, buf, true);
        match res {
            EncoderResult::InputEmpty => {
                assert!(
                    buf_len >= written,
                    "encoding_rs estimates the maximum amount of bytes written correctly"
                );
                assert_eq!(read, src_utf8.len(), "input buffer should be fully consumed");
                buf.truncate(written);
            }
            EncoderResult::OutputFull => {
                unreachable!("we assure that the output buffer is big enough as per the encoder's estimate")
            }
            EncoderResult::Unmappable(c) => {
                return Err(gix_error::ValidationError::new(format!(
                    "The character '{c}' could not be mapped to the {}",
                    worktree_encoding.name()
                ))
                .raise());
            }
        }
        Ok(())
    }
}
