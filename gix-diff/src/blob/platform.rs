use gix_error::Result;
use std::{cmp::Ordering, io::Write, process::Stdio};

use bstr::{BStr, BString, ByteSlice};
use gix_error::{ExnResult, OptionExt, ResultExt, bail, message, validation};

use super::Algorithm;
use crate::blob::{Pipeline, Platform, ResourceKind, pipeline};

/// A key to uniquely identify either a location in the worktree, or in the object database.
#[derive(Clone)]
pub(crate) struct CacheKey {
    id: Option<gix_hash::ObjectId>,
    location: BString,
    kind: CacheKeyKind,
    /// Retained per resource even when its converted content is shared.
    mode: gix_object::tree::EntryKind,
}

/// Distinguish locations without an identity from object IDs without enlarging each cache key.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
enum CacheKeyKind {
    Object,
    Missing,
    OldWorktree,
    NewWorktree,
}

/// A stored value representing a diffable resource.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub(crate) struct CacheValue {
    /// The outcome of converting a resource into a diffable format using [Pipeline::convert_to_diffable()].
    conversion: pipeline::Outcome,
    /// A possibly empty buffer, depending on `conversion.data` which may indicate the data is considered binary.
    buffer: Vec<u8>,
}

impl std::hash::Hash for CacheKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        if self.kind == CacheKeyKind::Object {
            self.id.hash(state);
            (self.mode == gix_object::tree::EntryKind::Link).hash(state);
        } else {
            self.location.hash(state);
        }
    }
}

impl PartialEq for CacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind
            && if self.kind == CacheKeyKind::Object {
                self.id == other.id
                    && (self.mode == gix_object::tree::EntryKind::Link)
                        == (other.mode == gix_object::tree::EntryKind::Link)
            } else {
                self.location == other.location
            }
    }
}

impl Eq for CacheKey {}

impl Default for CacheKey {
    fn default() -> Self {
        CacheKey {
            id: None,
            kind: CacheKeyKind::Missing,
            mode: gix_object::tree::EntryKind::Blob,
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
    /// The supplied Git object ID, or `None` if unavailable. Two unavailable IDs do not establish identity.
    /// Use [`data`](Self::data) to distinguish missing content from existing content without a hash.
    /// A supplied ID does not guarantee that the worktree resource still exists.
    /// Converted diff data can differ from the identified Git object.
    pub id: Option<&'a gix_hash::oid>,
}

///
pub mod resource {
    use bstr::ByteSlice;

    use crate::blob::{
        pipeline,
        platform::{CacheKey, CacheValue, Resource},
    };

    /// A token source that splits bytes into lines while removing trailing newline separators.
    // TODO: use `bstr::Lines` here, but it's not `Copy`
    #[derive(Clone, Copy)]
    pub struct ByteLinesWithoutTerminator<'a>(&'a [u8]);

    impl<'a> ByteLinesWithoutTerminator<'a> {
        /// Create a new instance over `data`.
        pub fn new(data: &'a [u8]) -> Self {
            Self(data)
        }
    }

    impl<'a> Iterator for ByteLinesWithoutTerminator<'a> {
        type Item = &'a [u8];

        #[inline(always)]
        fn next(&mut self) -> Option<Self::Item> {
            let mut l = match self.0.find_byte(b'\n') {
                None if self.0.is_empty() => None,
                None => {
                    let line = self.0;
                    self.0 = b"";
                    Some(line)
                }
                Some(end) => {
                    let line = &self.0[..=end];
                    self.0 = &self.0[end + 1..];
                    Some(line)
                }
            }?;

            if l.last_byte() == Some(b'\n') {
                l = &l[..l.len() - 1];
                if l.last_byte() == Some(b'\r') {
                    l = &l[..l.len() - 1];
                }
            }
            Some(l)
        }
    }

    impl<'a> imara_diff::TokenSource for ByteLinesWithoutTerminator<'a> {
        type Token = &'a [u8];
        type Tokenizer = Self;

        fn tokenize(&self) -> Self::Tokenizer {
            *self
        }

        fn estimate_tokens(&self) -> u32 {
            let len: usize = self.take(20).map(<[u8]>::len).sum();
            (self.0.len() * 20).checked_div(len).unwrap_or(100) as u32
        }
    }

    impl<'a> Resource<'a> {
        pub(crate) fn new(key: &'a CacheKey, value: &'a CacheValue) -> Self {
            Resource {
                driver_index: value.conversion.driver_index,
                data: value.conversion.data.map_or(Data::Missing, |data| match data {
                    pipeline::Data::Buffer { is_derived } => Data::Buffer {
                        buf: &value.buffer,
                        is_derived,
                    },
                    pipeline::Data::Binary { size } => Data::Binary { size },
                }),
                mode: key.mode,
                rela_path: key.location.as_ref(),
                id: key.id.as_deref(),
            }
        }
    }

    /// The data of a diffable resource, as it could be determined and computed previously.
    #[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
    pub enum Data<'a> {
        /// The object is missing, either because it didn't exist in the working tree or because its `id` was `None` and no worktree root was configured.
        Missing,
        /// The textual data as processed to be in a diffable state.
        Buffer {
            /// The buffer bytes.
            buf: &'a [u8],
            /// If `true`, a [binary to text filter](super::super::Driver::binary_to_text_command) was used to obtain the buffer,
            /// making it a derived value.
            ///
            /// Applications should check for this to avoid treating the buffer content as (original) resource content.
            is_derived: bool,
        },
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
                Data::Buffer { buf, .. } => Some(buf),
                Data::Binary { .. } | Data::Missing => None,
            }
        }

        /// Returns `true` if the data in this instance is derived.
        pub fn is_derived(&self) -> bool {
            match self {
                Data::Missing | Data::Binary { .. } => false,
                Data::Buffer { is_derived, .. } => *is_derived,
            }
        }
    }
}

