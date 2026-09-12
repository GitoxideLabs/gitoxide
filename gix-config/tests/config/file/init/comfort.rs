use gix_config::source;
use gix_testtools::Env;
use serial_test::serial;

#[test]
fn from_globals() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let worktree_dir = crate::scripted_fixture_read_only("make_config_repo.sh")?.canonicalize()?;
    let _env = Env::new().set(
        "GIT_CONFIG_GLOBAL",
        worktree_dir.join(".gitconfig").display().to_string(),
    );
    let config = gix_config::File::from_globals()?;
    assert!(!config.is_void(), "the fixture supplies global configuration");
    assert!(config.sections().all(|section| {
        let kind = section.meta().source.kind();
        kind != source::Kind::Repository && kind != source::Kind::Override
    }));
    Ok(())
}

#[test]
#[serial]
fn from_environment_overrides() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = Env::new().set("GIT_CONFIG_COUNT", "0");
    let config = gix_config::File::from_environment_overrides()?;
    assert!(config.is_void());
    Ok(())
}

#[test]
#[serial]
fn from_git_dir() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let worktree_dir = crate::scripted_fixture_read_only("make_config_repo.sh")?;
    let git_dir = worktree_dir.join(".git");
    let worktree_dir = worktree_dir.canonicalize()?;
    let _env = Env::new()
        .set(
            "GIT_CONFIG_SYSTEM",
            worktree_dir.join("system.config").display().to_string(),
        )
        .set("HOME", worktree_dir.display().to_string())
        .set("USERPROFILE", worktree_dir.display().to_string())
        .unset("GIT_CONFIG_GLOBAL")
        .unset("GIT_CONFIG_NOSYSTEM")
        .unset("XDG_CONFIG_HOME")
        .set("GIT_CONFIG_COUNT", "1")
        .set("GIT_CONFIG_KEY_0", "include.path")
        .set(
            "GIT_CONFIG_VALUE_0",
            worktree_dir.join("c.config").display().to_string(),
        );

    let config = gix_config::File::from_git_dir(git_dir)?;
    assert_eq!(
        config.string_by("a", None, "local").expect("present"),
        "value",
        "a value from the local repo configuration"
    );
    assert_eq!(config.string("a.local").expect("present"), "value");
    assert_eq!(
        config.string_by("a", None, "local-include").expect("present"),
        "from-a.config",
        "an override from a local repo include"
    );
    assert_eq!(
        config.string_by("a", None, "system").expect("present"),
        "from-system.config",
        "system configuration can be overridden with GIT_CONFIG_SYSTEM"
    );
    assert_eq!(
        config.string_by("a", None, "system-override").expect("present"),
        "from-b.config",
        "globals resolve their includes"
    );
    assert_eq!(
        config.string_by("a", None, "user").expect("present"),
        "from-user.config",
        "per-user configuration"
    );
    assert_eq!(
        config.string_by("env", None, "override").expect("present"),
        "from-c.config",
        "environment includes are resolved"
    );

    assert_eq!(
        config.string_by("a", None, "git").expect("present"),
        "git-application",
        "we load the XDG directories, based on the HOME fallback"
    );
    Ok(())
}

#[test]
#[serial]
fn from_git_dir_with_worktree_extension() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let git_dir = crate::scripted_fixture_read_only("config_with_worktree_extension.sh")?
        .join("main-worktree")
        .join(".git");
    let config = gix_config::File::from_git_dir(git_dir)?;

    assert_eq!(
        config
            .string_by("extensions", None, "worktreeConfig")
            .expect("extension present"),
        "true"
    );
    assert_eq!(
        config.string_by("worktree", None, "override").expect("section present"),
        "set in the main worktree"
    );

    Ok(())
}
