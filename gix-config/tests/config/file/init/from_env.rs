use std::{borrow::Cow, fs};

use gix_config::{
    File,
    file::{includes, init, init::from_env},
};
use gix_testtools::{Env, tempfile::tempdir};
use serial_test::serial;

use crate::file::init::from_paths::escape_backslashes;

#[test]
#[serial]
fn empty_without_relevant_environment() {
    let config = File::from_env(Default::default()).unwrap();
    assert!(config.is_none());
}

#[test]
#[serial]
fn empty_with_zero_count() {
    let _env = Env::new().set("GIT_CONFIG_COUNT", "0");
    let config = File::from_env(Default::default()).unwrap();
    assert!(config.is_none());
}

#[test]
#[serial]
fn parse_error_with_invalid_count() {
    let _env = Env::new().set("GIT_CONFIG_COUNT", "invalid");
    let err = File::from_env(Default::default()).unwrap_err();
    assert!(matches!(err, from_env::Error::InvalidConfigCount { .. }));
}

#[test]
#[serial]
fn single_key_value_pair() -> crate::Result {
    let _env = Env::new()
        .set("GIT_CONFIG_COUNT", "1")
        .set("GIT_CONFIG_KEY_0", "core.key")
        .set("GIT_CONFIG_VALUE_0", "value");

    let config = File::from_env(Default::default())?.unwrap();
    assert_eq!(config.raw_value("core.key")?, Cow::<[u8]>::Borrowed(b"value"));
    assert_eq!(
        config.section_by_key("core".into())?.meta(),
        &gix_config::file::Metadata::from(gix_config::Source::Env),
        "source if configured correctly"
    );
    assert_eq!(config.num_values(), 1);
    Ok(())
}

#[test]
#[serial]
fn multiple_key_value_pairs() {
    let _env = Env::new()
        .set("GIT_CONFIG_COUNT", "3")
        .set("GIT_CONFIG_KEY_0", "core.a")
        .set("GIT_CONFIG_VALUE_0", "a")
        .set("GIT_CONFIG_KEY_1", "core.b")
        .set("GIT_CONFIG_VALUE_1", "b")
        .set("GIT_CONFIG_KEY_2", "core.c")
        .set("GIT_CONFIG_VALUE_2", "c");

    let config = File::from_env(Default::default()).unwrap().unwrap();

    assert_eq!(config.raw_value("core.a").unwrap(), Cow::<[u8]>::Borrowed(b"a"));
    assert_eq!(config.raw_value("core.b").unwrap(), Cow::<[u8]>::Borrowed(b"b"));
    assert_eq!(config.raw_value("core.c").unwrap(), Cow::<[u8]>::Borrowed(b"c"));
    assert_eq!(config.num_values(), 3);
}

#[test]
#[serial]
fn error_on_relative_paths_in_include_paths() {
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
}

#[test]
#[serial]
fn follow_include_paths() {
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

    assert_eq!(config.raw_value("core.key").unwrap(), Cow::<[u8]>::Borrowed(b"changed"));
    assert_eq!(config.num_values(), 5);
}
