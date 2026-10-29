use std::{fs, sync::atomic::AtomicBool};

use gix::{
    index::entry::{Flags, Mode, Stage},
    progress,
};
use gix_testtools::{TestResult, git};

#[test]
fn hard_reset_matches_git_for_dirty_staged_and_unmerged_indexes() -> TestResult {
    for case in [
        "clean",
        "dirty-staged",
        "unmerged",
        "unmerged-df",
        "missing-index",
        "obsolete-directory",
        "untracked-obstruction",
        "nested-repository",
        "gitlink-empty",
        "gitlink-populated",
        "gitlink-file",
    ] {
        let mut states = Vec::new();
        for use_git in [true, false] {
            let (repo, _tmp) = crate::util::repo_rw("make_reset_hard.sh")?;
            let root = repo.workdir().expect("non-bare");
            fs::write(root.join("untracked"), "preserved\n")?;
            match case {
                "dirty-staged" => {
                    fs::write(root.join("changed"), "staged\n")?;
                    fs::write(root.join("staged-add"), "staged addition\n")?;
                    git(root, "add changed staged-add")?;
                    git(root, "rm --cached recreated")?;
                    fs::write(root.join("changed"), "unstaged\n")?;
                    fs::write(root.join("same"), "dirty unchanged path\n")?;
                }
                "unmerged" | "unmerged-df" => {
                    let mut index = repo.open_index()?;
                    index.remove_entries(|_, path, _| path == "changed");
                    for (stage, content) in [
                        (Stage::Base, "base\n"),
                        (Stage::Ours, "ours\n"),
                        (Stage::Theirs, "theirs\n"),
                    ] {
                        index.dangerously_push_entry(
                            Default::default(),
                            repo.write_blob(content)?.detach(),
                            Flags::from_stage(stage),
                            Mode::FILE,
                            "changed".into(),
                        );
                    }
                    if case == "unmerged-df" {
                        index.remove_entries(|_, path, _| {
                            path.starts_with(b"file-to-dir") || path.starts_with(b"dir-to-file")
                        });
                        for (path, stage) in [
                            ("file-to-dir", Stage::Ours),
                            ("file-to-dir/child", Stage::Theirs),
                            ("dir-to-file", Stage::Ours),
                            ("dir-to-file/child", Stage::Theirs),
                        ] {
                            index.dangerously_push_entry(
                                Default::default(),
                                repo.write_blob("conflict\n")?.detach(),
                                Flags::from_stage(stage),
                                Mode::FILE,
                                path.into(),
                            );
                        }
                    }
                    index.sort_entries();
                    index.write(Default::default())?;
                    fs::write(root.join("changed"), "conflict markers\n")?;
                }
                "missing-index" => fs::remove_file(repo.index_path())?,
                "obsolete-directory" => {
                    fs::remove_file(root.join("old-only"))?;
                    fs::create_dir(root.join("old-only"))?;
                    fs::write(root.join("old-only/untracked"), "preserved\n")?;
                    fs::write(root.join("obsolete-dir/untracked"), "preserved\n")?;
                }
                "untracked-obstruction" => {
                    fs::create_dir(root.join("target-only"))?;
                    fs::write(root.join("target-only/untracked"), "replaced\n")?;
                }
                "nested-repository" => {
                    git(root, "init -q target-only")?;
                    fs::write(root.join("target-only/untracked"), "replaced\n")?;
                }
                "gitlink-empty" | "gitlink-populated" | "gitlink-file" => {
                    let mut index = repo.open_index()?;
                    index.dangerously_push_entry(
                        Default::default(),
                        repo.head_id()?.detach(),
                        Flags::empty(),
                        Mode::COMMIT,
                        "submodule".into(),
                    );
                    index.sort_entries();
                    index.write(Default::default())?;
                    if case == "gitlink-file" {
                        fs::write(root.join("submodule"), "preserved\n")?;
                    } else {
                        fs::create_dir(root.join("submodule"))?;
                        if case == "gitlink-populated" {
                            fs::write(root.join("submodule/untracked"), "preserved\n")?;
                        }
                    }
                }
                "clean" => {}
                _ => unreachable!(),
            }
            let target_commit_id = repo.rev_parse_single("target")?.detach();
            let previous_commit_id = repo.head_id()?.detach();
            if use_git {
                git(root, &format!("reset --hard {target_commit_id}"))?;
            } else {
                repo.reset_hard(target_commit_id, progress::Discard, &AtomicBool::new(false))?;
            }
            assert_eq!(repo.head_id()?, target_commit_id, "{case}: HEAD moves to the target");
            assert_eq!(
                repo.find_reference("ORIG_HEAD")?.target().id(),
                previous_commit_id,
                "{case}: ORIG_HEAD records the former HEAD"
            );
            states.push(gix_testtools::repository::snapshot(root)?);
        }
        assert_eq!(states[0].index, states[1].index, "{case}: index agrees with Git");
        assert_eq!(
            states[0].worktree, states[1].worktree,
            "{case}: worktree agrees with Git"
        );
        assert_eq!(
            states[0].head, states[1].head,
            "{case}: HEAD attachment agrees with Git"
        );
    }
    Ok(())
}

