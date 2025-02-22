use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use bstr::{BStr, ByteSlice};
use gix_filter::{
    driver::apply::{Delay, MaybeDelayed},
    pipeline::convert::{ToGitOutcome, ToWorktreeOutcome},
};
use gix_object::tree::EntryKind;

use crate::blob::{Driver, Pipeline, ResourceKind};

/// A way to access roots for different kinds of resources that are possibly located and accessible in a worktree.
#[derive(Clone, Debug, Default)]
pub struct WorktreeRoots {
    /// A place where the source of a rewrite, rename or copy, or generally the previous version of resources, are located.
    pub old_root: Option<PathBuf>,
    /// A place where the destination of a rewrite, rename or copy, or generally the new version of resources, are located.
    pub new_root: Option<PathBuf>,
}

/// Access
impl WorktreeRoots {
    /// Return the root path for the given `kind`
    pub fn by_kind(&self, kind: ResourceKind) -> Option<&Path> {
        match kind {
            ResourceKind::OldOrSource => self.old_root.as_deref(),
            ResourceKind::NewOrDestination => self.new_root.as_deref(),
        }
    }

    /// Return `true` if all worktree roots are unset.
    pub fn is_unset(&self) -> bool {
        self.new_root.is_none() && self.old_root.is_none()
    }
}

/// Data as part of an [Outcome].
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum Data {
    /// The data to use for diffing was written into the buffer that was passed during the call to [`Pipeline::convert_to_diffable()`].
    Buffer,
    /// The size that the binary blob had at the given revision, without having applied filters, as it's either
    /// considered binary or above the big-file threshold.
    ///
    /// In this state, the binary file cannot be diffed.
    Binary {
        /// The size of the object prior to performing any filtering or as it was found on disk.
        ///
        /// Note that technically, the size isn't always representative of the same 'state' of the
        /// content, as once it can be the size of the blob in git, and once it's the size of file
        /// in the worktree.
        size: u64,
    },
}

/// The outcome returned by [Pipeline::convert_to_diffable()](super::Pipeline::convert_to_diffable()).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct Outcome {
    /// If available, an index into the `drivers` field to access more diff-related information of the driver for items
    /// at the given path, as previously determined by git-attributes.
    ///
    /// Note that drivers are queried even if there is no object available.
    pub driver_index: Option<usize>,
    /// The data itself, suitable for diffing, and if the object or worktree item is present at all.
    pub data: Option<Data>,
}

/// Options for use in a [`Pipeline`].
#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
pub struct Options {
    /// The amount of bytes that an object has to reach before being treated as binary.
    /// These objects will not be queried, nor will their data be processed in any way.
    /// If `0`, no file is ever considered binary due to their size.
    ///
    /// Note that for files stored in `git`, what counts is their stored, decompressed size,
    /// thus `git-lfs` files would typically not be considered binary unless one explicitly sets
    /// them
    pub large_file_threshold_bytes: u64,
    /// Capabilities of the file system which affect how we read worktree files.
    pub fs: gix_fs::Capabilities,
}

/// The specific way to convert a resource.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Mode {
    /// Always prepare the version of the resource as it would be in the work-tree, and
    /// apply binary-to-text filters if present.
    ///
    /// This is typically free for resources in the worktree, and will apply filters to resources in the
    /// object database.
    #[default]
    ToWorktreeAndBinaryToText,
    /// Prepare the version of the resource as it would be in the work-tree if
    /// binary-to-text filters are present (and apply them), or use the version in `git` otherwise.
    ToGitUnlessBinaryToTextIsPresent,
    /// Always prepare resources as they are stored in `git`.
    ///
    /// This is usually fastest, even though resources in the worktree needed to be converted files.
    ToGit,
}

impl Mode {
    fn to_worktree(self) -> bool {
        matches!(
            self,
            Mode::ToGitUnlessBinaryToTextIsPresent | Mode::ToWorktreeAndBinaryToText
        )
    }

    fn to_git(self) -> bool {
        matches!(self, Mode::ToGitUnlessBinaryToTextIsPresent | Mode::ToGit)
    }
}

///
pub mod convert_to_diffable {
    use std::collections::TryReserveError;

    use bstr::BString;
    use gix_object::tree::EntryKind;

