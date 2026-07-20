use std::{env, ffi::OsStr, path::PathBuf};

/// The error returned by [`gix_discover::upwards()`][crate::upwards()].
// TODO(review): the missing space in the `InvalidInput` message ("…\"tries…") replicates the
//                `thiserror` attribute text byte-for-byte.
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    CurrentDir(std::io::Error),
    InvalidInput {
        directory: PathBuf,
    },
    InaccessibleDirectory {
        path: PathBuf,
    },
    NoGitRepository {
        path: PathBuf,
    },
    NoGitRepositoryWithinCeiling {
        path: PathBuf,
        ceiling_height: usize,
    },
    NoGitRepositoryWithinFs {
        path: PathBuf,
        limit: PathBuf,
    },
    NoMatchingCeilingDir,
    NoTrustedGitRepository {
        path: PathBuf,
        candidate: PathBuf,
        required: gix_sec::Trust,
    },
    CheckTrust {
        path: PathBuf,
        err: std::io::Error,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CurrentDir(_) => f.write_str("Could not obtain the current working directory"),
            Error::InvalidInput { directory } => write!(
                f,
                "Relative path \"{}\"tries to reach beyond root filesystem",
                directory.display()
            ),
            Error::InaccessibleDirectory { path } => write!(
                f,
                "Failed to access a directory, or path is not a directory: '{}'",
                path.display()
            ),
            Error::NoGitRepository { path } => write!(
                f,
                "Could not find a git repository in '{}' or in any of its parents",
                path.display()
            ),
            Error::NoGitRepositoryWithinCeiling { path, ceiling_height } => write!(
                f,
                "Could not find a git repository in '{}' or in any of its parents within ceiling height of {}",
                path.display(),
                ceiling_height
            ),
            Error::NoGitRepositoryWithinFs { path, limit } => write!(
                f,
                "Could not find a git repository in '{}' or in any of its parents within device limits below '{}'",
                path.display(),
                limit.display()
            ),
            Error::NoMatchingCeilingDir => f.write_str(
                "None of the passed ceiling directories prefixed the git-dir candidate, making them ineffective.",
            ),
            Error::NoTrustedGitRepository { path, candidate, .. } => write!(
                f,
                "Could not find a trusted git repository in '{}' or in any of its parents, candidate at '{}' discarded",
                path.display(),
                candidate.display()
            ),
            Error::CheckTrust { path, .. } => {
                write!(f, "Could not determine trust level for path '{}'.", path.display())
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::CurrentDir(err) => Some(err),
            Error::CheckTrust { err, .. } => Some(err),
            Error::InvalidInput { .. }
            | Error::InaccessibleDirectory { .. }
            | Error::NoGitRepository { .. }
            | Error::NoGitRepositoryWithinCeiling { .. }
            | Error::NoGitRepositoryWithinFs { .. }
            | Error::NoMatchingCeilingDir
            | Error::NoTrustedGitRepository { .. } => None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::CurrentDir(err)
    }
}

/// How to obtain the trust level for a discovered repository.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum TrustPolicy {
    /// Determine trust from repository ownership and require it to be at least the given level.
    Required(gix_sec::Trust),
    /// Trust computation is skipped and the given trust level is assumed.
    Assume(gix_sec::Trust),
}

impl Default for TrustPolicy {
    fn default() -> Self {
        TrustPolicy::Required(gix_sec::Trust::Reduced)
    }
}

/// Options to help guide the [discovery][crate::upwards()] of repositories, along with their options
/// when instantiated.
pub struct Options<'a> {
    /// When discovering a repository, determine how trust should be obtained.
    ///
    /// This defaults to [`Required(Reduced)`][TrustPolicy::Required] as our default settings are geared towards avoiding abuse.
    /// Set it to `Required(Full)` to only see repositories that [are owned by the current user][gix_sec::Trust::from_path_ownership()],
    /// or [`TrustPolicy::Assume`] to skip trust computation and return the given trust level.
    pub trust: TrustPolicy,
    /// When discovering a repository, ignore any repositories that are located in these directories or any of their parents.
    ///
    /// Entries are made absolute and lexically normalized, but symlinks are not resolved. They must therefore use the
    /// physical, symlink-resolved spelling of the directory to match the path traversed during discovery.
    ///
    /// Note that we ignore ceiling directories if the search directory is directly on top of one, which by default is an error
    /// if `match_ceiling_dir_or_error` is true, the default.
    pub ceiling_dirs: Vec<PathBuf>,
    /// If true, default true, and `ceiling_dirs` is not empty, we expect at least one ceiling directory to
    /// contain our search dir or else there will be an error.
    pub match_ceiling_dir_or_error: bool,
    /// if `true` avoid crossing filesystem boundaries.
    /// Only supported on Unix-like systems.
    // TODO: test on Linux
    // TODO: Handle WASI once https://github.com/rust-lang/rust/issues/71213 is resolved
    pub cross_fs: bool,
    /// If true, limit discovery to `.git` directories.
    ///
    /// This  will fail to find typical bare repositories, but would find them if they happen to be named `.git`.
    /// Use this option if repos with worktrees are the only kind of repositories you are interested in for
    /// optimal discovery performance.
    pub dot_git_only: bool,
    /// If set, the _current working directory_ (absolute path) to use when resolving relative paths. Note that
    /// that this is merely an optimization for those who discover a lot of repositories in the same process.
    ///
    /// If unset, the current working directory will be obtained automatically.
    /// Note that the path here might or might not contained decomposed unicode, which may end up in a path
    /// relevant us, like the git-dir or the worktree-dir. However, when opening the repository, it will
    /// change decomposed unicode to precomposed unicode based on the value of `core.precomposeUnicode`, and we
    /// don't have to deal with that value here just yet.
    pub current_dir: Option<&'a std::path::Path>,
}

