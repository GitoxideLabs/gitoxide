mod single {
    use crate::matching::baseline;

    fn test_hashes() -> (String, String) {
        let annotated_tag = match gix_testtools::hash_kind_from_env() {
            gix_hash::Kind::Sha1 => "78b1c1be9421b33a49a7a8176d93eeeafa112da1",
            gix_hash::Kind::Sha256 => "b071221ea854da2958fba3a37527ca5cf32c4ebcd71ab0b68b6b8f10f04e93ad",
            _ => unimplemented!(),
        };
        let initial_commit = match gix_testtools::hash_kind_from_env() {
            gix_hash::Kind::Sha1 => "9d2fab1a0ba3585d0bc50922bfdd04ebb59361df",
            gix_hash::Kind::Sha256 => "ac050883b75422e0d03bfee760c591b292cbc10cee8ad934480ea5fb2ebc44fe",
            _ => unimplemented!(),
        };

        (annotated_tag.into(), initial_commit.into())
    }

    #[test]
    fn fetch_only() {
        let (annotated_tag, initial_commit) = test_hashes();

        baseline::agrees_with_fetch_specs(Some("refs/heads/main"));
        baseline::agrees_with_fetch_specs(Some("heads/main"));
        baseline::agrees_with_fetch_specs(Some("main"));
        baseline::agrees_with_fetch_specs(Some("v0.0-f1"));
        baseline::agrees_with_fetch_specs(Some("tags/v0.0-f2"));
        baseline::of_objects_always_matches_if_the_server_has_the_object(Some(annotated_tag.as_ref()));
        baseline::of_objects_always_matches_if_the_server_has_the_object(Some(initial_commit.as_ref()));
    }

    #[test]
    fn fetch_and_update() {
        let (annotated_tag, initial_commit) = test_hashes();

        baseline::of_objects_with_destinations_are_written_into_given_local_branches(
            Some(format!("{annotated_tag}:special").as_ref()),
            [format!("{annotated_tag}:refs/heads/special").as_ref()],
        );
        baseline::of_objects_with_destinations_are_written_into_given_local_branches(
            Some(format!("{annotated_tag}:1111111111111111111111111111111111111111").as_ref()),
            [format!("{annotated_tag}:refs/heads/1111111111111111111111111111111111111111").as_ref()],
        );
        baseline::of_objects_with_destinations_are_written_into_given_local_branches(
            Some(format!("{initial_commit}:tags/special").as_ref()),
            [format!("{initial_commit}:refs/tags/special").as_ref()],
        );
        baseline::of_objects_with_destinations_are_written_into_given_local_branches(
            Some(format!("{initial_commit}:refs/tags/special").as_ref()),
            [format!("{initial_commit}:refs/tags/special").as_ref()],
        );

        baseline::agrees_but_observable_refs_are_vague(Some("f1:origin/f1"), ["refs/heads/f1:refs/heads/origin/f1"]);
        baseline::agrees_but_observable_refs_are_vague(
            Some("f1:remotes/origin/f1"),
            ["refs/heads/f1:refs/remotes/origin/f1"],
        );
        baseline::agrees_but_observable_refs_are_vague(Some("f1:notes/f1"), ["refs/heads/f1:refs/heads/notes/f1"]);
        baseline::agrees_with_fetch_specs(Some("+refs/heads/*:refs/remotes/origin/*"));
        baseline::agrees_with_fetch_specs(Some("refs/heads/f*:refs/remotes/origin/a*"));
        baseline::agrees_with_fetch_specs(Some("refs/heads/*1:refs/remotes/origin/*1"));
    }
}

mod multiple {
    use gix_refspec::{
        match_group::validate::Fix,
        parse::{Error, Operation},
    };

    use crate::matching::baseline;

    #[test]
    fn fetch_only() {
        baseline::agrees_with_fetch_specs(["main", "f1"]);
        baseline::agrees_with_fetch_specs(["heads/main", "heads/f1"]);
        baseline::agrees_with_fetch_specs(["refs/heads/main", "refs/heads/f1"]);
        baseline::agrees_with_fetch_specs(["heads/f1", "f2", "refs/heads/f3", "heads/main"]);
        baseline::agrees_with_fetch_specs(["f*:a*", "refs/heads/main"]);
        baseline::agrees_with_fetch_specs([
            "refs/tags/*:refs/remotes/origin/*",
            "refs/heads/*:refs/remotes/origin/*",
        ]);
        baseline::agrees_with_fetch_specs(["refs/tags/*:refs/tags/*"]);
    }

