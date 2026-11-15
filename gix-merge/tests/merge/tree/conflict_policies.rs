use gix_merge::{
    blob::builtin_driver::text,
    tree::{DirectoryRenames, ResolutionFailure, ResolveWith, TreatAsUnresolved},
};
use gix_object::Write;
use gix_testtools::TestResult;

use super::{baseline, basic_merge_options, new_blob_merge_platform, new_diff_resource_cache, tree_baseline_fixture};

fn cases(prefix: &str) -> TestResult<Vec<baseline::Expectation>> {
    let Some(root) = tree_baseline_fixture()? else {
        return Ok(Vec::new());
    };
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    let cases: Vec<_> = baseline::Expectations::new(&root, &cases)
        .filter(|case| case.case_name.starts_with(prefix))
        .collect();
    assert!(!cases.is_empty(), "the fixture includes cases for {prefix}");
    Ok(cases)
}

fn merge(
    case: &baseline::Expectation,
    tree_merge: gix_merge::tree::Options,
) -> TestResult<gix_merge::tree::Outcome<'_>> {
    let mut options = basic_merge_options();
    options.tree_merge = tree_merge;
    Ok(gix_merge::commit(
        case.our_commit_id,
        case.their_commit_id,
        text::Labels {
            ancestor: None,
            current: Some(case.our_side_name.as_str().into()),
            other: Some(case.their_side_name.as_str().into()),
        },
        &mut gix_revwalk::Graph::new(&case.odb, None),
        &mut new_diff_resource_cache(&case.root),
        &mut new_blob_merge_platform(&case.root, 0),
        &case.odb,
        &mut |commit_id| commit_id.to_hex_with_len(7).to_string(),
        options,
    )?
    .tree_merge)
}

#[test]
fn forced_entries_preserve_sources_and_unrelated_paths_across_pairs() -> TestResult {
    for case in cases("shared-distinct-types-")?
        .into_iter()
        .chain(cases("structural-path-")?)
    {
        for (choice, name) in [(ResolveWith::Ancestor, "ancestor"), (ResolveWith::Ours, "ours")] {
            let mut options = basic_merge_options().tree_merge;
            options.tree_conflicts = Some(choice);
            options.fail_on_conflict = Some(TreatAsUnresolved::git());
            let mut actual = merge(&case, options)?;
            let expected_tree_id = gix_hash::ObjectId::from_hex(
                std::fs::read_to_string(case.root.join(format!(
                    ".git/resolve-{}-{}-with-{name}.tree",
                    case.our_side_name, case.their_side_name
                )))?
                .trim()
                .as_bytes(),
            )?;
            let actual_tree_id = actual.tree.write(|tree| case.odb.write(tree))?;
            if actual_tree_id != expected_tree_id {
                baseline::show_diff_trees_and_fail(
                    &case.case_name,
                    actual_tree_id,
                    &actual,
                    expected_tree_id,
                    name,
                    &case.odb,
                );
            }
            assert!(
                !actual.failed_on_first_unresolved_conflict
                    && !actual.has_unresolved_conflicts(TreatAsUnresolved::git())
                    && actual
                        .conflicts
                        .iter()
                        .all(|conflict| conflict.content_merge().is_none()),
                "{}: a forced entry cannot retain a discarded content conflict",
                case.case_name
            );
        }
    }
    Ok(())
}

#[test]
fn modes_without_shared_ancestor_conflict_independently_of_contents() -> TestResult {
    for case in cases("mode-conflict-")? {
        let mut options = basic_merge_options().tree_merge;
        options.blob_merge.text.conflict = text::Conflict::ResolveWithOurs;
        options.fail_on_conflict = Some(TreatAsUnresolved::git());
        let actual = merge(&case, options)?;
        assert!(
            actual.failed_on_first_unresolved_conflict
                && actual
                    .conflicts
                    .iter()
                    .any(|conflict| matches!(conflict.resolution, Err(ResolutionFailure::ModeConflict { .. }))),
            "{}: file favor cannot resolve a mode conflict: {:#?}",
            case.case_name,
            actual.conflicts
        );
        for (choice, name) in [(ResolveWith::Ancestor, "ancestor"), (ResolveWith::Ours, "ours")] {
            let mut options = basic_merge_options().tree_merge;
            options.tree_conflicts = Some(choice);
            options.fail_on_conflict = Some(TreatAsUnresolved::git());
            let mut actual = merge(&case, options)?;
            let expected_tree_id = gix_hash::ObjectId::from_hex(
                std::fs::read_to_string(case.root.join(format!(
                    ".git/resolve-{}-{}-with-{name}.tree",
                    case.our_side_name, case.their_side_name
                )))?
                .trim()
                .as_bytes(),
            )?;
            assert_eq!(
                actual.tree.write(|tree| case.odb.write(tree))?,
                expected_tree_id,
                "{}: tree favor selects the whole entry",
                case.case_name
            );
            assert!(
                !actual.failed_on_first_unresolved_conflict
                    && !actual.has_unresolved_conflicts(TreatAsUnresolved::git())
                    && actual
                        .conflicts
                        .iter()
                        .all(|conflict| conflict.content_merge().is_none()),
                "{}: the discarded content merge cannot remain unresolved",
                case.case_name
            );
        }
    }
    Ok(())
}

