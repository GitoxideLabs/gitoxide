use crate::{bstr::BString, object, reference, remote};

/// A hint to know what to do if refs and object names are equal.
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub enum RefsHint {
    /// This is the default, and leads to specs that look like objects identified by full hex sha and are objects to be used
    /// instead of similarly named references. The latter is not typical but can absolutely happen by accident.
    /// If the object prefix is shorter than the maximum hash length of the repository, use the reference instead, which is
    /// preferred as there are many valid object names like `beef` and `cafe` that are short and both valid and typical prefixes
    /// for objects.
    /// Git chooses this as default as well, even though it means that every object prefix is also looked up as ref.
    #[default]
    PreferObjectOnFullLengthHexShaUseRefOtherwise,
    /// No matter what, if it looks like an object prefix and has an object, use it.
    /// Note that no ref-lookup is made here which is the fastest option.
    PreferObject,
    /// When an object is found for a given prefix, also check if a reference exists with that name and if it does,
    /// use that moving forward.
    PreferRef,
    /// If there is an ambiguous situation, instead of silently choosing one over the other, fail instead.
    Fail,
}

/// A hint to know which object kind to prefer if multiple objects match a prefix.
///
/// This disambiguation mechanism is applied only if there is no disambiguation hints in the spec itself.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ObjectKindHint {
    /// Pick objects that are commits themselves.
    Commit,
    /// Pick objects that can be peeled into a commit, i.e. commits themselves or tags which are peeled until a commit is found.
    Committish,
    /// Pick objects that are trees themselves.
    Tree,
    /// Pick objects that can be peeled into a tree, i.e. trees themselves or tags which are peeled until a tree is found or commits
    /// whose tree is chosen.
    Treeish,
    /// Pick objects that are blobs.
    Blob,
}

/// Options for use in [`revision::Spec::from_bstr()`][crate::revision::Spec::from_bstr()].
#[derive(Debug, Default, Copy, Clone)]
pub struct Options {
    /// What to do if both refs and object names match the same input.
    pub refs_hint: RefsHint,
    /// The hint to use when encountering multiple object matching a prefix.
    ///
    /// If `None`, the rev-spec itself must disambiguate the object by drilling down to desired kinds or applying
    /// other disambiguating transformations.
    pub object_kind_hint: Option<ObjectKindHint>,
}

