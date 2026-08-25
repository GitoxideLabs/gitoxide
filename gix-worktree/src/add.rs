//! Prepare the administrative files for a linked worktree.

use std::{
    ffi::OsString,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

/// Options for linking a new worktree to its shared Git directory.
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Write relative paths into the `.git` and `gitdir` files, falling back to absolute paths across filesystem roots.
    ///
    /// This does not affect the paths passed to [`prepare()`], which may be absolute or relative regardless of
    /// this option. Relative links are computed automatically from those paths.
    ///
    /// This allows the repository and its worktrees to be moved together, or mounted at different absolute paths
    /// on a host and in a container, without repairing their links. Their relative directory layout must remain
    /// unchanged; moving just one worktree can still break the links.
    ///
    /// The caller must enable `extensions.relativeWorktrees` and repository format version 1 in the shared config.
    /// This compatibility marker prevents older Git versions from misinterpreting relative links.
    pub relative_paths: bool,
}

/// A linked worktree whose administrative files are prepared, but whose checkout is not complete yet.
///
/// Unless [`persist()`][PreparedWorktree::persist()] is called, dropping this value performs
/// [`rollback()`][PreparedWorktree::rollback()], ignoring cleanup errors. Call it explicitly to handle these errors.
#[derive(Debug)]
pub struct PreparedWorktree {
    /// Absolute path to the shared Git directory, such as `/repo/.git`, with symbolic links resolved.
    common_dir: PathBuf,
    /// Absolute path to this worktree's private administrative directory at `<common_dir>/worktrees/<id>`.
    git_dir: PathBuf,
    /// Absolute path to the checkout destination, with symbolic links in its parent components resolved.
    work_dir: PathBuf,
    work_dir_cleanup: Option<WorkDirCleanup>,
    rollback: bool,
}

#[derive(Debug)]
enum WorkDirCleanup {
    ClearContents,
    RemoveDirectory,
}

impl PreparedWorktree {
    /// Return the shared Git directory.
    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }

    /// Return the newly reserved private Git directory below `worktrees/`.
    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    /// Return the worktree directory.
    pub fn work_dir(&self) -> &Path {
        &self.work_dir
    }
}

/// Lifecycle
impl PreparedWorktree {
    /// Finish adding the worktree by removing the initialization lock and disabling rollback.
    pub fn persist(mut self) -> io::Result<()> {
        fs::remove_file(self.git_dir.join("locked"))?;
        self.rollback = false;
        Ok(())
    }

    /// Remove the private Git directory and undo adding the worktree.
    ///
    /// If [`prepare()`] created the destination directory, remove it and all its contents. If the destination
    /// already existed, remove its contents but leave the directory itself in place. This includes files added
    /// during checkout. Parent directories, including any created by [`prepare()`], remain in place.
    ///
    /// Both worktree cleanup and private Git directory removal are attempted, returning the first error, if any.
    /// This consumes the value even on error; dropping it will not retry cleanup.
    pub fn rollback(mut self) -> io::Result<()> {
        self.rollback = false;
        self.rollback_inner()
    }

    fn rollback_inner(&self) -> io::Result<()> {
        let work_dir = match self.work_dir_cleanup {
            None => Ok(()),
            Some(WorkDirCleanup::ClearContents) => remove_contents_non_recursively(&self.work_dir),
            Some(WorkDirCleanup::RemoveDirectory) => fs::remove_dir_all(&self.work_dir),
        };
        let git_dir = fs::remove_dir_all(&self.git_dir);
        work_dir.and(git_dir)
    }
}

impl Drop for PreparedWorktree {
    fn drop(&mut self) {
        if self.rollback {
            let _ = self.rollback_inner();
        }
    }
}

/// Reserve and initialize the administrative files for a linked worktree at `destination_dir`.
///
/// `common_dir` must be an existing shared Git directory, such as `./repo/.git` for a non-bare repository
/// or `./repo.git` for a bare repository. `destination` must be absent or an empty directory. Its final
/// path component must not be a symbolic link, but parent components may be symbolic links and are resolved
/// before creating the administrative files. The private Git directory is named after the sanitized destination
/// basename, with a numeric suffix added when needed.
///
/// Paths in the returned [`Prepared`] are absolute regardless of the link format selected by `options`.
pub fn prepare(
    common_dir: impl AsRef<Path>,
    destination_dir: impl AsRef<Path>,
    Options { relative_paths }: Options,
) -> io::Result<PreparedWorktree> {
    let common_dir = common_dir.as_ref();
    ensure_directory(common_dir, "common Git directory")?;
    let work_dir = destination_dir.as_ref();
    let empty_destination_exists = validate_destination(work_dir)?;
    let common_dir = gix_path::realpath(common_dir).map_err(io::Error::other)?;
    let work_dir = gix_path::realpath(work_dir).map_err(io::Error::other)?;

    let worktrees_dir = common_dir.join("worktrees");
    fs::create_dir_all(&worktrees_dir)?;
    ensure_directory(&worktrees_dir, "worktrees directory")?;

    let basename = work_dir.file_name().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("worktree destination '{}' has no basename", work_dir.display()),
        )
    })?;
    let basename = gix_path::os_str_into_bstr(basename)
        .map(gix_validate::reference::name_partial_or_sanitize)
        .and_then(gix_path::try_from_bstring)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    let git_dir = reserve_git_dir(&worktrees_dir, basename.as_os_str())?;

    let mut prepared = PreparedWorktree {
        common_dir,
        git_dir,
        work_dir,
        work_dir_cleanup: None,
        rollback: true,
    };

    if !empty_destination_exists {
        if let Some(parent) = prepared.work_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::create_dir(&prepared.work_dir) {
            Ok(()) => prepared.work_dir_cleanup = Some(WorkDirCleanup::RemoveDirectory),
            Err(err) => return Err(err),
        }
    }

    fs::write(prepared.git_dir.join("locked"), b"initializing\n")?;
    write_path(
        prepared.git_dir.join("gitdir"),
        b"",
        &link_path(&prepared.work_dir.join(".git"), &prepared.git_dir, relative_paths),
    )?;
    fs::write(prepared.git_dir.join("commondir"), b"../..\n")?;
    let dot_git = prepared.work_dir.join(".git");
    let mut dot_git = fs::OpenOptions::new().write(true).create_new(true).open(dot_git)?;
    if empty_destination_exists {
        prepared.work_dir_cleanup = Some(WorkDirCleanup::ClearContents);
    }
    write_path_to(
        &mut dot_git,
        b"gitdir: ",
        &link_path(&prepared.git_dir, &prepared.work_dir, relative_paths),
    )?;

    Ok(prepared)
}

