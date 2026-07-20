use std::{io::Read, ops::Deref};

use gix_hash::ObjectId;
use gix_object::Kind;

use crate::store;

mod error {
    use crate::{loose, store};

    /// The error returned by the [dynamic Store's][crate::Store] [`Write`](gix_object::Write) implementation.
    #[derive(Debug)]
    #[allow(missing_docs)]
    pub enum Error {
        LoadIndex(store::load_index::Error),
        LooseWrite(loose::write::Error),
        Io(std::io::Error),
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::LoadIndex(err) => std::fmt::Display::fmt(err, f),
                Error::LooseWrite(err) => std::fmt::Display::fmt(err, f),
                Error::Io(err) => std::fmt::Display::fmt(err, f),
            }
        }
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                Error::LoadIndex(err) => err.source(),
                Error::LooseWrite(err) => err.source(),
                Error::Io(err) => err.source(),
            }
        }
    }

    impl From<store::load_index::Error> for Error {
        fn from(err: store::load_index::Error) -> Self {
            Error::LoadIndex(err)
        }
    }

    impl From<loose::write::Error> for Error {
        fn from(err: loose::write::Error) -> Self {
            Error::LooseWrite(err)
        }
    }

    impl From<std::io::Error> for Error {
        fn from(err: std::io::Error) -> Self {
            Error::Io(err)
        }
    }
}
pub use error::Error;

use crate::store_impls::dynamic;

impl<S> gix_object::Write for store::Handle<S>
where
    S: Deref<Target = dynamic::Store> + Clone,
{
    fn write_stream(&self, kind: Kind, size: u64, from: &mut dyn Read) -> Result<ObjectId, gix_object::write::Error> {
        let mut snapshot = self.snapshot.borrow_mut();
        Ok(match snapshot.loose_dbs.first() {
            Some(ldb) => ldb.write_stream(kind, size, from)?,
            None => {
                let new_snapshot = self
                    .store
                    .load_one_index(self.index_ctx(snapshot.marker))
                    .map_err(Box::new)?
                    .expect("there is always at least one ODB, and this code runs only once for initialization");
                *snapshot = new_snapshot;
                snapshot.loose_dbs[0].write_stream(kind, size, from)?
            }
        })
    }

    fn write_buf_with_known_id(
        &self,
        kind: Kind,
        from: &[u8],
        id: ObjectId,
    ) -> Result<ObjectId, gix_object::write::Error> {
        let mut snapshot = self.snapshot.borrow_mut();
        Ok(match snapshot.loose_dbs.first() {
            Some(ldb) => ldb.write_buf_with_known_id(kind, from, id)?,
            None => {
                let new_snapshot = self
                    .store
                    .load_one_index(self.index_ctx(snapshot.marker))
                    .map_err(Box::new)?
                    .expect("there is always at least one ODB, and this code runs only once for initialization");
                *snapshot = new_snapshot;
                snapshot.loose_dbs[0].write_buf_with_known_id(kind, from, id)?
            }
        })
    }

    fn write_stream_with_known_id(
        &self,
        kind: Kind,
        size: u64,
        from: &mut dyn Read,
        id: ObjectId,
    ) -> Result<ObjectId, gix_object::write::Error> {
        let mut snapshot = self.snapshot.borrow_mut();
        Ok(match snapshot.loose_dbs.first() {
            Some(ldb) => ldb.write_stream_with_known_id(kind, size, from, id)?,
            None => {
                let new_snapshot = self
                    .store
                    .load_one_index(self.index_ctx(snapshot.marker))
                    .map_err(Box::new)?
                    .expect("there is always at least one ODB, and this code runs only once for initialization");
                *snapshot = new_snapshot;
                snapshot.loose_dbs[0].write_stream_with_known_id(kind, size, from, id)?
            }
        })
    }
}