/// Errors from setting a diff resource, including [`Platform::set_resource_by_change()`].
pub mod set_resource {
    use bstr::BString;

    use crate::blob::ResourceKind;

    /// A failure to set up a diff resource. Callee errors are retained as exception causes.
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Error {
        /// Only blobs and symbolic links can be diffed.
        InvalidMode {
            /// The unsupported mode.
            mode: gix_object::tree::EntryKind,
        },
        /// Attributes could not be obtained for the resource.
        Attributes {
            /// The side of the diff.
            kind: ResourceKind,
            /// The byte-oriented path relative to the worktree.
            rela_path: BString,
        },
        /// The resource could not be converted to diffable data.
        ConvertToDiffable {
            /// The side of the diff.
            kind: ResourceKind,
            /// The byte-oriented path relative to the worktree.
            rela_path: BString,
        },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::InvalidMode { mode } => write!(f, "Can only diff blobs and links, not {mode:?}"),
                Self::Attributes { kind, rela_path } => {
                    write!(f, "Failed to obtain attributes for {kind} resource at \"{rela_path}\"")
                }
                Self::ConvertToDiffable { kind, rela_path } => {
                    write!(f, "Failed to convert {kind} resource at '{rela_path}' to diffable data")
                }
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Self::InvalidMode { .. } => Some(const { &gix_error::ClassificationMarker::VALIDATION }),
                Self::Attributes { .. } | Self::ConvertToDiffable { .. } => None,
            }
        }
    }
}

///
pub mod prepare_diff {
    use bstr::BStr;

    use crate::blob::{
        DiffLineStats,
        platform::{Resource, resource::ByteLinesWithoutTerminator},
    };

