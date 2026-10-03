use std::path::Path;

use gix_error::{ClassificationMarker, ErrorExt, ResultExt, bail, message};

use crate::{Result, bstr::BString};
use gix_features::progress::{Count, NestedProgress, Progress};
use gix_path::realpath::MAX_SYMLINKS;

pub use gix_worktree::remove::Options;

/// The amount of validation to bypass when removing a linked worktree.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Force {
    /// Refuse to remove a dirty worktree or one containing initialized submodules.
    #[default]
    Never,
    /// Discard changes and initialized submodules, but retain locked worktrees.
    DiscardChanges,
    /// Also override a worktree lock.
    OverrideLock,
}

impl Force {
    fn discards_changes(self) -> bool {
        !matches!(self, Force::Never)
    }
}

/// A resolved linked worktree ready for inspection or removal.
pub struct Target<'repo> {
    /// The selected worktree's registration, providing access to its private Git directory, lock, and repository.
    proxy: crate::worktree::Proxy<'repo>,
    /// The checkout path cached from the registration's `gitdir` file during target resolution.
    /// The checkout itself may be missing or inaccessible.
    base: std::path::PathBuf,
    /// Options controlling traversal, deletion parallelism, and retries for both removal roots.
    options: Options,
}

impl Target<'_> {
    /// Return the absolute checkout directory, whether or not it is currently accessible.
    pub fn base(&self) -> &Path {
        &self.base
    }

    /// Open this worktree's repository, even if its checkout is currently inaccessible.
    pub fn repository(&self) -> Result<crate::Repository> {
        self.proxy.clone().into_repo_with_possibly_inaccessible_worktree()
    }

    /// Set the options for recursively deleting the checkout and its private Git directory.
    ///
    /// Defaults to [`Options::default()`]. This does not affect the safety checks controlled by [`Force`].
    pub fn options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    /// Validate and remove this linked worktree.
    ///
    /// The main worktree cannot be prepared for removal or removed. With [`Force::Never`], modified or untracked
    /// files and initialized submodules prevent removal. [`Force::DiscardChanges`] permits their deletion, while
    /// a locked worktree additionally requires [`Force::OverrideLock`].
    ///
    /// The worktree's attached branch, if any, is retained.
    ///
    /// `status.showUntrackedFiles` does not affect the untracked-file safety check.
    ///
    /// The calling repository's own linked worktree may also be removed. Callers doing so should leave the
    /// checkout before removal, particularly on Windows where a process working directory can prevent deletion.
    ///
    /// Recursive removal uses [`Options::default()`] unless overridden with [`Target::options()`].
    /// See [`gix_worktree::remove()`] for details about recursive removal, retries, and partial failures.
    ///
    /// # Selecting a worktree
    ///
    /// Obtain a target with [`Repository::prepare_remove_worktree()`][crate::Repository::prepare_remove_worktree()],
    /// which accepts an absolute or relative worktree path or a unique suffix of whole path components.
    /// Relative target paths use the process's current directory, while registered paths use the directory
    /// captured when the repository was opened.
    /// The returned target can be inspected with [`Target::repository()`] before it is consumed by this method.
    ///
    /// # Examples
    ///
    /// Suppose the main worktree is `/projects/main`, the linked worktrees are `/projects/feature/topic`
    /// and `/projects/release/topic`, and the current directory is `/projects/main`.
    /// These targets all select `/projects/feature/topic` without removing it:
    ///
    /// ```no_run
    /// # fn example(repo: &gix::Repository) -> gix::Result<()> {
    /// // Absolute and relative paths to the linked worktree.
    /// repo.prepare_remove_worktree("/projects/feature/topic")?;
    /// repo.prepare_remove_worktree("../feature/topic")?;
    ///
    /// // A unique suffix of whole path components, followed by inspection.
    /// let target = repo.prepare_remove_worktree("feature/topic")?;
    /// let linked_repo = target.repository()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Here, `"topic"` matches both linked worktrees and reports [`Error::Ambiguous`]; use `"feature/topic"`
    /// or `"release/topic"` to disambiguate. A partial component such as `"opic"` does not match either worktree.
    pub fn remove<P>(self, force: Force, mut progress: P) -> Result<()>
    where
        P: NestedProgress,
        P::SubProgress: NestedProgress + 'static,
    {
        let Target {
            proxy,
            base: work_dir,
            options,
        } = self;
        let work_dir = proxy.parent.current_dir().join(work_dir);
        let mut validation = progress.add_child("validate");
        validation.init(Some(1), gix_features::progress::count("worktree"));
        let git_dir = proxy.parent.current_dir().join(proxy.git_dir());
        let ignore_case = proxy.parent.config.ignore_case;

        if force != Force::OverrideLock && proxy.is_locked() {
            bail!(Error::Locked {
                path: work_dir,
                reason: proxy.lock_reason(),
            });
        }

        match std::fs::symlink_metadata(&work_dir) {
            Err(err) if gix_fs::io_err::is_not_found(err.kind(), err.raw_os_error()) => {}
            Err(err) => {
                bail!(err.and_raise(message("Could not read the location of a linked worktree")));
            }
            Ok(_) => {
                validate_backlink(&work_dir, &git_dir, ignore_case)?;
                if !force.discards_changes() {
                    let linked_repo = proxy
                        .into_repo_with_possibly_inaccessible_worktree()
                        .or_raise(|| message("Could not open the linked worktree repository"))?;
                    if has_populated_submodule(&linked_repo)? {
                        bail!(Error::ContainsSubmodule { path: work_dir });
                    }
                    let dirwalk_options = linked_repo
                        .dirwalk_options()
                        .or_raise(|| message("Could not prepare the linked worktree directory walk"))?;
                    let status = validation.add_child("status");
                    let mut iter = linked_repo
                        .status(status)
                        .or_raise(|| message("Could not prepare status for the linked worktree"))?
                        .index_worktree_options_mut(|opts| {
                            // Restore the walk even if status.showUntrackedFiles=no disabled it.
                            opts.dirwalk_options = Some(dirwalk_options);
                        })
                        .untracked_files(crate::status::UntrackedFiles::Files)
                        .index_worktree_submodules(crate::status::Submodule::AsConfigured { check_dirty: true })
                        .into_iter(None)
                        .or_raise(|| message("Could not inspect the linked worktree status"))?;
                    if iter
                        .next()
                        .transpose()
                        .or_raise(|| message("Could not inspect a linked worktree status item"))?
                        .is_some()
                    {
                        bail!(Error::Dirty { path: work_dir });
                    }
                }
            }
        }
        validation.inc();
        drop(validation);

        gix_worktree::remove(work_dir, git_dir, progress, options)
            .or_raise(|| message("Could not remove the linked worktree"))?;
        Ok(())
    }
}

