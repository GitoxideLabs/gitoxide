use gix_testtools::TestResult;
use std::path::{Path, PathBuf};

use gix_diff::Rewrites;
use gix_merge::{
    commit::Options,
    tree::{TreatAsUnresolved, treat_as_unresolved},
};
use gix_object::{FindExt, Write};
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

#[test]
fn directory_relocation_colliding_with_modified_file() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("directory-relocation-collision.sh")?.join("repo");
    let objects = gix_odb::memory::Proxy::new(
        gix_odb::at(root.join(".git/objects"), gix_testtools::object_hash())?,
        gix_testtools::object_hash(),
    );
    for layout in ["opposite", "same", "both", "different"] {
        let tree_ids = std::fs::read_to_string(root.join(if layout == "opposite" {
            "trees".to_owned()
        } else {
            format!("trees-{layout}")
        }))?
        .lines()
        .map(|hex| gix_hash::ObjectId::from_hex(hex.as_bytes()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
        let [base_tree_id, our_tree_id, their_tree_id] = tree_ids.as_slice() else {
            panic!("the fixture records the base and both side trees");
        };
        for (current_tree_id, other_tree_id, current, other) in [
            (our_tree_id, their_tree_id, "ours", "theirs"),
            (their_tree_id, our_tree_id, "theirs", "ours"),
        ] {
            let mut diff_state = gix_diff::tree::State::default();
            let mut diff_resource_cache = new_diff_resource_cache(&root);
            let mut blob_merge = new_blob_merge_platform(&root, 100);
            let mut outcome = gix_merge::tree(
                base_tree_id,
                current_tree_id,
                other_tree_id,
                gix_merge::blob::builtin_driver::text::Labels {
                    current: Some(current.into()),
                    other: Some(other.into()),
                    ancestor: None,
                },
                &objects,
                |buf| objects.write_buf(gix_object::Kind::Blob, buf),
                &mut diff_state,
                &mut diff_resource_cache,
                &mut blob_merge,
                basic_merge_options().tree_merge,
            )?;
            assert_no_unknown_conflicts(&outcome, "directory relocation colliding with a modified file");
            let prefix = if layout == "opposite" {
                String::new()
            } else {
                format!("{layout}-")
            };
            let expected = baseline::parse_merge_info(std::fs::read_to_string(
                root.join(format!("{prefix}{current}-{other}.merge-info")),
            )?);
            let actual_tree_id = outcome.tree.write(|tree| objects.write(tree))?;
            let modified_side = if layout == "same" { "ours" } else { "theirs" };
            let preserved_path = format!("existing/block~{modified_side}");
            let modified_blob_id =
                gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Blob, b"changed")?;
            let base_blob_id = gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Blob, b"base")?;
            let retained_blob_id = if layout == "different" {
                let (our_content, their_content) = if current == "ours" {
                    ("changed", "other-change")
                } else {
                    ("other-change", "changed")
                };
                objects.write_buf(
                    gix_object::Kind::Blob,
                    format!("<<<<<<< {current}\n{our_content}\n=======\n{their_content}\n>>>>>>> {other}\n").as_bytes(),
                )?
            } else {
                modified_blob_id
            };
            let mut buffer = Vec::new();
            let mut expected_tree = gix_object::tree::Editor::new(
                objects.find_tree(&expected.merged_tree, &mut buffer)?.to_owned(),
                &objects,
                gix_testtools::object_hash(),
            );
            assert!(
                expected_tree.get(preserved_path.split('/')).is_none(),
                "Git never moves the parent aside to preserve both the file and incoming child"
            );
            // Git loses a resolved parent, or the incoming child if parent edits conflict.
            // Preserve both as an ordinary file/directory conflict instead of reproducing the loss.
            if layout == "different" {
                assert!(
                    expected_tree.get(["existing", "block", "fresh"]).is_none(),
                    "Git loses the incoming child when the parent contents conflict"
                );
                expected_tree.remove(["existing", "block"])?;
                expected_tree.upsert(
                    ["existing", "block", "fresh"],
                    gix_object::tree::EntryKind::Blob,
                    gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Blob, b"nested")?,
                )?;
            }
            expected_tree.upsert(
                preserved_path.split('/'),
                gix_object::tree::EntryKind::Blob,
                retained_blob_id,
            )?;
            let expected_tree_id = expected_tree.write(|tree| objects.write(tree))?;
            assert_eq!(
                actual_tree_id,
                expected_tree_id,
                "{layout} {current}-{other}: the tree differs from Git only to preserve both the parent and child\nactual: {}\nexpected: {}",
                baseline::visualize_tree(&actual_tree_id, &objects, None),
                baseline::visualize_tree(&expected_tree_id, &objects, None)
            );
            let mut expected_index = gix_index::State::from_tree(&expected_tree_id, &objects, Default::default())?;
            if let Some(mut conflicts) = expected.conflicts {
                for conflict in &mut conflicts {
                    for entry in [&mut conflict.ancestor, &mut conflict.ours, &mut conflict.theirs]
                        .into_iter()
                        .flatten()
                    {
                        if entry.location == "existing/block" {
                            entry.location = preserved_path.as_str().into();
                        }
                    }
                }
                baseline::apply_git_index_entries(&conflicts, &mut expected_index);
            }
            let modified = baseline::Entry {
                location: preserved_path.as_str().into(),
                id: modified_blob_id,
                mode: gix_object::tree::EntryKind::Blob.into(),
            };
            let mut preserved = baseline::Conflict {
                ancestor: (layout != "both").then(|| baseline::Entry {
                    location: preserved_path.as_str().into(),
                    id: base_blob_id,
                    mode: gix_object::tree::EntryKind::Blob.into(),
                }),
                ..Default::default()
            };
            if current == modified_side {
                preserved.ours = Some(modified);
            } else {
                preserved.theirs = Some(modified);
            }
            if layout != "different" {
                baseline::apply_git_index_entries(&[preserved], &mut expected_index);
            }
            let actual_index = outcome
                .index
                .to_index(TreatAsUnresolved::git(), &objects, Default::default())?;
            assert_eq!(
                baseline::debug_entries(&actual_index),
                baseline::debug_entries(&expected_index),
                "Git's stages are preserved, with the modified parent represented as a file/directory conflict"
            );
            assert!(
                outcome.has_unresolved_conflicts(TreatAsUnresolved::git()),
                "both exact blockers and modified parent blockers require attention"
            );
        }
    }
    Ok(())
}

