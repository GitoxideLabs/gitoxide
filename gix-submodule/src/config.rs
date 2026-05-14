use bstr::{BStr, BString, ByteSlice};

/// Determine how the submodule participates in `git status` queries. This setting also affects `git diff`.
#[derive(Default, Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Ignore {
    /// Submodule changes won't be considered at all, which is the fastest option.
    All,
    /// Ignore any changes to the submodule working tree, only show committed differences between the `HEAD` of the submodule
    /// compared to the recorded commit in the superproject.
    Dirty,
    /// Only ignore untracked files in the submodule, but show modifications to the submodule working tree as well as differences
    /// between the recorded commit in the superproject and the checked-out commit in the submodule.
    Untracked,
    /// No modifications to the submodule are ignored, which shows untracked files, modified files in the submodule worktree as well as
    /// differences between the recorded commit in the superproject and the checked-out commit in the submodule.
    #[default]
    None,
}

impl TryFrom<&BStr> for Ignore {
    type Error = ();

    fn try_from(value: &BStr) -> Result<Self, Self::Error> {
        Ok(match value.as_bytes() {
            b"all" => Ignore::All,
            b"dirty" => Ignore::Dirty,
            b"untracked" => Ignore::Untracked,
            b"none" => Ignore::None,
            _ => return Err(()),
        })
    }
}

/// Determine how to recurse into this module from the superproject when fetching.
///
/// Generally, a fetch is only performed if the submodule commit referenced by the superproject isn't already
/// present in the submodule repository.
///
/// Note that when unspecified, the `fetch.recurseSubmodules` configuration variable should be used instead.
#[derive(Default, Debug, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum FetchRecurse {
    /// Fetch only changed submodules.
    #[default]
    OnDemand,
    /// Fetch all populated submodules, changed or not.
    ///
    /// This skips the work needed to determine whether a submodule has changed in the first place, but may work
    /// more as some fetches might not be necessary.
    Always,
    /// Submodules are never fetched.
    Never,
}

impl FetchRecurse {
    /// Check if `boolean` is set and translate it the respective variant, or check the underlying string
    /// value for non-boolean options.
    /// On error, it returns the obtained string value which would be the invalid value.
    pub fn new(boolean: Result<bool, gix_config::value::Error>) -> Result<Self, BString> {
        Ok(match boolean {
            Ok(value) => {
                if value {
                    FetchRecurse::Always
                } else {
                    FetchRecurse::Never
                }
            }
            Err(err) => {
                if err.input != "on-demand" {
                    return Err(err.input);
                }
                FetchRecurse::OnDemand
            }
        })
    }
}

/// Describes the branch that should be tracked on the remote.
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum Branch {
    /// The name of the remote branch should be the same as the one currently checked out in the superproject.
    CurrentInSuperproject,
    /// The validated remote-only branch that could be used for fetching.
    Name(BString),
}

impl Default for Branch {
    fn default() -> Self {
        Branch::Name("HEAD".into())
    }
}

impl TryFrom<&BStr> for Branch {
    type Error = gix_refspec::parse::Error;

    fn try_from(value: &BStr) -> Result<Self, Self::Error> {
        if value == "." {
            return Ok(Branch::CurrentInSuperproject);
        }

        gix_refspec::parse(value, gix_refspec::parse::Operation::Fetch)
            .map(|spec| Branch::Name(spec.source().expect("no object").to_owned()))
    }
}

/// Determine how `git submodule update` should deal with this submodule to bring it up-to-date with the
/// super-project's expectations.
#[derive(Default, Debug, Clone, Hash, PartialOrd, PartialEq, Ord, Eq)]
pub enum Update {
    /// The commit recorded in the superproject should be checked out on a detached `HEAD`.
    #[default]
    Checkout,
    /// The current branch in the submodule will be rebased onto the commit recorded in the superproject.
    Rebase,
    /// The commit recorded in the superproject will merged into the current branch of the submodule.
    Merge,
    /// A custom command to be called like `<command> hash-of-submodule-commit` that is to be executed to
    /// perform the submodule update.
    ///
    /// Note that this variant is only allowed if the value is coming from an override. Thus it's not allowed to distribute
    /// arbitrary commands via `.gitmodules` for security reasons.
    Command(BString),
    /// The submodule update is not performed at all.
    None,
}

impl TryFrom<&BStr> for Update {
    type Error = ();

    fn try_from(value: &BStr) -> Result<Self, Self::Error> {
        Ok(match value.as_bstr().as_bytes() {
            b"checkout" => Update::Checkout,
            b"rebase" => Update::Rebase,
            b"merge" => Update::Merge,
            b"none" => Update::None,
            command if command.first() == Some(&b'!') => Update::Command(command[1..].to_owned().into()),
            _ => return Err(()),
        })
    }
}

/// The error returned by [File::fetch_recurse()](crate::File::fetch_recurse) and [File::ignore()](crate::File::ignore).
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
#[error("The '{field}' field of submodule '{submodule}' was invalid: '{actual}'")]
pub struct Error {
    pub field: &'static str,
    pub submodule: BString,
    pub actual: BString,
}

///
pub mod branch {
    use bstr::BString;

    /// The error returned by [File::branch()](crate::File::branch).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    #[error(
        "The value '{actual}' of the 'branch' field of submodule '{submodule}' couldn't be turned into a valid fetch refspec"
    )]
    pub struct Error {
        pub submodule: BString,
        pub actual: BString,
        pub source: gix_refspec::parse::Error,
    }
}

///
pub mod update {
    use bstr::BString;

    /// The error returned by [File::update()](crate::File::update).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The 'update' field of submodule '{submodule}' tried to set command '{actual}' to be shared")]
        CommandForbiddenInModulesConfiguration { submodule: BString, actual: BString },
        #[error("The 'update' field of submodule '{submodule}' was invalid: '{actual}'")]
        Invalid { submodule: BString, actual: BString },
    }
}

///
pub mod url {
    use bstr::BString;

    /// The error returned by [File::url()](crate::File::url).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The url of submodule '{submodule}' could not be parsed")]
        Parse {
            submodule: BString,
            source: gix_url::parse::Error,
        },
        #[error("The submodule '{submodule}' was missing its 'url' field or it was empty")]
        Missing { submodule: BString },
    }
}

///
pub mod path {
    use bstr::BString;

    /// The error returned by [File::path()](crate::File::path).
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("The path '{actual}' of submodule '{submodule}' needs to be relative")]
        Absolute { actual: BString, submodule: BString },
        #[error("The submodule '{submodule}' was missing its 'path' field or it was empty")]
        Missing { submodule: BString },
        #[error("The path '{actual}' would lead outside of the repository worktree")]
        OutsideOfWorktree { actual: BString, submodule: BString },
    }
}
