use gix_error::Result;
use gix_merge::tree::{DirectoryRenames, ResolveWith, TreatAsUnresolved};
use gix_object::Write;
use gix_testtools::TestResult;

use super::{baseline, new_blob_merge_platform, new_diff_resource_cache};

fn fixture() -> gix_testtools::Result<Option<std::path::PathBuf>> {
    gix_testtools::scripted_fixture_read_only_with_git_version("tree-rename-parent-namespace.sh", |version| {
        // Git 2.51.1 fixes returned-to-source rename stages; older releases can also abort.
        version >= (2, 51, 1)
    })
}

fn tree_ids(root: &std::path::Path) -> gix_testtools::Result<Vec<gix_hash::ObjectId>> {
    Ok(std::fs::read_to_string(root.join("trees"))?
        .lines()
        .map(|line| gix_hash::ObjectId::from_hex(line.as_bytes()))
        .collect::<std::result::Result<_, _>>()?)
}

fn merge<'a>(
    root: &std::path::Path,
    objects: &'a (impl gix_object::FindObjectOrHeader + Write),
    tree_ids: [gix_hash::ObjectId; 3],
    options: gix_merge::tree::Options,
) -> Result<gix_merge::tree::Outcome<'a>> {
    gix_merge::tree(
        &tree_ids[0],
        &tree_ids[1],
        &tree_ids[2],
        Default::default(),
        objects,
        |buf| objects.write_buf(gix_object::Kind::Blob, buf),
        &mut Default::default(),
        &mut new_diff_resource_cache(root),
        &mut new_blob_merge_platform(root, None),
        Default::default(),
        options,
    )
}

fn options() -> gix_merge::tree::Options {
    gix_merge::tree::Options {
        rewrites: Some(gix_diff::Rewrites {
            percentage: Some(1.0),
            limit: 0,
            copies: None,
            track_empty: false,
        }),
        ..Default::default()
    }
}

#[test]
fn inferred_moves_back_to_a_rename_source_preserve_its_conflict_stages() -> TestResult {
    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    for layout in ["repo", "distinct"] {
        let root = fixture.join(layout);
        let hash = gix_testtools::object_hash();
        let objects = gix_odb::memory::Proxy::new(gix_odb::at(root.join(".git/objects"), hash)?, hash);
        let tree_ids = tree_ids(&root)?;
        for (current, other, direction) in [(1, 2, "A-B"), (2, 1, "B-A")] {
            // Git skips a returned-to-source rename before checking the next pair.
            // In A-B that loses its rename/rename conflict; B-A checks the pair in
            // the opposite order and preserves both destinations. The distinct-blob
            // control rules out rename ties. Use its conservative B-A result in
            // both directions, with the conflict sides exchanged for A-B.
            let mut expected = baseline::parse_merge_info(std::fs::read_to_string(root.join("B-A.merge-info"))?);
            if current == 1 {
                for conflict in expected
                    .conflicts
                    .as_mut()
                    .expect("Git B-A retains the rename conflict")
                {
                    std::mem::swap(&mut conflict.ours, &mut conflict.theirs);
                }
            }
            let mut actual = merge(
                &root,
                &objects,
                [tree_ids[0], tree_ids[current], tree_ids[other]],
                options(),
            )?;
            let tree_id = actual.tree.write(|tree| objects.write(tree))?;
            let index = actual
                .index
                .to_index(TreatAsUnresolved::git(), &objects, Default::default())?;
            index.verify_entries()?;
            let mut expected_index = gix_index::State::from_tree(&expected.merged_tree, &objects, Default::default())?;
            if let Some(conflicts) = &expected.conflicts {
                baseline::apply_git_index_entries(conflicts, &mut expected_index);
            }
            assert_eq!(
                tree_id, expected.merged_tree,
                "{layout} {direction}: both rename destinations survive regardless of side order"
            );
            assert_eq!(
                baseline::debug_entries(&index),
                baseline::debug_entries(&expected_index),
                "{layout} {direction}: the return-to-source rename owns its unresolved stages"
            );
            assert!(
                actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
                "{layout} {direction}: two rename destinations remain a conflict"
            );
        }
    }
    Ok(())
}

