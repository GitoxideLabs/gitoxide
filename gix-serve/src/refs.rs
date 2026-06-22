use gix_ref::file::Store;

use crate::{AdvertisableRef, Error};

/// Collect all refs from a ref store for advertisement to clients.
pub fn collect_refs(store: &Store) -> Result<Vec<AdvertisableRef>, Error> {
    let mut ref_adverts = Vec::new();
    let platform = store.iter()?;

    for ref_res in platform.pseudo()?.chain(platform.all()?) {
        let reference = ref_res?;
        match reference.target {
            gix_ref::Target::Object(object_id) => {
                ref_adverts.push(AdvertisableRef {
                    name: reference.name.as_bstr().to_owned(),
                    object_id,
                    peeled: reference.peeled,
                    symref_target: None,
                });
            }
            gix_ref::Target::Symbolic(target_name) => {
                if let Some(resolved) = store.try_find(target_name.as_bstr())? {
                    if let Some(oid) = resolved.target.try_id() {
                        ref_adverts.push(AdvertisableRef {
                            name: reference.name.as_bstr().to_owned(),
                            object_id: oid.to_owned(),
                            peeled: reference.peeled,
                            symref_target: Some(target_name.as_bstr().to_owned()),
                        });
                    }
                }
            }
        }
    }
    Ok(ref_adverts)
}
