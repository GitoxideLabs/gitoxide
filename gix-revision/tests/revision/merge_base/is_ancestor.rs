use gix_error::TestResult;
use gix_revision::merge_base::is_ancestor;

use super::{baseline, tag_commit_id};
use crate::odb_at;

#[test]
fn validate() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let mut count = 0;
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
        for baseline_path in baseline::expectation_paths(&root)? {
            for expected in baseline::parse_expectations(&baseline_path)? {
                let &[descendant_commit_id] = expected.others.as_slice() else {
                    continue;
                };
                count += 1;
                // `git merge-base --is-ancestor A B` succeeds exactly if `A` is the only merge-base of `A` and `B`.
                let expected_is_ancestor = expected.bases == Some(nonempty::NonEmpty::new(expected.first));
                let mut new_graph = gix_revision::Graph::new(&odb, cache.as_ref());
                for (graph, graph_kind) in [(&mut new_graph, "new"), (&mut reused_graph, "reused")] {
                    assert_eq!(
                        is_ancestor(expected.first, descendant_commit_id, graph)?,
                        expected_is_ancestor,
                        "sample {file:?}:{input} with a {graph_kind} graph, commit-graph: {use_commitgraph}",
                        file = baseline_path.with_extension("").file_name(),
                        input = expected.plain_input
                    );
                }
            }
        }
    }
    assert_ne!(count, 0, "there must be at least one pair of commits to check");
    Ok(())
}

#[test]
fn reliable_generations_limit_traversal() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    // PL merges L2 and C2, PR merges C2 and R2, and the L, C and R chains all start at S.
    let pl_commit_id = tag_commit_id(&root, "PL")?;
    let l2_commit_id = tag_commit_id(&root, "L2")?;
    let c2_commit_id = tag_commit_id(&root, "C2")?;
    let r2_commit_id = tag_commit_id(&root, "R2")?;
    let s_commit_id = tag_commit_id(&root, "S")?;
    for use_commitgraph in [false, true] {
        let cache = use_commitgraph
            .then(|| {
                gix_commitgraph::Graph::from_info_dir(
                    &odb.store_ref().path().join("info"),
                    odb.store_ref().object_hash(),
                )
            })
            .transpose()?;

        let mut graph = gix_revision::Graph::new(&odb, cache.as_ref());
        assert!(
            is_ancestor(c2_commit_id, pl_commit_id, &mut graph)?,
            "C2 is a parent of PL"
        );

        let mut graph = gix_revision::Graph::new(&odb, cache.as_ref());
        assert!(
            !is_ancestor(r2_commit_id, pl_commit_id, &mut graph)?,
            "R2 is only reachable from PR"
        );
        assert_eq!(
            graph.contains(&s_commit_id),
            !use_commitgraph,
            "only reliable generations end the walk before it reaches the shared root S"
        );

        let mut graph = gix_revision::Graph::new(&odb, cache.as_ref());
        assert!(
            !is_ancestor(pl_commit_id, c2_commit_id, &mut graph)?,
            "a child isn't an ancestor of its parent"
        );
        assert_eq!(
            graph.contains(&l2_commit_id),
            !use_commitgraph,
            "only reliable generations answer without visiting PL's parents, as PL's generation isn't lower than C2's"
        );
    }
    Ok(())
}

#[test]
fn commits_outside_the_commit_graph_are_not_ancestors_of_commits_inside() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let cache =
        gix_commitgraph::Graph::from_info_dir(&odb.store_ref().path().join("info"), odb.store_ref().object_hash())?;
    let pl_commit_id = tag_commit_id(&root, "PL")?;
    let l2_commit_id = tag_commit_id(&root, "L2")?;
    let c2_commit_id = tag_commit_id(&root, "C2")?;
    let r2_commit_id = tag_commit_id(&root, "R2")?;

    // Pretend that R2 was committed after the commit-graph was written, which leaves it without a generation.
    let mut graph = gix_revision::Graph::new(&odb, Some(&cache));
    graph.get_or_insert_full_commit(r2_commit_id, |commit| {
        commit.generation = None;
    })?;
    assert!(
        !is_ancestor(r2_commit_id, pl_commit_id, &mut graph)?,
        "a commit-graph contains all ancestors of its commits, so R2 can't be an ancestor of PL"
    );
    assert!(
        !graph.contains(&l2_commit_id),
        "that is known without visiting the parents of PL"
    );

    // The other way around, a commit without a generation can still have ancestors in the commit-graph.
    let mut graph = gix_revision::Graph::new(&odb, Some(&cache));
    graph.get_or_insert_full_commit(pl_commit_id, |commit| {
        commit.generation = None;
    })?;
    assert!(
        is_ancestor(c2_commit_id, pl_commit_id, &mut graph)?,
        "C2 is a parent of PL, whose generation is unknown"
    );
    Ok(())
}

#[test]
fn unreliable_generations_are_ignored() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    // Missing, zero or saturated generations can neither answer without traversal nor end it early.
    // The commit times of this graph can't be trusted either, as the root E is newer than some of its descendants.
    for generation in [None, Some(0), Some(gix_commitgraph::GENERATION_NUMBER_MAX)] {
        let mut graph = gix_revision::Graph::new(&odb, None);
        for name in ["A", "B", "C", "D", "E", "F", "G", "H"] {
            graph.get_or_insert_full_commit(tag_commit_id(&root, name)?, |commit| {
                commit.generation = generation;
            })?;
        }
        for (ancestor, descendant, expected) in
            [("B", "G", true), ("E", "H", true), ("G", "H", false), ("D", "F", false)]
        {
            let ancestor_commit_id = tag_commit_id(&root, ancestor)?;
            let descendant_commit_id = tag_commit_id(&root, descendant)?;
            assert_eq!(
                is_ancestor(ancestor_commit_id, descendant_commit_id, &mut graph)?,
                expected,
                "{ancestor} as ancestor of {descendant} is decided by traversal with generation {generation:?}"
            );
        }
    }
    Ok(())
}
