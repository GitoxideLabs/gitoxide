use crate::blob::{builtin_driver, PlatformRef, Resolution};
use std::io::Read;
use std::path::PathBuf;

/// Options for the use in the [`PlatformRef::merge()`] call.
#[derive(Default, Copy, Clone, Debug, Eq, PartialEq)]
pub struct Options {
    /// If `true`, the resources being merged are contained in a virtual ancestor,
    /// which is the case when merge bases are merged into one.
    pub is_virtual_ancestor: bool,
    /// Determine how to resolve conflicts. If `None`, no conflict resolution is possible, and it picks a side.
    pub resolve_binary_with: Option<builtin_driver::binary::ResolveWith>,
    /// Options for the builtin [text driver](crate::blob::BuiltinDriver::Text).
    pub text: builtin_driver::text::Options,
}

/// The error returned by [`PlatformRef::merge()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("At least one resource was too large to be processed")]
    ResourceTooLarge,
    #[error(transparent)]
    PrepareExternalDriver(#[from] inner::prepare_external_driver::Error),
    #[error("Failed to launch external merge driver: {cmd}")]
    SpawnExternalDriver { cmd: String, source: std::io::Error },
    #[error("External merge driver failed with non-zero exit status {status:?}: {cmd}")]
    ExternalDriverFailure {
        status: std::process::ExitStatus,
        cmd: String,
    },
    #[error("IO failed when dealing with merge-driver output")]
    ExternalDriverIO(#[from] std::io::Error),
}

/// The product of a [`PlatformRef::prepare_external_driver()`] operation.
///
/// This type allows to creation of [`std::process::Command`], ready to run, with `stderr` and `stdout` set to *inherit*,
/// but `stdin` closed.
/// It's expected to leave its result in the file substituted at `current` which is then supposed to be read back from there.
// TODO: remove dead-code annotation
#[allow(dead_code)]
pub struct Command {
    /// The pre-configured command
    cmd: std::process::Command,
    /// A tempfile holding the *current* (ours) state of the resource.
    current: gix_tempfile::Handle<gix_tempfile::handle::Closed>,
    /// The path at which `current` is located, for reading the result back from later.
    current_path: PathBuf,
    /// A tempfile holding the *ancestor* (base) state of the resource.
    ancestor: gix_tempfile::Handle<gix_tempfile::handle::Closed>,
    /// A tempfile holding the *other* (their) state of the resource.
    other: gix_tempfile::Handle<gix_tempfile::handle::Closed>,
}

// Just to keep things here but move them a level up later.
pub(super) mod inner {
    ///
    pub mod prepare_external_driver {
        use crate::blob::builtin_driver::text::Conflict;
        use crate::blob::platform::{merge, DriverChoice};
        use crate::blob::{builtin_driver, BuiltinDriver, Driver, PlatformRef, ResourceKind};
        use bstr::{BString, ByteVec};
        use gix_tempfile::{AutoRemove, ContainingDirectory};
        use std::io::Write;
        use std::ops::{Deref, DerefMut};
        use std::path::{Path, PathBuf};
        use std::process::Stdio;

        /// The error returned by [PlatformRef::prepare_external_driver()](PlatformRef::prepare_external_driver()).
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error("The resource of kind {kind:?} was too large to be processed")]
            ResourceTooLarge { kind: ResourceKind },
            #[error(
                "Tempfile to store content of '{rela_path}' ({kind:?}) for passing to external merge command could not be created"
            )]
            CreateTempfile {
                rela_path: BString,
                kind: ResourceKind,
                source: std::io::Error,
            },
            #[error(
                "Could not write content of '{rela_path}' ({kind:?}) to tempfile for passing to external merge command"
            )]
            WriteTempfile {
                rela_path: BString,
                kind: ResourceKind,
                source: std::io::Error,
            },
        }

        /// Plumbing
        impl<'parent> PlatformRef<'parent> {
            /// Given `merge_command` and `context`, typically obtained from git-configuration, and the currently set merge-resources,
            /// prepare the invocation and temporary files needed to launch it according to protocol.
            /// See the documentation of [`Driver::command`] for possible substitutions.
            ///
            /// Please note that this is an expensive operation this will always create three temporary files to hold all sides of the merge.
            ///
            /// The resulting command should be spawned, and when successful, [the result file can be opened](merge::Command::open_result_file)
            /// to read back the result into a suitable buffer.
            ///
            /// ### Deviation
            ///
            /// * We allow passing more context than Git would by taking a whole `context`,
            ///   it's up to the caller to decide how much is filled.
            /// * Our tempfiles aren't suffixed `.merge_file_XXXXXX` with `X` replaced with characters for uniqueness.
            pub fn prepare_external_driver(
                &self,
                merge_command: BString,
                builtin_driver::text::Labels {
                    ancestor,
                    current,
                    other,
                }: builtin_driver::text::Labels<'_>,
                context: gix_command::Context,
            ) -> Result<merge::Command, Error> {
                fn write_data(
                    data: &[u8],
                ) -> std::io::Result<(gix_tempfile::Handle<gix_tempfile::handle::Closed>, PathBuf)> {
                    let mut file = gix_tempfile::new(Path::new(""), ContainingDirectory::Exists, AutoRemove::Tempfile)?;
                    file.write_all(data)?;
                    let mut path = Default::default();
                    file.with_mut(|f| {
                        f.path().clone_into(&mut path);
                    })?;
                    let file = file.close()?;
                    Ok((file, path))
                }

                let base = self.ancestor.data.as_slice().ok_or(Error::ResourceTooLarge {
                    kind: ResourceKind::CommonAncestorOrBase,
                })?;
                let ours = self.current.data.as_slice().ok_or(Error::ResourceTooLarge {
                    kind: ResourceKind::CurrentOrOurs,
                })?;
                let theirs = self.other.data.as_slice().ok_or(Error::ResourceTooLarge {
                    kind: ResourceKind::OtherOrTheirs,
                })?;

                let (base_tmp, base_path) = write_data(base).map_err(|err| Error::CreateTempfile {
                    rela_path: self.ancestor.rela_path.into(),
                    kind: ResourceKind::CommonAncestorOrBase,
                    source: err,
                })?;
                let (ours_tmp, ours_path) = write_data(ours).map_err(|err| Error::CreateTempfile {
                    rela_path: self.current.rela_path.into(),
                    kind: ResourceKind::CurrentOrOurs,
                    source: err,
                })?;
                let (theirs_tmp, theirs_path) = write_data(theirs).map_err(|err| Error::CreateTempfile {
                    rela_path: self.other.rela_path.into(),
                    kind: ResourceKind::OtherOrTheirs,
                    source: err,
                })?;

                let mut cmd = BString::from(Vec::with_capacity(merge_command.len()));
                let mut count = 0;
                for token in merge_command.split(|b| *b == b'%') {
                    count += 1;
                    let token = if count > 1 {
                        match token.first() {
                            Some(&b'O') => {
                                cmd.push_str(gix_path::into_bstr(&base_path).as_ref());
                                &token[1..]
                            }
                            Some(&b'A') => {
                                cmd.push_str(gix_path::into_bstr(&ours_path).as_ref());
                                &token[1..]
                            }
                            Some(&b'B') => {
                                cmd.push_str(gix_path::into_bstr(&theirs_path).as_ref());
                                &token[1..]
                            }
                            Some(&b'L') => {
                                let marker_size = self
                                    .options
                                    .text
                                    .conflict
                                    .marker_size()
                                    .unwrap_or(Conflict::DEFAULT_MARKER_SIZE);
                                cmd.push_str(format!("{marker_size}"));
                                &token[1..]
                            }
                            Some(&b'P') => {
                                cmd.push_str(gix_quote::single(self.current.rela_path));
                                &token[1..]
                            }
                            Some(&b'S') => {
                                cmd.push_str(gix_quote::single(ancestor.unwrap_or_default()));
                                &token[1..]
                            }
                            Some(&b'X') => {
                                cmd.push_str(gix_quote::single(current.unwrap_or_default()));
                                &token[1..]
                            }
                            Some(&b'Y') => {
                                cmd.push_str(gix_quote::single(other.unwrap_or_default()));
                                &token[1..]
                            }
                            Some(_other) => {
                                cmd.push(b'%');
                                token
                            }
                            None => b"%",
                        }
                    } else {
                        token
                    };
                    cmd.extend_from_slice(token);
                }

                Ok(merge::Command {
                    cmd: gix_command::prepare(gix_path::from_bstring(cmd))
                        .with_context(context)
                        .with_shell()
                        .stdin(Stdio::null())
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .into(),
                    current: ours_tmp,
                    current_path: ours_path,
                    ancestor: base_tmp,
                    other: theirs_tmp,
                })
            }

            /// Return the configured driver program for use with [`Self::prepare_external_driver()`], or `Err`
            /// with the built-in driver to use instead.
            pub fn configured_driver(&self) -> Result<&'parent Driver, BuiltinDriver> {
                match self.driver {
                    DriverChoice::BuiltIn(builtin) => Err(builtin),
                    DriverChoice::Index(idx) => self.parent.drivers.get(idx).ok_or(BuiltinDriver::default()),
                }
            }
        }

        impl std::fmt::Debug for merge::Command {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.cmd.fmt(f)
            }
        }

        impl Deref for merge::Command {
            type Target = std::process::Command;

            fn deref(&self) -> &Self::Target {
                &self.cmd
            }
        }

        impl DerefMut for merge::Command {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.cmd
            }
        }

        impl merge::Command {
            /// Open the file which should have been written to the location of `ours`, to yield the result of the merge operation.
            /// Calling this makes sense only after the merge command has finished successfully.
            pub fn open_result_file(&self) -> std::io::Result<std::fs::File> {
                std::fs::File::open(&self.current_path)
            }
        }
    }

    ///
    pub mod builtin_merge {
        use crate::blob::{builtin_driver, BuiltinDriver, PlatformRef, Resolution};

        /// An identifier to tell us how a merge conflict was resolved by [builtin_merge](PlatformRef::builtin_merge).
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub enum Pick {
            /// In a binary merge, chose the ancestor.
            ///
            /// Use [`PlatformRef::buffer_by_pick()`] to retrieve it.
            Ancestor,
            /// In a binary merge, chose our side.
            ///
            /// Use [`PlatformRef::buffer_by_pick()`] to retrieve it.
            Ours,
            /// In a binary merge, chose their side.
            ///
            /// Use [`PlatformRef::buffer_by_pick()`] to retrieve it.
            Theirs,
            /// New data was produced with the result of the merge, to be found in the buffer that was passed to
            /// [builtin_merge()](PlatformRef::builtin_merge).
            /// This happens for any merge that isn't a binary merge.
            Buffer,
        }

        /// Plumbing
        impl<'parent> PlatformRef<'parent> {
            /// Perform the merge using the given `driver`, possibly placing the output in `out`.
            /// `input` can be used to keep tokens between runs, but note it will only grow in size unless cleared manually.
            /// Use `labels` to annotate conflict sections in case of a text-merge.
            /// Returns `None` if one of the buffers is too large, making a merge impossible.
            /// Note that if the *pick* wasn't [`Pick::Buffer`], then `out` will not have been cleared,
            /// and one has to take the data from the respective resource.
            pub fn builtin_merge(
                &self,
                driver: BuiltinDriver,
                out: &mut Vec<u8>,
                input: &mut imara_diff::intern::InternedInput<&'parent [u8]>,
                labels: builtin_driver::text::Labels<'_>,
            ) -> Option<(Pick, Resolution)> {
                let base = self.ancestor.data.as_slice()?;
                let ours = self.current.data.as_slice()?;
                let theirs = self.other.data.as_slice()?;
                let driver = if driver != BuiltinDriver::Binary
                    && (is_binary_buf(ours) || is_binary_buf(theirs) || is_binary_buf(base))
                {
                    BuiltinDriver::Binary
                } else {
                    driver
                };
                Some(match driver {
                    BuiltinDriver::Text => {
                        let resolution =
                            builtin_driver::text(out, input, labels, ours, base, theirs, self.options.text);
                        (Pick::Buffer, resolution)
                    }
                    BuiltinDriver::Binary => {
                        let (pick, resolution) = builtin_driver::binary(self.options.resolve_binary_with);
                        let pick = match pick {
                            builtin_driver::binary::Pick::Ours => Pick::Ours,
                            builtin_driver::binary::Pick::Theirs => Pick::Theirs,
                            builtin_driver::binary::Pick::Ancestor => Pick::Ancestor,
                        };
                        (pick, resolution)
                    }
                    BuiltinDriver::Union => {
                        let resolution = builtin_driver::text(
                            out,
                            input,
                            labels,
                            ours,
                            base,
                            theirs,
                            builtin_driver::text::Options {
                                conflict: builtin_driver::text::Conflict::ResolveWithUnion,
                                ..self.options.text
                            },
                        );
                        (Pick::Buffer, resolution)
                    }
                })
            }
        }

        fn is_binary_buf(buf: &[u8]) -> bool {
            let buf = &buf[..buf.len().min(8000)];
            buf.contains(&0)
        }
    }
}