#[test]
fn directory_rename_destination_already_consumed() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("directory-rename-destination-collision.sh")?.join("repo");
    let objects = gix_odb::memory::Proxy::new(
        gix_odb::at(root.join(".git/objects"), gix_testtools::object_hash())?,
        gix_testtools::object_hash(),
    );
    let tree_ids = std::fs::read_to_string(root.join("trees"))?
        .lines()
        .map(|hex| gix_hash::ObjectId::from_hex(hex.as_bytes()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let [base_tree_id, our_tree_id, their_tree_id] = tree_ids.as_slice() else {
        panic!("the fixture records the base and both side trees");
    };
    for (current_tree_id, other_tree_id) in [(our_tree_id, their_tree_id), (their_tree_id, our_tree_id)] {
        let mut diff_state = gix_diff::tree::State::default();
        let mut diff_resource_cache = new_diff_resource_cache(&root);
        let mut blob_merge = new_blob_merge_platform(&root, 100);
        let mut outcome = gix_merge::tree(
            base_tree_id,
            current_tree_id,
            other_tree_id,
            Default::default(),
            &objects,
            |buf| objects.write_buf(gix_object::Kind::Blob, buf),
            &mut diff_state,
            &mut diff_resource_cache,
            &mut blob_merge,
            basic_merge_options().tree_merge,
        )?;
        assert_no_unknown_conflicts(&outcome, "directory and file renames sharing a destination");
        for (path, content) in [
            ("a/a", "anchor"),
            ("a/d/d", "nested"),
            ("a~", "file"),
            ("b/a/f", "fresh"),
            ("d/a", "anchor"),
            ("d~", "anchor"),
        ] {
            let entry = outcome
                .tree
                .get(path.split('/'))
                .expect("all colliding entries survive");
            assert_eq!(
                entry.oid,
                gix_object::compute_hash(gix_testtools::object_hash(), gix_object::Kind::Blob, content.as_bytes())?,
                "the merge preserves {path} after its destination node was consumed"
            );
        }
        outcome.tree.write(|tree| objects.write(tree))?;
    }
    Ok(())
}

#[test]
fn rename_add_after_destination_was_consumed() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("rename-add-consumed-destination.sh")?.join("repo");
    let objects = gix_odb::memory::Proxy::new(
        gix_odb::at(root.join(".git/objects"), gix_testtools::object_hash())?,
        gix_testtools::object_hash(),
    );
    let tree_ids = std::fs::read_to_string(root.join("trees"))?
        .lines()
        .map(|hex| gix_hash::ObjectId::from_hex(hex.as_bytes()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let [base_tree_id, our_tree_id, their_tree_id] = tree_ids.as_slice() else {
        panic!("the fixture records the base and both side trees");
    };
    let object_ids = std::fs::read_to_string(root.join("objects"))?
        .lines()
        .map(|hex| gix_hash::ObjectId::from_hex(hex.as_bytes()))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let [commit_id, added_commit_id, blob_id] = object_ids.as_slice() else {
        panic!("the fixture records both gitlink targets and the renamed blob");
    };
    for (current_tree_id, other_tree_id) in [(our_tree_id, their_tree_id), (their_tree_id, our_tree_id)] {
        let mut diff_state = gix_diff::tree::State::default();
        let mut diff_resource_cache = new_diff_resource_cache(&root);
        let mut blob_merge = new_blob_merge_platform(&root, 1);
        let mut options = basic_merge_options().tree_merge;
        options
            .rewrites
            .as_mut()
            .expect("rename tracking is enabled")
            .percentage = Some(1.0);
        let mut outcome = gix_merge::tree(
            base_tree_id,
            current_tree_id,
            other_tree_id,
            Default::default(),
            &objects,
            |buf| objects.write_buf(gix_object::Kind::Blob, buf),
            &mut diff_state,
            &mut diff_resource_cache,
            &mut blob_merge,
            options,
        )?;
        assert_no_unknown_conflicts(&outcome, "rename/add after its destination was consumed");
        let actual_tree_id = outcome.tree.clone().write(|tree| objects.write(tree))?;
        for (path, mode, object_id) in [
            ("a/h", gix_object::tree::EntryKind::Commit, added_commit_id),
            ("c/a/h", gix_object::tree::EntryKind::Commit, commit_id),
            ("c/h/h", gix_object::tree::EntryKind::Commit, commit_id),
            ("h/h", gix_object::tree::EntryKind::Blob, blob_id),
        ] {
            let entry = outcome.tree.get(path.split('/')).unwrap_or_else(|| {
                panic!(
                    "the renamed entry {path} survives:\n{}\nConflicts: {:#?}",
                    baseline::visualize_tree(&actual_tree_id, &objects, None),
                    outcome.conflicts,
                )
            });
            assert_eq!(entry.mode.kind(), mode, "the merge preserves the mode at {path}");
            assert_eq!(entry.oid, *object_id, "the merge preserves the object at {path}");
        }
        assert_eq!(
            gix_index::State::from_tree(&actual_tree_id, &objects, Default::default())?
                .entries()
                .len(),
            4,
            "the normalized result matches Git without duplicate conflict paths"
        );
        outcome.tree.write(|tree| objects.write(tree))?;
    }
    Ok(())
}

// Reuse the expensive, Git-version-sensitive baseline even during native fixture QA.
fn tree_baseline_fixture() -> TestResult<Option<PathBuf>> {
    if *gix_testtools::GIT_BASH_VERSION >= (2, 50, 0) {
        Ok(Some(gix_testtools::scripted_fixture_read_only_needs_archive(
            "tree-baseline.sh",
        )?))
    } else {
        // Git before 2.50 ignores disabled rename detection; require a compatible archive.
        Ok(gix_testtools::scripted_fixture_read_only_with_git_version(
            "tree-baseline.sh",
            |_| false,
        )?)
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
    let Some(root) = tree_baseline_fixture()? else {
        return Ok(());
    };
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    let mut actual_cases = 0;
    let mut skipped_tree_resolve_cases = 0;
    let mut failures = Vec::new();
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
        let mut phase = "default merge";
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> TestResult {
            let mut graph = gix_revwalk::Graph::new(&odb, None);
            // Recursive conflict markers exceed the small threshold used by the large-file cases.
            let large_file_threshold_bytes = if case_name.contains("criss-cross") { 0 } else { 100 };
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
            if case_name.contains("file-favor-ours") {
                options.tree_merge.blob_merge.text.conflict =
                    gix_merge::blob::builtin_driver::text::Conflict::ResolveWithOurs;
                options.tree_merge.symlink_conflicts = Some(gix_merge::blob::builtin_driver::binary::ResolveWith::Ours);
                options.tree_merge.blob_merge.resolve_binary_with = options.tree_merge.symlink_conflicts;
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
            assert_no_unknown_conflicts(&actual, &case_name);
            if case_name.starts_with("directory-rename-tie-") {
                assert!(
                    merge_info
                        .information
                        .iter()
                        .any(|info| matches!(info.kind, baseline::ConflictKind::DirectoryRenameSplit)),
                    "Git reports the tied directory destinations"
                );
                assert!(
                    actual.conflicts.iter().any(|conflict| matches!(
                        conflict.resolution,
                        Err(gix_merge::tree::ResolutionFailure::DirectoryRenameSplit { .. })
                    )),
                    "tied destinations retain new paths and report the unresolved directory split"
                );
            }

            phase = "default tree";
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

            phase = "default index";
            let conflicts_like_in_git = TreatAsUnresolved::git();
            let actual_index = actual.index.to_index(conflicts_like_in_git, &odb, Default::default())?;
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
                    let mut index = gix_index::State::from_tree(&actual_id, &odb, Default::default())?;
                    if let Some(conflicts) = &merge_info.conflicts {
                        baseline::apply_git_index_entries(conflicts, &mut index);
                    }
                    index
                }
            };
            pretty_assertions::assert_eq!(
                baseline::debug_entries(&actual_index),
                baseline::debug_entries(&expected_index),
                "{case_name}: index mismatch\nOur conflicts {:#?}\nGit conflicts {:#?}",
                actual.conflicts,
                merge_info.conflicts
            );
            if deviation.is_none() {
                let git_has_unmerged_entries = expected_index
                    .entries()
                    .iter()
                    .any(|entry| entry.stage() != gix_index::entry::Stage::Unconflicted);
                assert_eq!(
                    actual.index.has_unmerged_entries(conflicts_like_in_git),
                    git_has_unmerged_entries,
                    "{case_name}: unmerged index stages must match Git"
                );
                assert_eq!(
                    actual.has_unresolved_conflicts(conflicts_like_in_git),
                    git_has_unmerged_entries
                        || merge_info.information.iter().any(|info| !matches!(
                            info.kind,
                            baseline::ConflictKind::AutoMerging
                                | baseline::ConflictKind::PathUpdatedDueToDirectoryRename
                        )),
                    "{case_name}: conflict status includes directory diagnostics with a clean index"
                );
            }

            // The content-merge mode is not relevant for the upcoming tree-conflict resolution.
            if case_name.contains("diff3") {
                return Ok(());
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
                phase = match tree_resolution {
                    gix_merge::tree::ResolveWith::Ancestor => "forced ancestor",
                    gix_merge::tree::ResolveWith::Ours => "forced ours",
                };
                let expected_tree_id =
                    gix_hash::ObjectId::from_hex(std::fs::read_to_string(tree_path)?.trim().as_bytes())?;
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
                    baseline::show_diff_trees_and_fail(
                        &case_name,
                        actual_id,
                        &actual,
                        expected_tree_id,
                        &basename,
                        &odb,
                    );
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
            Ok(())
        }));
        match result {
            Ok(Ok(())) => {}
            result => {
                if let Ok(Err(err)) = result {
                    eprintln!("{case_name} ({phase}): {err:?}");
                }
                failures.push(format!("{case_name}: {phase}"));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{}/{} baseline cases failed:\n{}",
        failures.len(),
        actual_cases,
        failures.join("\n"),
    );
    assert_eq!(
        actual_cases, 535,
        "BUG: update this number, and don't forget to remove a filter in the end"
    );
    assert_eq!(
        skipped_tree_resolve_cases, 586,
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
    let Some(root) = tree_baseline_fixture()? else {
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

    let Some(root) = tree_baseline_fixture()? else {
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

    let Some(root) = tree_baseline_fixture()? else {
        return Ok(());
    };
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
mod conflict_policies;
mod directory_outcomes;
mod gitlink_renames;
mod rename_parent_namespace;
mod renormalized_delete;
mod structural_differential;
mod synthetic_trees;