impl crate::worktree::Proxy<'_> {
    /// Remove this linked worktree at its registered checkout path, using [`Options::default()`].
    ///
    /// No path or suffix lookup is performed. See [`Target::remove()`] for removal behavior and examples.
    pub fn remove<P>(self, force: Force, progress: P) -> Result<()>
    where
        P: NestedProgress,
        P::SubProgress: NestedProgress + 'static,
    {
        let base = self
            .base()
            .or_raise(|| message("Could not read the linked worktree checkout location"))?;
        Target {
            proxy: self,
            base,
            options: Options::default(),
        }
        .remove(force, progress)
    }
}

/// A rejection reported while preparing or removing a linked worktree, as described by [`Target::remove()`].
/// These errors are retained in the returned error chain for callers to inspect.
///
/// Missing targets are classified as [`gix_error::Class::NotFound`], mismatched backlinks as
/// [`gix_error::Class::Corruption`], and locked, dirty, or populated-submodule worktrees as
/// [`gix_error::Class::Conflict`]. Empty, ambiguous, or main-worktree targets are invalid input and
/// classified as [`gix_error::Class::Validation`].
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    EmptyTarget,
    NotFound {
        target: std::path::PathBuf,
    },
    Ambiguous {
        target: std::path::PathBuf,
        candidates: Vec<std::path::PathBuf>,
    },
    MainWorktree {
        path: std::path::PathBuf,
    },
    Locked {
        path: std::path::PathBuf,
        reason: Option<BString>,
    },
    BacklinkMismatch {
        path: std::path::PathBuf,
        expected: std::path::PathBuf,
        actual: std::path::PathBuf,
    },
    ContainsSubmodule {
        path: std::path::PathBuf,
    },
    Dirty {
        path: std::path::PathBuf,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn display_reason(reason: Option<&BString>) -> String {
            reason.map_or_else(String::new, |reason| format!(": {reason}"))
        }
        match self {
            Error::EmptyTarget => f.write_str("The worktree name or path cannot be empty"),
            Error::NotFound { target } => write!(f, "\"{}\" is not a registered worktree", target.display()),
            Error::Ambiguous { target, candidates } => {
                write!(f, "\"{}\" is ambiguous and matches {candidates:?}", target.display())
            }
            Error::MainWorktree { path } => write!(f, "The main worktree \"{}\" cannot be removed", path.display()),
            Error::Locked { path, reason } => write!(
                f,
                "The worktree \"{}\" is locked{}",
                path.display(),
                display_reason(reason.as_ref())
            ),
            Error::BacklinkMismatch { path, expected, actual } => write!(
                f,
                "The .git file at \"{}\" points to \"{}\", not \"{}\"",
                path.display(),
                actual.display(),
                expected.display()
            ),
            Error::ContainsSubmodule { path } => {
                write!(f, "The worktree '{}' contains an initialized submodule", path.display())
            }
            Error::Dirty { path } => write!(
                f,
                "The worktree \"{}\" contains modified or untracked files",
                path.display()
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Error::NotFound { .. } => const { &ClassificationMarker::NOT_FOUND },
            Error::BacklinkMismatch { .. } => const { &ClassificationMarker::CORRUPTION },
            Error::EmptyTarget | Error::Ambiguous { .. } | Error::MainWorktree { .. } => {
                const { &ClassificationMarker::VALIDATION }
            }
            Error::Locked { .. } | Error::ContainsSubmodule { .. } | Error::Dirty { .. } => {
                const { &ClassificationMarker::CONFLICT }
            }
        })
    }
}

