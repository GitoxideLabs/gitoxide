use bstr::{BStr, BString, ByteSlice};
use std::cmp::Ordering;
use std::{io::Write, process::Stdio};

use super::Algorithm;
use crate::blob::{pipeline, Pipeline, Platform, ResourceKind};

/// A key to uniquely identify either a location in the worktree, or in the object database.
#[derive(Clone)]
pub(crate) struct CacheKey {
    id: gix_hash::ObjectId,
    location: BString,
    /// If `true`, this is an `id` based key, otherwise it's location based.
    use_id: bool,
    /// Only relevant when `id` is not null, to further differentiate content and allow us to
    /// keep track of both links and blobs with the same content (rare, but possible).
    is_link: bool,
}

/// A stored value representing a diffable resource.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub(crate) struct CacheValue {
    /// The outcome of converting a resource into a diffable format using [Pipeline::convert_to_diffable()].
    conversion: pipeline::Outcome,
    /// The kind of the resource we are looking at. Only possible values are `Blob`, `BlobExecutable` and `Link`.
    mode: gix_object::tree::EntryKind,
    /// A possibly empty buffer, depending on `conversion.data` which may indicate the data is considered binary.
    buffer: Vec<u8>,
}

impl std::hash::Hash for CacheKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        if self.use_id {
            self.id.hash(state);
            self.is_link.hash(state);
        } else {
            self.location.hash(state);
        }
    }
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        match (self.use_id, other.use_id) {
            (false, false) => self.location.eq(&other.location),
            (true, true) => self.id.eq(&other.id) && self.is_link.eq(&other.is_link),
            _ => false,
        }
    }
}

impl Eq for CacheKey {}

impl Default for CacheKey {
    fn default() -> Self {
        CacheKey {
            id: gix_hash::Kind::Sha1.null(),
            use_id: false,
            is_link: false,
            location: BString::default(),
        }
    }
}

impl CacheKey {
    fn set_location(&mut self, rela_path: &BStr) {
        self.location.clear();
        self.location.extend_from_slice(rela_path);
    }
}

/// A resource ready to be diffed in one way or another.
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Resource<'a> {
    /// If available, an index into the `drivers` field to access more diff-related information of the driver for items
    /// at the given path, as previously determined by git-attributes.
    ///
    /// Note that drivers are queried even if there is no object available.
    pub driver_index: Option<usize>,
    /// The data itself, suitable for diffing, and if the object or worktree item is present at all.
    pub data: resource::Data<'a>,
    /// The kind of the resource we are looking at. Only possible values are `Blob`, `BlobExecutable` and `Link`.
    pub mode: gix_object::tree::EntryKind,
    /// The location of the resource, relative to the working tree.
    pub rela_path: &'a BStr,
    /// The id of the content as it would be stored in `git`, or `null` if the content doesn't exist anymore at
    /// `rela_path` or if it was never computed. This can happen with content read from the worktree, which has to
    /// go through a filter to be converted back to what `git` would store.
    pub id: &'a gix_hash::oid,
}

///
pub mod resource {
    use crate::blob::{
        pipeline,
        platform::{CacheKey, CacheValue, Resource},
    };

    impl<'a> Resource<'a> {
        pub(crate) fn new(key: &'a CacheKey, value: &'a CacheValue) -> Self {
            Resource {
                driver_index: value.conversion.driver_index,
                data: value.conversion.data.map_or(Data::Missing, |data| match data {
                    pipeline::Data::Buffer => Data::Buffer(&value.buffer),
                    pipeline::Data::Binary { size } => Data::Binary { size },
                }),
                mode: value.mode,
                rela_path: key.location.as_ref(),
                id: &key.id,
            }
        }

        /// Produce an iterator over lines, separated by LF or CRLF and thus keeping newlines.
        ///
        /// Note that this will cause unusual diffs if a file didn't end in newline but lines were added
        /// on the other side.
        ///
        /// Suitable to create tokens using [`imara_diff::intern::InternedInput`].
        pub fn intern_source(&self) -> imara_diff::sources::ByteLines<'a, true> {
            crate::blob::sources::byte_lines_with_terminator(self.data.as_slice().unwrap_or_default())
        }

