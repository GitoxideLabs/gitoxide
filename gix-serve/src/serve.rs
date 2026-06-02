use std::io::{self, Read, Write};

use gix_hash::{oid, ObjectId};
use gix_pack::Find;
use gix_protocol::serve::{
    upload_pack::{self, serve_upload_pack_v1, serve_upload_pack_v2},
    RefAdvertisement,
};
use gix_ref::file::Store;
use gix_transport::{server::blocking_io::connection::Connection, Protocol};

use crate::{pack::generate_pack, refs::collect_refs, AdvertisableRef};

impl AdvertisableRef {
    /// Borrow as a `RefAdvertisement` for the protocol layer.
    pub fn as_advertisement(&self) -> RefAdvertisement<'_> {
        RefAdvertisement {
            name: &self.name,
            object_id: &self.object_id,
            peeled: self.peeled.as_deref(),
            symref_target: self.symref_target.as_ref().map(AsRef::as_ref),
        }
    }
}

/// Errors from serving upload-pack.
#[derive(Debug, thiserror::Error)]
#[allow(missing_docs)]
pub enum Error {
    #[error(transparent)]
    Refs(#[from] crate::Error),
    #[error(transparent)]
    Protocol(#[from] upload_pack::Error),
    #[error(transparent)]
    Pack(#[from] crate::pack::Error),
}

/// Serve an upload-pack session over a connection.
pub fn serve_upload_pack<R: Read, W: Write, F: Find + Send + Clone + 'static>(
    ref_store: &Store,
    db: F,
    connection: &mut Connection<R, W>,
    protocol: Protocol,
) -> Result<(), Error> {
    let owned_refs = collect_refs(ref_store)?;
    let refs: Vec<_> = owned_refs.iter().map(AdvertisableRef::as_advertisement).collect();

    let has_object = |oid: &oid| db.contains(oid);

    let generate_pack = |wants: &[ObjectId], haves: &[ObjectId], out: &mut dyn Write| {
        generate_pack(db.clone(), wants, haves, out).map_err(io::Error::other)
    };

    match protocol {
        Protocol::V1 | Protocol::V0 => {
            serve_upload_pack_v1(connection, &refs, has_object, generate_pack, &[])?;
        }
        Protocol::V2 => {
            serve_upload_pack_v2(
                connection,
                &refs,
                has_object,
                generate_pack,
                &[("fetch", None), ("ls-refs", None)],
            )?;
        }
    }

    Ok(())
}
