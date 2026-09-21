use std::path::Path;

use gix_sec::Permission;
use gix_testtools::{Env, tempfile::tempdir};

use super::{Cache, StageOne};
use crate::open::Permissions;

fn cache(directory: &Path, permissions: Permissions) -> gix_testtools::Result<Cache> {
    let branch_name: gix_ref::FullName = "refs/heads/main".try_into()?;
    Ok(Cache::from_stage_one(
        StageOne::new(directory, directory, gix_sec::Trust::Full, false, false)?,
        directory,
        Some(branch_name.as_ref()),
        crate::config::section::is_trusted,
        None,
        None,
        permissions.env,
        permissions.attributes,
        permissions.config,
        false,
        &[],
        &[],
        false,
    )?)
}

#[test]
fn configuration_sources_preserve_missing_and_empty_roots() -> gix_testtools::Result {
    let dir = tempdir()?;
    let config_path = dir.path().join("config");
    for contents in [None, Some("")] {
        if let Some(contents) = contents {
            std::fs::write(&config_path, contents)?;
        }
        let cache = cache(dir.path(), Permissions::isolated())?;
        assert_eq!(
            cache.source_paths,
            std::slice::from_ref(&config_path),
            "roots do not require a parsed section"
        );
    }
    std::fs::write(&config_path, "[extensions]\nworktreeConfig = true\n")?;
    let cache = cache(dir.path(), Permissions::isolated())?;
    assert_eq!(
        cache.source_paths,
        [config_path, dir.path().join("config.worktree")],
        "enabled worktree configuration remains a dependency while missing"
    );
    Ok(())
}

#[test]
fn configuration_sources_obey_permissions_and_include_conditions() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let dir = tempdir()?;
    let system = dir.path().join("system");
    let xdg = dir.path().join("xdg");
    std::fs::create_dir_all(xdg.join("git"))?;
    std::fs::write(xdg.join("git/config"), "")?;
    std::fs::write(
        dir.path().join("config"),
        "[include]\npath = missing\n[includeIf \"onbranch:main\"]\npath = active\n\
         [includeIf \"onbranch:other\"]\npath = inactive\n",
    )?;
    let _env = Env::new()
        .unset("GIT_CONFIG_NOSYSTEM")
        .unset("GIT_CONFIG_GLOBAL")
        .set("GIT_CONFIG_SYSTEM", system.to_str().expect("UTF-8 temporary path"))
        .set("XDG_CONFIG_HOME", xdg.to_str().expect("UTF-8 temporary path"));
    let mut permissions = Permissions::isolated();
    permissions.env.git_prefix = Permission::Allow;
    permissions.env.xdg_config_home = Permission::Allow;
    permissions.config.system = true;
    permissions.config.git = true;
    permissions.config.includes = true;
    let cache = cache(dir.path(), permissions)?;
    let mut expected = [
        dir.path().join("config"),
        system,
        xdg.join("git/config"),
        dir.path().join("missing"),
        dir.path().join("active"),
    ];
    expected.sort();
    assert_eq!(
        cache.source_paths, expected,
        "allowed missing roots and active includes are kept; denied sources and inactive includes are absent"
    );
    let isolated = self::cache(dir.path(), Permissions::isolated())?;
    assert_eq!(
        isolated.source_paths,
        [dir.path().join("config")],
        "disabling include resolution also disables observing include targets"
    );
    Ok(())
}

#[test]
fn environment_include_conditions_are_resolved_separately() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let dir = tempdir()?;
    std::fs::write(dir.path().join("config"), "[remote \"origin\"]\nurl = local\n")?;
    let included = dir.path().join("environment-include");
    let inactive = dir.path().join("must-not-include");
    let _env = Env::new()
        .set("GIT_CONFIG_COUNT", "2")
        .set("GIT_CONFIG_KEY_0", "include.path")
        .set("GIT_CONFIG_VALUE_0", included.to_str().expect("UTF-8 temporary path"))
        .set("GIT_CONFIG_KEY_1", "includeIf.hasconfig:remote.*.url:local.path")
        .set("GIT_CONFIG_VALUE_1", inactive.to_str().expect("UTF-8 temporary path"));
    let mut permissions = Permissions::isolated();
    permissions.config.env = true;
    permissions.config.includes = true;
    let cache = cache(dir.path(), permissions)?;
    assert_eq!(
        cache.source_paths,
        [dir.path().join("config"), included],
        "environment includes must not gain access to remote URLs from file configuration"
    );
    Ok(())
}

