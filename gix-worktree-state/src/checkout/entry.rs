use std::{
    borrow::Cow,
    io::Write,
    path::{Path, PathBuf},
};

use bstr::BStr;
use gix_filter::{driver::apply::MaybeDelayed, pipeline::convert::ToWorktreeOutcome};
use gix_index::{entry::Stat, Entry};
use gix_object::FindExt;
use gix_worktree::Stack;
use io_close::Close;

pub struct Context<'a, Find> {
    pub objects: &'a mut Find,
    pub path_cache: &'a mut Stack,
    pub filters: &'a mut gix_filter::Pipeline,
    pub buf: &'a mut Vec<u8>,
}

/// A delayed result of a long-running filter process, which is made available as stream.
pub struct DelayedFilteredStream<'a> {
    /// The key identifying the driver program
    pub key: gix_filter::driver::Key,
    /// If the file is going to be an executable.
    pub needs_executable_bit: bool,
    /// The validated path on disk at which the file should be placed.
    pub validated_file_path: PathBuf,
    /// The entry to adjust with the file we will write.
    pub entry: &'a mut gix_index::Entry,
    /// The relative path at which the entry resides (for use when querying the delayed entry).
    pub entry_path: &'a BStr,
}

pub enum Outcome<'a> {
    /// The file was written.
    Written {
        /// The amount of written bytes.
        bytes: usize,
    },
    /// The will be ready later.
    Delayed(DelayedFilteredStream<'a>),
}

impl Outcome<'_> {
    /// Return ourselves as (in-memory) bytes if possible.
    pub fn as_bytes(&self) -> Option<usize> {
        match self {
            Outcome::Written { bytes } => Some(*bytes),
            Outcome::Delayed { .. } => None,
        }
    }
}

#[cfg_attr(not(unix), allow(unused_variables))]
pub fn checkout<'entry, Find>(
    entry: &'entry mut Entry,
    entry_path: &'entry BStr,
    Context {
        objects,
        filters,
        path_cache,
        buf,
    }: Context<'_, Find>,
    crate::checkout::chunk::Options {
        fs: gix_fs::Capabilities {
            symlink,
            executable_bit,
            ..
        },
        destination_is_initially_empty,
        overwrite_existing,
        filter_process_delay,
        ..
    }: crate::checkout::chunk::Options,
) -> Result<Outcome<'entry>, crate::checkout::Error>
where
    Find: gix_object::Find,
{
    let dest_relative = gix_path::try_from_bstr(entry_path).map_err(|_| crate::checkout::Error::IllformedUtf8 {
        path: entry_path.to_owned(),
    })?;
    let path_cache = path_cache.at_path(dest_relative.as_ref(), Some(entry.mode), &*objects)?;
    let dest = path_cache.path();

    let object_size = match entry.mode {
        gix_index::entry::Mode::FILE | gix_index::entry::Mode::FILE_EXECUTABLE => {
            let obj = (*objects)
                .find_blob(&entry.id, buf)
                .map_err(|err| crate::checkout::Error::Find {
                    err,
                    path: dest.to_path_buf(),
                })?;

            let filtered = filters.convert_to_worktree(
                obj.data,
                entry_path,
                &mut |_, attrs| {
                    path_cache.matching_attributes(attrs);
                },
                filter_process_delay,
            )?;
            let (num_bytes, file, set_executable_after_creation) = match filtered {
                ToWorktreeOutcome::Unchanged(buf) | ToWorktreeOutcome::Buffer(buf) => {
                    let (mut file, flag) = open_file(
                        dest,
                        destination_is_initially_empty,
                        overwrite_existing,
                        executable_bit,
                        entry.mode,
                    )?;
                    file.write_all(buf)?;
                    (buf.len(), file, flag)
                }
                ToWorktreeOutcome::Process(MaybeDelayed::Immediate(mut filtered)) => {
                    let (mut file, flag) = open_file(
                        dest,
                        destination_is_initially_empty,
                        overwrite_existing,
                        executable_bit,
                        entry.mode,
                    )?;
                    let num_bytes = std::io::copy(&mut filtered, &mut file)? as usize;
                    (num_bytes, file, flag)
                }
                ToWorktreeOutcome::Process(MaybeDelayed::Delayed(key)) => {
                    return Ok(Outcome::Delayed(DelayedFilteredStream {
                        key,
                        needs_executable_bit: false,
                        validated_file_path: dest.to_owned(),
                        entry,
                        entry_path,
                    }))
                }
            };

            // For possibly existing, overwritten files, we must change the file mode explicitly.
            finalize_entry(entry, file, num_bytes as u64, set_executable_after_creation)?;
            num_bytes
        }
        gix_index::entry::Mode::SYMLINK => {
            let obj = (*objects)
                .find_blob(&entry.id, buf)
                .map_err(|err| crate::checkout::Error::Find {
                    err,
                    path: dest.to_path_buf(),
                })?;
            if symlink {
                #[cfg_attr(not(windows), allow(unused_mut))]
                let mut symlink_destination = Cow::Borrowed(
                    gix_path::try_from_byte_slice(obj.data)
                        .map_err(|_| crate::checkout::Error::IllformedUtf8 { path: obj.data.into() })?,
                );
                #[cfg(windows)]
                {
                    symlink_destination = gix_path::to_native_path_on_windows(gix_path::into_bstr(symlink_destination))
                }

                try_op_or_unlink(dest, overwrite_existing, |p| {
                    gix_fs::symlink::create(symlink_destination.as_ref(), p)
                })?;
            } else {
                let mut file = try_op_or_unlink(dest, overwrite_existing, |p| {
                    open_options(p, destination_is_initially_empty, overwrite_existing).open(dest)
                })?;
                file.write_all(obj.data)?;
                file.close()?;
            }

            entry.stat = Stat::from_fs(&gix_index::fs::Metadata::from_path_no_follow(dest)?)?;
            obj.data.len()
        }
        gix_index::entry::Mode::DIR => {
            gix_features::trace::warn!(
                "Skipped sparse directory at '{entry_path}' ({id}) as it cannot yet be handled",
                id = entry.id
            );
            0
        }
        gix_index::entry::Mode::COMMIT => {
            gix_features::trace::warn!(
                "Skipped submodule at '{entry_path}' ({id}) as it cannot yet be handled",
                id = entry.id
            );
            0
        }
        _ => unreachable!(),
    };
    Ok(Outcome::Written { bytes: object_size })
}

