use crate::{PartialNameRef, Reference, store};

mod error {
    use std::convert::Infallible;

    /// The error returned by [`crate::file::Store::find_loose()`].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("An error occurred while finding a reference in the loose file database")]
        Loose(#[from] crate::file::find::Error),
        #[error("The ref name or path is not a valid ref name")]
        RefnameValidation(#[from] crate::name::Error),
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
            handle::State::Loose { store: _, .. } => {
                todo!()
            }
        }
    }
}

mod existing {
    mod error {
        use std::path::PathBuf;

        /// The error returned by [file::Store::find_existing()][crate::file::Store::find_existing()].
        #[derive(Debug, thiserror::Error)]
        #[allow(missing_docs)]
        pub enum Error {
            #[error("An error occurred while finding a reference in the database")]
            Find(#[from] crate::store::find::Error),
            #[error("The ref partially named {name:?} could not be found")]
            NotFound { name: PathBuf },
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