        /// Produce an iterator over lines, but remove LF or CRLF.
        ///
        /// This produces the expected diffs when lines were added at the end of a file that didn't end
        /// with a newline before the change.
        ///
        /// Suitable to create tokens using [`imara_diff::intern::InternedInput`].
        pub fn intern_source_strip_newline_separators(&self) -> imara_diff::sources::ByteLines<'a, false> {
            crate::blob::sources::byte_lines(self.data.as_slice().unwrap_or_default())
        }
    }

    /// The data of a diffable resource, as it could be determined and computed previously.
    #[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub enum Data<'a> {
        /// The object is missing, either because it didn't exist in the working tree or because its `id` was null.
        Missing,
        /// The textual data as processed to be in a diffable state.
        Buffer(&'a [u8]),
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

    impl<'a> Data<'a> {
        /// Return ourselves as slice of bytes if this instance stores data.
        pub fn as_slice(&self) -> Option<&'a [u8]> {
            match self {
                Data::Buffer(d) => Some(d),
                Data::Binary { .. } | Data::Missing => None,
            }
        }
    }
}

///
pub mod set_resource {
    use bstr::BString;

    use crate::blob::{pipeline, ResourceKind};

    /// The error returned by [Platform::set_resource](super::Platform::set_resource).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Can only diff blobs and links, not {mode:?}")]
        InvalidMode { mode: gix_object::tree::EntryKind },
        #[error("Failed to read {kind} worktree data from '{rela_path}'")]
        Io {
            rela_path: BString,
            kind: ResourceKind,
            source: std::io::Error,
        },
        #[error("Failed to obtain attributes for {kind} resource at '{rela_path}'")]
        Attributes {
            rela_path: BString,
            kind: ResourceKind,
            source: std::io::Error,
        },
        #[error(transparent)]
        ConvertToDiffable(#[from] pipeline::convert_to_diffable::Error),
    }
}

///
pub mod prepare_diff {
    use bstr::BStr;

    use crate::blob::platform::Resource;

    /// The kind of operation that should be performed based on the configuration of the resources involved in the diff.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub enum Operation<'a> {
        /// The [internal diff algorithm](imara_diff::diff) should be called with the provided arguments.
        /// This only happens if none of the resources are binary, and if there is no external diff program configured via git-attributes
        /// *or* [Options::skip_internal_diff_if_external_is_configured](super::Options::skip_internal_diff_if_external_is_configured)
        /// is `false`.
        ///
        /// Use [`Outcome::interned_input()`] to easily obtain an interner for use with [`imara_diff::diff()`], or maintain one yourself
        /// for greater reuse.
        InternalDiff {
            /// The algorithm we determined should be used, which is one of (in order, first set one wins):
            ///
            /// * the driver's override
            /// * the platforms own configuration (typically from git-config)
            /// * the default algorithm
            algorithm: imara_diff::Algorithm,
        },
        /// Run the external diff program according as configured in the `source`-resources driver.
        /// This only happens if [Options::skip_internal_diff_if_external_is_configured](super::Options::skip_internal_diff_if_external_is_configured)
        /// was `true`, preventing the usage of the internal diff implementation.
        ExternalCommand {
            /// The command as extracted from [Driver::command](super::super::Driver::command).
            /// Use it in [`Platform::prepare_diff_command`](super::Platform::prepare_diff_command()) to easily prepare a compatible invocation.
            command: &'a BStr,
        },
        /// One of the involved resources, [`old`](Outcome::old) or [`new`](Outcome::new), were binary and thus no diff
        /// cannot be performed.
        SourceOrDestinationIsBinary,
    }

    /// The outcome of a [`prepare_diff`](super::Platform::prepare_diff()) operation.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct Outcome<'a> {
        /// The kind of diff that was actually performed. This may include skipping the internal diff as well.
        pub operation: Operation<'a>,
        /// The old or source of the diff operation.
        pub old: Resource<'a>,
        /// The new or destination of the diff operation.
        pub new: Resource<'a>,
    }

    impl<'a> Outcome<'a> {
        /// Produce an instance of an interner which `git` would use to perform diffs.
        ///
        /// Note that newline separators will be removed to improve diff quality
        /// at the end of files that didn't have a newline, but had lines added
        /// past the end.
        pub fn interned_input(&self) -> imara_diff::intern::InternedInput<&'a [u8]> {
            crate::blob::intern::InternedInput::new(
                self.old.intern_source_strip_newline_separators(),
                self.new.intern_source_strip_newline_separators(),
            )
        }
    }

    /// The error returned by [Platform::prepare_diff()](super::Platform::prepare_diff()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Either the source or the destination of the diff operation were not set")]
        SourceOrDestinationUnset,
        #[error("Tried to diff resources that are both considered removed")]
        SourceAndDestinationRemoved,
    }
}

