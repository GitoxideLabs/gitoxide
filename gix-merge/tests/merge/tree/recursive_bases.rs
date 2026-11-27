use gix_merge::{blob::builtin_driver::text, tree::TreatAsUnresolved};
use gix_object::Write;
use gix_testtools::TestResult;

use super::{baseline, basic_merge_options, new_blob_merge_platform, new_diff_resource_cache};

#[test]
fn recursive_bases_match_git_with_and_without_commit_graph() -> TestResult {
    let Some(root) =
        gix_testtools::scripted_fixture_read_only_with_git_version("recursive-merge-bases.sh", |version| {
            // Git 2.56 orders v1 graphs by generation too, changing equal-time merge bases.
            version >= (2, 56, 0)
        })?
    else {
        return Ok(());
    };
    for layout in [
        "sibling-depth",
        "skewed-order",
        "equal-times",
        "skewed-equal-times",
        "corrected-order",
    ] {
        let root = root.join(layout);
        let hash = gix_testtools::object_hash();
        let objects = gix_odb::memory::Proxy::new(gix_odb::at(root.join(".git/objects"), hash)?, hash);
        let commits = std::fs::read_to_string(root.join("commits"))?
            .lines()
            .map(|line| gix_hash::ObjectId::from_hex(line.as_bytes()))
            .collect::<Result<Vec<_>, _>>()?;
        for (direction, our_commit_id, their_commit_id) in
            [("A-B", commits[0], commits[1]), ("B-A", commits[1], commits[0])]
        {
            for commit_graph_mode in ["uncached", "v1", "v2", "partial-v2", "mixed"] {
                let commit_graph = (commit_graph_mode != "uncached")
                    .then(|| gix_commitgraph::Graph::from_info_dir(&root.join(commit_graph_mode), hash))
                    .transpose()?;
                let commit_graph = commit_graph.as_ref();
                if let Some(commit_graph) = commit_graph {
                    assert_eq!(
                        commit_graph.corrected_commit_dates_available(),
                        matches!(commit_graph_mode, "v2" | "partial-v2"),
                        "only graphs with corrected dates in every layer use generation version 2"
                    );
                }
                let expected = baseline::parse_merge_info(std::fs::read_to_string(
                    root.join(format!("{direction}-{commit_graph_mode}.merge-info")),
                )?);
                let expected_bases =
                    std::fs::read_to_string(root.join(format!("{direction}-{commit_graph_mode}.bases")))?
                        .lines()
                        .map(|line| gix_hash::ObjectId::from_hex(line.as_bytes()))
                        .collect::<Result<Vec<_>, _>>()?;
                let mut options = basic_merge_options();
                options.tree_merge.blob_merge.text.conflict = text::Conflict::Keep {
                    style: text::ConflictStyle::Diff3,
                    marker_size: text::Conflict::DEFAULT_MARKER_SIZE.try_into()?,
                };
                let mut outcome = gix_merge::commit(
                    our_commit_id,
                    their_commit_id,
                    text::Labels {
                        ancestor: None,
                        current: Some(direction[..1].into()),
                        other: Some(direction[2..].into()),
                    },
                    &mut gix_revwalk::Graph::new(&objects, commit_graph),
                    &mut new_diff_resource_cache(&root),
                    &mut new_blob_merge_platform(&root, 0),
                    &objects,
                    &mut |commit_id| commit_id.to_hex_with_len(7).to_string(),
                    gix_merge::tree::Context::default(),
                    options,
                )?;
                assert_eq!(
                    Vec::from(outcome.merge_bases.clone().expect("the tips share three bases")),
                    expected_bases,
                    "{layout}/{direction}, {commit_graph_mode}: source merge-base order matches Git"
                );
                let tree_id = outcome.tree_merge.tree.write(|tree| objects.write(tree))?;
                pretty_assertions::assert_eq!(
                    baseline::visualize_tree(&tree_id, &objects, None).to_string(),
                    baseline::visualize_tree(&expected.merged_tree, &objects, None).to_string(),
                    "{layout}/{direction}, graph={}: virtual-base ordering and recursion depth match Git",
                    commit_graph.is_some()
                );
                let index =
                    outcome
                        .tree_merge
                        .index
                        .to_index(TreatAsUnresolved::git(), None, &objects, Default::default())?;
                let mut expected_index =
                    gix_index::State::from_tree(&expected.merged_tree, &objects, Default::default())?;
                baseline::apply_git_index_entries(
                    expected.conflicts.as_ref().expect("both tips change file"),
                    &mut expected_index,
                );
                assert_eq!(
                    baseline::debug_entries(&index),
                    baseline::debug_entries(&expected_index),
                    "final stages match Git"
                );
                assert!(
                    outcome.tree_merge.has_unresolved_conflicts(TreatAsUnresolved::git()),
                    "both tips conflict"
                );
            }
        }
    }
    Ok(())
}
