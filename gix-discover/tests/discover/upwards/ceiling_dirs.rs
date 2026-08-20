use std::path::Path;

use gix_discover::upwards::Options;

use crate::upwards::repo_path;

fn assert_repo_is_current_workdir(path: gix_discover::repository::Path, work_dir: &Path) {
    assert_eq!(
        path.into_repository_and_work_tree_directories()
            .1
            .expect("work dir")
            .file_name(),
        work_dir.file_name()
    );
}

#[test]
fn git_dir_candidate_within_ceiling_allows_discovery() -> crate::Result {
    let work_dir = repo_path()?;
    let dir = work_dir.join("some/very/deeply/nested/subdir");
    let (repo_path, _trust) = gix_discover::upwards_opts(
        &dir,
        Options {
            ceiling_dirs: vec![work_dir.clone()],
            ..Default::default()
        },
    )
    .expect("ceiling dir should allow us to discover the repo");
    assert_repo_is_current_workdir(repo_path, &work_dir);

    Ok(())
}

#[test]
fn ceiling_dir_is_ignored_if_we_are_standing_on_the_ceiling_and_no_match_is_required() -> crate::Result {
    let work_dir = repo_path()?;
    let dir = work_dir.join("some/very/deeply/nested/subdir");
    // the ceiling dir is equal to the input dir, which itself doesn't contain a repository.
    // But we can ignore that just like git does (see https://github.com/GitoxideLabs/gitoxide/pull/723 for more information)
    // and imagine us to 'stand on the ceiling', hence we are already past it.
    let (repo_path, _trust) = gix_discover::upwards_opts(
        &dir.clone(),
        Options {
            ceiling_dirs: vec![dir],
            match_ceiling_dir_or_error: false,
            ..Default::default()
        },
    )
    .expect("ceiling dir should be skipped");
    assert_repo_is_current_workdir(repo_path, &work_dir);

    Ok(())
}

#[test]
fn discovery_fails_if_we_require_a_matching_ceiling_dir_but_are_standing_on_it() -> crate::Result {
    let work_dir = repo_path()?;
    let dir = work_dir.join("some/very/deeply/nested/subdir");
    let err = gix_discover::upwards_opts(
        &dir.clone(),
        Options {
            ceiling_dirs: vec![dir],
            match_ceiling_dir_or_error: true,
            ..Default::default()
        },
    )
    .unwrap_err();

    assert!(
        err.downcast_any_ref::<gix_error::ValidationError>().is_some(),
        "since standing on the ceiling dir doesn't match it, we get exactly the semantically correct error"
    );
    Ok(())
}

#[test]
fn ceiling_dir_limits_are_respected_and_prevent_discovery() -> crate::Result {
    let work_dir = repo_path()?;
    let dir = work_dir.join("some/very/deeply/nested/subdir");

    let err = gix_discover::upwards_opts(
        &dir,
        Options {
            ceiling_dirs: vec![work_dir.join("some/../some")],
            ..Default::default()
        },
    )
    .expect_err("ceiling dir prevents discovery as it ends on level too early, and they are also absolutized");
    assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
    assert!(err.to_string().contains("ceiling height of 5"));

    Ok(())
}

#[test]
fn no_matching_ceiling_dir_error_can_be_suppressed() -> crate::Result {
    let work_dir = repo_path()?;
    let dir = work_dir.join("some/very/deeply/nested/subdir");
    let (repo_path, _trust) = gix_discover::upwards_opts(
        &dir,
        Options {
            match_ceiling_dir_or_error: false,
            ceiling_dirs: vec![
                work_dir.canonicalize()?,
                work_dir.join("some/very/deeply/nested/subdir/too-deep"),
                work_dir.join("some/very/deeply/nested/unrelated-dir"),
                work_dir.join("a/completely/unrelated/dir"),
            ],
            ..Default::default()
        },
    )
    .expect("ceiling dir should allow us to discover the repo");
    assert_repo_is_current_workdir(repo_path, &work_dir);

    Ok(())
}

