use crate::remote;

#[test]
fn push_defaults_to_fetch() -> crate::Result {
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
fn separate_push_and_fetch() -> crate::Result {
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
fn not_configured() -> crate::Result {
    let repo = remote::repo("base");
    let head = repo.head()?;
    let branch = head.clone().try_into_referent().expect("history");

    assert_eq!(branch.remote_name(gix::remote::Direction::Push), None);
    assert_eq!(branch.remote_name(gix::remote::Direction::Fetch), None);
    assert_eq!(branch.remote(gix::remote::Direction::Fetch).transpose()?, None);
    assert_eq!(head.into_remote(gix::remote::Direction::Fetch).transpose()?, None);
    let err = repo.find_fetch_remote(None).unwrap_err();
    assert!(err.is_not_found(), "there is no remote to be found");
    assert_eq!(
        err.probable_cause().to_string(),
        "No configured remote could be found, or too many were available"
    );

    Ok(())
}

#[test]
fn dot_remote_behind_symbol() -> crate::Result {
    let repo = remote::repo("branch-dot-remote");
    let head = repo.head()?;
    let branch = head.clone().try_into_referent().expect("history");

    assert_eq!(
        branch.remote_name(gix::remote::Direction::Push).expect("derived push"),
        "."
    );
    assert_eq!(branch.remote_name(gix::remote::Direction::Fetch).expect("fetch"), ".");

    {
        let remote = branch
            .remote(gix::remote::Direction::Push)
            .transpose()?
            .expect("present");
        assert_eq!(remote.name(), None, "It's a url after all, anonymous");
        assert_eq!(remote.url(gix::remote::Direction::Push).unwrap().path, ".");
        assert_eq!(remote.url(gix::remote::Direction::Fetch).unwrap().path, ".");
    }

    Ok(())
}

#[test]
fn url_as_remote_name() -> crate::Result {
    let repo = remote::repo("remote-as-url");
    let branch = repo.head_ref()?.expect("history");

    assert_eq!(
        branch.remote_name(gix::remote::Direction::Push).expect("set"),
        "https://example.com/push-path.git",
        "remote names can also be urls"
    );
    assert_eq!(
        branch.remote_name(gix::remote::Direction::Fetch).expect("set"),
        "https://example.com/fetch-path.git"
    );
    {
        let remote = branch
            .remote(gix::remote::Direction::Push)
            .transpose()?
            .expect("present");
        assert_eq!(remote.name(), None, "It's a url after all, anonymous");
        assert_eq!(remote.url(gix::remote::Direction::Push).unwrap().path, "/push-path.git");
        assert_eq!(
            remote.url(gix::remote::Direction::Fetch).unwrap().path,
            "/push-path.git",
            "this is an anonymous remote with just a single url configured"
        );
    }
    {
        let remote = branch
            .remote(gix::remote::Direction::Fetch)
            .transpose()?
            .expect("present");
        assert_eq!(remote.name(), None, "It's a url after all, anonymous");
        assert_eq!(
            remote.url(gix::remote::Direction::Fetch).unwrap().path,
            "/fetch-path.git",
            "anonymous remotes have a single url only"
        );
        assert_eq!(
            remote.url(gix::remote::Direction::Push).unwrap().path,
            "/fetch-path.git"
        );
    }
    Ok(())
}