///
pub mod prepare_diff_command {
    use std::ops::{Deref, DerefMut};

    use bstr::BString;

    /// The error returned by [Platform::prepare_diff_command()](super::Platform::prepare_diff_command()).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("Either the source or the destination of the diff operation were not set")]
        SourceOrDestinationUnset,
        #[error("Binary resources can't be diffed with an external command (as we don't have the data anymore)")]
        SourceOrDestinationBinary,
        #[error(
            "Tempfile to store content of '{rela_path}' for passing to external diff command could not be created"
        )]
        CreateTempfile { rela_path: BString, source: std::io::Error },
        #[error("Could not write content of '{rela_path}' to tempfile for passing to external diff command")]
        WriteTempfile { rela_path: BString, source: std::io::Error },
    }

    /// The outcome of a [`prepare_diff_command`](super::Platform::prepare_diff_command()) operation.
    ///
    /// This type acts like [`std::process::Command`], ready to run, with `stdin`, `stdout` and `stderr` set to *inherit*
    /// all handles as this is expected to be for visual inspection.
    pub struct Command {
        pub(crate) cmd: std::process::Command,
        /// Possibly a tempfile to be removed after the run, or `None` if there is no old version.
        pub(crate) old: Option<gix_tempfile::Handle<gix_tempfile::handle::Closed>>,
        /// Possibly a tempfile to be removed after the run, or `None` if there is no new version.
        pub(crate) new: Option<gix_tempfile::Handle<gix_tempfile::handle::Closed>>,
    }

    impl Deref for Command {
        type Target = std::process::Command;

        fn deref(&self) -> &Self::Target {
            &self.cmd
        }
    }

    impl DerefMut for Command {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.cmd
        }
    }
}

/// Options for use in [Platform::new()].
#[derive(Default, Copy, Clone)]
pub struct Options {
    /// The algorithm to use when diffing.
    /// If unset, it uses the [default algorithm](Algorithm::default()).
    pub algorithm: Option<Algorithm>,
    /// If `true`, default `false`, then an external `diff` configured using gitattributes and drivers,
    /// will cause the built-in diff [to be skipped](prepare_diff::Operation::ExternalCommand).
    /// Otherwise, the internal diff is called despite the configured external diff, which is
    /// typically what callers expect by default.
    pub skip_internal_diff_if_external_is_configured: bool,
}

/// Lifecycle
impl Platform {
    /// Create a new instance with `options`, and a way to `filter` data from the object database to data that is diff-able.
    /// `filter_mode` decides how to do that specifically.
    /// Use `attr_stack` to access attributes pertaining worktree filters and diff settings.
    pub fn new(
        options: Options,
        filter: Pipeline,
        filter_mode: pipeline::Mode,
        attr_stack: gix_worktree::Stack,
    ) -> Self {
        Platform {
            old: None,
            new: None,
            diff_cache: Default::default(),
            free_list: Vec::with_capacity(2),
            options,
            filter,
            filter_mode,
            attr_stack,
        }
    }
}

