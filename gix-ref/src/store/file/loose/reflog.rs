use gix_error::{Exn, Metadata, ResultExt, message};

use std::{io::Read, path::PathBuf};

use crate::{
    FullNameRef,
    store_impl::{file, file::log},
};

impl file::Store {
    /// Returns true if a reflog exists for the given reference `name`.
    ///
    /// Please note that this method shouldn't be used to check if a log exists before trying to read it, but instead
    /// is meant to be the fastest possible way to determine if a log exists or not.
    /// If the caller needs to know if it's readable, try to read the log instead with a reverse or forward iterator.
    pub fn reflog_exists<'a, Name, E>(&self, name: Name) -> Result<bool, E>
    where
        Name: TryInto<&'a FullNameRef, Error = E>,
    {
        Ok(self.reflog_path(name.try_into()?).is_file())
    }

    /// Return a reflog reverse iterator for the given fully qualified `name`, reading chunks from the back into the fixed buffer `buf`.
    ///
    /// The iterator will traverse log entries from most recent to oldest, reading the underlying file in chunks from the back.
    /// Return `Ok(None)` if no reflog exists.
    ///
    /// Read failures include metadata `path` (native path), the resolved reflog path.
    pub fn reflog_iter_rev<'a, 'b, Name, E>(
        &self,
        name: Name,
        buf: &'b mut [u8],
    ) -> Result<Option<log::iter::Reverse<'b, std::fs::File>>, Exn>
    where
        Name: TryInto<&'a FullNameRef, Error = E>,
        Result<&'a FullNameRef, E>: ResultExt<Success = &'a FullNameRef>,
    {
        let name = name
            .try_into()
            .or_raise_erased(|| message("The reflog name or path is not a valid ref name"))?;
        self.reflog_iter_rev_inner(name, buf)
            .or_raise_erased(|| Metadata::new("Could not read reflog").with("path", self.reflog_path(name)))
    }

    pub(crate) fn reflog_iter_rev_inner<'b>(
        &self,
        name: &FullNameRef,
        buf: &'b mut [u8],
    ) -> std::io::Result<Option<log::iter::Reverse<'b, std::fs::File>>> {
        let path = self.reflog_path(name);
        if path.is_dir() {
            return Ok(None);
        }
        match std::fs::File::open(&path) {
            Ok(file) => Ok(Some(log::iter::reverse(file, buf)?)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Return a reflog forward iterator for the given fully qualified `name` and write its file contents into `buf`.
    ///
    /// The iterator will traverse log entries from oldest to newest.
    /// Return `Ok(None)` if no reflog exists.
    ///
    /// Read failures include metadata `path` (native path), the resolved reflog path.
    pub fn reflog_iter<'a, 'b, Name, E>(
        &self,
        name: Name,
        buf: &'b mut Vec<u8>,
    ) -> Result<Option<log::iter::Forward<'b>>, Exn>
    where
        Name: TryInto<&'a FullNameRef, Error = E>,
        Result<&'a FullNameRef, E>: ResultExt<Success = &'a FullNameRef>,
    {
        let name = name
            .try_into()
            .or_raise_erased(|| message("The reflog name or path is not a valid ref name"))?;
        self.reflog_iter_inner(name, buf)
            .or_raise_erased(|| Metadata::new("Could not read reflog").with("path", self.reflog_path(name)))
    }

    pub(crate) fn reflog_iter_inner<'b>(
        &self,
        name: &FullNameRef,
        buf: &'b mut Vec<u8>,
    ) -> std::io::Result<Option<log::iter::Forward<'b>>> {
        let path = self.reflog_path(name);
        match std::fs::File::open(&path) {
            Ok(mut file) => {
                buf.clear();
                if let Err(err) = file.read_to_end(buf) {
                    return if path.is_dir() { Ok(None) } else { Err(err) };
                }
                Ok(Some(log::iter::forward(buf)))
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            #[cfg(windows)]
            Err(err) if err.kind() == std::io::ErrorKind::PermissionDenied => Ok(None),
            Err(err) => Err(err),
        }
    }
}

impl file::Store {
    /// Implements the logic required to transform a fully qualified refname into its log name
    pub(crate) fn reflog_path(&self, name: &FullNameRef) -> PathBuf {
        let (base, rela_path) = self.reflog_base_and_relative_path(name);
        base.join(rela_path)
    }
}

///
pub mod create_or_update {
    use std::{
        borrow::Cow,
        io::Write,
        path::{Path, PathBuf},
    };

