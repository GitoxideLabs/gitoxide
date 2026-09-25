use crate::Result;
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

fn optional_repository_missing(err: &gix_error::Error) -> bool {
    use gix_discover::upwards::Error;
    matches!(
        err.downcast_any_ref::<Error>(),
        Some(
            Error::NoGitRepository { .. }
                | Error::NoGitRepositoryWithinCeiling { .. }
                | Error::NoGitRepositoryWithinFs { .. }
        )
    )
}

#[test]
fn discovery_error_variants_are_intrinsically_not_found() {
    use gix_discover::upwards::Error;
    use gix_error::{ErrorExt, message};

    for err in [
        Error::NoGitRepository { path: "start".into() },
        Error::NoGitRepositoryWithinCeiling {
            path: "start".into(),
            ceiling_height: 1,
        },
        Error::NoGitRepositoryWithinFs {
            path: "start".into(),
            limit: "limit".into(),
        },
        Error::NoTrustedGitRepository {
            path: "start".into(),
            candidate: "candidate".into(),
            required: gix_sec::Trust::Full,
            trust: gix_sec::Trust::Reduced,
        },
    ] {
        let err = err.raise_erased();
        assert!(err.is_not_found(), "each variant is classified without a call-site tag");

        let err = err.raise(message("repository discovery failed")).into_error();
        assert!(err.is_not_found(), "classification survives context and conversion");
        assert!(
            err.downcast_any_ref::<Error>().is_some(),
            "the concrete discovery error remains available for recovery"
        );
        assert!(
            err.classify()
                .next()
                .expect("not-found classification")
                .error()
                .is::<Error>(),
            "the classification identifies the discovery error rather than its marker"
        );
    }
}

#[test]
fn optional_repository_recovery_excludes_io_and_untrusted_candidates() -> Result {
    use gix_discover::upwards::{Error, Options, TrustPolicy};
    use gix_error::ErrorExt;

    let root = gix_testtools::tempfile::tempdir()?;
    let start = root.path().join("not-a-repository");
    std::fs::create_dir(&start)?;
    let options = || Options {
        current_dir: Some(root.path()),
        trust: TrustPolicy::Assume(gix_sec::Trust::Reduced),
        cross_fs: true,
        ..Default::default()
    };
    let err = gix_discover::upwards_opts(&start, options()).expect_err("no repository in temporary ancestry");
    assert!(
        optional_repository_missing(&err),
        "exhausting the search allows the optional-repository fallback"
    );
    assert!(
        err.is_not_found(),
        "missing repositories retain their broad classification"
    );
    assert!(
        matches!(err.downcast_any_ref::<Error>(), Some(Error::NoGitRepository { path }) if path == &start),
        "discovery retains the starting path"
    );
    assert!(
        err.classify()
            .next()
            .expect("not-found classification")
            .error()
            .is::<Error>(),
        "the classification identifies the discovery error"
    );

    let err = gix_discover::upwards_opts(&start.join("missing"), options()).expect_err("the input directory is absent");
    assert!(
        err.is_not_found(),
        "missing directories also have the broad NotFound class"
    );
    assert!(
        !optional_repository_missing(&err),
        "missing-directory I/O is not an optional repository outcome"
    );
    assert!(
        err.downcast_any_ref::<std::io::Error>().is_some(),
        "the original I/O cause is retained"
    );

    let err = Error::NoTrustedGitRepository {
        path: start.clone(),
        candidate: start.join(".git"),
        required: gix_sec::Trust::Full,
        trust: gix_sec::Trust::Reduced,
    }
    .raise()
    .into_error();
    assert!(
        err.is_not_found(),
        "rejected candidates retain the NotFound classification"
    );
    assert!(
        !optional_repository_missing(&err),
        "the fallback must propagate untrusted candidates"
    );
    Ok(())
}

// macOS filesystems require valid UTF-8 names.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn discovery_errors_preserve_non_utf8_paths() -> Result {
    use std::os::unix::ffi::OsStrExt;

    let root = gix_testtools::tempfile::tempdir()?;
    let start = root.path().join(std::ffi::OsStr::from_bytes(b"directory-\xff"));
    std::fs::create_dir(&start)?;
    let err = gix_discover::upwards_opts(
        &start,
        gix_discover::upwards::Options {
            current_dir: Some(root.path()),
            ceiling_dirs: vec![root.path().to_owned()],
            trust: gix_discover::upwards::TrustPolicy::Assume(gix_sec::Trust::Reduced),
            ..Default::default()
        },
    )
    .expect_err("no repository before the ceiling");
    assert!(
        matches!(err.downcast_any_ref::<gix_discover::upwards::Error>(),
            Some(gix_discover::upwards::Error::NoGitRepositoryWithinCeiling { path, ceiling_height: 2 }) if path == &start),
        "the payload preserves native path bytes and stops after searching the ceiling directory"
    );
    Ok(())
}

