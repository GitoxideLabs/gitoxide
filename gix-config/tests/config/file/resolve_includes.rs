use gix_config::{file, file::init};

#[test]
fn missing_includes_are_ignored_by_default() -> crate::Result {
    let input = r#"
        [include]
            path = /etc/absolute/missing.config
            path = relative-missing.config
            path = ./also-relative-missing.config
            path = %(prefix)/no-install.config
            path = ~/no-user.config
            
        [includeIf "onbranch:no-branch"]
            path = no-branch-provided.config
        [includeIf "gitdir:./no-git-dir"]
            path = no-git-dir.config
    "#;

    let mut config: gix_config::File = input.parse()?;

    let mut follow_options = file::includes::Options::follow(Default::default(), Default::default());
    follow_options.err_on_missing_config_path = false;
    config.resolve_includes(init::Options {
        includes: follow_options,
        ..Default::default()
    })?;

    assert!(
        config
            .resolve_includes(init::Options {
                includes: follow_options.strict(),
                ..Default::default()
            })
            .is_err(),
        "strict mode fails if something couldn't be interpolated"
    );
    Ok(())
}

#[test]
fn observer_reports_active_missing_empty_and_nested_includes() -> crate::Result {
    let dir = gix_testtools::tempfile::tempdir()?;
    let root = dir.path().join("config");
    std::fs::write(
        &root,
        "[include]\npath = missing\npath = empty\npath = nested\n\
         [includeIf \"onbranch:main\"]\npath = active\n\
         [includeIf \"onbranch:other\"]\npath = inactive\n",
    )?;
    std::fs::write(dir.path().join("empty"), "")?;
    std::fs::write(dir.path().join("nested"), "[include]\npath = nested-missing\n")?;
    let branch_name: gix_ref::FullName = "refs/heads/main".try_into()?;
    let mut config = gix_config::File::from_path_no_includes(root, gix_config::Source::Local)?;
    let mut observed = Vec::new();
    config.resolve_includes_with_observer(
        init::Options {
            includes: file::includes::Options::follow(
                Default::default(),
                file::includes::conditional::Context {
                    branch_name: Some(branch_name.as_ref()),
                    ..Default::default()
                },
            ),
            ..Default::default()
        },
        |path| observed.push(path.to_owned()),
    )?;
    assert_eq!(
        observed,
        ["missing", "empty", "nested", "nested-missing", "active"].map(|name| dir.path().join(name)),
        "observation follows actual include resolution, even without parsed sections in the target"
    );
    Ok(())
}

#[test]
fn observer_respects_disabled_resolution_and_missing_interpolation() -> crate::Result {
    let mut config: gix_config::File = "[include]\npath = ~/missing\n".parse()?;
    let mut observed = Vec::new();
    config.resolve_includes_with_observer(Default::default(), |path| observed.push(path.to_owned()))?;
    config.resolve_includes_with_observer(
        init::Options {
            includes: file::includes::Options::follow(Default::default(), Default::default()),
            ..Default::default()
        },
        |path| observed.push(path.to_owned()),
    )?;
    assert!(
        observed.is_empty(),
        "disabled includes and paths which cannot be interpolated produce no filesystem dependency"
    );
    Ok(())
}
