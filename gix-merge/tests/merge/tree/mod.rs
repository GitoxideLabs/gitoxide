use crate::tree::baseline::Deviation;
use gix_diff::Rewrites;
use gix_merge::commit::Options;
use gix_merge::tree::apply_index_entries::RemovalMode;
use gix_merge::tree::{treat_as_unresolved, TreatAsUnresolved};
use gix_object::Write;
use gix_worktree::stack::state::attributes;
use std::path::Path;

/// ### How to add a new baseline test
///
/// 1. Add it to the `tree_baseline.sh` script and don't forget to call the
///    `baseline` function there with the respective parameters.
/// 2. Run all tests - maybe it works, if so, jump to the last point.
/// 3. Change `let new_test = None` to `… = Some("case-name")` to focus on the
///    newly added test and its reversed version.
/// 4. Make it work, then set the `let new_test = Some(…)` back to `… = None`.
/// 5. Validate that all tests are still working, and adjust the expected number of cases
///    in the assertion that would then fail.
#[test]
fn run_baseline() -> crate::Result {
    let root = gix_testtools::scripted_fixture_read_only("tree-baseline.sh")?;
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    let mut actual_cases = 0;
    let mut skipped_tree_resolve_cases = 0;
    // let new_test = Some("tree-to-non-tree-with-rename-A-B");
    let new_test = None;
    for baseline::Expectation {
        root,
        conflict_style,
        odb,
        our_commit_id,
        our_side_name,
        their_commit_id,
        their_side_name,
        merge_info,
        case_name,
        deviation,
    } in baseline::Expectations::new(&root, &cases)
        .filter(|case| new_test.map_or(true, |prefix: &str| case.case_name.starts_with(prefix)))
    {
        actual_cases += 1;
        let mut graph = gix_revwalk::Graph::new(&odb, None);
        let large_file_threshold_bytes = 100;
        let mut blob_merge = new_blob_merge_platform(&root, large_file_threshold_bytes);
        let mut diff_resource_cache = new_diff_resource_cache(&root);
        let mut options = basic_merge_options();
        options.tree_merge.blob_merge.text.conflict = gix_merge::blob::builtin_driver::text::Conflict::Keep {
            style: conflict_style,
            marker_size: gix_merge::blob::builtin_driver::text::Conflict::DEFAULT_MARKER_SIZE
                .try_into()
                .expect("non-zero"),
        };

        let mut actual = gix_merge::commit(
            our_commit_id,
            their_commit_id,
            gix_merge::blob::builtin_driver::text::Labels {
                ancestor: None,
                current: Some(our_side_name.as_str().into()),
                other: Some(their_side_name.as_str().into()),
            },
            &mut graph,
            &mut diff_resource_cache,
            &mut blob_merge,
            &odb,
            &mut |id| id.to_hex_with_len(7).to_string(),
            options.clone(),
        )?
        .tree_merge;

        let actual_id = actual.tree.write(|tree| odb.write(tree))?;
        match &deviation {
            None => {
                if actual_id != merge_info.merged_tree {
                    baseline::show_diff_and_fail(&case_name, actual_id, &actual, &merge_info, &odb);
                }
            }
            Some(Deviation {
                message,
                expected_tree_id,
            }) => {
                // TODO: why did this ever work?
                // Sometimes only the reversed part of a specific test is different.
                // assert_eq!(
                //     actual_id, merge_info.merged_tree,
                //     "{case_name}: Git should match here, it just doesn't match in one of two cases"
                // );
                pretty_assertions::assert_str_eq!(
                    baseline::visualize_tree(&actual_id, &odb, None).to_string(),
                    baseline::visualize_tree(expected_tree_id, &odb, None).to_string(),
                    "{case_name}: tree mismatch: {message} \n{:#?}\n{case_name}",
                    actual.conflicts
                );
            }
        }

        let mut actual_index = gix_index::State::from_tree(&actual_id, &odb, Default::default())?;
        let expected_index = {
            let deviating_index_path = root.join(".git").join(format!("{case_name}.index"));
            if deviating_index_path.exists() {
                gix_index::File::at(
                    deviating_index_path,
                    odb.store().object_hash(),
                    true,
                    Default::default(),
                )?
                .into()
            } else {
                let mut index = actual_index.clone();
                if let Some(conflicts) = &merge_info.conflicts {
                    baseline::apply_git_index_entries(conflicts, &mut index);
                }
                index
            }
        };
        let conflicts_like_in_git = TreatAsUnresolved::git();
        let did_change =
            actual.index_changed_after_applying_conflicts(&mut actual_index, conflicts_like_in_git, RemovalMode::Prune);

        pretty_assertions::assert_eq!(
            baseline::clear_entries(&actual_index),
            baseline::clear_entries(&expected_index),
            "{case_name}: index mismatch\n{:#?}\n{:#?}",
            actual.conflicts,
            merge_info.conflicts
        );
        if deviation.is_none() {
            assert_eq!(
                did_change,
                actual.has_unresolved_conflicts(conflicts_like_in_git),
                "{case_name}: If there is any kind of conflict, the index should have been changed"
            );
        }

        // The content-merge mode is not relevant for the upcoming tree-conflict resolution.
        if case_name.contains("diff3") {
            continue;
        }

        for tree_resolution in [
            gix_merge::tree::ResolveWith::Ancestor,
            gix_merge::tree::ResolveWith::Ours,
        ] {
            let resolution_name = match tree_resolution {
                gix_merge::tree::ResolveWith::Ancestor => "ancestor",
                gix_merge::tree::ResolveWith::Ours => "ours",
            };
            let basename = format!("resolve-{our_side_name}-{their_side_name}-with-{resolution_name}");
            let tree_path = root.join(".git").join(format!("{basename}.tree"));
            if !tree_path.exists() {
                skipped_tree_resolve_cases += 1;
                continue;
            }
            let expected_tree_id = gix_hash::ObjectId::from_hex(std::fs::read_to_string(tree_path)?.trim().as_bytes())?;
            options.tree_merge.tree_conflicts = Some(tree_resolution);
            let resolve_with_ours = tree_resolution == gix_merge::tree::ResolveWith::Ours;
            if resolve_with_ours {
                options.tree_merge.blob_merge.text.conflict =
                    gix_merge::blob::builtin_driver::text::Conflict::ResolveWithOurs;
            }
            let mut actual = gix_merge::commit(
                our_commit_id,
                their_commit_id,
                gix_merge::blob::builtin_driver::text::Labels {
                    ancestor: None,
                    current: Some(our_side_name.as_str().into()),
                    other: Some(their_side_name.as_str().into()),
                },
                &mut graph,
                &mut diff_resource_cache,
                &mut blob_merge,
                &odb,
                &mut |id| id.to_hex_with_len(7).to_string(),
                options.clone(),
            )?
            .tree_merge;

            let actual_id = actual.tree.write(|tree| odb.write(tree))?;
            if actual_id != expected_tree_id {
                baseline::show_diff_trees_and_fail(&case_name, actual_id, &actual, expected_tree_id, &basename, &odb);
            }
            if resolve_with_ours {
                assert!(
                    !actual.has_unresolved_conflicts(conflicts_like_in_git),
                    "We have forcefully resolved all conflicts, as far as Git would be concerned\n{:#?}",
                    actual.conflicts
                );
                assert!(
                    actual.has_unresolved_conflicts(TreatAsUnresolved {
                        content_merge: treat_as_unresolved::ContentMerge::ForcedResolution,
                        tree_merge: treat_as_unresolved::TreeMerge::ForcedResolution,
                    }),
                    "But it's possible to adjust the parameter to still learn that a conflict happened,\
                and each of these tests has one that is irreconcilable"
                );
            }
        }
    }

    assert_eq!(
        actual_cases, 117,
        "BUG: update this number, and don't forget to remove a filter in the end"
    );
    assert_eq!(
        skipped_tree_resolve_cases, 102,
        "this is done when no case is skipped, and we don't want to accidentally skip them.\
        Some don't actually have conflicts.\
        The ones we skipped don't have irreconcilable conflicts"
    );

    Ok(())
}

