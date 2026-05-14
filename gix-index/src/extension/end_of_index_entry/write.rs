use crate::extension::{Signature, end_of_index_entry::SIGNATURE};

/// Write this extension to out and generate a hash of `object_hash` over all `prior_extensions` which are specified as `(signature, size)`
/// pair. `one_past_entries` is the offset to the first byte past the entries, which is also the first byte of the signature of the
/// first extension in `prior_extensions`. Note that `prior_extensions` must have been written prior to this one, as the name suggests,
/// allowing this extension to be the last one in the index file.
///
/// Even if there are no `prior_extensions`, this extension will be written unconditionally.
pub fn write_to(
    mut out: impl std::io::Write,
    object_hash: gix_hash::Kind,
    offset_to_extensions: u32,
    prior_extensions: impl IntoIterator<Item = (Signature, u32)>,
) -> Result<(), gix_hash::io::Error> {
    out.write_all(&SIGNATURE)?;
    let extension_size: u32 = 4 + object_hash.len_in_bytes() as u32;
    out.write_all(&extension_size.to_be_bytes())?;

    out.write_all(&offset_to_extensions.to_be_bytes())?;

    let mut hasher = gix_hash::hasher(object_hash);
    for (signature, size) in prior_extensions {
        hasher.update(&signature);
        hasher.update(&size.to_be_bytes());
    }
    out.write_all(hasher.try_finalize()?.as_slice())?;

    Ok(())
}
