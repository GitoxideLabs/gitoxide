use gix::prelude::ObjectIdExt;

use crate::{
    revision::spec::from_bytes::{parse_spec_no_baseline, repo},
    util::hex_to_id_sha1_only,
};

mod with_known_revision {
    use gix::revision::Spec;

    use super::*;
    use crate::revision::spec::from_bytes::parse_spec;

    #[test]
    #[cfg(not(feature = "revparse-regex"))]
    fn contained_string_matches_in_unanchored_regex_and_disambiguates_automatically() {
        let repo = repo("ambiguous_blob_tree_commit").unwrap();
        let expected = Spec::from_id(hex_to_id_sha1_only("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo));

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
        let expected = Spec::from_id(hex_to_id_sha1_only("0000000000e4f9fbd19cf1e932319e5ad0d1d00b").attach(&repo));

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

mod empty_pattern {
    use super::*;
    use crate::revision::spec::from_bytes::{parse_spec, repo};

    #[test]
    fn matches_everything_and_peels_to_a_commit() -> crate::Result {
        let repo = repo("complex_graph")?;

        assert_eq!(
            parse_spec("@^{/}", &repo)?,
            parse_spec_no_baseline("@^{commit}", &repo)?,
            "an empty pattern matches the first commit reachable from the anchor, i.e. the anchor peeled to a commit"
        );
        assert_eq!(
            parse_spec("b-tag^{/}", &repo)?,
            parse_spec("b", &repo)?,
            "the annotated tag is peeled to its commit first, just like Git"
        );

        let err = parse_spec("@^{/!-}", &repo).unwrap_err();
        let cause = err.probable_cause().to_string();
        assert!(
            cause.starts_with("None of") && cause.contains("matched"),
            "a negated empty pattern matches nothing and fails like Git: {cause}"
        );
        Ok(())
    }
}

mod find_youngest_matching_commit {
    use gix::revision::Spec;

    use super::*;
    use crate::revision::spec::from_bytes::{parse_spec, repo_with_correct_pattern_revision_order};

    #[test]
    #[cfg(not(feature = "revparse-regex"))]
    fn contained_string_matches() {
        let Some(repo) = repo_with_correct_pattern_revision_order("complex_graph").unwrap() else {
            return;
        };

        assert_eq!(
            parse_spec(":/message", &repo).unwrap(),
            Spec::from_id(hex_to_id_sha1_only("ef80b4b77b167f326351c93284dc0eb00dd54ff4").attach(&repo))
        );

        assert_eq!(
            parse_spec("@^{/!-B}", &repo).unwrap(),
            Spec::from_id(hex_to_id_sha1_only("55e825ebe8fd2ff78cad3826afb696b96b576a7e").attach(&repo)),
            "negations work as well"
        );

        assert_eq!(
            parse_spec(":/!-message", &repo).unwrap(),
            Spec::from_id(hex_to_id_sha1_only("55e825ebe8fd2ff78cad3826afb696b96b576a7e").attach(&repo))
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
        let Some(repo) = repo_with_correct_pattern_revision_order("complex_graph").unwrap() else {
            return;
        };

        assert_eq!(
            parse_spec(":/mes.age", &repo).unwrap(),
            Spec::from_id(hex_to_id_sha1_only("ef80b4b77b167f326351c93284dc0eb00dd54ff4").attach(&repo))
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
            Spec::from_id(hex_to_id_sha1_only("55e825ebe8fd2ff78cad3826afb696b96b576a7e").attach(&repo))
        );

        assert_eq!(
            parse_spec("@^{/!-B}", &repo).unwrap(),
            Spec::from_id(hex_to_id_sha1_only("55e825ebe8fd2ff78cad3826afb696b96b576a7e").attach(&repo)),
            "negations work as well"
        );
    }
}
