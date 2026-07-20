use std::path::PathBuf;

use bstr::ByteSlice;

///
pub mod gitdir {
    use bstr::BString;

    /// The error returned by [`parse::gitdir()`][super::gitdir()].
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        InvalidFormat { input: BString },
        IllformedUtf8 { input: BString },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::InvalidFormat { input } => {
                    write!(f, "Format should be 'gitdir: <path>', but got: {input:?}")
                }
                Error::IllformedUtf8 { input } => write!(f, "Couldn't decode {input:?} as UTF8"),
            }
        }
    }

    impl std::error::Error for Error {}
}

/// Parse typical `gitdir` files as seen in worktrees and submodules.
pub fn gitdir(input: &[u8]) -> Result<PathBuf, gitdir::Error> {
    let path = input
        .strip_prefix(b"gitdir: ")
        .ok_or_else(|| gitdir::Error::InvalidFormat { input: input.into() })?
        .as_bstr();
    let path = path.trim_end().as_bstr();
    if path.is_empty() {
        return Err(gitdir::Error::InvalidFormat { input: input.into() });
    }
    Ok(gix_path::try_from_bstr(path)
        .map_err(|_| gitdir::Error::IllformedUtf8 { input: input.into() })?
        .into_owned())
}
