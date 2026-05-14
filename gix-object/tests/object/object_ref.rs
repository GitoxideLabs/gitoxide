mod from_loose {
    use gix_object::ObjectRef;

    #[test]
    fn shorter_than_advertised() {
        assert_eq!(
            ObjectRef::from_loose(b"tree 1000\x00", gix_testtools::object_hash(),)
                .unwrap_err()
                .to_string(),
            "object data was shorter than its size declared in the header"
        );
    }
}
