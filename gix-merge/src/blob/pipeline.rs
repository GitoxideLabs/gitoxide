use super::{Pipeline, ResourceKind};
use bstr::BStr;
use gix_filter::driver::apply::{Delay, MaybeDelayed};
use gix_filter::pipeline::convert::{ToGitOutcome, ToWorktreeOutcome};
use gix_object::tree::EntryKind;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Options for use in a [`Pipeline`].
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
pub struct Options {
    /// The amount of bytes that an object has to reach before being treated as binary.
    /// These objects will not be queried, nor will their data be processed in any way.
    /// If `0`, no file is ever considered binary due to their size.
    ///
    /// Note that for files stored in `git`, what counts is their stored, decompressed size,
    /// thus `git-lfs` files would typically not be considered binary unless one explicitly sets
    /// them.
    /// However, if they are to be retrieved from the worktree, the worktree size is what matters,
    /// even though that also might be a `git-lfs` file which is small in Git.
    pub large_file_threshold_bytes: u64,
}

/// The specific way to convert a resource.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Mode {
    /// Prepare resources as they are stored in `git`.
    ///
    /// This is naturally the case when object-ids are used, but a conversion is needed
    /// when data is read from a worktree.
    #[default]
    ToGit,
    /// For sources that are object-ids, convert them to what *would* be stored in the worktree,
    /// and back to what *would* be stored in Git.
    ///
    /// Sources that are located in a worktree are merely converted to what *would* be stored in Git.
    ///
    /// This is useful to prevent merge conflicts due to inconcistent whitespace.
    Renormalize,
}

/// A way to access roots for different kinds of resources that are possibly located and accessible in a worktree.
#[derive(Clone, Debug, Default)]
pub struct WorktreeRoots {
    /// The worktree root where the current (or our) version of the resource is present.
    pub current_root: Option<PathBuf>,
    /// The worktree root where the other (or their) version of the resource is present.
    pub other_root: Option<PathBuf>,
    /// The worktree root where containing the resource of the common ancestor of our and their version.
    pub common_ancestor_root: Option<PathBuf>,
}

impl WorktreeRoots {
    /// Return the root path for the given `kind`
    pub fn by_kind(&self, kind: ResourceKind) -> Option<&Path> {
        match kind {
            ResourceKind::CurrentOrOurs => self.current_root.as_deref(),
            ResourceKind::CommonAncestorOrBase => self.common_ancestor_root.as_deref(),
            ResourceKind::OtherOrTheirs => self.other_root.as_deref(),
        }
    }

    /// Return `true` if all worktree roots are unset.
    pub fn is_unset(&self) -> bool {
        self.current_root.is_none() && self.other_root.is_none() && self.common_ancestor_root.is_none()
    }
}

/// Lifecycle
impl Pipeline {
    /// Create a new instance of a pipeline which produces blobs suitable for merging.
    ///
    /// `roots` allow to read worktree files directly, and `worktree_filter` is used
    /// to transform object database data directly.
    /// `options` are used to further configure the way we act.
    pub fn new(roots: WorktreeRoots, worktree_filter: gix_filter::Pipeline, options: Options) -> Self {
        Pipeline {
            roots,
            filter: worktree_filter,
            options,
            path: Default::default(),
        }
    }
}

/// Access
impl Pipeline {}

/// Data as returned by [`Pipeline::convert_to_mergeable()`].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Data {
    /// The data to use for merging was written into the buffer that was passed during the call to [`Pipeline::convert_to_mergeable()`].
    Buffer,
    /// The file or blob is above the big-file threshold and cannot be processed.
    ///
    /// In this state, the file cannot be merged.
    TooLarge {
        /// The size of the object prior to performing any filtering or as it was found on disk.
        ///
        /// Note that technically, the size isn't always representative of the same 'state' of the
        /// content, as once it can be the size of the blob in git, and once it's the size of file
        /// in the worktree - both can differ a lot depending on filters.
        size: u64,
    },
}

///
pub mod convert_to_mergeable {
    use std::collections::TryReserveError;

    use bstr::BString;
    use gix_object::tree::EntryKind;

