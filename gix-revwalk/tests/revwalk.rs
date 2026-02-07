mod graph {
    mod commit {
        use gix_testtools::size_ok;

        #[test]
        fn size_of_commit() {
            let actual = std::mem::size_of::<gix_revwalk::graph::Commit<()>>();
            let sha1 = 48;
            let sha256_extra = 16;
            let expected = sha1 + sha256_extra;
            assert!(
                size_ok(actual, expected),
                "We might see quite a lot of these, so they shouldn't grow unexpectedly: {actual} <~ {expected}"
            );
        }
    }
}