/// Note that this works only because we assume to not race ourselves when symlinks are involved, and we do this by
/// delaying symlink creation to the end and will always do that sequentially.
/// It's still possible to fall for a race if other actors create symlinks in our path, but that's nothing to defend against.
fn try_op_or_unlink<T>(
    path: &Path,
    overwrite_existing: bool,
    op: impl Fn(&Path) -> std::io::Result<T>,
) -> std::io::Result<T> {
    if overwrite_existing {
        match op(path) {
            Ok(res) => Ok(res),
            Err(err) if gix_fs::symlink::is_collision_error(&err) => {
                try_unlink_path_recursively(path, &std::fs::symlink_metadata(path)?)?;
                op(path)
            }
            Err(err) => Err(err),
        }
    } else {
        op(path)
    }
}

fn try_unlink_path_recursively(path: &Path, path_meta: &std::fs::Metadata) -> std::io::Result<()> {
    if path_meta.is_dir() {
        std::fs::remove_dir_all(path)
    } else if path_meta.file_type().is_symlink() {
        gix_fs::symlink::remove(path)
    } else {
        std::fs::remove_file(path)
    }
}

#[cfg(not(debug_assertions))]
fn debug_assert_dest_is_no_symlink(_path: &Path) {}

/// This is a debug assertion as we expect the machinery calling this to prevent this possibility in the first place
#[cfg(debug_assertions)]
fn debug_assert_dest_is_no_symlink(path: &Path) {
    if let Ok(meta) = path.metadata() {
        debug_assert!(
            !meta.file_type().is_symlink(),
            "BUG: should not ever allow to overwrite/write-into the target of a symbolic link: {}",
            path.display()
        );
    }
}

fn open_options(path: &Path, destination_is_initially_empty: bool, overwrite_existing: bool) -> std::fs::OpenOptions {
    if overwrite_existing || !destination_is_initially_empty {
        debug_assert_dest_is_no_symlink(path);
    }
    let mut options = gix_features::fs::open_options_no_follow();
    options
        .create_new(destination_is_initially_empty && !overwrite_existing)
        .create(!destination_is_initially_empty || overwrite_existing)
        .write(true)
        .truncate(true);
    options
}

pub(crate) fn open_file(
    path: &Path,
    destination_is_initially_empty: bool,
    overwrite_existing: bool,
    fs_supports_executable_bit: bool,
    entry_mode: gix_index::entry::Mode,
) -> std::io::Result<(std::fs::File, bool)> {
    #[cfg_attr(windows, allow(unused_mut))]
    let mut options = open_options(path, destination_is_initially_empty, overwrite_existing);
    let needs_executable_bit = fs_supports_executable_bit && entry_mode == gix_index::entry::Mode::FILE_EXECUTABLE;
    #[cfg(unix)]
    let set_executable_after_creation = if needs_executable_bit && destination_is_initially_empty {
        use std::os::unix::fs::OpenOptionsExt;
        // Note that these only work if the file was newly created, but won't if it's already
        // existing, possibly without the executable bit set. Thus we do this only if the file is new.
        options.mode(0o777);
        false
    } else {
        needs_executable_bit
    };
    //  not supported on windows
    #[cfg(windows)]
    let set_executable_after_creation = needs_executable_bit;
    try_op_or_unlink(path, overwrite_existing, |p| options.open(p)).map(|f| (f, set_executable_after_creation))
}