#[test]
fn hard_reset_refuses_locked_or_sparse_indexes_before_changing_anything() -> TestResult {
    for case in [
        "lock",
        "skip-worktree",
        "sparse-config",
        "compressed-index",
        "cancelled",
        "not-commit",
    ] {
        let (repo, _tmp) = crate::util::repo_rw("make_reset_hard.sh")?;
        let root = repo.workdir().expect("non-bare");
        fs::write(root.join("changed"), "dirty\n")?;
        let mut repo = repo;
        match case {
            "lock" => fs::write(repo.index_path().with_extension("lock"), "held\n")?,
            "skip-worktree" => {
                let mut index = repo.open_index()?;
                index
                    .entry_mut_by_path_and_stage("same".into(), Stage::Unconflicted)
                    .expect("tracked")
                    .flags |= Flags::EXTENDED | Flags::SKIP_WORKTREE;
                index.write(Default::default())?;
            }
            "sparse-config" => {
                repo.config_snapshot_mut()
                    .set_raw_value("core.sparseCheckout", "true")?;
            }
            "compressed-index" => {
                git(
                    repo.workdir().expect("non-bare"),
                    "sparse-checkout set --cone --sparse-index obsolete-dir",
                )?;
                assert!(
                    repo.open_index()?.is_sparse(),
                    "the fixture contains compressed sparse directories"
                );
            }
            "cancelled" | "not-commit" => {}
            _ => unreachable!(),
        }
        let root = repo.workdir().expect("non-bare");
        let before = gix_testtools::repository::snapshot(root)?;
        let old_index = fs::read(repo.index_path())?;
        let target_id = if case == "not-commit" {
            repo.head_tree_id()?.detach()
        } else {
            repo.rev_parse_single("target")?.detach()
        };
        assert!(
            repo.reset_hard(target_id, progress::Discard, &AtomicBool::new(case == "cancelled"))
                .is_err(),
            "{case} must fail before mutation"
        );
        let after = gix_testtools::repository::snapshot(root)?;
        assert_eq!(before, after, "{case}: repository and worktree stay unchanged");
        assert_eq!(
            fs::read(repo.index_path())?,
            old_index,
            "{case}: original index bytes survive"
        );
    }
    Ok(())
}

