use std::path::{Path, PathBuf};

use gix_discover::upwards::Options;
use serial_test::serial;

#[test]
#[serial]
fn in_cwd_upwards_from_nested_dir() -> gix_testtools::Result {
    let repo = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;

    let _keep = gix_testtools::set_current_dir(repo)?;
    for dir in ["subdir", "some/very/deeply/nested/subdir"] {
        let (repo_path, _trust) = gix_discover::upwards(Path::new(dir))?;
        assert_eq!(
            repo_path.kind(),
            gix_discover::repository::Kind::WorkTree { linked_git_dir: None },
        );
        assert_eq!(repo_path.as_ref(), Path::new("."), "{dir}");
    }
    Ok(())
}

#[test]
#[serial]
#[cfg(unix)]
fn upwards_from_relative_symlink_paths() -> gix_testtools::Result {
    let repo = gix_testtools::scripted_fixture_read_only("make_symlinked_nested_repo.sh")?;

    let _keep = gix_testtools::set_current_dir(repo.join("lexical-parent"))?;
    for path in [
        "./link/real-dir/..",
        "link/real-dir/..",
        "link/real-dir",
        "link/..",
        "./link/..",
    ] {
        let (repo_path, _trust) = gix_discover::upwards(Path::new(path))?;
        assert_repo_is_current_workdir(repo_path, Path::new(".."));
    }
    Ok(())
}

#[test]
#[serial]
#[cfg(unix)]
fn relative_paths_retain_a_symlinked_ancestor() -> gix_testtools::Result {
    let repo = gix_testtools::scripted_fixture_read_only("make_symlinked_nested_repo.sh")?;

    let _keep = gix_testtools::set_current_dir(repo)?;
    for path in ["linked-parent/repo/nested", "linked-parent/repo/nested/.."] {
        let (repo_path, _trust) = gix_discover::upwards(Path::new(path))?;
        assert_repo_is_current_workdir(repo_path, Path::new("linked-parent/repo"));
    }
    Ok(())
}

#[test]
#[serial]
fn upwards_bare_repo_with_index() -> gix_testtools::Result {
    let repo = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;

    let _keep = gix_testtools::set_current_dir(repo.join("bare-with-index.git"))?;
    let (repo_path, _trust) = gix_discover::upwards(".".as_ref())?;
    assert_eq!(
        repo_path.kind(),
        gix_discover::repository::Kind::PossiblyBare,
        "bare stays bare, even with index, as it resolves the path as needed in this special case"
    );
    Ok(())
}

#[test]
#[serial]
fn in_cwd_upwards_bare_repo_without_index() -> gix_testtools::Result {
    let repo = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;

    let _keep = gix_testtools::set_current_dir(repo.join("bare.git"))?;
    let (repo_path, _trust) = gix_discover::upwards(".".as_ref())?;
    assert_eq!(repo_path.kind(), gix_discover::repository::Kind::PossiblyBare);
    Ok(())
}

#[test]
#[serial]
fn in_cwd_upwards_nonbare_repo_without_index() -> gix_testtools::Result {
    let repo = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;

    let _keep = gix_testtools::set_current_dir(repo.join("non-bare-without-index"))?;
    let (repo_path, _trust) = gix_discover::upwards(".".as_ref())?;
    assert_eq!(
        repo_path.kind(),
        gix_discover::repository::Kind::WorkTree { linked_git_dir: None },
    );
    Ok(())
}

#[test]
#[serial]
fn upwards_with_relative_directories_and_optional_ceiling() -> gix_testtools::Result {
    let repo = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?;

    let _keep = gix_testtools::set_current_dir(repo.join("some"))?;
    let cwd = std::env::current_dir()?;

    for (search_dir, ceiling_dir_component) in [
        (".", ".."),
        (".", "./.."),
        ("./.", "./.."),
        (".", "./does-not-exist/../.."),
        ("./././very/deeply/nested/subdir", ".."),
        ("very/deeply/nested/subdir", ".."),
    ] {
        let search_dir = Path::new(search_dir);
        let ceiling_dir = cwd.join(ceiling_dir_component);
        let (repo_path, _trust) = gix_discover::upwards_opts(
            search_dir,
            Options {
                ceiling_dirs: vec![ceiling_dir],
                ..Default::default()
            },
        )
        .expect("ceiling dir should allow us to discover the repo");
        assert_repo_is_current_workdir(repo_path, Path::new(".."));

        let (repo_path, _trust) =
            gix_discover::upwards_opts(search_dir, Default::default()).expect("without ceiling dir we see the same");
        assert_repo_is_current_workdir(repo_path, Path::new(".."));

        let (repo_path, _trust) = gix_discover::upwards_opts(
            search_dir,
            Options {
                ceiling_dirs: vec![PathBuf::from("..")],
                ..Default::default()
            },
        )
        .expect("purely relative ceiling dirs work as well");
        assert_repo_is_current_workdir(repo_path, Path::new(".."));

        let err = gix_discover::upwards_opts(
            search_dir,
            Options {
                ceiling_dirs: vec![PathBuf::from(".")],
                ..Default::default()
            },
        )
        .unwrap_err();

        if search_dir.parent() == Some(".".as_ref()) || search_dir.parent() == Some("".as_ref()) {
            assert!(err.downcast_any_ref::<gix_error::ValidationError>().is_some());
        } else {
            assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
        }
    }

    Ok(())
}

#[test]
#[serial]
fn unc_paths_are_handled_on_windows() -> gix_testtools::Result {
    let repo = gix_testtools::scripted_fixture_read_only("make_basic_repo.sh").unwrap();

    let _keep = gix_testtools::set_current_dir(repo.join("some/very/deeply/nested/subdir")).unwrap();
    let cwd = std::env::current_dir().unwrap();
    let parent = cwd.parent().unwrap();
    // all discoveries should fail, as they'll hit `parent` before finding a git repository.

    // dir: normal, ceiling: normal
    let res = gix_discover::upwards_opts(
        &cwd,
        Options {
            ceiling_dirs: vec![parent.to_path_buf()],
            match_ceiling_dir_or_error: false,
            ..Default::default()
        },
    );
    assert!(res.is_err(), "{res:?}");

    let parent = parent.canonicalize().unwrap();
    // dir: normal, ceiling: extended
    let res = gix_discover::upwards_opts(
        &cwd,
        Options {
            ceiling_dirs: vec![parent],
            match_ceiling_dir_or_error: false,
            ..Default::default()
        },
    );
    assert!(res.is_err(), "{res:?}");

    let cwd = cwd.canonicalize().unwrap();

    let parent = cwd.parent().unwrap();
    // dir: extended, ceiling: normal
    let res = gix_discover::upwards_opts(
        &cwd,
        Options {
            ceiling_dirs: vec![parent.to_path_buf()],
            match_ceiling_dir_or_error: false,
            ..Default::default()
        },
    );
    assert!(res.is_err(), "{res:?}");

    let parent = parent.canonicalize().unwrap();
    // dir: extended, ceiling: extended
    let res = gix_discover::upwards_opts(
        &cwd,
        Options {
            ceiling_dirs: vec![parent],
            match_ceiling_dir_or_error: false,
            ..Default::default()
        },
    );
    assert!(res.is_err(), "{res:?}");
    Ok(())
}

fn assert_repo_is_current_workdir(path: gix_discover::repository::Path, work_dir: &Path) {
    assert_eq!(
        path.into_repository_and_work_tree_directories().1.expect("work dir"),
        work_dir,
    );
}
