use std::{io::Read, ops::Deref};

use gix_error::ExnResult;
use gix_hash::ObjectId;
use gix_object::Kind;

use crate::store;

use crate::store_impls::dynamic;

impl<S> gix_object::Write for store::Handle<S>
where
    S: Deref<Target = dynamic::Store> + Clone,
{
    fn write_stream(&self, kind: Kind, size: u64, from: &mut dyn Read) -> ExnResult<ObjectId> {
        let mut snapshot = self.snapshot.borrow_mut();
        match snapshot.loose_dbs.first() {
            Some(ldb) => ldb.write_stream(kind, size, from),
            None => {
                let new_snapshot = self
                    .store
                    .load_one_index(self.index_ctx(snapshot.marker))?
                    .expect("there is always at least one ODB, and this code runs only once for initialization");
                *snapshot = new_snapshot;
                snapshot.loose_dbs[0].write_stream(kind, size, from)
            }
        }
    }

    fn write_buf_with_known_id(&self, kind: Kind, from: &[u8], id: ObjectId) -> ExnResult<ObjectId> {
        let mut snapshot = self.snapshot.borrow_mut();
        match snapshot.loose_dbs.first() {
            Some(ldb) => ldb.write_buf_with_known_id(kind, from, id),
            None => {
                let new_snapshot = self
                    .store
                    .load_one_index(self.index_ctx(snapshot.marker))?
                    .expect("there is always at least one ODB, and this code runs only once for initialization");
                *snapshot = new_snapshot;
                snapshot.loose_dbs[0].write_buf_with_known_id(kind, from, id)
            }
        }
    }

    fn write_stream_with_known_id(
        &self,
        kind: Kind,
        size: u64,
        from: &mut dyn Read,
        id: ObjectId,
    ) -> ExnResult<ObjectId> {
        let mut snapshot = self.snapshot.borrow_mut();
        match snapshot.loose_dbs.first() {
            Some(ldb) => ldb.write_stream_with_known_id(kind, size, from, id),
            None => {
                let new_snapshot = self
                    .store
                    .load_one_index(self.index_ctx(snapshot.marker))?
                    .expect("there is always at least one ODB, and this code runs only once for initialization");
                *snapshot = new_snapshot;
                snapshot.loose_dbs[0].write_stream_with_known_id(kind, size, from, id)
            }
        }
    }
}
