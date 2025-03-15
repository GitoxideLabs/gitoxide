mod remote_at {
    use gix::remote::Direction;

    use crate::remote;

    #[test]
    fn url_and_push_url() -> crate::Result {
        let repo = remote::repo("base");
        let fetch_url = "https://github.com/byron/gitoxide";
        let remote = repo.remote_at(fetch_url)?;

        assert_eq!(remote.name(), None);
        assert_eq!(remote.url(Direction::Fetch).unwrap().to_bstring(), fetch_url);
        assert_eq!(remote.url(Direction::Push).unwrap().to_bstring(), fetch_url);

        let mut remote = remote.push_url("user@host.xz:./relative")?;
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            "user@host.xz:./relative"
        );
        assert_eq!(remote.url(Direction::Fetch).unwrap().to_bstring(), fetch_url);

        for (spec, direction) in [
            ("refs/heads/push", Direction::Push),
            ("refs/heads/fetch", Direction::Fetch),
        ] {
            assert_eq!(
                remote.refspecs(direction),
                &[],
                "no specs are preset for newly created remotes"
            );
            remote = remote.with_refspecs(Some(spec), direction)?;
            assert_eq!(remote.refspecs(direction).len(), 1, "the new refspec was added");

            remote = remote.with_refspecs(Some(spec), direction)?;
            assert_eq!(remote.refspecs(direction).len(), 1, "duplicates are disallowed");
        }

        Ok(())
    }

    #[test]
    fn url_rewrites_are_respected() -> crate::Result {
        let repo = remote::repo("url-rewriting");
        let remote = repo.remote_at("https://github.com/foobar/gitoxide")?;

        assert_eq!(remote.name(), None, "anonymous remotes are unnamed");
        let rewritten_fetch_url = "https://github.com/byron/gitoxide";
        assert_eq!(
            remote.url(Direction::Fetch).unwrap().to_bstring(),
            rewritten_fetch_url,
            "fetch was rewritten"
        );
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            rewritten_fetch_url,
            "push is the same as fetch was rewritten"
        );

        let remote = repo
            .remote_at("https://github.com/foobar/gitoxide".to_owned())?
            .push_url("file://dev/null".to_owned())?;
        assert_eq!(remote.url(Direction::Fetch).unwrap().to_bstring(), rewritten_fetch_url);
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            "ssh://dev/null",
            "push-url rewrite rules are applied"
        );
        Ok(())
    }

    #[test]
    fn url_rewrites_can_be_skipped() -> crate::Result {
        let repo = remote::repo("url-rewriting");
        let remote = repo.remote_at_without_url_rewrite("https://github.com/foobar/gitoxide")?;

        assert_eq!(remote.name(), None, "anonymous remotes are unnamed");
        let fetch_url = "https://github.com/foobar/gitoxide";
        assert_eq!(
            remote.url(Direction::Fetch).unwrap().to_bstring(),
            fetch_url,
            "fetch was rewritten"
        );
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            fetch_url,
            "push is the same as fetch was rewritten"
        );

        let remote = repo
            .remote_at_without_url_rewrite("https://github.com/foobar/gitoxide".to_owned())?
            .push_url_without_url_rewrite("file://dev/null".to_owned())?;
        assert_eq!(remote.url(Direction::Fetch).unwrap().to_bstring(), fetch_url);
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            "file://dev/null",
            "push-url rewrite rules are not applied"
        );
        Ok(())
    }
}

mod find_remote {
    use std::io::BufRead;

    use gix::{remote::Direction, Repository};
    use gix_object::bstr::BString;

    use crate::remote;

    #[test]
    fn tags_option() -> crate::Result {
        let repo = remote::repo("clone-no-tags");
        for (remote_name, expected) in [
            ("origin", gix::remote::fetch::Tags::None),
            ("myself-no-tags", gix::remote::fetch::Tags::None),
            ("myself-with-tags", gix::remote::fetch::Tags::All),
        ] {
            let remote = repo.find_remote(remote_name)?;
            assert_eq!(remote.fetch_tags(), expected, "specifically set in this repo");
        }
        Ok(())
    }

