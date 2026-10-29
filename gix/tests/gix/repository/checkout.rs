use std::{fs, sync::atomic::AtomicBool};

use gix::{
    Repository,
    index::{
        State,
        entry::{Flags, Mode, Stage},
    },
    progress,
    worktree::checkout::Options,
};
use gix_testtools::TestResult;

#[test]
fn checkout_updates_only_changed_paths_and_does_not_persist_the_index() -> TestResult {
    let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
    let workdir = repo.workdir().expect("non-bare");
    let baseline = index(&repo, &[("change", "old"), ("delete", "old"), ("same", "old")])?;
    let mut target = index(&repo, &[("added", "new"), ("change", "new"), ("same", "old")])?;
    let mut current = baseline.clone();
    current
        .entry_mut_by_path_and_stage("same".into(), Stage::Unconflicted)
        .expect("tracked")
        .id = repo.write_blob(b"staged")?.detach();
    for path in ["change", "delete"] {
        current
            .entry_mut_by_path_and_stage(path.into(), Stage::Unconflicted)
            .expect("tracked")
            .id = repo.write_blob(b"staged at a changed path")?.detach();
    }
    fs::write(workdir.join("change"), b"old")?;
    fs::write(workdir.join("delete"), b"old")?;
    fs::write(workdir.join("same"), b"unstaged")?;
    fs::write(workdir.join("untracked"), b"keep")?;
    let mut on_disk = gix::index::File::from_state(current.clone(), repo.index_path());
    on_disk.write(Default::default())?;
    let index_before = fs::read(repo.index_path())?;

    let out = checkout(
        &repo,
        &baseline,
        &mut target,
        &mut current,
        Options {
            overwrite_staged: true,
            ..Default::default()
        },
    )?;

    assert!(out.index_changed, "the caller must persist the updated staging index");
    assert_eq!(
        fs::read(workdir.join("change"))?,
        b"new",
        "the changed path is materialized"
    );
    assert_eq!(
        fs::read(workdir.join("added"))?,
        b"new",
        "the added path is materialized"
    );
    assert!(!workdir.join("delete").exists(), "the removed path is absent");
    assert_eq!(
        fs::read(workdir.join("same"))?,
        b"unstaged",
        "unrelated worktree edits survive"
    );
    assert_eq!(fs::read(workdir.join("untracked"))?, b"keep", "untracked data survives");
    assert_eq!(
        current.entry_by_path("same".into()).expect("still tracked").id,
        repo.write_blob(b"staged")?.detach(),
        "unrelated staged content survives"
    );
    assert!(
        current.entry_by_path("delete".into()).is_none(),
        "the in-memory index follows the selected deletion"
    );
    assert_eq!(
        fs::read(repo.index_path())?,
        index_before,
        "the API never persists the index"
    );
    Ok(())
}

