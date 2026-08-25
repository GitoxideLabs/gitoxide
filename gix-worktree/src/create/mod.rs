//! Prepare the administrative files for a linked worktree.

use std::{
    ffi::OsString,
    fs, io,
    io::Write,
    path::{Path, PathBuf},
};

/// A linked worktree whose administrative files are prepared, but whose checkout is not complete yet.
///
/// Unless [`persist()`][Prepared::persist()] is called, dropping this value removes the private Git directory and
/// empties the worktree directory. A destination directory which already existed remains in place.
#[derive(Debug)]
pub struct Prepared {
    common_dir: PathBuf,
    git_dir: PathBuf,
    work_dir: PathBuf,
    remove_work_dir: bool,
    clear_work_dir: bool,
    rollback: bool,
}

impl Prepared {
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

    /// Finish creation by removing the initialization lock and disabling rollback.
    pub fn persist(mut self) -> io::Result<()> {
        fs::remove_file(self.git_dir.join("locked"))?;
        self.rollback = false;
        Ok(())
    }
}

impl Drop for Prepared {
    fn drop(&mut self) {
        if !self.rollback {
            return;
        }

        if self.remove_work_dir {
            let _ = fs::remove_dir_all(&self.work_dir);
        } else if self.clear_work_dir {
            let _ = remove_contents(&self.work_dir);
        }
        let _ = fs::remove_dir_all(&self.git_dir);
    }
}

/// Reserve and initialize the administrative files for a linked worktree at `destination`.
///
/// `common_dir` must be an existing shared Git directory. `destination` must be absent or an empty directory and
/// must not be a symbolic link. The private Git directory is named after the sanitized destination basename, with a
/// numeric suffix added when needed.
pub fn prepare(common_dir: impl AsRef<Path>, destination: impl AsRef<Path>) -> io::Result<Prepared> {
    let common_dir = std::path::absolute(common_dir)?;
    ensure_directory(&common_dir, "common Git directory")?;
    let work_dir = std::path::absolute(destination)?;
    let destination_exists = validate_destination(&work_dir)?;
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

    let mut prepared = Prepared {
        common_dir,
        git_dir,
        work_dir,
        remove_work_dir: false,
        clear_work_dir: false,
        rollback: true,
    };

    if !destination_exists {
        if let Some(parent) = prepared.work_dir.parent() {
            fs::create_dir_all(parent)?;
        }
        match fs::create_dir(&prepared.work_dir) {
            Ok(()) => prepared.remove_work_dir = true,
            Err(err) => return Err(err),
        }
    }

    fs::write(prepared.git_dir.join("locked"), b"initializing\n")?;
    write_path(prepared.git_dir.join("gitdir"), b"", &prepared.work_dir.join(".git"))?;
    fs::write(prepared.git_dir.join("commondir"), b"../..\n")?;
    let dot_git = prepared.work_dir.join(".git");
    let mut dot_git = fs::OpenOptions::new().write(true).create_new(true).open(dot_git)?;
    prepared.clear_work_dir = !prepared.remove_work_dir;
    write_path_to(&mut dot_git, b"gitdir: ", &prepared.git_dir)?;

    Ok(prepared)
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
    let value = gix_path::to_unix_separators(value);
    out.write_all(prefix)?;
    out.write_all(&value)?;
    out.write_all(b"\n")
}

fn remove_contents(directory: &Path) -> io::Result<()> {
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