    #[test]
    fn fetch_and_update_and_negations() {
        baseline::invalid_specs_fail_to_parse_where_git_shows_surprising_behaviour(
            ["refs/heads/f*:refs/remotes/origin/a*", "^f1"],
            Error::NegativePartialName,
        );
        baseline::invalid_specs_fail_to_parse_where_git_shows_surprising_behaviour(
            ["heads/f2", "^refs/heads/f*:refs/remotes/origin/a*"],
            Error::NegativeWithDestination,
        );
        baseline::agrees_with_fetch_specs(["refs/heads/f*:refs/remotes/origin/a*", "^refs/heads/f1"]);
        baseline::invalid_specs_fail_to_parse_where_git_shows_surprising_behaviour(
            ["^heads/f2", "refs/heads/f*:refs/remotes/origin/a*"],
            Error::NegativePartialName,
        );
        baseline::agrees_with_fetch_specs(["^refs/heads/f2", "refs/heads/f*:refs/remotes/origin/a*"]);
        baseline::invalid_specs_fail_to_parse_where_git_shows_surprising_behaviour(
            ["^main", "refs/heads/*:refs/remotes/origin/*"],
            Error::NegativePartialName,
        );
        baseline::agrees_with_fetch_specs(["^refs/heads/main", "refs/heads/*:refs/remotes/origin/*"]);
        baseline::agrees_with_fetch_specs(["refs/heads/*:refs/remotes/origin/*", "^refs/heads/main"]);
    }

    #[test]
    fn fetch_and_update_with_empty_lhs() {
        baseline::agrees_but_observable_refs_are_vague([":refs/heads/f1"], ["HEAD:refs/heads/f1"]);
        baseline::agrees_but_observable_refs_are_vague([":f1"], ["HEAD:refs/heads/f1"]);
        baseline::agrees_but_observable_refs_are_vague(["@:f1"], ["HEAD:refs/heads/f1"]);
    }

    #[test]
    fn fetch_and_update_head_to_head_never_updates_actual_head_ref() {
        baseline::agrees_but_observable_refs_are_vague(["@:HEAD"], ["HEAD:refs/heads/HEAD"]);
    }

    #[test]
    fn fetch_and_update_head_with_empty_rhs() {
        baseline::agrees_but_observable_refs_are_vague([":"], ["HEAD:"]);
        baseline::agrees_but_observable_refs_are_vague(["HEAD:"], ["HEAD:"]);
        baseline::agrees_but_observable_refs_are_vague(["@:"], ["HEAD:"]);
    }

    #[test]
    fn fetch_and_update_multiple_destinations() {
        baseline::agrees_with_fetch_specs([
            "refs/heads/*:refs/remotes/origin/*",
            "refs/heads/main:refs/remotes/new-origin/main",
        ]);
        baseline::agrees_with_fetch_specs([
            "refs/heads/*:refs/remotes/origin/*",
            "refs/heads/main:refs/remotes/origin/main", // duplicates are removed immediately.
        ]);
    }

