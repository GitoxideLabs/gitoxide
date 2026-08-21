mod to_prefix {
    #[test]
    fn turns_byte_ranges_into_standalone_prefixes() {
        for id_hex in [
            "0123456789abcdef123456789abcdef123456789",
            "0123456789abcdef123456789abcdef1234567890123456789abcdef12345678",
        ] {
            let id = gix_hash::ObjectId::from_hex(id_hex.as_bytes()).expect("valid input");
            let prefix = id.to_prefix(1..3);
            assert_eq!(prefix.hex_len(), 4);
            assert_eq!(prefix.to_string(), &id_hex[2..6]);

            let empty = id.to_prefix(0..0);
            assert_eq!(empty.hex_len(), 0);
            assert_eq!(empty.to_string(), "");
        }
    }
}

mod to_hex_with_len {
    #[test]
    fn display_entire_range_sha1() {
        let id_hex = "0123456789abcdef123456789abcdef123456789";
        let id = gix_hash::ObjectId::from_hex(id_hex.as_bytes()).expect("valid input");
        for len in 0..=40 {
            assert_eq!(id.to_hex_with_len(len).to_string(), id_hex[..len]);
        }
        assert_eq!(
            id.to_hex_with_len(120).to_string(),
            id_hex,
            "values that are too long are truncated"
        );
    }
}

#[test]
fn is_null() {
    assert!(gix_hash::Kind::Sha1.null().is_null());
    assert!(gix_hash::Kind::Sha1.null().as_ref().is_null());
}

#[test]
#[cfg(feature = "sha256")]
fn is_null_sha256() {
    assert!(gix_hash::Kind::Sha256.null().is_null());
    assert!(gix_hash::Kind::Sha256.null().as_ref().is_null());
}

#[test]
fn is_empty_blob() {
    let empty_blob = gix_hash::ObjectId::empty_blob(gix_hash::Kind::Sha1);
    assert!(empty_blob.is_empty_blob());
    assert!(empty_blob.as_ref().is_empty_blob());

    let non_empty = gix_hash::Kind::Sha1.null();
    assert!(!non_empty.is_empty_blob());
    assert!(!non_empty.as_ref().is_empty_blob());
}

#[test]
fn is_empty_tree() {
    let empty_tree = gix_hash::ObjectId::empty_tree(gix_hash::Kind::Sha1);
    assert!(empty_tree.is_empty_tree());
    assert!(empty_tree.as_ref().is_empty_tree());

    let non_empty = gix_hash::Kind::Sha1.null();
    assert!(!non_empty.is_empty_tree());
    assert!(!non_empty.as_ref().is_empty_tree());
}
