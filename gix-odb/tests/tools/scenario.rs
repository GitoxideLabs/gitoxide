use crate::Result;
use gix_object::Find as _;
use gix_odb::Header as _;

pub(crate) fn assert_object_once(handle: &gix_odb::Handle, id: &gix_hash::oid) -> Result {
    let mut buffer = Vec::new();
    let object = handle
        .try_find(id, &mut buffer)?
        .ok_or_else(|| std::io::Error::other(format!("fixture object {id} is available")))?;
    assert_eq!(
        gix_object::compute_hash(id.kind(), object.kind, object.data)?,
        id,
        "the ODB returned bytes belonging to the requested object"
    );
    let header = handle
        .try_header(id)?
        .ok_or_else(|| std::io::Error::other(format!("fixture object {id} has a header")))?;
    assert_eq!(header.kind(), object.kind, "header and object kinds agree");
    assert_eq!(header.size(), object.data.len() as u64, "header and object sizes agree");
    Ok(())
}
