use gix_merge::{blob::pipeline::Mode, tree::TreatAsUnresolved};
use gix_object::Write;
use gix_testtools::TestResult;

use super::{baseline, basic_merge_options, new_blob_merge_platform, new_diff_resource_cache};

#[test]
fn normalized_modify_delete_matches_git() -> TestResult {
    let Some(root) =
        gix_testtools::scripted_fixture_read_only_with_git_version("tree-renormalized-delete.sh", |version| {
            version >= (2, 38, 0)
        })?
    else {
        return Ok(());
    };
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    let mut failures = Vec::new();
    let mut count = 0;
    for case in baseline::Expectations::new(&root, &cases) {
        count += 1;
        let renormalize = case.case_name.contains("-true-");
        let mut blob_merge = new_blob_merge_platform(&case.root, 0);
        blob_merge.filter_mode = if renormalize { Mode::Renormalize } else { Mode::ToGit };
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
            &mut blob_merge,
            &case.odb,
            &mut |commit_id| commit_id.to_hex_with_len(7).to_string(),
            gix_merge::tree::Context::default(),
            basic_merge_options(),
        )?
        .tree_merge;
        let actual_tree_id = actual.tree.write(|tree| case.odb.write(tree))?;
        let status = std::fs::read_to_string(
            case.root
                .join(format!(".git/{}--{}.status", case.our_side_name, case.their_side_name)),
        )?;
        let expected_conflicted = match status.trim() {
            "0" => false,
            "1" => true,
            other => panic!("{}: unexpected Git merge status {other}", case.case_name),
        };
        let should_resolve =
            renormalize && (case.case_name.contains("-newline-") || case.case_name.contains("-directory-true-"));
        assert_eq!(
            expected_conflicted, !should_resolve,
            "{}: Git removes normalization-only edits but preserves mode, content, and rename conflicts",
            case.case_name
        );
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
        let actual_conflicted = actual.has_unresolved_conflicts(TreatAsUnresolved::git());
        if actual_tree_id != case.merge_info.merged_tree
            || actual_entries != expected_entries
            || actual_conflicted != expected_conflicted
        {
            failures.push(format!(
                "{}: conflict status gix={actual_conflicted}, Git={expected_conflicted}; \
                 tree gix={actual_tree_id}, Git={}\ngix index: {actual_entries:#?}\nGit index: {expected_entries:#?}",
                case.case_name, case.merge_info.merged_tree,
            ));
        }
    }
    assert_eq!(
        count, 24,
        "all layouts and normalization settings run in both directions"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}
