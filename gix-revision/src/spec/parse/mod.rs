use crate::spec;
use bstr::BString;

/// The error returned by [`spec::parse()`][crate::spec::parse()].
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error("'~' needs to follow an anchor, like '@~'.")]
    MissingTildeAnchor,
    #[error("':' needs to be followed by either '/' and regex or the path to lookup in the HEAD tree.")]
    MissingColonSuffix,
    #[error("':/' must be followed by a regular expression.")]
    EmptyTopLevelRegex,
    #[error("Need one character after '/!', typically '-', but got {:?}", .regex)]
    UnspecifiedRegexModifier { regex: BString },
    #[error("Cannot peel to {:?} - unknown target.", .input)]
    InvalidObject { input: BString },
    #[error("Could not parse time {:?} for revlog lookup.", .input)]
    Time {
        input: BString,
        source: Option<Box<gix_date::Error>>,
    },
    #[error("Sibling branches like 'upstream' or 'push' require a branch name with remote configuration, got {:?}", .name)]
    SiblingBranchNeedsBranchName { name: BString },
    #[error("Reflog entries require a ref name, got {:?}", .name)]
    ReflogLookupNeedsRefName { name: BString },
    #[error("A reference name must be followed by positive numbers in '@{{n}}', got {:?}", .nav)]
    RefnameNeedsPositiveReflogEntries { nav: BString },
    #[error("Negative or explicitly positive numbers are invalid here: {:?}", .input)]
    SignedNumber { input: BString },
    #[error("Could not parse number from {input:?}")]
    InvalidNumber { input: BString },
    #[error("Negative zeroes are invalid: {:?} - remove the '-'", .input)]
    NegativeZero { input: BString },
    #[error("The opening brace in {:?} was not matched", .input)]
    UnclosedBracePair { input: BString },
    #[error("Cannot set spec kind more than once. Previous value was {:?}, now it is {:?}", .prev_kind, .kind)]
    KindSetTwice { prev_kind: spec::Kind, kind: spec::Kind },
    #[error("The @ character is either standing alone or followed by `{{<content>}}`, got {:?}", .input)]
    AtNeedsCurlyBrackets { input: BString },
    #[error("A portion of the input could not be parsed: {:?}", .input)]
    UnconsumedInput { input: BString },
    #[error("The delegate didn't indicate success - check delegate for more information")]
    Delegate,
}

///
pub mod delegate;

/// A delegate to be informed about parse events, with methods split into categories.
///
/// - **Anchors** - which revision to use as starting point for…
/// - **Navigation** - where to go once from the initial revision
/// - **Range** - to learn if the specification is for a single or multiple references, and how to combine them.
pub trait Delegate: delegate::Revision + delegate::Navigate + delegate::Kind {
    /// Called at the end of a successful parsing operation.
    /// It can be used as a marker to finalize internal data structures.
    ///
    /// Note that it will not be called if there is unconsumed input.
    fn done(&mut self);
}

pub(crate) mod function;
