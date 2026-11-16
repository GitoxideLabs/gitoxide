use std::fmt::Write as _;

use gix_merge::tree::{DirectoryRenames, TreatAsUnresolved};
use gix_object::Write;
use gix_testtools::TestResult;

use super::{baseline, basic_merge_options, new_blob_merge_platform, new_diff_resource_cache};

#[test]
fn directory_destinations_and_diagnostics_match_git() -> TestResult {
    let Some(root) =
        gix_testtools::scripted_fixture_read_only_with_git_version("tree-directory-outcomes.sh", |version| {
            version >= (2, 38, 0)
        })?
    else {
        return Ok(());
    };
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    let mut failures = String::new();
    let mut count = 0;
    for case in baseline::Expectations::new(&root, &cases) {
        count += 1;
        let (mode, directory_renames) = if case.case_name.contains("-true-") {
            ("true", DirectoryRenames::Enabled)
        } else if case.case_name.contains("-conflict-") {
            ("conflict", DirectoryRenames::Conflict)
        } else {
            ("false", DirectoryRenames::Disabled)
        };
        let mut options = basic_merge_options();
        options.tree_merge.directory_renames = directory_renames;
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
            &mut new_blob_merge_platform(&case.root, 0),
            &case.odb,
            &mut |commit_id| commit_id.to_hex_with_len(7).to_string(),
            gix_merge::tree::Context::default(),
            options,
        )?
        .tree_merge;
        let actual_tree_id = actual.tree.write(|tree| case.odb.write(tree))?;
        let status = std::fs::read_to_string(case.root.join(format!(
            ".git/{}--{}-{mode}.status",
            case.our_side_name, case.their_side_name
        )))?;
        let expected_conflicted = status.trim() == "1";
        let actual_index = actual
            .index
            .to_index(TreatAsUnresolved::git(), &case.odb, Default::default())?;
        let mut expected_index =
            gix_index::State::from_tree(&case.merge_info.merged_tree, &case.odb, Default::default())?;
        if let Some(conflicts) = &case.merge_info.conflicts {
            baseline::apply_git_index_entries(conflicts, &mut expected_index);
        }
        let actual_entries = baseline::debug_entries(&actual_index);
        let expected_entries = baseline::debug_entries(&expected_index);
        if actual_tree_id != case.merge_info.merged_tree
            || actual_entries != expected_entries
            || actual.has_unresolved_conflicts(TreatAsUnresolved::git()) != expected_conflicted
        {
            writeln!(
                failures,
                "{}: gix conflicted={}, Git conflicted={expected_conflicted}\ngix tree:\n{}\nGit tree:\n{}\ngix index: {actual_entries:#?}\nGit index: {expected_entries:#?}\ngix conflicts: {:#?}",
                case.case_name,
                actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
                baseline::visualize_tree(&actual_tree_id, &case.odb, None),
                baseline::visualize_tree(&case.merge_info.merged_tree, &case.odb, None),
                actual.conflicts
            )?;
        }
    }
    assert_eq!(count, 36, "all layouts, policies, and merge directions are covered");
    assert!(failures.is_empty(), "directory outcomes differ from Git:\n{failures}");
    Ok(())
}
