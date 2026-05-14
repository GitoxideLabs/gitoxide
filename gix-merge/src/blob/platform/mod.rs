use bstr::{BStr, BString};
use gix_filter::attributes;

use crate::blob::{BuiltinDriver, Pipeline, Platform, pipeline};

/// A stored value representing a resource that participates in a merge.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub(super) struct Resource {
    /// The `id` of the value, or `null` if it's only living in a worktree.
    id: gix_hash::ObjectId,
    /// The repository-relative path where the resource lives in the tree.
    rela_path: BString,
    /// The outcome of converting a resource into a mergable format using [Pipeline::convert_to_mergeable()].
    data: Option<pipeline::Data>,
    /// The kind of the resource we are looking at. Only possible values are `Blob` and `BlobExecutable`.
    mode: gix_object::tree::EntryKind,
    /// A possibly empty buffer, depending on `conversion.data` which may indicate the data is considered binary
    /// or the resource doesn't exist.
    buffer: Vec<u8>,
}

/// A blob or executable ready to be merged in one way or another.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct ResourceRef<'a> {
    /// The data itself, suitable for merging, and if the object or worktree item is present at all.
    pub data: resource::Data<'a>,
    /// The location of the resource, relative to the working tree.
    pub rela_path: &'a BStr,
    /// The id of the content as it would be stored in `git`, or `null` if the content doesn't exist anymore at
    /// `rela_path` or if it was never computed. This can happen with content read from the worktree, which
    /// after its 'to-git' conversion never had its hash computed.
    pub id: &'a gix_hash::oid,
}

/// Options for use in [`Platform::new()`].
#[derive(Default, Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd)]
pub struct Options {
    /// Define which driver to use by name if the `merge` attribute for a resource is unspecified.
    ///
    /// This is the value of the `merge.default` git configuration.
    pub default_driver: Option<BString>,
}

/// The selection of the driver to use by a resource obtained with [`Platform::prepare_merge()`].
///
/// If available, an index into the `drivers` field to access more diff-related information of the driver for items
/// at the given path, as previously determined by git-attributes.
///
/// * `merge` is set
///     - Use the [`BuiltinDriver::Text`]
/// * `-merge` is unset
///     - Use the [`BuiltinDriver::Binary`]
/// * `!merge` is unspecified
///     - Use [`Options::default_driver`] or [`BuiltinDriver::Text`].
/// * `merge=name`
///     - Search for a user-configured or built-in driver called `name`.
///     - If not found, silently default to [`BuiltinDriver::Text`]
///
/// Note that drivers are queried even if there is no object available.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum DriverChoice {
    /// Use the given built-in driver to perform the merge.
    BuiltIn(BuiltinDriver),
    /// Use the user-provided driver program using the index into [the platform drivers array](Platform::drivers()).
    Index(usize),
}

impl Default for DriverChoice {
    fn default() -> Self {
        DriverChoice::BuiltIn(Default::default())
    }
}

/// Lifecycle
impl Platform {
    /// Create a new instance with a way to `filter` data from the object database and turn it into something that is merge-able.
    /// `filter_mode` decides how to do that specifically.
    /// Use `attr_stack` to access attributes pertaining worktree filters and merge settings.
    /// `drivers` are the list of available merge drivers that individual paths can refer to by means of git attributes.
    /// `options` further configure the operation.
    pub fn new(
        filter: Pipeline,
        filter_mode: pipeline::Mode,
        attr_stack: gix_worktree::Stack,
        mut drivers: Vec<super::Driver>,
        options: Options,
    ) -> Self {
        drivers.sort_by(|a, b| a.name.cmp(&b.name));
        Platform {
            drivers,
            current: None,
            ancestor: None,
            other: None,
            filter,
            filter_mode,
            attr_stack,
            attrs: {
                let mut out = attributes::search::Outcome::default();
                out.initialize_with_selection(&Default::default(), ["merge", "conflict-marker-size"]);
                out
            },
            options,
        }
    }
}

/// Access
impl Platform {
    /// Return all drivers that this instance was initialized with.
    ///
    /// They are sorted by [`name`](super::Driver::name) to support binary searches.
    pub fn drivers(&self) -> &[super::Driver] {
        &self.drivers
    }
}

///
pub mod set_resource;

///
pub mod resource;

///
pub mod merge;
pub use merge::inner::{builtin_merge, prepare_external_driver};

///
pub mod prepare_merge;
