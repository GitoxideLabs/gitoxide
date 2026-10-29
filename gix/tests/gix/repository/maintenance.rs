use gix_testtools::TestResult;
use std::{collections::BTreeSet, sync::atomic::AtomicBool};

#[test]
fn roots_protect_tags_reflogs_indices_and_all_worktrees() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_maintenance_roots.sh")?;
    for name in ["repo/.git", "repo/.git/worktrees/linked"] {
        let mut repo = gix::open_opts(fixture.join(name), crate::restricted().open_path_as_is(true))?;
        repo.set_namespace("nested/space")?;
        let expected_namespace = repo.namespace().cloned();
        let roots = repo.object_reachability_roots(&AtomicBool::default())?;
        let root_set: BTreeSet<_> = roots.iter().map(ToString::to_string).collect();
        for expected in std::fs::read_to_string(fixture.join("expected"))?.lines() {
            assert!(root_set.contains(expected), "{name}: root {expected} remains protected");
        }
        let forbidden = std::fs::read_to_string(fixture.join("missing-gitlink"))?;
        assert!(
            !root_set.contains(forbidden.trim()),
            "gitlinks refer to other repositories"
        );
        assert_eq!(
            repo.namespace(),
            expected_namespace.as_ref(),
            "the caller's namespace stays unchanged"
        );
        let mut objects = repo.objects.clone();
        objects.ignore_replacements = true;
        let reachable: BTreeSet<_> = gix::traverse::object::BreadthFirst::new(roots, &objects)
            .map(|entry| entry.map(|entry| entry.id.to_string()))
            .collect::<gix::Result<_>>()?;
        for expected in std::fs::read_to_string(fixture.join("git-reachable"))?.lines() {
            assert!(
                reachable.contains(expected),
                "{name}: object {expected} reachable according to Git is retained"
            );
        }
    }
    Ok(())
}

#[test]
fn malformed_reflogs_fail_and_missing_reflog_objects_are_ignored() -> TestResult {
    let (repo, _directory) = crate::basic_rw_repo()?;
    let root_before = repo.object_reachability_roots(&AtomicBool::default())?;
    let missing = "1".repeat(repo.object_hash().len_in_hex());
    let log = repo.git_dir().join("logs/refs/heads/deleted");
    std::fs::create_dir_all(log.parent().expect("log has a parent"))?;
    std::fs::write(&log, format!("{missing} {missing} A <a@example.com> 1 +0000\tpruned\n"))?;
    assert_eq!(
        repo.object_reachability_roots(&AtomicBool::default())?,
        root_before,
        "already-pruned reflog entries add no roots"
    );
    std::fs::write(log, b"not a reflog line")?;
    repo.object_reachability_roots(&AtomicBool::default())
        .expect_err("malformed logs cannot silently hide roots");
    assert!(
        repo.object_reachability_roots(&AtomicBool::new(true))
            .expect_err("interrupt before reading state")
            .is_cancelled()
    );
    Ok(())
}
