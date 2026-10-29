use std::{fs, sync::atomic::AtomicBool};

use gix::{bstr::BString, worktree::clean::Options};
use gix_testtools::TestResult;

fn paths(entries: Vec<gix::dir::Entry>) -> Vec<BString> {
    entries.into_iter().map(|entry| entry.rela_path).collect()
}

#[test]
fn unborn_repositories_need_no_index_and_bare_repositories_are_rejected() -> TestResult {
    let (repo, _tmp) = crate::util::repo_rw("make_empty_repo.sh")?;
    let workdir = repo.workdir().expect("non-bare");
    fs::write(workdir.join("untracked"), b"remove")?;
    let removed = repo.clean_worktree(
        Vec::<BString>::new(),
        &AtomicBool::new(false),
        Options {
            dry_run: false,
            ..Default::default()
        },
    )?;
    assert_eq!(
        paths(removed),
        vec![BString::from("untracked")],
        "an unborn repository can clean before its first index write"
    );
    assert!(!repo.index_path().exists(), "clean never creates an absent index");
    assert!(workdir.is_dir(), "clean preserves the worktree root");
    let fixture = gix_testtools::scripted_fixture_read_only("make_clean_worktree.sh")?;
    let bare = gix::open_opts(fixture.join("protected/bare"), crate::restricted())?;
    assert!(
        bare.clean_worktree(Vec::<BString>::new(), &AtomicBool::new(false), Options::default())
            .is_err(),
        "bare repositories have no cleanable worktree"
    );
    Ok(())
}

#[test]
fn dry_run_and_deletion_match_git_clean() -> TestResult {
    for ignored in [false, true] {
        for directories in [false, true] {
            let fixture = gix_testtools::scripted_fixture_writable("make_clean_worktree.sh")?;
            let path = fixture.path().join("simple");
            #[cfg(unix)]
            let _sockets = {
                fs::create_dir(path.join("special-dir"))?;
                [
                    std::os::unix::net::UnixListener::bind(path.join("special-file"))?,
                    std::os::unix::net::UnixListener::bind(path.join("special-dir/socket"))?,
                ]
            };
            let repo = gix::open_opts(&path, crate::restricted())?;
            let before = fs::read(repo.index_path())?;
            let git = gix_testtools::git_command(&path)
                .args(["clean", if directories { "-nd" } else { "-n" }])
                .args(ignored.then_some("-x"))
                .output()?;
            assert!(git.status.success(), "Git provides the clean reference result");
            let mut expected: Vec<BString> = String::from_utf8(git.stdout)?
                .lines()
                .map(|line| {
                    line.strip_prefix("Would remove ")
                        .expect("Git dry-run output")
                        .trim_end_matches('/')
                        .into()
                })
                .collect();
            expected.sort();
            let options = Options {
                ignored,
                directories,
                ..Default::default()
            };
            let preview = repo.clean_worktree(Vec::<BString>::new(), &AtomicBool::new(false), options)?;
            assert_eq!(paths(preview.clone()), expected, "dry-run candidates match Git's flags");
            assert!(path.join("untracked").exists(), "dry-run leaves candidates intact");
            let deleted = repo.clean_worktree(
                Vec::<BString>::new(),
                &AtomicBool::new(false),
                Options {
                    dry_run: false,
                    ..options
                },
            )?;
            assert_eq!(
                paths(deleted),
                expected,
                "execution deletes the same candidates as dry-run"
            );
            for entry in preview {
                assert!(
                    !path.join(gix::path::from_bstr(entry.rela_path)?).exists(),
                    "each reported path was removed"
                );
            }
            assert_eq!(fs::read(repo.index_path())?, before, "clean never changes the index");
            assert!(path.join("tracked-dir/tracked").exists(), "tracked contents survive");
        }
    }
    Ok(())
}

#[test]
fn explicit_paths_enable_directories_and_obey_exclusions() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_clean_worktree.sh")?;
    let path = fixture.path().join("simple");
    let repo = gix::open_opts(&path, crate::restricted())?;
    let patterns = [
        BString::from("untracked-dir"),
        BString::from("tracked-dir"),
        BString::from(":(exclude)tracked-dir"),
    ];
    let removed = repo.clean_worktree(
        patterns,
        &AtomicBool::new(false),
        Options {
            dry_run: false,
            ..Default::default()
        },
    )?;
    assert_eq!(
        paths(removed),
        vec![BString::from("untracked-dir")],
        "explicit pathspecs imply directory cleaning like Git"
    );
    assert!(path.join("tracked-dir/untracked").exists(), "excluded paths survive");
    assert!(path.join("untracked").exists(), "unmatched paths survive");
    assert!(
        repo.clean_worktree(
            [BString::from("../outside")],
            &AtomicBool::new(false),
            Options::default()
        )
        .is_err(),
        "pathspecs cannot escape the worktree"
    );
    Ok(())
}

