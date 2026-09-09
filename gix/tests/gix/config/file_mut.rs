use crate::repository::config::config_snapshot::options_with_includes;
use crate::{named_repo, repo_rw_opts};
use std::io::Write;

#[test]
fn locks_and_edits_one_physical_file_until_explicit_reload() -> crate::Result {
    let (mut repo, _tmp) = repo_rw_opts("make_config_repo.sh", options_with_includes())?;
    let git_config_path = repo.git_dir().join("config");
    let target_path = repo.workdir().expect("worktree repository").join("a.config");
    std::fs::write(
        &target_path,
        br#"# leading comment
leading = value
[a]
  local-override = included

[committer]
  name = committer
  email = committer@email
# trailing comment
"#,
    )?;
    std::fs::OpenOptions::new()
        .append(true)
        .open(&git_config_path)?
        .write_all(
            br#"
[include]
  path = ../a.config
"#,
        )?;
    repo.reload()?;
    let git_config_before = std::fs::read(&git_config_path)?;

    repo.config_snapshot_mut().set_raw_value("memory.value", "transient")?;
    let mut file = repo.config_file_mut(&target_path)?;
    let err = match repo.config_file_mut(&target_path) {
        Err(err) => err,
        Ok(_) => panic!("a second transaction must not acquire the same lock"),
    };
    assert!(
        err.can_retry(),
        "lock contention must be reported as retryable: {err:?}"
    );
    file.set_raw_value("a.local-override", "written")?;
    file.commit()?;

    assert_eq!(
        std::fs::read(&git_config_path)?,
        git_config_before,
        "the root file is untouched"
    );
    let written = std::fs::read_to_string(&target_path)?;
    insta::assert_snapshot!(written, "frontmatter + postmatter are preserved, it doesn't create new sections", @"
    # leading comment
    leading = value
    [a]
      local-override = written

    [committer]
      name = committer
      email = committer@email
    # trailing comment
    ");
    assert_eq!(
        repo.config_snapshot()
            .string("a.local-override")
            .expect("cached include value"),
        "included",
        "committing a file doesn't refresh the repository"
    );
    assert_eq!(
        repo.config_snapshot().string("memory.value").expect("in-memory value"),
        "transient",
        "the in-memory value is preserved as well, the repo doesn't change after all"
    );

    repo.reload()?;
    assert_eq!(
        repo.config_snapshot()
            .string("a.local-override")
            .expect("reloaded include value"),
        "written",
        "after a reload, the written value is observed"
    );
    assert!(
        repo.config_snapshot().string("memory.value").is_none(),
        "reload discards in-memory configuration"
    );
    Ok(())
}

#[test]
fn honors_core_config_lock_timeout() -> crate::Result {
    let (mut repo, _tmp) = repo_rw_opts("make_config_repo.sh", gix::open::Options::isolated())?;
    let config_path = repo.git_dir().join("config");
    let mut lock_path = config_path.as_os_str().to_owned();
    lock_path.push(".lock");
    let lock_path = std::path::PathBuf::from(lock_path);
    std::fs::write(&lock_path, b"held")?;
    let release_path = lock_path.clone();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::remove_file(release_path)
    });

    let file = repo.config_file_mut(&config_path);
    release.join().expect("lock-release thread does not panic")?;
    drop(file?);

    repo.config_snapshot_mut()
        .set_raw_value(gix::config::tree::Core::CONFIG_LOCK_TIMEOUT, "0")?;
    std::fs::write(&lock_path, b"held")?;
    let err = match repo.config_file_mut(config_path) {
        Ok(_) => panic!("a zero timeout must not wait for the lock"),
        Err(err) => err,
    };
    std::fs::remove_file(lock_path)?;
    assert!(
        err.can_retry(),
        "lock contention must be reported as retryable: {err:?}"
    );
    assert!(
        err.iter_errors()
            .any(|source| source.to_string().contains("immediately")),
        "the configured zero timeout must be retained in the error: {err:?}"
    );
    Ok(())
}

