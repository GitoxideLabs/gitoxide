///
pub mod to_id {
    use gix_object::bstr::BString;

    /// The error returned by [`crate::file::ReferenceExt::peel_to_id()`].
    // TODO(review): this implementation hand-preserves `#[error(transparent)]` semantics for
    //                `FollowToObject`: `Display` passes the formatter through and `source()`
    //                forwards to the inner error's source, exactly like the `thiserror`-generated
    //                code did.
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        FollowToObject(super::to_object::Error),
        Find(gix_object::find::Error),
        NotFound { oid: gix_hash::ObjectId, name: BString },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::FollowToObject(err) => std::fmt::Display::fmt(err, f),
                Error::Find(_) => {
                    f.write_str("An error occurred when trying to resolve an object a reference points to")
                }
                Error::NotFound { oid, name } => {
                    write!(f, "Object {oid} as referred to by {name:?} could not be found")
                }
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::FollowToObject(err) => err.source(),
                Error::Find(err) => Some(&**err),
                Error::NotFound { .. } => None,
            }
        }
    }

    impl From<super::to_object::Error> for Error {
        fn from(err: super::to_object::Error) -> Self {
            Error::FollowToObject(err)
        }
    }

    impl From<gix_object::find::Error> for Error {
        fn from(err: gix_object::find::Error) -> Self {
            Error::Find(err)
        }
    }
}

///
pub mod to_object {
    use std::path::PathBuf;

    use crate::file;

    /// The error returned by [`file::ReferenceExt::follow_to_object_packed()`].
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Follow(file::find::existing::Error),
        Cycle { start_absolute: PathBuf },
        DepthLimitExceeded { max_depth: usize },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Follow(_) => f.write_str("Could not follow a single level of a symbolic reference"),
                #[allow(clippy::unnecessary_debug_formatting)]
                // `{:?}` of a `Path` is what `thiserror` generated; keep the rendered text identical.
                Error::Cycle { start_absolute } => write!(
                    f,
                    "Aborting due to reference cycle with first seen path being {start_absolute:?}"
                ),
                Error::DepthLimitExceeded { max_depth } => {
                    write!(f, "Refusing to follow more than {max_depth} levels of indirection")
                }
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Follow(err) => Some(err),
                Error::Cycle { .. } | Error::DepthLimitExceeded { .. } => None,
            }
        }
    }

    impl From<file::find::existing::Error> for Error {
        fn from(err: file::find::existing::Error) -> Self {
            Error::Follow(err)
        }
    }
}
