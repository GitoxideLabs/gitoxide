use gix_testtools::TestResult;
use std::path::Path;

use gix_diff::Rewrites;
use gix_merge::{
    commit::Options,
    tree::{TreatAsUnresolved, apply_index_entries::RemovalMode, treat_as_unresolved},
};
use gix_object::Write;
use gix_worktree::stack::state::attributes;

use crate::tree::baseline::Deviation;

/// Keep the production fallback recoverable, but require every conflict exercised by these broad fixtures to have a
/// concrete classification.
fn assert_no_unknown_conflicts(outcome: &gix_merge::tree::Outcome<'_>, context: &str) {
    for conflict in &outcome.conflicts {
        let (resolution, _, _) = conflict.clone().into_parts_by_resolution();
        assert!(
            !matches!(
                resolution,
                Err(gix_merge::tree::ResolutionFailure::Unknown)
                    | Ok(gix_merge::tree::Resolution::Forced(
                        gix_merge::tree::ResolutionFailure::Unknown
                    ))
            ),
            "{context}: all exercised conflict pairs should have a concrete classification: {conflict:#?}"
        );
    }
}

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
fn run_baseline() -> TestResult {
    // Git before 2.50 ignores disabled rename detection; require a compatible baseline archive then.
    let Some(root) = gix_testtools::scripted_fixture_read_only_with_git_version("tree-baseline.sh", |version| {
        version >= (2, 50, 0)
    })?
    else {
        return Ok(());
    };
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    let mut actual_cases = 0;
    let mut skipped_tree_resolve_cases = 0;
    // let new_test = Some("rename-within-rename-2-A-B-deviates");
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
        .filter(|case| new_test.is_none_or(|prefix: &str| case.case_name.starts_with(prefix)))
    {
        actual_cases += 1;
        let mut graph = gix_revwalk::Graph::new(&odb, None);
        let large_file_threshold_bytes = 100;
        let mut blob_merge = new_blob_merge_platform(&root, large_file_threshold_bytes);
        let mut diff_resource_cache = new_diff_resource_cache(&root);
        let mut options = basic_merge_options();
        options.tree_merge.directory_renames = directory_renames_for_case(&case_name);
        if case_name.contains("without-rewrites") {
            options.tree_merge.rewrites = None;
        }
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
        assert_no_unknown_conflicts(&actual, &case_name);

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
            baseline::debug_entries(&actual_index),
            baseline::debug_entries(&expected_index),
            "{case_name}: index mismatch\nOur conflicts {:#?}\nGit conflicts {:#?}",
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
            assert_no_unknown_conflicts(&actual, &basename);

            let actual_id = actual.tree.write(|tree| odb.write(tree))?;
            if actual_id != expected_tree_id {
                baseline::show_diff_trees_and_fail(&case_name, actual_id, &actual, expected_tree_id, &basename, &odb);
            }
            if resolve_with_ours && merge_info.conflicts.is_some() {
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
        actual_cases, 287,
        "BUG: update this number, and don't forget to remove a filter in the end"
    );
    assert_eq!(
        skipped_tree_resolve_cases, 342,
        "this is done when no case is skipped, and we don't want to accidentally skip them.\
        Some don't actually have conflicts.\
        The ones we skipped don't have irreconcilable conflicts"
    );

    Ok(())
}

#[test]
fn directory_rename_controls() -> TestResult {
    use gix_merge::tree::{DirectoryRenames, ResolveWith};

    assert_eq!(
        DirectoryRenames::default(),
        DirectoryRenames::Conflict,
        "the directory rename default matches Git"
    );
    assert_eq!(
        gix_merge::tree::Options::default().directory_renames,
        DirectoryRenames::Conflict,
        "default tree options report inferred file locations as conflicts"
    );
    let Some(root) = gix_testtools::scripted_fixture_read_only_with_git_version("tree-baseline.sh", |version| {
        version >= (2, 50, 0)
    })?
    else {
        return Ok(());
    };
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    let mut actual_cases = 0;
    for case in baseline::Expectations::new(&root, &cases).filter(|case| {
        (case
            .case_name
            .starts_with("directory-rename-paths-directory-renames-conflict")
            || case
                .case_name
                .starts_with("identical-additions-after-directory-rename-directory-renames-conflict"))
            && !case.case_name.contains("without-rewrites")
    }) {
        actual_cases += 1;
        let mut graph = gix_revwalk::Graph::new(&case.odb, None);
        let mut diff_resource_cache = new_diff_resource_cache(&case.root);
        let mut blob_merge = new_blob_merge_platform(&case.root, 0);
        let mut merge = |options| {
            gix_merge::commit(
                case.our_commit_id,
                case.their_commit_id,
                gix_merge::blob::builtin_driver::text::Labels::default(),
                &mut graph,
                &mut diff_resource_cache,
                &mut blob_merge,
                &case.odb,
                &mut |commit_id| commit_id.to_hex_with_len(7).to_string(),
                options,
            )
        };
        let mut options = basic_merge_options();
        options.tree_merge.directory_renames = Default::default();
        options.tree_merge.fail_on_conflict = Some(TreatAsUnresolved::git());
        let actual = merge(options.clone())?.tree_merge;
        assert!(
            actual.failed_on_first_unresolved_conflict,
            "{}: inferred file locations stop the merge even with clean contents",
            case.case_name
        );

        options.tree_merge.fail_on_conflict = None;
        for resolution in [ResolveWith::Ancestor, ResolveWith::Ours] {
            options.tree_merge.tree_conflicts = Some(resolution);
            let mut actual = merge(options.clone())?.tree_merge;
            let actual_tree_id = actual.tree.write(|tree| case.odb.write(tree))?;
            let side = match resolution {
                ResolveWith::Ancestor => "ancestor",
                ResolveWith::Ours => &case.our_side_name,
            };
            let pair_tree_path = case
                .root
                .join(format!(".git/force-{side}-{}.tree", case.their_side_name));
            let tree_path = if pair_tree_path.exists() {
                pair_tree_path
            } else {
                case.root.join(format!(".git/force-{side}.tree"))
            };
            let expected_tree_id = gix_hash::ObjectId::from_hex(std::fs::read_to_string(tree_path)?.trim().as_bytes())?;
            assert_eq!(
                actual_tree_id, expected_tree_id,
                "{}: forced resolution selects the requested side at inferred locations: {:#?}",
                case.case_name, actual.conflicts
            );
            assert!(
                !actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
                "{}: location conflicts were explicitly resolved",
                case.case_name
            );
            assert!(
                actual.has_unresolved_conflicts(TreatAsUnresolved::forced_resolution()),
                "{}: forced location decisions remain observable",
                case.case_name
            );
        }
    }
    assert_eq!(
        actual_cases, 8,
        "all forward and reversed location-conflict cases were exercised"
    );
    Ok(())
}

#[test]
fn directory_renames_are_disabled_in_virtual_merge_bases() -> TestResult {
    use gix_merge::tree::DirectoryRenames;

    let Some(root) = gix_testtools::scripted_fixture_read_only_with_git_version("tree-baseline.sh", |version| {
        version >= (2, 50, 0)
    })?
    else {
        return Ok(());
    };
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    for case in baseline::Expectations::new(&root, &cases).filter(|case| {
        case.case_name == "directory-rename-paths-directory-renames-false"
            || case.case_name == "directory-rename-paths-directory-renames-false-reversed"
    }) {
        let mut graph = gix_revwalk::Graph::new(&case.odb, None);
        let mut diff_resource_cache = new_diff_resource_cache(&case.root);
        let mut blob_merge = new_blob_merge_platform(&case.root, 0);
        for mode in [
            DirectoryRenames::Disabled,
            DirectoryRenames::Conflict,
            DirectoryRenames::Enabled,
        ] {
            let mut options = basic_merge_options().tree_merge;
            options.directory_renames = mode;
            let actual = gix_merge::commit::virtual_merge_base(
                case.our_commit_id,
                case.their_commit_id,
                Vec::new(),
                &mut graph,
                &mut diff_resource_cache,
                &mut blob_merge,
                &case.odb,
                &mut |commit_id| commit_id.to_hex_with_len(7).to_string(),
                options,
            )?;
            assert_eq!(
                actual.tree_id, case.merge_info.merged_tree,
                "{}: virtual bases disable inferred directory relocation, independent of {mode:?}",
                case.case_name
            );
        }
    }
    Ok(())
}

#[test]
fn relocated_rename_conflicts_and_forced_content() -> TestResult {
    use gix_merge::tree::{DirectoryRenames, ResolutionFailure, ResolveWith};

    let root = gix_testtools::scripted_fixture_read_only_needs_archive("tree-baseline.sh")?;
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    let mut actual_cases = 0;
    for case in baseline::Expectations::new(&root, &cases).filter(|case| {
        case.case_name.starts_with("relocated-") && case.case_name.contains("directory-renames-conflict")
    }) {
        actual_cases += 1;
        let mut graph = gix_revwalk::Graph::new(&case.odb, None);
        let mut diff_resource_cache = new_diff_resource_cache(&case.root);
        let mut blob_merge = new_blob_merge_platform(&case.root, 0);
        let mut options = basic_merge_options();
        options.tree_merge.directory_renames = DirectoryRenames::Conflict;
        let mut merge = |options| {
            gix_merge::commit(
                case.our_commit_id,
                case.their_commit_id,
                Default::default(),
                &mut graph,
                &mut diff_resource_cache,
                &mut blob_merge,
                &case.odb,
                &mut |commit_id| commit_id.to_hex_with_len(7).to_string(),
                options,
            )
        };
        let actual = merge(options.clone())?.tree_merge;
        assert!(
            actual
                .conflicts
                .iter()
                .any(|conflict| matches!(&conflict.resolution, Err(ResolutionFailure::DirectoryRename { .. }))),
            "{}: inferred locations require confirmation alongside any other conflict: {:#?}",
            case.case_name,
            actual.conflicts
        );
        if case.case_name.starts_with("relocated-rename-failures") {
            assert!(
                actual.conflicts.iter().any(|conflict| matches!(
                    &conflict.resolution,
                    Err(ResolutionFailure::OursDeletedTheirsRenamed
                        | ResolutionFailure::OursRenamedTheirsRenamedDifferently { .. })
                )),
                "{}: relocation preserves the original rename failure",
                case.case_name
            );
        }
        options.tree_merge.fail_on_conflict = Some(TreatAsUnresolved::git());
        assert!(
            merge(options.clone())?.tree_merge.failed_on_first_unresolved_conflict,
            "{}: location conflicts participate in early stopping",
            case.case_name
        );
        options.tree_merge.fail_on_conflict = None;
        if !case.case_name.starts_with("relocated-rename-failures") {
            for resolution in [ResolveWith::Ancestor, ResolveWith::Ours] {
                if resolution == ResolveWith::Ancestor && case.case_name.starts_with("relocated-merged-content") {
                    continue;
                }
                options.tree_merge.tree_conflicts = Some(resolution);
                let mut actual = merge(options.clone())?.tree_merge;
                let actual_tree_id = actual.tree.write(|tree| case.odb.write(tree))?;
                let side = match resolution {
                    ResolveWith::Ancestor => "ancestor",
                    ResolveWith::Ours => &case.our_side_name,
                };
                let expected_tree_id = gix_hash::ObjectId::from_hex(
                    std::fs::read_to_string(case.root.join(format!(".git/force-{side}.tree")))?
                        .trim()
                        .as_bytes(),
                )?;
                assert_eq!(
                    actual_tree_id, expected_tree_id,
                    "{}: {resolution:?} preserves the selected source and merged contents",
                    case.case_name
                );
                assert!(
                    !actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
                    "{}: explicit location choices resolve the tree conflict",
                    case.case_name
                );
            }
        }
    }
    assert_eq!(actual_cases, 10, "all relocation regressions run in both directions");
    Ok(())
}

#[test]
fn unrelated_commits_require_explicit_permission() -> TestResult {
    let root = gix_testtools::tempfile::TempDir::new()?;
    let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, gix_testtools::object_hash());
    let tree_id = objects.write_buf(gix_object::Kind::Tree, b"")?;
    let root_commit = |message: &str| {
        objects.write_buf(
            gix_object::Kind::Commit,
            format!(
                "tree {tree_id}\nauthor A <a@example.com> 0 +0000\ncommitter A <a@example.com> 0 +0000\n\n{message}\n"
            )
            .as_bytes(),
        )
    };
    let our_commit_id = root_commit("ours")?;
    let their_commit_id = root_commit("theirs")?;
    let mut graph = gix_revwalk::Graph::new(&objects, None);
    let mut diff_resource_cache = new_diff_resource_cache(root.path());
    let mut blob_merge = new_blob_merge_platform(root.path(), 0);

    for allow_missing_merge_base in [false, true] {
        let result = gix_merge::commit(
            our_commit_id,
            their_commit_id,
            Default::default(),
            &mut graph,
            &mut diff_resource_cache,
            &mut blob_merge,
            &objects,
            &mut |id| id.to_hex_with_len(7).to_string(),
            Options {
                allow_missing_merge_base,
                ..basic_merge_options()
            },
        );
        if allow_missing_merge_base {
            let out = result?;
            assert!(out.merge_bases.is_none(), "the two root commits remain unrelated");
            assert_eq!(
                out.merge_base_tree_id, tree_id,
                "an empty tree replaces the missing merge base"
            );
        } else {
            let err = result.err().expect("the caller requires a common ancestor");
            assert!(
                err.is_validation(),
                "the commit pair violates the requested ancestry contract"
            );
            assert_eq!(
                err.to_string(),
                format!("No common ancestor between {our_commit_id} and {their_commit_id}"),
                "classification preserves the diagnostic"
            );
        }
    }
    Ok(())
}

fn directory_renames_for_case(case_name: &str) -> gix_merge::tree::DirectoryRenames {
    use gix_merge::tree::DirectoryRenames;
    if case_name.contains("directory-renames-false") {
        DirectoryRenames::Disabled
    } else if case_name.contains("directory-renames-conflict") {
        DirectoryRenames::Conflict
    } else {
        DirectoryRenames::Enabled
    }
}

fn basic_merge_options() -> Options {
    gix_merge::commit::Options {
        allow_missing_merge_base: true,
        use_first_merge_base: false,
        tree_merge: gix_merge::tree::Options {
            directory_renames: gix_merge::tree::DirectoryRenames::Enabled,
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
        gix_diff::blob::Pipeline::new(
            Default::default(),
            gix_filter::Pipeline::new(Default::default(), gix_testtools::object_hash(), Default::default()),
            Vec::new(),
            Default::default(),
        ),
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
        gix_filter::Pipeline::new(Default::default(), gix_testtools::object_hash(), Default::default()),
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
mod cartesian;
