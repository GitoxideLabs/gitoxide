use gix_error::TestResult;
use gix_revision::merge_base;

use crate::odb_at;

mod ahead_behind_counts;
mod baseline;
mod is_ancestor;
mod octopus;

#[test]
fn lookup_failures_retain_their_causes() {
    let mut error_snapshots = Vec::new();
    use gix_error::ErrorExt;

    struct FailingLookup(std::io::ErrorKind);

    impl gix_object::Find for FailingLookup {
        fn try_find<'a>(
            &self,
            _id: &gix_hash::oid,
            _buffer: &'a mut Vec<u8>,
        ) -> gix_error::Result<Option<gix_object::Data<'a>>> {
            Err(std::io::Error::from(self.0).raise())
        }
    }

    for kind in [std::io::ErrorKind::NotFound, std::io::ErrorKind::TimedOut] {
        let mut graph = gix_revision::Graph::new(FailingLookup(kind), None);
        let hash = gix_testtools::object_hash();
        let err = merge_base(hash.null(), &[hash.empty_blob()], &mut graph)
            .expect_err("the custom object store always fails");
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&(err), &[]));
        assert_eq!(
            err.downcast_any_ref::<std::io::Error>().map(std::io::Error::kind),
            Some(kind),
            "merge-base context preserves the concrete lookup failure"
        );
        assert_eq!(err.is_not_found(), kind == std::io::ErrorKind::NotFound);
        assert_eq!(err.can_retry(), kind == std::io::ErrorKind::TimedOut);
    }
    insta::assert_debug_snapshot!(error_snapshots, "lookup failures retain their causes", @"
    [
        could not insert commit into graph
        
        Caused by:
            0: entity not found,
        could not insert commit into graph
        
        Caused by:
            0: timed out,
    ]
    ");
}

#[test]
fn validate() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let mut count = 0;
    let odb = odb_at(root.join(".git/objects"))?;
    for baseline_path in baseline::expectation_paths(&root)? {
        count += 1;
        for use_commitgraph in [false, true] {
            let cache = use_commitgraph
                .then(|| {
                    gix_commitgraph::Graph::from_info_dir(
                        &odb.store_ref().path().join("info"),
                        odb.store_ref().object_hash(),
                    )
                })
                .transpose()?;
            for expected in baseline::parse_expectations(&baseline_path)? {
                let mut graph = gix_revision::Graph::new(&odb, cache.as_ref());
                let actual = merge_base(expected.first, &expected.others, &mut graph)?;
                assert_eq!(
                    actual,
                    expected.bases,
                    "sample {file:?}:{input}",
                    file = baseline_path.with_extension("").file_name(),
                    input = expected.plain_input
                );
            }
            let mut graph = gix_revision::Graph::new(&odb, cache.as_ref());
            for expected in baseline::parse_expectations(&baseline_path)? {
                let actual = merge_base(expected.first, &expected.others, &mut graph)?;
                assert_eq!(
                    actual,
                    expected.bases,
                    "sample (reused graph) {file:?}:{input}",
                    file = baseline_path.with_extension("").file_name(),
                    input = expected.plain_input
                );
            }
        }
    }
    assert_ne!(count, 0, "there must be at least one baseline");
    Ok(())
}

#[test]
fn exhausted_side_skips_unrelated_history() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let tip_commit_id = tag_commit_id(&root, "PL")?;
    let base_commit_id = tag_commit_id(&root, "C2")?;
    let unrelated_commit_id = tag_commit_id(&root, "L0")?;

    // PL merges the C and L chains. Once C2 is found, the rest of L cannot
    // provide another merge base, even though its queued commits are not stale.
    for use_commitgraph in [false, true] {
        let cache = use_commitgraph
            .then(|| {
                gix_commitgraph::Graph::from_info_dir(
                    &odb.store_ref().path().join("info"),
                    odb.store_ref().object_hash(),
                )
            })
            .transpose()?;
        for (first_commit_id, other_commit_id) in [(tip_commit_id, base_commit_id), (base_commit_id, tip_commit_id)] {
            let mut graph = gix_revision::Graph::new(&odb, cache.as_ref());
            for others in [
                &[other_commit_id][..],
                &[other_commit_id, other_commit_id][..],
                &[other_commit_id][..],
            ] {
                assert_eq!(
                    merge_base(first_commit_id, others, &mut graph)?,
                    Some(nonempty::NonEmpty::new(base_commit_id)),
                    "the pending common ancestor survives side exhaustion, duplicates, and graph reuse"
                );
                assert_eq!(
                    graph.contains(&unrelated_commit_id),
                    !use_commitgraph,
                    "only reliable generation ordering lets the walk skip the unrelated L0 ancestor"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn unreliable_generations_do_not_allow_side_exhaustion() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    // G and H share B, but clock skew visits B's ancestor E first. Missing,
    // zero, or saturated generations cannot prevent an exhausted color from returning.
    // Zero represents a legacy commit-graph without computed generations.
    let first_commit_id = tag_commit_id(&root, "G")?;
    let other_commit_id = tag_commit_id(&root, "H")?;
    let base_commit_id = tag_commit_id(&root, "B")?;
    for generation in [None, Some(0), Some(gix_commitgraph::GENERATION_NUMBER_MAX)] {
        let mut graph = gix_revision::Graph::new(&odb, None);
        for name in ["A", "B", "C", "D", "E", "F", "G", "H"] {
            graph.get_or_insert_full_commit(tag_commit_id(&root, name)?, |commit| {
                commit.generation = generation;
            })?;
        }
        for (first_commit_id, other_commit_id) in
            [(first_commit_id, other_commit_id), (other_commit_id, first_commit_id)]
        {
            assert_eq!(
                merge_base(first_commit_id, &[other_commit_id, other_commit_id], &mut graph)?,
                Some(nonempty::NonEmpty::new(base_commit_id)),
                "generation {generation:?} requires finishing the date-ordered walk despite temporary side exhaustion"
            );
        }
    }
    Ok(())
}

fn tag_commit_id(root: &std::path::Path, name: &str) -> TestResult<gix_hash::ObjectId> {
    Ok(gix_hash::ObjectId::from_hex(
        std::fs::read_to_string(root.join(".git/refs/tags").join(name))?
            .trim()
            .as_bytes(),
    )?)
}
