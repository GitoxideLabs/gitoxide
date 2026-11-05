use gix_error::{Result, bail};
use std::{io::Read, path::PathBuf};

use crate::blob::{PlatformRef, Resolution, builtin_driver, platform::resource::Data};

/// Options for the use in the [`PlatformRef::merge()`] call.
#[derive(Default, Copy, Clone, Debug, Eq, PartialEq)]
pub struct Options {
    /// If `true`, the resources being merged are contained in a virtual ancestor,
    /// which is the case when merge bases are merged into one.
    /// This flag affects the choice of merge drivers.
    pub is_virtual_ancestor: bool,
    /// Determine how to resolve conflicts. If `None`, no conflict resolution is possible, and it picks a side.
    pub resolve_binary_with: Option<builtin_driver::binary::ResolveWith>,
    /// Options for the builtin [text driver](crate::blob::BuiltinDriver::Text).
    pub text: builtin_driver::text::Options,
}

/// The product of a [`PlatformRef::prepare_external_driver()`] operation.
///
/// This type allows to creation of [`std::process::Command`], ready to run, with `stderr` and `stdout` set to *inherit*,
/// but `stdin` closed.
/// It's expected to leave its result in the file substituted at `current` which is then supposed to be read back from there.
// TODO: remove dead-code annotation
#[expect(
    dead_code,
    reason = "the tempfile handles are retained for RAII cleanup while the external merge command runs"
)]
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
        use gix_error::Result;
        use std::{
            io::Write,
            ops::{Deref, DerefMut},
            path::{Path, PathBuf},
            process::Stdio,
        };

        use bstr::{BString, ByteVec};
        use gix_tempfile::{AutoRemove, ContainingDirectory};

        use crate::blob::{
            BuiltinDriver, Driver, PlatformRef, ResourceKind, builtin_driver,
            builtin_driver::text::Conflict,
            platform::{DriverChoice, merge},
        };

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
            /// Missing inputs are passed as empty temporary files; too-large inputs are rejected.
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
            ) -> Result<merge::Command> {
                use gix_error::{OptionExt, ResultExt, message, validation};

                fn write_data(
                    data: &[u8],
                    directory: &Path,
                ) -> std::io::Result<(gix_tempfile::Handle<gix_tempfile::handle::Closed>, PathBuf)> {
                    let mut file = gix_tempfile::new(directory, ContainingDirectory::Exists, AutoRemove::Tempfile)?;
                    file.write_all(data)?;
                    let mut path = Default::default();
                    file.with_mut(|f| {
                        f.path().clone_into(&mut path);
                    })?;
                    let file = file.close()?;
                    Ok((file, path))
                }

                let base = merge::content_merge_input(self.ancestor.data).ok_or_raise(|| {
                    validation(format!(
                        "The resource of kind {:?} was too large to be processed",
                        ResourceKind::CommonAncestorOrBase
                    ))
                })?;
                let ours = merge::content_merge_input(self.current.data).ok_or_raise(|| {
                    validation(format!(
                        "The resource of kind {:?} was too large to be processed",
                        ResourceKind::CurrentOrOurs
                    ))
                })?;
                let theirs = merge::content_merge_input(self.other.data).ok_or_raise(|| {
                    validation(format!(
                        "The resource of kind {:?} was too large to be processed",
                        ResourceKind::OtherOrTheirs
                    ))
                })?;

                let tmp_dir = context
                    .worktree_dir
                    .as_deref()
                    .or(context.git_dir.as_deref())
                    .unwrap_or(Path::new(""));
                let (base_tmp, base_path) = write_data(base, tmp_dir).or_raise(|| {
                    message!(
                        "Tempfile to store content of '{}' ({:?}) for passing to external merge command could not be created",
                        self.ancestor.rela_path,
                        ResourceKind::CommonAncestorOrBase
                    )
                })?;
                let (ours_tmp, ours_path) = write_data(ours, tmp_dir).or_raise(|| {
                    message!(
                        "Tempfile to store content of '{}' ({:?}) for passing to external merge command could not be created",
                        self.current.rela_path,
                        ResourceKind::CurrentOrOurs
                    )
                })?;
                let (theirs_tmp, theirs_path) = write_data(theirs, tmp_dir).or_raise(|| {
                    message!(
                        "Tempfile to store content of '{}' ({:?}) for passing to external merge command could not be created",
                        self.other.rela_path,
                        ResourceKind::OtherOrTheirs
                    )
                })?;

                let mut cmd = BString::from(Vec::with_capacity(merge_command.len()));
                let mut count = 0;
                for token in merge_command.split(|b| *b == b'%') {
                    count += 1;
                    let token = if count > 1 {
                        match token.first() {
                            Some(&b'O') => {
                                cmd.push_str(gix_path::into_bstr(&base_path)?.as_ref());
                                &token[1..]
                            }
                            Some(&b'A') => {
                                cmd.push_str(gix_path::into_bstr(&ours_path)?.as_ref());
                                &token[1..]
                            }
                            Some(&b'B') => {
                                cmd.push_str(gix_path::into_bstr(&theirs_path)?.as_ref());
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
                    cmd: gix_command::prepare(gix_path::from_bstring(cmd)?)
                        .with_context(context)
                        .command_may_be_shell_script()
                        .stdin(Stdio::null())
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .try_into()?,
                    current: ours_tmp,
                    current_path: ours_path,
                    ancestor: base_tmp,
                    other: theirs_tmp,
                })
            }

            /// Return the configured driver program for use with [`Self::prepare_external_driver()`], or `Err`
            /// with the built-in driver to use instead.
            pub fn configured_driver(&self) -> std::result::Result<&'parent Driver, BuiltinDriver> {
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
        use crate::blob::{
            BuiltinDriver, PlatformRef, Resolution, builtin_driver,
            platform::{merge::content_merge_input, resource::Data},
        };

        /// An identifier to tell us how a merge conflict was resolved by [builtin_merge](PlatformRef::builtin_merge).
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub enum Pick {
            /// In a binary merge, chose the ancestor.
            ///
            /// Use [`PlatformRef::data_by_pick()`] to retrieve it.
            Ancestor,
            /// In a binary merge, chose our side.
            ///
            /// Use [`PlatformRef::data_by_pick()`] to retrieve it.
            Ours,
            /// In a binary merge, chose their side.
            ///
            /// Use [`PlatformRef::data_by_pick()`] to retrieve it.
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
            /// Text and union drivers interpret missing inputs as empty content.
            /// Note that if the *pick* wasn't [`Pick::Buffer`], then `out` will not have been cleared,
            /// and one has to take the data from the respective resource.
            ///
            /// If there is no buffer loaded as the resource is too big, we will automatically perform a binary merge
            /// which effectively chooses our side by default.
            pub fn builtin_merge(
                &self,
                driver: BuiltinDriver,
                out: &mut Vec<u8>,
                input: &mut imara_diff::InternedInput<&'parent [u8]>,
                labels: builtin_driver::text::Labels<'_>,
            ) -> (Pick, Resolution) {
                let driver = if driver != BuiltinDriver::Binary
                    && (is_binary_buf(self.ancestor.data)
                        || is_binary_buf(self.other.data)
                        || is_binary_buf(self.current.data))
                {
                    BuiltinDriver::Binary
                } else {
                    driver
                };
                match driver {
                    BuiltinDriver::Text | BuiltinDriver::Union => {
                        let [base, ours, theirs] = [self.ancestor.data, self.current.data, self.other.data]
                            .map(|data| content_merge_input(data).expect("too-large resources use the binary driver"));
                        let options = if driver == BuiltinDriver::Union {
                            builtin_driver::text::Options {
                                conflict: builtin_driver::text::Conflict::ResolveWithUnion,
                                ..self.options.text
                            }
                        } else {
                            self.options.text
                        };
                        let resolution = builtin_driver::text(out, input, labels, ours, base, theirs, options);
                        (Pick::Buffer, resolution)
                    }
                    BuiltinDriver::Binary => {
                        let ours_matches_theirs = match (self.current.data, self.other.data) {
                            (Data::Missing, Data::Missing) => true,
                            (Data::Missing, _) | (_, Data::Missing) => false,
                            _ if !self.current.id.is_null() && !self.other.id.is_null() => {
                                self.current.id == self.other.id
                            }
                            (Data::Buffer(ours), Data::Buffer(theirs)) => ours == theirs,
                            _ => false,
                        };

                        if ours_matches_theirs {
                            (Pick::Ours, Resolution::Complete)
                        } else {
                            let (pick, resolution) = builtin_driver::binary(self.options.resolve_binary_with);
                            let pick = match pick {
                                builtin_driver::binary::Pick::Ours => Pick::Ours,
                                builtin_driver::binary::Pick::Theirs => Pick::Theirs,
                                builtin_driver::binary::Pick::Ancestor => Pick::Ancestor,
                            };
                            (pick, resolution)
                        }
                    }
                }
            }
        }

        fn is_binary_buf(data: Data<'_>) -> bool {
            match data {
                Data::Missing => false,
                Data::Buffer(buf) => {
                    let buf = &buf[..buf.len().min(8000)];
                    buf.contains(&0)
                }
                Data::TooLarge { .. } => true,
            }
        }
    }
}