    #[test]
    fn fetch_and_update_with_conflicts() {
        baseline::agrees_with_fetch_specs_validation_error(
            [
                "refs/heads/f1:refs/remotes/origin/conflict",
                "refs/heads/f2:refs/remotes/origin/conflict",
            ],
            "Found 1 issue that prevents the refspec mapping to be used: \n\tConflicting destination \"refs/remotes/origin/conflict\" would be written by refs/heads/f1 (\"refs/heads/f1:refs/remotes/origin/conflict\"), refs/heads/f2 (\"refs/heads/f2:refs/remotes/origin/conflict\")",
        );
        baseline::agrees_with_fetch_specs_validation_error(
            [
                "refs/heads/f1:refs/remotes/origin/conflict2",
                "refs/heads/f2:refs/remotes/origin/conflict2",
                "refs/heads/f1:refs/remotes/origin/conflict",
                "refs/heads/f2:refs/remotes/origin/conflict",
                "refs/heads/f3:refs/remotes/origin/conflict",
            ],
            "Found 2 issues that prevent the refspec mapping to be used: \n\tConflicting destination \"refs/remotes/origin/conflict\" would be written by refs/heads/f1 (\"refs/heads/f1:refs/remotes/origin/conflict\"), refs/heads/f2 (\"refs/heads/f2:refs/remotes/origin/conflict\"), refs/heads/f3 (\"refs/heads/f3:refs/remotes/origin/conflict\")\n\tConflicting destination \"refs/remotes/origin/conflict2\" would be written by refs/heads/f1 (\"refs/heads/f1:refs/remotes/origin/conflict2\"), refs/heads/f2 (\"refs/heads/f2:refs/remotes/origin/conflict2\")",
        );
        baseline::agrees_with_fetch_specs_validation_error(
            [
                "refs/heads/f1:refs/remotes/origin/same",
                "refs/tags/v0.0-f1:refs/remotes/origin/same",
            ],
            "Found 1 issue that prevents the refspec mapping to be used: \n\tConflicting destination \"refs/remotes/origin/same\" would be written by refs/heads/f1 (\"refs/heads/f1:refs/remotes/origin/same\"), refs/tags/v0.0-f1 (\"refs/tags/v0.0-f1:refs/remotes/origin/same\")",
        );
        baseline::agrees_with_fetch_specs_validation_error(
            [
                "+refs/heads/*:refs/remotes/origin/*",
                "refs/heads/f1:refs/remotes/origin/f2",
                "refs/heads/f2:refs/remotes/origin/f1",
            ],
            "Found 2 issues that prevent the refspec mapping to be used: \n\tConflicting destination \"refs/remotes/origin/f1\" would be written by refs/heads/f1 (\"+refs/heads/*:refs/remotes/origin/*\"), refs/heads/f2 (\"refs/heads/f2:refs/remotes/origin/f1\")\n\tConflicting destination \"refs/remotes/origin/f2\" would be written by refs/heads/f2 (\"+refs/heads/*:refs/remotes/origin/*\"), refs/heads/f1 (\"refs/heads/f1:refs/remotes/origin/f2\")",
        );
    }

    #[test]
    fn fetch_and_update_with_fixes() {
        let glob_spec = "refs/heads/f*:foo/f*";
        let glob_spec_ref = gix_refspec::parse(glob_spec.into(), Operation::Fetch).unwrap();
        baseline::agrees_and_applies_fixes(
            [glob_spec, "f1:f1"],
            [
                Fix::MappingWithPartialDestinationRemoved {
                    name: "foo/f1".into(),
                    spec: glob_spec_ref.to_owned(),
                },
                Fix::MappingWithPartialDestinationRemoved {
                    name: "foo/f2".into(),
                    spec: glob_spec_ref.to_owned(),
                },
                Fix::MappingWithPartialDestinationRemoved {
                    name: "foo/f3".into(),
                    spec: glob_spec_ref.to_owned(),
                },
            ],
            ["refs/heads/f1:refs/heads/f1"],
        );
    }
}

mod complex_globs {
    use bstr::BString;
    use gix_hash::ObjectId;
    use gix_refspec::{parse::Operation, MatchGroup};

    #[test]
    fn one_sided_complex_glob_patterns_can_be_parsed() {
        // The key change is that complex glob patterns with multiple asterisks
        // can now be parsed for one-sided refspecs
        let spec = gix_refspec::parse("refs/*/foo/*".into(), Operation::Fetch);
        assert!(spec.is_ok(), "Should parse complex glob pattern for one-sided refspec");

        let spec = gix_refspec::parse("refs/*/*/bar".into(), Operation::Fetch);
        assert!(
            spec.is_ok(),
            "Should parse complex glob pattern with multiple asterisks"
        );

        let spec = gix_refspec::parse("refs/heads/[a-z.]/release/*".into(), Operation::Fetch);
        assert!(spec.is_ok(), "Should parse complex glob pattern");

        // Two-sided refspecs with multiple asterisks should still fail
        let spec = gix_refspec::parse("refs/*/foo/*:refs/remotes/*".into(), Operation::Fetch);
        assert!(spec.is_err(), "Two-sided refspecs with multiple asterisks should fail");
    }

