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
#[derive(thiserror::Error, Debug)]
#[expect(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Realpath(#[from] gix_path::realpath::Error),
    #[error(transparent)]
    Parse(#[from] parse::Error),
    #[error("Alternates form a cycle: {} -> {}", .0.iter().map(|p| format!("'{}'", p.display())).collect::<Vec<_>>().join(" -> "), .0.first().expect("more than one directories").display())]
    Cycle(Vec<PathBuf>),
}

/// Given an `objects_directory`, try to resolve alternate object directories possibly located in the
/// `./info/alternates` file into canonical paths and resolve relative paths with the help of the `current_dir`.
/// If no alternate object database was resolved, the resulting `Vec` is empty (it is not an error
/// if there are no alternates).
/// An object directory that was resolved before is skipped, and it is an error if an alternate points back
/// into the chain of directories that is currently being followed, as that would form a cycle.
pub fn resolve(objects_directory: PathBuf, current_dir: &std::path::Path) -> Result<Vec<PathBuf>, Error> {
    let mut dirs = vec![(0, objects_directory.clone())];
    let mut out = Vec::new();
    let mut seen = vec![(
        gix_path::realpath_opts(&objects_directory, current_dir, MAX_SYMLINKS)?,
        None,
    )];
    while let Some((idx, dir)) = dirs.pop() {
        match fs::read(dir.join("info").join("alternates")) {
            Ok(input) => {
                for path in parse::content(&input)?.into_iter() {
                    let path = objects_directory.join(path);
                    let path_canonicalized = gix_path::realpath_opts(&path, current_dir, MAX_SYMLINKS)?;
                    match seen.iter().position(|(dir, _)| *dir == path_canonicalized) {
                        Some(seen_idx) => {
                            if chain(&seen, idx).any(|ancestor| ancestor == seen_idx) {
                                let mut cycle: Vec<_> = chain(&seen, idx).map(|idx| seen[idx].0.clone()).collect();
                                cycle.reverse();
                                return Err(Error::Cycle(cycle));
                            }
                        }
                        None => {
                            seen.push((path_canonicalized, Some(idx)));
                            dirs.push((seen.len() - 1, path));
                        }
                    }
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err.into()),
        }
        if idx != 0 {
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
