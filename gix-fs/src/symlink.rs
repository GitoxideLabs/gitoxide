use std::{io, io::ErrorKind::AlreadyExists, path::Path};

use gix_error::{Result, ResultExt, bail, message};

/// Append `relative_path` to the trusted `root`, rejecting symlinks in every appended component.
///
/// Reject rooted paths, platform prefixes, and paths that lexically escape `root` before probing the filesystem.
/// Check components before normalizing away `..`, and preserve the original path spelling in the returned path.
/// Missing components and non-directory obstructions are allowed, for example for uninitialized submodules.
/// Symlinks in `root` or its ancestors are allowed, but even dangling or terminal links below it are rejected.
///
/// Use this function for one-shot validation, for example of a submodule path like `modules/lib` or
/// `modules/../lib`. The latter is accepted only if it stays within `root` and `modules` is not a symlink.
/// Use [`crate::Stack`] instead when traversing related normalized paths, such as `src/lib.rs` followed by
/// `src/main.rs`, to reuse shared leading directories and maintain directory state through a delegate.
/// Unlike this function, `Stack` rejects `..` components and leaves filesystem checks to its delegate.
///
/// These metadata checks assume no concurrent replacement of checked components; they are not atomic with later I/O.
pub fn validate_relative_path(root: &Path, relative_path: &Path) -> Result<std::path::PathBuf> {
    use std::path::Component;

    if relative_path
        .components()
        .any(|component| matches!(component, Component::RootDir | Component::Prefix(_)))
        || gix_path::normalize(relative_path.into(), Path::new("")).is_none()
    {
        bail!(gix_error::validation(format!(
            "The path \"{}\" must stay relative to its root",
            relative_path.display()
        )));
    }
    let mut path = root.to_owned();
    for component in relative_path.components() {
        path.push(component);
        match path.symlink_metadata() {
            Ok(meta) if meta.file_type().is_symlink() => {
                bail!(gix_error::validation(format!(
                    "The path at \"{}\" contains a symbolic link",
                    path.display()
                )));
            }
            Ok(_) => {}
            Err(err) if matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::NotADirectory) => {}
            Err(err) => {
                return Err(err).or_raise(|| message!("Could not inspect path at \"{}\"", path.display()));
            }
        }
    }
    Ok(root.join(relative_path))
}

/// Create a new symlink at `link` which points to `original`.
///
/// Note that `original` doesn't have to exist.
#[cfg(unix)]
pub fn create(original: &Path, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(original, link)
}

/// Create a new symlink at `link` which points to `original`.
///
/// Note that `original` doesn't have to exist.
#[cfg(target_os = "wasi")]
pub fn create(original: &Path, link: &Path) -> io::Result<()> {
    std::fs::soft_link(original, link)
}

/// Create a new symlink at `link` which points to `original`.
///
/// Note that symbolic links are unsupported on this platform.
#[cfg(not(any(unix, windows, target_os = "wasi")))]
pub fn create(_original: &Path, _link: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "symbolic links are not supported on this platform",
    ))
}

/// Remove a symlink.
///
/// Note that on only on windows this is special.
#[cfg(any(unix, target_os = "wasi"))]
pub fn remove(path: &Path) -> io::Result<()> {
    std::fs::remove_file(path)
}

/// Remove a symlink.
#[cfg(not(any(unix, windows, target_os = "wasi")))]
pub fn remove(_path: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "symbolic links are not supported on this platform",
    ))
}

// TODO: use the `symlink` crate once it can delete directory symlinks
/// Remove a symlink.
#[cfg(windows)]
pub fn remove(path: &Path) -> io::Result<()> {
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.is_file() {
            std::fs::remove_file(path) // this removes the link itself
        } else {
            std::fs::remove_dir(path) // however, this sees the destination directory, which isn't the right thing actually
        }
    } else {
        std::fs::remove_file(path).or_else(|_| std::fs::remove_dir(path))
    }
}

/// Create a new symlink at `link` which points to `original`.
///
/// Note that if a symlink target (the `original`) isn't present on disk, it's assumed to be a
/// file, creating a dangling file symlink. This is similar to a dangling symlink on Unix,
/// which doesn't have to care about the target type though.
/// Targets starting with `/` or `\` are also treated as files without probing them, as Windows
/// can interpret these as network shares and probing them could disclose credentials.
/// This applies to every link in a target's chain, including links in parent components.
/// Local chains are resolved without following symlinks during metadata queries.
/// As with checkout path validation, this assumes no concurrent replacement of checked directories.
#[cfg(windows)]
pub fn create(original: &Path, link: &Path) -> io::Result<()> {
    create_inner(original, link, false).map(|_| ())
}