    /// A present resource with available diffable text, which may be empty or derived by a text conversion.
    /// Obtain this from [`TextChange`]; missing and binary resources cannot be represented by this type.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct TextResource<'a> {
        buf: &'a [u8],
        is_derived: bool,
    }

    impl<'a> TextResource<'a> {
        pub(super) fn new(buf: &'a [u8], is_derived: bool) -> Self {
            Self { buf, is_derived }
        }

        /// Return the available text bytes. An empty slice still represents a present resource.
        pub fn as_slice(&self) -> &'a [u8] {
            self.buf
        }

        /// Return whether these bytes were produced by a binary-to-text conversion.
        pub fn is_derived(&self) -> bool {
            self.is_derived
        }
    }

    /// Two present resources with available text, suitable for similarity comparisons.
    /// Obtain this from [`TextChange::Modification`]; absent or binary sides cannot form a pair.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct TextPair<'a> {
        old: TextResource<'a>,
        new: TextResource<'a>,
    }

    impl<'a> TextPair<'a> {
        pub(super) fn from_text(old: TextResource<'a>, new: TextResource<'a>) -> Self {
            Self { old, new }
        }

        /// Return the present source text.
        pub fn before(&self) -> TextResource<'a> {
            self.old
        }

        /// Return the present destination text.
        pub fn after(&self) -> TextResource<'a> {
            self.new
        }

        /// Compare the matching bytes of two present resources using `algorithm`.
        /// Unlike patch tokenization, line terminators participate in equality and byte counts.
        /// Two empty resources have similarity `1.0`; only one empty resource has similarity `0.0`.
        /// Whether empty resources are eligible for rename tracking is a separate caller policy.
        pub fn similarity(&self, algorithm: crate::blob::Algorithm) -> DiffLineStats {
            let input = crate::blob::InternedInput::new(
                crate::blob::sources::byte_lines(self.old.buf),
                crate::blob::sources::byte_lines(self.new.buf),
            );
            let diff = crate::blob::Diff::compute(algorithm, &input);
            let removed_bytes: usize = diff
                .hunks()
                .map(|hunk| {
                    input.before[hunk.before.start as usize..hunk.before.end as usize]
                        .iter()
                        .map(|token| input.interner[*token].len())
                        .sum::<usize>()
                })
                .sum();
            let max_len = self.old.buf.len().max(self.new.buf.len());
            DiffLineStats {
                removals: diff.count_removals(),
                insertions: diff.count_additions(),
                before: input.before.len(),
                after: input.after.len(),
                similarity: if max_len == 0 {
                    1.0
                } else {
                    (self.old.buf.len() - removed_bytes) as f32 / max_len as f32
                },
            }
        }
    }

    /// A text change retaining the distinction between absent and present, possibly empty, resources.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub enum TextChange<'a> {
        /// Only the destination exists.
        Addition {
            /// The present destination text.
            new: TextResource<'a>,
        },
        /// Only the source exists.
        Deletion {
            /// The present source text.
            old: TextResource<'a>,
        },
        /// Both resources exist, even when one or both have empty content.
        Modification(TextPair<'a>),
    }

    impl<'a> TextChange<'a> {
        /// Return source and destination bytes for patch generation, explicitly representing an absent side as empty.
        /// Use [`TextChange::Modification`] and [`TextPair::similarity()`] for similarity comparisons instead.
        pub fn patch_data(&self) -> (&'a [u8], &'a [u8]) {
            match self {
                Self::Addition { new } => (&[], new.buf),
                Self::Deletion { old } => (old.buf, &[]),
                Self::Modification(pair) => (pair.old.buf, pair.new.buf),
            }
        }

        /// Tokenize validated text for patch generation, interpreting an absent side as empty.
        /// Line terminators are removed to improve diffs when lines follow a previously unterminated final line.
        pub fn patch_input(&self) -> crate::blob::InternedInput<&'a [u8]> {
            let (old, new) = self.patch_data();
            crate::blob::InternedInput::new(
                ByteLinesWithoutTerminator::new(old),
                ByteLinesWithoutTerminator::new(new),
            )
        }
    }

    /// A failure to prepare a diff from the configured resources.
    #[derive(Debug)]
    #[non_exhaustive]
    pub enum Error {
        /// At least one side has not been set successfully.
        SourceOrDestinationUnset,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(match self {
                Self::SourceOrDestinationUnset => {
                    "Either the source or the destination of the diff operation were not set"
                }
            })
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(const { &gix_error::ClassificationMarker::VALIDATION })
        }
    }

    /// The kind of operation that should be performed based on the configuration of the resources involved in the diff.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub enum Operation<'a> {
        /// The internal diff algorithm should be computed with [`crate::blob::Diff::compute()`].
        /// This only happens if none of the resources are binary, and if there is no external diff program configured via git-attributes
        /// *or* [Options::skip_internal_diff_if_external_is_configured](super::Options::skip_internal_diff_if_external_is_configured)
        /// is `false`.
        ///
        /// Use [`TextChange::patch_input()`] for patch generation, or match [`TextChange::Modification`]
        /// to obtain a pair suitable for similarity checking.
        InternalDiff {
            /// The algorithm we determined should be used, which is one of (in order, first set one wins):
            ///
            /// * the driver's override
            /// * the platforms own configuration (typically from git-config)
            /// * the default algorithm
            algorithm: imara_diff::Algorithm,
            /// The validated text and its presence on each side.
            change: TextChange<'a>,
        },
        /// Run the external diff program according as configured in the `source`-resources driver.
        /// This only happens if [Options::skip_internal_diff_if_external_is_configured](super::Options::skip_internal_diff_if_external_is_configured)
        /// was `true`, preventing the usage of the internal diff implementation.
        ExternalCommand {
            /// The command as extracted from [Driver::command](super::super::Driver::command).
            /// Use it in [`Platform::prepare_diff_command`](super::Platform::prepare_diff_command()) to easily prepare a compatible invocation.
            command: &'a BStr,
            /// Validated text available for optional internal patch statistics.
            change: TextChange<'a>,
        },
        /// Both resources were set successfully, but neither exists. No comparison can be performed.
        NeitherExists,
        /// One of the involved resources, [`old`](Outcome::old) or [`new`](Outcome::new), was binary and thus no diff
        /// can be performed.
        SourceOrDestinationIsBinary,
    }

    /// The outcome of a [`prepare_diff`](super::Platform::prepare_diff()) operation.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub struct Outcome<'a> {
        /// The kind of diff that was actually performed. This may include skipping the internal diff as well.
        pub operation: Operation<'a>,
        /// If `true`, a [binary to text filter](super::super::Driver::binary_to_text_command) was used to obtain the buffer
        /// of `old` or `new`, making it a derived value.
        ///
        /// Applications should check for this to avoid treating the buffer content as (original) resource content.
        pub old_or_new_is_derived: bool,
        /// The old or source of the diff operation.
        pub old: Resource<'a>,
        /// The new or destination of the diff operation.
        pub new: Resource<'a>,
    }
}

