use gix_error::{ResultExt, message};
use gix_merge::tree::{DirectoryRenames, Resolution, ResolutionFailure, ResolveWith, TreatAsUnresolved};
use gix_object::Write;
use gix_testtools::TestResult;

use super::{baseline, basic_merge_options, new_blob_merge_platform, new_diff_resource_cache};

#[test]
fn gitlinks_use_typed_merges_during_rename_normalization() -> TestResult {
    let Some(root) =
        gix_testtools::scripted_fixture_read_only_with_git_version("tree-gitlink-renames.sh", |version| {
            version >= (2, 38, 0)
        })?
    else {
        return Ok(());
    };
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    let mut count = 0;
    let mut failures = Vec::new();
    for case in baseline::Expectations::new(&root, &cases) {
        count += 1;
        let status =
            std::fs::read_to_string(case.root.join(
                format!("{}.status", case.case_name.strip_prefix("matrix-").expect("fixture directory prefix")),
            ))?;
        let expected_conflicted = match status.trim() {
            "0" => false,
            "1" => true,
            _ => panic!("unexpected Git merge status: {status}"),
        };
        for virtual_ancestor in [false, true] {
            for favor in [None, Some(ResolveWith::Ancestor), Some(ResolveWith::Ours)] {
                let mut options = basic_merge_options();
                options.tree_merge.tree_conflicts = favor;
                options.tree_merge.blob_merge.is_virtual_ancestor = virtual_ancestor;
                options.tree_merge.directory_renames = if case.case_name.contains("-conflict-") {
                    DirectoryRenames::Conflict
                } else {
                    DirectoryRenames::Enabled
                };
                let mut actual = gix_merge::commit(
                    case.our_commit_id,
                    case.their_commit_id,
                    gix_merge::blob::builtin_driver::text::Labels {
                        ancestor: None,
                        current: Some(case.our_side_name.as_str().into()),
                        other: Some(case.their_side_name.as_str().into()),
                    },
                    &mut gix_revwalk::Graph::new(&case.odb, None),
                    &mut new_diff_resource_cache(&case.root),
                    &mut new_blob_merge_platform(&case.root, None),
                    &case.odb,
                    &mut |commit_id| commit_id.to_hex_with_len(7).to_string(),
                    gix_merge::tree::Context::default(),
                    options,
                )
                .or_raise(|| message!("{}: favor={favor:?}, virtual={virtual_ancestor}", case.case_name))?
                .tree_merge;
                assert!(
                    actual
                        .conflicts
                        .iter()
                        .all(|conflict| conflict.content_merge().is_none()),
                    "{}: gitlink conflicts never carry blob-merge metadata",
                    case.case_name
                );
                let actual_tree_id = actual.tree.write(|tree| case.odb.write(tree))?;
                let actual_index =
                    actual
                        .index
                        .to_index(TreatAsUnresolved::git(), None, &case.odb, Default::default())?;
                actual_index.verify_entries()?;
                assert!(
                    case.case_name.contains("-type-")
                        || actual_index
                            .entries()
                            .iter()
                            .all(|entry| entry.mode == gix_index::entry::Mode::COMMIT),
                    "{}: every index stage remains a gitlink",
                    case.case_name
                );
                if case.case_name.contains("-add-add-")
                    || case.case_name.contains("-modify-")
                    || case.case_name.contains("-type-")
                {
                    assert!(
                        actual.conflicts.iter().any(|conflict| {
                            match &conflict.resolution {
                                Err(failure) | Ok(Resolution::Forced(failure)) => match failure {
                                    ResolutionFailure::SubmoduleAddAdd => {
                                        case.case_name.contains("-add-add-") || case.case_name.contains("-type-")
                                    }
                                    ResolutionFailure::SubmoduleMerge => case.case_name.contains("-modify-"),
                                    _ => false,
                                },
                                _ => false,
                            }
                        }),
                        "{}: typed submodule classifications survive every tree policy",
                        case.case_name
                    );
                }
                if let Some(favor) = favor {
                    let layout = &case.our_side_name[2..];
                    let current = &case.our_side_name[..1];
                    let enabled = case.case_name.contains("-true-");
                    let filename = match (layout, favor, current, virtual_ancestor, enabled) {
                        ("directory", ResolveWith::Ancestor, _, true, _) => "directory.virtual-ancestor.tree".into(),
                        ("directory", ResolveWith::Ours, "B", true, _) => "directory.B-virtual.tree".into(),
                        ("directory", ResolveWith::Ours, "B", false, true) => "directory.B-relocated.tree".into(),
                        ("nested-directory", ResolveWith::Ours, current, true, _) => {
                            format!("nested-directory.{current}-virtual.tree")
                        }
                        ("nested-directory", ResolveWith::Ours, "A", false, true) => {
                            "nested-directory.A-relocated.tree".into()
                        }
                        (_, ResolveWith::Ancestor, _, _, _) => format!("{layout}.base.tree"),
                        (_, ResolveWith::Ours, _, _, _) => format!("{}.tree", case.our_side_name),
                    };
                    let tree_id = std::fs::read_to_string(case.root.join(filename))?;
                    let tree_id = gix_hash::ObjectId::from_hex(tree_id.trim().as_bytes())?;
                    let expected_index = gix_index::State::from_tree(&tree_id, &case.odb, Default::default())?;
                    if actual_tree_id != tree_id
                        || baseline::debug_entries(&actual_index) != baseline::debug_entries(&expected_index)
                    {
                        failures.push(format!(
                            "{}: {favor:?}, virtual={virtual_ancestor}\n{}\nindex: {:#?}",
                            case.case_name,
                            baseline::visualize_tree(&actual_tree_id, &case.odb, None),
                            baseline::debug_entries(&actual_index)
                        ));
                    }
                }
                if favor.is_none() && !virtual_ancestor {
                    let mut expected_index =
                        gix_index::State::from_tree(&case.merge_info.merged_tree, &case.odb, Default::default())?;
                    if let Some(conflicts) = &case.merge_info.conflicts {
                        baseline::apply_git_index_entries(conflicts, &mut expected_index);
                    }
                    assert_eq!(
                        actual_tree_id, case.merge_info.merged_tree,
                        "{}: final tree matches Git",
                        case.case_name
                    );
                    assert_eq!(
                        actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
                        expected_conflicted,
                        "{}: conflict status matches Git",
                        case.case_name
                    );
                    assert_eq!(
                        baseline::debug_entries(&actual_index),
                        baseline::debug_entries(&expected_index),
                        "{}: final conflict stages match Git",
                        case.case_name
                    );
                }
            }
        }
    }
    assert_eq!(count, 24, "all gitlink layouts, directory policies, and directions run");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}
