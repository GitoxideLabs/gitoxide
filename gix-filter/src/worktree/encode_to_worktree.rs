/// The error returned by [`encode_to_worktree()][super::encode_to_worktree()].
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    Overflow {
        input_len: usize,
    },
    InputAsUtf8(std::str::Utf8Error),
    Unmappable {
        character: char,
        worktree_encoding: &'static str,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Overflow { input_len } => write!(
                f,
                "Cannot convert input of {input_len} UTF-8 bytes to target encoding without overflowing"
            ),
            Error::InputAsUtf8(_) => f.write_str("Input was not UTF-8 encoded"),
            Error::Unmappable {
                character,
                worktree_encoding,
            } => write!(
                f,
                "The character '{character}' could not be mapped to the {worktree_encoding}"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Overflow { .. } | Error::Unmappable { .. } => None,
            Error::InputAsUtf8(err) => Some(err),
        }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(err: std::str::Utf8Error) -> Self {
        Error::InputAsUtf8(err)
    }
}

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
        let mut encoder = worktree_encoding.new_encoder();
        let buf_len = encoder
            .max_buffer_length_from_utf8_if_no_unmappables(src_utf8.len())
            .ok_or(Error::Overflow {
                input_len: src_utf8.len(),
            })?;
        buf.clear();
        buf.resize(buf_len, 0);
        let src = std::str::from_utf8(src_utf8)?;
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
                return Err(Error::Unmappable {
                    worktree_encoding: worktree_encoding.name(),
                    character: c,
                });
            }
        }
        Ok(())
    }
}