#[test]
fn staged_changes_at_target_paths_match_git_read_tree() -> TestResult {
    for case in [
        "modified",
        "deleted",
        "added",
        "staged-deletion",
        "staged-mode",
        "already-target",
        "already-target-dirty",
        "added-at-target",
        "deleted-at-target",
        "deleted-at-target-recreated",
        "mode-at-target",
        "file-to-directory-at-target",
        "file-to-directory-at-target-empty",
        "file-to-directory-at-target-missing",
        "file-to-directory-at-target-dirty",
        "file-to-directory-at-target-untracked",
        "directory-to-file-at-target",
        "directory-to-file-at-target-missing",
        "directory-to-file-at-target-dirty",
        "deleted-at-target-empty-directory",
        "deleted-at-target-untracked-directory",
        "gitlink-deleted-at-target-empty-directory",
        "gitlink-deleted-at-target-populated-directory",
        "gitlink-deleted-at-target-recreated-file",
    ] {
        let mut snapshots = Vec::new();
        for use_git in [true, false] {
            let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
            let workdir = repo.workdir().expect("non-bare");
            let added = case.starts_with("added");
            let deleted = case == "deleted" || case.contains("deleted-at-target");
            let already_target = case.contains("target");
            let file_to_directory = case.starts_with("file-to-directory");
            let directory_to_file = case.starts_with("directory-to-file");
            let gitlink = case.starts_with("gitlink");
            let previous_path = if directory_to_file { "path/child" } else { "path" };
            let desired_path = if file_to_directory { "path/child" } else { "path" };
            let mut baseline_entries = vec![("a-delete", "old")];
            if !added {
                baseline_entries.push((previous_path, "old"));
            }
            let mut baseline = index(&repo, &baseline_entries)?;
            if gitlink {
                baseline
                    .entry_mut_by_path_and_stage("path".into(), Stage::Unconflicted)
                    .expect("baseline gitlink")
                    .mode = Mode::COMMIT;
            }
            let target_entries = [(desired_path, "new")];
            let mut target = index(&repo, if deleted { &[] } else { &target_entries })?;
            if case == "mode-at-target" {
                target.entries_mut()[0].id = repo.write_blob(b"old")?.detach();
                target.entries_mut()[0].mode = Mode::FILE_EXECUTABLE;
            }
            gix::index::File::from_state(target.clone(), repo.index_path()).write(Default::default())?;
            let target_tree_id = gix_testtools::git(workdir, "write-tree")?;
            gix::index::File::from_state(baseline.clone(), repo.index_path()).write(Default::default())?;
            let baseline_tree_id = gix_testtools::git(workdir, "write-tree")?;
            gix_testtools::git(workdir, &format!("read-tree --reset -u {}", baseline_tree_id.trim()))?;
            let mut current = (*repo.open_index()?).clone();
            current.remove_tree();
            current.remove_entries(|_, path, _| path == previous_path);
            if case != "staged-deletion" && !(already_target && deleted) {
                let (object_id, mode) = if already_target {
                    (target.entries()[0].id, target.entries()[0].mode)
                } else if case == "staged-mode" {
                    (
                        baseline.entry_by_path("path".into()).expect("baseline path").id,
                        Mode::FILE_EXECUTABLE,
                    )
                } else {
                    (repo.write_blob(b"staged only")?.detach(), Mode::FILE)
                };
                current.dangerously_push_entry(
                    Default::default(),
                    object_id,
                    Flags::empty(),
                    mode,
                    desired_path.into(),
                );
                current.sort_entries();
            }
            if file_to_directory {
                fs::remove_file(workdir.join("path"))?;
                if !case.ends_with("-missing") {
                    fs::create_dir(workdir.join("path"))?;
                    if !case.ends_with("-empty") {
                        fs::write(workdir.join("path/child"), "new")?;
                    }
                    if case.ends_with("-untracked") {
                        fs::write(workdir.join("path/untracked"), "keep")?;
                    }
                }
            } else if directory_to_file {
                fs::remove_file(workdir.join("path/child"))?;
                fs::remove_dir(workdir.join("path"))?;
                if !case.ends_with("-missing") {
                    fs::write(workdir.join("path"), "new")?;
                }
            } else if deleted && case.ends_with("-directory") {
                if !gitlink {
                    fs::remove_file(workdir.join("path"))?;
                    fs::create_dir(workdir.join("path"))?;
                }
                if !case.ends_with("-empty-directory") {
                    fs::write(workdir.join("path/untracked"), "keep")?;
                }
            } else if gitlink {
                fs::remove_dir(workdir.join("path"))?;
                fs::write(workdir.join("path"), "untracked")?;
            } else if case == "already-target-dirty" {
                fs::write(workdir.join("path"), "unstaged only")?;
            } else if case == "deleted-at-target" {
                fs::remove_file(workdir.join("path"))?;
            }
            gix::index::File::from_state(current.clone(), repo.index_path()).write(Default::default())?;
            if file_to_directory || directory_to_file {
                // Structural checks in read-tree require refreshed stat data for clean target entries.
                gix_testtools::git(workdir, "update-index --ignore-missing --refresh")?;
                current = (*repo.open_index()?).clone();
                if case.ends_with("-dirty") {
                    fs::write(workdir.join(desired_path), "dirty")?;
                }
            }
            let before = gix_testtools::repository::snapshot(workdir)?;
            let success = if use_git {
                let output = gix_testtools::git_command(workdir)
                    .args(["read-tree", "-m", "-u", baseline_tree_id.trim(), target_tree_id.trim()])
                    .output()?;
                assert!(
                    output.status.success() || !String::from_utf8_lossy(&output.stderr).contains("a-delete"),
                    "the oracle must reach the staged path check: {output:?}"
                );
                output.status.success()
            } else {
                let success = checkout(&repo, &baseline, &mut target, &mut current, Options::default()).is_ok();
                gix::index::File::from_state(current, repo.index_path()).write(Default::default())?;
                success
            };
            let expected_success = already_target
                && !matches!(
                    case,
                    "deleted-at-target-recreated"
                        | "file-to-directory-at-target-dirty"
                        | "file-to-directory-at-target-untracked"
                        | "directory-to-file-at-target-dirty"
                        | "deleted-at-target-untracked-directory"
                        | "gitlink-deleted-at-target-recreated-file"
                );
            assert_eq!(success, expected_success, "staged {case} refusal, Git: {use_git}");
            let after = gix_testtools::repository::snapshot(workdir)?;
            if !success {
                assert_eq!(after.index, before.index, "refusal preserves staged {case}");
                assert_eq!(
                    after.worktree, before.worktree,
                    "refusal precedes every worktree write for {case}"
                );
            }
            snapshots.push(after);
        }
        assert_eq!(
            snapshots[0].index, snapshots[1].index,
            "staged entries agree with Git for {case}"
        );
        assert_eq!(
            snapshots[0].worktree, snapshots[1].worktree,
            "unstaged content agrees with Git for {case}"
        );
    }
    Ok(())
}