#[test]
fn optional_pattern_sources_keep_configured_missing_paths_and_default_selection() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let dir = tempdir()?;
    let xdg = dir.path().join("xdg");
    let _env = Env::new().set("XDG_CONFIG_HOME", xdg.to_str().expect("UTF-8 temporary path"));
    let mut permissions = Permissions::isolated();
    permissions.env.xdg_config_home = Permission::Allow;
    permissions.attributes.git = true;
    let cache = cache(dir.path(), permissions)?;
    assert_eq!(
        cache.excludes_file(&mut |_| {})?,
        Some(xdg.join("git/ignore")),
        "unset ignore source uses XDG"
    );
    assert_eq!(
        cache
            .attribute_global_paths(permissions.attributes, &mut |_| {})?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        [xdg.join("git/attributes")],
        "allowed default attributes are dependencies even while absent"
    );

    std::fs::write(
        dir.path().join("config"),
        "[core]\nexcludesFile = missing-ignore\nattributesFile = missing-attributes\n",
    )?;
    let cache = self::cache(dir.path(), Permissions::isolated())?;
    assert_eq!(
        cache.excludes_file(&mut |_| {})?,
        Some("missing-ignore".into()),
        "a configured missing ignore file suppresses the default"
    );
    assert_eq!(
        cache
            .attribute_global_paths(permissions.attributes, &mut |_| {})?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        [std::path::PathBuf::from("missing-attributes")],
        "configured attributes suppress the default without checking existence"
    );
    Ok(())
}

#[test]
fn optional_pattern_sources_preserve_permission_semantics() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let dir = tempdir()?;
    let _env = Env::new().set(
        "XDG_CONFIG_HOME",
        dir.path().join("xdg").to_str().expect("UTF-8 temporary path"),
    );
    let mut permissions = Permissions::isolated();
    permissions.env.xdg_config_home = Permission::Forbid;
    permissions.attributes.git = true;
    let cache = cache(dir.path(), permissions)?;
    assert!(
        cache.excludes_file(&mut |_| {}).is_err(),
        "ignore lookup reports forbidden fallback access"
    );
    assert_eq!(
        cache
            .attribute_global_paths(permissions.attributes, &mut |_| {})?
            .into_iter()
            .flatten()
            .count(),
        0,
        "attribute lookup tolerates forbidden fallback access"
    );
    Ok(())
}

#[test]
fn optional_pattern_candidates_are_observed_before_using_the_fallback() -> gix_testtools::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let dir = tempdir()?;
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(dir.path())?;
    let xdg = dir.path().join("xdg");
    let _env = Env::new().set("XDG_CONFIG_HOME", xdg.to_str().expect("UTF-8 temporary path"));
    std::fs::write(
        dir.path().join("config"),
        "[core]\nexcludesFile = :(optional)missing-ignore\nattributesFile = :(optional)missing-attributes\n",
    )?;
    let mut permissions = Permissions::isolated();
    permissions.env.xdg_config_home = Permission::Allow;
    permissions.attributes.git = true;
    let cache = cache(dir.path(), permissions)?;
    let mut ignores = Vec::new();
    assert_eq!(
        cache.excludes_file(&mut |path| ignores.push(path.to_owned()))?,
        Some(xdg.join("git/ignore")),
        "the missing optional ignore file still selects the default"
    );
    assert_eq!(
        ignores,
        ["missing-ignore".into(), xdg.join("git/ignore")],
        "creating the optional candidate or editing the fallback must both invalidate patterns"
    );
    let mut attributes = Vec::new();
    assert_eq!(
        cache
            .attribute_global_paths(permissions.attributes, &mut |path| attributes.push(path.to_owned()))?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        [xdg.join("git/attributes")],
        "the missing optional attribute file still selects the default"
    );
    assert_eq!(
        attributes,
        ["missing-attributes".into(), xdg.join("git/attributes")],
        "attribute candidates are observed before the optional existence check"
    );

    std::fs::write(dir.path().join("missing-ignore"), "")?;
    std::fs::write(dir.path().join("missing-attributes"), "")?;
    assert_eq!(
        cache.excludes_file(&mut |_| {})?,
        Some("missing-ignore".into()),
        "the same selection logic starts using an optional candidate once it exists"
    );
    assert_eq!(
        cache
            .attribute_global_paths(permissions.attributes, &mut |_| {})?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>(),
        [std::path::PathBuf::from("missing-attributes")],
        "an existing optional attribute file suppresses the default"
    );
    std::env::set_current_dir(original_dir)?;
    Ok(())
}
