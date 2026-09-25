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
use gix_error::Result;
use std::{fs, io, path::PathBuf};

use gix_error::{ErrorExt, Message, ResultExt};
use gix_path::realpath::MAX_SYMLINKS;

mod parse;
pub use parse::parse;

/// An alternate object directory points back into the chain being resolved.
#[derive(Debug)]
pub struct Cycle {
    /// Canonical object directories in traversal order, with an implicit link from the last to the first.
    pub paths: Vec<PathBuf>,
}

impl std::fmt::Display for Cycle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Alternates form a cycle")?;
        for path in &self.paths {
            write!(f, " -> {}", path.display())?;
        }
        Ok(())
    }
}

impl std::error::Error for Cycle {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(const { &gix_error::ClassificationMarker::CORRUPTION })
    }
}

/// Given an `objects_directory`, try to resolve alternate object directories possibly located in the
/// `./info/alternates` file into canonical paths and resolve relative paths with the help of the `current_dir`.
/// If no alternate object database was resolved, the resulting `Vec` is empty (it is not an error
/// if there are no alternates).
/// An object directory that was resolved before is skipped, and it is an error if an alternate points back
/// into the chain of directories that is currently being followed, as that would form a cycle.
/// Read and parse failures include [metadata](gix_error::Error::metadata()) `path` (native path), the alternates file.
/// Cycles retain their canonical directory chain in [`Cycle`].
pub fn resolve(objects_directory: PathBuf, current_dir: &std::path::Path) -> Result<Vec<PathBuf>> {
    let mut dirs = vec![(None, objects_directory.clone())];
    let mut out = Vec::new();
    let mut seen = Vec::new();
    while let Some((parent_idx, dir)) = dirs.pop() {
        let dir_canonicalized = gix_path::realpath_opts(&dir, current_dir, MAX_SYMLINKS)?;
        if let Some(seen_idx) = seen.iter().position(|(seen_dir, _)| *seen_dir == dir_canonicalized) {
            if let Some(parent_idx) = parent_idx
                && chain(&seen, parent_idx).any(|ancestor| ancestor == seen_idx)
            {
                let mut cycle: Vec<_> = chain(&seen, parent_idx)
                    .take_while(|ancestor| *ancestor != seen_idx)
                    .map(|idx| seen[idx].0.clone())
                    .collect();
                cycle.push(seen[seen_idx].0.clone());
                cycle.reverse();
                return Err(Cycle { paths: cycle }.raise().into());
            }
            continue;
        }
        let idx = seen.len();
        seen.push((dir_canonicalized, parent_idx));
        let path = dir.join("info").join("alternates");
        match fs::read(&path) {
            Ok(input) => {
                for path in parse(&input)
                    .or_raise_erased(|| Message::new("Could not parse alternates").with("path", path))?
                    .into_iter()
                    .rev()
                {
                    dirs.push((Some(idx), objects_directory.join(path)));
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(err
                    .and_raise(Message::new("Could not read alternates").with("path", path))
                    .into());
            }
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