/// Conversions
impl Platform {
    /// Store enough information about a resource to eventually diff it, where…
    ///
    /// * `id` is the hash of the resource. If it [is null](gix_hash::ObjectId::is_null()), it should either
    ///   be a resource in the worktree, or it's considered a non-existing, deleted object.
    ///   If an `id` is known, as the hash of the object as (would) be stored in `git`, then it should be provided
    ///   for completeness.
    /// * `mode` is the kind of object (only blobs and links are allowed)
    /// * `rela_path` is the relative path as seen from the (work)tree root.
    /// * `kind` identifies the side of the diff this resource will be used for.
    ///   A diff needs both `OldOrSource` *and* `NewOrDestination`.
    /// * `objects` provides access to the object database in case the resource can't be read from a worktree.
    ///
    /// Note that it's assumed that either `id + mode (` or `rela_path` can serve as unique identifier for the resource,
    /// depending on whether or not a [worktree root](pipeline::WorktreeRoots) is set for the resource of `kind`,
    /// with resources with worktree roots using the `rela_path` as unique identifier.
    ///
    /// ### Important
    ///
    /// If an error occurs, the previous resource of `kind` will be cleared, preventing further diffs
    /// unless another attempt succeeds.
    pub fn set_resource(
        &mut self,
        id: gix_hash::ObjectId,
        mode: gix_object::tree::EntryKind,
        rela_path: &BStr,
        kind: ResourceKind,
        objects: &impl gix_object::FindObjectOrHeader, // TODO: make this `dyn` once https://github.com/rust-lang/rust/issues/65991 is stable, then also make tracker.rs `objects` dyn
    ) -> Result<(), set_resource::Error> {
        let res = self.set_resource_inner(id, mode, rela_path, kind, objects);
        if res.is_err() {
            *match kind {
                ResourceKind::OldOrSource => &mut self.old,
                ResourceKind::NewOrDestination => &mut self.new,
            } = None;
        }
        res
    }

    /// Given `diff_command` and `context`, typically obtained from git-configuration, and the currently set diff-resources,
    /// prepare the invocation and temporary files needed to launch it according to protocol.
    /// `count` / `total` are used for progress indication passed as environment variables `GIT_DIFF_PATH_(COUNTER|TOTAL)`
    /// respectively (0-based), so the first path has `count=0` and `total=1` (assuming there is only one path).
    /// Returns `None` if at least one resource is unset, see [`set_resource()`](Self::set_resource()).
    ///
    /// Please note that this is an expensive operation this will always create up to two temporary files to hold the data
    /// for the old and new resources.
    ///
    /// ### Deviation
    ///
    /// If one of the resources is binary, the operation reports an error as such resources don't make their data available
    /// which is required for the external diff to run.
    // TODO: fix this - the diff shouldn't fail if binary (or large) files are used, just copy them into tempfiles.
    pub fn prepare_diff_command(
        &self,
        diff_command: BString,
        context: gix_command::Context,
        count: usize,
        total: usize,
    ) -> Result<prepare_diff_command::Command, prepare_diff_command::Error> {
        fn add_resource(
            cmd: &mut std::process::Command,
            res: Resource<'_>,
        ) -> Result<Option<gix_tempfile::Handle<gix_tempfile::handle::Closed>>, prepare_diff_command::Error> {
            let tmpfile = match res.data {
                resource::Data::Missing => {
                    cmd.args(["/dev/null", ".", "."]);
                    None
                }
                resource::Data::Buffer(buf) => {
                    let mut tmp = gix_tempfile::new(
                        std::env::temp_dir(),
                        gix_tempfile::ContainingDirectory::Exists,
                        gix_tempfile::AutoRemove::Tempfile,
                    )
                    .map_err(|err| prepare_diff_command::Error::CreateTempfile {
                        rela_path: res.rela_path.to_owned(),
                        source: err,
                    })?;
                    tmp.write_all(buf)
                        .map_err(|err| prepare_diff_command::Error::WriteTempfile {
                            rela_path: res.rela_path.to_owned(),
                            source: err,
                        })?;
                    tmp.with_mut(|f| {
                        cmd.arg(f.path());
                    })
                    .map_err(|err| prepare_diff_command::Error::WriteTempfile {
                        rela_path: res.rela_path.to_owned(),
                        source: err,
                    })?;
                    cmd.arg(res.id.to_string()).arg(res.mode.as_octal_str().to_string());
                    let tmp = tmp.close().map_err(|err| prepare_diff_command::Error::WriteTempfile {
                        rela_path: res.rela_path.to_owned(),
                        source: err,
                    })?;
                    Some(tmp)
                }
                resource::Data::Binary { .. } => return Err(prepare_diff_command::Error::SourceOrDestinationBinary),
            };
            Ok(tmpfile)
        }

        let (old, new) = self
            .resources()
            .ok_or(prepare_diff_command::Error::SourceOrDestinationUnset)?;
        let mut cmd: std::process::Command = gix_command::prepare(gix_path::from_bstring(diff_command))
            .with_context(context)
            .env("GIT_DIFF_PATH_COUNTER", (count + 1).to_string())
            .env("GIT_DIFF_PATH_TOTAL", total.to_string())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .into();

        cmd.arg(gix_path::from_bstr(old.rela_path).into_owned());
        let mut out = prepare_diff_command::Command {
            cmd,
            old: None,
            new: None,
        };

        out.old = add_resource(&mut out.cmd, old)?;
        out.new = add_resource(&mut out.cmd, new)?;

        if old.rela_path != new.rela_path {
            out.cmd.arg(gix_path::from_bstr(new.rela_path).into_owned());
        }

        Ok(out)
    }

