use std::fs;

use gix_config::{
    File,
    file::{includes, init, init::from_env},
};
use gix_testtools::{Env, tempfile::tempdir};
use serial_test::serial;

use crate::file::init::from_paths::escape_backslashes;

#[test]
#[serial]
fn empty_without_relevant_environment() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = Env::new().unset("GIT_CONFIG_COUNT");
    let config = File::from_env(Default::default())?;
    assert!(config.is_none());
    Ok(())
}

#[test]
#[serial]
fn empty_with_zero_count() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = Env::new().set("GIT_CONFIG_COUNT", "0");
    let config = File::from_env(Default::default())?;
    assert!(config.is_none());
    Ok(())
}

#[test]
#[serial]
fn parse_error_with_invalid_count() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = Env::new().set("GIT_CONFIG_COUNT", "invalid");
    let err = File::from_env(Default::default()).unwrap_err();
    assert!(matches!(err, from_env::Error::InvalidConfigCount { .. }));
    Ok(())
}

#[test]
#[serial]
fn single_key_value_pair() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = Env::new()
        .set("GIT_CONFIG_COUNT", "1")
        .set("GIT_CONFIG_KEY_0", "core.key")
        .set("GIT_CONFIG_VALUE_0", "value");

    let config = File::from_env(Default::default())?.unwrap();
    assert_eq!(config.raw_value("core.key")?, "value");
    assert_eq!(
        config.section_by_key("core")?.meta(),
        &gix_config::file::Metadata::from(gix_config::Source::Env),
        "source if configured correctly"
    );
    assert_eq!(config.num_values(), 1);
    Ok(())
}

#[test]
#[serial]
fn multiple_key_value_pairs() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = Env::new()
        .set("GIT_CONFIG_COUNT", "3")
        .set("GIT_CONFIG_KEY_0", "core.a")
        .set("GIT_CONFIG_VALUE_0", "a")
        .set("GIT_CONFIG_KEY_1", "core.b")
        .set("GIT_CONFIG_VALUE_1", "b")
        .set("GIT_CONFIG_KEY_2", "core.c")
        .set("GIT_CONFIG_VALUE_2", "c");

    let config = File::from_env(Default::default()).unwrap().unwrap();

    assert_eq!(config.raw_value("core.a").unwrap(), "a");
    assert_eq!(config.raw_value("core.b").unwrap(), "b");
    assert_eq!(config.raw_value("core.c").unwrap(), "c");
    assert_eq!(config.num_values(), 3);
    Ok(())
}

#[test]
#[serial]
fn error_on_relative_paths_in_include_paths() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = Env::new()
        .set("GIT_CONFIG_COUNT", "1")
        .set("GIT_CONFIG_KEY_0", "include.path")
        .set("GIT_CONFIG_VALUE_0", "some_git_config");

    let res = File::from_env(init::Options {
        includes: includes::Options {
            max_depth: 1,
            ..Default::default()
        }
        .strict(),
        ..Default::default()
    });
    assert!(matches!(
        res,
        Err(from_env::Error::Includes(includes::Error::MissingConfigPath))
    ));
    Ok(())
}

#[test]
#[serial]
fn follow_include_paths() -> crate::Result {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let dir = tempdir().unwrap();
    let a_path = dir.path().join("a");
    fs::write(&a_path, "[core]\nkey = changed").unwrap();
    let b_path = dir.path().join("b");
    fs::write(&b_path, "[core]\nkey = invalid").unwrap();

    let _env = Env::new()
        .set("GIT_CONFIG_COUNT", "4")
        .set("GIT_CONFIG_KEY_0", "core.key")
        .set("GIT_CONFIG_VALUE_0", "value")
        .set("GIT_CONFIG_KEY_1", "include.path")
        .set("GIT_CONFIG_VALUE_1", escape_backslashes(a_path))
        .set("GIT_CONFIG_KEY_2", "other.path")
        .set("GIT_CONFIG_VALUE_2", escape_backslashes(&b_path))
        .set("GIT_CONFIG_KEY_3", "include.origin.path")
        .set("GIT_CONFIG_VALUE_3", escape_backslashes(b_path));

    let config = File::from_env(init::Options {
        includes: includes::Options {
            max_depth: 1,
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap()
    .unwrap();

    assert_eq!(config.raw_value("core.key").unwrap(), "changed");
    assert_eq!(config.num_values(), 5);
    Ok(())
}
