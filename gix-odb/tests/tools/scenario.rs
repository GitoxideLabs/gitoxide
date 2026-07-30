use gix_object::Find as _;
use gix_odb::Header as _;

use crate::Result;

pub(crate) fn assert_object(handle: &gix_odb::Handle, id: &gix_hash::oid) -> Result {
    let mut buffer = Vec::new();
    let object = handle.try_find(id, &mut buffer)?.expect("fixture object is available");
    assert_eq!(
        gix_object::compute_hash(id.kind(), object.kind, object.data)?,
        id,
        "the ODB returned bytes belonging to the requested object"
    );
    let header = handle.try_header(id)?.expect("the same object has a header");
    assert_eq!(header.kind(), object.kind, "header and object kinds agree");
    assert_eq!(header.size(), object.data.len() as u64, "header and object sizes agree");
    Ok(())
}
