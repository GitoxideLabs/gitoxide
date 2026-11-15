use std::fmt::Write as _;

use gix_merge::tree::TreatAsUnresolved;
use gix_object::{FindExt, Write};
use gix_testtools::TestResult;

use super::{baseline, basic_merge_options, new_blob_merge_platform, new_diff_resource_cache};

#[test]
fn generated_structural_histories_match_git() -> TestResult {
    let Some(root) =
        gix_testtools::scripted_fixture_read_only_with_git_version("tree-structural-differential.sh", |version| {
            version >= (2, 38, 0)
        })?
    else {
        return Ok(());
    };
    let cases = std::fs::read_to_string(root.join("baseline.cases"))?;
    let mut failures = Vec::new();
    let mut count = 0;
    for case in baseline::Expectations::new(&root, &cases) {
        assert!(case.deviation.is_none(), "generated histories must agree with Git");
        count += 1;
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
            basic_merge_options(),
        )?
        .tree_merge;
        let actual_tree_id = actual.tree.write(|tree| case.odb.write(tree))?;
        let expected_status = std::fs::read_to_string(
            case.root
                .join(format!(".git/{}--{}.status", case.our_side_name, case.their_side_name)),
        )?;
        let expected_conflicted = match expected_status.trim() {
            "0" => false,
            "1" => true,
            other => panic!("{}: unexpected Git merge status {other}", case.case_name),
        };
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
        let unknown_conflict = actual.conflicts.iter().any(|conflict| {
            matches!(
                conflict.resolution,
                Err(gix_merge::tree::ResolutionFailure::Unknown)
                    | Ok(gix_merge::tree::Resolution::Forced(
                        gix_merge::tree::ResolutionFailure::Unknown
                    ))
            )
        });
        if actual_tree_id != case.merge_info.merged_tree
            || actual_entries != expected_entries
            || actual_conflicted != expected_conflicted
            || unknown_conflict
        {
            let mut details = format!(
                "{}\nGit reproducer: git -C '{}' merge-tree --write-tree {} {}\n\
                 conflict status: gix={actual_conflicted}, Git={expected_conflicted}\n",
                case.case_name,
                case.root.display(),
                case.our_side_name,
                case.their_side_name,
            );
            writeln!(
                details,
                "gix tree:\n{}",
                baseline::visualize_tree(&actual_tree_id, &case.odb, None)
            )?;
            writeln!(
                details,
                "Git tree:\n{}",
                baseline::visualize_tree(&case.merge_info.merged_tree, &case.odb, None)
            )?;
            writeln!(
                details,
                "gix index: {actual_entries:#?}\nGit index: {expected_entries:#?}"
            )?;
            writeln!(details, "gix conflicts: {:#?}", actual.conflicts)?;
            let size = side_entry_count(&case, case.our_commit_id)? + side_entry_count(&case, case.their_commit_id)?;
            failures.push((size, case.case_name, details));
        }
    }
    assert_eq!(
        count, 170,
        "every generated history must be compared in both directions"
    );
    // Every optional operation also has a reduced fixture variant; report a smallest failing input.
    failures.sort_by_key(|(size, _, _)| *size);
    assert!(
        failures.is_empty(),
        "{}/{} generated histories disagree with Git; input sizes and cases: {:?}\n\
         Smallest generated failing history:\n{}",
        failures.len(),
        count,
        failures.iter().map(|(size, name, _)| (size, name)).collect::<Vec<_>>(),
        failures.first().map_or("", |(_, _, details)| details.as_str()),
    );
    Ok(())
}

fn side_entry_count(case: &baseline::Expectation, commit_id: gix_hash::ObjectId) -> TestResult<usize> {
    let mut buf = Vec::new();
    let mut pending = vec![case.odb.find_commit(&commit_id, &mut buf)?.tree()];
    let mut count = 0;
    while let Some(tree_id) = pending.pop() {
        for entry in case.odb.find_tree(&tree_id, &mut buf)?.entries {
            if entry.mode.is_tree() {
                pending.push(entry.oid.to_owned());
            } else {
                count += 1;
            }
        }
    }
    Ok(count)
}
