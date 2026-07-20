//! A file with directories of other git object databases to use when reading objects.
//!
//! This inherently makes alternates read-only.
//!
//! An alternate file in `<git-dir>/info/alternates` can look as follows:
//!
//! ```text
//! # a comment, empty lines are also allowed
//! # relative paths resolve relative to the parent git repository
//! ../path/relative/to/repo/.git
//! /absolute/path/to/repo/.git
//!
//! "/a/ansi-c-quoted/path/with/tabs\t/.git"
//!
//! # each .git directory should indeed be a directory, and not a file
//! ```
//!
//! Based on the [canonical implementation](https://github.com/git/git/blob/master/sha1-file.c#L598:L609).
use std::{fs, io, path::PathBuf};

use gix_path::realpath::MAX_SYMLINKS;

///
pub mod parse;

/// Returned by [`resolve()`]
#[derive(Debug)]
#[allow(missing_docs)]
pub enum Error {
    Io(io::Error),
    Realpath(gix_path::realpath::Error),
    Parse(parse::Error),
    Cycle(Vec<PathBuf>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(err) => std::fmt::Display::fmt(err, f),
            Error::Realpath(err) => std::fmt::Display::fmt(err, f),
            Error::Parse(err) => std::fmt::Display::fmt(err, f),
            Error::Cycle(paths) => write!(
                f,
                "Alternates form a cycle: {} -> {}",
                paths
                    .iter()
                    .map(|p| format!("'{}'", p.display()))
                    .collect::<Vec<_>>()
                    .join(" -> "),
                paths.first().expect("more than one directories").display()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(err) => err.source(),
            Error::Realpath(err) => err.source(),
            Error::Parse(err) => err.source(),
            Error::Cycle(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::Io(err)
    }
}

impl From<gix_path::realpath::Error> for Error {
    fn from(err: gix_path::realpath::Error) -> Self {
        Error::Realpath(err)
    }
}

impl From<parse::Error> for Error {
    fn from(err: parse::Error) -> Self {
        Error::Parse(err)
    }
}

/// Given an `objects_directory`, try to resolve alternate object directories possibly located in the
/// `./info/alternates` file into canonical paths and resolve relative paths with the help of the `current_dir`.
/// If no alternate object database was resolved, the resulting `Vec` is empty (it is not an error
/// if there are no alternates).
/// It is an error once a repository is seen again as it would lead to a cycle.
pub fn resolve(objects_directory: PathBuf, current_dir: &std::path::Path) -> Result<Vec<PathBuf>, Error> {
    let mut dirs = vec![(0, objects_directory.clone())];
    let mut out = Vec::new();
    let mut seen = vec![gix_path::realpath_opts(&objects_directory, current_dir, MAX_SYMLINKS)?];
    while let Some((depth, dir)) = dirs.pop() {
        match fs::read(dir.join("info").join("alternates")) {
            Ok(input) => {
                for path in parse::content(&input)?.into_iter() {
                    let path = objects_directory.join(path);
                    let path_canonicalized = gix_path::realpath_opts(&path, current_dir, MAX_SYMLINKS)?;
                    if seen.contains(&path_canonicalized) {
                        return Err(Error::Cycle(seen));
                    }
                    seen.push(path_canonicalized);
                    dirs.push((depth + 1, path));
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
        if depth != 0 {
            out.push(dir);
        }
    }
    Ok(out)
}
