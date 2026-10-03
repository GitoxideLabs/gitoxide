//! Recursive removal of a linked worktree and its administrative directory.

use std::{
    fmt, fs, io,
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

use gix_features::progress::Progress;

/// Options for removing a linked worktree and its administrative directory.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// The maximum number of worker threads used in each traversal or leaf-deletion phase,
    /// independently of the `parallel` feature.
    ///
    /// `None` (the default) and `Some(0)` use the available logical cores, falling back to one.
    /// `Some(1)` uses one traversal worker and deletes leaves on the calling thread.
    /// Traversal workers are kept idle during deletion so they can be reused for retries.
    pub thread_limit: Option<usize>,
    /// The maximum number of retries after the initial deletion attempt, per root.
    ///
    /// Only directory-not-empty errors are retried. After the first retry, further retries
    /// require fewer scanned entries than in the previous pass.
    /// Defaults to `2`, allowing three attempts in total. `0` disables retries.
    pub max_retries: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            thread_limit: None,
            max_retries: 2,
        }
    }
}

impl Options {
    fn num_threads(&self) -> usize {
        self.thread_limit
            .filter(|threads| *threads != 0)
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
    }
}

/// A failure while scanning or deleting one of the removal roots.
#[derive(Debug)]
pub struct DirectoryError {
    /// The path whose scan or deletion failed.
    pub path: PathBuf,
    /// The underlying filesystem error.
    pub source: io::Error,
}

impl fmt::Display for DirectoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Could not remove \"{}\": {}", self.path.display(), self.source)
    }
}

impl std::error::Error for DirectoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// The error returned by [`crate::remove()`].
///
#[derive(Debug)]
pub enum Error {
    /// A removal root was not absolute, so neither root was removed.
    RelativePath {
        /// The root which must be made absolute by the caller.
        path: PathBuf,
    },
    /// The checkout could not be fully removed, but its private Git directory was removed.
    Worktree(DirectoryError),
    /// The private Git directory could not be fully removed, but its checkout was removed.
    GitDir(DirectoryError),
    /// Neither the checkout nor its private Git directory could be fully removed.
    Both {
        /// The first checkout-removal error.
        worktree: DirectoryError,
        /// The first administrative-removal error.
        git_dir: DirectoryError,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RelativePath { path } => write!(f, "Removal root \"{}\" must be an absolute path", path.display()),
            Self::Worktree(_) => f.write_str("Could not fully remove the linked-worktree checkout"),
            Self::GitDir(_) => f.write_str("Could not fully remove the linked-worktree administration"),
            Self::Both { worktree, git_dir } => write!(
                f,
                "Could not fully remove the checkout ({worktree}) or its administration ({git_dir})"
            ),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Worktree(err) | Self::GitDir(err) => Some(err),
            Self::RelativePath { .. } | Self::Both { .. } => None,
        }
    }
}

pub(super) mod _impl {
    use super::{Error, Options, remove_root};

    use std::{fs, path::Path};

    use gix_features::progress::NestedProgress;

    /// Recursively remove `work_dir` and its private `git_dir` without following symbolic links.
    ///
    /// Both roots must be absolute. Relative roots are rejected before either root is touched
    /// to leave no ambiguity with respect to the CWD.
    ///
    /// Traversal and leaf deletion use [`Options::thread_limit`], and protect Unix mount boundaries
    /// by not crossing them. Directory deletion is currently single-threaded.
    /// The private Git directory is removed even if removing the checkout fails. A missing root is
    /// considered removed successfully, and an empty parent `worktrees` directory is removed as well.
    /// Successful removal of a root supersedes errors encountered while scanning it.
    ///
    /// Entry types, including each leaf's symlink flag, are cached during traversal and may be stale
    /// when deletion runs, creating a time-of-check/time-of-use (TOCTOU) race with concurrent replacements.
    /// A retry rescans the tree and refreshes this information, but only occurs if the first recorded
    /// error is directory-not-empty. Retries are bounded by [`Options::max_retries`], stopping when
    /// subsequent passes no longer find fewer entries. Other non-ignored errors are returned without
    /// retrying, so retries mitigate concurrent changes but do not eliminate TOCTOU races.
    pub fn remove(
        work_dir: impl AsRef<Path>,
        git_dir: impl AsRef<Path>,
        mut progress: impl NestedProgress,
        options: Options,
    ) -> Result<(), Error> {
        let work_dir = work_dir.as_ref();
        let git_dir = git_dir.as_ref();
        if let Some(path) = [work_dir, git_dir].into_iter().find(|path| !path.is_absolute()) {
            return Err(Error::RelativePath { path: path.to_owned() });
        }
        let worktree = remove_root(
            work_dir,
            progress.add_child("scan worktree"),
            progress.add_child("remove worktree"),
            options,
        )
        .err();
        let git_dir_error = remove_root(
            git_dir,
            progress.add_child("scan administration"),
            progress.add_child("remove administration"),
            options,
        )
        .err();

        if git_dir_error.is_none()
            && let Some(worktrees_dir) = git_dir
                .parent()
                .filter(|dir| dir.file_name().is_some_and(|name| name == "worktrees"))
        {
            // This is intentionally best-effort, like Git: a non-empty directory merely means other
            // linked worktrees remain.
            fs::remove_dir(worktrees_dir).ok();
        }

        match (worktree, git_dir_error) {
            (None, None) => Ok(()),
            (Some(err), None) => Err(Error::Worktree(err)),
            (None, Some(err)) => Err(Error::GitDir(err)),
            (Some(worktree), Some(git_dir)) => Err(Error::Both { worktree, git_dir }),
        }
    }
}

