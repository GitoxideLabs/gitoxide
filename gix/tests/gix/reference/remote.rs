use crate::remote;
use gix_testtools::TestResult;

#[test]
fn push_defaults_to_fetch() -> TestResult {
    let repo = remote::repo("many-fetchspecs");
    let head = repo.head()?;
    let branch = head.clone().try_into_referent().expect("history");
    assert_eq!(
        branch
            .remote_name(gix::remote::Direction::Push)
            .expect("fallback to fetch"),
        branch.remote_name(gix::remote::Direction::Fetch).expect("configured"),
        "push falls back to fetch"
    );
    assert_eq!(
        branch
            .remote(gix::remote::Direction::Push)
            .expect("configured")?
            .name()
            .expect("set"),
        "origin"
    );
    assert_eq!(
        head.into_remote(gix::remote::Direction::Push)
            .expect("same with branch")?
            .name()
            .expect("set"),
        "origin"
    );
    Ok(())
}

#[test]
fn separate_push_and_fetch() -> TestResult {
    for name in ["push-default", "branch-push-remote"] {
        let repo = remote::repo(name);
        let head = repo.head()?;
        let branch = head.clone().try_into_referent().expect("history");

        assert_eq!(branch.remote_name(gix::remote::Direction::Push).expect("set"), "myself");
        assert_eq!(
            branch.remote_name(gix::remote::Direction::Fetch).expect("set"),
            "new-origin"
        );

        assert_ne!(
            branch.remote(gix::remote::Direction::Push).transpose()?,
            branch.remote(gix::remote::Direction::Fetch).transpose()?
        );
        assert_ne!(
            head.clone().into_remote(gix::remote::Direction::Push).transpose()?,
            head.into_remote(gix::remote::Direction::Fetch).transpose()?
        );
    }
    Ok(())
}

