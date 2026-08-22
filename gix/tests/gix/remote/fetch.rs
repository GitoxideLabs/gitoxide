mod shallow {
    use gix::remote::fetch::Shallow;

    #[test]
    fn undo() {
        assert_eq!(
            Shallow::undo(),
            Shallow::DepthAtRemote(2147483647u32.try_into().expect("known at compile time"))
        );
    }
}

#[cfg(any(feature = "blocking-network-client", feature = "async-network-client-async-std"))]
mod blocking_and_async_io {
    use std::sync::atomic::AtomicBool;

    use gix::{
        config::tree::Protocol,
        remote::{Direction::Fetch, fetch, fetch::Status},
    };
    use gix_features::progress;
    use gix_protocol::bisync;
    use gix_testtools::tempfile::TempDir;

    use crate::{
        remote,
        remote::{into_daemon_remote_if_async, spawn_git_daemon_if_async},
        util::hex_to_id,
    };

    /// The objects `clone-as-base-with-changes` sends on top of what the client already has:
    /// the new commit, its tree, and the empty blob it adds.
    const PACK_OBJECTS_WITHOUT_TAG: &[&str] = &[
        "17f2f33324841332c4f6156b36ecde6e44d1bb23",
        "4d979abcde5cea47b079c38850828956c9382a56",
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
    ];

    /// As above, plus the annotated tag object included when tags are fetched.
    const PACK_OBJECTS_WITH_TAG: &[&str] = &[
        "17f2f33324841332c4f6156b36ecde6e44d1bb23",
        "4d979abcde5cea47b079c38850828956c9382a56",
        "978f927e6397113757dfec6332e7d9c7e356ac25",
        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
    ];

    fn expected_pack_object_ids(sha1_hexes: &[&str]) -> Vec<gix::ObjectId> {
        let mut ids: Vec<_> = sha1_hexes.iter().copied().map(hex_to_id).collect();
        ids.sort();
        ids
    }

    /// Return the sorted object ids of the pack whose index was written to `index_path`.
    ///
    /// Prefer this over asserting the pack's `data_hash`/`index_hash`: those cover the *compressed*
    /// bytes, which differ between zlib implementations — a `zlib-ng`-linked `git` deflates some
    /// objects a byte smaller than stock zlib — while object ids hash uncompressed content and are
    /// therefore identical on every host.
    fn pack_object_ids(index_path: Option<&std::path::Path>, hash: gix::hash::Kind) -> Vec<gix::ObjectId> {
        let index = gix::odb::pack::index::File::at(index_path.expect("pack index was written"), hash)
            .expect("written index can be read back");
        let mut ids: Vec<_> = index.iter().map(|e| e.oid).collect();
        ids.sort();
        ids
    }

    pub(crate) fn base_repo_path() -> String {
        gix::path::realpath(
            gix_testtools::scripted_fixture_read_only("make_remote_repos.sh")
                .unwrap()
                .join("base"),
        )
        .unwrap()
        .to_string_lossy()
        .into_owned()
    }

    pub(crate) fn repo_path(name: &str) -> std::path::PathBuf {
        let dir =
            gix_testtools::scripted_fixture_read_only_with_args("make_fetch_repos.sh", [base_repo_path()]).unwrap();
        dir.join(name)
    }

    pub(crate) fn try_repo_rw(
        name: &str,
    ) -> Result<(gix::Repository, gix_testtools::tempfile::TempDir), gix::open::Error> {
        try_repo_rw_args(name, Vec::<String>::new(), Mode::FastClone)
    }

    pub(crate) enum Mode {
        FastClone,
        CloneWithShallowSupport,
    }

    pub(crate) fn try_repo_rw_args<S: Into<String>>(
        name: &str,
        args: impl IntoIterator<Item = S>,
        mode: Mode,
    ) -> Result<(gix::Repository, gix_testtools::tempfile::TempDir), gix::open::Error> {
        let dir = gix_testtools::scripted_fixture_writable_with_args_single_archive(
            "make_fetch_repos.sh",
            [{
                let mut url = base_repo_path();
                if matches!(mode, Mode::CloneWithShallowSupport) {
                    url.insert_str(0, "file://");
                }
                url
            }]
            .into_iter()
            .chain(args.into_iter().map(Into::into)),
            gix_testtools::Creation::Execute,
        )
        .unwrap();
        let repo = gix::open_opts(dir.path().join(name), crate::restricted())?;
        Ok((repo, dir))
    }

