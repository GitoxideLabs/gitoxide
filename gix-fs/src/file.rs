/// Prepare open options which won't follow a symlink in the final path component on Unix or Windows.
///
/// On Unix, opening a symlink fails. On Windows, opening can succeed with a handle to the reparse point
/// itself. Callers must inspect its metadata before I/O. In particular, avoid requesting truncation
/// while opening: validate the handle before calling [`std::fs::File::set_len()`].
/// For reading, prefer [`open_read_only_no_follow()`], which performs the classification for the caller.
///
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
/// Return [`FileOrSymlink::Symlink`] for a live or dangling symlink, or [`FileOrSymlink::File`] for an
/// opened non-symlink handle. Missing paths and other I/O errors are returned as errors.
///
/// Opened handles are inspected before being returned. If opening fails, the path may be checked for
/// a symlink, but the open is never retried. This lets callers choose how to handle symlinks without
/// depending on platform-specific open errors or accidentally reading through a link.
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