    /// Returns the resource of the given kind if it was set.
    pub fn resource(&self, kind: ResourceKind) -> Option<Resource<'_>> {
        let key = match kind {
            ResourceKind::OldOrSource => self.old.as_ref(),
            ResourceKind::NewOrDestination => self.new.as_ref(),
        }?;
        Resource::new(key, self.diff_cache.get(key)?).into()
    }

    /// Obtain the two resources that were previously set as `(OldOrSource, NewOrDestination)`, if both are set and available.
    ///
    /// This is useful if one wishes to manually prepare the diff, maybe for invoking external programs, instead of relying on
    /// [`Self::prepare_diff()`].
    pub fn resources(&self) -> Option<(Resource<'_>, Resource<'_>)> {
        let key = &self.old.as_ref()?;
        let value = self.diff_cache.get(key)?;
        let old = Resource::new(key, value);

        let key = &self.new.as_ref()?;
        let value = self.diff_cache.get(key)?;
        let new = Resource::new(key, value);
        Some((old, new))
    }

    /// Prepare a diff operation on the [previously set](Self::set_resource()) [old](ResourceKind::OldOrSource) and
    /// [new](ResourceKind::NewOrDestination) resources.
    ///
    /// The returned outcome allows to easily perform diff operations, based on the [`prepare_diff::Outcome::operation`] field,
    /// which hints at what should be done.
    pub fn prepare_diff(&mut self) -> Result<prepare_diff::Outcome<'_>, prepare_diff::Error> {
        let old_key = &self.old.as_ref().ok_or(prepare_diff::Error::SourceOrDestinationUnset)?;
        let old = self
            .diff_cache
            .get(old_key)
            .ok_or(prepare_diff::Error::SourceOrDestinationUnset)?;
        let new_key = &self.new.as_ref().ok_or(prepare_diff::Error::SourceOrDestinationUnset)?;
        let new = self
            .diff_cache
            .get(new_key)
            .ok_or(prepare_diff::Error::SourceOrDestinationUnset)?;
        let mut out = prepare_diff::Outcome {
            operation: prepare_diff::Operation::SourceOrDestinationIsBinary,
            old: Resource::new(old_key, old),
            new: Resource::new(new_key, new),
        };

        match (old.conversion.data, new.conversion.data) {
            (None, None) => return Err(prepare_diff::Error::SourceAndDestinationRemoved),
            (Some(pipeline::Data::Binary { .. }), _) | (_, Some(pipeline::Data::Binary { .. })) => return Ok(out),
            _either_missing_or_non_binary => {
                if let Some(command) = old
                    .conversion
                    .driver_index
                    .and_then(|idx| self.filter.drivers[idx].command.as_deref())
                    .filter(|_| self.options.skip_internal_diff_if_external_is_configured)
                {
                    out.operation = prepare_diff::Operation::ExternalCommand {
                        command: command.as_bstr(),
                    };
                    return Ok(out);
                }
            }
        }

        out.operation = prepare_diff::Operation::InternalDiff {
            algorithm: old
                .conversion
                .driver_index
                .and_then(|idx| self.filter.drivers[idx].algorithm)
                .or(self.options.algorithm)
                .unwrap_or_default(),
        };
        Ok(out)
    }

    /// Every call to [set_resource()](Self::set_resource()) will keep the diffable data in memory, and that will never be cleared.
    ///
    /// Use this method to clear the cache, releasing memory. Note that this will also lose all information about resources
    /// which means diffs would fail unless the resources are set again.
    ///
    /// Note that this also has to be called if the same resource is going to be diffed in different states, i.e. using different
    /// `id`s, but the same `rela_path`.
    pub fn clear_resource_cache(&mut self) {
        self.old = None;
        self.new = None;
        self.diff_cache.clear();
        self.free_list.clear();
    }

    /// Every call to [set_resource()](Self::set_resource()) will keep the diffable data in memory, and that will never be cleared.
    ///
    /// Use this method to clear the cache, but keep the previously used buffers around for later re-use.
    ///
    /// If there are more buffers on the free-list than there are stored sources, we half that amount each time this method is called,
    /// or keep as many resources as were previously stored, or 2 buffers, whatever is larger.
    /// If there are fewer buffers in the free-list than are in the resource cache, we will keep as many as needed to match the
    /// number of previously stored resources.
    ///
    /// Returns the number of available buffers.
    pub fn clear_resource_cache_keep_allocation(&mut self) -> usize {
        self.old = None;
        self.new = None;

        let diff_cache = std::mem::take(&mut self.diff_cache);
        match self.free_list.len().cmp(&diff_cache.len()) {
            Ordering::Less => {
                let to_take = diff_cache.len() - self.free_list.len();
                self.free_list
                    .extend(diff_cache.into_values().map(|v| v.buffer).take(to_take));
            }
            Ordering::Equal => {}
            Ordering::Greater => {
                let new_len = (self.free_list.len() / 2).max(diff_cache.len()).max(2);
                self.free_list.truncate(new_len);
            }
        }
        self.free_list.len()
    }
}