fn remove_root(
    root: &Path,
    scan: impl Progress,
    remove: impl Progress,
    options: Options,
) -> Result<(), DirectoryError> {
    remove_root_with_after_scan(root, scan, remove, options, || {})
}

fn remove_root_with_after_scan(
    root: &Path,
    mut scan: impl Progress,
    mut remove: impl Progress,
    options @ Options { max_retries, .. }: Options,
    mut after_scan: impl FnMut(),
) -> Result<(), DirectoryError> {
    let root = normalize_root(root)?;
    let root = root.as_path();
    scan.init(None, gix_features::progress::count("entries"));
    #[cfg(unix)]
    let may_descend = {
        let containing_device = root
            .parent()
            .and_then(|parent| fs::metadata(parent).ok())
            .map(|metadata| std::os::unix::fs::MetadataExt::dev(&metadata));
        let root_is_filesystem_root = root.parent().is_none();
        move |entry: &dua_core::Entry| may_descend(containing_device, root_is_filesystem_root, entry)
    };
    #[cfg(not(unix))]
    let may_descend = |_: &dua_core::Entry| -> io::Result<bool> { Ok(true) };
    let num_threads = options.num_threads();
    let mut walk = dua_core::walk(
        root,
        num_threads,
        dua_core::Order::Completion,
        dua_core::Options::default().skip_metadata(),
        move |entry| {
            // Failed checks are reported when entries are collected below.
            may_descend(entry).unwrap_or(false)
        },
    );
    let mut previous_entry_count = None;
    for attempt in 0..=max_retries {
        let mut leaves = Vec::new();
        let mut directories = Vec::new();
        #[cfg(unix)]
        let mut retained_mounts = Vec::new();
        let mut scan_error = None;
        let mut entry_count = 0;
        for entry in walk.by_ref() {
            entry_count += 1;
            scan.inc();
            match entry {
                Ok(entry) => {
                    let path = entry.path();
                    if entry.file_type.is_dir() {
                        #[cfg(unix)]
                        match may_descend(&entry) {
                            Ok(true) => {}
                            Ok(false) => retained_mounts.push(path.clone()),
                            Err(source) => {
                                if !gix_fs::io_err::is_not_found(source.kind(), source.raw_os_error()) {
                                    scan_error.get_or_insert(DirectoryError { path, source });
                                }
                                continue;
                            }
                        }
                        directories.push((entry.depth, path));
                    } else {
                        leaves.push((path, entry.file_type.is_symlink()));
                    }
                }
                Err(source) if gix_fs::io_err::is_not_found(source.kind(), source.raw_os_error()) => {}
                Err(source) => {
                    scan_error.get_or_insert_with(|| DirectoryError {
                        path: root.to_owned(),
                        source,
                    });
                }
            }
        }
        after_scan();

        remove.init(
            Some(leaves.len() + directories.len()),
            gix_features::progress::count("entries"),
        );
        let counter = remove.counter();
        let mut first_error = remove_leaves(&leaves, num_threads, |path, is_symlink| {
            let result = remove_leaf(path, is_symlink);
            counter.fetch_add(1, Ordering::Relaxed);
            result
                .err()
                .filter(|err| !gix_fs::io_err::is_not_found(err.kind(), err.raw_os_error()))
                .map(|source| DirectoryError {
                    path: path.to_owned(),
                    source,
                })
        });
        directories.sort_unstable_by_key(|(depth, _)| std::cmp::Reverse(*depth));
        for (_, path) in directories {
            #[cfg(unix)]
            if let Some(mount) = retained_mount(&path, &retained_mounts) {
                // ponytail: mount points are rare; index their ancestors if this scan ever becomes measurable.
                if first_error.is_none() && mount == path {
                    first_error = Some(DirectoryError {
                        path: path.clone(),
                        source: io::Error::other("refusing to remove a mounted filesystem"),
                    });
                }
                remove.inc();
                continue;
            }
            let result = fs::remove_dir(&path);
            remove.inc();
            match result {
                Err(source) if !gix_fs::io_err::is_not_found(source.kind(), source.raw_os_error()) => {
                    first_error.get_or_insert(DirectoryError { path, source });
                }
                // The root's removal proves that scan failures did not leave anything behind.
                _ if path == root => scan_error = None,
                _ => {}
            }
        }

        let Some(err) = scan_error.or(first_error) else {
            return Ok(());
        };
        if err.source.kind() != io::ErrorKind::DirectoryNotEmpty
            || attempt == max_retries
            || previous_entry_count.is_some_and(|previous| entry_count >= previous)
            || !walk.restart()
        {
            return Err(err);
        }
        previous_entry_count = Some(entry_count);
    }
    unreachable!("the final deletion attempt always returns")
}