#[test]
fn preserves_permissions() -> crate::Result {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let (repo, _tmp) = repo_rw_opts("make_config_repo.sh", options_with_includes())?;
        let target_path = repo.workdir().expect("worktree repository").join("a.config");
        std::fs::set_permissions(&target_path, std::fs::Permissions::from_mode(0o600))?;
        let mut file = repo.config_file_mut(&target_path)?;
        file.set_raw_value("a.local-override", "written")?;
        file.commit()?;
        assert_eq!(
            std::fs::metadata(target_path)?.permissions().mode() & 0o777,
            0o600,
            "writing preserves file permissions"
        );
    }
    #[cfg(windows)]
    {
        let (repo, _tmp) = repo_rw_opts("make_config_repo.sh", options_with_includes())?;
        let target_path = repo.workdir().expect("worktree repository").join("a.config");
        let mut permissions = std::fs::metadata(&target_path)?.permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&target_path, permissions)?;

        let mut file = repo.config_file_mut(&target_path)?;
        file.set_raw_value("a.local-override", "written")?;
        file.commit()?;

        let mut permissions = std::fs::metadata(&target_path)?.permissions();
        let is_readonly = permissions.readonly();
        #[expect(clippy::permissions_set_readonly_false, reason = "this test only runs on Windows")]
        permissions.set_readonly(false);
        std::fs::set_permissions(&target_path, permissions)?;
        assert!(is_readonly, "writing preserves read-only file permissions");
    }
    Ok(())
}

#[test]
#[cfg(unix)]
fn follows_symlinked_configuration_files() -> crate::Result {
    use std::os::unix::fs::symlink;

    let repo = named_repo("make_basic_repo.sh")?;
    let dir = gix_testtools::tempfile::tempdir()?;
    let target = dir.path().join("target.config");
    let link = dir.path().join("link.config");
    std::fs::write(
        &target,
        br#"[core]
  abbrev = 7
"#,
    )?;
    symlink("target.config", &link)?;

    let mut file = repo.config_file_mut(&link)?;
    let err = match repo.config_file_mut(&target) {
        Err(err) => err,
        Ok(_) => panic!("the symlink and its target must use the same lock"),
    };
    assert!(err.can_retry(), "the target must report lock contention: {err:?}");
    file.set_raw_value("core.abbrev", "8")?;
    file.commit()?;

    assert!(
        link.symlink_metadata()?.file_type().is_symlink(),
        "the symlink isn't altered"
    );
    let config = gix_config::File::from_path_no_includes(target, gix_config::Source::Local)?;
    assert_eq!(
        config.string("core.abbrev").expect("updated value"),
        "8",
        "the write goes to the symlink target"
    );
    Ok(())
}

