mod graph {
    mod commit {
        use gix_testtools::size_ok;

        #[test]
        fn size_of_commit() {
            let actual = std::mem::size_of::<gix_revwalk::graph::Commit<()>>();
            let expected = 64;
            assert!(
                size_ok(actual, expected),
                "We might see quite a lot of these, so they shouldn't grow unexpectedly: {actual} <~ {expected}"
            );
        }
    }
}
