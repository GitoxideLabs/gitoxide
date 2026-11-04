// TODO: remove this - only needed while &mut Vec<u8> isn't used.
#![allow(clippy::ptr_arg)]

use std::path::PathBuf;

use bstr::BString;

use crate::blob::platform::{DriverChoice, ResourceRef};

///
pub mod builtin_driver;
///
pub mod pipeline;
///
pub mod platform;

/// Define if a merge is conflicted or not.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Resolution {
    /// Everything could be resolved during the merge, and there was no conflict.
    Complete,
    /// Conflicts were resolved automatically, even thought the result is complete
    /// and free of conflict markers.
    /// This can only be the case for text-file content merges.
    CompleteWithAutoResolvedConflict,
    /// A conflict is still present in the form of conflict markers.
    Conflict,
}

/// A way to classify the side of a resource for merging.
#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum ResourceKind {
    /// Our side of the state.
    CurrentOrOurs,
    /// Their side of the state.
    OtherOrTheirs,
    /// The state of the common base of both ours and theirs.
    CommonAncestorOrBase,
}

/// Define a built-in way of performing a three-way merge, including auto-resolution support.
///
/// Some values are related to diffing, some are related to conversions.
#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BuiltinDriver {
    /// Perform a merge between text-sources such that conflicts are marked according to
    /// `merge.conflictStyle` in the Git configuration.
    ///
    /// If any of the inputs, *base*, *ours* or *theirs* looks like non-text/binary,
    /// the [`Binary`](Self::Binary) driver will be used instead.
    ///
    /// Also see [`builtin_driver::text::ConflictStyle`].
    #[default]
    Text,
    /// Merge 'unmergable' content by choosing *ours* or *theirs*, without performing
    /// an actual merge.
    ///
    /// Note that if the merge operation is for virtual ancestor (a merge for merge-bases),
    /// then *ours* will always be chosen.
    Binary,
    /// Merge text-sources and resolve conflicts by adding conflicting lines one after another,
    /// in random order, without adding conflict markers either.
    ///
    /// This can be useful for files that change a lot, but will remain usable merely by adding
    /// all changed lines.
    Union,
}

/// Define a driver program that performs a three-way merge.
///
/// Some values are related to diffing, some are related to conversions.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Driver {
    /// The name of the driver, as referred to by `[merge "name"]` in the git configuration.
    pub name: BString,
    /// The human-readable version of `name`, only to be used for displaying driver-information to the user.
    pub display_name: BString,
    /// The command to execute to perform the merge entirely like `<command> %O %A %B %L %P %S %X %Y`.
    ///
    /// * **%O**
    ///     - the common ancestor version, or *base*.
    /// * **%A**
    ///     - the current version, or *ours*.
    /// * **%B**
    ///     - the other version, or *theirs*.
    /// * **%L**
    ///     - The conflict-marker size as positive number.
    /// * **%P**
    ///     - The path in which the merged result would be stored, as workspace-relative path, of the current/ours side.
    /// * **%S**
    ///     - The conflict-label for the common ancestor or *base*.
    /// * **%X**
    ///     - The conflict-label for the current version or *ours*.
    /// * **%Y**
    ///     - The conflict-label for the other version or *theirs*.
    ///
    /// Note that conflict-labels are behind the conflict markers, to annotate them.
    ///
    /// A typical invocation with all arguments substituted could then look like this:
    ///
    /// ```sh
    /// <driver-program> .merge_file_nR2Qs1 .merge_file_WYXCJe .merge_file_UWbzrm 7 file e2a2970 HEAD feature
    /// ```
    ///
    /// The driver is expected to leave its version in the file at `%A`, by overwriting it.
    /// It exits with `0` if the merge is clean, or with a status of up to `128` if conflicts remain in `%A`.
    /// Any other exit status, or termination by signal, is considered a failure of the driver.
    pub command: BString,
    /// If `true`, this is the `name` of the driver to use when a virtual-merge-base is created, as a merge of all
    /// available merge-bases if there are more than one.
    ///
    /// This value can also be special built-in drivers named `text`, `binary` or `union`. Note that user-defined
    /// drivers with the same name will be preferred over built-in ones, but only for files whose git attributes
    /// specified the driver by *name*.
    pub recursive: Option<BString>,
}

/// A conversion pipeline to take an object or path from what's stored in Git to what can be merged, while
/// following the guidance of git-attributes at the respective path to learn how the merge should be performed.
///
/// Depending on the source, different conversions are performed:
///
/// * `worktree on disk` -> `object for storage in git`
/// * `object` -> `possibly renormalized object`
///     - Renormalization means that the `object` is converted to what would be checked out into the work-tree,
///       just to turn it back into an object.
#[derive(Clone)]
pub struct Pipeline {
    /// A way to read data directly from the worktree.
    pub roots: pipeline::WorktreeRoots,
    /// A pipeline to convert objects from the worktree to Git, and also from Git to the worktree, and back to Git.
    pub filter: gix_filter::Pipeline,
    /// Options affecting the way we read files.
    pub options: pipeline::Options,
    /// A buffer to produce disk-accessible paths from worktree roots.
    path: PathBuf,
}

/// A utility for gathering and processing all state necessary to perform a three-way merge.
///
/// It can re-use buffers if all three parts of participating in the merge are
/// set repeatedly.
#[derive(Clone)]
pub struct Platform {
    /// The current version (ours).
    current: Option<platform::Resource>,
    /// The ancestor version (base).
    ancestor: Option<platform::Resource>,
    /// The other version (theirs).
    other: Option<platform::Resource>,

    /// A way to convert objects into a diff-able format.
    pub filter: Pipeline,
    /// A way to access `.gitattributes`
    pub attr_stack: gix_worktree::Stack,
    /// Further configuration that affects the merge.
    pub options: platform::Options,
    /// All available merge drivers.
    ///
    /// They are referenced in git-attributes by name, and we hand out indices into this array.
    drivers: Vec<Driver>,
    /// Pre-configured attributes to obtain additional merge-related information.
    attrs: gix_filter::attributes::search::Outcome,
    /// The way we convert resources into mergeable states.
    pub filter_mode: pipeline::Mode,
}

/// The product of a [`prepare_merge()`](Platform::prepare_merge()) call to finally
/// perform the merge and retrieve the merge results.
#[derive(Copy, Clone)]
pub struct PlatformRef<'parent> {
    /// The platform that hosts the resources, used to access drivers.
    pub(super) parent: &'parent Platform,
    /// The current or our side of the merge operation.
    pub current: ResourceRef<'parent>,
    /// The ancestor or base of the merge operation.
    pub ancestor: ResourceRef<'parent>,
    /// The other or their side of the merge operation.
    pub other: ResourceRef<'parent>,
    /// Which driver to use according to the resource's configuration,
    /// using the path of `current` to read git-attributes.
    pub driver: DriverChoice,
    /// Possibly processed options for use when performing the actual merge.
    ///
    /// They may be inspected before the merge, or altered at will.
    pub options: platform::merge::Options,
}