impl crate::Repository {
    /// Resolve `target` for later inspection and removal.
    ///
    /// See [`Target::remove()`] for target selection, removal behavior, and examples.
    pub fn prepare_remove_worktree(&self, target: impl AsRef<Path>) -> Result<Target<'_>> {
        resolve(self, target.as_ref())
    }

    /// Resolve `target` and remove the linked worktree using [`Options::default()`].
    ///
    /// Combines [`Self::prepare_remove_worktree()`] with [`Target::remove()`], which documents target selection,
    /// removal behavior, and examples.
    pub fn remove_worktree<P>(&self, target: impl AsRef<Path>, force: Force, progress: P) -> Result<()>
    where
        P: NestedProgress,
        P::SubProgress: NestedProgress + 'static,
    {
        self.prepare_remove_worktree(target)?.remove(force, progress)
    }
}

fn resolve<'repo>(repo: &'repo crate::Repository, target: &Path) -> Result<Target<'repo>> {
    enum Match<'repo> {
        Main(std::path::PathBuf),
        Linked(Target<'repo>),
    }
    if target.as_os_str().is_empty() {
        bail!(Error::EmptyTarget);
    }
    let main = repo
        .main_repo()
        .or_raise(|| message("Could not open the linked worktree repository"))?
        .workdir()
        .map(Path::to_owned);
    let ignore_case = repo.config.ignore_case;
    let mut linked = Vec::new();
    for proxy in repo
        .worktrees()
        .or_raise(|| message("Could not list linked worktrees"))?
    {
        let Ok(base) = proxy.base() else { continue };
        linked.push(Target {
            proxy,
            base,
            options: Options::default(),
        });
    }

    let mut suffix_matches = Vec::new();
    if let Some(path) = main.as_ref()
        && path_ends_with(path, target, ignore_case)
    {
        suffix_matches.push(Match::Main(path.clone()));
    }
    for candidate in &linked {
        if path_ends_with(&candidate.base, target, ignore_case) {
            suffix_matches.push(Match::Linked(Target {
                proxy: candidate.proxy.clone(),
                base: candidate.base.clone(),
                options: candidate.options,
            }));
        }
    }
    if suffix_matches.len() == 1 {
        return match suffix_matches.pop().expect("exactly one match") {
            Match::Main(path) => Err(Error::MainWorktree { path }.raise()),
            Match::Linked(candidate) => Ok(candidate),
        };
    }

    let resolved_target =
        gix_path::realpath(target).or_raise(|| message!("Could not resolve worktree path \"{}\"", target.display()))?;
    let mut exact_matches = Vec::new();
    if let Some(path) = main {
        let resolved = gix_path::realpath_opts(&path, repo.current_dir(), MAX_SYMLINKS)
            .or_raise(|| message!("Could not resolve worktree path \"{}\"", path.display()))?;
        if path_eq(&resolved, &resolved_target, ignore_case) {
            exact_matches.push(Match::Main(path));
        }
    }
    for candidate in linked {
        let resolved = gix_path::realpath_opts(&candidate.base, repo.current_dir(), MAX_SYMLINKS)
            .or_raise(|| message!("Could not resolve worktree path \"{}\"", candidate.base.display()))?;
        if path_eq(&resolved, &resolved_target, ignore_case) {
            exact_matches.push(Match::Linked(candidate));
        }
    }

    match exact_matches.len() {
        0 if suffix_matches.is_empty() => Err(Error::NotFound {
            target: target.to_owned(),
        }
        .raise()),
        0 => Err(Error::Ambiguous {
            target: target.to_owned(),
            candidates: suffix_matches
                .into_iter()
                .map(|candidate| match candidate {
                    Match::Main(path) => path,
                    Match::Linked(candidate) => candidate.base,
                })
                .collect(),
        }
        .raise()),
        1 => match exact_matches.pop().expect("exactly one match") {
            Match::Main(path) => Err(Error::MainWorktree { path }.raise()),
            Match::Linked(candidate) => Ok(candidate),
        },
        _ => Err(Error::Ambiguous {
            target: target.to_owned(),
            candidates: exact_matches
                .into_iter()
                .map(|candidate| match candidate {
                    Match::Main(path) => path,
                    Match::Linked(candidate) => candidate.base,
                })
                .collect(),
        }
        .raise()),
    }
}

