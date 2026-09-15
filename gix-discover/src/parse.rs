use std::path::PathBuf;

use bstr::ByteSlice;

/// Parse typical `gitdir` files as seen in worktrees and submodules.
pub fn gitdir(input: &[u8]) -> Result<PathBuf, gix_error::ValidationError> {
    let path = input
        .strip_prefix(b"gitdir: ")
        .ok_or_else(|| gix_error::ValidationError::new_with_input("Format should be 'gitdir: <path>', but got", input))?
        .as_bstr();
    let path = path.trim_end().as_bstr();
    if path.is_empty() {
        return Err(gix_error::ValidationError::new_with_input(
            "Format should be 'gitdir: <path>', but got",
            input,
        ));
    }
    Ok(gix_path::try_from_bstr(path)
        .map_err(|_| gix_error::ValidationError::new_with_input("Couldn't decode input as UTF8", input))?
        .into_owned())
}