#[test]
fn supports_empty_and_missing_files() -> crate::Result {
    let repo = named_repo("make_basic_repo.sh")?;
    let dir = gix_testtools::tempfile::tempdir()?;
    let empty = dir.path().join("empty.config");
    std::fs::write(&empty, b"# retained\n")?;

    let mut file = repo.config_file_mut(&empty)?;
    file.set_raw_value("fresh.value", "first")?;
    file.commit()?;
    insta::assert_snapshot!(std::fs::read_to_string(&empty)?, "comment-only frontmatter is retained", @"
    # retained
    [fresh]
    	value = first
    ");

    let missing = dir.path().join("missing.config");
    let mut file = repo.config_file_mut(&missing)?;
    file.set_raw_value("fresh.value", "created")?;
    file.commit()?;
    let config = gix_config::File::from_path_no_includes(missing, gix_config::Source::Local)?;
    assert_eq!(config.string("fresh.value").expect("created value"), "created");
    Ok(())
}

#[test]
#[cfg(unix)]
fn applies_shared_repository_permissions_to_new_files() -> crate::Result {
    use std::os::unix::fs::PermissionsExt;

    let mut repo = named_repo("make_basic_repo.sh")?;
    repo.config_snapshot_mut()
        .set_raw_value(gix::config::tree::Core::SHARED_REPOSITORY, "group")?;
    let dir = gix_testtools::tempfile::tempdir()?;
    let path = dir.path().join("missing.config");
    let mut file = repo.config_file_mut(&path)?;
    file.set_raw_value("fresh.value", "created")?;
    file.commit()?;

    assert_eq!(
        path.metadata()?.permissions().mode() & 0o777,
        (0o666 & !gix_testtools::umask()) | 0o660,
        "new configuration files honor core.sharedRepository after applying the umask"
    );
    Ok(())
}

#[test]
fn semantic_validation_happens_on_reload() -> crate::Result {
    let (mut repo, _tmp) = repo_rw_opts("make_config_repo.sh", options_with_includes().strict_config(true))?;
    let original_format_version = repo.config_snapshot().integer("core.repositoryFormatVersion");
    let config_path = repo.git_dir().join("config");
    let mut file = repo.config_file_mut(&config_path)?;
    file.set_raw_value("core.repositoryFormatVersion", "2")?;
    file.commit()?;

    let disk = gix_config::File::from_path_no_includes(config_path, gix_config::Source::Local)?;
    assert_eq!(disk.integer("core.repositoryFormatVersion")?, Some(2));
    let err = match repo.reload() {
        Ok(_) => panic!("the persisted repository format must be rejected while reopening"),
        Err(err) => err,
    };
    assert!(err.is_validation(), "reload reports the semantic error: {err:?}");
    assert_eq!(
        err.probable_cause().to_string(),
        "Unsupported repository format version 2; only versions 0 and 1 are supported"
    );
    assert_eq!(
        repo.config_snapshot().integer("core.repositoryFormatVersion"),
        original_format_version,
        "a failed reload leaves the live repository unchanged"
    );
    Ok(())
}

#[test]
fn include_changes_take_effect_only_after_reload() -> crate::Result {
    let (mut repo, _tmp) = repo_rw_opts("make_config_repo.sh", options_with_includes())?;
    let replacement_path = repo.workdir().expect("worktree repository").join("replacement.config");
    std::fs::write(
        &replacement_path,
        br#"[replacement]
value = visible
"#,
    )?;

    let mut file = repo.config_file_mut(repo.git_dir().join("config"))?;
    file.set_raw_value("include.path", "../replacement.config")?;
    file.commit()?;
    assert!(
        repo.config_snapshot().string("replacement.value").is_none(),
        "commit leaves the cached include graph untouched"
    );

    repo.reload()?;
    assert_eq!(
        repo.config_snapshot()
            .string("replacement.value")
            .expect("replacement include"),
        "visible"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn paths_with_parent_components_retain_symlink_semantics() -> crate::Result {
    use std::os::unix::fs::symlink;

    let repo = named_repo("make_basic_repo.sh")?;
    let dir = gix_testtools::tempfile::tempdir()?;
    let logical = dir.path().join("logical");
    let physical = dir.path().join("physical");
    std::fs::create_dir(&logical)?;
    std::fs::create_dir_all(physical.join("child"))?;
    symlink(physical.join("child"), logical.join("link"))?;
    std::fs::write(
        logical.join("config"),
        br#"[target]
value = lexical
"#,
    )?;
    std::fs::write(
        physical.join("config"),
        br#"[target]
value = physical
"#,
    )?;

    let mut file = repo.config_file_mut(logical.join("link/../config"))?;
    file.set_raw_value("target.value", "written")?;
    file.commit()?;

    let lexical = gix_config::File::from_path_no_includes(logical.join("config"), gix_config::Source::Local)?;
    let physical = gix_config::File::from_path_no_includes(physical.join("config"), gix_config::Source::Local)?;
    assert_eq!(
        lexical.string("target.value").expect("lexical value"),
        "lexical",
        "`link/..` must not collapse to `logical`, leaving `logical/config` untouched"
    );
    assert_eq!(
        physical.string("target.value").expect("physical value"),
        "written",
        "following `link` before `..` resolves the write to `physical/config`"
    );
    Ok(())
}