/// Create a new symlink at `link` pointing to `original`, returning whether it was created.
///
/// On Windows, return `false` without creating a link if the target's type cannot be determined.
/// Callers can retry after creating other links, then use [`create()`] for any unresolved targets
/// once no more progress is possible. Targets starting with `/` or `\` are never probed and are
/// immediately created as file links, just like in [`create()`].
/// The same restriction applies to indirect targets reached through other links.
/// On other platforms, this is equivalent to [`create()`] and always returns `true` on success.
pub fn create_or_delay(original: &Path, link: &Path) -> io::Result<bool> {
    #[cfg(windows)]
    {
        create_inner(original, link, true)
    }
    #[cfg(not(windows))]
    {
        create(original, link).map(|()| true)
    }
}

#[cfg(windows)]
fn starts_with_separator(path: &Path) -> bool {
    matches!(path.as_os_str().as_encoded_bytes().first(), Some(b'/' | b'\\'))
}

#[cfg(windows)]
fn create_inner(original: &Path, link: &Path, delay_if_unknown: bool) -> io::Result<bool> {
    use std::os::windows::fs::{symlink_dir, symlink_file};

    // Match Git's `process_phantom_symlink()`: a leading separator can introduce a UNC
    // share or an NT namespace path. Check the original spelling before any target probe.
    let is_dir = if starts_with_separator(original) {
        false
    } else {
        let parent = link
            .parent()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "symlink path has no parent directory"))?;
        match target_is_dir(&parent.join(original)) {
            Ok(is_dir) => is_dir,
            Err(_) if delay_if_unknown => return Ok(false),
            Err(_) => false,
        }
    };
    if is_dir {
        symlink_dir(original, link)?;
    } else {
        symlink_file(original, link)?;
    }
    Ok(true)
}

#[cfg(windows)]
fn target_is_dir(path: &Path) -> io::Result<bool> {
    use std::path::{Component, PathBuf, Prefix};

    let mut path = path.to_owned();
    // Bound resolution for cycles and excessive chains. Unknown types retain the
    // caller's existing retry/dangling-file behavior.
    'resolve: for _ in 0..40 {
        let mut current = PathBuf::new();
        let mut components = path.components();
        while let Some(component) = components.next() {
            current.push(component);
            if let Component::Prefix(prefix) = component {
                if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_)) {
                    return Ok(false);
                }
                continue;
            }
            // Check every parent before appending its child: no-follow metadata
            // alone only protects the final component of the queried path.
            let meta = current.symlink_metadata()?;
            // Use the same classification as the worktree stack, including Windows junctions
            // and mount points, but not unrelated reparse points.
            if meta.file_type().is_symlink() {
                let target = std::fs::read_link(&current)?;
                if starts_with_separator(&target) {
                    return Ok(false);
                }
                current.pop();
                path = current.join(target).join(components.as_path());
                continue 'resolve;
            }
            if components.as_path().as_os_str().is_empty() {
                return Ok(meta.is_dir());
            }
        }
    }
    Err(io::Error::other("symlink target exceeds the resolution limit"))
}

/// Return true if `err` indicates that a file collision happened, i.e. a symlink couldn't be created as the `link`
/// already exists as filesystem object.
#[cfg(any(unix, target_os = "wasi"))]
pub fn is_collision_error(err: &std::io::Error) -> bool {
    // TODO: use `ErrorKind::FilesystemLoop` once stabilized.
    err.kind() == AlreadyExists
            || err.kind() == std::io::ErrorKind::IsADirectory
            || err.raw_os_error() == Some(62) // no-follow on symlnk on mac-os
            || err.raw_os_error() == Some(40) // no-follow on symlnk on ubuntu
}

/// Return true if `err` indicates that a file collision happened, i.e. a symlink couldn't be created as the `link`
/// already exists as filesystem object.
#[cfg(windows)]
pub fn is_collision_error(err: &std::io::Error) -> bool {
    err.kind() == AlreadyExists || err.kind() == std::io::ErrorKind::PermissionDenied
}

/// Return true if `err` indicates that a file collision happened, i.e. a symlink couldn't be created as the `link`
/// already exists as filesystem object.
#[cfg(not(any(unix, windows, target_os = "wasi")))]
pub fn is_collision_error(err: &std::io::Error) -> bool {
    err.kind() == AlreadyExists
}
