use std::collections::BTreeMap;

use gix_error::TestResult;
use gix_revision::{Graph, contains::Query, graph::Commit, merge_base::Flags};

use super::{baseline, tag_commit_id};
use crate::odb_at;

#[test]
fn many_tips_match_git_across_queries_on_one_graph() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let disk = gix_commitgraph::Graph::from_info_dir(&root.join(".git/objects/info"), gix_testtools::object_hash())?;
    let mut queries = BTreeMap::<_, Vec<_>>::new();
    for path in baseline::expectation_paths(&root)? {
        for row in baseline::parse_expectations(&path)? {
            if let &[tip] = row.others.as_slice() {
                let contains = row.bases == Some(nonempty::NonEmpty::new(row.first));
                queries.entry(row.first).or_default().push((tip, contains));
            }
        }
    }
    assert!(
        queries.values().any(|tips| tips.len() > 1),
        "Git's baseline has multi-tip queries"
    );
    for cache in [None, Some(&disk)] {
        let mut graph = Graph::<Commit<Flags>>::new(&odb, cache);
        for (&target, tips) in &queries {
            let mut query = Query::new(target, &mut graph)?;
            assert!(query.contains(target)?, "each commit contains itself");
            for &(tip, expected) in tips {
                assert_eq!(query.contains(tip)?, expected, "{target} in {tip} matches Git");
            }
        }
    }
    Ok(())
}

#[test]
fn generations_prune_history_below_the_target() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let disk = gix_commitgraph::Graph::from_info_dir(&root.join(".git/objects/info"), gix_testtools::object_hash())?;
    let target = tag_commit_id(&root, "R2")?;
    let left_tip = tag_commit_id(&root, "PL")?;
    let right_tip = tag_commit_id(&root, "PR")?;
    let shared_root = tag_commit_id(&root, "S")?;
    for cache in [None, Some(&disk)] {
        let mut graph = Graph::<Commit<()>>::new(&odb, cache);
        {
            let mut query = Query::new(target, &mut graph)?;
            assert!(!query.contains(left_tip)?, "R2 is only on the right side of the graph");
            assert!(
                query.contains(right_tip)?,
                "reaching R2 through a later merge parent counts"
            );
        }
        assert_eq!(
            graph.contains(&shared_root),
            cache.is_none(),
            "only a usable commit-graph stops the walk before the shared root"
        );
    }
    Ok(())
}

#[test]
fn errors_leave_the_query_reusable() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_ancestor_missing_parent_repo.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let base = tag_commit_id(&root, "base")?;
    let tip = tag_commit_id(&root, "tip")?;
    let missing_parent = tag_commit_id(&root, "parent")?;
    let mut graph = Graph::<Commit<()>>::new(&odb, None);
    assert!(Query::new(missing_parent, &mut graph).is_err(), "the target must exist");
    {
        let mut query = Query::new(base, &mut graph)?;
        for _ in 0..2 {
            let err = query.contains(tip).expect_err("a traversed parent is missing");
            assert!(err.is_not_found(), "a retry retains the missing-parent error: {err}");
            assert!(
                err.to_string().contains(&missing_parent.to_string()),
                "the error names the parent"
            );
        }
        assert!(
            query.contains(base)?,
            "a different tip remains queryable after an error"
        );
    }

    graph.get_or_insert_full_commit(tip, |commit| {
        commit.parents.clear();
        commit.parents.push(tip);
    })?;
    let mut query = Query::new(base, &mut graph)?;
    let err = query.contains(tip).expect_err("a cycle cannot be walked");
    assert!(err.to_string().contains("cycle"), "the graph cycle is reported: {err}");
    Ok(())
}
