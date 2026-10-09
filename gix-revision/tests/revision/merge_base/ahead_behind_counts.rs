use gix_error::TestResult;
use gix_revision::{ahead_behind_counts, ahead_behind_counts::Outcome};

use super::{baseline, tag_commit_id};
use crate::odb_at;

#[test]
fn counts_match_git() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let mut samples = Vec::new();
    for baseline_path in baseline::expectation_paths(&root)? {
        for expected in baseline::parse_expectations(&baseline_path)? {
            let &[base_commit_id] = expected.others.as_slice() else {
                continue;
            };
            let expected_counts = expected.ahead_behind.expect("pairs have Git's recorded counts");
            samples.push((expected.first, base_commit_id, expected_counts, expected.plain_input));
        }
    }
    assert!(!samples.is_empty(), "the fixture must provide pairs of commits");
    for use_commitgraph in [false, true] {
        let cache = use_commitgraph
            .then(|| {
                gix_commitgraph::Graph::from_info_dir(
                    &odb.store_ref().path().join("info"),
                    odb.store_ref().object_hash(),
                )
            })
            .transpose()?;
        let mut reused_graph = gix_revision::Graph::new(&odb, cache.as_ref());
        for (tip_commit_id, base_commit_id, expected, input) in &samples {
            let mut new_graph = gix_revision::Graph::new(&odb, cache.as_ref());
            for (graph, graph_kind) in [(&mut new_graph, "new"), (&mut reused_graph, "reused")] {
                assert_eq!(
                    ahead_behind_counts(*tip_commit_id, *base_commit_id, graph)?,
                    *expected,
                    "{input} with a {graph_kind} graph, commit-graph: {use_commitgraph}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn counts_both_sides_after_one_side_is_exhausted() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let cache =
        gix_commitgraph::Graph::from_info_dir(&odb.store_ref().path().join("info"), odb.store_ref().object_hash())?;
    let pl_commit_id = tag_commit_id(&root, "PL")?;
    let c2_commit_id = tag_commit_id(&root, "C2")?;
    let l0_commit_id = tag_commit_id(&root, "L0")?;
    let mut graph = gix_revision::Graph::new(&odb, Some(&cache));
    assert_eq!(
        ahead_behind_counts(pl_commit_id, c2_commit_id, &mut graph)?,
        Outcome { ahead: 4, behind: 0 },
        "PL and its exclusive L chain must all be counted after the common C2 is found"
    );
    assert!(graph.contains(&l0_commit_id), "the entire exclusive L chain is visited");
    assert_eq!(
        ahead_behind_counts(c2_commit_id, pl_commit_id, &mut graph)?,
        Outcome { ahead: 0, behind: 4 },
        "swapping the tips swaps counts even when reusing the graph"
    );
    assert!(
        gix_revision::merge_base::is_ancestor(c2_commit_id, pl_commit_id, &mut graph)?,
        "the graph can also be reused for an ancestor check"
    );
    let mut graph = gix_revision::Graph::new(&odb, Some(&cache));
    assert_eq!(
        ahead_behind_counts(tag_commit_id(&root, "A")?, tag_commit_id(&root, "B")?, &mut graph)?,
        Outcome { ahead: 1, behind: 0 },
        "only A is exclusive to A relative to its parent B"
    );
    assert!(
        !graph.contains(&tag_commit_id(&root, "C")?),
        "reliable generations let the walk stop before loading B's parents"
    );
    Ok(())
}

#[test]
fn uncached_tips_restore_pruning_at_cached_parents() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let cache =
        gix_commitgraph::Graph::from_info_dir(&odb.store_ref().path().join("info"), odb.store_ref().object_hash())?;
    let base_commit_id = tag_commit_id(&root, "A")?;
    let tip_commit_id = tag_commit_id(&root, "UNCACHED_A")?;
    assert!(
        cache.lookup(base_commit_id).is_some(),
        "the parent is inside the commit-graph"
    );
    assert!(
        cache.lookup(tip_commit_id).is_none(),
        "the new tip is outside the commit-graph"
    );
    let mut graph = gix_revision::Graph::new(&odb, Some(&cache));
    for (tip, base, expected) in [
        (tip_commit_id, base_commit_id, Outcome { ahead: 1, behind: 0 }),
        (base_commit_id, tip_commit_id, Outcome { ahead: 0, behind: 1 }),
    ] {
        assert_eq!(
            ahead_behind_counts(tip, base, &mut graph)?,
            expected,
            "only the uncached child is exclusive"
        );
        assert_eq!(
            graph.len(),
            2,
            "traversal stops at the shared cached parent, including with a reused graph"
        );
    }
    Ok(())
}

#[test]
fn unreliable_generations_do_not_double_count() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let cache =
        gix_commitgraph::Graph::from_info_dir(&odb.store_ref().path().join("info"), odb.store_ref().object_hash())?;
    for generation in [None, Some(0), Some(gix_commitgraph::GENERATION_NUMBER_MAX)] {
        for use_commitgraph in [false, true] {
            let mut graph = gix_revision::Graph::new(&odb, use_commitgraph.then_some(&cache));
            for name in ["A", "B", "C", "D", "E", "F", "G", "H", "PL"] {
                graph.get_or_insert_full_commit(tag_commit_id(&root, name)?, |commit| {
                    commit.generation = generation;
                })?;
            }
            for (tip, base, expected) in [
                ("G", "H", Outcome { ahead: 1, behind: 3 }),
                ("E", "H", Outcome { ahead: 0, behind: 6 }),
                ("PL", "PR", Outcome { ahead: 4, behind: 4 }),
            ] {
                assert_eq!(
                    ahead_behind_counts(tag_commit_id(&root, tip)?, tag_commit_id(&root, base)?, &mut graph)?,
                    expected,
                    "{tip}...{base} with generation {generation:?}, commit-graph: {use_commitgraph}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn invalid_inputs_are_errors_even_when_equal() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let tip_commit_id = tag_commit_id(&root, "PL")?;
    let hash = gix_testtools::object_hash();
    let mut graph = gix_revision::Graph::new(&odb, None);
    for invalid_commit_id in [hash.null(), hash.empty_tree()] {
        for (tip, base) in [
            (invalid_commit_id, invalid_commit_id),
            (tip_commit_id, invalid_commit_id),
            (invalid_commit_id, tip_commit_id),
        ] {
            assert!(
                ahead_behind_counts(tip, base, &mut graph).is_err(),
                "both input commits must be validated, including the equality shortcut"
            );
        }
    }
    Ok(())
}

#[test]
fn missing_parents_end_history_but_lookup_failures_are_errors() -> TestResult {
    use gix_error::ErrorExt;

    struct ParentLookup<'a> {
        odb: &'a gix_odb::Handle,
        parent_commit_id: gix_hash::ObjectId,
        failure: Option<std::io::ErrorKind>,
    }

    impl gix_object::Find for ParentLookup<'_> {
        fn try_find<'a>(
            &self,
            commit_id: &gix_hash::oid,
            buffer: &'a mut Vec<u8>,
        ) -> gix_error::Result<Option<gix_object::Data<'a>>> {
            if commit_id == self.parent_commit_id {
                return match self.failure {
                    Some(kind) => Err(std::io::Error::from(kind).raise()),
                    None => Ok(None),
                };
            }
            self.odb.try_find(commit_id, buffer)
        }
    }

    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    for failure in [None, Some(std::io::ErrorKind::TimedOut)] {
        let mut graph = gix_revision::Graph::new(
            ParentLookup {
                odb: &odb,
                parent_commit_id: tag_commit_id(&root, "B")?,
                failure,
            },
            None,
        );
        let actual = ahead_behind_counts(tag_commit_id(&root, "A")?, tag_commit_id(&root, "DA")?, &mut graph);
        if let Some(kind) = failure {
            let err = actual.expect_err("parent lookup failures must be propagated");
            assert_eq!(
                err.downcast_any_ref::<std::io::Error>().map(std::io::Error::kind),
                Some(kind),
                "traversal context retains the concrete lookup error"
            );
            assert!(err.can_retry(), "a parent lookup timeout remains retryable");
        } else {
            assert_eq!(
                actual?,
                Outcome { ahead: 1, behind: 1 },
                "only available commits are counted when a parent is missing"
            );
        }
    }
    Ok(())
}