#[test]
fn repositories_precious_paths_and_tracked_typechanges_survive() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_clean_worktree.sh")?;
    let path = fixture.path().join("protected");
    let repo = gix::open_opts(&path, crate::restricted())?;
    let removed = paths(repo.clean_worktree(
        Vec::<BString>::new(),
        &AtomicBool::new(false),
        Options {
            dry_run: false,
            ignored: true,
            directories: true,
        },
    )?);
    for name in ["untracked", "ignored/trash", "precious-dir/trash", "outer/trash"] {
        assert!(
            removed.contains(&BString::from(name)),
            "ordinary candidates are removed: {name}"
        );
    }
    for name in [
        "nested/.git",
        "bare/HEAD",
        "ignored/nested/.git",
        "ignored/bare/HEAD",
        "ignored/linked/.git",
        "precious",
        "precious-dir/keep",
        "tracked-file/child",
        "outer/tracked/child",
        "tracked-dir",
    ] {
        assert!(
            path.join(name).exists(),
            "protected data survives, including inside ignored directories: {name}"
        );
    }
    assert_eq!(
        fs::read(fixture.path().join("outside/sentinel"))?,
        b"outside\n",
        "symlink targets outside the worktree survive"
    );
    #[cfg(unix)]
    assert!(
        fs::symlink_metadata(path.join("outside-link")).is_err(),
        "an untracked symlink itself is removed"
    );
    Ok(())
}

#[test]
fn cancellation_and_index_lock_precede_any_deletion() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_clean_worktree.sh")?;
    let path = fixture.path().join("simple");
    let repo = gix::open_opts(&path, crate::restricted())?;
    let options = Options {
        dry_run: false,
        directories: true,
        ..Default::default()
    };
    assert!(
        repo.clean_worktree(Vec::<BString>::new(), &AtomicBool::new(true), options)
            .is_err(),
        "cancellation is an error before any mutation"
    );
    fs::write(repo.index_path().with_extension("lock"), b"held")?;
    assert!(
        repo.clean_worktree(Vec::<BString>::new(), &AtomicBool::new(false), options)
            .is_err(),
        "another writer's index lock prevents deletion"
    );
    assert!(
        !repo
            .clean_worktree(Vec::<BString>::new(), &AtomicBool::new(false), Options::default())?
            .is_empty(),
        "dry-run does not need the index lock"
    );
    assert!(
        path.join("untracked").exists(),
        "blocked operations leave the worktree intact"
    );
    assert_eq!(
        fs::read(repo.index_path().with_extension("lock"))?,
        b"held",
        "another writer's lock is untouched"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlinked_index_uses_the_target_lock() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_clean_worktree.sh")?;
    let repo = gix::open_opts(fixture.path().join("simple"), crate::restricted())?;
    let index = repo.index_path();
    let target = index.with_file_name("other-index");
    fs::rename(&index, &target)?;
    std::os::unix::fs::symlink("other-index", &index)?;
    fs::write(target.with_extension("lock"), b"held")?;
    let options = Options {
        dry_run: false,
        directories: true,
        ..Default::default()
    };
    assert!(
        repo.clean_worktree(Vec::<BString>::new(), &AtomicBool::new(false), options)
            .is_err(),
        "a writer locking a symlinked index target prevents cleaning"
    );
    assert!(
        repo.workdir().expect("non-bare").join("untracked").exists(),
        "a symlinked index lock protects the worktree before deletion"
    );
    assert_eq!(
        fs::read(target.with_extension("lock"))?,
        b"held",
        "the writer's target lock stays intact"
    );
    Ok(())
}

#[test]
fn current_directory_prefix_and_empty_directory_are_preserved() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable("make_clean_worktree.sh")?;
    let path = fixture.path().join("simple").canonicalize()?;
    let _cwd = gix_testtools::set_current_dir(path.join("tracked-dir"))?;
    let repo = gix::open_opts(&path, crate::restricted())?;
    let removed = repo.clean_worktree(
        Vec::<BString>::new(),
        &AtomicBool::new(false),
        Options {
            dry_run: false,
            directories: true,
            ..Default::default()
        },
    )?;
    assert_eq!(
        paths(removed),
        vec![BString::from("tracked-dir/untracked")],
        "empty pathspecs apply to the repository prefix"
    );
    assert!(path.join("untracked").exists(), "paths outside the prefix survive");
    drop(_cwd);
    let _cwd = gix_testtools::set_current_dir(path.join("empty-dir"))?;
    let repo = gix::open_opts(&path, crate::restricted())?;
    assert!(
        repo.clean_worktree(
            Vec::<BString>::new(),
            &AtomicBool::new(false),
            Options {
                dry_run: false,
                directories: true,
                ..Default::default()
            }
        )?
        .is_empty(),
        "the empty current directory is not a deletion candidate"
    );
    assert!(path.join("empty-dir").is_dir(), "the current directory survives");
    Ok(())
}
