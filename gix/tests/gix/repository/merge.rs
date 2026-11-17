use crate::util::named_repo;
use gix::merge::tree::DirectoryRenames;
use gix_testtools::TestResult;

#[test]
fn large_text_merges_ignore_the_diff_size_threshold() -> TestResult {
    use gix::merge::{plumbing::blob::builtin_driver::text::Labels, tree::TreatAsUnresolved};

    let Some(fixture) =
        gix_testtools::scripted_fixture_read_only_with_git_version("make_large_text_merge_repo.sh", |version| {
            version >= (2, 38, 0)
        })?
    else {
        return Ok(());
    };
    let repo = gix::open_opts(&fixture, crate::util::restricted())?.with_object_memory();
    let tree_ids = std::fs::read_to_string(fixture.join("trees"))?
        .lines()
        .map(|line| gix::ObjectId::from_hex(line.as_bytes()))
        .collect::<Result<Vec<_>, _>>()?;
    for ((side, conflicted), their_tree_id) in [("B", false), ("C", true)].into_iter().zip(&tree_ids[2..]) {
        let expected = std::fs::read_to_string(
            repo.workdir()
                .expect("fixture has a worktree")
                .join(format!("{side}.merge-info")),
        )?;
        let expected_tree_id = gix::ObjectId::from_hex(
            expected
                .split('\0')
                .next()
                .expect("Git emits its result tree")
                .as_bytes(),
        )?;
        let mut outcome = repo.merge_trees(
            tree_ids[0],
            tree_ids[1],
            *their_tree_id,
            Labels {
                ancestor: None,
                current: Some("A".into()),
                other: Some(side.into()),
            },
            repo.tree_merge_options()?,
        )?;
        assert_eq!(
            outcome.tree.write()?,
            expected_tree_id,
            "text beyond the diff threshold merges like Git"
        );
        assert_eq!(
            outcome.has_unresolved_conflicts(TreatAsUnresolved::git()),
            conflicted,
            "only overlapping text edits conflict"
        );
    }
    Ok(())
}

#[test]
fn final_index_is_independent_of_mutable_tree_and_diagnostics() -> TestResult {
    use gix::{
        bstr::BString,
        index::entry::{Mode, Stage},
        merge::tree::TreatAsUnresolved,
        object::tree::EntryKind,
    };

    let repo = named_repo("make_basic_repo.sh")?.with_object_memory();
    let tree = |content: &str| -> gix::Result<_> {
        let blob_id = repo.write_blob(content)?;
        let tree_id = repo
            .edit_tree(repo.object_hash().empty_tree())?
            .upsert("file", EntryKind::Blob, blob_id)?
            .write()?;
        Ok((tree_id, blob_id))
    };
    let (base_tree_id, base_blob_id) = tree("base\n")?;
    let (our_tree_id, our_blob_id) = tree("ours\n")?;
    let (their_tree_id, their_blob_id) = tree("theirs\n")?;
    let mut outcome = repo.merge_trees(
        base_tree_id,
        our_tree_id,
        their_tree_id,
        Default::default(),
        repo.tree_merge_options()?,
    )?;
    let how = TreatAsUnresolved::git();
    outcome.conflicts.clear();
    assert_eq!(
        outcome.tree.remove("file")?.write()?,
        repo.object_hash().empty_tree(),
        "the caller can edit the provisional tree independently"
    );
    assert!(
        outcome.has_unresolved_conflicts(how),
        "clearing diagnostics does not clear the finalized conflict verdict"
    );
    let entries = |index: &gix::index::State| {
        index
            .entries()
            .iter()
            .map(|entry| (entry.path(index).to_owned(), entry.stage(), entry.mode, entry.id))
            .collect::<Vec<_>>()
    };
    let expected = [
        (Stage::Base, base_blob_id),
        (Stage::Ours, our_blob_id),
        (Stage::Theirs, their_blob_id),
    ]
    .map(|(stage, blob_id)| (BString::from("file"), stage, Mode::FILE, blob_id.detach()));
    let index = outcome.index(how)?;
    index.verify_entries()?;
    assert_eq!(
        entries(&index),
        expected,
        "the index retains the final merge stages after its provisional tree is edited"
    );

    let index_on_disk = std::fs::read(repo.index_path())?;
    let file = outcome.index_file(how)?;
    assert_eq!(
        std::fs::read(repo.index_path())?,
        index_on_disk,
        "constructing the index file does not write the repository's index"
    );
    let plan = outcome.index.clone();
    drop(outcome);
    assert_eq!(
        file.path(),
        repo.index_path(),
        "the file targets this repository's index"
    );
    assert_eq!(
        entries(&file),
        expected,
        "the index file retains the final merge stages after the merge outcome is dropped"
    );
    assert_eq!(
        entries(&plan.to_index(how, &repo, Default::default())?),
        expected,
        "a detached plan can construct the same index after the merge outcome is dropped"
    );
    Ok(())
}

#[test]
fn tree_merge_options() -> TestResult {
    let repo = named_repo("make_basic_repo.sh")?;
    let opts: gix::merge::plumbing::tree::Options = repo.tree_merge_options()?.into();
    assert_eq!(
        opts.rewrites,
        Some(gix::diff::Rewrites::default()),
        "If merge options aren't set, it defaults to diff options, and these default to doing rename tracking"
    );
    assert_eq!(
        opts.directory_renames,
        DirectoryRenames::Conflict,
        "repository-derived options use Git's conflict default, like the plumbing options"
    );
    Ok(())
}

#[test]
fn tree_merge_options_directory_renames() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;
    for renames in ["merge.renames=false", "diff.renames=false"] {
        for directory_renames in ["merge.directoryRenames=bogus", "merge.directoryRenames=true"] {
            let repo = gix::open_opts(
                &fixture,
                crate::util::restricted()
                    .strict_config(true)
                    .config_overrides([renames, directory_renames]),
            )?;
            let opts: gix::merge::plumbing::tree::Options = repo.tree_merge_options()?.into();
            assert_eq!(
                opts.rewrites, None,
                "directory renames do not re-enable rewrite tracking"
            );
            assert_eq!(
                opts.directory_renames,
                DirectoryRenames::Disabled,
                "disabled rewrite tracking ignores even invalid directory rename configuration"
            );
        }
    }
    let repo = gix::open_opts(
        &fixture,
        crate::util::restricted()
            .strict_config(true)
            .config_overrides(["merge.renames=true", "merge.directoryRenames=bogus"]),
    )?;
    assert!(
        repo.tree_merge_options().is_err(),
        "strict configuration still rejects invalid directory rename settings when rewrites are enabled"
    );
    Ok(())
}
