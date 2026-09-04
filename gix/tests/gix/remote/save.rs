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
            .section_by_key("remote.origin")
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
            config.section_by_key("remote.origin").expect("present").to_bstring(),
            previous_remote_state,
            "the serialization doesn't modify anything"
        );
        Ok(())
    }
}

mod save_as_to {
    use crate::{
        basic_repo, remote,
        remote::save::{remote_config, uniformize},
    };

    #[test]
    fn anonymous_remotes_cannot_be_saved_lacking_a_name() -> crate::Result {
        let repo = basic_repo()?;
        let remote = repo.remote_at("https://example.com/path")?;
        assert_eq!(
            remote
                .save_to(&mut gix::config::File::default())
                .unwrap_err()
                .to_string(),
            "The remote pointing to https://example.com/path is anonymous and can't be saved."
        );
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
        remote.save_as_to(remote_name.as_bytes(), &mut config)?;
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
            new_section.push("a", "value")?;

            config
                .section_mut_or_create_new("initially-empty-not-removed", "name")
                .expect("works");

            let mut existing_section = config.section_mut_or_create_new("remote", "origin").expect("works");
            existing_section.push("free", "should not be removed")?;
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
        let repo = gix::open_opts(
            gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?,
            gix::open::Options::isolated().config_overrides(["remote.origin.prune=true"]),
        )?;

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

    /// The overrides below add URLs to `origin` from outside the local file. As URLs are multi-valued,
    /// saving another `origin` must clear those inherited values locally, or reopening would merge them
    /// back into the saved URL lists.
    #[test]
    fn inherited_urls_are_saved_with_reset_markers() -> crate::Result {
        use gix::bstr::{BStr, BString};

        let repo = gix::open_opts(
            gix_testtools::scripted_fixture_read_only("make_basic_repo.sh")?,
            gix::open::Options::isolated().config_overrides([
                "remote.origin.url=https://inherited.example/path",
                "remote.origin.pushUrl=https://inherited.example/push",
            ]),
        )?;
        let mut remote = repo
            .remote_at("https://example.com/path")?
            .with_push_url("https://push.example/path")?;
        let mut config = repo.config_snapshot().plumbing().clone();
        let local_meta = config.meta().clone();

        // We save the anonymous remote as `origin`, and writing our remote shouldn't
        // inherit the values that are already present.
        remote.save_as_to("origin", &mut config)?;
        // A repeated save must preserve reset markers after the inherited values were removed in-memory.
        remote.save_as_to("origin", &mut config)?;

        let local_values = |key| -> Vec<BString> {
            config
                .sections_by_name("remote")
                .into_iter()
                .flatten()
                .filter(|s| s.header().subsection_name() == Some(BStr::new("origin")) && *s.meta() == local_meta)
                .flat_map(|s| s.values(key))
                .collect()
        };
        assert_eq!(
            local_values("url"),
            vec![BString::from(""), BString::from("https://example.com/path")],
            "local URL values clear inherited URLs before writing the effective list"
        );
        assert_eq!(
            local_values("pushurl"),
            vec![BString::from(""), BString::from("https://push.example/path")],
            "local push URL values clear inherited push URLs before writing the effective list"
        );
        insta::assert_snapshot!(
            remote_config(&config, "origin"),
            "empty URL values reset inherited fetch and push URLs so reopening
            doesn't append them to the saved effective values",
            @r#"
        [remote "origin"]
        	url = 
        	url = https://example.com/path
        	pushUrl = 
        	pushurl = https://push.example/path
        "#
        );
        Ok(())
    }

    /// Simulate a URL from an include following the local remote section. Saving must place the
    /// reset-bearing local section after it so reopening clears the included URL before restoring
    /// the remote's effective URL list.
    #[test]
    fn reset_markers_follow_later_foreign_url_sections() -> crate::Result {
        use gix::bstr::{BStr, BString, ByteSlice};

        let inherited_url = "https://included.example/path";
        let repo = gix::open_opts(
            remote::repo("clone").path(),
            gix::open::Options::isolated().config_overrides([format!("remote.origin.url={inherited_url}")]),
        )?;
        let remote = repo.find_remote("origin")?;
        let expected_urls: Vec<BString> = remote
            .urls(gix::remote::Direction::Fetch)
            .map(gix::Url::to_bstring)
            .collect();
        let mut config = repo.config_snapshot().plumbing().clone();
        let local_meta = config.meta().clone();
        let render_config = |config: &gix::config::File| {
            remote_config(config, "origin")
                .to_str_lossy()
                .replace(expected_urls[0].to_str_lossy().as_ref(), "<local-clone-url>")
        };

        // Keep the original local section visible after saving removes its managed keys. Then switch
        // file metadata and add a foreign `origin` section, as if it came from a later include; the
        // unrelated `prune` values keep both sections around to expose their order. Restoring the local
        // metadata makes it the save target, where the reset-bearing section must follow the foreign one.
        config
            .section_mut_filter("remote", "origin", |meta| *meta == local_meta)?
            .expect("the fixture has a local origin section")
            .push("prune", "1")?;
        let included_meta = gix_config::file::Metadata::from(gix_config::Source::Local);
        assert_ne!(
            included_meta, local_meta,
            "metadata without the target file's path gives the simulated include a distinct file identity"
        );
        config.set_meta(included_meta);
        let mut included = config.new_section("remote", "origin")?;
        included.push("url", inherited_url)?;
        included.push("prune", "true")?;
        config.set_meta(local_meta.clone());

        insta::assert_snapshot!(
            render_config(&config),
            "before saving, the resolved configuration contains the original local remote, the API-provided URL
            used to build the effective remote, and a later simulated include whose unrelated value must survive",
            @r#"
        [remote "origin"]
        	url = <local-clone-url>
        	fetch = +refs/heads/*:refs/remotes/origin/*
        	prune = 1
        [remote "origin"]
        	url = https://included.example/path
        [remote "origin"]
        	url = https://included.example/path
        	prune = true
        "#
        );

        remote.save_to(&mut config)?;

        insta::assert_snapshot!(
            render_config(&config),
            "after saving, unrelated values keep the original and included sections intact while a trailing local
            section resets the foreign URLs before writing the remote's effective URLs and fetch refspec",
            @r#"
        [remote "origin"]
        	prune = 1
        [remote "origin"]
        	prune = true
        [remote "origin"]
        	url = 
        	url = <local-clone-url>
        	url = https://included.example/path
        	fetch = +refs/heads/*:refs/remotes/origin/*
        "#
        );

        let ordered_urls: Vec<BString> = config
            .sections_by_name("remote")
            .into_iter()
            .flatten()
            .filter(|section| section.header().subsection_name() == Some(BStr::new("origin")))
            .flat_map(|section| section.values("url"))
            .collect();
        let mut expected = vec![BString::from("")];
        expected.extend(expected_urls);
        assert_eq!(
            ordered_urls, expected,
            "the local reset and effective URL list follow URL values from later includes"
        );
        let origin_sections: Vec<_> = config
            .sections_by_name("remote")
            .into_iter()
            .flatten()
            .filter(|section| section.header().subsection_name() == Some(BStr::new("origin")))
            .collect();
        let reset_position = origin_sections
            .iter()
            .position(|section| section.values("url").iter().any(|url| url.is_empty()))
            .expect("a URL reset was written");
        let included_position = origin_sections
            .iter()
            .position(|section| *section.meta() != local_meta)
            .expect("the included section remains present");
        assert!(
            reset_position > included_position,
            "the reset-bearing section must follow later included sections"
        );
        Ok(())
    }
}

fn remote_config(config: &gix::config::File, name: &str) -> gix::bstr::BString {
    let mut out = Vec::new();
    config
        .write_to_filter(&mut out, |section| {
            section.header().name() == "remote"
                && section.header().subsection_name() == Some(gix::bstr::BStr::new(name))
        })
        .expect("writing to memory cannot fail");
    out.into()
}

fn uniformize(input: String) -> String {
    input.replace("\r\n", "\n")
}
