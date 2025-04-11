use crate::named_repo;
use crate::util::named_subrepo_opts;
use gix_sec::Permission;
use gix_testtools::Env;
use serial_test::serial;
use std::path::Path;

#[test]
#[serial]
fn author_included_by_hasconfig() -> crate::Result {
    let repo = named_subrepo_opts("make_config_repos.sh", "with-hasconfig", gix::open::Options::isolated())?;
    let _env = Env::new()
        .set(
            "GIT_CONFIG_SYSTEM",
            repo.git_dir().join("system.config").display().to_string(),
        )
        .unset("GIT_AUTHOR_NAME")
        .unset("GIT_COMMITTER_NAME");
    let repo = gix::open_opts(repo.git_dir(), allow_system_options(repo.open_options().clone()))?;
    let author = repo.author().expect("set in system config via include")?;
    assert_eq!(author.name, "works");
    assert_eq!(author.email, "works@example.com");
    Ok(())
}

#[test]
#[serial]
fn author_and_committer_and_fallback() -> crate::Result {
    for trust in [gix_sec::Trust::Full, gix_sec::Trust::Reduced] {
        let repo = named_repo("make_config_repo.sh")?;
        let work_dir = repo.workdir().expect("present").canonicalize()?;
        let _env = Env::new()
            .set(
                "GIT_CONFIG_SYSTEM",
                work_dir.join("system.config").display().to_string(),
            )
            .set("GIT_AUTHOR_NAME", "author")
            .set("GIT_AUTHOR_EMAIL", "author@email")
            .set("GIT_AUTHOR_DATE", "Thu, 1 Aug 2022 12:45:06 +0800")
            .set("GIT_COMMITTER_NAME", "committer-overrider-unused")
            .set("GIT_COMMITTER_EMAIL", "committer-override-unused@email")
            .set("GIT_COMMITTER_DATE", "Thu, 1 Aug 2022 12:45:06 -0200")
            .set("EMAIL", "general@email-unused")
            .set("GIT_CONFIG_COUNT", "1")
            .set("GIT_CONFIG_KEY_0", "include.path")
            .set("GIT_CONFIG_VALUE_0", work_dir.join("c.config").display().to_string());
        let repo = gix::open_opts(
            repo.git_dir(),
            allow_system_options(repo.open_options().clone().with(trust)),
        )?;

        assert_eq!(
            repo.author().expect("present")?,
            gix_actor::SignatureRef {
                name: "author".into(),
                email: "author@email".into(),
                time: "1659329106 +0800".into(),
            }
        );

        assert_eq!(
            repo.committer().expect("present")?,
            gix_actor::SignatureRef {
                name: "committer".into(),
                email: "committer@email".into(),
                time: "1659365106 -0200".into(),
            }
        );
        let config = repo.config_snapshot();

        assert_eq!(config.boolean("core.bare"), Some(false));
        assert_eq!(config.boolean("a.bad-bool"), None);
        assert_eq!(config.try_boolean("core.bare"), Some(Ok(false)));
        assert!(matches!(config.try_boolean("a.bad-bool"), Some(Err(_))));

        assert_eq!(config.integer("a.int"), Some(42));
        assert_eq!(config.integer("a.int-overflowing"), None);
        assert_eq!(config.integer("a.int-overflowing"), None);
        assert!(config.try_integer("a.int-overflowing").expect("present").is_err());

        assert_eq!(
            config.string("a.single-string").expect("present").as_ref(),
            "hello world"
        );

        assert_eq!(
            config.string("a.local-override").expect("present").as_ref(),
            "from-a.config"
        );
        assert_eq!(
            config.string("a.system").expect("present").as_ref(),
            "from-system.config"
        );
        assert_eq!(
            config.string("a.system-override").expect("present").as_ref(),
            "from-b.config"
        );

        assert_eq!(
            config.string("a.env-override").expect("present").as_ref(),
            "from-c.config"
        );

        assert_eq!(config.boolean("core.missing"), None);
        assert_eq!(config.try_boolean("core.missing"), None);

        let relative_path_key = "a.relative-path";
        if trust == gix_sec::Trust::Full {
            assert_eq!(
                config
                    .trusted_path(relative_path_key)
                    .expect("exists")
                    .expect("no error"),
                Path::new("./something")
            );
            assert_eq!(
                config
                    .trusted_path("a.absolute-path")
                    .expect("exists")
                    .expect("no error"),
                Path::new("/etc/man.conf")
            );
            assert!(config.trusted_path("a.bad-user-path").expect("exists").is_err());
        } else {
            assert!(
                config.trusted_path(relative_path_key).is_none(),
                "trusted paths need full trust"
            );
        }
    }
    Ok(())
}

#[test]
#[serial]
fn author_from_different_config_sections() -> crate::Result {
    let repo = named_repo("make_signatures_repo.sh")?;
    let work_dir = repo.workdir().unwrap().canonicalize()?;

    let _env = Env::new()
        .set("GIT_CONFIG_GLOBAL", work_dir.join("global.config").to_str().unwrap())
        .set("GIT_CONFIG_SYSTEM", work_dir.join("system.config").to_str().unwrap())
        .set("GIT_AUTHOR_DATE", "1979-02-26 18:30:00")
        .unset("GIT_AUTHOR_NAME")
        .set("GIT_COMMITTER_DATE", "1980-02-26 18:30:00 +0000")
        .unset("GIT_COMMITTER_NAME")
        .set("EMAIL", "general@email-unused");

    let repo = gix::open_opts(
        repo.git_dir(),
        allow_system_options(
            repo.open_options()
                .clone()
                .config_overrides(None::<&str>)
                .with(gix_sec::Trust::Full),
        ),
    )?;

    assert_eq!(
        repo.author().transpose()?,
        Some(gix_actor::SignatureRef {
            name: "global name".into(),
            email: "local@example.com".into(),
            time: "42 +0030".into()
        }),
        "author name comes from global config, \
         but email comes from repository-local config",
    );
    assert_eq!(
        repo.committer().transpose()?,
        Some(gix_actor::SignatureRef {
            name: "local committer".into(),
            email: "global-committer@example.com".into(),
            time: "320437800 +0000".into()
        }),
        "committer name comes from repository-local config, \
         but committer email comes from global config"
    );
    Ok(())
}

fn allow_system_options(opts: gix::open::Options) -> gix::open::Options {
    opts.permissions(gix::open::Permissions {
        env: gix::open::permissions::Environment {
            xdg_config_home: Permission::Deny,
            home: Permission::Deny,
            ..gix::open::permissions::Environment::all()
        },
        ..Default::default()
    })
}
