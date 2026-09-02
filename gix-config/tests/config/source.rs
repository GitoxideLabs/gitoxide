use std::path::Path;

use gix_config::Source;

#[test]
fn git_config_no_system() {
    assert_eq!(
        Source::GitInstallation.storage_location(&mut |name| {
            assert_eq!(
                name, "GIT_CONFIG_NOSYSTEM",
                "it only checks this var, and if set, nothing else"
            );
            Some("1".into())
        }),
        None
    );
    assert!(
        Source::GitInstallation
            .storage_location(&mut |name| {
                match name {
                    "GIT_CONFIG_NOSYSTEM" => Some("false".into()),
                    "GIT_CONFIG_SYSTEM" => None,
                    _ => unreachable!("known set"),
                }
            })
            .is_some(),
        "it treats the variable as boolean"
    );
    assert_eq!(
        Source::System.storage_location(&mut |name| {
            assert_eq!(
                name, "GIT_CONFIG_NOSYSTEM",
                "it only checks this var, and if set, nothing else"
            );
            Some("1".into())
        }),
        None
    );
    assert!(
        Source::System
            .storage_location(&mut |name| {
                match name {
                    "GIT_CONFIG_NOSYSTEM" => Some("false".into()),
                    "GIT_CONFIG_SYSTEM" => None,
                    _ => unreachable!("known set"),
                }
            })
            .is_some()
    );
}

#[test]
fn git_config_system() {
    assert_eq!(
        Source::System
            .storage_location(&mut |name| {
                match name {
                    "GIT_CONFIG_NOSYSTEM" => None,
                    "GIT_CONFIG_SYSTEM" => Some("alternative".into()),
                    unexpected => unreachable!("unexpected env var: {unexpected}"),
                }
            })
            .expect("set"),
        Path::new("alternative"),
        "we respect the system config variable for overrides"
    );

    let default_installation = gix_path::env::installation_config().map(ToOwned::to_owned);
    let overridden_installation = Source::GitInstallation.storage_location(&mut |name| match name {
        "GIT_CONFIG_NOSYSTEM" => None,
        "GIT_CONFIG_SYSTEM" => Some("alternative".into()),
        unexpected => unreachable!("unexpected env var: {unexpected}"),
    });
    assert_eq!(
        overridden_installation,
        if gix_path::env::installation_config_is_system() {
            Some("alternative".into())
        } else {
            default_installation
        },
        "the override replaces system-scoped installation configuration but preserves other scopes"
    );
}

#[test]
fn git_config_global() {
    for source in [Source::Git, Source::User] {
        assert_eq!(
            source
                .storage_location(&mut |name| {
                    assert_eq!(
                        name, "GIT_CONFIG_GLOBAL",
                        "it only checks this var, and if set, nothing else"
                    );
                    Some("alternative".into())
                })
                .expect("set"),
            Path::new("alternative"),
            "we respect the global config variable for 'git' overrides"
        );
    }
}
