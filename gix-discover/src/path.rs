use crate::{DOT_GIT_DIR, MODULES};
use std::ffi::OsStr;
use std::path::Path;
use std::{io::Read, path::PathBuf};

/// The kind of repository by looking exclusively at its `git_dir`.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum RepositoryKind {
    /// The repository resides in `.git/modules/`.
    Submodule,
    /// The repository resides in `.git/worktrees/`.
    LinkedWorktree,
    /// The repository is in a `.git` directory.
    Common,
}

///
pub mod from_gitdir_file {
    /// The error returned by [`from_gitdir_file()`][crate::path::from_gitdir_file()].
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Io(std::io::Error),
        Parse(crate::parse::gitdir::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Io(err) => std::fmt::Display::fmt(err, f),
                Error::Parse(err) => std::fmt::Display::fmt(err, f),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Io(err) => err.source(),
                Error::Parse(err) => err.source(),
            }
        }
    }

    impl From<std::io::Error> for Error {
        fn from(err: std::io::Error) -> Self {
            Error::Io(err)
        }
    }

    impl From<crate::parse::gitdir::Error> for Error {
        fn from(err: crate::parse::gitdir::Error) -> Self {
            Error::Parse(err)
        }
    }
}

fn read_regular_file_content_with_size_limit(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
    let mut file = std::fs::File::open(path)?;
    let max_file_size = 1024 * 64; // NOTE: git allows 1MB here
    let file_size = file.metadata()?.len();
    if file_size > max_file_size {
        return Err(std::io::Error::other(format!(
            "Refusing to open files larger than {} bytes, '{}' was {} bytes large",
            max_file_size,
            path.display(),
            file_size
        )));
    }
    let mut buf = Vec::with_capacity(512);
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

/// Read a plain path file, returning `None` if the file is missing.
///
/// Linked-worktree `gitdir` files are plain path files in Git, not `gitdir:`
/// files. Match Git's `get_linked_worktree()` behavior by trimming trailing
/// whitespace before interpreting the content. Empty or whitespace-only path
/// files are invalid.
fn read_plain_file_content(path: &std::path::Path) -> Option<std::io::Result<Vec<u8>>> {
    use bstr::ByteSlice;
    let mut buf = match read_regular_file_content_with_size_limit(path) {
        Ok(buf) => buf,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return None,
        Err(err) => return Some(Err(err)),
    };
    let trimmed_len = buf.trim_end().len();
    buf.truncate(trimmed_len);
    if buf.is_empty() {
        return Some(Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Refusing to read an empty path from '{}'", path.display()),
        )));
    }
    Some(Ok(buf))
}

/// Guess the kind of repository by looking at its `git_dir` path and return it.
/// Return `None` if `git_dir` isn't called `.git` or isn't within `.git/worktrees` or `.git/modules`, or if it's
/// a `.git` suffix like in `foo.git`.
/// The check for markers is case-sensitive under the assumption that nobody meddles with standard directories.
///
/// As this considers only the path, it cannot recognize linked worktrees of repositories whose Git directory isn't
/// named `.git`, such as natively bare repositories. Inspect the worktree's `commondir` file to identify those.
pub fn repository_kind(git_dir: &Path) -> Option<RepositoryKind> {
    if git_dir.file_name() == Some(OsStr::new(DOT_GIT_DIR)) {
        return Some(RepositoryKind::Common);
    }

    let mut last_comp = None;
    git_dir.components().rev().skip(1).any(|c| {
        if c.as_os_str() == OsStr::new(DOT_GIT_DIR) {
            true
        } else {
            last_comp = Some(c.as_os_str());
            false
        }
    });
    let last_comp = last_comp?;
    if last_comp == OsStr::new(MODULES) {
        RepositoryKind::Submodule.into()
    } else if last_comp == OsStr::new("worktrees") {
        RepositoryKind::LinkedWorktree.into()
    } else {
        None
    }
}

/// Reads a plain path from a file that contains it as its only content, with trailing whitespace trimmed.
///
/// Empty or whitespace-only path files are invalid.
pub fn from_plain_file(path: &std::path::Path) -> Option<std::io::Result<PathBuf>> {
    read_plain_file_content(path).map(|res| res.map(gix_path::from_bstring))
}

/// Reads a plain path from a file like [`from_plain_file()`], resolving relative paths against
/// the file's containing directory as needed.
///
/// The `path` argument is expected to name the path file itself, which is always supposed to be
/// a file path.
pub fn from_plain_file_relative_to_file(path: &std::path::Path) -> Option<std::io::Result<PathBuf>> {
    read_plain_file_content(path).map(|res| {
        res.and_then(|buf| {
            let plain_path = gix_path::from_bstring(buf);
            if !plain_path.is_relative() {
                return Ok(plain_path);
            }
            match path.parent() {
                Some(parent) => Ok(parent.join(plain_path)),
                _ => Err(std::io::Error::other(format!(
                    "'{path}' has no parent, but '{plain_path}' is relative. It's impossible",
                    path = path.display(),
                    plain_path = plain_path.display()
                ))),
            }
        })
    })
}

/// Reads typical `gitdir: ` files from disk as used by worktrees and submodules.
pub fn from_gitdir_file(path: &std::path::Path) -> Result<PathBuf, from_gitdir_file::Error> {
    let buf = read_regular_file_content_with_size_limit(path)?;
    let mut gitdir = crate::parse::gitdir(&buf)?;
    if let Some(parent) = path.parent() {
        gitdir = parent.join(gitdir);
    }
    Ok(gitdir)
}

/// Conditionally pop a trailing `.git` dir if present.
pub fn without_dot_git_dir(mut path: PathBuf) -> PathBuf {
    if path.file_name().and_then(std::ffi::OsStr::to_str) == Some(DOT_GIT_DIR) {
        path.pop();
    }
    path
}
