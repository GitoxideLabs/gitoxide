use crate::{PartialNameRef, Reference, store};

mod error {
    use std::convert::Infallible;

    /// The error returned by [`crate::file::Store::find_loose()`].
    #[derive(Debug)]
    #[expect(missing_docs)]
    pub enum Error {
        Loose(crate::file::find::Error),
        RefnameValidation(crate::name::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::Loose(_) => {
                    f.write_str("An error occurred while finding a reference in the loose file database")
                }
                Error::RefnameValidation(_) => f.write_str("The ref name or path is not a valid ref name"),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::Loose(err) => Some(err),
                Error::RefnameValidation(err) => Some(err),
            }
        }
    }

    impl From<crate::file::find::Error> for Error {
        fn from(err: crate::file::find::Error) -> Self {
            Error::Loose(err)
        }
    }

    impl From<crate::name::Error> for Error {
        fn from(err: crate::name::Error) -> Self {
            Error::RefnameValidation(err)
        }
    }

    impl From<Infallible> for Error {
        fn from(_: Infallible) -> Self {
            unreachable!("this impl is needed to allow passing a known valid partial path as parameter")
        }
    }
}

pub use error::Error;

use crate::store::handle;

impl store::Handle {
    /// TODO: actually implement this with handling of the packed buffer.
    pub fn try_find<'a, Name, E>(&self, partial: Name) -> Result<Option<Reference>, Error>
    where
        Name: TryInto<&'a PartialNameRef, Error = E>,
        Error: From<E>,
    {
        let _name = partial.try_into()?;
        match &self.state {
            handle::State::Loose { .. } => {
                todo!()
            }
        }
    }
}

mod existing {
    mod error {
        use std::path::PathBuf;

        /// The error returned by [file::Store::find_existing()][crate::file::Store::find_existing()].
        #[derive(Debug)]
        pub enum Error {
            Find(crate::store::find::Error),
            NotFound { name: PathBuf },
        }

        impl std::fmt::Display for Error {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Error::Find(_) => f.write_str("An error occurred while finding a reference in the database"),
                    #[allow(clippy::unnecessary_debug_formatting)]
                    // `{:?}` of a `Path` is what `thiserror` generated; keep the rendered text identical.
                    Error::NotFound { name } => write!(f, "The ref partially named {name:?} could not be found"),
                }
            }
        }

        impl std::error::Error for Error {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                match self {
                    Error::Find(err) => Some(err),
                    Error::NotFound { .. } => None,
                }
            }
        }

        impl From<crate::store::find::Error> for Error {
            fn from(err: crate::store::find::Error) -> Self {
                Error::Find(err)
            }
        }
    }

    pub use error::Error;

    use crate::{PartialNameRef, Reference, store};

    impl store::Handle {
        /// Similar to [`crate::file::Store::find()`] but a non-existing ref is treated as error.
        pub fn find<'a, Name, E>(&self, _partial: Name) -> Result<Reference, Error>
        where
            Name: TryInto<&'a PartialNameRef, Error = E>,
            crate::name::Error: From<E>,
        {
            todo!()
            // match self.try_find(partial) {}
            // match self.find_one_with_verified_input(path.to_partial_path().as_ref(), packed) {
            //     Ok(Some(r)) => Ok(r),
            //     Ok(None) => Err(Error::NotFound(path.to_partial_path().into_owned())),
            //     Err(err) => Err(err.into()),
            // }
        }
    }
}