#[test]
fn added_modes_do_not_change_content_sides() -> TestResult {
    for case in cases("mode-conflict-add-add-conflicting-")? {
        let mut options = basic_merge_options().tree_merge;
        if case.case_name.contains("file-favor-ours") {
            options.blob_merge.text.conflict = text::Conflict::ResolveWithOurs;
        }
        let mut actual = merge(&case, options)?;
        assert_eq!(
            actual.tree.write(|tree| case.odb.write(tree))?,
            case.merge_info.merged_tree,
            "{}: content favor and marker labels follow the requested sides",
            case.case_name
        );
    }
    Ok(())
}

#[test]
fn recursive_renames_preserve_paths() -> TestResult {
    for case in cases("recursive-rename-")?
        .into_iter()
        .chain(cases("recursive-shared-source-")?)
    {
        let mut options = basic_merge_options().tree_merge;
        if case.case_name.contains("file-favor-ours") {
            options.blob_merge.text.conflict = text::Conflict::ResolveWithOurs;
            options.symlink_conflicts = Some(gix_merge::blob::builtin_driver::binary::ResolveWith::Ours);
            options.blob_merge.resolve_binary_with = options.symlink_conflicts;
        }
        let mut actual = merge(&case, options.clone())?;
        assert_eq!(
            actual.tree.write(|tree| case.odb.write(tree))?,
            case.merge_info.merged_tree,
            "{}: surviving sources and the merged destination agree with Git",
            case.case_name
        );
        // Recursive content conflicts must not stop construction of the base.
        options.fail_on_conflict = Some(TreatAsUnresolved::git());
        assert_eq!(
            merge(&case, options)?.failed_on_first_unresolved_conflict,
            case.merge_info.conflicts.is_some(),
            "{}: early stopping applies only to the final merge",
            case.case_name
        );
        assert_eq!(
            actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
            case.merge_info.conflicts.is_some(),
            "{}: virtual bases preserve later conflicts",
            case.case_name
        );
    }
    Ok(())
}

#[test]
fn forced_entries_discard_only_unused_content() -> TestResult {
    for case in cases("forced-content-")? {
        let mut options = basic_merge_options().tree_merge;
        options.directory_renames = DirectoryRenames::Conflict;
        options.tree_conflicts = Some(ResolveWith::Ancestor);
        options.fail_on_conflict = Some(TreatAsUnresolved::git());
        let mut actual = merge(&case, options.clone())?;
        let expected_tree_id = gix_hash::ObjectId::from_hex(
            std::fs::read_to_string(case.root.join(".git/force-ancestor.tree"))?
                .trim()
                .as_bytes(),
        )?;
        assert_eq!(
            actual.tree.write(|tree| case.odb.write(tree))?,
            expected_tree_id,
            "{}: ancestor selection restores the entry and retains unrelated changes",
            case.case_name
        );
        assert!(
            !actual.failed_on_first_unresolved_conflict
                && !actual.has_unresolved_conflicts(TreatAsUnresolved::git())
                && actual
                    .conflicts
                    .iter()
                    .all(|conflict| conflict.content_merge().is_none()),
            "{}: discarded content cannot prevent committing the selected tree: {:#?}",
            case.case_name,
            actual.conflicts
        );
        assert!(
            actual.has_unresolved_conflicts(TreatAsUnresolved::forced_resolution()),
            "{}: the forced decision remains observable",
            case.case_name
        );

        options.tree_conflicts = Some(ResolveWith::Ours);
        options.fail_on_conflict = None;
        let actual = merge(&case, options)?;
        let keeps_content = !case.case_name.starts_with("forced-content-add-add-");
        assert_eq!(
            actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
            keeps_content,
            "{}: choosing a location retains its content conflict; choosing a whole entry discards it",
            case.case_name
        );
        assert_eq!(
            actual
                .conflicts
                .iter()
                .any(|conflict| conflict.content_merge().is_some()),
            keeps_content,
            "{}: content metadata describes the selected tree",
            case.case_name
        );
    }
    Ok(())
}