///
pub mod prepare_diff_command {
    use std::ops::{Deref, DerefMut};

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
            missing_driver_index: [None; 2],
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
    /// * `id` is the hash of the resource. If it is `None`, it should either
    ///   be a resource in the worktree, or it's considered a non-existing, deleted object.
    ///   If an `id` is known, as the hash of the object as (would) be stored in `git`, then it should be provided
    ///   for completeness.
    ///   Null IDs in `Some` are treated as `None`.
    /// * `mode` is the kind of object (only blobs and links are allowed)
    /// * `rela_path` is the relative path as seen from the (work)tree root.
    /// * `kind` identifies the side of the diff this resource will be used for.
    ///   A diff needs both `OldOrSource` *and* `NewOrDestination`.
    /// * `objects` provides access to the object database in case the resource can't be read from a worktree.
    ///
    /// Object-backed resources are cached by non-null ID and whether they are links. Worktree resources
    /// are cached by root and relative path. Missing object-backed resources get their own attribute lookup
    /// but are not cached, keeping memory bounded when many paths are absent.
    /// Null IDs never identify shared content. Clear the cache when changing roots, filters, or worktree content.
    /// Reusing an object ID across paths currently assumes equivalent path-dependent conversions and drivers;
    /// clear the cache between comparisons with different conversion contexts.
    ///
    /// ### Important
    ///
    /// If an error occurs, the previous resource of `kind` will be cleared, preventing further diffs
    /// unless another attempt succeeds.
    pub fn set_resource(
        &mut self,
        id: Option<gix_hash::ObjectId>,
        mode: gix_object::tree::EntryKind,
        rela_path: &BStr,
        kind: ResourceKind,
        objects: &impl gix_object::FindObjectOrHeader, // TODO: make this `dyn` once https://github.com/rust-lang/rust/issues/65991 is stable, then also make tracker.rs `objects` dyn
    ) -> ExnResult<(), set_resource::Error> {
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
    ) -> Result<prepare_diff_command::Command> {
        fn add_resource(
            cmd: &mut std::process::Command,
            res: Resource<'_>,
            object_hash: gix_hash::Kind,
        ) -> Result<Option<gix_tempfile::Handle<gix_tempfile::handle::Closed>>> {
            let tmpfile = match res.data {
                resource::Data::Missing => {
                    cmd.args(["/dev/null", ".", "."]);
                    None
                }
                resource::Data::Buffer { buf, is_derived: _ } => {
                    let mut tmp = gix_tempfile::new(
                        std::env::temp_dir(),
                        gix_tempfile::ContainingDirectory::Exists,
                        gix_tempfile::AutoRemove::Tempfile,
                    )
                    .or_raise(|| {
                        message!(
                            "Tempfile to store content of \"{}\" for passing to external diff command could not be created",
                            res.rela_path
                        )
                    })?;
                    tmp.write_all(buf).or_raise(|| {
                        message!(
                            "Could not write content of \"{}\" to tempfile for passing to external diff command",
                            res.rela_path
                        )
                    })?;
                    tmp.with_mut(|f| {
                        cmd.arg(f.path());
                    })
                    .or_raise(|| {
                        message!(
                            "Could not access tempfile for \"{}\" while preparing external diff command",
                            res.rela_path
                        )
                    })?;
                    cmd.arg(res.id.unwrap_or(object_hash.null_ref()).to_string())
                        .arg(res.mode.as_octal_str().to_string());
                    let tmp = tmp.close().or_raise(|| {
                        message!(
                            "Could not close tempfile for \"{}\" while preparing external diff command",
                            res.rela_path
                        )
                    })?;
                    Some(tmp)
                }
                resource::Data::Binary { .. } => {
                    bail!(validation(
                        "Binary resources can't be diffed with an external command (as we don't have the data anymore)"
                    ));
                }
            };
            Ok(tmpfile)
        }

        let (old, new) = self
            .resources()
            .ok_or_raise(|| message("Either the source or the destination of the diff operation were not set"))?;
        let mut cmd: std::process::Command = gix_command::prepare(gix_path::from_bstring(diff_command)?)
            .command_may_be_shell_script_disallow_manual_argument_splitting()
            .with_context(context)
            .env("GIT_DIFF_PATH_COUNTER", (count + 1).to_string())
            .env("GIT_DIFF_PATH_TOTAL", total.to_string())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .try_into()?;

        cmd.arg(gix_path::from_bstr(old.rela_path)?.into_owned());
        let mut out = prepare_diff_command::Command {
            cmd,
            old: None,
            new: None,
        };

        out.old = add_resource(&mut out.cmd, old, self.filter.worktree_filter.object_hash())?;
        out.new = add_resource(&mut out.cmd, new, self.filter.worktree_filter.object_hash())?;

        if old.rela_path != new.rela_path {
            out.cmd.arg(gix_path::from_bstr(new.rela_path)?.into_owned());
        }

        Ok(out)
    }

