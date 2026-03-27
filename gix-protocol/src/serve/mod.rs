mod ref_advertisement;
pub use ref_advertisement::{write_capabilities_v2, write_v1, write_v2_ls_refs};

///
pub mod upload_pack;

/// A reference to advertise to clients.
pub struct RefAdvertisement<'a> {
    /// The ref name, e.g. `refs/heads/main`.
    pub name: &'a [u8],
    /// The object ID the ref points to.
    pub object_id: &'a gix_hash::oid,
    /// The peeled object ID, if this is an annotated tag.
    pub peeled: Option<&'a gix_hash::oid>,
    /// The symref target, e.g. `refs/heads/main` for `HEAD`.
    pub symref_target: Option<&'a [u8]>,
}
