use bstr::{ByteSlice, ByteVec};

use crate::{
    clear_and_set_capacity,
    eol::{AttributesDigest, Configuration, Mode, Stats},
};

/// The error produced by [`convert_to_worktree()`].
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    OutOfMemory(std::collections::TryReserveError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::OutOfMemory(_) => f.write_str("Could not allocate buffer"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::OutOfMemory(err) => Some(err),
        }
    }
}

impl From<std::collections::TryReserveError> for Error {
    fn from(err: std::collections::TryReserveError) -> Self {
        Error::OutOfMemory(err)
    }
}

/// Convert all `\n` in `src` to `crlf` if `digest` and `config` indicate it, returning `true` if `buf` holds the result, or `false`
/// if no change was made after all.
pub fn convert_to_worktree(
    src: &[u8],
    digest: AttributesDigest,
    buf: &mut Vec<u8>,
    config: Configuration,
) -> Result<bool, Error> {
    if src.is_empty() || digest.to_eol(config) != Some(Mode::CrLf) {
        return Ok(false);
    }
    let stats = Stats::from_bytes(src);
    if !stats.will_convert_lf_to_crlf(digest, config) {
        return Ok(false);
    }

    clear_and_set_capacity(buf, src.len() + stats.lone_lf)?;

    let mut ofs = 0;
    while let Some(pos) = src[ofs..].find_byteset(b"\r\n") {
        match src[ofs + pos] {
            b'\r' => {
                if src.get(ofs + pos + 1) == Some(&b'\n') {
                    buf.push_str(&src[ofs..][..pos + 2]);
                    ofs += pos + 2;
                } else {
                    buf.push_str(&src[ofs..][..pos + 1]);
                    ofs += pos + 1;
                }
            }
            b'\n' => {
                buf.push_str(&src[ofs..][..pos]);
                buf.push_str(b"\r\n");
                ofs += pos + 1;
            }
            _ => unreachable!("would only find one of two possible values"),
        }
    }
    buf.push_str(&src[ofs..]);
    Ok(true)
}