    #[test]
    fn typical() -> crate::Result {
        let repo = remote::repo("clone");
        let mut count = 0;
        let base_dir = base_dir(&repo);
        let expected = [
            (".", "+refs/heads/*:refs/remotes/myself/*"),
            (base_dir.as_str(), "+refs/heads/*:refs/remotes/origin/*"),
        ];
        for (name, (url, refspec)) in repo.remote_names().into_iter().zip(expected) {
            count += 1;
            let remote = repo.find_remote(name.as_ref())?;
            assert_eq!(remote.name().expect("set").as_bstr(), name);

            assert_eq!(
                remote.fetch_tags(),
                gix::remote::fetch::Tags::Included,
                "the default value as it's not specified"
            );

            let url = gix::url::parse(url.into())?;
            assert_eq!(remote.url(Direction::Fetch).expect("present"), &url);

            assert_eq!(
                remote.refspecs(Direction::Fetch),
                &[fetchspec(refspec)],
                "default refspecs are set by git"
            );
            assert_eq!(
                remote.refspecs(Direction::Push),
                &[],
                "push-specs aren't configured by default"
            );
        }
        assert!(count > 0, "should have seen more than one commit");
        assert!(matches!(
            repo.find_remote("unknown").unwrap_err(),
            gix::remote::find::existing::Error::NotFound { .. }
        ));
        Ok(())
    }

    #[test]
    fn push_url_and_push_specs() {
        let repo = remote::repo("push-url");
        let remote = repo.find_remote("origin").expect("present");
        assert_eq!(remote.url(Direction::Push).unwrap().path, ".");
        assert_eq!(remote.url(Direction::Fetch).unwrap().path, base_dir(&repo));
        assert_eq!(remote.refspecs(Direction::Push), &[pushspec("refs/tags/*:refs/tags/*")]);
    }

    #[test]
    fn many_fetchspecs() {
        let repo = remote::repo("many-fetchspecs");
        let remote = repo.find_remote("origin").expect("present");
        assert_eq!(
            remote.refspecs(Direction::Fetch),
            &[
                fetchspec("HEAD"),
                fetchspec("+refs/heads/*:refs/remotes/origin/*"),
                fetchspec("refs/tags/*:refs/tags/*")
            ]
        );
    }

    #[test]
    fn instead_of_url_rewriting() -> crate::Result {
        let repo = remote::repo("url-rewriting");

        let baseline = std::fs::read(repo.git_dir().join("baseline.git"))?;
        let mut baseline = baseline.lines().map_while(Result::ok);
        let expected_fetch_url: BString = baseline.next().expect("fetch").into();
        let expected_push_url: BString = baseline.next().expect("push").into();

        let remote = repo.find_remote("origin")?;
        assert_eq!(remote.url(Direction::Fetch).unwrap().to_bstring(), expected_fetch_url,);
        {
            let actual_push_url = remote.url(Direction::Push).unwrap().to_bstring();
            assert_ne!(
                actual_push_url, expected_push_url,
                "here we actually resolve something that git doesn't probably because it's missing the host. Our parser is OK with it for some reason."
            );
            assert_eq!(actual_push_url, "ssh://dev/null", "file:// gets replaced actually");
        }

        let mut remote = repo.try_find_remote_without_url_rewrite("origin").expect("exists")?;
        assert_eq!(
            remote.url(Direction::Fetch).unwrap().to_bstring(),
            "https://github.com/foobar/gitoxide"
        );
        assert_eq!(remote.url(Direction::Push).unwrap().to_bstring(), "file://dev/null");
        remote.rewrite_urls()?;
        assert_eq!(remote.url(Direction::Push).unwrap().to_bstring(), "ssh://dev/null");
        Ok(())
    }