    /// The error returned by [Pipeline::convert_to_diffable()](super::Pipeline::convert_to_diffable()).
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
        #[error("Failed to run '{cmd}' for binary-to-text conversion of entry at {rela_path}")]
        RunTextConvFilter {
            rela_path: BString,
            cmd: String,
            source: std::io::Error,
        },
        #[error("Tempfile for binary-to-text conversion for entry at {rela_path} could not be created")]
        CreateTempfile { rela_path: BString, source: std::io::Error },
        #[error("Binary-to-text conversion '{cmd}' for entry at {rela_path} failed with: {stderr}")]
        TextConvFilterFailed {
            rela_path: BString,
            cmd: String,
            stderr: BString,
        },
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

/// Lifecycle
impl Pipeline {
    /// Create a new instance of a pipeline which produces blobs suitable for diffing. `roots` allow to read worktree files directly, otherwise
    /// `worktree_filter` is used to transform object database data directly. `drivers` further configure individual paths.
    /// `options` are used to further configure the way we act..
    pub fn new(
        roots: WorktreeRoots,
        worktree_filter: gix_filter::Pipeline,
        mut drivers: Vec<super::Driver>,
        options: Options,
    ) -> Self {
        drivers.sort_by(|a, b| a.name.cmp(&b.name));
        Pipeline {
            roots,
            worktree_filter,
            drivers,
            options,
            attrs: {
                let mut out = gix_filter::attributes::search::Outcome::default();
                out.initialize_with_selection(&Default::default(), Some("diff"));
                out
            },
            path: Default::default(),
        }
    }
}

/// Access
impl Pipeline {
    /// Return all drivers that this instance was initialized with.
    ///
    /// They are sorted by [`name`](Driver::name) to support binary searches.
    pub fn drivers(&self) -> &[super::Driver] {
        &self.drivers
    }
}

/// Conversion
impl Pipeline {
    /// Convert the object at `id`, `mode`, `rela_path` and `kind`, providing access to `attributes` and `objects`.
    /// The resulting diff-able data is written into `out`, assuming it's not too large. The returned [`Outcome`]
    /// contains information on how to use `out`, or if it's filled at all.
    ///
    /// `attributes` must be returning the attributes at `rela_path`, and `objects` must be usable if `kind` is
    /// a resource in the object database, i.e. has no worktree root available.
    ///
    /// If `id` [is null](gix_hash::ObjectId::is_null()) or the file in question doesn't exist in the worktree in case
    /// [a root](WorktreeRoots) is present, then `out` will be left cleared and [Outcome::data] will be `None`.
    ///
    /// Note that `mode` is trusted, and we will not re-validate that the entry in the worktree actually is of that mode.
    ///
    /// Use `convert` to control what kind of the resource will be produced.
    ///
    /// ### About Tempfiles
    ///
    /// When querying from the object database and a binary and a [binary-to-text](Driver::binary_to_text_command) is set,
    /// a temporary file will be created to serve as input for the converter program, containing the worktree-data that
    /// exactly as it would be present in the worktree if checked out.
    ///
    /// As these files are ultimately named tempfiles, they will be leaked unless the [gix_tempfile] is configured with
    /// a signal handler. If they leak, they would remain in the system's `$TMP` directory.
    #[allow(clippy::too_many_arguments)]
    pub fn convert_to_diffable(
        &mut self,
        id: &gix_hash::oid,
        mode: EntryKind,
        rela_path: &BStr,
        kind: ResourceKind,
        attributes: &mut dyn FnMut(&BStr, &mut gix_filter::attributes::search::Outcome),
        objects: &dyn gix_object::FindObjectOrHeader,
        convert: Mode,
        out: &mut Vec<u8>,
    ) -> Result<Outcome, convert_to_diffable::Error> {
        let is_symlink = match mode {
            EntryKind::Link if self.options.fs.symlink => true,
            EntryKind::Blob | EntryKind::BlobExecutable => false,
            _ => {
                return Err(convert_to_diffable::Error::InvalidEntryKind {
                    rela_path: rela_path.to_owned(),
                    actual: mode,
                })
            }
        };

        out.clear();
        attributes(rela_path, &mut self.attrs);
        let attr = self.attrs.iter_selected().next().expect("pre-initialized with 'diff'");
        let driver_index = attr
            .assignment
            .state
            .as_bstr()
            .and_then(|name| self.drivers.binary_search_by(|d| d.name.as_bstr().cmp(name)).ok());
        let driver = driver_index.map(|idx| &self.drivers[idx]);
        let mut is_binary = if let Some(driver) = driver {
            driver
                .is_binary
                .map(|is_binary| is_binary && driver.binary_to_text_command.is_none())
        } else {
            attr.assignment.state.is_unset().then_some(true)
        };
        match self.roots.by_kind(kind) {
            Some(root) => {
                self.path.clear();
                self.path.push(root);
                self.path.push(gix_path::from_bstr(rela_path));
                let data = if is_symlink {
                    let target = none_if_missing(std::fs::read_link(&self.path)).map_err(|err| {
                        convert_to_diffable::Error::ReadLink {
                            rela_path: rela_path.to_owned(),
                            source: err,
                        }
                    })?;
                    target.map(|target| {
                        out.extend_from_slice(gix_path::into_bstr(target).as_ref());
                        Data::Buffer
                    })
                } else {
                    let need_size_only = is_binary == Some(true);
                    let size_in_bytes = (need_size_only
                        || (is_binary != Some(false) && self.options.large_file_threshold_bytes > 0))
                        .then(|| {
                            none_if_missing(self.path.metadata().map(|md| md.len())).map_err(|err| {
                                convert_to_diffable::Error::OpenOrRead {
                                    rela_path: rela_path.to_owned(),
                                    source: err,
                                }
                            })
                        })
                        .transpose()?;
                    match size_in_bytes {
                        Some(None) => None, // missing as identified by the size check
                        Some(Some(size)) if size > self.options.large_file_threshold_bytes || need_size_only => {
                            Some(Data::Binary { size })
                        }
                        _ => {
                            match driver
                                .filter(|_| convert.to_worktree())
                                .and_then(|d| d.prepare_binary_to_text_cmd(&self.path))
                            {
                                Some(cmd) => {
                                    // Avoid letting the driver program fail if it doesn't exist.
                                    if self.options.large_file_threshold_bytes == 0
                                        && none_if_missing(std::fs::symlink_metadata(&self.path))
                                            .map_err(|err| convert_to_diffable::Error::OpenOrRead {
                                                rela_path: rela_path.to_owned(),
                                                source: err,
                                            })?
                                            .is_none()
                                    {
                                        None
                                    } else {
                                        run_cmd(rela_path, cmd, out)?;
                                        Some(Data::Buffer)
                                    }
                                }
                                None => {
                                    let file = none_if_missing(std::fs::File::open(&self.path)).map_err(|err| {
                                        convert_to_diffable::Error::OpenOrRead {
                                            rela_path: rela_path.to_owned(),
                                            source: err,
                                        }
                                    })?;

                                    match file {
                                        Some(mut file) => {
                                            if convert.to_git() {
                                                let res = self.worktree_filter.convert_to_git(
                                                    file,
                                                    gix_path::from_bstr(rela_path).as_ref(),
                                                    attributes,
                                                    &mut |buf| objects.try_find(id, buf).map(|obj| obj.map(|_| ())),
                                                )?;

                                                match res {
                                                    ToGitOutcome::Unchanged(mut file) => {
                                                        file.read_to_end(out).map_err(|err| {
                                                            convert_to_diffable::Error::OpenOrRead {
                                                                rela_path: rela_path.to_owned(),
                                                                source: err,
                                                            }
                                                        })?;
                                                    }
                                                    ToGitOutcome::Process(mut stream) => {
                                                        stream.read_to_end(out).map_err(|err| {
                                                            convert_to_diffable::Error::OpenOrRead {
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
                                            } else {
                                                file.read_to_end(out).map_err(|err| {
                                                    convert_to_diffable::Error::OpenOrRead {
                                                        rela_path: rela_path.to_owned(),
                                                        source: err,
                                                    }
                                                })?;
                                            }

                                            Some(if is_binary.unwrap_or_else(|| is_binary_buf(out)) {
                                                let size = out.len() as u64;
                                                out.clear();
                                                Data::Binary { size }
                                            } else {
                                                Data::Buffer
                                            })
                                        }
                                        None => None,
                                    }
                                }
                            }
                        }
                    }
                };
                Ok(Outcome { driver_index, data })
            }
            None => {
                let data = if id.is_null() {
                    None
                } else {
                    let header = objects
                        .try_header(id)
                        .map_err(gix_object::find::existing_object::Error::Find)?
                        .ok_or_else(|| gix_object::find::existing_object::Error::NotFound { oid: id.to_owned() })?;
                    if is_binary.is_none()
                        && self.options.large_file_threshold_bytes > 0
                        && header.size > self.options.large_file_threshold_bytes
                    {
                        is_binary = Some(true);
                    }
                    let data = if is_binary == Some(true) {
                        Data::Binary { size: header.size }
                    } else {
                        objects
                            .try_find(id, out)
                            .map_err(gix_object::find::existing_object::Error::Find)?
                            .ok_or_else(|| gix_object::find::existing_object::Error::NotFound { oid: id.to_owned() })?;
                        if matches!(mode, EntryKind::Blob | EntryKind::BlobExecutable)
                            && convert == Mode::ToWorktreeAndBinaryToText
                            || (convert == Mode::ToGitUnlessBinaryToTextIsPresent
                                && driver.is_some_and(|d| d.binary_to_text_command.is_some()))
                        {
                            let res =
                                self.worktree_filter
                                    .convert_to_worktree(out, rela_path, attributes, Delay::Forbid)?;

                            let cmd_and_file = driver
                                .and_then(|d| {
                                    d.binary_to_text_command.is_some().then(|| {
                                        gix_tempfile::new(
                                            std::env::temp_dir(),
                                            gix_tempfile::ContainingDirectory::Exists,
                                            gix_tempfile::AutoRemove::Tempfile,
                                        )
                                        .and_then(|mut tmp_file| {
                                            self.path.clear();
                                            tmp_file.with_mut(|tmp| self.path.push(tmp.path()))?;
                                            Ok(tmp_file)
                                        })
                                        .map(|tmp_file| {
                                            (
                                                d.prepare_binary_to_text_cmd(&self.path)
                                                    .expect("always get cmd if command is set"),
                                                tmp_file,
                                            )
                                        })
                                    })
                                })
                                .transpose()
                                .map_err(|err| convert_to_diffable::Error::CreateTempfile {
                                    source: err,
                                    rela_path: rela_path.to_owned(),
                                })?;
                            match cmd_and_file {
                                Some((cmd, mut tmp_file)) => {
                                    match res {
                                        ToWorktreeOutcome::Unchanged(buf) | ToWorktreeOutcome::Buffer(buf) => {
                                            tmp_file.write_all(buf)
                                        }
                                        ToWorktreeOutcome::Process(MaybeDelayed::Immediate(mut stream)) => {
                                            std::io::copy(&mut stream, &mut tmp_file).map(|_| ())
                                        }
                                        ToWorktreeOutcome::Process(MaybeDelayed::Delayed(_)) => {
                                            unreachable!("we prohibit this")
                                        }
                                    }
                                    .map_err(|err| {
                                        convert_to_diffable::Error::StreamCopy {
                                            source: err,
                                            rela_path: rela_path.to_owned(),
                                        }
                                    })?;
                                    out.clear();
                                    run_cmd(rela_path, cmd, out)?;
                                }
                                None => match res {
                                    ToWorktreeOutcome::Unchanged(_) => {}
                                    ToWorktreeOutcome::Buffer(src) => {
                                        out.clear();
                                        out.try_reserve(src.len())?;
                                        out.extend_from_slice(src);
                                    }
                                    ToWorktreeOutcome::Process(MaybeDelayed::Immediate(mut stream)) => {
                                        std::io::copy(&mut stream, out).map_err(|err| {
                                            convert_to_diffable::Error::StreamCopy {
                                                rela_path: rela_path.to_owned(),
                                                source: err,
                                            }
                                        })?;
                                    }
                                    ToWorktreeOutcome::Process(MaybeDelayed::Delayed(_)) => {
                                        unreachable!("we prohibit this")
                                    }
                                },
                            }
                        }

                        if driver.map_or(true, |d| d.binary_to_text_command.is_none())
                            && is_binary.unwrap_or_else(|| is_binary_buf(out))
                        {
                            let size = out.len() as u64;
                            out.clear();
                            Data::Binary { size }
                        } else {
                            Data::Buffer
                        }
                    };
                    Some(data)
                };
                Ok(Outcome { driver_index, data })
            }
        }
    }
}

fn is_binary_buf(buf: &[u8]) -> bool {
    let buf = &buf[..buf.len().min(8000)];
    buf.contains(&0)
}

fn none_if_missing<T>(res: std::io::Result<T>) -> std::io::Result<Option<T>> {
    match res {
        Ok(data) => Ok(Some(data)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn run_cmd(rela_path: &BStr, mut cmd: Command, out: &mut Vec<u8>) -> Result<(), convert_to_diffable::Error> {
    gix_trace::debug!(cmd = ?cmd, "Running binary-to-text command");
    let mut res = cmd
        .output()
        .map_err(|err| convert_to_diffable::Error::RunTextConvFilter {
            rela_path: rela_path.to_owned(),
            cmd: format!("{cmd:?}"),
            source: err,
        })?;
    if !res.status.success() {
        return Err(convert_to_diffable::Error::TextConvFilterFailed {
            rela_path: rela_path.to_owned(),
            cmd: format!("{cmd:?}"),
            stderr: res.stderr.into(),
        });
    }
    out.append(&mut res.stdout);
    Ok(())
}

impl Driver {
    /// Produce an invocable command pre-configured to produce the filtered output on stdout after reading `path`.
    pub fn prepare_binary_to_text_cmd(&self, path: &Path) -> Option<std::process::Command> {
        let command: &BStr = self.binary_to_text_command.as_ref()?.as_ref();
        let cmd = gix_command::prepare(gix_path::from_bstr(command).into_owned())
            // TODO: Add support for an actual Context, validate it *can* match Git
            .with_context(Default::default())
            .command_may_be_shell_script()
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .arg(path)
            .into();
        Some(cmd)
    }
}