fn basic_merge_options() -> Options {
    gix_merge::commit::Options {
        allow_missing_merge_base: true,
        use_first_merge_base: false,
        tree_merge: gix_merge::tree::Options {
            symlink_conflicts: None,
            tree_conflicts: None,
            rewrites: Some(Rewrites {
                copies: None,
                percentage: Some(0.5),
                limit: 0,
                track_empty: false,
            }),
            blob_merge: gix_merge::blob::platform::merge::Options::default(),
            blob_merge_command_ctx: Default::default(),
            fail_on_conflict: None,
            marker_size_multiplier: 0,
        },
    }
}

fn new_diff_resource_cache(root: &Path) -> gix_diff::blob::Platform {
    gix_diff::blob::Platform::new(
        Default::default(),
        gix_diff::blob::Pipeline::new(Default::default(), Default::default(), Vec::new(), Default::default()),
        Default::default(),
        gix_worktree::Stack::new(
            root,
            gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::default()),
            Default::default(),
            Vec::new(),
            Vec::new(),
        ),
    )
}

fn new_blob_merge_platform(
    root: &Path,
    large_file_threshold_bytes: impl Into<Option<u64>>,
) -> gix_merge::blob::Platform {
    let attributes = gix_worktree::Stack::new(
        root,
        gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::new(
            Default::default(),
            None,
            attributes::Source::WorktreeThenIdMapping,
            Default::default(),
        )),
        gix_worktree::glob::pattern::Case::Sensitive,
        Vec::new(),
        Vec::new(),
    );
    let filter = gix_merge::blob::Pipeline::new(
        Default::default(),
        gix_filter::Pipeline::default(),
        gix_merge::blob::pipeline::Options {
            large_file_threshold_bytes: large_file_threshold_bytes.into().unwrap_or_default(),
        },
    );
    gix_merge::blob::Platform::new(
        filter,
        gix_merge::blob::pipeline::Mode::ToGit,
        attributes,
        vec![],
        Default::default(),
    )
}

mod baseline;
