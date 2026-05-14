use gix_testtools::fixture_path;

fn db() -> gix_odb::Handle {
    gix_odb::at(fixture_path("objects")).expect("valid object path")
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
