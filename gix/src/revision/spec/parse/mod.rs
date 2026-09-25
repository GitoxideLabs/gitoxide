use crate::{Repository, Result, bstr::BStr, revision::Spec};
use gix_error::{ClassificationMarker, Exn};
use gix_hash::ObjectId;

mod types;
pub use types::{ObjectKindHint, Options, RefsHint};

use crate::bstr::BString;

///
pub mod error;
pub use error::CandidateInfo;

/// A recoverable failure while parsing a revision specification.
/// Other failures retain their original causes in [`crate::Error`].
///
/// A failure can contain several recovery errors, for example when both ends of a range fail to resolve.
/// Use [`crate::Error::iter_errors()`] to inspect all of them.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A reference needed to resolve the specification could not be found.
    /// Intrinsically classified as [`gix_error::Class::NotFound`].
    ///
    /// The original reference lookup error remains available as a cause. Missing objects and tree or index paths
    /// do not produce this variant. Whether to treat the input as a filesystem path is a caller policy.
    MissingReference {
        /// The missing reference name, which may have been discovered while following symbolic references.
        /// It is not necessarily the original revision specification.
        name: std::path::PathBuf,
    },
    /// More than one object matches the prefix and the specification did not disambiguate them.
    /// Intrinsically classified as [`gix_error::Class::Validation`].
    AmbiguousPrefix {
        /// The ambiguous object-id prefix.
        prefix: gix_hash::Prefix,
        /// Candidates ordered by known kind (tag, commit, tree, blob), then object id, with failed lookups last.
        candidates: Vec<(gix_hash::Prefix, CandidateInfo)>,
    },
    /// A prefix matches both a reference and at least one object, and [`RefsHint::Fail`] forbids choosing either.
    /// Intrinsically classified as [`gix_error::Class::Validation`].
    AmbiguousRefAndObject {
        /// The object-id prefix that also matched a reference.
        prefix: gix_hash::Prefix,
        /// The full name of the matching reference.
        reference: gix_ref::FullName,
        /// Object candidates ordered as in [`Error::AmbiguousPrefix`]. There may be only one.
        candidates: Vec<(gix_hash::Prefix, CandidateInfo)>,
    },
}

impl std::fmt::Display for Error {
    #[allow(
        clippy::unnecessary_debug_formatting,
        reason = "auto-encloses in quotes and escapes values"
    )]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let candidates = match self {
            Self::MissingReference { name } => return write!(f, "Reference {name:?} could not be found"),
            Self::AmbiguousPrefix { prefix, candidates } => {
                write!(f, "Short id {prefix} is ambiguous. Candidates are:")?;
                candidates
            }
            Self::AmbiguousRefAndObject {
                prefix,
                reference,
                candidates,
            } => {
                write!(
                    f,
                    "The object-id prefix {prefix} matched both the reference {reference} and at least one object. Candidates are:"
                )?;
                candidates
            }
        };
        for (object_id, info) in candidates {
            write!(f, "\n\t{object_id} {info}")?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::MissingReference { .. } => Some(const { &ClassificationMarker::NOT_FOUND }),
            Self::AmbiguousPrefix { .. } | Self::AmbiguousRefAndObject { .. } => {
                Some(const { &ClassificationMarker::VALIDATION })
            }
        }
    }
}

impl<'repo> Spec<'repo> {
    /// Parse `spec` and use information from `repo` to resolve it, using `opts` to learn how to deal with ambiguity.
    ///
    /// Recoverable failures can be inspected through [`Error`] in the returned error's causes.
    ///
    /// Note that it's easier to use [`repo.rev_parse()`][Repository::rev_parse()] instead.
    pub fn from_bstr<'a>(spec: impl Into<&'a BStr>, repo: &'repo Repository, opts: Options) -> Result<Self> {
        let mut delegate = Delegate::new(repo, opts);
        match gix_revision::spec::parse(spec.into(), &mut delegate) {
            Err(err) => {
                if let Some(delegate_err) = delegate.into_delayed_errors() {
                    let mut err = err.into_exn();
                    let sources: Vec<_> = err.drain_children().collect();
                    Err(err.chain(delegate_err.chain_all(sources)).into_error())
                } else {
                    Err(err)
                }
            }
            Ok(()) => delegate.into_rev_spec(),
        }
    }
}

struct Delegate<'repo> {
    refs: [Option<gix_ref::Reference>; 2],
    objs: [Option<Vec<ObjectId>>; 2],
    /// Path specified like `@:<path>` or `:<path>` for later use when looking up specs.
    /// Note that it terminates spec parsing, so it's either `0` or `1`, never both.
    paths: [Option<(BString, gix_object::tree::EntryMode)>; 2],
    /// The originally encountered ambiguous objects for potential later use in errors.
    ambiguous_objects: [Option<Vec<ObjectId>>; 2],
    idx: usize,
    kind: Option<gix_revision::spec::Kind>,

    opts: Options,
    /// Keeps track of errors that are supposed to be returned later.
    delayed_errors: Vec<Exn>,
    /// The ambiguous prefix obtained during a call to `disambiguate_prefix()`.
    prefix: [Option<gix_hash::Prefix>; 2],
    /// If true, we didn't try to do any other transformation which might have helped with disambiguation.
    last_call_was_disambiguate_prefix: [bool; 2],

    repo: &'repo Repository,
}

mod delegate;