#[test]
fn refuses_dirty_untracked_and_ignored_blockers_before_any_writes() -> TestResult {
    for kind in [
        "dirty",
        "untracked",
        "ignored",
        "directory",
        "ignored-directory",
        "nested-repository",
    ] {
        for overwrite_tracked in [false, true] {
            if kind == "dirty" && overwrite_tracked {
                continue;
            }
            let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
            let workdir = repo.workdir().expect("non-bare");
            let mut entries = vec![("a-delete", "old")];
            if kind == "dirty" {
                entries.push(("target", "old"));
            }
            let baseline = index(&repo, &entries)?;
            let mut target = index(&repo, &[("target", "new")])?;
            let mut current = baseline.clone();
            fs::write(workdir.join("a-delete"), b"old")?;
            if kind.ends_with("directory") || kind == "nested-repository" {
                fs::create_dir(workdir.join("target"))?;
                fs::write(workdir.join("target/untracked"), b"keep")?;
                if kind == "nested-repository" {
                    crate::init_repo_isolated(workdir.join("target"), gix::create::Kind::WithWorktree)?;
                }
            } else {
                fs::write(workdir.join("target"), b"keep")?;
            }
            if kind.starts_with("ignored") {
                fs::write(workdir.join(".gitignore"), b"target\n")?;
            }
            let before = current.entries().to_vec();
            let err = checkout(
                &repo,
                &baseline,
                &mut target,
                &mut current,
                Options {
                    overwrite_tracked,
                    ..Default::default()
                },
            )
            .expect_err("unapproved changes prevent the whole checkout");
            assert!(err.is_validation(), "the refusal identifies a worktree conflict: {err}");
            assert_eq!(
                fs::read(workdir.join("a-delete"))?,
                b"old",
                "preflight finishes before any deletion"
            );
            assert!(current.entries() == before, "refusal leaves the staging index intact");
        }
    }
    Ok(())
}

#[test]
fn overwriting_ignored_obstructions_matches_git_read_tree() -> TestResult {
    for obstruction in ["file", "symlink", "directory", "parent"] {
        let mut snapshots = Vec::new();
        for use_git in [false, true] {
            let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
            let workdir = repo.workdir().expect("non-bare");
            if obstruction == "symlink" && !gix::fs::Capabilities::probe_dir(workdir).symlink {
                break;
            }
            let baseline = index(&repo, &[("a-delete", "old")])?;
            let mut target = index(
                &repo,
                if obstruction == "parent" {
                    &[("path/one", "new"), ("path/two", "new")]
                } else {
                    &[("path", "new")]
                },
            )?;
            gix::index::File::from_state(target.clone(), repo.index_path()).write(Default::default())?;
            let target_tree_id = gix_testtools::git(workdir, "write-tree")?;
            gix::index::File::from_state(baseline.clone(), repo.index_path()).write(Default::default())?;
            let baseline_tree_id = gix_testtools::git(workdir, "write-tree")?;
            gix_testtools::git(workdir, &format!("read-tree --reset -u {}", baseline_tree_id.trim()))?;
            fs::write(workdir.join(".gitignore"), "path\nignored-elsewhere\n")?;
            fs::write(workdir.join("ignored-elsewhere"), "keep ignored")?;
            fs::write(workdir.join("untracked"), "keep untracked")?;
            match obstruction {
                "file" | "parent" => fs::write(workdir.join("path"), "ignored")?,
                "symlink" => gix::fs::symlink::create("untracked".as_ref(), &workdir.join("path"))?,
                "directory" => {
                    fs::create_dir_all(workdir.join("path/nested"))?;
                    fs::create_dir_all(workdir.join("path/empty"))?;
                    fs::write(workdir.join("path/nested/file"), "ignored")?;
                }
                _ => unreachable!(),
            }
            if use_git {
                gix_testtools::git(
                    workdir,
                    &format!("read-tree -m -u {} {}", baseline_tree_id.trim(), target_tree_id.trim()),
                )?;
            } else {
                let mut current = baseline.clone();
                checkout(
                    &repo,
                    &baseline,
                    &mut target,
                    &mut current,
                    Options {
                        overwrite_ignored: true,
                        ..Default::default()
                    },
                )?;
                gix::index::File::from_state(current, repo.index_path()).write(Default::default())?;
            }
            snapshots.push(gix_testtools::repository::snapshot(workdir)?);
        }
        if let [actual, expected] = snapshots.as_slice() {
            assert_eq!(
                actual.index, expected.index,
                "staged entries match Git for ignored {obstruction}"
            );
            assert_eq!(
                actual.worktree, expected.worktree,
                "only obstructing ignored content is removed for {obstruction}"
            );
        }
    }
    Ok(())
}