#[test]
fn hard_reset_reference_updates_and_reflogs_match_git() -> TestResult {
    for case in [
        "attached",
        "detached",
        "unborn",
        "symbolic-orig",
        "orig-aliases-head",
        "dangling-orig",
    ] {
        let mut states = Vec::new();
        let mut logs = Vec::new();
        for use_git in [true, false] {
            let (repo, _tmp) = crate::util::repo_rw("make_reset_hard.sh")?;
            let root = repo.workdir().expect("non-bare");
            git(root, "update-ref --create-reflog ORIG_HEAD target")?;
            match case {
                "detached" => {
                    git(root, "checkout --detach")?;
                }
                "unborn" => {
                    git(root, "symbolic-ref HEAD refs/heads/unborn")?;
                }
                "symbolic-orig" => {
                    git(root, "update-ref refs/heads/original target")?;
                    git(root, "symbolic-ref ORIG_HEAD refs/heads/original")?;
                }
                "dangling-orig" => {
                    git(root, "symbolic-ref ORIG_HEAD refs/heads/missing-original")?;
                }
                "orig-aliases-head" => {
                    git(root, "symbolic-ref ORIG_HEAD refs/heads/main")?;
                }
                "attached" => {}
                _ => unreachable!(),
            }
            let target_commit_id = repo.rev_parse_single("target")?.detach();
            if use_git {
                git(root, &format!("reset --hard {target_commit_id}"))?;
            } else {
                repo.reset_hard(target_commit_id, progress::Discard, &AtomicBool::new(false))?;
            }
            states.push(gix_testtools::repository::snapshot(root)?);
            let mut transitions = Vec::new();
            for name in [
                "HEAD",
                "ORIG_HEAD",
                "refs/heads/main",
                "refs/heads/unborn",
                "refs/heads/original",
            ] {
                let path = repo.git_dir().join("logs").join(name);
                if let Ok(data) = fs::read(path) {
                    let last = data
                        .split(|byte| *byte == b'\n')
                        .rfind(|line| !line.is_empty())
                        .expect("existing log is nonempty");
                    let line = gix::refs::file::log::LineRef::from_bytes(last)?;
                    transitions.push((name, line.previous_oid(), line.new_oid(), line.message.to_owned()));
                }
            }
            logs.push(transitions);
        }
        assert_eq!(
            states[0].head, states[1].head,
            "{case}: HEAD attachment agrees with Git"
        );
        assert_eq!(
            states[0].references, states[1].references,
            "{case}: referents agree with Git"
        );
        assert_eq!(logs[0], logs[1], "{case}: reflog transitions agree with Git");
    }
    Ok(())
}

