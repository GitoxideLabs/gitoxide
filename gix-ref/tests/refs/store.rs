#[test]
#[cfg(feature = "parallel")]
fn is_send_and_sync() {
    pub fn store_at(name: &str) -> crate::Result<gix_ref::file::Store> {
        let path = crate::scripted_fixture_read_only(name)?;
        Ok(gix_ref::file::Store::at(
            path.join(".git"),
            gix_ref::store::init::Options {
                write_reflog: gix_ref::store::WriteReflog::Normal,
                object_hash: crate::fixture_hash_kind(),
                ..Default::default()
            },
        ))
    }

    pub fn store_with_packed_refs() -> crate::Result<gix_ref::file::Store> {
        store_at("make_packed_ref_repository.sh")
    }
    fn assert_type<T: Send + Sync>(_t: T) {}
    let store = store_with_packed_refs().unwrap();
    assert_type(&store);
    assert_type(store);
}
