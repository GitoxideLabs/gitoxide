/// Clear the Windows read-only attribute at `path`, following symbolic links to their targets.
///
/// On Windows, read metadata and clear the attribute only if it is set, propagating I/O errors.
/// This allows removing or replacing read-only files, but doesn't bypass ACLs or other access restrictions.
/// On other platforms, return success without accessing `path` or changing permissions, even if it doesn't exist.
pub fn clear_readonly_attribute(path: &std::path::Path) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        let mut permissions = std::fs::metadata(path)?.permissions();
        if permissions.readonly() {
            #[expect(clippy::permissions_set_readonly_false, reason = "this block only runs on Windows")]
            permissions.set_readonly(false);
            std::fs::set_permissions(path, permissions)?;
        }
    }
    #[cfg(not(windows))]
    let _ = path;
    Ok(())
}

/// Prepare open options which won't follow a symlink in the final path component on Unix or Windows.
///
/// These options do not guarantee that opening a symlink fails: Unix uses `O_NOFOLLOW` and rejects it,
/// while Windows uses `FILE_FLAG_OPEN_REPARSE_POINT` and may return a handle to the symlink itself.
/// On Windows, inspect [`File::metadata()`](std::fs::File::metadata) and reject the handle if
/// `file_type().is_symlink()` is true before any I/O. A metadata lookup by path does not validate the
/// opened handle.
///
/// For reading, prefer [`open_read_only_no_follow()`], which returns [`FileOrSymlink`] after checking
/// the handle. Its [`FileOrSymlink::File`] variant can be read without a separate symlink check.
///
/// For writing, leave [`OpenOptions::truncate()`](std::fs::OpenOptions::truncate) disabled while opening,
/// then validate the handle before calling [`File::set_len()`](std::fs::File::set_len). Truncation while
/// opening happens before callers can inspect the handle.
/// [`OpenOptions::create_new()`](std::fs::OpenOptions::create_new) atomically rejects existing final entries,
/// including dangling symlinks.
///
/// Calling a platform-specific `custom_flags()` setter again replaces the no-follow flags.
/// Symlinks in parent components are still followed. On other platforms, these options do not prevent following symlinks.
pub fn open_options_no_follow() -> std::fs::OpenOptions {
    #[cfg_attr(not(any(unix, windows)), allow(unused_mut))]
    let mut options = std::fs::OpenOptions::new();
    #[cfg(unix)]
    {
        /// Make sure that it's impossible to follow through to the target of symlinks.
        /// Note that this will still follow symlinks in the path, which is what we assume
        /// has been checked separately.
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Open the reparse point itself so the handle can be checked without following it.
        const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
    options
}

/// The outcome of [`open_read_only_no_follow()`].
#[derive(Debug)]
pub enum FileOrSymlink {
    /// An opened filesystem handle which isn't itself a symlink.
    File(std::fs::File),
    /// A symlink was found, without returning a handle to it or its target.
    Symlink,
}

/// Open `path` for reading without following a symlink in its final component on Unix or Windows.
///
/// Prefer this to opening directly with [`open_options_no_follow()`] for read-only access. That helper
/// can return a symlink handle on Windows; this function checks the opened handle before exposing it
/// as [`FileOrSymlink::File`].
///
/// Return [`FileOrSymlink::Symlink`] for a live or dangling symlink, which callers must handle explicitly,
/// or [`FileOrSymlink::File`] for an opened non-symlink handle ready for reading. Missing paths and other
/// I/O errors are returned as errors.
///
/// If opening fails, the path may be checked for a symlink, but the open is never retried. The enum
/// lets callers choose how to handle symlinks without depending on platform-specific open errors.
///
/// # Example
///
/// ```no_run
/// # fn read(path: &std::path::Path) -> std::io::Result<Vec<u8>> {
/// use std::io::Read;
///
/// let mut file = match gix_fs::open_read_only_no_follow(path)? {
///     gix_fs::FileOrSymlink::File(file) => file,
///     gix_fs::FileOrSymlink::Symlink => {
///         return Err(std::io::ErrorKind::InvalidInput.into());
///     }
/// };
/// let mut contents = Vec::new();
/// file.read_to_end(&mut contents)?;
/// # Ok(contents)
/// # }
/// ```
///
/// Symlinks in parent components are still followed. No-follow protection is only provided on Unix and Windows.
pub fn open_read_only_no_follow(path: &std::path::Path) -> std::io::Result<FileOrSymlink> {
    match open_options_no_follow().read(true).open(path) {
        // This is naturally Windows-only as opening a symlink with O_NOFOLLOW fails on linux.
        Ok(file) if file.metadata()?.file_type().is_symlink() => Ok(FileOrSymlink::Symlink),
        Ok(file) => Ok(FileOrSymlink::File(file)),
        // Unix no-follow opens reject symlinks. Classify the failed open without retrying it.
        Err(err)
            if err.kind() != std::io::ErrorKind::NotFound
                && path.symlink_metadata().is_ok_and(|meta| meta.file_type().is_symlink()) =>
        {
            Ok(FileOrSymlink::Symlink)
        }
        Err(err) => Err(err),
    }
}
