use crate::find;

/// A way to obtain object properties without fully decoding it.
pub trait Header {
    /// Try to read the header of the object associated with `id` or return `None` if it could not be found.
    fn try_header(&self, id: &gix_hash::oid) -> Result<Option<find::Header>, gix_object::find::Error>;
}

mod _impls {
    use std::{ops::Deref, rc::Rc, sync::Arc};

    use gix_hash::oid;

    use crate::find::Header;

    impl<T> crate::Header for &T
    where
        T: crate::Header,
    {
        fn try_header(&self, id: &oid) -> Result<Option<Header>, gix_object::find::Error> {
            (*self).try_header(id)
        }
    }

    impl<T> crate::Header for Rc<T>
    where
        T: crate::Header,
    {
        fn try_header(&self, id: &oid) -> Result<Option<Header>, gix_object::find::Error> {
            self.deref().try_header(id)
        }
    }

    impl<T> crate::Header for Arc<T>
    where
        T: crate::Header,
    {
        fn try_header(&self, id: &oid) -> Result<Option<Header>, gix_object::find::Error> {
            self.deref().try_header(id)
        }
    }
}

mod ext {
    use gix_error::{ErrorExt, NotFoundError};

    use crate::find;
    /// An extension trait with convenience functions.
    pub trait HeaderExt: super::Header {
        /// Like [`try_header(…)`][super::Header::try_header()], but flattens the `Result<Option<_>>` into a single `Result` making a non-existing object an error.
        fn header(&self, id: impl AsRef<gix_hash::oid>) -> Result<find::Header, gix_object::find::existing::Error> {
            let id = id.as_ref();
            self.try_header(id)?
                .ok_or_else(|| NotFoundError::new(format!("An object with id {id} could not be found")).raise_erased())
        }
    }

    impl<T: super::Header> HeaderExt for T {}
}
pub use ext::HeaderExt;