impl Default for Options<'_> {
    fn default() -> Self {
        Options {
            trust: TrustPolicy::default(),
            ceiling_dirs: vec![],
            match_ceiling_dir_or_error: true,
            cross_fs: false,
            dot_git_only: false,
            current_dir: None,
        }
    }
}

impl Options<'_> {
    /// Loads discovery options overrides from the environment.
    ///
    /// The environment variables are:
    /// - `GIT_CEILING_DIRECTORIES` for `ceiling_dirs`
    ///
    /// Note that `GIT_DISCOVERY_ACROSS_FILESYSTEM` for `cross_fs` is **not** read,
    /// as it requires parsing of `git-config` style boolean values.
    // TODO: test
    pub fn apply_environment(mut self) -> Self {
        let name = "GIT_CEILING_DIRECTORIES";
        if let Some(ceiling_dirs) = env::var_os(name) {
            self.ceiling_dirs = parse_ceiling_dirs(&ceiling_dirs);
        }
        self
    }
}

/// Parse a byte-string of `:`-separated paths into `Vec<PathBuf>`.
/// On Windows, paths are separated by `;`.
/// Non-absolute paths are discarded.
/// To match git, all paths are normalized, until an empty path is encountered.
pub(crate) fn parse_ceiling_dirs(ceiling_dirs: &OsStr) -> Vec<PathBuf> {
    let mut should_normalize = true;
    let mut out = Vec::new();
    for ceiling_dir in std::env::split_paths(ceiling_dirs) {
        if ceiling_dir.as_os_str().is_empty() {
            should_normalize = false;
            continue;
        }

        // Only absolute paths are allowed
        if ceiling_dir.is_relative() {
            continue;
        }

        let mut dir = ceiling_dir;
        if should_normalize {
            if let Ok(normalized) = gix_path::realpath(&dir) {
                dir = normalized;
            }
        }
        out.push(dir);
    }
    out
}

#[cfg(test)]
mod tests {

    #[test]
    #[cfg(unix)]
    fn parse_ceiling_dirs_from_environment_format() -> std::io::Result<()> {
        use std::{fs, os::unix::fs::symlink};

        use super::*;

        // Setup filesystem
        let dir = tempfile::tempdir().expect("success creating temp dir");
        let direct_path = dir.path().join("direct");
        let symlink_path = dir.path().join("symlink");
        fs::create_dir(&direct_path)?;
        symlink(&direct_path, &symlink_path)?;

        // Parse & build ceiling dirs string
        let symlink_str = symlink_path.to_str().expect("symlink path is valid utf8");
        let ceiling_dir_string = format!("{symlink_str}:relative::{symlink_str}");
        let ceiling_dirs = parse_ceiling_dirs(OsStr::new(ceiling_dir_string.as_str()));

        assert_eq!(ceiling_dirs.len(), 2, "Relative path is discarded");
        assert_eq!(
            ceiling_dirs[0],
            symlink_path.canonicalize().expect("symlink path exists"),
            "Symlinks are resolved"
        );
        assert_eq!(
            ceiling_dirs[1], symlink_path,
            "Symlink are not resolved after empty item"
        );

        dir.close()
    }

    #[test]
    #[cfg(windows)]
    fn parse_ceiling_dirs_from_environment_format() -> std::io::Result<()> {
        use std::{fs, os::windows::fs::symlink_dir};

        use super::*;

        // Setup filesystem
        let dir = tempfile::tempdir().expect("success creating temp dir");
        let direct_path = dir.path().join("direct");
        let symlink_path = dir.path().join("symlink");
        fs::create_dir(&direct_path)?;
        symlink_dir(&direct_path, &symlink_path)?;

        // Parse & build ceiling dirs string
        let symlink_str = symlink_path.to_str().expect("symlink path is valid utf8");
        let ceiling_dir_string = format!("{};relative;;{}", symlink_str, symlink_str);
        let ceiling_dirs = parse_ceiling_dirs(OsStr::new(ceiling_dir_string.as_str()));

        assert_eq!(ceiling_dirs.len(), 2, "Relative path is discarded");
        assert_eq!(ceiling_dirs[0], direct_path, "Symlinks are resolved");
        assert_eq!(
            ceiling_dirs[1], symlink_path,
            "Symlink are not resolved after empty item"
        );

        dir.close()
    }
}