#[test]
fn can_override_computed_trust() -> Result {
    let dir = repo_path()?.join("some/very/deeply/nested/subdir");
    let overridden_trust = match expected_trust() {
        gix_sec::Trust::Full => gix_sec::Trust::Reduced,
        gix_sec::Trust::Reduced => gix_sec::Trust::Full,
    };

    let (path, trust) = gix_discover::upwards_opts(
        &dir,
        gix_discover::upwards::Options {
            trust: gix_discover::upwards::TrustPolicy::Assume(overridden_trust),
            ..Default::default()
        },
    )?;

    assert_eq!(
        path.kind(),
        Kind::WorkTree { linked_git_dir: None },
        "discovery still finds the worktree"
    );
    assert_eq!(
        trust, overridden_trust,
        "the caller-provided trust is returned instead of the computed ownership trust"
    );
    if expected_trust() == gix_sec::Trust::Reduced {
        let err = gix_discover::upwards_opts(
            &dir,
            gix_discover::upwards::Options {
                trust: gix_discover::upwards::TrustPolicy::Required(gix_sec::Trust::Full),
                ..Default::default()
            },
        )
        .expect_err("a foreign-owned fixture cannot meet full trust");
        assert!(
            matches!(err.downcast_any_ref::<gix_discover::upwards::Error>(),
                Some(gix_discover::upwards::Error::NoTrustedGitRepository {
                    path, candidate, required: gix_sec::Trust::Full, trust: gix_sec::Trust::Reduced,
                }) if path == &dir && candidate.ends_with(".git")),
            "trust rejection retains the search path, candidate, and both trust levels"
        );
        assert!(
            !optional_repository_missing(&err),
            "an untrusted repository must be propagated"
        );
    }
    Ok(())
}

#[test]
fn from_bare_git_dir() -> Result {
    let dir = repo_path()?.join("bare.git");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.as_ref(), dir, "the bare .git dir is directly returned");
    assert_eq!(path.kind(), Kind::PossiblyBare);
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_bare_with_index() -> Result {
    let dir = repo_path()?.join("bare-with-index.git");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.as_ref(), dir, "the bare .git dir is directly returned");
    assert_eq!(path.kind(), Kind::PossiblyBare);
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_non_bare_without_index() -> Result {
    let dir = repo_path()?.join("non-bare-without-index");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.as_ref(), dir, "now we refer to a worktree");
    assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None });
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_non_bare_repo_with_git_extension() -> Result {
    let dir = repo_path()?.join("repo.git");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(
        path.as_ref(),
        dir,
        "a non-bare repository named repo.git is returned as a worktree"
    );
    assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None });
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_bare_git_dir_without_config_file() -> Result {
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
fn from_inside_bare_git_dir() -> Result {
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
fn from_git_dir() -> Result {
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
fn from_working_dir() -> Result {
    let dir = repo_path()?;
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.as_ref(), dir, "a working tree dir yields the git dir");
    assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None });
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn from_working_dir_no_config() -> Result {
    for name in ["worktree-no-config-after-init", "worktree-no-config"] {
        let dir = repo_path()?.join(name);
        let (path, trust) = gix_discover::upwards(&dir)?;
        assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None });
        assert_eq!(path.as_ref(), dir, "a working tree dir yields the git dir");
        assert_eq!(trust, expected_trust());
    }
    Ok(())
}

