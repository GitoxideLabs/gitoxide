use gix::prelude::ObjectIdExt;

use crate::{
    revision::spec::from_bytes::{parse_spec_no_baseline, repo},
    util::hex_to_id,
};

mod with_known_revision {
    use gix::revision::Spec;

    use super::*;
    use crate::revision::spec::from_bytes::parse_spec;

    #[test]
    #[cfg(not(feature = "revparse-regex"))]
    fn contained_string_matches_in_unanchored_regex_and_disambiguates_automatically() {
        let repo = repo("ambiguous_blob_tree_commit").unwrap();
        let expected = Spec::from_id(hex_to_id("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo));

        assert_eq!(parse_spec("0000000000^{/x}", &repo).unwrap(), expected);
        assert_eq!(parse_spec("@^{/x}", &repo).unwrap(), expected, "ref names are resolved");

        let err = parse_spec_no_baseline("@^{/.*x}", &repo).unwrap_err();
        insta::assert_debug_snapshot!(err, @r#"
        Delegate couldn't find '.*x' (negated: false)
        |
        └─ None of 1 commits from 0000000000e matched text ".*x"
        "#);
        assert_eq!(
            err.probable_cause().to_string(),
            "None of 1 commits from 0000000000e matched text \".*x\"",
            "regexes are not actually available for us, but git could do that"
        );
    }

    #[test]
    #[cfg(feature = "revparse-regex")]
    fn contained_string_matches_in_unanchored_regex_and_disambiguates_automatically() {
        let repo = repo("ambiguous_blob_tree_commit").unwrap();
        let expected = Spec::from_id(hex_to_id("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo));

        assert_eq!(
            parse_spec("0000000000^{/x}", &repo).unwrap(),
            expected,
            "search is unanchored by default"
        );
        assert_eq!(
            parse_spec("@^{/x}", &repo).unwrap(),
            expected,
            "ref names are resolved as well"
        );

        assert_eq!(
            parse_spec("@^{/^.*x}", &repo).unwrap(),
            expected,
            "we can use real regexes here"
        );
        let err = parse_spec_no_baseline("@^{/^x}", &repo).unwrap_err();
        insta::assert_debug_snapshot!(err, @r#"
        Delegate couldn't find '^x' (negated: false)
        |
        └─ None of 1 commits from 0000000000e matched regex "^x"
        "#);
        assert_eq!(
            err.probable_cause().to_string(),
            "None of 1 commits from 0000000000e matched regex \"^x\"",
        );
    }
}

mod find_youngest_matching_commit {
    use gix::revision::Spec;

    use super::*;
    use crate::revision::spec::from_bytes::parse_spec;

    #[test]
    #[cfg(not(feature = "revparse-regex"))]
    fn contained_string_matches() {
        let repo = repo("complex_graph").unwrap();

        assert_eq!(
            parse_spec(":/message", &repo).unwrap(),
            Spec::from_id(hex_to_id("ef80b4b77b167f326351c93284dc0eb00dd54ff4").attach(&repo))
        );

        assert_eq!(
            parse_spec("@^{/!-B}", &repo).unwrap(),
            Spec::from_id(hex_to_id("55e825ebe8fd2ff78cad3826afb696b96b576a7e").attach(&repo)),
            "negations work as well"
        );

        assert_eq!(
            parse_spec(":/!-message", &repo).unwrap(),
            Spec::from_id(hex_to_id("55e825ebe8fd2ff78cad3826afb696b96b576a7e").attach(&repo))
        );

        let err = parse_spec_no_baseline(":/messa.e", &repo).unwrap_err();
        insta::assert_debug_snapshot!(err, @r#"
        Delegate couldn't find 'messa.e' (negated: false)
        |
        └─ None of 10 commits reached from all references matched text "messa.e"
        "#);
        assert_eq!(
            err.probable_cause().to_string(),
            "None of 10 commits reached from all references matched text \"messa.e\"",
            "regex definitely don't work as it's not compiled in"
        );
    }

    #[test]
    #[cfg(feature = "revparse-regex")]
    fn regex_matches() {
        let repo = repo("complex_graph").unwrap();

        assert_eq!(
            parse_spec(":/mes.age", &repo).unwrap(),
            Spec::from_id(hex_to_id("ef80b4b77b167f326351c93284dc0eb00dd54ff4").attach(&repo))
        );

        let err = parse_spec(":/not there", &repo).unwrap_err();
        insta::assert_debug_snapshot!(err, @r#"
        Delegate couldn't find 'not there' (negated: false)
        |
        └─ None of 10 commits reached from all references matched regex "not there"
        "#);
        assert_eq!(
            err.probable_cause().to_string(),
            "None of 10 commits reached from all references matched regex \"not there\""
        );

        assert_eq!(
            parse_spec(":/!-message", &repo).unwrap(),
            Spec::from_id(hex_to_id("55e825ebe8fd2ff78cad3826afb696b96b576a7e").attach(&repo))
        );

        assert_eq!(
            parse_spec("@^{/!-B}", &repo).unwrap(),
            Spec::from_id(hex_to_id("55e825ebe8fd2ff78cad3826afb696b96b576a7e").attach(&repo)),
            "negations work as well"
        );
    }
}
