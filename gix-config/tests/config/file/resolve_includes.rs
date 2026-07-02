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