impl Platform {
    fn set_resource_inner(
        &mut self,
        id: gix_hash::ObjectId,
        mode: gix_object::tree::EntryKind,
        rela_path: &BStr,
        kind: ResourceKind,
        objects: &impl gix_object::FindObjectOrHeader,
    ) -> Result<(), set_resource::Error> {
        if matches!(
            mode,
            gix_object::tree::EntryKind::Commit | gix_object::tree::EntryKind::Tree
        ) {
            return Err(set_resource::Error::InvalidMode { mode });
        }
        let storage = match kind {
            ResourceKind::OldOrSource => &mut self.old,
            ResourceKind::NewOrDestination => &mut self.new,
        }
        .get_or_insert_with(Default::default);

        storage.id = id;
        storage.set_location(rela_path);
        storage.is_link = matches!(mode, gix_object::tree::EntryKind::Link);
        storage.use_id = self.filter.roots.by_kind(kind).is_none();

        if self.diff_cache.contains_key(storage) {
            return Ok(());
        }
        let entry =
            self.attr_stack
                .at_entry(rela_path, None, objects)
                .map_err(|err| set_resource::Error::Attributes {
                    source: err,
                    kind,
                    rela_path: rela_path.to_owned(),
                })?;
        let mut buf = self.free_list.pop().unwrap_or_default();
        let out = self.filter.convert_to_diffable(
            &id,
            mode,
            rela_path,
            kind,
            &mut |_, out| {
                let _ = entry.matching_attributes(out);
            },
            objects,
            self.filter_mode,
            &mut buf,
        )?;
        let key = storage.clone();
        assert!(
            self.diff_cache
                .insert(
                    key,
                    CacheValue {
                        conversion: out,
                        mode,
                        buffer: buf,
                    },
                )
                .is_none(),
            "The key impl makes clashes impossible with our usage"
        );
        Ok(())
    }
}
