//! These are old tests of the now removed linked odb, but executed on the general store
//! to be sure we don't loose coverage. This might, however, be overlapping with much more thorough
//! tests o the general store itself, so they can possibly be removed at some point.

mod locate {
    use gix_object::Find;

    use crate::{db, hex_to_id};

    fn can_locate(db: &gix_odb::Handle, hex_id: &str) {
        let mut buf = vec![];
        assert!(
            db.try_find(&hex_to_id(hex_id), &mut buf)
                .expect("no read error")
                .is_some()
        );
    }

    #[test]
    fn loose_object() {
        can_locate(&db(), "37d4e6c5c48ba0d245164c4e10d5f41140cab980");
    }

    #[test]
    fn pack_object() {
        can_locate(&db(), "501b297447a8255d3533c6858bb692575cdefaa0"); // pack 11fd
        can_locate(&db(), "4dac9989f96bc5b5b1263b582c08f0c5f0b58542"); // pack a2bf
        can_locate(&db(), "dd25c539efbb0ab018caa4cda2d133285634e9b5"); // pack c043
    }
}