fn normalize_root(root: &Path) -> Result<PathBuf, DirectoryError> {
    debug_assert!(root.is_absolute(), "removal roots are validated before normalization");
    // Trailing separators make `symlink_metadata()` follow the final symlink.
    let absolute: PathBuf = root.components().collect();
    let resolved = match fs::symlink_metadata(&absolute) {
        // Keep file and symlink leaves intact; only directories need root and mount checks.
        Ok(metadata) if !metadata.is_dir() => return Ok(absolute),
        Ok(_) => fs::canonicalize(&absolute),
        Err(source) => Err(source),
    };
    match resolved {
        Ok(path) => Ok(path),
        Err(source) if gix_fs::io_err::is_not_found(source.kind(), source.raw_os_error()) => Ok(absolute),
        Err(source) => Err(DirectoryError { path: absolute, source }),
    }
}

fn remove_leaf(path: &Path, is_symlink: bool) -> io::Result<()> {
    let result = if is_symlink {
        gix_fs::symlink::remove(path)
    } else {
        fs::remove_file(path)
    };
    #[cfg(windows)]
    if !is_symlink
        && result
            .as_ref()
            .is_err_and(|err| err.kind() == io::ErrorKind::PermissionDenied)
    {
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)?;
        return fs::remove_file(path);
    }
    result
}

#[cfg(unix)]
fn may_descend(
    containing_device: Option<u64>,
    root_is_filesystem_root: bool,
    entry: &dua_core::Entry,
) -> io::Result<bool> {
    if root_is_filesystem_root {
        return Ok(false);
    }
    let Some(containing_device) = containing_device else {
        return Ok(true);
    };
    fs::symlink_metadata(entry.path())
        .map(|metadata| std::os::unix::fs::MetadataExt::dev(&metadata) == containing_device)
}

#[cfg(unix)]
fn retained_mount<'a>(path: &Path, mounts: &'a [PathBuf]) -> Option<&'a Path> {
    mounts
        .iter()
        .find(|mount| mount.starts_with(path))
        .map(PathBuf::as_path)
}