    #[test]
    fn one_sided_simple_glob_patterns_match() {
        // Test that simple glob patterns (one asterisk) work correctly with matching
        let refs = [
            new_ref("refs/heads/feature/foo", "1111111111111111111111111111111111111111"),
            new_ref("refs/heads/bugfix/bar", "2222222222222222222222222222222222222222"),
            new_ref("refs/tags/v1.0", "3333333333333333333333333333333333333333"),
            new_ref("refs/pull/123", "4444444444444444444444444444444444444444"),
        ];
        let items: Vec<_> = refs.iter().map(|r| r.to_item()).collect();

        // Test: refs/heads/* should match all refs under refs/heads/
        let spec = gix_refspec::parse("refs/heads/*".into(), Operation::Fetch).unwrap();
        let group = MatchGroup::from_fetch_specs([spec]);
        let outcome = group.match_lhs(items.iter().copied());

        insta::assert_debug_snapshot!(outcome.mappings, @r#"
        [
            Mapping {
                item_index: Some(
                    0,
                ),
                lhs: FullName(
                    "refs/heads/feature/foo",
                ),
                rhs: None,
                spec_index: 0,
            },
            Mapping {
                item_index: Some(
                    1,
                ),
                lhs: FullName(
                    "refs/heads/bugfix/bar",
                ),
                rhs: None,
                spec_index: 0,
            },
        ]
        "#);

        // Test: refs/tags/* should match all refs under refs/tags/
        let items: Vec<_> = refs.iter().map(|r| r.to_item()).collect();
        let spec = gix_refspec::parse("refs/tags/v[0-9]*".into(), Operation::Fetch).unwrap();
        let group = MatchGroup::from_fetch_specs([spec]);
        let outcome = group.match_lhs(items.iter().copied());

        insta::assert_debug_snapshot!(outcome.mappings, @r#"
        [
            Mapping {
                item_index: Some(
                    2,
                ),
                lhs: FullName(
                    "refs/tags/v1.0",
                ),
                rhs: None,
                spec_index: 0,
            },
        ]
        "#);
    }

    #[test]
    fn one_sided_glob_with_suffix_matches() {
        // Test that glob patterns with suffix work correctly
        let refs = [
            new_ref("refs/heads/feature", "1111111111111111111111111111111111111111"),
            new_ref("refs/heads/feat", "2222222222222222222222222222222222222222"),
            new_ref("refs/heads/main", "3333333333333333333333333333333333333333"),
        ];
        let items: Vec<_> = refs.iter().map(|r| r.to_item()).collect();

        // Test: refs/heads/feat* should match refs/heads/feature and refs/heads/feat
        let spec = gix_refspec::parse("refs/heads/feat*".into(), Operation::Fetch).unwrap();
        let group = MatchGroup::from_fetch_specs([spec]);
        let outcome = group.match_lhs(items.iter().copied());
        let mappings = outcome.mappings;

        insta::assert_debug_snapshot!(mappings, @r#"
        [
            Mapping {
                item_index: Some(
                    0,
                ),
                lhs: FullName(
                    "refs/heads/feature",
                ),
                rhs: None,
                spec_index: 0,
            },
            Mapping {
                item_index: Some(
                    1,
                ),
                lhs: FullName(
                    "refs/heads/feat",
                ),
                rhs: None,
                spec_index: 0,
            },
        ]
        "#);
    }

    fn new_ref(name: &str, id_hex: &str) -> Ref {
        Ref {
            name: name.into(),
            target: ObjectId::from_hex(id_hex.as_bytes()).unwrap(),
            object: None,
        }
    }

    #[derive(Debug, Clone)]
    struct Ref {
        name: BString,
        target: ObjectId,
        object: Option<ObjectId>,
    }

    impl Ref {
        fn to_item(&self) -> gix_refspec::match_group::Item<'_> {
            gix_refspec::match_group::Item {
                full_ref_name: self.name.as_ref(),
                target: &self.target,
                object: self.object.as_deref(),
            }
        }
    }
}
