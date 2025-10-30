mod single {
    use crate::matching::baseline;

    #[test]
    fn fetch_only() {
        baseline::agrees_with_fetch_specs(Some("refs/heads/main"));
        baseline::agrees_with_fetch_specs(Some("heads/main"));
        baseline::agrees_with_fetch_specs(Some("main"));
        baseline::agrees_with_fetch_specs(Some("v0.0-f1"));
        baseline::agrees_with_fetch_specs(Some("tags/v0.0-f2"));
        baseline::of_objects_always_matches_if_the_server_has_the_object(Some(
            "78b1c1be9421b33a49a7a8176d93eeeafa112da1",
        ));
        baseline::of_objects_always_matches_if_the_server_has_the_object(Some(
            "9d2fab1a0ba3585d0bc50922bfdd04ebb59361df",
        ));
    }

    #[test]
    fn fetch_and_update() {
        baseline::of_objects_with_destinations_are_written_into_given_local_branches(
            Some("78b1c1be9421b33a49a7a8176d93eeeafa112da1:special"),
            ["78b1c1be9421b33a49a7a8176d93eeeafa112da1:refs/heads/special"],
        );
        baseline::of_objects_with_destinations_are_written_into_given_local_branches(
            Some("78b1c1be9421b33a49a7a8176d93eeeafa112da1:1111111111111111111111111111111111111111"),
            ["78b1c1be9421b33a49a7a8176d93eeeafa112da1:refs/heads/1111111111111111111111111111111111111111"],
        );
        baseline::of_objects_with_destinations_are_written_into_given_local_branches(
            Some("9d2fab1a0ba3585d0bc50922bfdd04ebb59361df:tags/special"),
            ["9d2fab1a0ba3585d0bc50922bfdd04ebb59361df:refs/tags/special"],
        );
        baseline::of_objects_with_destinations_are_written_into_given_local_branches(
            Some("9d2fab1a0ba3585d0bc50922bfdd04ebb59361df:refs/tags/special"),
            ["9d2fab1a0ba3585d0bc50922bfdd04ebb59361df:refs/tags/special"],
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
    use gix_refspec::{parse::Operation, MatchGroup};
    use gix_hash::ObjectId;
    use bstr::BString;
    use std::borrow::Cow;

    #[test]
    fn one_sided_complex_glob_patterns_can_be_parsed() {
        // The key change is that complex glob patterns with multiple asterisks
        // can now be parsed for one-sided refspecs
        let spec1 = gix_refspec::parse("refs/*/foo/*".into(), Operation::Fetch);
        assert!(spec1.is_ok(), "Should parse complex glob pattern for one-sided refspec");
        
        let spec2 = gix_refspec::parse("refs/*/*/bar".into(), Operation::Fetch);
        assert!(spec2.is_ok(), "Should parse complex glob pattern with multiple asterisks");
        
        let spec3 = gix_refspec::parse("refs/heads/*/release/*".into(), Operation::Fetch);
        assert!(spec3.is_ok(), "Should parse complex glob pattern");
        
        // Two-sided refspecs with multiple asterisks should still fail
        let spec4 = gix_refspec::parse("refs/*/foo/*:refs/remotes/*".into(), Operation::Fetch);
        assert!(spec4.is_err(), "Two-sided refspecs with multiple asterisks should fail");
    }
    
    #[test]
    fn one_sided_simple_glob_patterns_match() {
        // Test that simple glob patterns (one asterisk) work correctly with matching
        let refs = vec![
            create_ref("refs/heads/feature/foo", "1111111111111111111111111111111111111111"),
            create_ref("refs/heads/bugfix/bar", "2222222222222222222222222222222222222222"),
            create_ref("refs/tags/v1.0", "3333333333333333333333333333333333333333"),
            create_ref("refs/pull/123", "4444444444444444444444444444444444444444"),
        ];
        let items: Vec<_> = refs.iter().map(|r| r.to_item()).collect();
        
        // Test: refs/heads/* should match all refs under refs/heads/
        let spec = gix_refspec::parse("refs/heads/*".into(), Operation::Fetch).unwrap();
        let group = MatchGroup::from_fetch_specs([spec]);
        let outcome = group.match_lhs(items.iter().copied());
        let mappings = outcome.mappings;
        
        assert_eq!(mappings.len(), 2, "Should match two refs under refs/heads/");
        
        // Test: refs/tags/* should match all refs under refs/tags/
        let items2: Vec<_> = refs.iter().map(|r| r.to_item()).collect();
        let spec2 = gix_refspec::parse("refs/tags/*".into(), Operation::Fetch).unwrap();
        let group2 = MatchGroup::from_fetch_specs([spec2]);
        let outcome2 = group2.match_lhs(items2.iter().copied());
        let mappings2 = outcome2.mappings;
        
        assert_eq!(mappings2.len(), 1, "Should match one ref under refs/tags/");
    }
    
    #[test]
    fn one_sided_glob_with_suffix_matches() {
        // Test that glob patterns with suffix work correctly
        let refs = vec![
            create_ref("refs/heads/feature", "1111111111111111111111111111111111111111"),
            create_ref("refs/heads/feat", "2222222222222222222222222222222222222222"),
            create_ref("refs/heads/main", "3333333333333333333333333333333333333333"),
        ];
        let items: Vec<_> = refs.iter().map(|r| r.to_item()).collect();
        
        // Test: refs/heads/feat* should match refs/heads/feature and refs/heads/feat
        let spec = gix_refspec::parse("refs/heads/feat*".into(), Operation::Fetch).unwrap();
        let group = MatchGroup::from_fetch_specs([spec]);
        let outcome = group.match_lhs(items.iter().copied());
        let mappings = outcome.mappings;
        
        assert_eq!(mappings.len(), 2, "Should match two refs starting with feat");
    }
    
    // Helper function to create a ref
    fn create_ref(name: &str, id_hex: &str) -> Ref {
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
