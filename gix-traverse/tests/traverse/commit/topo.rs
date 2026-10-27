use crate::Result;
use crate::hex_to_id;
use crate::util::{
    CountingFind, commit_graph, fixture, fixture_odb, named_fixture, parse_commit_names, rev_list_baseline,
};
use gix_hash::{ObjectId, oid};
use gix_object::bstr::ByteSlice;
use gix_traverse::commit::{Parents, topo};
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn odb() -> Result<gix_odb::Handle> {
    fixture_odb("make_repo_for_topo.sh")
}

fn fixture_dir() -> Result<PathBuf> {
    fixture("make_repo_for_topo.sh")
}

#[test]
fn predicates_prune_paths_and_run_once_per_visible_commit() -> gix_testtools::TestResult {
    let repo_dir = fixture_dir()?.join("hidden-ends");
    let odb = crate::util::odb_at(repo_dir.join(".git/objects"))?;
    let commits = parse_commit_names(&repo_dir)?;
    for sorting in [topo::Sorting::TopoOrder, topo::Sorting::DateOrder] {
        for (tips, expected) in [
            (vec![commits["visible"]], vec![commits["visible"]]),
            (
                vec![commits["visible"], commits["side3"]],
                ["visible", "side3", "side2", "side1", "base"]
                    .map(|name| commits[name])
                    .to_vec(),
            ),
        ] {
            for graph in [None, commit_graph(odb.store_ref())] {
                let mut calls = std::collections::HashMap::new();
                let actual = topo::Builder::from_iters(&odb, tips.clone(), None::<Vec<ObjectId>>)
                    .sorting(sorting)
                    .with_commit_graph(graph)
                    .with_predicate(|commit_id| {
                        *calls.entry(commit_id.to_owned()).or_insert(0) += 1;
                        commit_id != commits["merge"]
                    })
                    .build()?
                    .map(|item| item.map(|info| info.id))
                    .collect::<gix_error::Result<Vec<_>>>()?;
                assert_eq!(
                    actual, expected,
                    "another accepted path can still reach the pruned ancestry"
                );
                assert!(
                    calls.values().all(|&count| count == 1),
                    "each visible candidate is tested once"
                );
                assert!(
                    !calls.contains_key(&commits["main"]),
                    "a rejected merge does not visit its parents"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn rejected_tips_and_parents_are_not_looked_up() -> gix_testtools::TestResult {
    let repo_dir = fixture_dir()?.join("hidden-ends");
    let odb = crate::util::odb_at(repo_dir.join(".git/objects"))?;
    let commits = parse_commit_names(&repo_dir)?;
    let missing_commit_id = ObjectId::null(gix_testtools::object_hash());
    let lookups = Rc::new(Cell::new(0));
    let actual = topo::Builder::from_iters(
        CountingFind {
            inner: &odb,
            lookups: Rc::clone(&lookups),
        },
        [missing_commit_id],
        None::<Vec<ObjectId>>,
    )
    .with_predicate(|_| false)
    .build()?
    .collect::<gix_error::Result<Vec<_>>>()?;
    assert!(actual.is_empty(), "rejecting a tip removes the whole path");
    assert_eq!(lookups.get(), 0, "a rejected tip need not exist");

    struct MissingParent<'a> {
        odb: &'a gix_odb::Handle,
        missing_commit_id: ObjectId,
    }
    impl gix_object::Find for MissingParent<'_> {
        fn try_find<'a>(
            &self,
            commit_id: &oid,
            buf: &'a mut Vec<u8>,
        ) -> gix_error::Result<Option<gix_object::Data<'a>>> {
            if commit_id == self.missing_commit_id {
                Ok(None)
            } else {
                gix_object::Find::try_find(self.odb, commit_id, buf)
            }
        }
    }
    for sorting in [topo::Sorting::TopoOrder, topo::Sorting::DateOrder] {
        for hidden in [vec![], vec![commits["side3"]]] {
            let mut calls = std::collections::HashMap::new();
            let actual = topo::Builder::from_iters(
                MissingParent {
                    odb: &odb,
                    missing_commit_id: commits["merge"],
                },
                [commits["visible"]],
                Some(hidden),
            )
            .sorting(sorting)
            .with_predicate(|commit_id| {
                *calls.entry(commit_id.to_owned()).or_insert(0) += 1;
                commit_id != commits["merge"]
            })
            .build()?
            .collect::<gix_error::Result<Vec<_>>>()?;
            assert_eq!(actual.len(), 1, "the accepted tip survives a rejected missing parent");
            assert_eq!(actual[0].id, commits["visible"]);
            assert_eq!(
                actual[0].parent_ids.as_slice(),
                &[commits["merge"]],
                "metadata retains the original parent ID"
            );
            assert!(
                calls.values().all(|&count| count == 1),
                "hidden-frontier preparation and the main walk share predicate decisions"
            );
        }
    }
    Ok(())
}

#[test]
fn hidden_ancestry_ignores_the_visible_predicate() -> gix_testtools::TestResult {
    let repo_dir = fixture_dir()?.join("hidden-ends");
    let odb = crate::util::odb_at(repo_dir.join(".git/objects"))?;
    let commits = parse_commit_names(&repo_dir)?;
    for sorting in [topo::Sorting::TopoOrder, topo::Sorting::DateOrder] {
        for parents in [Parents::All, Parents::First] {
            let actual = traverse_with_predicate(
                [commits["visible"]],
                [commits["side3"]],
                &odb,
                sorting,
                parents,
                |commit_id| commit_id != commits["side2"],
            )?;
            assert_eq!(
                actual,
                ["visible", "merge", "main"].map(|name| commits[name]),
                "hidden painting crosses rejected commits too"
            );
        }
    }
    Ok(())
}

#[test]
fn first_parent_metadata_matches_the_traversed_edges() -> gix_testtools::TestResult {
    let repo_dir = fixture_dir()?.join("hidden-ends");
    let odb = crate::util::odb_at(repo_dir.join(".git/objects"))?;
    let commits = parse_commit_names(&repo_dir)?;
    for sorting in [topo::Sorting::TopoOrder, topo::Sorting::DateOrder] {
        for graph in [None, commit_graph(odb.store_ref())] {
            let actual = topo::Builder::from_iters(&odb, [commits["merge"]], None::<Vec<ObjectId>>)
                .sorting(sorting)
                .parents(Parents::First)
                .with_commit_graph(graph)
                .build()?
                .next()
                .expect("the merge starts the traversal")?;
            assert_eq!(
                actual.parent_ids.as_slice(),
                &[commits["main"]],
                "first-parent metadata excludes the merge's second parent"
            );
        }
    }
    Ok(())
}

#[test]
fn hidden_ends_exclude_tips_and_all_their_ancestors() -> gix_testtools::TestResult {
    use crate::util::{odb_at, parse_commit_names, rev_list_baseline};

    let repo_dir = fixture_dir()?.join("hidden-ends");
    let odb = odb_at(repo_dir.join(".git/objects"))?;
    let commits = parse_commit_names(&repo_dir)?;
    for (sorting, order) in [
        (topo::Sorting::TopoOrder, "topo-order"),
        (topo::Sorting::DateOrder, "date-order"),
    ] {
        for (parents, parent_mode) in [(Parents::All, "do-walk"), (Parents::First, "first-parent")] {
            for (tip_ref, tip_name) in [
                ("main", "merge"),
                ("side", "side3"),
                ("side2", "side2"),
                ("side1", "side1"),
                ("visible", "visible"),
            ] {
                let expected = rev_list_baseline(&repo_dir, &format!("{order}-{parent_mode}-{tip_ref}"))?;
                assert_eq!(
                    traverse_both([commits[tip_name]], [commits["merge"]], &odb, sorting, parents)?,
                    expected,
                    "--{order} --{parent_mode} {tip_ref} ^main must match Git"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn late_hidden_marks_revisit_previously_explored_commits() -> gix_testtools::TestResult {
    use crate::util::{odb_at, parse_commit_names, rev_list_baseline};

    let repo_dir = fixture_dir()?.join("hidden-skewed");
    let odb = odb_at(repo_dir.join(".git/objects"))?;
    let commits = parse_commit_names(&repo_dir)?;
    for (sorting, order) in [
        (topo::Sorting::TopoOrder, "topo-order"),
        (topo::Sorting::DateOrder, "date-order"),
    ] {
        for (parents, parent_mode) in [(Parents::All, "do-walk"), (Parents::First, "first-parent")] {
            for (tips, ends, case) in [
                (
                    vec![commits["visible"], commits["side"]],
                    vec![commits["hidden"]],
                    "hidden",
                ),
                (vec![commits["visible"]], vec![], "visible"),
            ] {
                let expected = rev_list_baseline(&repo_dir, &format!("{order}-{parent_mode}-{case}"))?;
                assert_eq!(
                    traverse_both(tips, ends, &odb, sorting, parents)?,
                    expected,
                    "late hidden propagation and visible first-parent traversal must match Git: --{order} --{parent_mode}, {case}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn merged_visible_ranges_stop_near_hidden_boundaries() -> gix_testtools::TestResult {
    let (repo_dir, odb) = named_fixture("make_repo_for_hidden_bug.sh", "merged_visible_range")?;
    let commits = parse_commit_names(&repo_dir)?;
    for (sorting, order) in [
        (topo::Sorting::TopoOrder, "topo-order"),
        (topo::Sorting::DateOrder, "date-order"),
    ] {
        for (parents, parent_mode) in [(Parents::All, "do-walk"), (Parents::First, "first-parent")] {
            for (tip, tip_ref, max_lookups) in [("visible", "main", 100), ("visible-wide", "wide", 800)] {
                let baseline = rev_list_baseline(&repo_dir, &format!("{order}-{parent_mode}-{tip_ref}"))?;
                for graph in [None, commit_graph(odb.store_ref())] {
                    let lookups = Rc::new(Cell::new(0));
                    let actual = topo::Builder::from_iters(
                        CountingFind {
                            inner: &odb,
                            lookups: Rc::clone(&lookups),
                        },
                        [commits[tip]],
                        Some([commits["hidden"]]),
                    )
                    .sorting(sorting)
                    .parents(parents)
                    .with_commit_graph(graph)
                    .build()?
                    .map(|item| item.map(|info| info.id))
                    .collect::<gix_error::Result<Vec<_>>>()?;
                    assert_eq!(
                        actual, baseline,
                        "--{order} --{parent_mode} {tip_ref} ^hidden must match Git's ordering"
                    );
                    assert!(
                        lookups.get() <= max_lookups,
                        "{tip_ref} must avoid traversing the old hidden merge history: {} reads",
                        lookups.get()
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn hidden_lookahead_paints_loaded_ancestors() -> gix_testtools::TestResult {
    let (repo_dir, odb) = named_fixture("make_repo_for_hidden_bug.sh", "skewed_lookahead")?;
    let commits = parse_commit_names(&repo_dir)?;
    let baseline = rev_list_baseline(&repo_dir, "rev-list")?;
    for sorting in [topo::Sorting::TopoOrder, topo::Sorting::DateOrder] {
        for parents in [Parents::All, Parents::First] {
            assert_eq!(
                traverse_both([commits["visible"]], [commits["hidden"]], &odb, sorting, parents)?,
                baseline,
                "the date cutoff must not expose an already-loaded hidden root"
            );
        }
    }
    Ok(())
}

#[test]
fn empty_and_short_ranges_do_not_walk_shared_history() -> gix_testtools::TestResult {
    let (repo_dir, odb) = named_fixture("make_repo_for_hidden_bug.sh", "short_visible_range")?;
    let commits = parse_commit_names(&repo_dir)?;
    let baseline = rev_list_baseline(&repo_dir, "rev-list")?;
    for sorting in [topo::Sorting::TopoOrder, topo::Sorting::DateOrder] {
        for parents in [Parents::All, Parents::First] {
            for (tip, expected, max_lookups) in [
                (commits["visible"], baseline.as_slice(), 40),
                (commits["shared-63"], &[][..], 8),
            ] {
                for graph in [None, commit_graph(odb.store_ref())] {
                    let lookups = Rc::new(Cell::new(0));
                    let actual = topo::Builder::from_iters(
                        CountingFind {
                            inner: &odb,
                            lookups: Rc::clone(&lookups),
                        },
                        [tip],
                        Some([commits["shared-63"]]),
                    )
                    .sorting(sorting)
                    .parents(parents)
                    .with_commit_graph(graph)
                    .build()?
                    .map(|item| item.map(|info| info.id))
                    .collect::<gix_error::Result<Vec<_>>>()?;
                    assert_eq!(actual, expected, "the range excludes its hidden boundary");
                    assert!(
                        lookups.get() <= max_lookups,
                        "an empty or one-commit range must avoid the 64-commit shared history: {} reads",
                        lookups.get()
                    );
                }
            }
        }
    }
    Ok(())
}

/// Run a topo traversal with both commit-graph enabled and disabled to ensure consistency.
fn traverse_both(
    tips: impl IntoIterator<Item = ObjectId> + Clone,
    ends: impl IntoIterator<Item = ObjectId> + Clone,
    odb: &gix_odb::Handle,
    sorting: topo::Sorting,
    parents: Parents,
) -> Result<Vec<ObjectId>> {
    // Without commit graph
    let without_graph: Vec<_> = topo::Builder::from_iters(odb, tips.clone(), Some(ends.clone()))
        .sorting(sorting)
        .with_commit_graph(None)
        .parents(parents)
        .build()?
        .map(|res| res.map(|info| info.id))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // With commit graph
    let graph = commit_graph(odb.store_ref());
    let with_graph: Vec<_> = topo::Builder::from_iters(odb, tips, Some(ends))
        .sorting(sorting)
        .with_commit_graph(graph)
        .parents(parents)
        .build()?
        .map(|res| res.map(|info| info.id))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    assert_eq!(
        without_graph, with_graph,
        "results must be consistent with and without commit-graph"
    );
    Ok(with_graph)
}

/// Run a topo traversal with a predicate filter.
fn traverse_with_predicate(
    tips: impl IntoIterator<Item = ObjectId> + Clone,
    ends: impl IntoIterator<Item = ObjectId> + Clone,
    odb: &gix_odb::Handle,
    sorting: topo::Sorting,
    parents: Parents,
    predicate: impl FnMut(&oid) -> bool + Clone,
) -> Result<Vec<ObjectId>> {
    // Without commit graph
    let without_graph: Vec<_> = topo::Builder::from_iters(odb, tips.clone(), Some(ends.clone()))
        .sorting(sorting)
        .with_commit_graph(None)
        .parents(parents)
        .with_predicate(predicate.clone())
        .build()?
        .map(|res| res.map(|info| info.id))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    // With commit graph
    let graph = commit_graph(odb.store_ref());
    let with_graph: Vec<_> = topo::Builder::from_iters(odb, tips, Some(ends))
        .sorting(sorting)
        .with_commit_graph(graph)
        .parents(parents)
        .with_predicate(predicate)
        .build()?
        .map(|res| res.map(|info| info.id))
        .collect::<std::result::Result<Vec<_>, _>>()?;

    assert_eq!(
        without_graph, with_graph,
        "results must be consistent with and without commit-graph"
    );
    Ok(with_graph)
}

/// Read baseline file and parse expected commit hashes.
fn read_baseline(fixture_dir: &std::path::Path, name: &str) -> Result<Vec<String>> {
    let buf = std::fs::read(fixture_dir.join(format!("{name}.baseline")))?;
    Ok(buf.lines().map(|s| s.to_str().unwrap().to_string()).collect())
}

mod basic {
    use super::*;

    #[test]
    fn simple() -> gix_testtools::TestResult {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "3be0c4c793c634c8fd95054345d4935d10a0879a",
            "2083b02a78e88b747e305b6ed3d5a861cf9fb73f",
            "302a5d0530ec688c241f32c2f2b61b964dd17bee",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "22fbc169eeca3c9678fc7028aa80fad5ef49019f",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
            "f1cce1b5c7efcdfa106e95caa6c45a2cae48a481",
            "945d8a360915631ad545e0cf04630d86d3d4eaa1",
            "a863c02247a6c5ba32dff5224459f52aa7f77f7b",
            "2f291881edfb0597493a52d26ea09dd7340ce507",
            "9c46b8765703273feb10a2ebd810e70b8e2ca44a",
            "fb3e21cf45b04b617011d2b30973f3e5ce60d0cd",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [], &odb, topo::Sorting::TopoOrder, Parents::All)?;
        assert_eq!(result, expected);

        // Verify against baseline
        let baseline = read_baseline(&fixture_dir()?, "all-commits")?;
        let expected_strs: Vec<_> = expected.iter().map(std::string::ToString::to_string).collect();
        assert_eq!(expected_strs, baseline, "Baseline must match the expectation");

        Ok(())
    }

    #[test]
    fn duplicate_tips_are_ignored() -> gix_testtools::TestResult {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");

        for sorting in [topo::Sorting::TopoOrder, topo::Sorting::DateOrder] {
            let expected = traverse_both([tip], [], &odb, sorting, Parents::All)?;
            let actual = traverse_both([tip, tip], [], &odb, sorting, Parents::All)?;
            assert_eq!(actual, expected, "duplicate tips must not affect the traversal");
        }
        Ok(())
    }

    #[test]
    fn one_end() -> gix_testtools::TestResult {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");
        let end = hex_to_id("f1cce1b5c7efcdfa106e95caa6c45a2cae48a481");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "3be0c4c793c634c8fd95054345d4935d10a0879a",
            "2083b02a78e88b747e305b6ed3d5a861cf9fb73f",
            "302a5d0530ec688c241f32c2f2b61b964dd17bee",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "22fbc169eeca3c9678fc7028aa80fad5ef49019f",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [end], &odb, topo::Sorting::TopoOrder, Parents::All)?;
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn empty_range() -> gix_testtools::TestResult {
        let odb = odb()?;
        let tip = hex_to_id("f1cce1b5c7efcdfa106e95caa6c45a2cae48a481");
        let end = hex_to_id("eeab3243aad67bc838fc4425f759453bf0b47785");

        let result = traverse_both([tip], [end], &odb, topo::Sorting::TopoOrder, Parents::All)?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn two_tips_two_ends() -> gix_testtools::TestResult {
        let odb = odb()?;
        let tips = [
            hex_to_id("d09384f312b03e4a1413160739805ff25e8fe99d"),
            hex_to_id("3be0c4c793c634c8fd95054345d4935d10a0879a"),
        ];
        let ends = [
            hex_to_id("1a27cb1a26c9faed9f0d1975326fe51123ab01ed"),
            hex_to_id("22fbc169eeca3c9678fc7028aa80fad5ef49019f"),
        ];

        let expected = [
            "3be0c4c793c634c8fd95054345d4935d10a0879a",
            "2083b02a78e88b747e305b6ed3d5a861cf9fb73f",
            "302a5d0530ec688c241f32c2f2b61b964dd17bee",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
        ]
        .map(hex_to_id);

        let result = traverse_both(tips, ends, &odb, topo::Sorting::TopoOrder, Parents::All)?;
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn predicate_prunes_ancestry() -> gix_testtools::TestResult {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");
        let filter_out = hex_to_id("eeab3243aad67bc838fc4425f759453bf0b47785");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "3be0c4c793c634c8fd95054345d4935d10a0879a",
            "2083b02a78e88b747e305b6ed3d5a861cf9fb73f",
            "302a5d0530ec688c241f32c2f2b61b964dd17bee",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "22fbc169eeca3c9678fc7028aa80fad5ef49019f",
            "f1cce1b5c7efcdfa106e95caa6c45a2cae48a481",
            "945d8a360915631ad545e0cf04630d86d3d4eaa1",
            "a863c02247a6c5ba32dff5224459f52aa7f77f7b",
            "2f291881edfb0597493a52d26ea09dd7340ce507",
            "9c46b8765703273feb10a2ebd810e70b8e2ca44a",
            "fb3e21cf45b04b617011d2b30973f3e5ce60d0cd",
        ]
        .map(hex_to_id);

        let result = traverse_with_predicate([tip], [], &odb, topo::Sorting::TopoOrder, Parents::All, move |oid| {
            oid != filter_out
        })?;
        assert_eq!(
            result, expected,
            "only another accepted path can reach the pruned ancestry"
        );
        Ok(())
    }

    #[test]
    fn end_along_first_parent() -> gix_testtools::TestResult {
        let odb = odb()?;
        let tip = hex_to_id("d09384f312b03e4a1413160739805ff25e8fe99d");
        let end = hex_to_id("33eb18340e4eaae3e3dcf80222b02f161cd3f966");

        let expected = [
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "22fbc169eeca3c9678fc7028aa80fad5ef49019f",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [end], &odb, topo::Sorting::TopoOrder, Parents::All)?;
        assert_eq!(result, expected);
        Ok(())
    }
}

mod first_parent {
    use super::*;

    #[test]
    fn basic() -> gix_testtools::TestResult {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
            "f1cce1b5c7efcdfa106e95caa6c45a2cae48a481",
            "945d8a360915631ad545e0cf04630d86d3d4eaa1",
            "a863c02247a6c5ba32dff5224459f52aa7f77f7b",
            "2f291881edfb0597493a52d26ea09dd7340ce507",
            "9c46b8765703273feb10a2ebd810e70b8e2ca44a",
            "fb3e21cf45b04b617011d2b30973f3e5ce60d0cd",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [], &odb, topo::Sorting::TopoOrder, Parents::First)?;
        assert_eq!(result, expected);

        // Verify against baseline
        let baseline = read_baseline(&fixture_dir()?, "first-parent")?;
        let expected_strs: Vec<_> = expected.iter().map(std::string::ToString::to_string).collect();
        assert_eq!(expected_strs, baseline, "Baseline must match the expectation");

        Ok(())
    }

    #[test]
    fn with_end() -> gix_testtools::TestResult {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");
        let end = hex_to_id("f1cce1b5c7efcdfa106e95caa6c45a2cae48a481");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [end], &odb, topo::Sorting::TopoOrder, Parents::First)?;
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn end_is_second_parent() -> gix_testtools::TestResult {
        let odb = odb()?;
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");
        let end = hex_to_id("3be0c4c793c634c8fd95054345d4935d10a0879a");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [end], &odb, topo::Sorting::TopoOrder, Parents::First)?;
        assert_eq!(result, expected);
        Ok(())
    }
}

mod date_order {
    use super::*;

    #[test]
    fn with_ends() -> gix_testtools::TestResult {
        let odb = odb()?;
        // Same tip and end as basic::one_end() but the order should be different.
        let tip = hex_to_id("62ed296d9986f50477e9f7b7e81cd0258939a43d");
        let end = hex_to_id("f1cce1b5c7efcdfa106e95caa6c45a2cae48a481");

        let expected = [
            "62ed296d9986f50477e9f7b7e81cd0258939a43d",
            "722bf6b8c3d9e3a11fa5100a02ed9b140e1d209c",
            "3be0c4c793c634c8fd95054345d4935d10a0879a",
            "2083b02a78e88b747e305b6ed3d5a861cf9fb73f",
            "302a5d0530ec688c241f32c2f2b61b964dd17bee",
            "d09384f312b03e4a1413160739805ff25e8fe99d",
            "eeab3243aad67bc838fc4425f759453bf0b47785",
            "22fbc169eeca3c9678fc7028aa80fad5ef49019f",
            "693c775700cf90bd158ee6e7f14dd1b7bd83a4ce",
            "33eb18340e4eaae3e3dcf80222b02f161cd3f966",
            "1a27cb1a26c9faed9f0d1975326fe51123ab01ed",
        ]
        .map(hex_to_id);

        let result = traverse_both([tip], [end], &odb, topo::Sorting::DateOrder, Parents::All)?;
        assert_eq!(result, expected);

        // Verify against baseline
        let baseline = read_baseline(&fixture_dir()?, "date-order")?;
        let expected_strs: Vec<_> = expected.iter().map(std::string::ToString::to_string).collect();
        assert_eq!(expected_strs, baseline, "Baseline must match the expectation");

        Ok(())
    }
}
