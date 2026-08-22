use std::io::Write;

use gix::refs::{
    FullName, Target,
    transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
};

fn refname(value: &str) -> FullName {
    value.try_into().expect("test branch names are valid")
}

#[test]
fn deletes_a_batch_and_all_of_its_local_config_without_inspecting_commits() -> crate::Result {
    let (mut repo, _tmp) = crate::repo_rw("make_references_repo.sh")?;
    let direct = refname("refs/heads/delete-direct");
    let symbolic = refname("refs/heads/delete-symbolic");
    repo.reference(
        direct.clone(),
        repo.object_hash().empty_tree(),
        PreviousValue::MustNotExist,
        "create test branch",
    )?;
    repo.edit_reference(RefEdit {
        change: Change::Update {
            log: LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: true,
                message: "create broken symbolic test branch".into(),
            },
            expected: PreviousValue::MustNotExist,
            new: Target::Symbolic(refname("refs/heads/missing-target")),
        },
        name: symbolic.clone(),
        deref: false,
    })?;

    let included_path = repo.common_dir().join("included-config");
    std::fs::write(&included_path, b"[branch \"delete-direct\"]\n\tremote = elsewhere\n")?;
    let mut config = std::fs::OpenOptions::new()
        .append(true)
        .open(repo.common_dir().join("config"))?;
    let included_path_for_config = gix_path::to_unix_separators_on_windows(gix_path::into_bstr(&included_path));
    write!(
        config,
        "\n[branch \"delete-direct\"]\n\tremote = origin\n\
         [branch \"delete-direct\"]\n\tmerge = refs/heads/delete-direct\n\
         [branch \"delete-symbolic\"]\n\tdescription = remove me\n\
         [branch \"keep\"]\n\tremote = origin\n\
         [include]\n\tpath = {included_path_for_config}\n"
    )?;
    drop(config);
    let work_dir = repo
        .workdir()
        .expect("the reference fixture is a non-bare repository")
        .to_owned();
    let mut options = crate::restricted();
    options.permissions.config.includes = true;
    repo = gix::open_opts(work_dir, options)?;
    repo.config_snapshot_mut()
        .append_config(["test.inMemory=retained"], gix_config::Source::Api)?;

    repo.delete_local_branches([direct.clone(), symbolic.clone()])?;

    assert!(
        repo.try_find_reference(direct.as_ref())?.is_none(),
        "the direct branch is gone"
    );
    assert!(
        repo.try_find_reference(symbolic.as_ref())?.is_none(),
        "broken symbolic branches are deleted without peeling"
    );
    let config = std::fs::read_to_string(repo.common_dir().join("config"))?;
    assert!(!config.contains("delete-direct"), "all duplicate sections are removed");
    assert!(
        !config.contains("delete-symbolic"),
        "the symbolic branch section is removed"
    );
    assert!(
        config.contains("branch \"keep\""),
        "unrelated local configuration remains"
    );
    assert!(
        config.contains("[include]"),
        "include directives remain in the local file"
    );
    assert_eq!(
        std::fs::read_to_string(&included_path)?,
        "[branch \"delete-direct\"]\n\tremote = elsewhere\n",
        "included files are not rewritten"
    );
    assert_eq!(
        repo.branch_names().into_iter().collect::<Vec<_>>(),
        vec!["delete-direct", "keep"],
        "only configuration from the local file are deleted, but this one is from the included file"
    );
    let config = repo.config_snapshot();
    assert!(
        config
            .string("test.inMemory")
            .is_some_and(|value| value.as_slice() == b"retained"),
        "the deleting repository instance retains unrelated in-memory-only configuration, proving it wasn't reloaded"
    );
    Ok(())
}

#[test]
fn validation_failure_leaves_the_entire_batch_unchanged() -> crate::Result {
    let (mut repo, _tmp) = crate::repo_rw("make_references_repo.sh")?;
    let work_dir = repo
        .workdir()
        .expect("the reference fixture is a non-bare repository")
        .to_owned();
    let deletable = refname("refs/heads/d1");
    let checked_out = refname("refs/heads/main");

    let err = repo
        .delete_local_branches([deletable.clone(), checked_out.clone()])
        .expect_err("the checked-out branch prevents the whole batch");
    assert_eq!(
        err.to_string(),
        format!("The local branch {checked_out:?} is checked out in [{work_dir:?}]")
    );
    assert!(repo.try_find_reference(deletable.as_ref())?.is_some());

    let tag = refname("refs/tags/t1");
    let err = repo
        .delete_local_branches([tag.clone()])
        .expect_err("non-local references are rejected");
    assert_eq!(err.to_string(), format!("{tag:?} is not a local branch"));
    assert!(repo.try_find_reference(tag.as_ref())?.is_some());
    Ok(())
}