#[test]
fn overwriting_ignored_files_preserves_untracked_content_and_repositories() -> TestResult {
    for obstruction in ["untracked", "precious", "non-bare", "bare"] {
        let (mut repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
        repo.config_snapshot_mut()
            .set_value(&gix::config::tree::Gitoxide::PARSE_PRECIOUS, "true")?;
        let workdir = repo.workdir().expect("non-bare");
        let baseline = index(&repo, &[("a-delete", "old")])?;
        let mut target = index(&repo, &[("path", "new")])?;
        let mut current = baseline.clone();
        fs::write(workdir.join("a-delete"), "old")?;
        fs::create_dir(workdir.join("path"))?;
        fs::write(workdir.join("path/ignored"), "ignored")?;
        fs::write(
            workdir.join(".gitignore"),
            match obstruction {
                "untracked" => "path/ignored\n",
                "precious" => "$path\n",
                _ => "path\n",
            },
        )?;
        match obstruction {
            "untracked" | "precious" => fs::write(workdir.join("path/keep"), "keep")?,
            "non-bare" | "bare" => {
                crate::init_repo_isolated(
                    workdir.join("path/repo"),
                    if obstruction == "bare" {
                        gix::create::Kind::Bare
                    } else {
                        gix::create::Kind::WithWorktree
                    },
                )?;
            }
            _ => unreachable!(),
        }
        let before = gix_testtools::repository::snapshot(workdir)?;
        checkout(
            &repo,
            &baseline,
            &mut target,
            &mut current,
            Options {
                overwrite_ignored: true,
                ..Default::default()
            },
        )
        .expect_err("untracked content and nested repositories prevent removing an obstruction");
        let after = gix_testtools::repository::snapshot(workdir)?;
        assert_eq!(
            after.worktree, before.worktree,
            "all worktree data survives the failed preflight for {obstruction}"
        );
        assert!(
            current.entries() == baseline.entries(),
            "staged entries survive the failed preflight"
        );
    }
    Ok(())
}

#[test]
fn force_restores_dirty_and_missing_tracked_files_even_without_a_tree_delta() -> TestResult {
    let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
    let workdir = repo.workdir().expect("non-bare");
    let baseline = index(&repo, &[("changed", "expected"), ("missing", "expected")])?;
    let mut target = baseline.clone();
    let mut current = baseline.clone();
    fs::write(workdir.join("changed"), b"dirty")?;
    fs::write(workdir.join("untracked"), b"keep")?;
    checkout(
        &repo,
        &baseline,
        &mut target,
        &mut current,
        Options {
            overwrite_tracked: true,
            ..Default::default()
        },
    )?;
    assert_eq!(
        fs::read(workdir.join("changed"))?,
        b"expected",
        "force restores a dirty tracked file"
    );
    assert_eq!(
        fs::read(workdir.join("missing"))?,
        b"expected",
        "force recreates a missing tracked file"
    );
    assert_eq!(
        fs::read(workdir.join("untracked"))?,
        b"keep",
        "force does not select untracked files"
    );
    fs::write(workdir.join("changed"), b"dirty again")?;
    let staged_id = repo.write_blob(b"staged only")?.detach();
    current
        .entry_mut_by_path_and_stage("changed".into(), Stage::Unconflicted)
        .expect("tracked")
        .id = staged_id;
    checkout(
        &repo,
        &baseline,
        &mut target,
        &mut current,
        Options {
            overwrite_tracked: true,
            ..Default::default()
        },
    )
    .expect_err("restoring the worktree does not implicitly discard staging");
    assert_eq!(
        current.entry_by_path("changed".into()).expect("tracked").id,
        staged_id,
        "refusal preserves staging even without a tree delta"
    );
    checkout(
        &repo,
        &baseline,
        &mut target,
        &mut current,
        Options {
            overwrite_tracked: true,
            overwrite_staged: true,
            ..Default::default()
        },
    )?;
    assert_eq!(
        current.entry_by_path("changed".into()).expect("tracked").id,
        target.entry_by_path("changed".into()).expect("tracked").id,
        "explicit staged overwrite restores unchanged-tree paths too"
    );
    Ok(())
}

#[test]
fn checkout_accepts_worktree_content_already_equal_to_the_target() -> TestResult {
    let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
    let baseline = index(&repo, &[("file", "old")])?;
    let mut target = index(&repo, &[("file", "new")])?;
    let mut current = baseline.clone();
    fs::write(repo.workdir().expect("non-bare").join("file"), b"new")?;
    checkout(&repo, &baseline, &mut target, &mut current, Options::default())?;
    assert_eq!(
        current.entries()[0].id,
        target.entries()[0].id,
        "a file already matching the target is safe to record"
    );
    Ok(())
}

#[test]
fn checkout_preserves_sparse_marks_while_updating_excluded_content() -> TestResult {
    let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
    let mut baseline = index(&repo, &[("excluded", "old")])?;
    baseline.entries_mut()[0].flags |= Flags::EXTENDED | Flags::SKIP_WORKTREE;
    let mut target = index(&repo, &[("excluded", "new")])?;
    let mut current = baseline.clone();
    checkout(&repo, &baseline, &mut target, &mut current, Options::default())?;
    assert!(
        !repo.workdir().expect("non-bare").join("excluded").exists(),
        "an excluded path remains absent"
    );
    assert_eq!(
        current.entries()[0].id,
        target.entries()[0].id,
        "the excluded index entry receives the target content"
    );
    assert!(
        current.entries()[0].flags.contains(Flags::SKIP_WORKTREE),
        "updating content preserves sparse exclusion"
    );
    Ok(())
}

#[test]
fn checkout_handles_file_directory_transitions_without_losing_unrelated_files() -> TestResult {
    let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
    let workdir = repo.workdir().expect("non-bare");
    let baseline = index(&repo, &[("directory/old", "old"), ("file", "old")])?;
    let mut target = index(&repo, &[("directory", "new"), ("file/new", "new")])?;
    let mut current = baseline.clone();
    fs::create_dir(workdir.join("directory"))?;
    fs::write(workdir.join("directory/old"), b"old")?;
    fs::write(workdir.join("file"), b"old")?;
    fs::write(workdir.join("untracked"), b"keep")?;
    checkout(&repo, &baseline, &mut target, &mut current, Options::default())?;
    assert_eq!(
        fs::read(workdir.join("directory"))?,
        b"new",
        "tracked child removals permit replacing a directory"
    );
    assert_eq!(
        fs::read(workdir.join("file/new"))?,
        b"new",
        "a removed tracked file can become a directory"
    );
    assert_eq!(
        fs::read(workdir.join("untracked"))?,
        b"keep",
        "unrelated untracked files survive transitions"
    );
    Ok(())
}

#[test]
fn staged_parent_and_descendant_entries_cannot_be_lost_in_a_type_change() -> TestResult {
    for (staged, stage) in [
        ("path", Stage::Unconflicted),
        ("path/staged", Stage::Unconflicted),
        ("path", Stage::Ours),
        ("path/staged", Stage::Theirs),
    ] {
        let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
        let workdir = repo.workdir().expect("non-bare");
        let baseline = index(&repo, &[("a-delete", "old")])?;
        let target_path = if staged == "path" { "path/new" } else { "path" };
        let mut target = index(&repo, &[(target_path, "new")])?;
        let mut current = index(&repo, &[("a-delete", "old"), (staged, "staged")])?;
        current.entries_mut()[1].flags = Flags::from_stage(stage);
        fs::write(workdir.join("a-delete"), b"old")?;
        let before = current.entries().to_vec();
        checkout(&repo, &baseline, &mut target, &mut current, Options::default())
            .expect_err("an unrelated staged file cannot be replaced by a directory or its parent file");
        assert_eq!(
            fs::read(workdir.join("a-delete"))?,
            b"old",
            "index shape is checked before worktree mutation"
        );
        assert!(current.entries() == before, "staged data remains unchanged");
    }
    Ok(())
}

#[test]
fn symlinked_parents_are_protected_during_forced_and_deleted_transitions() -> TestResult {
    for delete in [false, true] {
        let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
        let workdir = repo.workdir().expect("non-bare");
        if !gix::fs::Capabilities::probe_dir(workdir).symlink {
            return Ok(());
        }
        let outside = gix_testtools::tempfile::tempdir()?;
        fs::write(outside.path().join("file"), b"outside")?;
        gix::fs::symlink::create(outside.path(), &workdir.join("parent"))?;
        let baseline = index(&repo, &[("parent/file", "tracked")])?;
        let mut target = if delete { index(&repo, &[])? } else { baseline.clone() };
        let mut current = baseline.clone();
        let result = checkout(
            &repo,
            &baseline,
            &mut target,
            &mut current,
            Options {
                overwrite_tracked: true,
                ..Default::default()
            },
        );
        if delete {
            result?;
            assert!(
                current.entries().is_empty(),
                "the missing tracked file can be removed from the index"
            );
        } else {
            result.expect_err("force must report the untracked symlink obstruction");
        }
        assert_eq!(
            fs::read(outside.path().join("file"))?,
            b"outside",
            "no operation reaches through the symlink"
        );
        assert!(
            workdir.join("parent").symlink_metadata()?.file_type().is_symlink(),
            "the untracked symlink remains intact"
        );
    }
    Ok(())
}

#[test]
fn target_case_aliases_cannot_overwrite_an_unchanged_tracked_path() -> TestResult {
    let (mut repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
    repo.config_snapshot_mut()
        .set_value(&gix::config::tree::Core::IGNORE_CASE, "true")?;
    let workdir = repo.workdir().expect("non-bare");
    let baseline = index(&repo, &[("A", "old"), ("a-delete", "old")])?;
    let mut target = index(&repo, &[("A", "old"), ("a", "new")])?;
    let mut current = baseline.clone();
    fs::write(workdir.join("A"), b"old")?;
    fs::write(workdir.join("a-delete"), b"old")?;
    checkout(&repo, &baseline, &mut target, &mut current, Options::default())
        .expect_err("both target entries cannot share a filesystem path");
    assert_eq!(
        fs::read(workdir.join("a-delete"))?,
        b"old",
        "case collisions are detected before mutation"
    );
    assert_eq!(
        fs::read(workdir.join("A"))?,
        b"old",
        "the unchanged tracked file stays intact"
    );
    Ok(())
}

#[test]
fn checkout_turns_a_tracked_file_into_an_uninitialized_submodule_directory() -> TestResult {
    let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
    let workdir = repo.workdir().expect("non-bare");
    let baseline = index(&repo, &[("a-delete", "old"), ("sub", "old")])?;
    let mut target = index(&repo, &[("sub", "gitlink")])?;
    target.entries_mut()[0].mode = Mode::COMMIT;
    let mut current = baseline.clone();
    fs::write(workdir.join("a-delete"), b"old")?;
    fs::write(workdir.join("sub"), b"old")?;
    checkout(&repo, &baseline, &mut target, &mut current, Options::default())?;
    assert!(
        workdir.join("sub").is_dir(),
        "a gitlink is materialized as an empty directory without recursively checking it out"
    );
    assert_eq!(
        current.entries()[0].mode,
        Mode::COMMIT,
        "the staging index records the gitlink"
    );
    Ok(())
}

#[test]
fn checkout_preserves_populated_submodules() -> TestResult {
    for nested_repository in [false, true] {
        for transition in ["delete", "update", "file"] {
            let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
            let workdir = repo.workdir().expect("non-bare");
            let mut baseline = index(&repo, &[("path", "old")])?;
            baseline.entries_mut()[0].mode = Mode::COMMIT;
            let mut target = index(
                &repo,
                if transition == "delete" {
                    &[]
                } else {
                    &[("path", "new")]
                },
            )?;
            if transition == "update" {
                target.entries_mut()[0].mode = Mode::COMMIT;
            }
            let mut current = baseline.clone();
            fs::create_dir(workdir.join("path"))?;
            if nested_repository {
                crate::init_repo_isolated(workdir.join("path"), gix::create::Kind::WithWorktree)?;
            }
            fs::write(workdir.join("path/keep"), "contents")?;
            let result = checkout(&repo, &baseline, &mut target, &mut current, Options::default());
            if transition == "file" {
                result.expect_err("a file cannot replace a populated submodule directory");
                assert!(
                    current.entries() == baseline.entries(),
                    "refusal preserves staged entries"
                );
            } else {
                result?;
            }
            assert_eq!(
                fs::read(workdir.join("path/keep"))?,
                b"contents",
                "submodule contents remain untouched"
            );
            assert_eq!(
                workdir.join("path/.git").exists(),
                nested_repository,
                "submodule metadata remains intact"
            );
        }
    }
    Ok(())
}

#[test]
fn checkout_anchors_relative_repository_paths_before_io() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = gix_testtools::tempfile::tempdir()?;
    for directory in ["one", "two"] {
        let base = root.path().join(directory);
        fs::create_dir(&base)?;
        let repo = crate::init_repo_isolated(base.join("repo"), gix::create::Kind::WithWorktree)?.to_thread_local();
        repo.write_blob(b"old")?;
        repo.write_blob(b"new")?;
        fs::write(base.join("repo/file"), b"old")?;
    }
    let _cwd = gix_testtools::set_current_dir(root.path().join("one"))?;
    let repo = gix::open_opts("repo", crate::restricted())?;
    let baseline = index(&repo, &[("file", "old")])?;
    let mut target = index(&repo, &[("file", "new")])?;
    let mut current = baseline.clone();
    let _elsewhere = gix_testtools::set_current_dir(root.path().join("two"))?;
    checkout(&repo, &baseline, &mut target, &mut current, Options::default())?;
    assert_eq!(
        fs::read(root.path().join("one/repo/file"))?,
        b"new",
        "checkout remains rooted where the repository was opened"
    );
    assert_eq!(
        fs::read(root.path().join("two/repo/file"))?,
        b"old",
        "a changed process directory cannot redirect worktree writes"
    );
    Ok(())
}

#[test]
fn checkout_handles_symlink_transitions_including_content_already_at_the_target() -> TestResult {
    for target_is_symlink in [false, true] {
        for (already_at_target, dirty, emulate_symlinks) in [
            (false, false, false),
            (true, false, false),
            (true, true, false),
            (false, false, true),
            (true, false, true),
            (true, true, true),
        ] {
            let (mut repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
            if emulate_symlinks {
                repo.config_snapshot_mut()
                    .set_value(&gix::config::tree::Core::SYMLINKS, "false")?;
            }
            let workdir = repo.workdir().expect("non-bare");
            if !emulate_symlinks && !gix::fs::Capabilities::probe_dir(workdir).symlink {
                continue;
            }
            let mut baseline = index(&repo, &[("path", "old")])?;
            let mut target = index(&repo, &[("path", "new")])?;
            if target_is_symlink {
                target.entries_mut()[0].mode = Mode::SYMLINK;
            } else {
                baseline.entries_mut()[0].mode = Mode::SYMLINK;
            }
            let mut current = baseline.clone();
            let contents = if dirty {
                "dirty"
            } else if already_at_target {
                "new"
            } else {
                "old"
            };
            if target_is_symlink == already_at_target && !emulate_symlinks {
                gix::fs::symlink::create(contents.as_ref(), &workdir.join("path"))?;
            } else {
                fs::write(workdir.join("path"), contents)?;
            }
            let result = checkout(&repo, &baseline, &mut target, &mut current, Options::default());
            if dirty {
                result.expect_err("matching the target type alone does not permit overwriting dirty content");
                continue;
            }
            result?;
            if target_is_symlink && !emulate_symlinks {
                assert_eq!(fs::read_link(workdir.join("path"))?, std::path::Path::new("new"));
            } else {
                assert_eq!(fs::read(workdir.join("path"))?, b"new");
            }
            assert_eq!(current.entries()[0].mode, target.entries()[0].mode);
        }
    }
    Ok(())
}

#[test]
fn checkout_can_replace_nested_empty_directories() -> TestResult {
    let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
    let baseline = index(&repo, &[])?;
    let mut target = index(&repo, &[("path", "new")])?;
    let mut current = baseline.clone();
    let workdir = repo.workdir().expect("non-bare");
    fs::create_dir_all(workdir.join("path/empty/nested"))?;
    fs::create_dir_all(workdir.join("path/other"))?;
    checkout(&repo, &baseline, &mut target, &mut current, Options::default())?;
    assert_eq!(
        fs::read(workdir.join("path"))?,
        b"new",
        "empty directories contain no data to preserve"
    );
    Ok(())
}

#[test]
fn emulated_symlink_type_changes_compare_target_content_with_its_own_filters() -> TestResult {
    for target_is_symlink in [false, true] {
        let (mut repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
        repo.config_snapshot_mut()
            .set_value(&gix::config::tree::Core::SYMLINKS, "false")?;
        let attributes = "path text eol=crlf\n";
        let mut baseline = index(&repo, &[(".gitattributes", attributes), ("path", "old\n")])?;
        let contents = if target_is_symlink { "new\r\n" } else { "new\n" };
        let mut target = index(&repo, &[(".gitattributes", attributes), ("path", contents)])?;
        let symlink_index = if target_is_symlink { &mut target } else { &mut baseline };
        symlink_index
            .entry_mut_by_path_and_stage("path".into(), Stage::Unconflicted)
            .expect("tracked")
            .mode = Mode::SYMLINK;
        let mut current = baseline.clone();
        let workdir = repo.workdir().expect("non-bare");
        fs::write(workdir.join(".gitattributes"), attributes)?;
        fs::write(workdir.join("path"), b"new\r\n")?;
        checkout(&repo, &baseline, &mut target, &mut current, Options::default())?;
        assert_eq!(
            fs::read(workdir.join("path"))?,
            b"new\r\n",
            "matching target content survives its own conversion rules"
        );
        assert_eq!(
            current.entry_by_path("path".into()).expect("tracked").id,
            repo.write_blob(contents.as_bytes())?.detach(),
            "emulated symlinks keep raw bytes while regular files use cleaned bytes"
        );
    }
    Ok(())
}

#[test]
fn checkout_rejects_unmaterialized_inputs_before_mutation_and_keeps_unrelated_conflicts() -> TestResult {
    for invalid in [
        "baseline-conflict",
        "target-conflict",
        "intent-to-add",
        "mode",
        "sparse",
        "touched-conflict",
        "unrelated-conflict",
    ] {
        let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
        let workdir = repo.workdir().expect("non-bare");
        let mut baseline = index(&repo, &[("a-delete", "old"), ("path", "old")])?;
        let mut target = index(&repo, &[("path", "new")])?;
        let mut current = baseline.clone();
        match invalid {
            "baseline-conflict" => baseline.entries_mut()[0].flags = Flags::from_stage(Stage::Ours),
            "target-conflict" => target.entries_mut()[0].flags = Flags::from_stage(Stage::Theirs),
            "intent-to-add" => target.entries_mut()[0].flags = Flags::INTENT_TO_ADD | Flags::EXTENDED,
            "mode" => target.entries_mut()[0].mode = Mode::empty(),
            "sparse" => {
                target.entries_mut()[0].mode = Mode::DIR;
                target.entries_mut()[0].flags = Flags::SKIP_WORKTREE | Flags::EXTENDED;
                let path = repo.git_dir().join("sparse-index");
                gix::index::File::from_state(target, path.clone()).write(Default::default())?;
                target = (*gix::index::File::at(path, repo.object_hash(), false, Default::default())?).clone();
                assert!(target.is_sparse(), "decoding recognizes the compressed sparse index");
            }
            "touched-conflict" => current.entries_mut()[1].flags = Flags::from_stage(Stage::Ours),
            "unrelated-conflict" => {
                current.dangerously_push_entry(
                    Default::default(),
                    target.entries()[0].id,
                    Flags::from_stage(Stage::Ours),
                    Mode::FILE,
                    "unrelated".into(),
                );
                current.sort_entries();
            }
            _ => unreachable!(),
        }
        fs::write(workdir.join("a-delete"), b"old")?;
        fs::write(workdir.join("path"), b"old")?;
        let before = current.entries().to_vec();
        let result = checkout(&repo, &baseline, &mut target, &mut current, Options::default());
        if invalid == "unrelated-conflict" {
            result?;
            assert!(
                current
                    .entry_by_path_and_stage("unrelated".into(), Stage::Ours)
                    .is_some(),
                "unrelated conflicts stay staged"
            );
        } else {
            result.expect_err("invalid materialization or touched conflicts must be rejected");
            assert_eq!(
                fs::read(workdir.join("a-delete"))?,
                b"old",
                "validation precedes all deletions ({invalid})"
            );
            assert!(
                current.entries() == before,
                "validation failure leaves the staging index unchanged"
            );
        }
    }
    Ok(())
}

#[test]
fn checkout_keeps_the_staging_index_on_cancellation_and_write_failure() -> TestResult {
    for cancel in [false, true] {
        let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
        let baseline = index(&repo, &[("a-delete", "old")])?;
        let mut target = index(&repo, &[("missing-object", "new")])?;
        target.entries_mut()[0].id = repo.object_hash().null();
        let mut current = baseline.clone();
        let before = current.entries().to_vec();
        let workdir = repo.workdir().expect("non-bare");
        fs::write(workdir.join("a-delete"), b"old")?;
        repo.checkout_index_with_baseline(
            &baseline,
            &mut target,
            &mut current,
            progress::Discard,
            &AtomicBool::new(cancel),
            Options::default(),
        )
        .expect_err("cancelled checkout and missing objects fail");
        assert!(
            current.entries() == before,
            "even partial checkout failure does not replace the staging index"
        );
        if cancel {
            assert_eq!(
                fs::read(workdir.join("a-delete"))?,
                b"old",
                "early cancellation leaves the worktree untouched"
            );
        }
    }
    Ok(())
}

#[test]
fn clean_transitions_match_git_read_tree() -> TestResult {
    for case in [
        "files",
        "file-directory",
        "case",
        "case-symlink",
        "symlink",
        "submodule",
        "directory-submodule",
        "submodule-delete",
        "submodule-file",
    ] {
        let mut snapshots = Vec::new();
        for use_git in [false, true] {
            let (mut repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
            let capabilities = gix::fs::Capabilities::probe_dir(repo.workdir().expect("non-bare"));
            if case.ends_with("symlink") && !capabilities.symlink {
                break;
            }
            if case.starts_with("case") {
                repo.config_snapshot_mut()
                    .set_value(&gix::config::tree::Core::IGNORE_CASE, "true")?;
                gix_testtools::git(repo.workdir().expect("non-bare"), "config core.ignoreCase true")?;
            }
            let workdir = repo.workdir().expect("non-bare");
            let (from, to): (&[_], &[_]) = match case {
                "files" => (
                    &[("change", "old"), ("delete", "old"), ("same", "same")],
                    &[("added", "new"), ("change", "new"), ("same", "same")],
                ),
                "file-directory" => (
                    &[("directory/old", "old"), ("file", "old")],
                    &[("directory", "new"), ("file/new", "new")],
                ),
                "case" | "case-symlink" => (&[("FILE", "old")], &[("file", "new")]),
                "symlink" | "submodule" | "submodule-file" => (&[("path", "old")], &[("path", "new")]),
                "directory-submodule" => (&[("path/file", "old")], &[("path", "new")]),
                "submodule-delete" => (&[("path", "old")], &[]),
                _ => unreachable!(),
            };
            let mut baseline = index(&repo, from)?;
            let mut target = index(&repo, to)?;
            if matches!(case, "submodule-delete" | "submodule-file") {
                baseline.entries_mut()[0].mode = Mode::COMMIT;
            }
            if case.ends_with("symlink") {
                target.entries_mut()[0].mode = Mode::SYMLINK;
            } else if matches!(case, "submodule" | "directory-submodule") {
                target.entries_mut()[0].mode = Mode::COMMIT;
            }
            gix::index::File::from_state(target.clone(), repo.index_path()).write(Default::default())?;
            let target_tree_id = gix_testtools::git(workdir, "write-tree")?;
            gix::index::File::from_state(baseline.clone(), repo.index_path()).write(Default::default())?;
            let baseline_tree_id = gix_testtools::git(workdir, "write-tree")?;
            gix_testtools::git(workdir, &format!("read-tree --reset -u {}", baseline_tree_id.trim()))?;
            fs::write(workdir.join("untracked"), b"keep")?;
            if use_git {
                gix_testtools::git(
                    workdir,
                    &format!("read-tree -m -u {} {}", baseline_tree_id.trim(), target_tree_id.trim()),
                )?;
            } else {
                let mut current = baseline.clone();
                checkout(&repo, &baseline, &mut target, &mut current, Options::default())?;
                gix::index::File::from_state(current, repo.index_path()).write(Default::default())?;
            }
            snapshots.push(gix_testtools::repository::snapshot(workdir)?);
        }
        if let [actual, expected] = snapshots.as_slice() {
            assert_eq!(actual.index, expected.index, "staged entries match Git for {case}");
            assert_eq!(
                actual.worktree, expected.worktree,
                "worktree contents and types match Git for {case}"
            );
        }
    }
    Ok(())
}

#[test]
fn checkout_checks_cleaned_content_and_writes_filtered_content() -> TestResult {
    let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
    let baseline = index(
        &repo,
        &[(".gitattributes", "*.txt text eol=crlf\n"), ("file.txt", "old\n")],
    )?;
    let mut target = index(
        &repo,
        &[(".gitattributes", "*.txt text eol=crlf\n"), ("file.txt", "new\n")],
    )?;
    let mut current = baseline.clone();
    let workdir = repo.workdir().expect("non-bare");
    fs::write(workdir.join(".gitattributes"), b"*.txt text eol=crlf\n")?;
    fs::write(workdir.join("file.txt"), b"old\r\n")?;
    checkout(&repo, &baseline, &mut target, &mut current, Options::default())?;
    assert_eq!(
        fs::read(workdir.join("file.txt"))?,
        b"new\r\n",
        "checkout applies worktree filters after checking cleaned content"
    );
    assert_eq!(
        current.entry_by_path("file.txt".into()).expect("still tracked").id,
        repo.write_blob(b"new\n")?.detach(),
        "the staged object keeps canonical line endings"
    );
    Ok(())
}

fn checkout(
    repo: &Repository,
    baseline: &State,
    target: &mut State,
    index: &mut State,
    options: Options,
) -> gix::Result<gix::worktree::checkout::Outcome> {
    repo.checkout_index_with_baseline(
        baseline,
        target,
        index,
        progress::Discard,
        &AtomicBool::default(),
        options,
    )
}

fn index(repo: &Repository, entries: &[(&str, &str)]) -> gix::Result<State> {
    let mut index = State::new(repo.object_hash());
    for (path, contents) in entries {
        index.dangerously_push_entry(
            Default::default(),
            repo.write_blob(contents.as_bytes())?.detach(),
            Flags::empty(),
            Mode::FILE,
            (*path).into(),
        );
    }
    index.sort_entries();
    Ok(index)
}
