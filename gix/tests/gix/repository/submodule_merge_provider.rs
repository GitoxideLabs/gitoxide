use gix::{ObjectId, index::entry::Stage, merge::tree::TreatAsUnresolved};
use gix_testtools::TestResult;

fn fixture() -> gix_testtools::Result<Option<std::path::PathBuf>> {
    gix_testtools::scripted_fixture_read_only_with_git_version("merge_submodule_provider.sh", |version| {
        version >= (2, 38, 0)
    })
}

#[test]
fn local_submodule_history_respects_modified_graphs_and_linked_repositories() -> TestResult {
    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    for layout in [
        "shallow",
        "replacement",
        "missing-graph-tip",
        "linked-submodule",
        "linked-super",
        "incomplete-modules",
    ] {
        let root = fixture.join(layout);
        let repo = gix::open_opts(&root, crate::util::restricted())?.with_object_memory();
        let tree_ids = std::fs::read_to_string(root.join("trees"))?
            .lines()
            .map(|line| ObjectId::from_hex(line.as_bytes()))
            .collect::<Result<Vec<_>, _>>()?;
        for (current, other, direction) in [(1, 2, "A-B"), (2, 1, "B-A")] {
            let expected_tree_id = std::fs::read_to_string(root.join(format!("{direction}.tree")))?;
            let expected_tree_id = ObjectId::from_hex(expected_tree_id.trim().as_bytes())?;
            let expected_conflict = std::fs::read_to_string(root.join(format!("{direction}.status")))?.trim() == "1";
            let mut outcome = repo.merge_trees(
                tree_ids[0],
                tree_ids[current],
                tree_ids[other],
                Default::default(),
                repo.tree_merge_options()?,
            )?;
            assert_eq!(
                outcome.tree.write()?,
                expected_tree_id,
                "{layout} {direction}: the local submodule history produces Git's tree"
            );
            assert_eq!(
                outcome.has_unresolved_conflicts(TreatAsUnresolved::git()),
                expected_conflict,
                "{layout} {direction}: submodule history availability matches Git's verdict"
            );
            let index = outcome.index(TreatAsUnresolved::git(), None)?;
            assert_eq!(
                index
                    .entries()
                    .iter()
                    .filter(|entry| entry.stage() != Stage::Unconflicted)
                    .count(),
                if expected_conflict { 3 } else { 0 },
                "{layout} {direction}: unresolved gitlinks retain all three source stages"
            );
        }
    }
    Ok(())
}

#[test]
fn configure_related_repository_skip_prevents_merge_fallback() -> TestResult {
    use std::{
        ops::ControlFlow,
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    let root = fixture.join("corrupt-config");
    let calls = Arc::new(AtomicUsize::new(0));
    let repo = crate::util::restricted()
        .configure_related_repository({
            let calls = Arc::clone(&calls);
            move |related, _| {
                assert_eq!(related.kind, gix::open::RelatedRepositoryKind::Submodule);
                calls.fetch_add(1, Ordering::Relaxed);
                Ok(ControlFlow::Break(()))
            }
        })
        .open(&root)?
        .with_object_memory();
    let tree_ids = std::fs::read_to_string(root.join("trees"))?
        .lines()
        .map(|line| ObjectId::from_hex(line.as_bytes()))
        .collect::<Result<Vec<_>, _>>()?;
    let outcome = repo.merge_trees(
        tree_ids[0],
        tree_ids[1],
        tree_ids[2],
        Default::default(),
        repo.tree_merge_options()?,
    )?;
    assert!(
        outcome.has_unresolved_conflicts(TreatAsUnresolved::git()),
        "merging the gitlink requires the skipped repository's history"
    );
    assert_eq!(
        calls.load(Ordering::Relaxed),
        1,
        "skip ends the lookup for this submodule"
    );
    Ok(())
}

#[test]
fn corrupt_submodule_repository_is_an_error() -> TestResult {
    let Some(fixture) = fixture()? else {
        return Ok(());
    };
    let root = fixture.join("corrupt-config");
    let repo = gix::open_opts(&root, crate::util::restricted())?.with_object_memory();
    let tree_ids = std::fs::read_to_string(root.join("trees"))?
        .lines()
        .map(|line| ObjectId::from_hex(line.as_bytes()))
        .collect::<Result<Vec<_>, _>>()?;
    for (current, other) in [(1, 2), (2, 1)] {
        let error = repo
            .merge_trees(
                tree_ids[0],
                tree_ids[current],
                tree_ids[other],
                Default::default(),
                repo.tree_merge_options()?,
            )
            .err()
            .expect("corrupt local repository configuration must not become a resolved or unavailable merge");
        assert!(
            error
                .to_string()
                .contains("Could not inspect history of submodule module"),
            "the error identifies the submodule query that failed: {error}"
        );
    }
    Ok(())
}