    /// The error returned by [Pipeline::convert_to_mergeable()](super::Pipeline::convert_to_mergeable()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Entry at '{rela_path}' must be regular file or symlink, but was {actual:?}")]
        InvalidEntryKind { rela_path: BString, actual: EntryKind },
        #[error("Entry at '{rela_path}' could not be read as symbolic link")]
        ReadLink { rela_path: BString, source: std::io::Error },
        #[error("Entry at '{rela_path}' could not be opened for reading or read from")]
        OpenOrRead { rela_path: BString, source: std::io::Error },
        #[error("Entry at '{rela_path}' could not be copied from a filter process to a memory buffer")]
        StreamCopy { rela_path: BString, source: std::io::Error },
        #[error(transparent)]
        FindObject(#[from] gix_object::find::existing_object::Error),
        #[error(transparent)]
        ConvertToWorktree(#[from] gix_filter::pipeline::convert::to_worktree::Error),
        #[error(transparent)]
        ConvertToGit(#[from] gix_filter::pipeline::convert::to_git::Error),
        #[error("Memory allocation failed")]
        OutOfMemory(#[from] TryReserveError),
    }
}

/// Conversion
impl Pipeline {
    /// Convert the object at `id`, `mode`, `rela_path` and `kind`, providing access to `attributes` and `objects`.
    /// The resulting merge-able data is written into `out`, if it's not too large.
    /// The returned [`Data`] contains information on how to use `out`, which will be cleared if it is `None`, indicating
    /// that no object was found at the location *on disk* - it's always an error to provide an object ID that doesn't exist
    /// in the object database.
    ///
    /// `attributes` must be returning the attributes at `rela_path` and is used for obtaining worktree filter settings,
    /// and `objects` must be usable if `kind` is a resource in the object database,
    /// i.e. if no worktree root is available. It's notable that if a worktree root is present for `kind`,
    /// then a `rela_path` is used to access it on disk.
    ///
    /// If `id` [is null](gix_hash::ObjectId::is_null()) or the file in question doesn't exist in the worktree in case
    /// [a root](WorktreeRoots) is present, then `out` will be left cleared and the output data will be `None`.
    /// This is useful to simplify the calling code as empty buffers signal that nothing is there.
    ///
    /// Note that `mode` is trusted, and we will not re-validate that the entry in the worktree actually is of that mode.
    /// Only blobs are allowed.
    ///
    /// Use `convert` to control what kind of the resource will be produced.
    #[allow(clippy::too_many_arguments)]
    pub fn convert_to_mergeable(
        &mut self,
        id: &gix_hash::oid,
        mode: EntryKind,
        rela_path: &BStr,
        kind: ResourceKind,
        attributes: &mut dyn FnMut(&BStr, &mut gix_filter::attributes::search::Outcome),
        objects: &dyn gix_object::FindObjectOrHeader,
        convert: Mode,
        out: &mut Vec<u8>,
    ) -> Result<Option<Data>, convert_to_mergeable::Error> {
        if !matches!(mode, EntryKind::Blob | EntryKind::BlobExecutable) {
            return Err(convert_to_mergeable::Error::InvalidEntryKind {
                rela_path: rela_path.to_owned(),
                actual: mode,
            });
        }

        out.clear();
        match self.roots.by_kind(kind) {
            Some(root) => {
                self.path.clear();
                self.path.push(root);
                self.path.push(gix_path::from_bstr(rela_path));
                let size_in_bytes = (self.options.large_file_threshold_bytes > 0)
                    .then(|| {
                        none_if_missing(self.path.metadata().map(|md| md.len())).map_err(|err| {
                            convert_to_mergeable::Error::OpenOrRead {
                                rela_path: rela_path.to_owned(),
                                source: err,
                            }
                        })
                    })
                    .transpose()?;
                let data = match size_in_bytes {
                    Some(None) => None, // missing as identified by the size check
                    Some(Some(size)) if size > self.options.large_file_threshold_bytes => Some(Data::TooLarge { size }),
                    _ => {
                        let file = none_if_missing(std::fs::File::open(&self.path)).map_err(|err| {
                            convert_to_mergeable::Error::OpenOrRead {
                                rela_path: rela_path.to_owned(),
                                source: err,
                            }
                        })?;

                        if let Some(file) = file {
                            match convert {
                                Mode::ToGit | Mode::Renormalize => {
                                    let res = self.filter.convert_to_git(
                                        file,
                                        gix_path::from_bstr(rela_path).as_ref(),
                                        attributes,
                                        &mut |buf| {
                                            if convert == Mode::Renormalize {
                                                Ok(None)
                                            } else {
                                                objects.try_find(id, buf).map(|obj| obj.map(|_| ()))
                                            }
                                        },
                                    )?;

                                    match res {
                                        ToGitOutcome::Unchanged(mut file) => {
                                            file.read_to_end(out).map_err(|err| {
                                                convert_to_mergeable::Error::OpenOrRead {
                                                    rela_path: rela_path.to_owned(),
                                                    source: err,
                                                }
                                            })?;
                                        }
                                        ToGitOutcome::Process(mut stream) => {
                                            stream.read_to_end(out).map_err(|err| {
                                                convert_to_mergeable::Error::OpenOrRead {
                                                    rela_path: rela_path.to_owned(),
                                                    source: err,
                                                }
                                            })?;
                                        }
                                        ToGitOutcome::Buffer(buf) => {
                                            out.clear();
                                            out.try_reserve(buf.len())?;
                                            out.extend_from_slice(buf);
                                        }
                                    }
                                }
                            }

                            Some(Data::Buffer)
                        } else {
                            None
                        }
                    }
                };
                Ok(data)
            }
            None => {
                let data = if id.is_null() {
                    None
                } else {
                    let header = objects
                        .try_header(id)
                        .map_err(gix_object::find::existing_object::Error::Find)?
                        .ok_or_else(|| gix_object::find::existing_object::Error::NotFound { oid: id.to_owned() })?;
                    let is_binary = self.options.large_file_threshold_bytes > 0
                        && header.size > self.options.large_file_threshold_bytes;
                    let data = if is_binary {
                        Data::TooLarge { size: header.size }
                    } else {
                        objects
                            .try_find(id, out)
                            .map_err(gix_object::find::existing_object::Error::Find)?
                            .ok_or_else(|| gix_object::find::existing_object::Error::NotFound { oid: id.to_owned() })?;

                        if convert == Mode::Renormalize {
                            {
                                let res = self
                                    .filter
                                    .convert_to_worktree(out, rela_path, attributes, Delay::Forbid)?;

                                match res {
                                    ToWorktreeOutcome::Unchanged(_) => {}
                                    ToWorktreeOutcome::Buffer(src) => {
                                        out.clear();
                                        out.try_reserve(src.len())?;
                                        out.extend_from_slice(src);
                                    }
                                    ToWorktreeOutcome::Process(MaybeDelayed::Immediate(mut stream)) => {
                                        std::io::copy(&mut stream, out).map_err(|err| {
                                            convert_to_mergeable::Error::StreamCopy {
                                                rela_path: rela_path.to_owned(),
                                                source: err,
                                            }
                                        })?;
                                    }
                                    ToWorktreeOutcome::Process(MaybeDelayed::Delayed(_)) => {
                                        unreachable!("we prohibit this")
                                    }
                                }
                            }

                            let res = self.filter.convert_to_git(
                                &**out,
                                &gix_path::from_bstr(rela_path),
                                attributes,
                                &mut |_buf| Ok(None),
                            )?;

                            match res {
                                ToGitOutcome::Unchanged(_) => {}
                                ToGitOutcome::Process(mut stream) => {
                                    stream
                                        .read_to_end(out)
                                        .map_err(|err| convert_to_mergeable::Error::OpenOrRead {
                                            rela_path: rela_path.to_owned(),
                                            source: err,
                                        })?;
                                }
                                ToGitOutcome::Buffer(buf) => {
                                    out.clear();
                                    out.try_reserve(buf.len())?;
                                    out.extend_from_slice(buf);
                                }
                            }
                        }

                        Data::Buffer
                    };
                    Some(data)
                };
                Ok(data)
            }
        }
    }
}

fn none_if_missing<T>(res: std::io::Result<T>) -> std::io::Result<Option<T>> {
    match res {
        Ok(data) => Ok(Some(data)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}