/// Convenience
impl<'parent> PlatformRef<'parent> {
    /// Perform the merge, possibly invoking an external merge command, and store the result in `out`, returning `(pick, resolution)`.
    /// Note that `pick` indicates which resource the buffer should be taken from, unless it's [`Pick::Buffer`](inner::builtin_merge::Pick::Buffer)
    /// to indicate it's `out`.
    /// Use `labels` to annotate conflict sections in case of a text-merge.
    /// The merge is configured by `opts` and possible merge driver command executions are affected by `context`.
    ///
    /// Note that at this stage, none-existing input data will simply default to an empty buffer when running the actual merge algorithm.
    /// Too-large resources will result in an error.
    ///
    /// Generally, it is assumed that standard logic, like deletions of files, is handled before any of this is called, so we are lenient
    /// in terms of buffer handling to make it more useful in the face of missing local files.
    pub fn merge(
        &self,
        out: &mut Vec<u8>,
        labels: builtin_driver::text::Labels<'_>,
        context: &gix_command::Context,
    ) -> Result<(inner::builtin_merge::Pick, Resolution), Error> {
        let _span = gix_trace::coarse!(
            "gix_merge::blob::PlatformRef::merge()",
            current_rela_path = %self.current.rela_path
        );
        match self.configured_driver() {
            Ok(driver) => {
                let mut cmd = self.prepare_external_driver(driver.command.clone(), labels, context.clone())?;
                let status = cmd.status().map_err(|err| Error::SpawnExternalDriver {
                    cmd: format!("{:?}", cmd.cmd),
                    source: err,
                })?;
                if !status.success() {
                    return Err(Error::ExternalDriverFailure {
                        cmd: format!("{:?}", cmd.cmd),
                        status,
                    });
                }
                out.clear();
                cmd.open_result_file()?.read_to_end(out)?;
                Ok((inner::builtin_merge::Pick::Buffer, Resolution::Complete))
            }
            Err(builtin) => {
                let mut input = imara_diff::intern::InternedInput::new(&[][..], &[]);
                out.clear();
                let (pick, resolution) = self
                    .builtin_merge(builtin, out, &mut input, labels)
                    .ok_or(Error::ResourceTooLarge)?;
                Ok((pick, resolution))
            }
        }
    }

    /// Using a `pick` obtained from [`merge()`](Self::merge), obtain the respective buffer suitable for reading or copying.
    /// Return `None` if the buffer is too large, or if the `pick` corresponds to a buffer (that was written separately).
    pub fn buffer_by_pick(&self, pick: inner::builtin_merge::Pick) -> Option<&'parent [u8]> {
        match pick {
            inner::builtin_merge::Pick::Ancestor => self.ancestor.data.as_slice(),
            inner::builtin_merge::Pick::Ours => self.current.data.as_slice(),
            inner::builtin_merge::Pick::Theirs => self.other.data.as_slice(),
            inner::builtin_merge::Pick::Buffer => None,
        }
    }
}
