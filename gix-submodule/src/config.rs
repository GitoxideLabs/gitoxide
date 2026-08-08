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
    pub fn new(boolean: Result<Option<bool>, gix_config::value::Error>) -> Result<Option<Self>, BString> {
        Ok(match boolean {
            Ok(Some(value)) => Some(if value {
                FetchRecurse::Always
            } else {
                FetchRecurse::Never
            }),
            Ok(None) => None,
            Err(err) => {
                if err.input != "on-demand" {
                    return Err(err.input);
                }
                Some(FetchRecurse::OnDemand)
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
#[derive(Debug)]
#[expect(missing_docs)]
pub struct Error {
    pub field: &'static str,
    pub submodule: BString,
    pub actual: BString,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The '{}' field of submodule '{}' was invalid: '{}'",
            self.field, self.submodule, self.actual
        )
    }
}

impl std::error::Error for Error {}

///
pub mod branch {
    use bstr::BString;

    /// The error returned by [File::branch()](crate::File::branch).
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub struct Error {
        pub submodule: BString,
        pub actual: BString,
        pub source: gix_refspec::parse::Error,
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "The value '{}' of the 'branch' field of submodule '{}' couldn't be turned into a valid fetch refspec",
                self.actual, self.submodule
            )
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.source)
        }
    }
}

///
pub mod update {
    use bstr::BString;

    /// The error returned by [File::update()](crate::File::update).
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        CommandForbiddenInModulesConfiguration { submodule: BString, actual: BString },
        Invalid { submodule: BString, actual: BString },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::CommandForbiddenInModulesConfiguration { submodule, actual } => write!(
                    f,
                    "The 'update' field of submodule '{submodule}' tried to set command '{actual}' to be shared"
                ),
                Error::Invalid { submodule, actual } => {
                    write!(
                        f,
                        "The 'update' field of submodule '{submodule}' was invalid: '{actual}'"
                    )
                }
            }
        }
    }

    impl std::error::Error for Error {}
}

///
pub mod url {
    use bstr::BString;

    /// The error returned by [File::url()](crate::File::url).
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Parse {
            submodule: BString,
            source: gix_url::parse::Error,
        },
        Missing {
            submodule: BString,
        },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Parse { submodule, .. } => {
                    write!(f, "The url of submodule '{submodule}' could not be parsed")
                }
                Error::Missing { submodule } => {
                    write!(
                        f,
                        "The submodule '{submodule}' was missing its 'url' field or it was empty"
                    )
                }
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Parse { source, .. } => Some(source),
                Error::Missing { .. } => None,
            }
        }
    }
}

///
pub mod path {
    use bstr::BString;

    /// The error returned by [File::path()](crate::File::path).
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Absolute { actual: BString, submodule: BString },
        Missing { submodule: BString },
        OutsideOfWorktree { actual: BString, submodule: BString },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Absolute { actual, submodule } => {
                    write!(f, "The path '{actual}' of submodule '{submodule}' needs to be relative")
                }
                Error::Missing { submodule } => {
                    write!(
                        f,
                        "The submodule '{submodule}' was missing its 'path' field or it was empty"
                    )
                }
                Error::OutsideOfWorktree { actual, .. } => {
                    write!(f, "The path '{actual}' would lead outside of the repository worktree")
                }
            }
        }
    }

    impl std::error::Error for Error {}
}
