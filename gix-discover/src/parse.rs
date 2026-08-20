use std::path::PathBuf;

use bstr::ByteSlice;

///
pub mod gitdir {
    /// The error returned by [`parse::gitdir()`][super::gitdir()].
    pub type Error = gix_error::ValidationError;
}

/// Parse typical `gitdir` files as seen in worktrees and submodules.
pub fn gitdir(input: &[u8]) -> Result<PathBuf, gitdir::Error> {
    let path = input
        .strip_prefix(b"gitdir: ")
        .ok_or_else(|| gitdir::Error::new_with_input("Format should be 'gitdir: <path>', but got", input))?
        .as_bstr();
    let path = path.trim_end().as_bstr();
    if path.is_empty() {
        return Err(gitdir::Error::new_with_input(
            "Format should be 'gitdir: <path>', but got",
            input,
        ));
    }
    Ok(gix_path::try_from_bstr(path)
        .map_err(|_| gitdir::Error::new_with_input("Couldn't decode input as UTF8", input))?
        .into_owned())
}
