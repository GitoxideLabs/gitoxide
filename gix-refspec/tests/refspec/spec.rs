mod prefix {
    use gix_refspec::{parse::Operation, RefSpec};

    #[test]
    fn head_is_specifically_known() {
        assert_eq!(parse("HEAD").to_ref().prefix().unwrap(), "HEAD");
    }

    #[test]
    fn partial_refs_have_no_prefix() {
        assert_eq!(parse("main").to_ref().prefix(), None);
    }

    #[test]
    fn negative_specs_have_no_prefix() {
        assert_eq!(parse("^refs/heads/main").to_ref().prefix(), None);
    }

    #[test]
    fn short_absolute_refs_have_no_prefix() {
        assert_eq!(parse("refs/short").to_ref().prefix().unwrap(), "refs/short");
    }

    #[test]
    fn push_specs_use_the_destination() {
        assert_eq!(
            gix_refspec::parse("refs/local/main:refs/remote/main".into(), Operation::Push)
                .unwrap()
                .prefix()
                .unwrap(),
            "refs/remote/main"
        );
    }

    #[test]
    fn full_names_have_a_prefix() {
        assert_eq!(parse("refs/heads/main").to_ref().prefix().unwrap(), "refs/heads/main");
        assert_eq!(parse("refs/foo/bar").to_ref().prefix().unwrap(), "refs/foo/bar");
        assert_eq!(
            parse("refs/heads/*:refs/remotes/origin/*").to_ref().prefix().unwrap(),
            "refs/heads/"
        );
    }

    #[test]
    fn strange_glob_patterns_have_no_prefix() {
        assert_eq!(parse("refs/*/main:refs/*/main").to_ref().prefix(), None);
    }

    #[test]
    fn object_names_have_no_prefix() {
        assert_eq!(
            parse("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391").to_ref().prefix(),
            None
        );
    }

    fn parse(spec: &str) -> RefSpec {
        gix_refspec::parse(spec.into(), Operation::Fetch).unwrap().to_owned()
    }
}

mod expand_prefixes {
    use gix_refspec::parse::Operation;

    #[test]
    fn head_is_specifically_known() {
        assert_eq!(parse("HEAD"), ["HEAD"]);
    }

    #[test]
    fn partial_refs_have_many_prefixes() {
        assert_eq!(
            parse("main"),
            [
                "main",
                "refs/main",
                "refs/tags/main",
                "refs/heads/main",
                "refs/remotes/main",
                "refs/remotes/main/HEAD"
            ]
        );
    }

    #[test]
    fn negative_specs_have_no_prefix() {
        assert_eq!(parse("^refs/heads/main").len(), 0);
    }

    #[test]
    fn short_absolute_refs_expand_to_themselves() {
        assert_eq!(parse("refs/short"), ["refs/short"]);
    }

    #[test]
    fn full_names_expand_to_their_prefix() {
        assert_eq!(parse("refs/heads/main"), ["refs/heads/main"]);
        assert_eq!(parse("refs/foo/bar"), ["refs/foo/bar"]);
        assert_eq!(parse("refs/heads/*:refs/remotes/origin/*"), ["refs/heads/"]);
    }

    #[test]
    fn push_specs_use_the_destination() {
        let mut out = Vec::new();
        gix_refspec::parse("refs/local/main:refs/remote/main".into(), Operation::Push)
            .unwrap()
            .expand_prefixes(&mut out);
        assert_eq!(out, ["refs/remote/main"]);
    }

    #[test]
    fn strange_glob_patterns_expand_to_nothing() {
        assert_eq!(parse("refs/*/main:refs/*/main").len(), 0);
    }

    #[test]
    fn object_names_expand_to_nothing() {
        assert_eq!(parse("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391").len(), 0);
    }

    fn parse(spec: &str) -> Vec<String> {
        let mut out = Vec::new();
        gix_refspec::parse(spec.into(), Operation::Fetch)
            .unwrap()
            .to_owned()
            .to_ref()
            .expand_prefixes(&mut out);
        out.into_iter().map(|b| b.to_string()).collect()
    }
}
