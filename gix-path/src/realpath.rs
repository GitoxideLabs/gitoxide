/// The error returned by [`realpath()`][super::realpath()].
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    MaxSymlinksExceeded { max_symlinks: u8 },
    ExcessiveComponentCount { max_symlink_checks: usize },
    ReadLink(std::io::Error),
    CurrentWorkingDir(std::io::Error),
    EmptyPath,
    MissingParent,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::MaxSymlinksExceeded { max_symlinks } => write!(
                f,
                "The maximum allowed number {max_symlinks} of symlinks in path is exceeded"
            ),
            Error::ExcessiveComponentCount { max_symlink_checks } => write!(
                f,
                "Cannot resolve symlinks in path with more than {max_symlink_checks} components (takes too long)"
            ),
            Error::ReadLink(err) | Error::CurrentWorkingDir(err) => std::fmt::Display::fmt(err, f),
            Error::EmptyPath => f.write_str("Empty is not a valid path"),
            Error::MissingParent => f.write_str("Ran out of path components while following parent component '..'"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ReadLink(err) | Error::CurrentWorkingDir(err) => err.source(),
            Error::MaxSymlinksExceeded { .. }
            | Error::ExcessiveComponentCount { .. }
            | Error::EmptyPath
            | Error::MissingParent => None,
        }
    }
}

/// The default amount of symlinks we may follow when resolving a path in [`realpath()`][crate::realpath()].
pub const MAX_SYMLINKS: u8 = 32;

pub(crate) mod function {
    use std::path::{
        Component::{CurDir, Normal, ParentDir, Prefix, RootDir},
        Path, PathBuf,
    };

    use super::Error;
    use crate::realpath::MAX_SYMLINKS;

    /// Check each component of `path` and see if it is a symlink. If so, resolve it.
    /// Do not fail for non-existing components, but assume these are as is.
    ///
    /// If `path` is relative, the current working directory be used to make it absolute.
    /// Note that the returned path will be verbatim, and repositories with `core.precomposeUnicode`
    /// set will probably want to precompose the paths unicode.
    pub fn realpath(path: impl AsRef<Path>) -> Result<PathBuf, Error> {
        let path = path.as_ref();
        let cwd = path
            .is_relative()
            .then(std::env::current_dir)
            .unwrap_or_else(|| Ok(PathBuf::default()))
            .map_err(Error::CurrentWorkingDir)?;
        realpath_opts(path, &cwd, MAX_SYMLINKS)
    }

    /// The same as [`realpath()`], but allow to configure `max_symlinks` to configure how many symbolic links we are going to follow.
    /// This serves to avoid running into cycles or doing unreasonable amounts of work.
    pub fn realpath_opts(path: &Path, cwd: &Path, max_symlinks: u8) -> Result<PathBuf, Error> {
        if path.as_os_str().is_empty() {
            return Err(Error::EmptyPath);
        }

        let mut real_path = PathBuf::new();
        if path.is_relative() {
            real_path.push(cwd);
        }

        let mut num_symlinks = 0;
        let mut path_backing: PathBuf;
        let mut components = path.components();
        const MAX_SYMLINK_CHECKS: usize = 2048;
        let mut symlink_checks = 0;
        while let Some(component) = components.next() {
            match component {
                part @ (RootDir | Prefix(_)) => real_path.push(part),
                CurDir => {}
                ParentDir => {
                    if !real_path.pop() {
                        return Err(Error::MissingParent);
                    }
                }
                Normal(part) => {
                    real_path.push(part);
                    symlink_checks += 1;
                    if real_path.is_symlink() {
                        num_symlinks += 1;
                        if num_symlinks > max_symlinks {
                            return Err(Error::MaxSymlinksExceeded { max_symlinks });
                        }
                        let mut link_destination = std::fs::read_link(real_path.as_path()).map_err(Error::ReadLink)?;
                        if link_destination.is_absolute() {
                            // pushing absolute path to real_path resets it to the pushed absolute path
                        } else {
                            assert!(real_path.pop(), "we just pushed a component");
                        }
                        link_destination.extend(components);
                        path_backing = link_destination;
                        components = path_backing.components();
                    }
                    if symlink_checks > MAX_SYMLINK_CHECKS {
                        return Err(Error::ExcessiveComponentCount {
                            max_symlink_checks: MAX_SYMLINK_CHECKS,
                        });
                    }
                }
            }
        }
        Ok(real_path)
    }
}