fn validate_backlink(work_dir: &Path, git_dir: &Path, ignore_case: bool) -> Result<()> {
    let path = work_dir.join(gix_discover::DOT_GIT_DIR);
    let actual = gix_discover::path::from_gitdir_file(&path)
        .or_raise(|| message!("Could not read the .git file at \"{}\"", path.display()))?;
    let actual = gix_path::realpath(&actual)
        .or_raise(|| message!("Could not resolve the private Git directory \"{}\"", actual.display()))?;
    let expected = gix_path::realpath(git_dir)
        .or_raise(|| message!("Could not resolve the private Git directory \"{}\"", git_dir.display()))?;
    if !path_eq(&actual, &expected, ignore_case) {
        bail!(Error::BacklinkMismatch { path, expected, actual });
    }
    Ok(())
}

fn path_eq(left: &Path, right: &Path, ignore_case: bool) -> bool {
    left.components().count() == right.components().count() && path_ends_with(left, right, ignore_case)
}

fn path_ends_with(path: &Path, suffix: &Path, ignore_case: bool) -> bool {
    if !ignore_case {
        return path.ends_with(suffix);
    }
    let mut path = path.components().rev();
    suffix.components().rev().all(|suffix| {
        path.next().is_some_and(|path| {
            path.as_os_str()
                .as_encoded_bytes()
                .eq_ignore_ascii_case(suffix.as_os_str().as_encoded_bytes())
        })
    })
}

/// Return whether `repo` has submodule state that blocks removal with [`Force::Never`].
///
/// A `modules` directory in the private Git directory is conservatively considered sufficient, even if empty.
/// Otherwise, check index gitlinks for an existing `.git` entry in their checkout. Unpopulated gitlinks alone
/// do not count. Submodule cleanliness and ignore settings are not consulted.
fn has_populated_submodule(repo: &crate::Repository) -> Result<bool> {
    if repo.git_dir().join("modules").is_dir() {
        return Ok(true);
    }
    let index = repo
        .index_or_empty()
        .or_raise(|| message("Could not open the linked worktree index"))?;
    Ok(index.entries().iter().any(|entry| {
        entry.mode == gix_index::entry::Mode::COMMIT
            && repo.workdir().is_some_and(|work_dir| {
                work_dir
                    .join(gix_path::from_bstr(entry.path(&index)))
                    .join(".git")
                    .exists()
            })
    }))
}
