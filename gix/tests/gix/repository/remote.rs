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

        let mut remote = remote.with_push_url("user@host.xz:./relative")?;
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            "user@host.xz:./relative"
        );
        assert_eq!(remote.url(Direction::Fetch).unwrap().to_bstring(), fetch_url);

        let new_fetch_url = "https://host.xz/byron/gitoxide";
        remote = remote.with_url(new_fetch_url)?;
        assert_eq!(remote.url(Direction::Fetch).unwrap().to_bstring(), new_fetch_url);

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

        let remote = remote.with_url("https://github.com/foobar/gitoxide")?;
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

        let explicit_push_url = "file://dev/null";
        let remote = repo
            .remote_at("https://github.com/foobar/gitoxide".to_owned())?
            .with_push_url(explicit_push_url.to_owned())?;
        assert_eq!(remote.url(Direction::Fetch).unwrap().to_bstring(), rewritten_fetch_url);
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            explicit_push_url,
            "pushInsteadOf does not rewrite explicit push URLs, just like Git"
        );

        let remote = remote.with_push_url("https://github.com/foobar/gitoxide".to_owned())?;
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            rewritten_fetch_url,
            "insteadOf rewrites explicit push URLs, just like Git"
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

        let remote = remote.with_url_without_url_rewrite("https://github.com/foobaz/gitoxide")?;
        assert_eq!(
            remote.url(Direction::Fetch).unwrap().to_bstring(),
            "https://github.com/foobaz/gitoxide",
            "fetch was rewritten"
        );
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            "https://github.com/foobaz/gitoxide",
            "push is the same as fetch was rewritten"
        );

        let remote = repo
            .remote_at_without_url_rewrite("https://github.com/foobar/gitoxide".to_owned())?
            .with_push_url_without_url_rewrite("file://dev/null".to_owned())?;
        assert_eq!(remote.url(Direction::Fetch).unwrap().to_bstring(), fetch_url);
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            "file://dev/null",
            "push-url rewrite rules are not applied"
        );

        let remote = remote
            .with_url_without_url_rewrite("https://github.com/foobaz/gitoxide".to_owned())?
            .with_push_url_without_url_rewrite("file://dev/null".to_owned())?;
        assert_eq!(
            remote.url(Direction::Fetch).unwrap().to_bstring(),
            "https://github.com/foobaz/gitoxide"
        );
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            "file://dev/null",
            "push-url rewrite rules are not applied"
        );
        Ok(())
    }

    #[test]
    fn with_url_ignores_bad_push_fallback_rewrites() -> crate::Result {
        let repo = remote::repo("bad-push-fallback-url-rewriting");
        let remote = repo.remote_at("alias:one")?.with_url("alias:two")?;

        assert_eq!(
            remote.url(Direction::Fetch).expect("present").to_bstring(),
            "alias:two",
            "changing the fetch URL should not fail due to a malformed push-only rewrite"
        );
        assert_eq!(
            remote.url(Direction::Push).expect("present").to_bstring(),
            "alias:two",
            "the invalid push fallback rewrite is left unapplied"
        );
        Ok(())
    }
}

mod find_remote {
    use std::io::BufRead;

    use gix::{Repository, remote::Direction};
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
            let remote = repo.find_remote(&name)?;
            assert_eq!(remote.name().expect("set"), name);

            assert_eq!(
                remote.fetch_tags(),
                gix::remote::fetch::Tags::Included,
                "the default value as it's not specified"
            );

