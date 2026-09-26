use gix_error::Result;
use std::path::PathBuf;

use bstr::ByteSlice;
use gix_error::{ErrorExt, OptionExt, ResultExt};

/// Parse typical `gitdir` files as seen in worktrees and submodules.
/// Errors include the original `input` bytes as [metadata](gix_error::Error::metadata()).
pub fn gitdir(input: &[u8]) -> Result<PathBuf> {
    let path = input
        .strip_prefix(b"gitdir: ")
        .ok_or_raise(|| gix_error::validation("Format should be 'gitdir: <path>', but got").with("input", input))?
        .as_bstr();
    let path = path.trim_end().as_bstr();
    if path.is_empty() {
        return Err(gix_error::validation("Format should be 'gitdir: <path>', but got")
            .with("input", input)
            .raise());
    }
    Ok(gix_path::try_from_bstr(path)
        .or_raise(|| gix_error::validation("Couldn't decode input as UTF8").with("input", input))?
        .into_owned())
}