    pub(crate) fn repo_rw(name: &str) -> (gix::Repository, gix_testtools::tempfile::TempDir) {
        try_repo_rw(name).unwrap()
    }

    fn commit_empty(repo: &gix::Repository, message: &str) -> crate::Result<gix::ObjectId> {
        Ok(repo
            .commit(
                "HEAD",
                message,
                gix::hash::ObjectId::empty_tree(repo.object_hash()),
                gix::commit::NO_PARENT_IDS,
            )?
            .detach())
    }

    fn init_repo(path: &std::path::Path) -> crate::Result<gix::Repository> {
        Ok(gix::ThreadSafeRepository::init_opts(
            path,
            gix::create::Kind::WithWorktree,
            gix::create::Options::default(),
            crate::restricted(),
        )?
        .to_thread_local())
    }

    fn shallow_ids(repo: &gix::Repository, expected: &'static str) -> crate::Result<Vec<gix::ObjectId>> {
        let commits = repo.shallow_commits()?.expect(expected);
        Ok(std::iter::once(commits.head)
            .chain(commits.tail.iter().copied())
            .collect())
    }

    #[test]
    #[cfg(feature = "blocking-network-client")]
    fn fetch_more_packs_than_can_be_handled() -> gix_testtools::Result {
        use gix::{config::tree::User, interrupt::IS_INTERRUPTED};
        use gix_odb::store::init::Slots;
        use gix_testtools::tempfile;
        fn create_empty_commit(repo: &gix::Repository) -> anyhow::Result<()> {
            let name = repo.head_name()?.expect("no detached head");
            repo.commit(
                name.as_bstr(),
                "empty",
                gix::hash::ObjectId::empty_tree(repo.object_hash()),
                repo.try_find_reference(name.as_ref())?.map(|r| r.id()),
            )?;
            Ok(())
        }
        fn check_fetch_output(
            repo: &gix::Repository,
            out: gix::remote::fetch::Outcome,
            expected_count: usize,
        ) -> gix_testtools::Result {
            for local_tracking_branch_name in out.ref_map.mappings.into_iter().filter_map(|m| m.local) {
                let r = repo.find_reference(&local_tracking_branch_name)?;
                r.id()
                    .object()
                    .expect("object should be present after fetching, triggering pack refreshes works");
                repo.head_ref()?.unwrap().set_target_id(r.id(), "post fetch")?;
            }
            check_odb_accessability(repo, expected_count)?;
            Ok(())
        }
        fn check_odb_accessability(repo: &gix::Repository, expected_count: usize) -> gix_testtools::Result {
            let mut count_unique = 0;
            // TODO: somehow there is a lot of duplication when receiving objects.
            let mut seen = gix_hashtable::HashSet::default();
            for id in repo.objects.iter()? {
                let id = id?;
                if !seen.insert(id) {
                    continue;
                }
                let _obj = repo.find_object(id)?;
                count_unique += 1;
            }
            assert_eq!(
                count_unique, expected_count,
                "Each round we receive exactly one commit, effectively"
            );
            Ok(())
        }
        for max_packs in 1..=3 {
            let remote_dir = tempfile::tempdir()?;
            let mut remote_repo = gix::init_bare(remote_dir.path())?;
            {
                let mut config = remote_repo.config_snapshot_mut();
                config.set_value(&User::NAME, "author")?;
                config.set_value(&User::EMAIL, "email@example.com")?;
            }
            create_empty_commit(&remote_repo)?;

            let local_dir = tempfile::tempdir()?;
            let (local_repo, _) = gix::clone::PrepareFetch::new(
                remote_repo.path(),
                local_dir.path(),
                gix::create::Kind::Bare,
                Default::default(),
                gix::open::Options::isolated().object_store_slots(Slots::Given(max_packs)),
            )?
            .fetch_only(gix::progress::Discard, &IS_INTERRUPTED)?;

            let remote = local_repo
                .branch_remote(
                    local_repo.head_ref()?.expect("branch available").name().shorten(),
                    Fetch,
                )
                .expect("remote is configured after clone")?;
            let minimum_slots = 5;
            let slots = Slots::AsNeededByDiskState {
                multiplier: 1.1,
                minimum: minimum_slots,
            };
            let one_more_than_minimum = minimum_slots + 1;
            for round_to_create_pack in 1..one_more_than_minimum {
                let expected_object_count = round_to_create_pack + 1 + 1 /* first commit + tree */;
                create_empty_commit(&remote_repo)?;
                match remote
                    .connect(Fetch)?
                    .prepare_fetch(gix::progress::Discard, Default::default())?
                    .receive(gix::progress::Discard, &IS_INTERRUPTED)
                {
                    Ok(out) => check_fetch_output(&local_repo, out, expected_object_count)?,
                    Err(err) => {
                        assert!(
                            err.to_string()
                                .starts_with("The slotmap turned out to be too small with ")
                        );
                        // But opening a new repo will always be able to read all objects
                        // as it dynamically sizes the otherwise static slotmap.
                        let local_repo = gix::open_opts(
                            local_repo.path(),
                            gix::open::Options::isolated().object_store_slots(slots),
                        )?;
                        check_odb_accessability(&local_repo, expected_object_count)?;
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    #[cfg(feature = "blocking-network-client")]
    fn collate_fetch_error() -> Result<(), gix::env::collate::fetch::Error> {
        let (repo, _tmp) = try_repo_rw("two-origins")?;
        let remote = repo
            .head()?
            .into_remote(Fetch)
            .expect("present")?
            .with_fetch_tags(fetch::Tags::Included);

        remote
            .connect(Fetch)?
            .prepare_fetch(gix::progress::Discard, Default::default())?
            .receive(gix::progress::Discard, &AtomicBool::default())?;

        assert!(
            repo.path()
                .join("HEAD")
                .metadata()
                .map_err(gix::Error::from_error)?
                .is_file(),
            "just to show off the 'Other' error type"
        );
        Ok(())
    }

    #[test]
    #[cfg(feature = "blocking-network-client")]
    fn fetch_with_alternates_adds_tips_from_alternates() -> crate::Result<()> {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let remote_repo = remote::repo("base");
        let (_repo, out) = gix::clone::PrepareFetch::new(
            remote::repo("multi_round/server").path(),
            tmp.path(),
            gix::create::Kind::Bare,
            Default::default(),
            gix::open::Options::isolated(),
        )?
        .configure_remote({
            move |r| {
                std::fs::write(
                    r.repo().objects.store_ref().path().join("info").join("alternates"),
                    format!(
                        "{}\n",
                        gix::path::realpath(remote_repo.objects.store_ref().path())?.display()
                    )
                    .as_bytes(),
                )?;
                Ok(r)
            }
        })
        .fetch_only(gix::progress::Discard, &AtomicBool::default())?;

        match out.status {
            Status::Change {
                negotiate,
                write_pack_bundle,
                ..
            } => {
                assert_eq!(
                    negotiate.rounds.len(),
                    1,
                    "we don't really have a way to see that tips from alternates were added, I think"
                );
                assert_eq!(
                    write_pack_bundle.index.num_objects, 66,
                    "this test just exercises code for adding alternate-repo tips to the negotiator"
                );
            }
            _ => unreachable!("we get a pack as alternates are unrelated"),
        }
        Ok(())
    }

    #[bisync::bisync]
    #[cfg_attr(feature = "blocking-network-client", test)]
    #[cfg_attr(feature = "async-network-client-async-std", async_std::test)]
    async fn local_transport_fetches_head_against_remote_refs() -> crate::Result {
        let remote_dir = TempDir::new()?;
        let remote_repo = init_repo(remote_dir.path())?;
        let remote_main = commit_empty(&remote_repo, "remote-main")?;

        let local_dir = TempDir::new()?;
        let local_repo = init_repo(local_dir.path())?;
        let local_main = commit_empty(&local_repo, "local-main")?;
        assert_ne!(
            remote_main, local_main,
            "the regression requires matching refnames to resolve to different objects locally and remotely"
        );

        let daemon = spawn_git_daemon_if_async(remote_repo.workdir().expect("non-bare"))?;
        let mut remote = into_daemon_remote_if_async(
            local_repo
                .remote_at(remote_repo.workdir().expect("non-bare"))?
                .with_fetch_tags(fetch::Tags::None),
            daemon.as_ref(),
            None,
        );
        remote.replace_refspecs(Some("+HEAD:refs/test/repo"), Fetch)?;

        let outcome = remote
            .connect(Fetch)
            .await?
            .prepare_fetch(gix::progress::Discard, Default::default())
            .await?
            .receive(gix::progress::Discard, &AtomicBool::default())
            .await?;

        assert!(
            matches!(outcome.status, Status::Change { .. }),
            "the remote object is missing locally and must be fetched"
        );
        let fetched = local_repo.find_reference("refs/test/repo")?;
        assert_eq!(
            fetched.target().try_id(),
            Some(remote_main.as_ref()),
            "HEAD must resolve through the remote's symbolic target, not through the local ref with the same name"
        );
        assert_eq!(
            local_repo.find_reference("refs/heads/main")?.id(),
            local_main,
            "the local HEAD branch remains distinct from the fetched remote HEAD"
        );
        Ok(())
    }

    #[bisync::bisync]
    #[cfg_attr(feature = "blocking-network-client", test)]
    #[cfg_attr(feature = "async-network-client-async-std", async_std::test)]
    async fn fetch_with_multi_round_negotiation() -> crate::Result {
        for (algorithm, expected_negotiation_rounds) in [
            (gix::negotiate::Algorithm::Consecutive, 4),
            (gix::negotiate::Algorithm::Skipping, 2),
        ] {
            for version in [
                gix::protocol::transport::Protocol::V1,
                gix::protocol::transport::Protocol::V2,
            ] {
                let (mut client_repo, _tmp) = {
                    let client_repo = remote::repo("multi_round/client");
                    let daemon = spawn_git_daemon_if_async(client_repo.workdir().expect("non-bare"))?;
                    let tmp = TempDir::new()?;
                    let repo = gix::prepare_clone_bare(
                        daemon.as_ref().map_or_else(
                            || client_repo.git_dir().to_owned(),
                            |d| std::path::PathBuf::from(format!("{}/", d.url)),
                        ),
                        tmp.path(),
                    )?
                    .fetch_only(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())
                    .await?
                    .0;
                    (repo, tmp)
                };

                {
                    let mut config = client_repo.config_snapshot_mut();
                    config.set_value(
                        &gix::config::tree::Protocol::VERSION,
                        (version as u8).to_string().as_str(),
                    )?;
                    config.set_value(
                        &gix::config::tree::Fetch::NEGOTIATION_ALGORITHM,
                        algorithm.to_string().as_str(),
                    )?;
                }
                let server_repo = remote::repo("multi_round/server");
                let daemon = spawn_git_daemon_if_async(server_repo.workdir().expect("non-bare"))?;
                let remote = into_daemon_remote_if_async(
                    client_repo.remote_at(server_repo.workdir().expect("non-bare"))?,
                    daemon.as_ref(),
                    None,
                );
                let changes = remote
                    .with_refspecs(Some("refs/heads/*:refs/remotes/origin/*"), Fetch)?
                    .connect(Fetch)
                    .await?
                    .prepare_fetch(gix::progress::Discard, Default::default())
                    .await?
                    .receive(gix::progress::Discard, &AtomicBool::default())
                    .await?;

                match changes.status {
                    Status::Change {
                        write_pack_bundle,
                        negotiate,
                        ..
                    } => {
                        assert_eq!(
                            negotiate.rounds.len(),
                            expected_negotiation_rounds,
                            "we need multiple rounds"
                        );
                        // the server only has our `b1` and an extra commit or two.
                        assert_eq!(
                            write_pack_bundle.index.num_objects, 7,
                            "this is the number git gets as well, we are quite perfectly aligned :)"
                        );
                    }
                    _ => unreachable!("We expect a pack for sure"),
                }
            }
        }
        Ok(())
    }

    #[bisync::bisync]
    #[cfg_attr(feature = "blocking-network-client", test)]
    #[cfg_attr(feature = "async-network-client-async-std", async_std::test)]
    async fn fetch_shallow_deepen_zero_does_not_fail() -> crate::Result {
        let (repo, tmp) = try_repo_rw_args("two-origins", ["--depth=2"], Mode::CloneWithShallowSupport)?;
        let daemon = spawn_git_daemon_if_async(tmp.path().join("base"))?;
        let remote = into_daemon_remote_if_async(
            repo.head()?
                .into_remote(Fetch)
                .expect("present")?
                .with_fetch_tags(fetch::Tags::Included),
            daemon.as_ref(),
            None,
        );

        let prev_commits = repo.head_id()?.ancestors().all()?.count();

        let changes = remote
            .connect(Fetch)
            .await?
            .prepare_fetch(gix::progress::Discard, Default::default())
            .await?
            .with_shallow(fetch::Shallow::Deepen(0))
            .receive(gix::progress::Discard, &AtomicBool::default())
            .await?;

        assert!(
            matches!(changes.status, Status::NoPackReceived { .. }),
            "we didn't negotiate at all, but ran all other ref updates"
        );
        assert_eq!(
            repo.head_id()?.ancestors().all()?.count(),
            prev_commits,
            "no more commits are available - we didn't fetch anything and there would be nothing to fetch"
        );
        Ok(())
    }

    #[bisync::bisync]
    #[cfg_attr(feature = "blocking-network-client", test)]
    #[cfg_attr(feature = "async-network-client-async-std", async_std::test)]
    async fn fetch_shallow_deepen_not_possible() -> crate::Result {
        let (repo, tmp) = try_repo_rw_args("two-origins", ["--depth=2"], Mode::CloneWithShallowSupport)?;
        let daemon = spawn_git_daemon_if_async(tmp.path().join("base"))?;
        let remote = into_daemon_remote_if_async(
            repo.head()?
                .into_remote(Fetch)
                .expect("present")?
                .with_fetch_tags(fetch::Tags::Included),
            daemon.as_ref(),
            None,
        );

        assert_eq!(
            shallow_ids(&repo, "shallow clone")?,
            [
                hex_to_id("2d9d136fb0765f2e24c44a0f91984318d580d03b"),
                hex_to_id("dfd0954dabef3b64f458321ef15571cc1a46d552"),
                hex_to_id("dfd0954dabef3b64f458321ef15571cc1a46d552")
            ]
        );
        let prev_commits = repo.head_id()?.ancestors().all()?.count();
        let changes = remote
            .connect(Fetch)
            .await?
            .prepare_fetch(gix::progress::Discard, Default::default())
            .await?
            .with_shallow(fetch::Shallow::Deepen(1))
            .receive(gix::progress::Discard, &AtomicBool::default())
            .await?;

        assert!(
            matches!(changes.status, Status::Change {write_pack_bundle, ..} if write_pack_bundle.index.num_objects == 0),
            "we get an empty pack as there is nothing to do"
        );

        assert_eq!(
            shallow_ids(&repo, "shallow clone")?,
            [
                hex_to_id("2d9d136fb0765f2e24c44a0f91984318d580d03b"),
                hex_to_id("dfd0954dabef3b64f458321ef15571cc1a46d552"),
                hex_to_id("dfd0954dabef3b64f458321ef15571cc1a46d552")
            ],
            "the base is shallow, and so is the clone, and we can't extend further"
        );
        assert_eq!(
            repo.head_id()?.ancestors().all()?.count(),
            prev_commits,
            "no more commits are available - there simply isn't more information"
        );
        Ok(())
    }

    #[bisync::bisync]
    #[cfg_attr(feature = "blocking-network-client", test)]
    #[cfg_attr(feature = "async-network-client-async-std", async_std::test)]
    async fn fetch_empty_pack() -> crate::Result {
        for version in [
            gix::protocol::transport::Protocol::V1,
            gix::protocol::transport::Protocol::V2,
        ] {
            for (shallow_args, expected) in [(None, [1, 7, 7]), (Some("--depth=2"), [1, 2, 2])] {
                if version == gix::protocol::transport::Protocol::V1 && shallow_args.is_some() {
                    // TODO: We cannot yet handle shallow mode for V1 as it will send shallow-info as part of the handshake :/.
                    //       It's probably not the most important thing to be able to clone from a shallow remote anyway.
                    continue;
                }
                for (fetch_tags, expected_ref_count) in [fetch::Tags::None, fetch::Tags::Included, fetch::Tags::All]
                    .into_iter()
                    .zip(expected)
                {
                    let (mut repo, _tmp) = try_repo_rw_args(
                        "two-origins",
                        shallow_args,
                        if shallow_args.is_some() {
                            Mode::CloneWithShallowSupport
                        } else {
                            Mode::FastClone
                        },
                    )?;
                    let daemon = spawn_git_daemon_if_async(
                        repo.workdir()
                            .expect("non-bare")
                            .ancestors()
                            .nth(1)
                            .expect("parent")
                            .join("base"),
                    )?;
                    repo.config_snapshot_mut().set_value(
                        &gix::config::tree::Protocol::VERSION,
                        (version as u8).to_string().as_str(),
                    )?;
                    let mut remote = into_daemon_remote_if_async(
                        repo.head()?
                            .into_remote(Fetch)
                            .expect("present")?
                            .with_fetch_tags(fetch_tags),
                        daemon.as_ref(),
                        None,
                    );
                    remote.replace_refspecs(Some("HEAD:refs/remotes/origin/does-not-yet-exist"), Fetch)?;

                    let res = remote
                        .connect(Fetch)
                        .await?
                        .prepare_fetch(gix::progress::Discard, Default::default())
                        .await?
                        .receive(gix::progress::Discard, &AtomicBool::default())
                        .await?;

                    match res.status {
                        fetch::Status::NoPackReceived {
                            update_refs,
                            negotiate: _,
                            dry_run,
                        } => {
                            assert_eq!(
                                update_refs.edits.len(),
                                expected_ref_count,
                                "{shallow_args:?}|{fetch_tags:?}"
                            );
                            assert!(!dry_run, "we actually perform the operation");
                        }
                        _ => unreachable!(
                            "{shallow_args:?}|{fetch_tags:?}: default negotiation is able to realize nothing is required and doesn't get to receiving a pack"
                        ),
                    }
                }
            }
        }
        Ok(())
    }

    #[bisync::bisync]
    #[cfg_attr(feature = "blocking-network-client", test)]
    #[cfg_attr(feature = "async-network-client-async-std", async_std::test)]
    async fn fetch_pack_without_local_destination() -> crate::Result {
        let daemon = spawn_git_daemon_if_async(repo_path("clone-as-base-with-changes"))?;
        for (fetch_tags, expected_pack_objects, expected_ref_edits) in [
            (gix::remote::fetch::Tags::None, PACK_OBJECTS_WITHOUT_TAG, 0),
            (gix::remote::fetch::Tags::Included, PACK_OBJECTS_WITH_TAG, 7),
            (gix::remote::fetch::Tags::All, PACK_OBJECTS_WITH_TAG, 7),
        ] {
            let (repo, _tmp) = repo_rw("two-origins");
            let mut remote = into_daemon_remote_if_async(
                repo.find_remote("changes-on-top-of-origin")?
                    .with_fetch_tags(fetch_tags),
                daemon.as_ref(),
                None,
            );
            remote.replace_refspecs(Some("HEAD"), Fetch)?;

            let res: gix::remote::fetch::Outcome = remote
                .connect(Fetch)
                .await?
                .prepare_fetch(gix::progress::Discard, Default::default())
                .await?
                .receive(gix::progress::Discard, &AtomicBool::default())
                .await?;

            match res.status {
                gix::remote::fetch::Status::Change {
                    write_pack_bundle,
                    update_refs,
                    negotiate,
                } => {
                    assert_eq!(negotiate.rounds.len(), 1);
                    assert_eq!(
                        pack_object_ids(write_pack_bundle.index_path.as_deref(), repo.object_hash()),
                        expected_pack_object_ids(expected_pack_objects),
                        "{fetch_tags:?}: the pack contains exactly the expected objects"
                    );
                    assert!(
                        write_pack_bundle
                            .data_path
                            .as_deref()
                            .is_some_and(std::path::Path::is_file)
                    );
                    assert!(
                        write_pack_bundle
                            .index_path
                            .as_deref()
                            .is_some_and(std::path::Path::is_file)
                    );
                    assert_eq!(update_refs.edits.len(), expected_ref_edits, "{fetch_tags:?}");
                    assert_eq!(
                        write_pack_bundle
                            .keep_path
                            .as_deref()
                            .is_some_and(std::path::Path::is_file),
                        update_refs.edits.is_empty(),
                        ".keep are kept if there was no edit to prevent `git gc` from clearing out the pack as it's not referred to necessarily"
                    );
                }
                _ => unreachable!(
                    "Naive negotiation sends the same have and wants, resulting in an empty pack (technically no change, but we don't detect it) - empty packs are fine"
                ),
            }
        }
        Ok(())
    }

    #[bisync::bisync]
    #[cfg_attr(feature = "blocking-network-client", test)]
    #[cfg_attr(feature = "async-network-client-async-std", async_std::test)]
    async fn fetching_a_missing_explicit_ref_fails_even_if_ls_refs_returns_nothing() -> crate::Result {
        let daemon = spawn_git_daemon_if_async({
            let mut p = repo_path("base");
            p.pop();
            p
        })?;
        let (repo, _tmp) = repo_rw("two-origins");
        let mut remote = into_daemon_remote_if_async(
            repo.find_remote("origin")?.with_fetch_tags(fetch::Tags::None),
            daemon.as_ref(),
            "base",
        );
        remote.replace_refspecs(Some("refs/heads/does-not-exist"), Fetch)?;

        let err = remote
            .connect(Fetch)
            .await?
            .prepare_fetch(gix::progress::Discard, Default::default())
            .await?
            .receive(gix::progress::Discard, &AtomicBool::default())
            .await
            .unwrap_err();

        assert_eq!(
            err.to_string(),
            "None of the refspec(s) refs/heads/does-not-exist matched any of the 0 refs on the remote"
        );
        Ok(())
    }

    #[bisync::bisync]
    #[cfg_attr(feature = "blocking-network-client", test)]
    #[cfg_attr(feature = "async-network-client-async-std", async_std::test)]
    async fn fetch_pack() -> crate::Result {
        let daemon = spawn_git_daemon_if_async({
            let mut p = repo_path("base");
            p.pop();
            p
        })?;
        for version in [
            None,
            Some(gix::protocol::transport::Protocol::V2),
            Some(gix::protocol::transport::Protocol::V1),
        ] {
            let (mut repo, _tmp) = repo_rw("two-origins");
            if let Some(version) = version {
                repo.config_snapshot_mut()
                    .set_raw_value(Protocol::VERSION, (version as u8).to_string().as_str())?;
            }

            // No updates
            {
                let remote = into_daemon_remote_if_async(repo.find_remote("origin")?, daemon.as_ref(), "base");
                {
                    remote
                        .connect(Fetch)
                        .await?
                        .prepare_fetch(progress::Discard, Default::default())
                        .await?;
                    // early drops are fine and won't block.
                }
                let outcome = remote
                    .connect(Fetch)
                    .await?
                    .prepare_fetch(progress::Discard, Default::default())
                    .await?
                    .receive(progress::Discard, &AtomicBool::default())
                    .await?;
                assert!(matches!(
                    outcome.status,
                    gix::remote::fetch::Status::NoPackReceived { .. }
                ));
            }

            // Some updates to be fetched
            for dry_run in [true, false] {
                let remote = into_daemon_remote_if_async(
                    repo.find_remote("changes-on-top-of-origin")?
                        .with_fetch_tags(gix::remote::fetch::Tags::None),
                    daemon.as_ref(),
                    "clone-as-base-with-changes",
                );
                let outcome: gix::remote::fetch::Outcome = remote
                    .connect(Fetch)
                    .await?
                    .prepare_fetch(progress::Discard, Default::default())
                    .await?
                    .with_dry_run(dry_run)
                    .receive(progress::Discard, &AtomicBool::default())
                    .await?;
                let refs = match outcome.status {
                    fetch::Status::Change {
                        write_pack_bundle,
                        update_refs,
                        negotiate,
                    } => {
                        assert_eq!(negotiate.rounds.len(), 1);
                        assert_eq!(write_pack_bundle.pack_version, gix::odb::pack::data::Version::V2);
                        assert_eq!(write_pack_bundle.object_hash, repo.object_hash());
                        assert_eq!(
                            write_pack_bundle.index.index_version,
                            gix::odb::pack::index::Version::V2
                        );
                        assert_eq!(
                            pack_object_ids(write_pack_bundle.index_path.as_deref(), repo.object_hash()),
                            expected_pack_object_ids(PACK_OBJECTS_WITH_TAG),
                            "{dry_run}: these 4 objects are what git sends with 'consecutive' negotiation style; \
                             a completely naive negotiation would send 33"
                        );
                        assert!(write_pack_bundle.data_path.is_some_and(|f| f.is_file()));
                        assert!(write_pack_bundle.index_path.is_some_and(|f| f.is_file()));
                        assert_eq!(update_refs.edits.len(), 2);

                        let edit = &update_refs.edits[0];
                        assert_eq!(edit.name.as_bstr(), "refs/remotes/changes-on-top-of-origin/main");
                        assert!(
                            edit.change.new_value().expect("no deletion").try_id().is_some(),
                            "a simple peeled ref"
                        );
                        let edit = &update_refs.edits[1];
                        assert_eq!(edit.name.as_bstr(), "refs/remotes/changes-on-top-of-origin/symbolic");
                        match version.unwrap_or_default() {
                            gix::protocol::transport::Protocol::V2 => {
                                assert!(
                                    edit.change.new_value().expect("no deletion").try_name().is_none(),
                                    "{version:?} on the remote this is a symbolic ref to a tag, but we don't pull tags, hence we point to the tag object itself (instead of the refname)"
                                );
                            }
                            gix::protocol::transport::Protocol::V1 => {
                                assert!(
                                    edit.change.new_value().expect("no deletion").try_id().is_some(),
                                    "on the remote this is a symbolic ref, but in V1 symrefs are never visible"
                                );
                            }
                            gix::protocol::transport::Protocol::V0 => {
                                unreachable!("we don't test this here as there is no need")
                            }
                        }

                        assert!(
                            !write_pack_bundle.keep_path.is_some_and(|f| f.is_file()),
                            ".keep files are deleted if there is one edit"
                        );

                        update_refs
                    }
                    fetch::Status::NoPackReceived {
                        dry_run,
                        update_refs,
                        negotiate: _,
                    } => {
                        assert!(
                            dry_run,
                            "the only reason we receive no pack is if we are in dry-run mode"
                        );
                        update_refs
                    }
                };

                assert_eq!(
                    refs.updates,
                    vec![
                        fetch::refs::Update {
                            mode: fetch::refs::update::Mode::New,
                            type_change: None,
                            edit_index: Some(0),
                        },
                        fetch::refs::Update {
                            mode: fetch::refs::update::Mode::New,
                            type_change: None,
                            edit_index: Some(1),
                        }
                    ]
                );
                for (_update, mapping, _spec, edit) in refs.iter_mapping_updates(
                    &outcome.ref_map.mappings,
                    remote.refspecs(Fetch),
                    &outcome.ref_map.extra_refspecs,
                ) {
                    let edit = edit.expect("refedit present even if it's a no-op");
                    if dry_run {
                        match edit.change.new_value().expect("no deletions") {
                            gix_ref::TargetRef::Object(id) => {
                                assert_eq!(id, mapping.remote.as_id().expect("no unborn"));
                            }
                            gix_ref::TargetRef::Symbolic(target) => {
                                assert_eq!(target.as_bstr(), mapping.remote.as_target().expect("no direct ref"));
                            }
                        }
                        assert!(
                            repo.try_find_reference(edit.name.as_ref())?.is_none(),
                            "no ref created in dry-run mode"
                        );
                    } else {
                        let r = repo.find_reference(edit.name.as_ref()).unwrap();
                        match r.target() {
                            gix_ref::TargetRef::Object(id) => {
                                assert_eq!(
                                    id,
                                    mapping.remote.as_id().expect("no unborn"),
                                    "local reference should point to remote id"
                                );
                            }
                            gix_ref::TargetRef::Symbolic(target) => {
                                assert_eq!(target.as_bstr(), mapping.remote.as_target().expect("no direct ref"));
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
