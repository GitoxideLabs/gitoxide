/// Information gathered during the run of [`iter_from_objects()`][super::objects()].
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Outcome {
    /// The amount of objects provided to start the iteration.
    pub input_objects: usize,
    /// The amount of objects that have been expanded from the input source.
    /// It's desirable to do that as expansion happens on multiple threads, allowing the amount of input objects to be small.
    /// `expanded_objects - decoded_objects` is the 'cheap' object we found without decoding the object itself.
    pub expanded_objects: usize,
    /// The amount of fully decoded objects. These are the most expensive as they are fully decoded
    pub decoded_objects: usize,
    /// The total amount of encountered objects. Should be `expanded_objects + input_objects`.
    pub total_objects: usize,
}

impl Outcome {
    pub(in crate::data::output::count) fn aggregate(
        &mut self,
        Outcome {
            input_objects,
            decoded_objects,
            expanded_objects,
            total_objects,
        }: Self,
    ) {
        self.input_objects += input_objects;
        self.decoded_objects += decoded_objects;
        self.expanded_objects += expanded_objects;
        self.total_objects += total_objects;
    }
}

/// The way input objects are handled
#[derive(Default, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ObjectExpansion {
    /// Don't do anything with the input objects except for transforming them into pack entries
    #[default]
    AsIs,
    /// If the input object is a Commit then turn it into a pack entry. Additionally obtain its tree, turn it into a pack entry
    /// along with all of its contents, that is nested trees, and any other objects reachable from it.
    /// Otherwise, the same as [`AsIs`][ObjectExpansion::AsIs].
    ///
    /// This mode is useful if all reachable objects should be added, as in cloning a repository.
    TreeContents,
    /// If the input is a commit, obtain its ancestors and turn them into pack entries. Obtain the ancestor trees along with the commits
    /// tree and turn them into pack entries. Finally obtain the added/changed objects when comparing the ancestor trees with the
    /// current tree and turn them into entries as well.
    /// Otherwise, the same as [`AsIs`][ObjectExpansion::AsIs].
    ///
    /// This mode is useful to build a pack containing only new objects compared to a previous state.
    TreeAdditionsComparedToAncestor,
}

/// Configuration options for the pack generation functions provided in [this module][crate::data::output].
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone, Copy)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Options {
    /// The amount of threads to use at most when resolving the pack. If `None`, all logical cores are used.
    /// If more than one thread is used, the order of returned [counts][crate::data::output::Count] is not deterministic anymore
    /// especially when tree traversal is involved. Thus deterministic ordering requires `Some(1)` to be set.
    pub thread_limit: Option<usize>,
    /// The amount of objects per chunk or unit of work to be sent to threads for processing
    pub chunk_size: usize,
    /// The way input objects are handled
    pub input_object_expansion: ObjectExpansion,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            thread_limit: None,
            chunk_size: 10,
            input_object_expansion: Default::default(),
        }
    }
}

/// The error returned by the pack generation iterator [`bytes::FromEntriesIter`][crate::data::output::bytes::FromEntriesIter].
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error(transparent)]
    CommitDecode(gix_object::decode::Error),
    #[error(transparent)]
    FindExisting(gix_error::Error),
    #[error(transparent)]
    InputIteration(Box<dyn std::error::Error + Send + Sync + 'static>),
    #[error(transparent)]
    TreeTraverse(gix_error::Error),
    #[error(transparent)]
    TreeChanges(gix_diff::tree::Error),
    #[error("Operation interrupted")]
    Interrupted,
}

impl From<gix_object::find::existing::Error> for Error {
    fn from(err: gix_object::find::existing::Error) -> Self {
        Error::FindExisting(err.into_error())
    }
}