/// Close `file` and store its stats in `entry`, possibly setting `file` executable.
///
/// `desired_bytes` is the amount of bytes Git thinks the file should have after writing.
pub(crate) fn finalize_entry(
    entry: &mut gix_index::Entry,
    file: std::fs::File,
    desired_bytes: u64,
    #[cfg_attr(windows, allow(unused_variables))] set_executable_after_creation: bool,
) -> Result<(), crate::checkout::Error> {
    // For possibly existing, overwritten files, we must change the file mode explicitly.
    #[cfg(unix)]
    if set_executable_after_creation {
        set_executable(&file)?;
    }

    let md = &gix_index::fs::Metadata::from_file(&file)?;
    // A last sanity check: if the file wasn't truncated upon opening, which is good in case something
    // goes wrong during writing, not everything is lost, then after writing the file is smaller than it was
    // before, it needs truncation. We do that here.
    let needs_truncation = md.len() > desired_bytes;
    if needs_truncation {
        file.set_len(desired_bytes)?;
    }
    // NOTE: we don't call `file.sync_all()` here knowing that some filesystems don't handle this well.
    //       revisit this once there is a bug to fix.
    entry.stat = Stat::from_fs(md)?;
    file.close()?;
    Ok(())
}

/// Use `fstat` and `fchmod` on a file descriptor to make a regular file executable.
///
/// See `let_readers_execute` for the exact details of how the mode is transformed.
#[cfg(unix)]
fn set_executable(file: &std::fs::File) -> Result<(), std::io::Error> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let old_mode = file.metadata()?.mode();
    let new_mode = let_readers_execute(old_mode);
    file.set_permissions(std::fs::Permissions::from_mode(new_mode))?;
    Ok(())
}

/// Given the st_mode of a regular file, compute the mode with executable bits safely added.
///
/// Currently this adds executable bits for whoever has read bits already. It doesn't use the umask.
/// Set-user-ID and set-group-ID bits are unset for safety. The sticky bit is also unset.
///
/// This returns only mode bits, not file type. The return value can be used in chmod or fchmod.
#[cfg(any(unix, test))]
fn let_readers_execute(mut mode: u32) -> u32 {
    assert_eq!(mode & 0o170000, 0o100000, "bug in caller if not from a regular file");
    mode &= 0o777; // Clear type, non-rwx mode bits (setuid, setgid, sticky).
    mode |= (mode & 0o444) >> 2; // Let readers also execute.
    mode
}

#[cfg(test)]
mod tests {
    #[test]
    fn let_readers_execute() {
        let cases = [
            // Common cases:
            (0o100755, 0o755),
            (0o100644, 0o755),
            (0o100750, 0o750),
            (0o100640, 0o750),
            (0o100700, 0o700),
            (0o100600, 0o700),
            (0o100775, 0o775),
            (0o100664, 0o775),
            (0o100770, 0o770),
            (0o100660, 0o770),
            (0o100764, 0o775),
            (0o100760, 0o770),
            // Less common:
            (0o100674, 0o775),
            (0o100670, 0o770),
            (0o100000, 0o000),
            (0o100400, 0o500),
            (0o100440, 0o550),
            (0o100444, 0o555),
            (0o100462, 0o572),
            (0o100242, 0o252),
            (0o100167, 0o177),
            // With set-user-ID, set-group-ID, and sticky bits:
            (0o104755, 0o755),
            (0o104644, 0o755),
            (0o102755, 0o755),
            (0o102644, 0o755),
            (0o101755, 0o755),
            (0o101644, 0o755),
            (0o106755, 0o755),
            (0o106644, 0o755),
            (0o104750, 0o750),
            (0o104640, 0o750),
            (0o102750, 0o750),
            (0o102640, 0o750),
            (0o101750, 0o750),
            (0o101640, 0o750),
            (0o106750, 0o750),
            (0o106640, 0o750),
            (0o107644, 0o755),
            (0o107000, 0o000),
            (0o106400, 0o500),
            (0o102462, 0o572),
        ];
        for (st_mode, expected) in cases {
            let actual = super::let_readers_execute(st_mode);
            assert_eq!(
                actual, expected,
                "{st_mode:06o} should become {expected:04o}, became {actual:04o}"
            );
        }
    }

    #[test]
    #[should_panic]
    fn let_readers_execute_panics_on_directory() {
        super::let_readers_execute(0o040644);
    }

    #[test]
    #[should_panic]
    fn let_readers_execute_should_panic_on_symlink() {
        super::let_readers_execute(0o120644);
    }
}
