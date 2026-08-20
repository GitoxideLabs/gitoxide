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
pub use parse::function::parse;

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
/// An object directory that was resolved before is skipped, and it is an error if an alternate points back
/// into the chain of directories that is currently being followed, as that would form a cycle.
pub fn resolve(objects_directory: PathBuf, current_dir: &std::path::Path) -> Result<Vec<PathBuf>, Error> {
    let mut dirs = vec![(None, objects_directory.clone())];
    let mut out = Vec::new();
    let mut seen = Vec::new();
    while let Some((parent_idx, dir)) = dirs.pop() {
        let dir_canonicalized = gix_path::realpath_opts(&dir, current_dir, MAX_SYMLINKS)?;
        if let Some(seen_idx) = seen.iter().position(|(seen_dir, _)| *seen_dir == dir_canonicalized) {
            if let Some(parent_idx) = parent_idx {
                if chain(&seen, parent_idx).any(|ancestor| ancestor == seen_idx) {
                    let mut cycle: Vec<_> = chain(&seen, parent_idx)
                        .take_while(|ancestor| *ancestor != seen_idx)
                        .map(|idx| seen[idx].0.clone())
                        .collect();
                    cycle.push(seen[seen_idx].0.clone());
                    cycle.reverse();
                    return Err(Error::Cycle(cycle));
                }
            }
            continue;
        }
        let idx = seen.len();
        seen.push((dir_canonicalized, parent_idx));
        match fs::read(dir.join("info").join("alternates")) {
            Ok(input) => {
                for path in parse(&input)?.into_iter().rev() {
                    dirs.push((Some(idx), objects_directory.join(path)));
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
        if parent_idx.is_some() {
            out.push(dir);
        }
    }
    Ok(out)
}

/// Yield `idx` and the indices of all directories it was reached through, starting at `idx`.
fn chain(seen: &[(PathBuf, Option<usize>)], idx: usize) -> impl Iterator<Item = usize> + '_ {
    let mut next = Some(idx);
    std::iter::from_fn(move || {
        let idx = next?;
        next = seen[idx].1;
        Some(idx)
    })
}