#[test]
fn from_nested_dir() -> Result {
    let working_dir = repo_path()?;
    let dir = working_dir.join("some/very/deeply/nested/subdir");
    let (path, trust) = gix_discover::upwards(&dir)?;
    assert_eq!(path.kind(), Kind::WorkTree { linked_git_dir: None });
    assert_eq!(path.as_ref(), working_dir, "a working tree dir yields the git dir");
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
fn an_invalid_dot_git_directory_does_not_skip_ancestors() -> Result {
    let repo = gix_path::realpath(repo_path()?)?;
    let start = repo.join("non-repo/.git");

    let (path, _trust) = gix_discover::upwards(&start)?;
    assert_eq!(
        path.as_ref(),
        repo,
        "after rejecting an invalid `.git`, discovery checks every physical ancestor"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn from_symlinked_nested_dir_follows_target_ancestors() -> Result {
    let root = gix_testtools::scripted_fixture_read_only("make_symlinked_nested_repo.sh")?;
    let link = root.join("lexical-parent/link");

    let (path, trust) = gix_discover::upwards(&link)?;
    assert_eq!(
        path.kind(),
        Kind::WorkTree { linked_git_dir: None },
        "discovery through the symlink finds the target's worktree"
    );
    assert_eq!(
        gix_path::realpath(path.as_ref())?,
        gix_path::realpath(&root)?,
        "parent traversal follows the symlink target instead of the symlink's parent"
    );
    assert_eq!(trust, expected_trust());
    Ok(())
}

#[test]
#[cfg(unix)]
fn symlink_is_resolved_before_parent_component() -> Result {
    let root = gix_testtools::scripted_fixture_read_only("make_symlinked_nested_repo.sh")?;

    for path in [
        "lexical-parent/link/real-dir/..",
        "lexical-parent/link/real-dir",
        "lexical-parent/link/..",
        "lexical-parent/link",
    ] {
        let (actual, _trust) = gix_discover::upwards(&root.join(path))?;
        assert_eq!(
            gix_path::realpath(actual.as_ref())?,
            gix_path::realpath(&root)?,
            "{path} follows the symlink before ascending, just like Git"
        );
    }
    Ok(())
}

#[test]
#[cfg(unix)]
fn relative_symlinks_use_the_configured_current_dir() -> Result {
    let root = gix_testtools::scripted_fixture_read_only("make_symlinked_nested_repo.sh")?;
    let root = std::env::current_dir()?.join(root);

    for (cwd, input, expected_worktree) in [
        (root.join("lexical-parent"), "link/..", std::path::Path::new("..")),
        (
            root.clone(),
            "linked-parent/repo/nested",
            std::path::Path::new("linked-parent/repo"),
        ),
    ] {
        let (path, _trust) = gix_discover::upwards_opts(
            input.as_ref(),
            gix_discover::upwards::Options {
                current_dir: Some(&cwd),
                ..Default::default()
            },
        )?;
        assert_eq!(
            path.into_repository_and_work_tree_directories().1.as_deref(),
            Some(expected_worktree),
            "{input} is resolved and returned relative to the configured current directory"
        );
    }
    Ok(())
}

#[test]
fn from_dir_with_dot_dot() -> Result {
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
fn from_nested_dir_inside_a_git_dir() -> Result {
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
fn from_existing_worktree() -> Result {
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

#[test]
fn from_existing_worktree_with_relative_linking_files() -> Result {
    let fixture = gix_testtools::scripted_fixture_read_only_needs_archive("make_worktree_relative_linking.sh")?;
    let main = fixture.join("main");
    let linked = fixture.join("linked");
    let private_git_dir = main.join(".git/worktrees/linked");
    assert_eq!(
        std::fs::read_to_string(linked.join(".git"))?,
        "gitdir: ../main/.git/worktrees/linked\n",
        "the linked checkout uses a relative gitdir file"
    );
    let backlink = std::fs::read_to_string(private_git_dir.join("gitdir"))?;
    assert_eq!(
        backlink, "../../../../linked/.git\n",
        "the private git dir points back to the checkout with a relative path"
    );

    for discover_path in [&linked, &private_git_dir] {
        let (path, trust) = gix_discover::upwards(discover_path)?;
        assert_eq!(trust, expected_trust());
        let (actual_git_dir, actual_worktree) = path.into_repository_and_work_tree_directories();
        assert_eq!(
            gix_path::realpath(&actual_git_dir)?,
            gix_path::realpath(&private_git_dir)?,
            "discovery resolves the private git dir from relative worktree metadata"
        );
        assert_eq!(
            actual_worktree.as_deref().map(gix_path::realpath).transpose()?,
            Some(gix_path::realpath(&linked)?),
            "discovery resolves the linked worktree from relative worktree metadata"
        );
    }

    Ok(())
}

#[test]
#[cfg(unix)]
fn from_symlinked_worktree_with_relative_linking_files() -> Result {
    let fixture = gix_testtools::scripted_fixture_read_only_needs_archive("make_worktree_relative_linking.sh")?;
    let main = fixture.join("actual/main");
    let linked_symlink = fixture.join("linked-symlink");

    let (path, trust) = gix_discover::upwards(&linked_symlink)?;
    assert_eq!(trust, expected_trust());
    let (actual_git_dir, actual_worktree) = path.into_repository_and_work_tree_directories();
    assert_eq!(
        gix_path::realpath(&actual_git_dir)?,
        gix_path::realpath(main.join(".git/worktrees/linked"))?,
        "the private git dir is found through a relative gitdir file reached via a symlinked checkout"
    );
    assert_eq!(
        actual_worktree.as_deref(),
        Some(linked_symlink.as_path()),
        "the discovered worktree remains the user-provided symlinked checkout"
    );

    Ok(())
}

#[cfg(target_os = "macos")]
#[test]
fn cross_fs() -> Result {
    use std::process::Command;

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

        // Mount the separate filesystem directly inside the repository so physical parent
        // traversal reaches the repository when crossing filesystem boundaries is allowed.
        let mount_point = top_level_repo.path().join("remote");
        std::fs::create_dir(&mount_point)?;
        Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-mountpoint"])
            .arg(&mount_point)
            .arg(&dmg_file)
            .status()?;

        // Ensure that the mount point is always cleaned up
        defer::defer({
            let arg = mount_point;
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
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&res, &[
        (&top_level_repo.path().canonicalize()?.to_string_lossy(), "<repository>"),
        (&top_level_repo.path().to_string_lossy(), "<repository>"),
    ]), "discovery stops at the filesystem boundary", @"Could not find a git repository in '<repository>/remote' or in any of its parents within device limits below '<repository>'");
    assert!(res.is_not_found());
    assert!(
        matches!(res.downcast_any_ref::<gix_discover::upwards::Error>(),
            Some(gix_discover::upwards::Error::NoGitRepositoryWithinFs { path, limit })
                if path == &top_level_repo.path().join("remote")
                    && limit == &top_level_repo.path().canonicalize()?),
        "filesystem limits retain the starting path and the physical stopping directory"
    );
    assert!(
        optional_repository_missing(&res),
        "a filesystem search limit allows the fallback"
    );

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
fn do_not_shorten_absolute_paths() -> Result {
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

#[cfg(unix)]
#[test]
fn preserves_symlinked_ancestor_of_absolute_paths() -> Result {
    let root = gix_testtools::scripted_fixture_read_only("make_symlinked_nested_repo.sh")?;
    let root = std::env::current_dir()?.join(root);
    let worktree = root.join("linked-parent/repo");

    for input in ["", "nested", "nested/..", ".git/objects"] {
        let (path, _trust) = gix_discover::upwards(&worktree.join(input))?;
        let (git_dir, actual_worktree) = path.into_repository_and_work_tree_directories();
        assert_eq!(
            git_dir,
            worktree.join(".git"),
            "{input} retains the symlinked spelling for the git directory"
        );
        assert_eq!(
            actual_worktree.as_deref(),
            Some(worktree.as_path()),
            "{input} retains the symlinked spelling for the worktree"
        );
    }

    let bare = root.join("linked-parent/bare.git");
    for input in ["", "objects"] {
        let (path, _trust) = gix_discover::upwards(&bare.join(input))?;
        let (git_dir, actual_worktree) = path.into_repository_and_work_tree_directories();
        assert_eq!(
            git_dir, bare,
            "{input} retains the symlinked spelling for a bare repository"
        );
        assert_eq!(actual_worktree, None, "a bare repository has no worktree");
    }
    Ok(())
}

mod dot_git_only {
    use crate::Result;
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
    fn succeeds_in_worktree_dir() -> Result {
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
    fn succeeds_from_within_dot_git_dir() -> Result {
        let top_level_repo = repo_path()?;
        for inside_git_dir in [top_level_repo.join(".git"), top_level_repo.join(".git").join("refs")] {
            let repo_path = find_dot_git(inside_git_dir);
            assert_is_worktree_at(repo_path, &top_level_repo);
        }
        Ok(())
    }

    #[test]
    fn bare_repos_are_ignored() -> Result {
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
    use crate::Result;

    #[test]
    fn by_their_worktree_checkout() -> Result {
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
    fn by_their_module_git_dir() -> Result {
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

pub(crate) fn repo_path() -> Result<PathBuf> {
    gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")
}