#[test]
fn hard_reset_clears_only_completed_branch_state_like_git() -> TestResult {
    for lines in [0, 1, 2] {
        for use_git in [true, false] {
            let (repo, _tmp) = crate::util::repo_rw("make_reset_hard.sh")?;
            let previous_commit_id = repo.head_id()?;
            let admin = repo.git_dir();
            for name in [
                "MERGE_HEAD",
                "CHERRY_PICK_HEAD",
                "REVERT_HEAD",
                "AUTO_MERGE",
                "REBASE_HEAD",
            ] {
                fs::write(admin.join(name), format!("{previous_commit_id}\n"))?;
            }
            for name in ["MERGE_MSG", "MERGE_MODE", "MERGE_RR", "SQUASH_MSG", "BISECT_LOG"] {
                fs::write(admin.join(name), "operation metadata\n")?;
            }
            for name in ["sequencer", "rebase-merge", "rebase-apply"] {
                fs::create_dir_all(admin.join(name))?;
                fs::write(admin.join(name).join("keep"), "recovery metadata\n")?;
            }
            if lines != 0 {
                fs::write(
                    admin.join("sequencer/todo"),
                    format!("pick {previous_commit_id} pending\n").repeat(lines),
                )?;
            }
            let target_commit_id = repo.rev_parse_single("target")?.detach();
            if use_git {
                git(
                    repo.workdir().expect("non-bare"),
                    &format!("reset --hard {target_commit_id}"),
                )?;
            } else {
                repo.reset_hard(target_commit_id, progress::Discard, &AtomicBool::new(false))?;
            }
            for name in [
                "MERGE_HEAD",
                "CHERRY_PICK_HEAD",
                "REVERT_HEAD",
                "AUTO_MERGE",
                "MERGE_MSG",
                "MERGE_MODE",
                "MERGE_RR",
                "SQUASH_MSG",
            ] {
                assert!(
                    !admin.join(name).exists(),
                    "Git={use_git}, todo lines={lines}: {name} is cleared"
                );
            }
            for name in [
                "REBASE_HEAD",
                "BISECT_LOG",
                "rebase-merge/keep",
                "rebase-apply/keep",
                "ORIG_HEAD",
            ] {
                assert!(
                    admin.join(name).exists(),
                    "Git={use_git}, todo lines={lines}: {name} remains"
                );
            }
            assert_eq!(
                admin.join("sequencer").exists(),
                lines != 1,
                "Git={use_git}: only a finished sequence is cleared"
            );
        }
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn hard_reset_handles_symlink_parents_and_index_like_git() -> TestResult {
    use std::os::unix::fs::symlink;

    for case in ["target-parent", "obsolete-parent", "index", "index-locked"] {
        let mut states = Vec::new();
        for use_git in [true, false] {
            let (repo, _tmp) = crate::util::repo_rw("make_reset_hard.sh")?;
            let root = repo.workdir().expect("non-bare");
            let outside = gix_testtools::tempfile::tempdir()?;
            fs::write(outside.path().join("child"), "outside child\n")?;
            fs::write(outside.path().join("leaf"), "outside leaf\n")?;
            match case {
                "target-parent" => {
                    fs::remove_file(root.join("file-to-dir"))?;
                    symlink(outside.path(), root.join("file-to-dir"))?;
                }
                "obsolete-parent" => {
                    fs::remove_dir_all(root.join("obsolete-dir"))?;
                    symlink(outside.path(), root.join("obsolete-dir"))?;
                }
                "index" | "index-locked" => {
                    fs::rename(repo.index_path(), repo.git_dir().join("index-real"))?;
                    symlink("index-real", repo.index_path())?;
                    if case == "index-locked" {
                        fs::write(repo.git_dir().join("index-real.lock"), "held\n")?;
                    }
                }
                _ => unreachable!(),
            }
            let old_index = fs::read(repo.index_path())?;
            let target_commit_id = repo.rev_parse_single("target")?.detach();
            let success = if use_git {
                gix_testtools::git_command(root)
                    .args(["reset", "--hard", &target_commit_id.to_string()])
                    .output()?
                    .status
                    .success()
            } else {
                repo.reset_hard(target_commit_id, progress::Discard, &AtomicBool::new(false))
                    .is_ok()
            };
            assert_eq!(
                success,
                case != "index-locked",
                "Git={use_git}, {case}: only the lock blocks reset"
            );
            assert_eq!(
                fs::read(outside.path().join("child"))?,
                b"outside child\n",
                "{case}: no writes through symlink parents"
            );
            assert_eq!(
                fs::read(outside.path().join("leaf"))?,
                b"outside leaf\n",
                "{case}: no removal through symlink parents"
            );
            if case.starts_with("index") {
                assert_eq!(
                    fs::read_link(repo.index_path())?,
                    std::path::Path::new("index-real"),
                    "Git={use_git}: reset preserves the index symlink"
                );
            }
            if case == "index-locked" {
                assert_eq!(
                    fs::read(repo.index_path())?,
                    old_index,
                    "Git={use_git}: locking the referent prevents index changes"
                );
            }
            if case == "obsolete-parent" {
                // The preserved link points at a different fixture directory in each run.
                assert!(root.join("obsolete-dir").is_symlink(), "the unrelated symlink survives");
                fs::remove_file(root.join("obsolete-dir"))?;
            }
            states.push(gix_testtools::repository::snapshot(root)?);
        }
        assert_eq!(states[0].index, states[1].index, "{case}: index agrees with Git");
        assert_eq!(
            states[0].worktree, states[1].worktree,
            "{case}: worktree agrees with Git"
        );
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn hard_reset_follows_filesystem_symbolic_references_like_git() -> TestResult {
    use std::os::unix::fs::symlink;

    for (name, target) in [
        ("HEAD", "refs/heads/main"),
        ("HEAD", "refs/heads/unborn"),
        ("ORIG_HEAD", "refs/heads/original"),
    ] {
        let mut states = Vec::new();
        for use_git in [true, false] {
            let (repo, _tmp) = crate::util::repo_rw("make_reset_hard.sh")?;
            git(
                repo.workdir().expect("non-bare"),
                "update-ref refs/heads/original target",
            )?;
            if name == "HEAD" {
                fs::remove_file(repo.git_dir().join(name))?;
            }
            symlink(target, repo.git_dir().join(name))?;
            let target_commit_id = repo.rev_parse_single("target")?.detach();
            if use_git {
                git(
                    repo.workdir().expect("non-bare"),
                    &format!("reset --hard {target_commit_id}"),
                )?;
            } else {
                repo.reset_hard(target_commit_id, progress::Discard, &AtomicBool::new(false))?;
            }
            assert_eq!(
                fs::read_link(repo.git_dir().join(name))?,
                std::path::Path::new(target),
                "Git={use_git}: dereferencing preserves the {name} symlink"
            );
            states.push(gix_testtools::repository::snapshot(repo.workdir().expect("non-bare"))?);
        }
        assert_eq!(
            states[0].head, states[1].head,
            "{name}: HEAD attachment agrees with Git"
        );
        assert_eq!(
            states[0].references, states[1].references,
            "{name}: referents agree with Git"
        );
    }
    Ok(())
}

#[test]
fn hard_reset_applies_filters_and_keeps_index_and_head_on_filter_failure() -> TestResult {
    for case in ["filtered", "filter-failure"] {
        let mut states = Vec::new();
        for use_git in [true, false] {
            let fixture = gix_testtools::scripted_fixture_writable_with_args(
                "make_reset_hard.sh",
                [case],
                gix_testtools::Creation::CopyFromReadOnly,
            )?;
            let repo = gix::open_opts(fixture.path(), crate::restricted())?;
            let root = repo.workdir().expect("non-bare");
            let target_commit_id = repo.rev_parse_single("target")?.detach();
            let previous_head_id = repo.head_id()?.detach();
            let previous_index = fs::read(repo.index_path())?;
            let success = if use_git {
                gix_testtools::git_command(root)
                    .args(["reset", "--hard", &target_commit_id.to_string()])
                    .output()?
                    .status
                    .success()
            } else {
                repo.reset_hard(target_commit_id, progress::Discard, &AtomicBool::new(false))
                    .is_ok()
            };
            assert_eq!(
                success,
                case == "filtered",
                "Git={use_git}: required filter failures are reported"
            );
            if success {
                assert_eq!(
                    fs::read(root.join("filtered.txt"))?,
                    b"FILTERED CONTENT\n",
                    "Git={use_git}: the configured smudge filter runs"
                );
                assert_eq!(
                    fs::read(root.join("crlf.txt"))?,
                    b"line one\r\nline two\r\n",
                    "Git={use_git}: checkout attributes control line endings"
                );
                states.push(gix_testtools::repository::snapshot(root)?);
            } else {
                assert_eq!(
                    repo.head_id()?,
                    previous_head_id,
                    "Git={use_git}: filter failure does not move HEAD"
                );
                assert_eq!(
                    fs::read(repo.index_path())?,
                    previous_index,
                    "Git={use_git}: filter failure leaves the original index bytes"
                );
                assert!(
                    !repo.index_path().with_extension("lock").exists(),
                    "Git={use_git}: a failed checkout releases its index lock"
                );
            }
        }
        if case == "filtered" {
            assert_eq!(states[0].index, states[1].index, "filtered index agrees with Git");
            assert_eq!(
                states[0].worktree, states[1].worktree,
                "filtered checkout agrees with Git"
            );
        }
    }
    Ok(())
}
