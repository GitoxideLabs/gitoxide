mod save_to {

    use crate::{remote, remote::save::uniformize};

    #[test]
    fn named_remotes_save_as_is() -> crate::Result {
        let repo = remote::repo("clone");
        let remote = repo.find_remote("origin")?;

        let mut config = gix::config::File::default();
        remote.save_to(&mut config)?;
        let actual = uniformize(config.to_string());
        assert!(
            actual.starts_with("[remote \"origin\"]\n\turl = "),
            "workaround absolute paths in test fixture…"
        );
        assert!(
            actual.ends_with("/base\n\tfetch = +refs/heads/*:refs/remotes/origin/*\n"),
            "…by checking only the parts that are similar"
        );

        let previous_remote_state = repo
            .config_snapshot()
            .plumbing()
            .section_by_key("remote.origin".into())
            .expect("present")
            .to_bstring();
        let mut config = repo.config_snapshot().plumbing().clone();
        remote.save_to(&mut config)?;
        assert_eq!(
            config.sections_by_name("remote").expect("more than one").count(),
            2,
            "amount of remotes are unaltered"
        );
        assert_eq!(
            config
                .section_by_key("remote.origin".into())
                .expect("present")
                .to_bstring(),
            previous_remote_state,
            "the serialization doesn't modify anything"
        );
        Ok(())
    }
}

mod save_as_to {
    use crate::{basic_repo, remote::save::uniformize};

    #[test]
    fn anonymous_remotes_cannot_be_saved_lacking_a_name() -> crate::Result {
        let repo = basic_repo()?;
        let remote = repo.remote_at("https://example.com/path")?;
        assert!(matches!(
            remote.save_to(&mut gix::config::File::default()).unwrap_err(),
            gix::remote::save::Error::NameMissing { .. }
        ));
        Ok(())
    }

    #[test]
    fn new_anonymous_remote_with_name() -> crate::Result {
        let repo = basic_repo()?;
        let mut remote = repo
            .remote_at("https://example.com/path")?
            .with_push_url("https://ein.hub/path")?
            .with_fetch_tags(gix::remote::fetch::Tags::All)
            .with_refspecs(
                [
                    "+refs/heads/*:refs/remotes/any/*",
                    "refs/heads/special:refs/heads/special-upstream",
                ],
                gix::remote::Direction::Fetch,
            )?
            .with_refspecs(
                [
                    "refs/heads/main:refs/heads/main", // similar to 'simple' for `push.default`
                    ":",                               // similar to 'matching'
                ],
                gix::remote::Direction::Push,
            )?;
        let remote_name = "origin";
        assert!(
            repo.find_remote(remote_name).is_err(),
            "there is no remote of that name"
        );
        assert_eq!(remote.name(), None);
        let mut config = gix::config::File::default();
        remote.save_as_to(remote_name, &mut config)?;
        let expected = "[remote \"origin\"]\n\turl = https://example.com/path\n\tpushurl = https://ein.hub/path\n\ttagOpt = --tags\n\tfetch = +refs/heads/*:refs/remotes/any/*\n\tfetch = refs/heads/special:refs/heads/special-upstream\n\tpush = refs/heads/main:refs/heads/main\n\tpush = :\n";
        assert_eq!(uniformize(config.to_string()), expected);

        remote.save_as_to(remote_name, &mut config)?;
        assert_eq!(
            uniformize(config.to_string()),
            expected,
            "it appears to be idempotent in this case"
        );

        {
            let mut new_section = config.section_mut_or_create_new("unrelated", None).expect("works");
            new_section.push("a".try_into().unwrap(), Some("value".into()));

            config
                .section_mut_or_create_new("initially-empty-not-removed", Some("name".into()))
                .expect("works");

            let mut existing_section = config
                .section_mut_or_create_new("remote", Some("origin".into()))
                .expect("works");
            existing_section.push("free".try_into().unwrap(), Some("should not be removed".into()));
        }
        remote.save_as_to(remote_name, &mut config)?;
        assert_eq!(
            uniformize(config.to_string()),
            "[remote \"origin\"]\n\tfree = should not be removed\n\turl = https://example.com/path\n\tpushurl = https://ein.hub/path\n\ttagOpt = --tags\n\tfetch = +refs/heads/*:refs/remotes/any/*\n\tfetch = refs/heads/special:refs/heads/special-upstream\n\tpush = refs/heads/main:refs/heads/main\n\tpush = :\n[unrelated]\n\ta = value\n[initially-empty-not-removed \"name\"]\n",
            "unrelated keys are kept, and so are keys in the sections we edit"
        );
        Ok(())
    }

    #[test]
    fn new_remote_in_presence_of_global_section_writes_to_local_file() -> crate::Result {
        use gix::bstr::ByteSlice;
        // A repo with no remotes, opened so that `remote.origin` exists only as a non-local (`Api`)
        // override, mirroring global configuration like `remote.origin.prune = true` (issue #1951).
        let repo = gix::ThreadSafeRepository::open_opts(
            gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?,
            gix::open::Options::isolated().config_overrides(["remote.origin.prune=true"]),
        )?
        .to_thread_local();

        let mut config = repo.config_snapshot().plumbing().clone();
        let local_meta = config.meta().clone();

        repo.remote_at("https://example.com/")?
            .save_as_to("origin", &mut config)?;

        let url_is_in_local_section = config
            .sections_by_name("remote")
            .into_iter()
            .flatten()
            .filter(|s| s.header().subsection_name() == Some(b"origin".as_bstr()))
            .any(|s| *s.meta() == local_meta && s.value("url").is_some());

        assert!(
            url_is_in_local_section,
            "the new remote's url must be written to a section owned by the local config file, \
             not merged into the global/override section"
        );
        Ok(())
    }
}

fn uniformize(input: String) -> String {
    input.replace("\r\n", "\n")
}