            let url = gix::url::parse(url)?;
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
        let err = repo.find_remote("unknown").unwrap_err();
        assert!(err.is_not_found());
        assert!(
            err.iter_errors()
                .any(|source| source.to_string() == "The remote named \"unknown\" did not exist")
        );
        Ok(())
    }

    #[test]
    fn missing_fetch_urls_only_fall_back_to_url_shaped_remote_names() -> crate::Result {
        let repo = remote::repo("missing-urls");
        for name in ["no-url", "reset-url"] {
            let remote = repo.find_remote(name)?;
            assert_eq!(
                remote.url(Direction::Fetch),
                None,
                "a symbolic remote name must not become a fetch URL"
            );
            assert_eq!(
                remote.url(Direction::Push),
                None,
                "without an explicit push URL there is no fetch URL to fall back to"
            );
        }

        let remote = repo.find_remote("push-only")?;
        assert_eq!(remote.url(Direction::Fetch), None, "there is no fetch URL");
        assert_eq!(
            remote.url(Direction::Push).expect("explicit push URL").to_bstring(),
            "ssh://push.example/repo",
            "an explicit push URL remains available"
        );

        for name in ["https://fallback.example/repo", "relative/path", "example.com:repo"] {
            let remote = repo.find_remote(name)?;
            assert_eq!(
                remote.url(Direction::Fetch).expect("fallback URL").to_bstring(),
                name,
                "a URL-shaped remote name becomes the missing fetch URL"
            );
            assert_eq!(
                remote.url(Direction::Push).expect("fallback URL").to_bstring(),
                name,
                "the fetch URL is also the push fallback"
            );
        }
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
        assert_eq!(
            remote.url(Direction::Fetch).unwrap().to_bstring(),
            expected_fetch_url,
            "the configured fetch URL is rewritten by the normal insteadOf rule, matching Git"
        );
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            expected_push_url,
            "explicit pushUrl values use normal insteadOf rewrites; 
            pushInsteadOf only rewrites remote URLs used as a fallback when no pushUrl is configured"
        );

        let mut remote = repo.try_find_remote_without_url_rewrite("origin").expect("exists")?;
        let expected_fetch_url_no_rewrite = "https://github.com/foobar/gitoxide";
        assert_ne!(expected_fetch_url_no_rewrite, expected_fetch_url);
        assert_eq!(
            remote.url(Direction::Fetch).unwrap().to_bstring(),
            expected_fetch_url_no_rewrite,
            "without URL rewriting the fetch URL is returned exactly as configured"
        );
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            expected_push_url,
            "without URL rewriting the explicit pushUrl is returned exactly as configured"
        );
        remote.rewrite_urls()?;
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            expected_push_url,
            "refreshing rewrites keeps the explicit pushUrl unchanged because
            pushInsteadOf is ignored and no insteadOf rule matches"
        );
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

        let remote = repo.find_remote("origin")?;
        assert_eq!(
            remote.url(Direction::Fetch).unwrap().to_bstring(),
            expected_fetch_url,
            "the valid fetch insteadOf rewrite succeeds despite the invalid pushInsteadOf rule"
        );
        assert_eq!(
            remote.url(Direction::Push).unwrap().to_bstring(),
            expected_push_url,
            "a bad pushInsteadOf rule is ignored for explicit pushUrl values"
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
                    expected_fetch_url,
                    "it can rewrite a single url like git can"
                );
            }
            remote.rewrite_urls()?;
            assert_eq!(
                remote.url(Direction::Push).unwrap().to_bstring(),
                expected_push_url,
                "explicit pushUrl values still ignore pushInsteadOf when rewrites are refreshed"
            );
        }
        Ok(())
    }

    #[test]
    fn multiple_urls_are_preserved_in_order_and_single_url_matches_git_first_url() -> crate::Result {
        let repo = remote::repo("multiple-urls");

        let baseline = std::fs::read(repo.git_dir().join("baseline.git"))?;
        let mut baseline = baseline.lines().map_while(Result::ok);
        let expected_fetch_url: BString = baseline.next().expect("single fetch").into();
        let expected_fetch_urls: Vec<BString> = baseline
            .by_ref()
            .take(2 /* multiple expected fetch urls */)
            .map(Into::into)
            .collect();
        let expected_push_url: BString = baseline.next().expect("single push").into();
        let expected_push_urls: Vec<BString> = baseline.map(Into::into).collect();

        let remote = repo.find_remote("origin")?;
        assert_eq!(
            remote.url(Direction::Fetch).expect("present").to_bstring(),
            expected_fetch_url,
            "the single fetch URL matches Git, which returns the first configured URL"
        );
        assert_eq!(
            urls(&remote, Direction::Fetch),
            expected_fetch_urls,
            "all fetch URLs are returned in configuration order"
        );
        assert_eq!(
            remote.url(Direction::Push).expect("present").to_bstring(),
            expected_push_url,
            "the single push URL matches Git, which returns the first configured URL"
        );
        assert_eq!(
            urls(&remote, Direction::Push),
            expected_push_urls,
            "all push URLs are returned in configuration order"
        );

        let mut remote = repo.try_find_remote_without_url_rewrite("origin").expect("exists")?;
        assert_eq!(
            urls(&remote, Direction::Fetch),
            ["alias:one", "alias:two"],
            "without URL rewriting the raw fetch URLs are visible"
        );
        assert_eq!(
            urls(&remote, Direction::Push),
            ["alias:one", "alias:two"],
            "without URL rewriting the raw fetch URLs are visible for pushing as there is no pushUrl"
        );
        remote.rewrite_urls()?;
        assert_eq!(
            urls(&remote, Direction::Push),
            expected_push_urls,
            "rewriting can apply pushInsteadOf to every URL"
        );

        Ok(())
    }

    #[test]
    fn multiple_url_rewriting_preserves_successful_rewrites_when_another_url_fails() -> crate::Result {
        let repo = remote::repo("multiple-bad-url-rewriting");

        let mut remote = repo.try_find_remote_without_url_rewrite("origin").expect("exists")?;
        assert_eq!(
            remote.rewrite_urls().unwrap_err().to_string(),
            "The rewritten fetch url \":://gitoxide\" failed to parse",
            "one malformed rewrite is reported"
        );
        assert_eq!(
            urls(&remote, Direction::Fetch),
            ["https://github.com/byron/gitoxide", "bad:gitoxide"],
            "valid rewrites are preserved even when another URL fails"
        );

        Ok(())
    }

    #[test]
    fn empty_url_values_reset_earlier_url_lists() -> crate::Result {
        let repo = remote::repo("multiple-urls-with-empty-reset");

        let baseline = std::fs::read(repo.git_dir().join("baseline.git"))?;
        let mut baseline = baseline.lines().map_while(Result::ok);
        let expected_fetch_url: BString = baseline.next().expect("single fetch").into();
        let expected_fetch_urls: Vec<BString> = baseline
            .by_ref()
            .take(1 /* just one fetch ref in the --all variant */)
            .map(Into::into)
            .collect();
        let expected_push_url: BString = baseline.next().expect("single push").into();
        let expected_push_urls: Vec<BString> = baseline.map(Into::into).collect();

        let remote = repo.find_remote("origin")?;
        assert_eq!(
            remote.url(Direction::Fetch).expect("present").to_bstring(),
            expected_fetch_url,
            "the singular fetch URL comes from the post-reset list"
        );
        assert_eq!(
            urls(&remote, Direction::Fetch),
            expected_fetch_urls,
            "empty fetch URL values clear earlier values like Git"
        );
        assert_eq!(
            remote.url(Direction::Push).expect("present").to_bstring(),
            expected_push_url,
            "the singular push URL comes from the post-reset list"
        );
        assert_eq!(
            urls(&remote, Direction::Push),
            expected_push_urls,
            "empty push URL values clear earlier values like Git"
        );

        Ok(())
    }

    #[test]
    fn bad_push_fallback_rewriting_does_not_break_fetch_remote() -> crate::Result {
        let repo = remote::repo("bad-push-fallback-url-rewriting");

        let remote = repo.find_remote("origin")?;
        assert_eq!(
            remote.url(Direction::Fetch).expect("present").to_bstring(),
            "alias:repo",
            "a malformed push-only rewrite must not prevent loading the fetch remote"
        );
        assert_eq!(
            remote.url(Direction::Push).expect("present").to_bstring(),
            "alias:repo",
            "the invalid push fallback rewrite is left unapplied during construction"
        );

        let mut remote = repo.try_find_remote_without_url_rewrite("origin").expect("exists")?;
        assert_eq!(
            remote.rewrite_urls().unwrap_err().to_string(),
            "The rewritten push url \":://repo\" failed to parse",
            "explicit rewriting still reports the malformed push fallback rewrite"
        );
        assert_eq!(
            remote.url(Direction::Fetch).expect("present").to_bstring(),
            "alias:repo",
            "the fetch URL remains usable after a failed push-only rewrite"
        );

        Ok(())
    }

    #[test]
    fn bad_explicit_push_url_rewriting_is_reported_as_push_url() -> crate::Result {
        let repo = remote::repo("bad-explicit-push-url-rewriting");

        let expected_err_msg = "The rewritten push url \":://repo\" failed to parse";
        assert!(
            repo.find_remote("origin")
                .unwrap_err()
                .iter_errors()
                .any(|source| source.to_string() == expected_err_msg),
            "explicit pushUrl values use normal insteadOf rewriting, 
            so a malformed result must fail and be labeled as a push URL error"
        );

        let mut remote = repo.try_find_remote_without_url_rewrite("origin").expect("exists")?;
        assert_eq!(
            remote.rewrite_urls().unwrap_err().to_string(),
            expected_err_msg,
            "refreshing rewrites also rejects the malformed result of applying insteadOf to an explicit pushUrl"
        );

        Ok(())
    }

    fn urls(remote: &gix::Remote<'_>, direction: Direction) -> Vec<BString> {
        remote.urls(direction).map(gix::Url::to_bstring).collect()
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
