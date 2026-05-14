use std::{fs, path::Path, str::FromStr};

use gix_config::{
    File,
    file::{includes, init},
    path,
};
use gix_testtools::tempfile::tempdir;

use crate::file::{cow_str, init::from_paths::escape_backslashes};

mod gitdir;
mod hasconfig;
mod onbranch;

#[test]
fn include_and_includeif_correct_inclusion_order_and_delayed_resolve_include() -> crate::Result {
    let dir = tempdir()?;
    let config_path = dir.path().join("root");
    let first_include_path = dir.path().join("first-incl");
    let second_include_path = dir.path().join("second-incl");
    let include_if_path = dir.path().join("incl-if");
    fs::write(
        first_include_path.as_path(),
        "
; first include beginning
[section]
  value = first-incl-path
# first include end no nl",
    )?;

    fs::write(
        second_include_path.as_path(),
        "; second include beginning
[section]
  value = second-incl-path ; post value comment
# second include end
",
    )?;

    fs::write(
        include_if_path.as_path(),
        "
# includeIf beginning
[section]
  value = incl-if-path
; include if end no nl",
    )?;

    let root_config = format!(
        r#" ; root beginning
# root pre base
[section]
    value = base # base comment
; root post base    
[include]
  path = {}
  path = {} ; paths are multi-values
# root past first include
[section]
  value = base-past-first-include
# root before include-if no-nl
[includeIf "gitdir:root/"]
  path = {}
[section]
  value = base-past-includeIf
[include]
  path = {}
# root past last include
[section]
  value = base-past-second-include 
; root last include"#,
        escape_backslashes(&first_include_path),
        escape_backslashes(&first_include_path),
        escape_backslashes(&include_if_path),
        escape_backslashes(&second_include_path),
    );
    fs::write(config_path.as_path(), &root_config)?;

    let dir = config_path.join(".git");
    for delayed_resolve in [false, true] {
        let meta = gix_config::file::Metadata::try_from_path(&config_path, gix_config::Source::Api)?;
        let options = options_with_git_dir(&dir);
        let config = if delayed_resolve {
            let mut config = File::from_bytes_owned(&mut root_config.as_bytes().into(), meta, Default::default())?;
            config.resolve_includes(options)?;
            config
        } else {
            File::from_paths_metadata(Some(meta), options)?.expect("non-empty")
        };

        assert_eq!(
            config.strings_by("section", None, "value"),
            Some(vec![
                cow_str("base"),
                cow_str("first-incl-path"),
                cow_str("first-incl-path"),
                cow_str("base-past-first-include"),
                cow_str("incl-if-path"),
                cow_str("base-past-includeIf"),
                cow_str("second-incl-path"),
                cow_str("base-past-second-include"),
            ]),
            "include order isn't changed also in relation to the root configuration, delayed_resolve = {delayed_resolve}",
        );
        assert_eq!(config.sections().count(), 11);

        let config_string = config.to_string();
        let deserialized = File::from_str(&config_string)?;
        assert_eq!(config, config, "equality comparisons work");
        assert_eq!(
            deserialized.sections().count(),
            config.sections().count(),
            "sections must match to have a chance for equality"
        );
        assert_eq!(config, deserialized, "we can round-trip the information at least");
        assert_eq!(
            deserialized.to_string(),
            config_string,
            "serialization works exactly as before"
        );
    }
    Ok(())
}

fn options_with_git_dir(git_dir: &Path) -> init::Options<'_> {
    init::Options {
        includes: includes::Options::follow(
            path::interpolate::Context {
                home_dir: Some(git_dir.parent().unwrap()),
                ..Default::default()
            },
            includes::conditional::Context {
                git_dir: Some(git_dir),
                ..Default::default()
            },
        ),
        ..Default::default()
    }
}

fn git_init(dir: impl AsRef<std::path::Path>, bare: bool) -> crate::Result {
    let dir = dir.as_ref();
    let mut args = vec!["init"];
    if bare {
        args.push("--bare");
    }
    let output = std::process::Command::new(gix_path::env::exe_invocation())
        .args(args)
        .arg(dir)
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("XDG_CONFIG_HOME")
        .output()?;

    assert!(output.status.success(), "{output:?}, {dir:?}");
    Ok(())
}

fn create_symlink(from: impl AsRef<Path>, to: impl AsRef<Path>) {
    std::fs::create_dir_all(from.as_ref().parent().unwrap()).unwrap();
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(to, from).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(to, from).unwrap();
}
