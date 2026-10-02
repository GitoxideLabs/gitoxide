use crate::Result;
use gix_revision::merge_base;

use crate::odb_at;

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
        |
        └─ entity not found,
        could not insert commit into graph
        |
        └─ timed out,
    ]
    ");
}

#[test]
fn validate() -> Result {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let mut count = 0;
    let odb = odb_at(root.join(".git/objects"))?;
    for baseline_path in baseline::expectation_paths(&root)? {
        count += 1;
        for use_commitgraph in [false, true] {
            let cache = use_commitgraph
                .then(|| gix_commitgraph::Graph::from_info_dir(&odb.store_ref().path().join("info")).unwrap());
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
fn exhausted_side_skips_unrelated_history() -> Result {
    let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
    let odb = odb_at(root.join(".git/objects"))?;
    let tip_commit_id = tag_commit_id(&root, "PL")?;
    let base_commit_id = tag_commit_id(&root, "C2")?;
    let unrelated_commit_id = tag_commit_id(&root, "L0")?;

    // PL merges the C and L chains. Once C2 is found, the rest of L cannot
    // provide another merge base, even though its queued commits are not stale.
    for use_commitgraph in [false, true] {
        let cache = use_commitgraph
            .then(|| gix_commitgraph::Graph::from_info_dir(&odb.store_ref().path().join("info")))
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
fn unreliable_generations_do_not_allow_side_exhaustion() -> Result {
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

fn tag_commit_id(root: &std::path::Path, name: &str) -> Result<gix_hash::ObjectId> {
    Ok(gix_hash::ObjectId::from_hex(
        std::fs::read_to_string(root.join(".git/refs/tags").join(name))?
            .trim()
            .as_bytes(),
    )?)
}

mod octopus {
    use crate::Result;
    use crate::{hex_to_id, odb_at};

    #[test]
    fn three_sequential_commits() -> Result {
        let odb = octopus_odb_at("three-sequential-commits")?;
        let mut graph = gix_revision::Graph::new(&odb, None);
        let first_commit = hex_to_id("e5d0542bd38431f105a8de8e982b3579647feb9f");
        let mut heads = vec![
            hex_to_id("4fbed377d3eab982d4a465cafaf34b64207da847"),
            hex_to_id("8bc2f99c9aacf07568a2bbfe1269f6e543f22d6b"),
            first_commit,
        ];
        let mut heap = permutohedron::Heap::new(&mut heads);
        while let Some(heads) = heap.next_permutation() {
            let actual =
                gix_revision::merge_base::octopus(*heads.first().expect("three heads"), &heads[1..], &mut graph)?
                    .expect("a merge base");
            assert_eq!(actual, first_commit);
        }
        Ok(())
    }

    #[test]
    fn three_parallel_commits() -> Result {
        let odb = octopus_odb_at("three-parallel-commits")?;
        let mut graph = gix_revision::Graph::new(&odb, None);
        let base = hex_to_id("3ca3e3dd12585fabbef311d524a5e54678090528");
        let mut heads = vec![
            hex_to_id("4ce66b336dff547fdeb6cd86e04c617c8d998ff5"),
            hex_to_id("6291f6d7da04208dc4ccbbdf9fda98ac9ae67bc0"),
            hex_to_id("c507d5413da00c32e5de1ea433030e8e4716bc60"),
        ];
        let mut heap = permutohedron::Heap::new(&mut heads);
        while let Some(heads) = heap.next_permutation() {
            let actual =
                gix_revision::merge_base::octopus(*heads.first().expect("three heads"), &heads[1..], &mut graph)?
                    .expect("a merge base");
            assert_eq!(actual, base);
        }
        Ok(())
    }

    #[test]
    fn three_forked_commits() -> Result {
        let odb = octopus_odb_at("three-forked-commits")?;
        let mut graph = gix_revision::Graph::new(&odb, None);
        let base = hex_to_id("3ca3e3dd12585fabbef311d524a5e54678090528");
        let mut heads = vec![
            hex_to_id("413d38a3fe7453c68cb7314739d7775f68ab89f5"),
            hex_to_id("d4d01a9b6f6fcb23d57cd560229cd9680ec9bd6e"),
            hex_to_id("c507d5413da00c32e5de1ea433030e8e4716bc60"),
        ];
        let mut heap = permutohedron::Heap::new(&mut heads);
        while let Some(heads) = heap.next_permutation() {
            let actual =
                gix_revision::merge_base::octopus(*heads.first().expect("three heads"), &heads[1..], &mut graph)?
                    .expect("a merge base");
            assert_eq!(actual, base);
        }
        Ok(())
    }

    fn octopus_odb_at(name: &str) -> Result<gix_odb::Handle> {
        let root = gix_testtools::scripted_fixture_read_only("merge_base_octopus_repos.sh")?;
        odb_at(root.join(name).join(".git/objects"))
    }
}

mod is_ancestor {
    use gix_revision::merge_base::is_ancestor;

    use super::{baseline, tag_commit_id};
    use crate::{Result, odb_at};

    #[test]
    fn validate() -> Result {
        let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
        let odb = odb_at(root.join(".git/objects"))?;
        let mut count = 0;
        for use_commitgraph in [false, true] {
            let cache = use_commitgraph
                .then(|| gix_commitgraph::Graph::from_info_dir(&odb.store_ref().path().join("info")))
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
    fn reliable_generations_limit_traversal() -> Result {
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
                .then(|| gix_commitgraph::Graph::from_info_dir(&odb.store_ref().path().join("info")))
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
    fn commits_outside_the_commit_graph_are_not_ancestors_of_commits_inside() -> Result {
        let root = gix_testtools::scripted_fixture_read_only("make_merge_base_repos.sh")?;
        let odb = odb_at(root.join(".git/objects"))?;
        let cache = gix_commitgraph::Graph::from_info_dir(&odb.store_ref().path().join("info"))?;
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
    fn unreliable_generations_are_ignored() -> Result {
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
}

mod baseline {
    use std::{
        ffi::OsStr,
        path::{Path, PathBuf},
    };

    use bstr::ByteSlice;
    use gix_hash::ObjectId;
    use nonempty::NonEmpty;

    /// The expectation as produced by Git itself
    #[derive(Debug)]
    pub struct Expectation {
        pub plain_input: String,
        pub first: ObjectId,
        pub others: Vec<ObjectId>,
        pub bases: Option<NonEmpty<ObjectId>>,
    }

    pub fn parse_expectations(baseline: &Path) -> std::io::Result<Vec<Expectation>> {
        let lines = std::fs::read(baseline)?;
        let mut lines = lines.lines();
        let mut out = Vec::new();
        while let Some(plain_input) = lines.next() {
            let plain_input = plain_input.to_str_lossy().into_owned();
            let mut input = lines
                .next()
                .expect("second line is resolved input objects")
                .split(|b| *b == b' ');
            let first = ObjectId::from_hex(input.next().expect("at least one object")).unwrap();
            let others = input.map(|hex_id| ObjectId::from_hex(hex_id).unwrap()).collect();
            let bases: Vec<_> = lines
                .by_ref()
                .take_while(|l| !l.is_empty())
                .map(|hex_id| ObjectId::from_hex(hex_id).unwrap())
                .collect();
            out.push(Expectation {
                plain_input,
                first,
                others,
                bases: NonEmpty::from_vec(bases),
            });
        }
        Ok(out)
    }

    pub fn expectation_paths(root: &Path) -> std::io::Result<Vec<PathBuf>> {
        let mut out: Vec<_> = std::fs::read_dir(root)?
            .map(std::result::Result::unwrap)
            .filter_map(|e| (e.path().extension() == Some(OsStr::new("baseline"))).then(|| e.path()))
            .collect();
        out.sort();
        Ok(out)
    }
}
