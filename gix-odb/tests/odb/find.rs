use gix_testtools::fixture_path;

fn db() -> gix_odb::Handle {
    gix_odb::at(fixture_path("objects"), gix_hash::Kind::Sha1).expect("valid object path")
}

use crate::hex_to_id;

fn can_find(db: impl gix_object::Find, hex_id: &str) {
    let mut buf = vec![];
    assert!(
        db.try_find(&hex_to_id(hex_id), &mut buf)
            .expect("no read error")
            .is_some()
    );
}

#[test]
fn loose_object() {
    can_find(db(), "37d4e6c5c48ba0d245164c4e10d5f41140cab980");
}

#[test]
fn pack_object() {
    let db = db();
    can_find(&db, "501b297447a8255d3533c6858bb692575cdefaa0"); // pack 11fd
    can_find(&db, "4dac9989f96bc5b5b1263b582c08f0c5f0b58542"); // pack a2bf
    can_find(&db, "dd25c539efbb0ab018caa4cda2d133285634e9b5"); // pack c043
}

#[test]
fn into_arc_restarts_the_decoded_pack_cache() -> gix_error::TestResult {
    use gix_odb::Header as _;
    use gix_pack::Find;

    let fixture = gix_testtools::scripted_fixture_read_only("make_pack_cache_conversion.sh")?;
    let handle = crate::odb_at(fixture.join("objects"))?
        .with_pack_cache(|| Box::new(gix_pack::cache::lru::StaticLinkedList::<8>::default()));
    let a_blob_id = gix_hash::ObjectId::from_hex(std::fs::read_to_string(fixture.join("a.oid"))?.trim().as_bytes())?;
    let b_blob_id = gix_hash::ObjectId::from_hex(std::fs::read_to_string(fixture.join("b.oid"))?.trim().as_bytes())?;
    for blob_id in [&a_blob_id, &b_blob_id] {
        let header = crate::odb_at(fixture.join("objects"))?.try_header(blob_id)?;
        assert!(
            matches!(header, Some(gix_odb::find::Header::Packed(header)) if header.num_deltas > 0),
            "both fixture objects must exercise the decoded delta cache"
        );
    }
    let mut buffer = Vec::new();
    let (object, a_location) = Find::try_find(&handle, &a_blob_id, &mut buffer)?.expect("the first blob is packed");
    assert_eq!(
        object.data,
        std::fs::read(fixture.join("a"))?,
        "the first lookup warms the pack cache"
    );
    let a_location = a_location.expect("the first blob has a pack location");

    let converted = handle.into_arc()?;
    assert!(
        converted.has_pack_cache(),
        "conversion preserves the configured cache policy"
    );
    let (object, b_location) = Find::try_find(&converted, &b_blob_id, &mut buffer)?.expect("the second blob is packed");
    assert_eq!(
        a_location.pack_offset,
        b_location.expect("the second blob has a pack location").pack_offset,
        "the packs have matching delta offsets but different decoded contents"
    );
    assert_eq!(
        object.data,
        std::fs::read(fixture.join("b"))?,
        "a fresh store cannot interpret the old store's decoded pack cache"
    );
    Ok(())
}