/// The error returned by [`crate::Repository::rev_parse()`].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("Could not peel '{}' to obtain its target", name)]
    PeelToId {
        name: gix_ref::FullName,
        source: reference::peel::Error,
    },
    #[error("The rev-spec is malformed and misses a ref name")]
    Malformed,
    #[error("Unborn heads do not have a reflog yet")]
    UnbornHeadsHaveNoRefLog,
    #[error("Unborn heads cannot have push or upstream tracking branches")]
    UnbornHeadForSibling,
    #[error("Branch named {name} does not have a {} tracking branch configured", direction.as_str())]
    NoTrackingBranch {
        name: gix_ref::FullName,
        direction: remote::Direction,
    },
    #[error("Error when obtaining {} tracking branch for {name}", direction.as_str())]
    GetTrackingBranch {
        name: gix_ref::FullName,
        direction: remote::Direction,
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    #[error("Reference {reference:?} does not have a reference log, cannot {action}")]
    MissingRefLog { reference: BString, action: &'static str },
    #[error("HEAD has {available} prior checkouts and checkout number {desired} is out of range")]
    PriorCheckoutOutOfRange { desired: usize, available: usize },
    #[error("Reference {:?} has {available} ref-log entries and entry number {desired} is out of range", reference.name.as_bstr())]
    RefLogEntryOutOfRange {
        reference: gix_ref::Reference,
        desired: usize,
        available: usize,
    },
    #[error(
        "Commit {oid} has {available} ancestors along the first parent and ancestor number {desired} is out of range"
    )]
    AncestorOutOfRange {
        oid: gix_hash::Prefix,
        desired: usize,
        available: usize,
    },
    #[error("Commit {oid} has {available} parents and parent number {desired} is out of range")]
    ParentOutOfRange {
        oid: gix_hash::Prefix,
        desired: usize,
        available: usize,
    },
    #[error("Path {desired_path:?} did not exist in index at stage {}{}{}", *desired_stage as u8, stage_hint.map(|actual|format!(". It does exist at stage {}", actual as u8)).unwrap_or_default(), exists.then(|| ". It exists on disk").unwrap_or(". It does not exist on disk"))]
    IndexLookup {
        desired_path: BString,
        desired_stage: gix_index::entry::Stage,
        stage_hint: Option<gix_index::entry::Stage>,
        exists: bool,
    },
    #[error(transparent)]
    FindHead(#[from] reference::find::existing::Error),
    #[error(transparent)]
    Index(#[from] crate::worktree::open_index::Error),
    #[error(transparent)]
    RevWalkIterInit(#[from] crate::reference::iter::init::Error),
    #[error(transparent)]
    RevWalkAllReferences(#[from] gix_ref::packed::buffer::open::Error),
    #[cfg(feature = "revparse-regex")]
    #[error(transparent)]
    InvalidRegex(#[from] regex::Error),
    #[cfg_attr(
        feature = "revparse-regex",
        error("None of {commits_searched} commits from {oid} matched regex {regex:?}")
    )]
    #[cfg_attr(
        not(feature = "revparse-regex"),
        error("None of {commits_searched} commits from {oid} matched text {regex:?}")
    )]
    NoRegexMatch {
        regex: BString,
        oid: gix_hash::Prefix,
        commits_searched: usize,
    },
    #[cfg_attr(
        feature = "revparse-regex",
        error("None of {commits_searched} commits reached from all references matched regex {regex:?}")
    )]
    #[cfg_attr(
        not(feature = "revparse-regex"),
        error("None of {commits_searched} commits reached from all references matched text {regex:?}")
    )]
    NoRegexMatchAllRefs { regex: BString, commits_searched: usize },
    #[error(
    "The short hash {prefix} matched both the reference {} and at least one object", reference.name)]
    AmbiguousRefAndObject {
        /// The prefix to look for.
        prefix: gix_hash::Prefix,
        /// The reference matching the prefix.
        reference: gix_ref::Reference,
    },
    #[error(transparent)]
    IdFromHex(#[from] gix_hash::decode::Error),
    #[error(transparent)]
    FindReference(#[from] gix_ref::file::find::existing::Error),
    #[error(transparent)]
    FindObject(#[from] object::find::existing::Error),
    #[error(transparent)]
    LookupPrefix(#[from] gix_odb::store::prefix::lookup::Error),
    #[error(transparent)]
    PeelToKind(#[from] object::peel::to_kind::Error),
    #[error("Object {oid} was a {actual}, but needed it to be a {expected}")]
    ObjectKind {
        oid: gix_hash::Prefix,
        actual: gix_object::Kind,
        expected: gix_object::Kind,
    },
    #[error(transparent)]
    Parse(#[from] gix_revision::spec::parse::Error),
    #[error("An object prefixed {prefix} could not be found")]
    PrefixNotFound { prefix: gix_hash::Prefix },
    #[error("Short id {prefix} is ambiguous. Candidates are:\n{}", info.iter().map(|(oid, info)| format!("\t{oid} {info}")).collect::<Vec<_>>().join("\n"))]
    AmbiguousPrefix {
        prefix: gix_hash::Prefix,
        info: Vec<(gix_hash::Prefix, super::error::CandidateInfo)>,
    },
    #[error("Could not find path {path:?} in tree {tree} of parent object {object}")]
    PathNotFound {
        object: gix_hash::Prefix,
        tree: gix_hash::Prefix,
        path: BString,
    },
    #[error("{current}")]
    Multi {
        current: Box<dyn std::error::Error + Send + Sync + 'static>,
        #[source]
        next: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
    #[error(transparent)]
    Traverse(#[from] crate::revision::walk::iter::Error),
    #[error(transparent)]
    Walk(#[from] crate::revision::walk::Error),
    #[error("Spec does not contain a single object id")]
    SingleNotFound,
    #[error("Reflog does not contain any entries")]
    EmptyReflog,
}