    #[test]
    fn bad_url_rewriting_can_be_handled_much_like_git() -> crate::Result {
        let repo = remote::repo("bad-url-rewriting");

        let baseline = std::fs::read(repo.git_dir().join("baseline.git"))?;
        let mut baseline = baseline.lines().map_while(Result::ok);
        let expected_fetch_url: BString = baseline.next().expect("fetch").into();
        let expected_push_url: BString = baseline.next().expect("push").into();
        assert_eq!(
            expected_push_url, "file://dev/null",
            "git leaves the failed one as is without any indication…"
        );
        assert_eq!(
            expected_fetch_url, "https://github.com/byron/gitoxide",
            "…but is able to replace the fetch url successfully"
        );

        let expected_err_msg = "The rewritten push url \"invalid:://dev/null\" failed to parse";
        assert_eq!(
            repo.find_remote("origin").unwrap_err().to_string(),
            expected_err_msg,
            "this fails by default as rewrites fail"
        );

        let mut remote = repo.try_find_remote_without_url_rewrite("origin").expect("exists")?;
        for round in 1..=2 {
            if round == 1 {
                assert_eq!(
                    remote.url(Direction::Fetch).unwrap().to_bstring(),
                    "https://github.com/foobar/gitoxide",
                    "no rewrite happened"
                );
            } else {
                assert_eq!(
                    remote.url(Direction::Fetch).unwrap().to_bstring(),
                    "https://github.com/byron/gitoxide",
                    "it can rewrite a single url like git can"
                );
            }
            assert_eq!(
                remote.rewrite_urls().unwrap_err().to_string(),
                expected_err_msg,
                "rewriting fails, but it will rewrite what it can while reporting a single error."
            );
        }
        Ok(())
    }

    fn fetchspec(spec: &str) -> gix_refspec::RefSpec {
        gix::refspec::parse(spec.into(), gix::refspec::parse::Operation::Fetch)
            .unwrap()
            .to_owned()
    }

    fn pushspec(spec: &str) -> gix_refspec::RefSpec {
        gix::refspec::parse(spec.into(), gix::refspec::parse::Operation::Push)
            .unwrap()
            .to_owned()
    }

    fn base_dir(repo: &Repository) -> String {
        gix_path::to_unix_separators_on_windows(gix::path::into_bstr(
            gix::path::realpath(repo.workdir().unwrap())
                .unwrap()
                .parent()
                .unwrap()
                .join("base"),
        ))
        .into_owned()
        .to_string()
    }
}

mod find_fetch_remote {
    use crate::remote;

    #[test]
    fn symbol_name() -> crate::Result {
        let repo = remote::repo("clone-no-tags");
        assert_eq!(
            repo.find_fetch_remote(Some("origin".into()))?
                .name()
                .expect("set")
                .as_bstr(),
            "origin"
        );
        Ok(())
    }

    #[test]
    fn urls() -> crate::Result {
        let repo = remote::repo("clone-no-tags");
        for url in [
            "some-path",
            "https://example.com/repo",
            "other/path",
            "ssh://host/ssh-aliased-repo",
        ] {
            let remote = repo.find_fetch_remote(Some(url.into()))?;
            assert_eq!(remote.name(), None, "this remote is anonymous");
            assert_eq!(
                remote
                    .url(gix::remote::Direction::Fetch)
                    .expect("url is set")
                    .to_bstring(),
                url,
                "if it's not a configured remote, we take it as URL"
            );
        }
        Ok(())
    }
}

mod find_default_remote {

    use crate::remote;

    #[test]
    fn works_on_detached_heads() -> crate::Result {
        let repo = remote::repo("detached-head");
        assert_eq!(
            repo.find_default_remote(gix::remote::Direction::Fetch)
                .transpose()?
                .expect("present")
                .name()
                .expect("always named")
                .as_bstr(),
            "origin"
        );
        Ok(())
    }
}
