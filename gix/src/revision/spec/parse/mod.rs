use crate::{bstr::BStr, revision::Spec, Repository};
use gix_error::Exn;
use gix_hash::ObjectId;

mod types;
pub use types::{ObjectKindHint, Options, RefsHint};

use crate::bstr::BString;

///
pub mod single {
    use crate::bstr::BString;

    /// The error returned by [`crate::Repository::rev_parse_single()`].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error(transparent)]
        Parse(#[from] gix_error::Error),
        #[error("revspec {spec:?} did not resolve to a single object")]
        RangedRev { spec: BString },
    }
}

///
pub mod error;

impl<'repo> Spec<'repo> {
    /// Parse `spec` and use information from `repo` to resolve it, using `opts` to learn how to deal with ambiguity.
    ///
    /// Note that it's easier and to use [`repo.rev_parse()`][Repository::rev_parse()] instead.
    pub fn from_bstr<'a>(
        spec: impl Into<&'a BStr>,
        repo: &'repo Repository,
        opts: Options,
    ) -> Result<Self, gix_error::Error> {
        let mut delegate = Delegate::new(repo, opts);
        match gix_revision::spec::parse(spec.into(), &mut delegate) {
            Err(mut err) => {
                if let Some(delegate_err) = delegate.into_delayed_errors() {
                    let sources: Vec<_> = err.drain_children().collect();
                    Err(err.chain(delegate_err.chain_iter(sources)).into_error())
                } else {
                    Err(err.into_error())
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