#[test]
fn missing_branches_are_successful_and_their_config_is_removed() -> crate::Result {
    let (mut repo, _tmp) = crate::repo_rw("make_references_repo.sh")?;
    let existing = refname("refs/heads/d1");
    let missing = refname("refs/heads/does-not-exist");
    let reserved_for_creation = refname("refs/heads/HEAD");
    let mut config = std::fs::OpenOptions::new()
        .append(true)
        .open(repo.common_dir().join("config"))?;
    write!(
        config,
        "\n[branch \"d1\"]\n\tremote = origin\n\
         [branch \"does-not-exist\"]\n\tremote = origin\n\
         [branch \"HEAD\"]\n\tremote = origin\n"
    )?;
    drop(config);
    repo.reload()?;
    assert_eq!(
        repo.branch_names().into_iter().collect::<Vec<_>>(),
        vec!["HEAD", "d1", "does-not-exist"],
        "all branch sections are loaded before deletion"
    );

    repo.delete_local_branches([existing.clone(), missing.clone(), reserved_for_creation.clone()])?;

    assert!(
        repo.try_find_reference(existing.as_ref())?.is_none(),
        "the existing branch is deleted"
    );
    assert!(
        repo.try_find_reference(missing.as_ref())?.is_none(),
        "the missing branch remains absent"
    );
    assert!(
        repo.try_find_reference(reserved_for_creation.as_ref())?.is_none(),
        "a valid full name isn't subjected to additional creation-time validation"
    );
    let config = std::fs::read_to_string(repo.common_dir().join("config"))?;
    assert!(
        !config.contains("branch \"d1\""),
        "existing branch configuration is removed"
    );
    assert!(
        !config.contains("branch \"does-not-exist\""),
        "missing branch configuration is removed"
    );
    assert!(
        !config.contains("branch \"HEAD\""),
        "configuration is removed even for names reserved only during branch creation"
    );
    assert!(
        repo.branch_names().is_empty(),
        "configuration for existing, missing, and creation-reserved branch names is removed from memory"
    );
    Ok(())
}

#[test]
fn linked_worktree_branches_are_protected_and_common_config_is_updated() -> crate::Result {
    // `git worktree add --relative-paths`, used by the fixture, was added in Git 2.48.
    let Some(fixture) = gix_testtools::scripted_fixture_writable_with_args_with_git_version(
        "make_worktree_relative_linking.sh",
        None::<String>,
        gix_testtools::Creation::Execute,
        |version| version >= (2, 48, 0),
    )?
    else {
        return Ok(());
    };
    let main_path = fixture.path().join("main");
    let linked_path = fixture.path().join("linked");
    let main = gix::open_opts(&main_path, crate::restricted())?;
    let target = main.head_id()?.detach();
    let checked_out = refname("refs/heads/linked");
    let deletable = refname("refs/heads/delete-from-linked");
    main.reference(
        checked_out.clone(),
        target,
        PreviousValue::MustNotExist,
        "create branch to be checked out by the linked worktree",
    )?;
    main.reference(
        deletable.clone(),
        target,
        PreviousValue::MustNotExist,
        "create deletable branch",
    )?;
    let linked_repo = gix::open_opts(&linked_path, crate::restricted())?;
    std::fs::write(linked_repo.git_dir().join("HEAD"), b"ref: refs/heads/linked\n")?;

    let mut main = gix::open_opts(&main_path, crate::restricted())?;
    let linked_work_dir = main
        .worktree_proxy_by_id("linked")
        .expect("the linked worktree is registered")
        .base()?;
    let err = main
        .delete_local_branches([checked_out.clone()])
        .expect_err("a linked worktree checkout is protected");
    assert_eq!(
        err.to_string(),
        format!("The local branch {checked_out:?} is checked out in [{linked_work_dir:?}]")
    );

    let mut config = std::fs::OpenOptions::new()
        .append(true)
        .open(main.common_dir().join("config"))?;
    write!(config, "\n[branch \"delete-from-linked\"]\n\tremote = origin\n")?;
    drop(config);
    let mut linked_repo = gix::open_opts(&linked_path, crate::restricted())?;
    assert!(
        linked_repo.branch_names().contains("delete-from-linked"),
        "the common branch configuration is loaded into the linked repository"
    );
    linked_repo.delete_local_branches([deletable.clone()])?;
    assert!(linked_repo.try_find_reference(deletable.as_ref())?.is_none());
    assert!(
        !std::fs::read_to_string(main.common_dir().join("config"))?.contains("delete-from-linked"),
        "linked worktrees rewrite the common configuration"
    );
    assert!(
        !linked_repo.branch_names().contains("delete-from-linked"),
        "the linked repository's in-memory configuration is updated"
    );
    Ok(())
}
