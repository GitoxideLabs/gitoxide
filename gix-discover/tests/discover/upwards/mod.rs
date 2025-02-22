use std::path::PathBuf;

use gix_discover::repository::Kind;

fn expected_trust() -> gix_sec::Trust {
    if std::env::var_os("GIX_TEST_EXPECT_REDUCED_TRUST").is_some() {
        gix_sec::Trust::Reduced
    } else {
        gix_sec::Trust::Full
    }
}

mod ceiling_dirs;

#[test]
fn from_bare_git_dir() -> crate::Result {
    let dir = repo_path()?.join("bare.git");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.as_ref(), dir, "the bare .git dir is directly returned");
    assert_eq!(path.kind(), Kind::PossiblyBare);
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_bare_with_index() -> crate::Result {
    let dir = repo_path()?.join("bare-with-index.git");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.as_ref(), dir, "the bare .git dir is directly returned");
    assert_eq!(path.kind(), Kind::PossiblyBare);
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_non_bare_without_index() -> crate::Result {
    let dir = repo_path()?.join("non-bare-without-index");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.as_ref(), dir, "now we refer to a worktree");
    assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None });
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_bare_git_dir_without_config_file() -> crate::Result {
    for name in ["bare-no-config.git", "bare-no-config-after-init.git"] {
        let dir = repo_path()?.join(name);
        let (path, trust) = gix_discover::upwards(&dir)?;
        assert_eq!(path.as_ref(), dir, "the bare .git dir is directly returned");
        assert_eq!(path.kind(), Kind::PossiblyBare);
        assert_eq!(trust, expected_trust());
    }
    Ok(())
}

#[test]
fn from_inside_bare_git_dir() -> crate::Result {
    let git_dir = repo_path()?.join("bare.git");
    let dir = git_dir.join("objects");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(
        path.as_ref(),
        git_dir,
        "the bare .git dir is found while traversing upwards"
    );
    assert_eq!(path.kind(), Kind::PossiblyBare);
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_git_dir() -> crate::Result {
    let dir = repo_path()?.join(".git");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None });
    assert_eq!(
        path.into_repository_and_work_tree_directories().0,
        dir,
        "the .git dir is directly returned if valid"
    );
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_working_dir() -> crate::Result {
    let dir = repo_path()?;
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.as_ref(), dir, "a working tree dir yields the git dir");
    assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None });
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_working_dir_no_config() -> crate::Result {
    for name in ["worktree-no-config-after-init", "worktree-no-config"] {
        let dir = repo_path()?.join(name);
        let (path, trust) = gix_discover::upwards(&dir)?;
        assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None },);
        assert_eq!(path.as_ref(), dir, "a working tree dir yields the git dir");
        assert_eq!(trust, expected_trust());
    }
    Ok(())
}

#[test]
fn from_nested_dir() -> crate::Result {
    let working_dir = repo_path()?;
    let dir = working_dir.join("some/very/deeply/nested/subdir");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None });
    assert_eq!(path.as_ref(), working_dir, "a working tree dir yields the git dir");
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_dir_with_dot_dot() -> crate::Result {
    // This would be neater if we could just change the actual working directory,
    // but Rust tests run in parallel by default so we'd interfere with other tests.
    // Instead ensure it finds the gitoxide repo instead of a test repo if we crawl
    // up far enough. (This tests that `discover::existing` canonicalizes paths before
    // exploring ancestors.)
    let working_dir = repo_path()?;
    let dir = working_dir.join("some/very/deeply/nested/subdir/../../../../../..");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_ne!(
        path.as_ref().canonicalize()?,
        working_dir.canonicalize()?,
        "a relative path that climbs above the test repo should yield the parent-gitoxide repo"
    );
    // If the parent repo is actually a main worktree, we can make more assertions. If it is not,
    // it will use an absolute paths and we have to bail.
    if path.as_ref() == std::path::Path::new("..") {
        assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None });
        assert_eq!(
            path.as_ref(),
            std::path::Path::new(".."),
            "there is only the minimal amount of relative path components to see this worktree"
        );
    } else {
        assert!(
            path.as_ref().is_absolute(),
            "worktree paths are absolute and the parent repo is one"
        );
        assert!(matches!(
            path.kind(),
            Kind::WorkTree {
                linked_git_dir: Some(_)
            }
        ));
    }
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_nested_dir_inside_a_git_dir() -> crate::Result {
    let working_dir = repo_path()?;
    let dir = working_dir.join(".git").join("objects");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None });
    assert_eq!(path.as_ref(), working_dir, "we find .git directories on the way");
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_non_existing_worktree() {
    let top_level_repo = repo_path().unwrap();
    let (path, _trust) = gix_discover::upwards(&top_level_repo.join("worktrees/b-private-dir-deleted")).unwrap();
    assert_eq!(path, gix_discover::repository::Path::WorkTree(top_level_repo.clone()));

    let (path, _trust) =
        gix_discover::upwards(&top_level_repo.join("worktrees/from-bare/d-private-dir-deleted")).unwrap();
    assert_eq!(path, gix_discover::repository::Path::WorkTree(top_level_repo));
}

