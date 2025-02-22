//!
use std::path::Path;

/// The amount of retries to do during various aspects of the directory creation.
#[derive(Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct Retries {
    /// How many times the whole directory can be created in the light of racy interference.
    /// This count combats racy situations where another process is trying to remove a directory that we want to create,
    /// and is deliberately higher than those who do deletion. That way, creation usually wins.
    pub to_create_entire_directory: usize,
    /// The amount of times we can try to create a directory because we couldn't as the parent didn't exist.
    /// This amounts to the maximum subdirectory depth we allow to be created. Counts once per attempt to create the entire directory.
    pub on_create_directory_failure: usize,
    /// How often to retry to create a single directory if an interrupt happens, as caused by signals.
    pub on_interrupt: usize,
}

impl Default for Retries {
    fn default() -> Self {
        Retries {
            on_interrupt: 10,
            to_create_entire_directory: 5,
            on_create_directory_failure: 25,
        }
    }
}

mod error {
    use std::{fmt, path::Path};

    use crate::dir::create::Retries;

    /// The error returned by [all()][super::all()].
    #[allow(missing_docs)]
    #[derive(Debug)]
    pub enum Error<'a> {
        /// A failure we will probably recover from by trying again.
        Intermediate { dir: &'a Path, kind: std::io::ErrorKind },
        /// A failure that ends the operation.
        Permanent {
            dir: &'a Path,
            err: std::io::Error,
            /// The retries left after running the operation
            retries_left: Retries,
            /// The original amount of retries to allow determining how many were actually used
            retries: Retries,
        },
    }

    impl fmt::Display for Error<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Error::Intermediate { dir, kind } => write!(
                    f,
                    "Intermediae failure creating {:?} with error: {:?}",
                    dir.display(),
                    kind
                ),
                Error::Permanent {
                    err: _,
                    dir,
                    retries_left,
                    retries,
                } => write!(
                    f,
                    "Permanently failing to create directory {dir:?} ({retries_left:?} of {retries:?})",
                ),
            }
        }
    }

    impl std::error::Error for Error<'_> {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Permanent { err, .. } => Some(err),
                _ => None,
            }
        }
    }
}
pub use error::Error;

enum State {
    CurrentlyCreatingDirectories,
    SearchingUpwardsForExistingDirectory,
}

/// A special iterator which communicates its operation through results where…
///
/// * `Some(Ok(created_directory))` is yielded once or more success, followed by `None`
/// * `Some(Err(Error::Intermediate))` is yielded zero or more times while trying to create the directory.
/// * `Some(Err(Error::Permanent))` is yielded exactly once on failure.
pub struct Iter<'a> {
    cursors: Vec<&'a Path>,
    retries: Retries,
    original_retries: Retries,
    state: State,
}

/// Construction
impl<'a> Iter<'a> {
    /// Create a new instance that creates `target` when iterated with the default amount of [`Retries`].
    pub fn new(target: &'a Path) -> Self {
        Self::new_with_retries(target, Default::default())
    }

    /// Create a new instance that creates `target` when iterated with the specified amount of `retries`.
    pub fn new_with_retries(target: &'a Path, retries: Retries) -> Self {
        Iter {
            cursors: vec![target],
            original_retries: retries,
            retries,
            state: State::SearchingUpwardsForExistingDirectory,
        }
    }
}

impl<'a> Iter<'a> {
    fn permanent_failure(
        &mut self,
        dir: &'a Path,
        err: impl Into<std::io::Error>,
    ) -> Option<Result<&'a Path, Error<'a>>> {
        self.cursors.clear();
        Some(Err(Error::Permanent {
            err: err.into(),
            dir,
            retries_left: self.retries,
            retries: self.original_retries,
        }))
    }

    fn intermediate_failure(&self, dir: &'a Path, err: std::io::Error) -> Option<Result<&'a Path, Error<'a>>> {
        Some(Err(Error::Intermediate { dir, kind: err.kind() }))
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<&'a Path, Error<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        use std::io::ErrorKind::*;
        match self.cursors.pop() {
            Some(dir) => match std::fs::create_dir(dir) {
                Ok(()) => {
                    self.state = State::CurrentlyCreatingDirectories;
                    Some(Ok(dir))
                }
                Err(err) => match err.kind() {
                    AlreadyExists if dir.is_dir() => {
                        self.state = State::CurrentlyCreatingDirectories;
                        Some(Ok(dir))
                    }
                    AlreadyExists => self.permanent_failure(dir, err), // is non-directory
                    NotFound => {
                        self.retries.on_create_directory_failure -= 1;
                        if let State::CurrentlyCreatingDirectories = self.state {
                            self.state = State::SearchingUpwardsForExistingDirectory;
                            self.retries.to_create_entire_directory -= 1;
                            if self.retries.to_create_entire_directory < 1 {
                                return self.permanent_failure(dir, NotFound);
                            }
                            self.retries.on_create_directory_failure =
                                self.original_retries.on_create_directory_failure;
                        }
                        if self.retries.on_create_directory_failure < 1 {
                            return self.permanent_failure(dir, NotFound);
                        }
                        self.cursors.push(dir);
                        self.cursors.push(match dir.parent() {
                            None => return self.permanent_failure(dir, InvalidInput),
                            Some(parent) => parent,
                        });
                        self.intermediate_failure(dir, err)
                    }
                    Interrupted => {
                        self.retries.on_interrupt -= 1;
                        if self.retries.on_interrupt <= 1 {
                            return self.permanent_failure(dir, Interrupted);
                        }
                        self.cursors.push(dir);
                        self.intermediate_failure(dir, err)
                    }
                    _unexpected_kind => self.permanent_failure(dir, err),
                },
            },
            None => None,
        }
    }
}

/// Create all directories leading to `dir` including `dir` itself with the specified amount of `retries`.
/// Returns the input `dir` on success that make it useful in expressions.
pub fn all(dir: &Path, retries: Retries) -> std::io::Result<&Path> {
    for res in Iter::new_with_retries(dir, retries) {
        match res {
            Err(Error::Permanent { err, .. }) => return Err(err),
            Err(Error::Intermediate { .. }) | Ok(_) => continue,
        }
    }
    Ok(dir)
}
