use std::{fs, io, io::Write};

use gix_error::{Result, ResultExt, bail, message};

use crate::{FullNameRef, file};

/// Settings for [`file::Store::reflog_retain()`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Options {
    /// Evaluate the predicate under the reference lock without rewriting the reflog.
    pub dry_run: bool,
}

/// Statistics for a completed or simulated reflog rewrite.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Entries retained by the predicate.
    pub retained: usize,
    /// Entries removed by the predicate.
    pub removed: usize,
}

impl file::Store {
    /// Retain reflog entries selected by `keep`, without changing the reference or rewriting IDs.
    ///
    /// Lock the named reference before reading its reflog, excluding ordinary reference writers.
    /// Symbolic references are not followed: their own logs are filtered. Namespaces and linked
    /// worktree paths use the store's usual reference and reflog routing. Missing logs are a no-op.
    ///
    /// The complete log is decoded before an atomic replacement is published. Errors from parsing,
    /// locking, the predicate, or writing leave the original log intact. Kept entries retain their
    /// exact bytes, including their original old/new IDs, whitespace, and messages. A log with no
    /// retained entries becomes an empty file. No object lookup or expiry policy is applied here.
    pub fn reflog_retain<'a, Name, E>(
        &self,
        name: Name,
        options: Options,
        mut keep: impl FnMut(super::LineRef<'_>) -> Result<bool>,
    ) -> Result<Outcome>
    where
        Name: TryInto<&'a FullNameRef, Error = E>,
        std::result::Result<&'a FullNameRef, E>: ResultExt<Success = &'a FullNameRef>,
    {
        let name = name.try_into().or_raise(|| message("Invalid reflog name"))?;
        self.check_windows_device_name(name).or_error()?;
        let (base, relative) = self.reference_path_with_base(name)?;
        let _reference_lock = gix_lock::Marker::acquire_to_hold_resource(
            base.join(relative),
            gix_lock::acquire::Fail::Immediately,
            Some(base.into_owned()),
            0,
        )?;
        let path = self.reflog_path(name)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => bail!(message!("Expected a regular reflog file at {}", path.display()).validation()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(Outcome::default()),
            Err(err) => return Err(err).or_raise(|| message!("Could not inspect reflog {}", path.display())),
        }
        let bytes = fs::read(&path).or_raise(|| message!("Could not read reflog {}", path.display()))?;
        let mut output = if options.dry_run {
            None
        } else {
            Some(gix_lock::File::acquire_to_update_resource(
                &path,
                gix_lock::acquire::Fail::Immediately,
                None,
                0,
            )?)
        };
        let mut outcome = Outcome::default();
        for bytes in bytes.split_inclusive(|byte| *byte == b'\n') {
            let line = super::LineRef::from_bytes(bytes)
                .or_raise(|| message!("Could not decode reflog {}", path.display()))?;
            if keep(line)? {
                outcome.retained += 1;
                if let Some(output) = output.as_mut() {
                    output
                        .write_all(bytes)
                        .or_raise(|| message!("Could not rewrite reflog {}", path.display()))?;
                }
            } else {
                outcome.removed += 1;
            }
        }
        if outcome.removed != 0
            && let Some(output) = output
        {
            output
                .commit()
                .or_raise(|| message!("Could not publish reflog {}", path.display()))?;
        }
        Ok(outcome)
    }
}