#[test]
fn from_existing_worktree_inside_dot_git() {
    let top_level_repo = repo_path().unwrap();
    let (path, _trust) = gix_discover::upwards(&top_level_repo.join(".git/worktrees/a")).unwrap();
    let suffix = std::path::Path::new(top_level_repo.file_name().unwrap())
        .join("worktrees")
        .join("a");
    assert!(
        matches!(path, gix_discover::repository::Path::LinkedWorkTree { work_dir, .. } if work_dir.ends_with(suffix)),
        "we can handle to start from within a (somewhat partial) worktree git dir"
    );
}

#[test]
fn from_non_existing_worktree_inside_dot_git() {
    let top_level_repo = repo_path().unwrap();
    let (path, _trust) = gix_discover::upwards(&top_level_repo.join(".git/worktrees/c-worktree-deleted")).unwrap();
    let suffix = std::path::Path::new(top_level_repo.file_name().unwrap())
        .join("worktrees")
        .join("c-worktree-deleted");
    assert!(
        matches!(path, gix_discover::repository::Path::LinkedWorkTree { work_dir, .. } if work_dir.ends_with(suffix)),
        "it's no problem if work-dirs don't exist - this can be discovered later and a lot of operations are possible anyway."
    );
}

#[test]
fn from_existing_worktree() -> crate::Result {
    let top_level_repo = repo_path()?;
    for (discover_path, expected_worktree_path, expected_git_dir) in [
        (top_level_repo.join("worktrees/a"), "worktrees/a", ".git/worktrees/a"),
        (
            top_level_repo.join("worktrees/from-bare/c"),
            "worktrees/from-bare/c",
            "bare.git/worktrees/c",
        ),
    ] {
        let (path, trust) = gix_discover::upwards(&discover_path)?;
        assert!(matches!(path, gix_discover::repository::Path::LinkedWorkTree { .. }));

        assert_eq!(trust, expected_trust());
        let (git_dir, worktree) = path.into_repository_and_work_tree_directories();
        assert_eq!(
            git_dir.strip_prefix(gix_path::realpath(&top_level_repo).unwrap()),
            Ok(std::path::Path::new(expected_git_dir)),
            "we don't skip over worktrees and discover their git dir (gitdir is absolute in file)"
        );
        let worktree = worktree.expect("linked worktree is set");
        assert_eq!(
            worktree.strip_prefix(&top_level_repo),
            Ok(std::path::Path::new(expected_worktree_path)),
            "the worktree path is the .git file's directory"
        );
    }
    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn cross_fs() -> crate::Result {
    use std::{os::unix::fs::symlink, process::Command};

    use gix_discover::upwards::Options;
    if gix_testtools::is_ci::cached() {
        // Don't run on CI as it's too slow there, resource busy, it fails more often than it succeeds by now.
        return Ok(());
    }

    let top_level_repo = gix_testtools::scripted_fixture_writable("make_basic_repo.sh")?;

    let _cleanup = {
        // Create an empty dmg file
        let dmg_location = tempfile::tempdir()?;
        let dmg_file = dmg_location.path().join("temp.dmg");
        Command::new("hdiutil")
            .args(["create", "-size", "1m"])
            .arg(&dmg_file)
            .status()?;

        // Mount dmg file into temporary location
        let mount_point = tempfile::tempdir()?;
        Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-mountpoint"])
            .arg(mount_point.path())
            .arg(&dmg_file)
            .status()?;

        // Symlink the mount point into the repo
        symlink(mount_point.path(), top_level_repo.path().join("remote"))?;

        // Ensure that the mount point is always cleaned up
        defer::defer({
            let arg = mount_point.path().to_owned();
            move || {
                Command::new("hdiutil")
                    .arg("detach")
                    .arg(arg)
                    .status()
                    .expect("detach temporary test dmg filesystem successfully");
            }
        })
    };

    let res = gix_discover::upwards(&top_level_repo.path().join("remote"))
        .expect_err("the cross-fs option should prevent us from discovering the repo");
    assert!(matches!(
        res,
        gix_discover::upwards::Error::NoGitRepositoryWithinFs { .. }
    ));

    let (repo_path, _trust) = gix_discover::upwards_opts(
        &top_level_repo.path().join("remote"),
        Options {
            cross_fs: true,
            ..Default::default()
        },
    )
    .expect("the cross-fs option should allow us to discover the repo");

    assert_eq!(
        repo_path
            .into_repository_and_work_tree_directories()
            .1
            .expect("work dir")
            .file_name(),
        top_level_repo.path().file_name()
    );

    Ok(())
}

#[test]
fn do_not_shorten_absolute_paths() -> crate::Result {
    let top_level_repo = repo_path()?.canonicalize().expect("repo path exists");
    let (repo_path, _trust) = gix_discover::upwards(&top_level_repo).expect("we can discover the repo");

    match repo_path {
        gix_discover::repository::Path::WorkTree(work_dir) => {
            assert!(work_dir.is_absolute());
        }
        _ => panic!("expected worktree path"),
    }

    Ok(())
}

mod dot_git_only {
    use crate::upwards::repo_path;

    fn find_dot_git(base: impl AsRef<std::path::Path>) -> gix_discover::repository::Path {
        gix_discover::upwards_opts(
            base.as_ref(),
            gix_discover::upwards::Options {
                dot_git_only: true,
                ..Default::default()
            },
        )
        .expect("we can discover the repo")
        .0
    }

    fn assert_is_worktree_at(repo_path: gix_discover::repository::Path, expected: impl AsRef<std::path::Path>) {
        match repo_path {
            gix_discover::repository::Path::WorkTree(work_dir) => {
                assert_eq!(work_dir, expected.as_ref());
            }
            _ => panic!("expected worktree path"),
        }
    }

    #[test]
    fn succeeds_in_worktree_dir() -> crate::Result {
        let top_level_repo = repo_path()?;
        for base in [
            top_level_repo.join("some/very/deeply/nested/subdir"),
            top_level_repo.clone(),
        ] {
            let repo_path = find_dot_git(base);
            assert_is_worktree_at(repo_path, &top_level_repo);
        }
        Ok(())
    }

    #[test]
    fn succeeds_from_within_dot_git_dir() -> crate::Result {
        let top_level_repo = repo_path()?;
        for inside_git_dir in [top_level_repo.join(".git"), top_level_repo.join(".git").join("refs")] {
            let repo_path = find_dot_git(inside_git_dir);
            assert_is_worktree_at(repo_path, &top_level_repo);
        }
        Ok(())
    }

    #[test]
    fn bare_repos_are_ignored() -> crate::Result {
        let top_level_repo = repo_path()?;
        for bare_dir in [
            top_level_repo.join("bare.git"),
            top_level_repo.join("bare.git").join("refs"),
        ] {
            let repo_path = find_dot_git(bare_dir);
            assert_is_worktree_at(repo_path, &top_level_repo);
        }
        Ok(())
    }
}

mod submodules {
    #[test]
    fn by_their_worktree_checkout() -> crate::Result {
        let dir = gix_testtools::scripted_fixture_read_only("make_submodules.sh")?;
        let parent = dir.join("with-submodules");
        let modules = parent.join(".git").join("modules");
        for module in ["m1", "dir/m1"] {
            let submodule_m1_workdir = parent.join(module);
            let submodule_m1_gitdir = modules.join(module);
            let (path, _trust) = gix_discover::upwards(&submodule_m1_workdir)?;
            assert!(
                matches!(path, gix_discover::repository::Path::LinkedWorkTree{ref work_dir, ref git_dir} if work_dir == &submodule_m1_workdir && git_dir == &submodule_m1_gitdir),
                "{path:?} should match {submodule_m1_workdir:?} {submodule_m1_gitdir:?}"
            );

            let (path, _trust) = gix_discover::upwards(&submodule_m1_workdir.join("subdir"))?;
            assert!(
                matches!(path, gix_discover::repository::Path::LinkedWorkTree{ref work_dir, ref git_dir} if work_dir == &submodule_m1_workdir && git_dir == &submodule_m1_gitdir),
                "{path:?} should match {submodule_m1_workdir:?} {submodule_m1_gitdir:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn by_their_module_git_dir() -> crate::Result {
        let dir = gix_testtools::scripted_fixture_read_only("make_submodules.sh")?;
        let modules = dir.join("with-submodules").join(".git").join("modules");
        for module in ["m1", "dir/m1"] {
            let submodule_m1_gitdir = modules.join(module);
            let (path, _trust) = gix_discover::upwards(&submodule_m1_gitdir)?;
            assert!(
                matches!(path, gix_discover::repository::Path::Repository(ref dir) if dir == &submodule_m1_gitdir),
                "{path:?} should match {submodule_m1_gitdir:?}"
            );
        }
        Ok(())
    }
}

pub(crate) fn repo_path() -> crate::Result<PathBuf> {
    gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")
}