#[test]
fn forced_rename_choices_preserve_the_selected_file_directory_namespace() -> TestResult {
    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    let root = fixture.join("file-directory");
    let hash = gix_testtools::object_hash();
    let objects = gix_odb::memory::Proxy::new(gix_odb::at(root.join(".git/objects"), hash)?, hash);
    let tree_ids = tree_ids(&root)?;
    for (current, other, direction) in [(1, 2, "A-B"), (2, 1, "B-A")] {
        for directory in [DirectoryRenames::Enabled, DirectoryRenames::Conflict] {
            for early_stop in [false, true] {
                let mut options = options();
                options.directory_renames = directory;
                options.tree_conflicts = Some(ResolveWith::Ours);
                options.fail_on_conflict = early_stop.then(TreatAsUnresolved::git);
                let mut actual = merge(
                    &root,
                    &objects,
                    [tree_ids[0], tree_ids[current], tree_ids[other]],
                    options,
                )?;
                let tree_id = actual.tree.write(|tree| objects.write(tree))?;
                let provisional_index = gix_index::State::from_tree(&tree_id, &objects, Default::default())?;
                provisional_index.verify_entries()?;
                let selected_path = if current == 1 { "a/a/a" } else { "a" };
                let selected_index = gix_index::State::from_tree(&tree_ids[current], &objects, Default::default())?;
                let selected = selected_index
                    .entries()
                    .iter()
                    .find(|entry| entry.path(&selected_index) == selected_path)
                    .expect("the chosen input has its directory child or parent file");
                let retained = provisional_index
                    .entries()
                    .iter()
                    .find(|entry| entry.path(&provisional_index) == selected_path)
                    .expect("choosing a side preserves its independent file");
                assert_eq!(
                    retained.id, selected.id,
                    "{direction}, directory={directory:?}, early={early_stop}: selecting the current namespace retains its original data"
                );
                assert!(
                    provisional_index.entries().iter().all(|entry| {
                        let path = entry.path(&provisional_index);
                        if current == 1 {
                            path != "a"
                        } else {
                            !path.starts_with(b"a/")
                        }
                    }),
                    "{direction}, directory={directory:?}, early={early_stop}: a competing parent or descendant cannot replace the selected namespace"
                );
                let index = actual
                    .index
                    .to_index(TreatAsUnresolved::git(), &objects, Default::default())?;
                index.verify_entries()?;
                assert!(
                    !actual.has_unresolved_conflicts(TreatAsUnresolved::git()),
                    "{direction}: choosing the current namespace resolves its structural conflicts"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn a_single_return_to_source_rename_matches_git_with_an_opposite_deletion() -> TestResult {
    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    let root = fixture.join("single-return");
    let hash = gix_testtools::object_hash();
    let objects = gix_odb::memory::Proxy::new(gix_odb::at(root.join(".git/objects"), hash)?, hash);
    let tree_ids = tree_ids(&root)?;
    for (current, other, direction) in [(1, 2, "A-B"), (2, 1, "B-A")] {
        for (directory, policy) in [
            (DirectoryRenames::Enabled, "true"),
            (DirectoryRenames::Conflict, "conflict"),
        ] {
            let expected = baseline::parse_merge_info(std::fs::read_to_string(
                root.join(format!("{direction}.{policy}.merge-info")),
            )?);
            let mut options = options();
            options.directory_renames = directory;
            let mut actual = merge(
                &root,
                &objects,
                [tree_ids[0], tree_ids[current], tree_ids[other]],
                options,
            )?;
            let tree_id = actual.tree.write(|tree| objects.write(tree))?;
            let index = actual
                .index
                .to_index(TreatAsUnresolved::git(), &objects, Default::default())?;
            let mut expected_index = gix_index::State::from_tree(&expected.merged_tree, &objects, Default::default())?;
            if let Some(conflicts) = &expected.conflicts {
                baseline::apply_git_index_entries(conflicts, &mut expected_index);
            }
            assert_eq!(
                tree_id, expected.merged_tree,
                "{direction}, {directory:?}: a lone identity follows Git's directory relocation policy"
            );
            assert_eq!(
                baseline::debug_entries(&index),
                baseline::debug_entries(&expected_index),
                "{direction}, {directory:?}: a lone returned-to-source rename retains the required stages"
            );
        }
    }
    Ok(())
}