fn remove_leaves(
    leaves: &[(PathBuf, bool)],
    num_threads: usize,
    remove: impl Fn(&Path, bool) -> Option<DirectoryError> + Sync,
) -> Option<DirectoryError> {
    let remove_chunk = |leaves: &[(PathBuf, bool)]| {
        leaves.iter().fold(None, |first_error, (path, is_symlink)| {
            let error = remove(path, *is_symlink);
            first_error.or(error)
        })
    };
    if num_threads <= 1 || leaves.len() <= 1 {
        return remove_chunk(leaves);
    }

    std::thread::scope(|scope| {
        let remove_chunk = &remove_chunk;
        let handles: Vec<_> = leaves
            .chunks(leaves.len().div_ceil(num_threads))
            .map(|chunk| scope.spawn(gix_features::trace::in_thread(move || remove_chunk(chunk))))
            .collect();
        handles.into_iter().fold(None, |first_error, handle| {
            let error = handle.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            first_error.or(error)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::Options;

    #[test]
    fn default_options_and_thread_limit_resolution() {
        let options = Options::default();
        assert_eq!(options.thread_limit, None, "threading is automatic by default");
        assert_eq!(options.max_retries, 2, "the default permits three deletion attempts");

        let available = std::thread::available_parallelism().map_or(1, usize::from);
        for thread_limit in [None, Some(0)] {
            assert_eq!(
                Options {
                    thread_limit,
                    ..options
                }
                .num_threads(),
                available,
                "automatic thread counts do not depend on the parallel feature"
            );
        }
        for threads in [1, 2, usize::MAX] {
            assert_eq!(
                Options {
                    thread_limit: Some(threads),
                    ..options
                }
                .num_threads(),
                threads,
                "explicit thread limits are honored"
            );
        }
    }

    #[test]
    fn leaf_deletion_honors_thread_limits() {
        let caller = std::thread::current().id();
        for num_leaves in [0, 1, 8] {
            let leaves: Vec<_> = (0..num_leaves)
                .map(|idx| (std::path::PathBuf::from(format!("leaf-{idx}")), false))
                .collect();
            for threads in [1, 2, 4, usize::MAX] {
                let (send, receive) = std::sync::mpsc::channel();
                let error = super::remove_leaves(&leaves, threads, |_, _| {
                    send.send(std::thread::current().id())
                        .expect("the receiver is still alive");
                    None
                });
                assert!(error.is_none(), "successful leaf deletions do not produce an error");
                let visits: Vec<_> = receive.try_iter().collect();
                assert_eq!(visits.len(), num_leaves, "every leaf is processed exactly once");
                let workers: std::collections::HashSet<_> = visits.into_iter().collect();
                assert_eq!(
                    workers.len(),
                    threads.min(num_leaves),
                    "worker counts respect the thread limit and the available work"
                );
                assert_eq!(
                    workers.contains(&caller),
                    num_leaves == 1 || (num_leaves != 0 && threads == 1),
                    "a single worker or leaf is processed directly on the calling thread"
                );
            }
        }
    }

    #[test]
    fn leaf_errors_do_not_stop_other_deletions() {
        let leaves: Vec<_> = (0..8)
            .map(|idx| (std::path::PathBuf::from(format!("leaf-{idx}")), false))
            .collect();
        for threads in [1, 2, 4] {
            let visits = std::sync::atomic::AtomicUsize::new(0);
            let error = super::remove_leaves(&leaves, threads, |path, _| {
                visits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Some(super::DirectoryError {
                    path: path.to_owned(),
                    source: std::io::Error::other("removal failed"),
                })
            })
            .expect("the first deletion error is retained");
            assert_eq!(
                visits.load(std::sync::atomic::Ordering::Relaxed),
                leaves.len(),
                "all leaves are processed despite errors in any worker"
            );
            assert_eq!(error.path, leaves[0].0, "the first chunk's error is retained");
        }
    }

    #[cfg(unix)]
    #[test]
    fn normalized_roots_keep_their_containing_device() -> gix_testtools::Result {
        let tmp = gix_testtools::tempfile::tempdir()?;
        let root = super::normalize_root(&tmp.path().join("one-component"))?;
        assert!(root.is_absolute(), "root normalization preserves absolute paths");
        assert!(
            root.parent()
                .and_then(|parent| std::fs::metadata(parent).ok())
                .is_some(),
            "a missing root still has a containing filesystem"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn mount_boundaries_are_checked_without_traversal_metadata() -> std::io::Result<()> {
        let tmp = gix_testtools::tempfile::tempdir()?;
        let entry = dua_core::Entry::from_path(tmp.path(), dua_core::Options::default().skip_metadata())?;
        assert!(entry.metadata.is_none(), "removal walks omit entry metadata");
        let device = std::os::unix::fs::MetadataExt::dev(&std::fs::symlink_metadata(tmp.path())?);
        assert!(
            super::may_descend(Some(device), false, &entry)?,
            "the same filesystem is traversed"
        );
        assert!(
            !super::may_descend(Some(device.wrapping_add(1)), false, &entry)?,
            "mounted filesystems are not traversed"
        );
        let mount = tmp.path().join("checkout/mount");
        assert_eq!(
            super::retained_mount(&mount, std::slice::from_ref(&mount)),
            Some(mount.as_path()),
            "the mount itself records the removal error"
        );
        assert_eq!(
            super::retained_mount(tmp.path(), std::slice::from_ref(&mount)),
            Some(mount.as_path()),
            "mount ancestors are retained too"
        );
        tmp.close()?;
        assert_eq!(
            super::may_descend(Some(device), false, &entry)
                .expect_err("a failed metadata check cannot allow descent")
                .kind(),
            std::io::ErrorKind::NotFound,
            "directory metadata errors remain available to the removal loop"
        );
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn filesystem_root_aliases_are_not_traversed() -> gix_testtools::Result {
        let tmp = gix_testtools::tempfile::tempdir()?;
        let root_link = tmp.path().join("root-link");
        gix_fs::symlink::create(std::path::Path::new("/"), &root_link)?;

        // Only inspect descent decisions; never invoke removal on the filesystem root.
        for root in [
            std::path::PathBuf::from("/"),
            std::path::PathBuf::from("/.."),
            root_link.join(".."),
        ] {
            let root = super::normalize_root(&root)?;
            let entry = dua_core::Entry::from_path(&root, dua_core::Options::default().skip_metadata())?;
            let device = std::os::unix::fs::MetadataExt::dev(&std::fs::symlink_metadata(&root)?);
            assert!(
                !super::may_descend(Some(device), root.parent().is_none(), &entry)?,
                "the filesystem root cannot be traversed via {}",
                root.display()
            );
        }
        Ok(())
    }

    #[test]
    fn retries_when_an_entry_appears_after_scanning() -> gix_testtools::Result {
        for options in [
            Options::default(),
            Options {
                max_retries: usize::MAX,
                ..Options::default()
            },
        ] {
            let tmp = gix_testtools::tempfile::tempdir()?;
            let root = tmp.path().join("worktree");
            std::fs::create_dir(&root)?;
            let mut scans = 0;

            super::remove_root_with_after_scan(
                &root,
                gix_features::progress::Discard,
                gix_features::progress::Discard,
                options,
                || {
                    scans += 1;
                    if scans == 1 {
                        std::fs::write(root.join("late"), b"late").expect("late file can be created");
                    }
                },
            )?;

            assert_eq!(scans, 2, "one retry was needed, even with an unbounded retry budget");
            assert!(!root.exists(), "the root was removed");
        }
        Ok(())
    }

    #[test]
    fn stops_retrying_if_the_workload_does_not_shrink() {
        for equal_size in [false, true] {
            let tmp = gix_testtools::tempfile::tempdir().expect("temporary directory can be created");
            let root = tmp.path().join("worktree");
            std::fs::create_dir(&root).expect("removal root can be created");
            let root = std::fs::canonicalize(&root).expect("the removal root exists");
            if equal_size {
                std::fs::write(root.join("initial"), b"initial").expect("initial file can be created");
            } else {
                std::fs::create_dir(root.join("nested")).expect("initial directory can be created");
            }
            let mut scans = 0;

            let err = super::remove_root_with_after_scan(
                &root,
                gix_features::progress::Discard,
                gix_features::progress::Discard,
                Options {
                    max_retries: 5,
                    ..Options::default()
                },
                || {
                    let parent = if !equal_size && scans == 0 {
                        root.join("nested")
                    } else {
                        root.clone()
                    };
                    std::fs::write(parent.join(format!("late-{scans}")), b"late").expect("late file can be created");
                    scans += 1;
                },
            )
            .expect_err("a non-shrinking root is left to the concurrent writer");

            assert_eq!(err.source.kind(), std::io::ErrorKind::DirectoryNotEmpty);
            assert_eq!(err.path, root, "the latest attempt's error is returned");
            assert_eq!(scans, 2, "an equal or growing workload stops after one retry");
        }
    }

    #[test]
    fn stops_after_the_configured_number_of_improving_deletion_passes() -> gix_testtools::Result {
        for max_retries in [0, 1, 2, 4] {
            let tmp = gix_testtools::tempfile::tempdir()?;
            let root = tmp.path().join("worktree");
            std::fs::create_dir(&root)?;
            for idx in 0..max_retries + 2 {
                std::fs::write(root.join(format!("initial-{idx}")), b"initial")?;
            }
            let mut scans = 0;

            let err = super::remove_root_with_after_scan(
                &root,
                gix_features::progress::Discard,
                gix_features::progress::Discard,
                Options {
                    max_retries,
                    ..Options::default()
                },
                || {
                    // Leave fewer files on each pass, but always leave at least one to prevent success.
                    for idx in 0..max_retries + 1 - scans {
                        std::fs::write(root.join(format!("late-{scans}-{idx}")), b"late")
                            .expect("late files can be created");
                    }
                    scans += 1;
                },
            )
            .expect_err("the configured retry limit stops deletion despite continued progress");

            assert_eq!(
                err.source.kind(),
                std::io::ErrorKind::DirectoryNotEmpty,
                "the remaining late file prevents removing the root directory"
            );
            assert_eq!(scans, max_retries + 1, "the initial attempt is not counted as a retry");
            assert!(
                !root.join("initial-0").exists(),
                "the initial deletion pass runs even when retries are disabled"
            );
        }
        Ok(())
    }
}