#[test]
fn more_restrictive_ceiling_dirs_overrule_less_restrictive_ones() -> crate::Result {
    let work_dir = repo_path()?;
    let dir = work_dir.join("some/very/deeply/nested/subdir");
    let err = gix_discover::upwards_opts(
        &dir,
        Options {
            ceiling_dirs: vec![work_dir.clone(), work_dir.join("some")],
            ..Default::default()
        },
    )
    .expect_err("more restrictive ceiling dirs overrule less restrictive ones");
    assert!(err.downcast_any_ref::<gix_error::NotFoundError>().is_some());
    assert!(err.to_string().contains("ceiling height of 5"));

    Ok(())
}

#[test]
fn ceiling_dirs_are_not_processed_differently_than_the_git_dir_candidate() -> crate::Result {
    let work_dir = repo_path()?;
    let dir = work_dir.join("some/very/deeply/nested/subdir/../../../../../..");
    let (repo_path, _trust) = gix_discover::upwards_opts(
        &dir,
        Options {
            match_ceiling_dir_or_error: false,
            ceiling_dirs: vec![Path::new("./some").into()],
            ..Default::default()
        },
    )
    .expect("the repo can be discovered because the relative ceiling doesn't _look_ like it has something to do with the git dir candidate");

    assert_ne!(
        &repo_path.as_ref().canonicalize()?,
        &work_dir,
        "a relative path that climbs above the test repo should yield the gitoxide repo"
    );

    Ok(())
}

#[test]
fn no_matching_ceiling_dirs_errors_by_default() -> crate::Result {
    let relative_work_dir = repo_path()?;
    let dir = relative_work_dir.join("some");
    let res = gix_discover::upwards_opts(
        &dir,
        Options {
            ceiling_dirs: vec!["/something/somewhere".into()],
            ..Default::default()
        },
    );

    let err = res.expect_err("an unrelated ceiling directory cannot match");
    assert!(
        err.downcast_any_ref::<gix_error::ValidationError>().is_some(),
        "the canonicalized ceiling dir doesn't have the same root as the git dir candidate, and can never match."
    );
    Ok(())
}

#[test]
fn ceilings_are_adjusted_to_match_search_dir() -> crate::Result {
    let relative_work_dir = repo_path()?;
    let cwd = std::env::current_dir()?;
    let absolute_ceiling_dir = gix_path::realpath_opts(&relative_work_dir, &cwd, 8)?;
    let dir = relative_work_dir.join("some");
    assert!(dir.is_relative());
    let (repo_path, _trust) = gix_discover::upwards_opts(
        &dir,
        Options {
            ceiling_dirs: vec![absolute_ceiling_dir],
            ..Default::default()
        },
    )?;
    assert_repo_is_current_workdir(repo_path, &relative_work_dir);

    assert!(relative_work_dir.is_relative());
    let absolute_dir = gix_path::realpath_opts(relative_work_dir.join("some").as_ref(), &cwd, 8)?;
    let (repo_path, _trust) = gix_discover::upwards_opts(
        &absolute_dir,
        Options {
            ceiling_dirs: vec![relative_work_dir.clone()],
            ..Default::default()
        },
    )?;
    assert_repo_is_current_workdir(repo_path, &relative_work_dir);
    Ok(())
}

#[test]
#[cfg(unix)]
fn ceiling_dirs_limit_the_physical_symlink_target() -> crate::Result {
    let root = gix_testtools::scripted_fixture_read_only("make_symlinked_nested_repo.sh")?;
    let search_dir = root.join("lexical-parent/link/real-dir");
    let err = gix_discover::upwards_opts(
        &search_dir,
        Options {
            ceiling_dirs: vec![root.join("subdir")],
            ..Default::default()
        },
    )
    .expect_err("the physical ceiling prevents discovery of the repository above it");

    assert!(
        err.downcast_any_ref::<gix_error::NotFoundError>().is_some(),
        "the symlink target matches the ceiling before traversal reaches the repository"
    );
    Ok(())
}