/// Return the path to write into a linking file in the directory `base`, pointing to `target`.
/// Both inputs must be absolute. If `relative` is set and they share a filesystem root, compute the path
/// relative to `base`; otherwise return `target` unchanged, including across Windows drives or UNC shares.
/// This only manipulates path components and does not access the filesystem or resolve symbolic links.
fn link_path(target: &Path, base: &Path, relative: bool) -> PathBuf {
    if relative
        && let Some(root) = base.ancestors().last()
        && let Ok(target) = target.strip_prefix(root)
        && let Ok(base) = base.strip_prefix(root)
    {
        gix_path::relativize_with_prefix(target, base).into_owned()
    } else {
        target.to_owned()
    }
}

fn validate_destination(path: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("worktree destination '{}' is a symbolic link", path.display()),
        )),
        Ok(metadata) if !metadata.is_dir() => Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("worktree destination '{}' is not a directory", path.display()),
        )),
        Ok(_) => match fs::read_dir(path)?.next().transpose()? {
            Some(_) => Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("worktree destination '{}' is not empty", path.display()),
            )),
            None => Ok(true),
        },
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

fn ensure_directory(path: &Path, name: &str) -> io::Result<()> {
    if !fs::metadata(path)?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{name} '{}' is not a directory", path.display()),
        ));
    }
    Ok(())
}

/// Create an empty private Git directory under the existing `parent` and return its path.
/// Try `basename` first, then append increasing numeric suffixes (`basename1`, `basename2`, …) on collisions.
/// Creating each candidate atomically reserves its name, including against concurrent worktree additions.
/// Other filesystem errors and suffix overflow are returned; the caller owns cleanup of the created directory.
fn reserve_git_dir(parent: &Path, basename: &std::ffi::OsStr) -> io::Result<PathBuf> {
    let mut suffix = 0_u64;
    loop {
        let mut name = OsString::from(basename);
        if suffix != 0 {
            name.push(suffix.to_string());
        }
        let candidate = parent.join(name);
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {
                suffix = suffix
                    .checked_add(1)
                    .ok_or_else(|| io::Error::other("worktree ID suffix overflow"))?;
            }
            Err(err) => return Err(err),
        }
    }
}

fn write_path(path: PathBuf, prefix: &[u8], value: &Path) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    write_path_to(&mut file, prefix, value)
}

fn write_path_to(mut out: impl Write, prefix: &[u8], value: &Path) -> io::Result<()> {
    let value = gix_path::try_into_bstr(value).map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    let value = gix_path::to_unix_separators_on_windows(value);
    out.write_all(prefix)?;
    out.write_all(&value)?;
    out.write_all(b"\n")
}

fn remove_contents_non_recursively(directory: &Path) -> io::Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            gix_fs::symlink::remove(&entry.path())?;
        } else if file_type.is_dir() {
            fs::remove_dir_all(entry.path())?;
        } else {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

#[cfg(all(test, windows))]
mod tests {
    use super::link_path;
    use std::path::Path;

    #[test]
    fn relative_links_across_filesystem_roots_remain_absolute() {
        for (target, base) in [
            ("D:/worktree/.git", "C:/repo.git/worktrees/id"),
            ("//server/other/worktree/.git", "//server/share/repo.git/worktrees/id"),
        ] {
            assert_eq!(
                link_path(Path::new(target), Path::new(base), true),
                Path::new(target),
                "different drives or UNC shares cannot be linked with relative paths"
            );
        }
        assert_eq!(
            link_path(
                Path::new("C:/worktree/.git"),
                Path::new("C:/repo.git/worktrees/id"),
                true
            ),
            Path::new("../../../worktree/.git"),
            "paths on the same drive can be relative"
        );
    }
}