    /// Returns the resource of the given kind if it was set.
    pub fn resource(&self, kind: ResourceKind) -> Option<Resource<'_>> {
        let key = match kind {
            ResourceKind::OldOrSource => self.old.as_ref(),
            ResourceKind::NewOrDestination => self.new.as_ref(),
        }?;
        Some(if key.kind == CacheKeyKind::Missing {
            Resource {
                driver_index: self.missing_driver_index[kind as usize],
                data: resource::Data::Missing,
                mode: key.mode,
                rela_path: key.location.as_ref(),
                id: key.id.as_deref(),
            }
        } else {
            Resource::new(key, self.diff_cache.get(key)?)
        })
    }

    /// Obtain the two resources that were previously set as `(OldOrSource, NewOrDestination)`, if both are set and available.
    ///
    /// This is useful if one wishes to manually prepare the diff, maybe for invoking external programs, instead of relying on
    /// [`Self::prepare_diff()`].
    pub fn resources(&self) -> Option<(Resource<'_>, Resource<'_>)> {
        Some((
            self.resource(ResourceKind::OldOrSource)?,
            self.resource(ResourceKind::NewOrDestination)?,
        ))
    }

    /// Prepare a diff operation on the [previously set](Self::set_resource()) [old](ResourceKind::OldOrSource) and
    /// [new](ResourceKind::NewOrDestination) resources.
    ///
    /// Match [`prepare_diff::Outcome::operation`] to obtain validated text for patch generation or similarity checking.
    /// Missing sides are represented as additions or deletions, not as empty resources. If both sides are missing,
    /// [`prepare_diff::Operation::NeitherExists`] is returned. A side that was never set successfully remains an error.
    pub fn prepare_diff(&mut self) -> ExnResult<prepare_diff::Outcome<'_>, prepare_diff::Error> {
        use prepare_diff::{Operation, TextChange, TextPair, TextResource};
        use resource::Data;

        let (old, new) = self.resources().ok_or(prepare_diff::Error::SourceOrDestinationUnset)?;
        let mut out = prepare_diff::Outcome {
            operation: Operation::SourceOrDestinationIsBinary,
            old_or_new_is_derived: old.data.is_derived() || new.data.is_derived(),
            old,
            new,
        };
        let change = match (old.data, new.data) {
            (Data::Missing, Data::Missing) => {
                out.operation = Operation::NeitherExists;
                return Ok(out);
            }
            (Data::Binary { .. }, _) | (_, Data::Binary { .. }) => return Ok(out),
            (Data::Missing, Data::Buffer { buf, is_derived }) => TextChange::Addition {
                new: TextResource::new(buf, is_derived),
            },
            (Data::Buffer { buf, is_derived }, Data::Missing) => TextChange::Deletion {
                old: TextResource::new(buf, is_derived),
            },
            (
                Data::Buffer {
                    buf: old,
                    is_derived: old_is_derived,
                },
                Data::Buffer {
                    buf: new,
                    is_derived: new_is_derived,
                },
            ) => TextChange::Modification(TextPair::from_text(
                TextResource::new(old, old_is_derived),
                TextResource::new(new, new_is_derived),
            )),
        };
        let driver = old.driver_index.and_then(|idx| self.filter.drivers.get(idx));
        out.operation = match driver
            .and_then(|driver| driver.command.as_deref())
            .filter(|_| self.options.skip_internal_diff_if_external_is_configured)
        {
            Some(command) => Operation::ExternalCommand {
                command: command.as_bstr(),
                change,
            },
            None => Operation::InternalDiff {
                algorithm: driver
                    .and_then(|driver| driver.algorithm)
                    .or(self.options.algorithm)
                    .unwrap_or_default(),
                change,
            },
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
        id: Option<gix_hash::ObjectId>,
        mode: gix_object::tree::EntryKind,
        rela_path: &BStr,
        kind: ResourceKind,
        objects: &impl gix_object::FindObjectOrHeader,
    ) -> ExnResult<(), set_resource::Error> {
        if matches!(
            mode,
            gix_object::tree::EntryKind::Commit | gix_object::tree::EntryKind::Tree
        ) {
            bail!(set_resource::Error::InvalidMode { mode });
        }
        let storage = match kind {
            ResourceKind::OldOrSource => &mut self.old,
            ResourceKind::NewOrDestination => &mut self.new,
        }
        .get_or_insert_with(Default::default);

        let id = id.filter(|id| !id.is_null());
        storage.id = id;
        storage.set_location(rela_path);
        storage.mode = mode;
        storage.kind = match self.filter.roots.by_kind(kind) {
            None if id.is_none() => CacheKeyKind::Missing,
            None => CacheKeyKind::Object,
            Some(_) => match kind {
                ResourceKind::OldOrSource => CacheKeyKind::OldWorktree,
                ResourceKind::NewOrDestination if self.filter.roots.old_root == self.filter.roots.new_root => {
                    CacheKeyKind::OldWorktree
                }
                ResourceKind::NewOrDestination => CacheKeyKind::NewWorktree,
            },
        };

        if self.diff_cache.contains_key(storage) {
            return Ok(());
        }
        let entry =
            self.attr_stack
                .at_entry(rela_path, None, objects)
                .or_raise_typed(|| set_resource::Error::Attributes {
                    kind,
                    rela_path: rela_path.into(),
                })?;
        let mut buf = self.free_list.pop().unwrap_or_default();
        let out = self
            .filter
            .convert_to_diffable(
                id.as_deref(),
                mode,
                rela_path,
                kind,
                &mut |_, out| {
                    let _ = entry.matching_attributes(out);
                },
                objects,
                self.filter_mode,
                &mut buf,
            )
            .or_raise_typed(|| set_resource::Error::ConvertToDiffable {
                kind,
                rela_path: rela_path.into(),
            })?;
        if storage.kind == CacheKeyKind::Missing {
            self.missing_driver_index[kind as usize] = out.driver_index;
            self.free_list.push(buf);
            return Ok(());
        }
        let key = storage.clone();
        assert!(
            self.diff_cache
                .insert(
                    key,
                    CacheValue {
                        conversion: out,
                        buffer: buf,
                    },
                )
                .is_none(),
            "The key impl makes clashes impossible with our usage"
        );
        Ok(())
    }
}
