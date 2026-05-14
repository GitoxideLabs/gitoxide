//!
use std::path::{Path, PathBuf};

/// A special iterator which communicates its operation through results where…
///
/// * `Some(Ok(removed_directory))` is yielded once or more success, followed by `None`
/// * `Some(Err(std::io::Error))` is yielded exactly once on failure.
pub struct Iter<'a> {
    cursor: Option<&'a Path>,
    boundary: &'a Path,
}

/// Construction
impl<'a> Iter<'a> {
    /// Create a new instance that deletes `target` but will stop at `boundary`, without deleting the latter.
    /// Returns an error if `boundary` doesn't contain `target`
    ///
    /// **Note** that we don't canonicalize the path for performance reasons.
    pub fn new(target: &'a Path, boundary: &'a Path) -> std::io::Result<Self> {
        if !target.starts_with(boundary) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "Removal target '{target}' must be contained in boundary '{boundary}'",
                    target = target.display(),
                    boundary = boundary.display()
                ),
            ));
        }
        let cursor = if target == boundary {
            None
        } else if target.exists() {
            Some(target)
        } else {
            None
        };
        Ok(Iter { cursor, boundary })
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = std::io::Result<&'a Path>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.cursor.take() {
            Some(dir) => {
                let next = match std::fs::remove_dir(dir) {
                    Ok(()) => Some(Ok(dir)),
                    Err(err) => match err.kind() {
                        std::io::ErrorKind::NotFound => Some(Ok(dir)),
                        _other_error_kind => return Some(Err(err)),
                    },
                };
                self.cursor = match dir.parent() {
                    Some(parent) => (parent != self.boundary).then_some(parent),
                    None => {
                        unreachable!(
                            "directory {dir:?} ran out of parents, this really shouldn't happen before hitting the boundary {:?}",
                            self.boundary
                        )
                    }
                };
                next
            }
            None => None,
        }
    }
}

/// Delete all empty directories from `delete_dir` upward and until (not including) the `boundary_dir`.
///
/// Note that `boundary_dir` must contain `delete_dir` or an error is returned, otherwise `delete_dir` is returned on success.
pub fn empty_upward_until_boundary<'a>(delete_dir: &'a Path, boundary_dir: &'a Path) -> std::io::Result<&'a Path> {
    for item in Iter::new(delete_dir, boundary_dir)? {
        match item {
            Ok(_dir) => continue,
            Err(err) => return Err(err),
        }
    }
    Ok(delete_dir)
}

/// Delete all empty directories reachable from `delete_dir` from empty leaves moving upward to and including `delete_dir`.
///
/// If any encountered directory contains a file the entire operation is aborted.
/// Please note that this is inherently racy and no attempts are made to counter that, which will allow creators to win
/// as long as they retry.
pub fn empty_depth_first(delete_dir: PathBuf) -> std::io::Result<()> {
    if let Ok(()) = std::fs::remove_dir(&delete_dir) {
        return Ok(());
    }

    let mut stack = vec![delete_dir];
    let mut next_to_push = Vec::new();
    while let Some(dir_to_delete) = stack.pop() {
        let mut num_entries = 0;
        for entry in std::fs::read_dir(&dir_to_delete)? {
            num_entries += 1;
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                next_to_push.push(entry.path());
            } else {
                return Err(std::io::Error::other("Directory not empty"));
            }
        }
        if num_entries == 0 {
            std::fs::remove_dir(&dir_to_delete)?;
        } else {
            stack.push(dir_to_delete);
            stack.append(&mut next_to_push);
        }
    }
    Ok(())
}