#[test]
fn not_configured() -> TestResult {
    let repo = remote::repo("base");
    let head = repo.head()?;
    let branch = head.clone().try_into_referent().expect("history");

    assert_eq!(branch.remote_name(gix::remote::Direction::Push), None);
    assert_eq!(branch.remote_name(gix::remote::Direction::Fetch), None);
    assert_eq!(branch.remote(gix::remote::Direction::Fetch).transpose()?, None);
    assert_eq!(head.into_remote(gix::remote::Direction::Fetch).transpose()?, None);
    let err = repo.find_fetch_remote(None).unwrap_err();
    assert!(err.is_not_found(), "there is no remote to be found");
    insta::assert_debug_snapshot!(err.probable_cause(), "not configured", @r#"
    Message {
        message: "No configured remote could be found, or too many were available",
        class: NotFound,
    }
    "#);

    Ok(())
}

#[test]
fn dot_remote_without_a_section_resolves_to_the_repository_itself() -> TestResult {
    use gix::remote::Direction;

    let repo = remote::repo("branch-dot-remote");
    let branch = repo.head_ref()?.expect("the repository has a branch");
    let git_dir = gix::path::into_bstr(repo.current_dir().join(repo.git_dir()))?.into_owned();

    assert!(repo.try_find_remote(".").is_none(), "there is no dot subsection");
    for direction in [Direction::Fetch, Direction::Push] {
        assert_eq!(
            repo.branch_remote_name("main", direction).expect("dot is configured"),
            "."
        );
        assert_eq!(branch.remote_name(direction), None, "dot isn't a symbolic name");

        let remote = branch
            .remote(direction)
            .transpose()?
            .expect("dot can be used as a remote");
        let url = remote.url(direction).expect("dot resolves to a usable URL");
        assert_eq!(remote.name(), None, "there is no configured remote section");
        assert_eq!(remote.configured_name(), None);
        assert_eq!(url.scheme, gix::url::Scheme::File);
        assert_eq!(url.path, git_dir, "the destination is this repository's Git directory");
    }
    let literal_path = repo.remote_at(".")?;
    assert_eq!(
        literal_path.url(Direction::Fetch).expect("fetch URL").path,
        ".",
        "an explicit path supplied to remote_at is still relative"
    );
    Ok(())
}

#[test]
fn dot_remote_uses_configured_refspecs_and_explicit_urls() -> TestResult {
    use gix::remote::Direction::{Fetch, Push};

    let fetch_spec = "refs/heads/main:refs/tracking/main";
    let push_spec = "refs/heads/main:refs/heads/published";
    let explicit_fetch_url = Some("https://example.com/override.git");
    let explicit_push_url = Some("ssh://example.com/publish.git");
    for (fetch_url, push_url) in [
        (None, None),
        (None, explicit_push_url),
        (explicit_fetch_url, None),
        (explicit_fetch_url, explicit_push_url),
    ] {
        let mut config = vec![
            format!("remote...fetch={fetch_spec}"),
            format!("remote...push={push_spec}"),
        ];
        if let Some(url) = fetch_url {
            config.push(format!("remote...url={url}"));
        }
        if let Some(url) = push_url {
            config.push(format!("remote...pushurl={url}"));
        }
        let repo = gix::open_opts(
            remote::repo_path("branch-dot-remote"),
            gix::open::Options::isolated().config_overrides(config),
        )?;
        let branch = repo.head_ref()?.expect("the repository has a branch");
        let remote = repo.find_remote(".")?;
        assert_eq!(remote.name(), None, "a dot subsection is not a validated name");
        assert_eq!(remote.configured_name(), Some(&gix::remote::ConfiguredName::Dot));

        let git_dir = gix::path::into_bstr(repo.current_dir().join(repo.git_dir()))?.into_owned();
        for (direction, expected_spec, expected_url) in [
            (Fetch, fetch_spec, fetch_url),
            (Push, push_spec, push_url.or(fetch_url)),
        ] {
            assert_eq!(
                repo.branch_remote_name("main", direction).expect("dot is configured"),
                "."
            );
            assert_eq!(branch.remote_name(direction), None, "dot isn't a symbolic name");
            assert_eq!(
                repo.branch_remote("main", direction)
                    .expect("the configured remote resolves")?,
                remote
            );
            assert_eq!(
                branch.remote(direction).expect("the configured remote resolves")?,
                remote
            );

            let [spec] = remote.refspecs(direction) else {
                panic!("the dot remote should retain its configured {direction:?} refspec");
            };
            assert_eq!(spec.to_ref().to_bstring(), expected_spec);
            let url = remote.url(direction).expect("dot resolves to a usable URL");
            if let Some(expected) = expected_url {
                assert_eq!(url.to_bstring(), expected, "an explicit URL still takes precedence");
            } else {
                assert_eq!(url.scheme, gix::url::Scheme::File);
                assert_eq!(url.path, git_dir, "refspecs alone don't change the destination");
            }
        }

        for mut config in [gix::config::File::default(), repo.config_snapshot().plumbing().clone()] {
            remote.save_to(&mut config)?;
            let saved = gix_config::File::from_bytes_no_includes(
                &config.to_bstring(),
                gix_config::file::Metadata::from(gix_config::Source::Api),
                Default::default(),
            )?;
            assert_eq!(
                saved.section("remote", ".")?.header().to_bstring(),
                r#"[remote "."]"#,
                "saving preserves the exact dot subsection"
            );
            if fetch_url.is_none() {
                assert!(
                    saved.strings_by("remote", ".", "url").is_none(),
                    "saving must not persist the inferred Git directory as a fetch URL"
                );
            }
            if push_url.is_none() {
                assert!(
                    saved.strings_by("remote", ".", "pushurl").is_none(),
                    "saving must not persist a fallback as an explicit push URL"
                );
            }

            let mut reopened = gix::open_opts(remote::repo_path("branch-dot-remote"), gix::open::Options::isolated())?;
            let mut config = reopened.config_snapshot_mut();
            config.append(saved)?;
            let reloaded = config.commit()?.find_remote(".")?;
            assert_eq!(reloaded.configured_name(), Some(&gix::remote::ConfiguredName::Dot));
            assert_eq!(reloaded.name(), None, "the saved dot subsection is still not symbolic");
            for direction in [Fetch, Push] {
                assert_eq!(
                    reloaded.refspecs(direction),
                    remote.refspecs(direction),
                    "the configured {direction:?} refspec survives saving"
                );
                assert_eq!(
                    reloaded.urls(direction).collect::<Vec<_>>(),
                    remote.urls(direction).collect::<Vec<_>>(),
                    "the {direction:?} URL is preserved or inferred again after reloading"
                );
            }
        }
    }
    Ok(())
}

#[cfg(any(feature = "async-network-client", feature = "blocking-network-client"))]
#[test]
fn dot_remote_does_not_connect_to_the_working_directory() -> TestResult {
    use gix::remote::Direction::Fetch;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = remote::repo_path("branch-dot-remote").canonicalize()?;
    let parent = fixture.parent().expect("the repository is inside the fixture");
    let _cwd = gix_testtools::set_current_dir(parent)?;
    let repo = gix::open_opts("branch-dot-remote", gix::open::Options::isolated())?;
    let remote = repo
        .branch_remote("main", Fetch)
        .transpose()?
        .expect("dot is configured");

    let _cwd = gix_testtools::set_current_dir(parent.join("base"))?;
    let (url, _) = remote.sanitized_url_and_version(Fetch)?;
    assert_eq!(
        gix::path::from_bstr(std::borrow::Cow::Borrowed(url.path.as_ref()))?.canonicalize()?,
        fixture.join(".git").canonicalize()?,
        "the transport uses the opened repository even from a different repository"
    );
    Ok(())
}

#[test]
fn direct_urls_resolve_to_anonymous_remotes() -> TestResult {
    use gix::remote::Direction::{Fetch, Push};

    let repo = remote::repo("remote-as-url");
    let branch = repo.head_ref()?.expect("history");

    for (direction, path) in [(Push, "/push-path.git"), (Fetch, "/fetch-path.git")] {
        assert_eq!(
            branch.remote_name(direction),
            None,
            "a direct URL does not name a remote"
        );
        let remote = branch.remote(direction).transpose()?.expect("the URL resolves");
        assert_eq!(remote.name(), None, "the remote is anonymous");
        assert_eq!(remote.configured_name(), None, "there is no matching subsection");
        for direction in [Fetch, Push] {
            assert_eq!(
                remote.url(direction).expect("the direct URL is available").path,
                path,
                "the chosen URL is used for both directions on the anonymous remote"
            );
        }
    }
    Ok(())
}
