mod single {
    use crate::matching::baseline;

    fn test_hashes() -> (String, String) {
        let annotated_tag = match gix_testtools::object_hash() {
            gix_hash::Kind::Sha1 => "78b1c1be9421b33a49a7a8176d93eeeafa112da1",
            gix_hash::Kind::Sha256 => "b071221ea854da2958fba3a37527ca5cf32c4ebcd71ab0b68b6b8f10f04e93ad",
            _ => unimplemented!(),
        };
        let initial_commit = match gix_testtools::object_hash() {
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
    use bstr::BString;
    use gix_hash::ObjectId;
    use gix_refspec::{MatchGroup, match_group::Item, match_group::validate::Fix, parse::Operation};

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
        baseline::agrees_with_fetch_specs(["refs/heads/f*:refs/remotes/origin/a*", "^f1"]);
        baseline::agrees_with_fetch_specs(["refs/heads/f*:refs/remotes/origin/a*", "^refs/heads/f1"]);
        baseline::agrees_with_fetch_specs(["^heads/f2", "refs/heads/f*:refs/remotes/origin/a*"]);
        baseline::agrees_with_fetch_specs(["^refs/heads/f2", "refs/heads/f*:refs/remotes/origin/a*"]);
        baseline::agrees_with_fetch_specs(["^main", "refs/heads/*:refs/remotes/origin/*"]);
        baseline::agrees_with_fetch_specs(["^refs/heads/main", "refs/heads/*:refs/remotes/origin/*"]);
        baseline::agrees_with_fetch_specs(["refs/heads/*:refs/remotes/origin/*", "^refs/heads/main"]);
        baseline::agrees_with_fetch_specs(["refs/heads/*:refs/remotes/origin/*", "^refs/heads/*-deploy"]);
    }

    #[test]
    fn negative_sources_are_matched_literally_in_both_directions() {
        let target = ObjectId::from_hex(b"1111111111111111111111111111111111111111").expect("valid object id");
        let remote_item = Item {
            full_ref_name: "refs/heads/main".into(),
            target: &target,
            object: None,
        };
        let local_item = Item {
            full_ref_name: "refs/remotes/origin/main".into(),
            target: &target,
            object: None,
        };
        let positive = "refs/heads/*:refs/remotes/origin/*";

        for (negative, should_exclude) in [
            ("^main", false),
            ("^heads/*", false),
            ("^refs/heads/main", true),
            ("^refs/heads/*", true),
        ] {
            let specs = || {
                [positive, negative]
                    .into_iter()
                    .map(|spec| gix_refspec::parse(spec.into(), Operation::Fetch).expect("valid refspec"))
            };

            let outcome = MatchGroup::from_fetch_specs(specs()).match_lhs([remote_item].into_iter());
            assert_eq!(
                outcome.mappings.is_empty(),
                should_exclude,
                "{negative} applies literally when matching sources"
            );

            let outcome = MatchGroup::from_fetch_specs(specs()).match_rhs([local_item].into_iter());
            assert_eq!(
                outcome.mappings.is_empty(),
                should_exclude,
                "{negative} applies literally after reverse mapping destinations"
            );
        }
    }

    #[test]
    fn reverse_fetch_mapping_honors_negative_source_patterns() {
        let specs = ["refs/heads/*:refs/remotes/origin/*", "^refs/heads/*-deploy"]
            .into_iter()
            .map(|spec| gix_refspec::parse(spec.into(), Operation::Fetch).expect("valid refspec"));
        let target = ObjectId::from_hex(b"1111111111111111111111111111111111111111").expect("valid object id");
        let refs = [
            BString::from("refs/remotes/origin/main"),
            BString::from("refs/remotes/origin/foo-deploy"),
        ];
        let items: Vec<_> = refs
            .iter()
            .map(|name| Item {
                full_ref_name: name.as_ref(),
                target: &target,
                object: None,
            })
            .collect();

        let outcome = MatchGroup::from_fetch_specs(specs).match_rhs(items.iter().copied());
        insta::assert_debug_snapshot!(outcome.mappings, @r#"
        [
            Mapping {
                item_index: Some(
                    0,
                ),
                lhs: FullName(
                    "refs/heads/main",
                ),
                rhs: Some(
                    "refs/remotes/origin/main",
                ),
                spec_index: 0,
            },
        ]
        "#);
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
