use crate::{db, hex_to_id};

fn find_header(db: impl gix_odb::Header, hex_id: &str) -> gix_odb::find::Header {
    db.try_header(&hex_to_id(hex_id))
        .expect("no read error")
        .expect("object exists")
}

#[test]
fn loose_object() {
    find_header(db(), "37d4e6c5c48ba0d245164c4e10d5f41140cab980");
}

#[test]
fn pack_object() {
    let db = db();
    assert_eq!(
        find_header(&db, "501b297447a8255d3533c6858bb692575cdefaa0"), // pack 11fd
        gix_odb::find::Header::Packed(gix_pack::data::decode::header::Outcome {
            kind: gix_object::Kind::Commit,
            object_size: 225,
            num_deltas: 0,
        })
    );
    assert_eq!(
        find_header(&db, "4dac9989f96bc5b5b1263b582c08f0c5f0b58542"), // pack a2bf
        gix_odb::find::Header::Packed(gix_pack::data::decode::header::Outcome {
            kind: gix_object::Kind::Tree,
            object_size: 34,
            num_deltas: 0,
        })
    );
    assert_eq!(
        find_header(&db, "dd25c539efbb0ab018caa4cda2d133285634e9b5"), // pack c043
        gix_odb::find::Header::Packed(gix_pack::data::decode::header::Outcome {
            kind: gix_object::Kind::Blob,
            object_size: 860,
            num_deltas: 0,
        })
    );
}
