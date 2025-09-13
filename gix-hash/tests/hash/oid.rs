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