    use gix_error::{ErrorExt, Exn, Metadata, ResultExt};
    use gix_hash::{ObjectId, oid};
    use gix_object::bstr::BStr;

    use crate::store_impl::{file, file::WriteReflog};

    impl file::Store {
        /// Append a reflog entry. Filesystem failures include metadata `path` (native path), the affected file or directory.
        /// A missing identity is reported as [`MissingCommitter`] only when a log entry must actually be written.
        pub(crate) fn reflog_create_or_append(
            &self,
            name: &FullNameRef,
            previous_oid: Option<ObjectId>,
            new: &oid,
            committer: Option<gix_actor::SignatureRef<'_>>,
            message: &BStr,
            mut force_create_reflog: bool,
        ) -> Result<(), Exn> {
            let (reflog_base, full_name) = self.reflog_base_and_relative_path(name);
            match self.write_reflog {
                WriteReflog::Normal | WriteReflog::Always => {
                    if self.write_reflog == WriteReflog::Always {
                        force_create_reflog = true;
                    }
                    let mut options = std::fs::OpenOptions::new();
                    options.append(true).read(false);
                    let log_path = reflog_base.join(&full_name);

                    if force_create_reflog || self.should_autocreate_reflog(&full_name) {
                        let parent_dir = log_path.parent().expect("always with parent directory");
                        gix_tempfile::create_dir::all(parent_dir, Default::default()).or_raise_erased(|| {
                            Metadata::new("Could not create reflog directory").with("path", parent_dir)
                        })?;
                        options.create(true);
                    }

                    let file_for_appending = match options.open(&log_path) {
                        Ok(f) => Some(f),
                        Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
                        Err(err) => {
                            // TODO: when Kind::IsADirectory becomes stable, use that.
                            if log_path.is_dir() {
                                gix_tempfile::remove_dir::empty_depth_first(log_path.clone())
                                    .and_then(|_| options.open(&log_path))
                                    .map(Some)
                                    .or_raise_erased(|| {
                                        Metadata::new("Could not open reflog for appending")
                                            .with("path", log_path.as_path())
                                    })?
                            } else {
                                return Err(err
                                    .and_raise(
                                        Metadata::new("Could not open reflog for appending").with("path", log_path),
                                    )
                                    .erased());
                            }
                        }
                    };

                    if let Some(mut file) = file_for_appending {
                        let committer = committer.ok_or_else(|| MissingCommitter.raise_erased())?;
                        write!(file, "{} {} ", previous_oid.unwrap_or_else(|| new.kind().null()), new)
                            .and_then(|_| committer.trim().write_to(&mut file))
                            .and_then(|_| {
                                if !message.is_empty() {
                                    writeln!(file, "\t{message}")
                                } else {
                                    writeln!(file)
                                }
                            })
                            .or_raise_erased(|| {
                                Metadata::new("Could not append reflog entry").with("path", log_path.as_path())
                            })?;
                    }
                    Ok(())
                }
                WriteReflog::Disable => Ok(()),
            }
        }

        fn should_autocreate_reflog(&self, full_name: &Path) -> bool {
            full_name.starts_with("refs/heads/")
                || full_name.starts_with("refs/remotes/")
                || full_name.starts_with("refs/notes/")
                || full_name.starts_with("refs/worktree/") // NOTE: git does not write reflogs for worktree private refs
                || full_name == Path::new("HEAD")
        }

        /// Returns the base paths for all reflogs
        pub(in crate::store_impl::file) fn reflog_base_and_relative_path<'a>(
            &self,
            name: &'a FullNameRef,
        ) -> (PathBuf, Cow<'a, Path>) {
            let is_reflog = true;
            let (base, name) = self.to_base_dir_and_relative_name(name, is_reflog);
            (
                base.join("logs"),
                match &self.namespace {
                    None => gix_path::to_native_path_on_windows(name.as_bstr()),
                    Some(namespace) => gix_path::to_native_path_on_windows(
                        namespace.to_owned().into_namespaced_name(name).into_inner(),
                    ),
                },
            )
        }
    }

    /// A reflog entry requires a committer identity which wasn't provided.
    #[derive(Debug)]
    pub struct MissingCommitter;

    impl std::fmt::Display for MissingCommitter {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("reflog messages need a committer which isn't set")
        }
    }

    impl std::error::Error for MissingCommitter {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&crate::INVALID_REFLOG)
        }
    }

    #[cfg(test)]
    mod tests;

    use crate::FullNameRef;
}