/// Convenience
impl PlatformRef<'_> {
    /// Perform the merge, possibly invoking an external merge command, and store the result in `out`, returning `(pick, resolution)`.
    /// Note that `pick` indicates which resource the buffer should be taken from, unless it's [`Pick::Buffer`](inner::builtin_merge::Pick::Buffer)
    /// to indicate it's `out`.
    /// Use `labels` to annotate conflict sections in case of a text-merge.
    /// The merge is configured by `opts` and possible merge driver command executions are affected by `context`.
    ///
    /// Text, union, and external drivers interpret missing inputs as empty content. Their output buffers cannot
    /// distinguish a file deletion from empty content; callers must handle deletions before content merging.
    /// Binary merges retain the distinction when selecting a resource.
    /// External drivers reject too-large resources; built-in drivers fall back to binary merging.
    ///
    /// Like Git, an external merge driver exiting with a status from 1 to 128 has left conflicts in its result,
    /// which is then returned with [`Resolution::Conflict`].
    /// Builtin conflict-resolution options do not override the external driver's output or conflict status,
    /// as retrying with a builtin driver could discard edits already merged by the external driver.
    ///
    /// External merge driver launch errors provide [metadata](gix_error::Error::metadata) with `program` (Path),
    /// the invoked program name or path, and `command` (String), the prepared command's debug representation,
    /// and preserve the underlying IO error as a cause. Failed exits, i.e. those with a status above 128 or by
    /// signal, additionally provide `exit_status` (String), its display representation, and `exit_code` (I64)
    /// when available. Output streams are not captured.
    ///
    /// Generally, it is assumed that standard logic, like deletions of files, is handled before any of this is called, so we are lenient
    /// in terms of buffer handling to make it more useful in the face of missing local files.
    pub fn merge(
        &self,
        out: &mut Vec<u8>,
        labels: builtin_driver::text::Labels<'_>,
        context: &gix_command::Context,
    ) -> Result<(inner::builtin_merge::Pick, Resolution)> {
        use gix_error::{ResultExt, message};

        match self.configured_driver() {
            Ok(driver) => {
                let mut cmd = self
                    .prepare_external_driver(driver.command.clone(), labels, context.clone())
                    .or_raise(|| message("Failed to prepare external merge driver"))?;
                let status = cmd
                    .status()
                    .or_raise(|| message("Failed to launch external merge driver").with_command(&cmd))?;
                let resolution = match status.code() {
                    Some(0) => Resolution::Complete,
                    Some(1..=128) => Resolution::Conflict,
                    _ => {
                        let failure = message("External merge driver failed")
                            .with_command(&cmd)
                            .with_exit_status(status);
                        bail!(failure);
                    }
                };
                out.clear();
                cmd.open_result_file()
                    .or_raise(|| message("IO failed when dealing with merge-driver output"))?
                    .read_to_end(out)
                    .or_raise(|| message("IO failed when dealing with merge-driver output"))?;
                Ok((inner::builtin_merge::Pick::Buffer, resolution))
            }
            Err(builtin) => {
                let mut input = imara_diff::InternedInput::new(&[][..], &[]);
                out.clear();
                let (pick, resolution) = self.builtin_merge(builtin, out, &mut input, labels);
                Ok((pick, resolution))
            }
        }
    }

    /// Return the data selected by `pick`, preserving missing, present, and too-large resource states.
    /// Pass the output buffer supplied to [`merge()`](Self::merge) or [`builtin_merge()`](Self::builtin_merge)
    /// as `merged`.
    ///
    /// [`Pick::Buffer`](inner::builtin_merge::Pick::Buffer) returns [`Data::Buffer`] containing `merged`, even
    /// if it is empty or all inputs were missing. Other picks return the selected resource's original data,
    /// including [`Data::Missing`] regardless of any supplied ID.
    pub fn data_by_pick<'a>(&'a self, pick: inner::builtin_merge::Pick, merged: &'a [u8]) -> Data<'a> {
        match pick {
            inner::builtin_merge::Pick::Ancestor => self.ancestor.data,
            inner::builtin_merge::Pick::Ours => self.current.data,
            inner::builtin_merge::Pick::Theirs => self.other.data,
            inner::builtin_merge::Pick::Buffer => Data::Buffer(merged),
        }
    }

    /// Use `pick` to return the object id of the merged result, assuming that `buf` was passed as `out` to [merge()](Self::merge).
    /// In case of binary or large files, this will simply be the existing ID of the resource.
    /// In case of resources available in the object DB for binary merges, the object ID will be returned.
    /// If new content was produced due to a content merge, `buf` will be written out
    /// to the object database using `write_blob`.
    /// For [`Pick::Buffer`](inner::builtin_merge::Pick::Buffer), even an empty `buf` is written as a blob;
    /// missing inputs do not make a content-merge result a deletion.
    /// Beware that the returned ID could be `Ok(None)` if the underlying resource was loaded
    /// from the worktree *and* was too large so it was never loaded from disk.
    /// `Ok(None)` will also be returned if the selected resource was missing, even if an ID was supplied.
    /// `write_blob()` is used to store buffers that have no existing ID.
    pub fn id_by_pick<E>(
        &self,
        pick: inner::builtin_merge::Pick,
        buf: &[u8],
        mut write_blob: impl FnMut(&[u8]) -> std::result::Result<gix_hash::ObjectId, E>,
    ) -> std::result::Result<Option<gix_hash::ObjectId>, E> {
        let field = match pick {
            inner::builtin_merge::Pick::Ancestor => &self.ancestor,
            inner::builtin_merge::Pick::Ours => &self.current,
            inner::builtin_merge::Pick::Theirs => &self.other,
            inner::builtin_merge::Pick::Buffer => return write_blob(buf).map(Some),
        };
        use crate::blob::platform::resource::Data;
        match field.data {
            Data::TooLarge { .. } if !field.id.is_null() => Ok(Some(field.id.to_owned())),
            Data::TooLarge { .. } | Data::Missing => Ok(None),
            Data::Buffer(buf) if field.id.is_null() => write_blob(buf).map(Some),
            Data::Buffer(_) => Ok(Some(field.id.to_owned())),
        }
    }
}

/// Content merge protocols represent absent inputs as empty files, but cannot process unread content.
fn content_merge_input(data: Data<'_>) -> Option<&[u8]> {
    match data {
        Data::Missing => Some(&[]),
        Data::Buffer(buf) => Some(buf),
        Data::TooLarge { .. } => None,
    }
}