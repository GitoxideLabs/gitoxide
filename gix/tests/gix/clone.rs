use crate::{remote, util::restricted};

#[cfg(all(feature = "worktree-mutation", feature = "blocking-network-client"))]
mod blocking_io {
    use std::path::Path;
    use std::{borrow::Cow, sync::atomic::AtomicBool};

    use gix::{
        bstr::BString,
        config::tree::{Clone, Core, Init, Key},
        remote::{
            fetch::{refmap::SpecIndex, Shallow},
            Direction,
        },
    };
    use gix_object::bstr::ByteSlice;
    use gix_ref::TargetRef;

    use crate::{
        remote,
        util::{hex_to_id, restricted},
    };

    #[test]
    fn fetch_shallow_no_checkout_then_unshallow() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let called_configure_remote = std::sync::Arc::new(std::sync::atomic::AtomicBool::default());
        let remote_name = "special";
        let desired_fetch_tags = gix::remote::fetch::Tags::Included;
        let mut prepare = gix::prepare_clone_bare(remote::repo("base").path(), tmp.path())?
            .with_remote_name(remote_name)?
            .configure_remote({
                move |r| {
                    called_configure_remote.store(true, std::sync::atomic::Ordering::Relaxed);
                    let mut r = r.with_fetch_tags(desired_fetch_tags);
                    r.replace_refspecs(
                        [
                            BString::from(format!("refs/heads/main:refs/remotes/{remote_name}/main")),
                            "+refs/tags/b-tag:refs/tags/b-tag".to_owned().into(),
                        ],
                        Direction::Fetch,
                    )?;
                    Ok(r)
                }
            })
            .with_shallow(Shallow::DepthAtRemote(2.try_into().expect("non-zero")));
        let (repo, _out) = prepare.fetch_only(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;
        drop(prepare);

        assert_eq!(
            repo.shallow_commits()?.expect("shallow").as_slice(),
            [
                hex_to_id("27e71576a6335294aa6073ab767f8b36bdba81d0"),
                hex_to_id("2d9d136fb0765f2e24c44a0f91984318d580d03b"),
                hex_to_id("82024b2ef7858273337471cbd1ca1cedbdfd5616"),
                hex_to_id("b5152869aedeb21e55696bb81de71ea1bb880c85")
            ],
            "shallow information is written"
        );

        let shallow_commit_count = repo.head_id()?.ancestors().all()?.count();
        let remote = repo.head()?.into_remote(Direction::Fetch).expect("present")?;

        remote
            .connect(Direction::Fetch)?
            .prepare_fetch(gix::progress::Discard, Default::default())?
            .with_shallow(Shallow::undo())
            .receive(gix::progress::Discard, &AtomicBool::default())?;

        assert!(repo.shallow_commits()?.is_none(), "the repo isn't shallow anymore");
        assert!(
            !repo.is_shallow(),
            "both methods agree - if there are no shallow commits, it shouldn't think the repo is shallow"
        );
        assert!(
            !repo.shallow_file().exists(),
            "when the repo is not shallow anymore, there is no need for a shallow file"
        );
        assert!(
            repo.head_id()?.ancestors().all()?.count() > shallow_commit_count,
            "there are more commits now as the history is complete"
        );

        Ok(())
    }

    #[test]
    fn from_shallow_prohibited_with_option() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let err = gix::clone::PrepareFetch::new(
            remote::repo("base.shallow").path(),
            tmp.path(),
            gix::create::Kind::Bare,
            Default::default(),
            gix::open::Options::isolated().config_overrides([Clone::REJECT_SHALLOW.validated_assignment_fmt(&true)?]),
        )?
        .fetch_only(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())
        .unwrap_err();
        assert!(
            matches!(
                err,
                gix::clone::fetch::Error::Fetch(gix::remote::fetch::Error::Fetch(
                    gix_protocol::fetch::Error::RejectShallowRemote
                ))
            ),
            "we can avoid fetching from remotes with this setting"
        );
        Ok(())
    }

    #[test]
    fn from_shallow_allowed_by_default() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let (repo, _change) = gix::prepare_clone_bare(remote::repo("base.shallow").path(), tmp.path())?
            .with_in_memory_config_overrides(Some("my.marker=1"))
            .fetch_only(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;
        assert_eq!(
            repo.shallow_commits()?.expect("present").as_slice(),
            vec![
                hex_to_id("2d9d136fb0765f2e24c44a0f91984318d580d03b"),
                hex_to_id("dfd0954dabef3b64f458321ef15571cc1a46d552"),
                hex_to_id("dfd0954dabef3b64f458321ef15571cc1a46d552"),
            ]
        );
        assert_eq!(
            repo.config_snapshot().boolean("my.marker"),
            Some(true),
            "configuration overrides are set in time"
        );
        assert_eq!(
            gix::open_opts(repo.git_dir(), gix::open::Options::isolated())?
                .config_snapshot()
                .boolean("my.marker"),
            None,
            "these options are not persisted"
        );
        Ok(())
    }

    #[test]
    fn from_non_shallow_then_deepen_then_deepen_since_to_unshallow() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let (repo, _change) = gix::prepare_clone_bare(remote::repo("base").path(), tmp.path())?
            .with_shallow(Shallow::DepthAtRemote(2.try_into()?))
            .configure_remote(|mut r| {
                r.replace_refspecs(Some("refs/heads/main:refs/remotes/origin/main"), Direction::Fetch)?;
                Ok(r)
            })
            .fetch_only(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;

        assert!(repo.is_shallow());
        assert_eq!(
            repo.shallow_commits()?.expect("present").as_slice(),
            vec![
                hex_to_id("2d9d136fb0765f2e24c44a0f91984318d580d03b"),
                hex_to_id("dfd0954dabef3b64f458321ef15571cc1a46d552"),
            ]
        );

        let shallow_commit_count = repo.head_id()?.ancestors().all()?.count();

        let remote = repo.head()?.into_remote(Direction::Fetch).expect("present")?;
        remote
            .connect(Direction::Fetch)?
            .prepare_fetch(gix::progress::Discard, Default::default())?
            .with_shallow(Shallow::Deepen(1))
            .receive(gix::progress::Discard, &AtomicBool::default())?;

        assert_eq!(
            repo.shallow_commits()?.expect("present").as_slice(),
            vec![
                hex_to_id("27e71576a6335294aa6073ab767f8b36bdba81d0"),
                hex_to_id("82024b2ef7858273337471cbd1ca1cedbdfd5616"),
                hex_to_id("b5152869aedeb21e55696bb81de71ea1bb880c85"),
            ],
            "the shallow boundary was changed"
        );
        assert!(
            repo.head_id()?.ancestors().all()?.count() > shallow_commit_count,
            "there are more commits now as the history was deepened"
        );

        let shallow_commit_count = repo.head_id()?.ancestors().all()?.count();
        remote
            .connect(Direction::Fetch)?
            .prepare_fetch(gix::progress::Discard, Default::default())?
            .with_shallow(Shallow::Since {
                cutoff: gix::date::Time::new(1112354053, 0),
            })
            .receive(gix::progress::Discard, &AtomicBool::default())?;

        assert!(
            !repo.is_shallow(),
            "the cutoff date is before the first commit, effectively unshallowing"
        );
        assert!(
            repo.head_id()?.ancestors().all()?.count() > shallow_commit_count,
            "there is even more commits than previously"
        );
        Ok(())
    }

    #[test]
    fn from_non_shallow_by_deepen_exclude_then_deepen_to_unshallow() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let excluded_leaf_refs = ["g", "h", "j"];
        let (repo, _change) = gix::prepare_clone_bare(remote::repo("base").path(), tmp.path())?
            .with_shallow(Shallow::Exclude {
                remote_refs: excluded_leaf_refs
                    .into_iter()
                    .map(|n| n.try_into().expect("valid"))
                    .collect(),
                since_cutoff: None,
            })
            .fetch_only(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;

        assert!(repo.is_shallow());
        assert_eq!(
            repo.shallow_commits()?.expect("present").as_slice(),
            vec![
                hex_to_id("27e71576a6335294aa6073ab767f8b36bdba81d0"),
                hex_to_id("82024b2ef7858273337471cbd1ca1cedbdfd5616"),
            ]
        );

        let remote = repo.head()?.into_remote(Direction::Fetch).expect("present")?;
        remote
            .connect(Direction::Fetch)?
            .prepare_fetch(gix::progress::Discard, Default::default())?
            .with_shallow(Shallow::Deepen(2))
            .receive(gix::progress::Discard, &AtomicBool::default())?;

        assert!(!repo.is_shallow(), "one is just enough to unshallow it");
        Ok(())
    }

    #[test]
    fn fetch_only_with_configuration() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let called_configure_remote = std::sync::Arc::new(std::sync::atomic::AtomicBool::default());
        let remote_name = "special";
        let desired_fetch_tags = gix::remote::fetch::Tags::Included;
        let mut prepare = gix::clone::PrepareFetch::new(
            remote::repo("base").path(),
            tmp.path(),
            gix::create::Kind::Bare,
            Default::default(),
            gix::open::Options::isolated().config_overrides([
                Init::DEFAULT_BRANCH.validated_assignment_fmt(&"unused-as-overridden-by-remote")?,
                Core::LOG_ALL_REF_UPDATES.logical_name().into(),
                // missing user and email is acceptable in this special case, i.e. `git` also doesn't mind filling it in.
            ]),
        )?
        .with_remote_name(remote_name)?
        .configure_remote({
            let called_configure_remote = called_configure_remote.clone();
            move |r| {
                called_configure_remote.store(true, std::sync::atomic::Ordering::Relaxed);
                let r = r
                    .with_refspecs(Some("+refs/tags/b-tag:refs/tags/b-tag"), gix::remote::Direction::Fetch)?
                    .with_fetch_tags(desired_fetch_tags);
                Ok(r)
            }
        });
        let (repo, out) = prepare.fetch_only(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;
        drop(prepare);

        assert!(
            called_configure_remote.load(std::sync::atomic::Ordering::Relaxed),
            "custom remote configuration is called"
        );
        assert_eq!(repo.remote_names().len(), 1, "only ever one remote");
        let remote = repo.find_remote(remote_name)?;
        let num_refspecs = remote.refspecs(gix::remote::Direction::Fetch).len();
        assert_eq!(
            num_refspecs, 2,
            "our added spec was stored as well, but no implied specs due to the `Tags::All` setting"
        );
        assert_eq!(
            remote.fetch_tags(),
            desired_fetch_tags,
            "fetch-tags are persisted via the 'tagOpt` key"
        );
        assert!(
            gix::path::from_bstr(Cow::Borrowed(
                remote
                    .url(gix::remote::Direction::Fetch)
                    .expect("present")
                    .path
                    .as_ref()
            ))
            .is_absolute(),
            "file urls can't be relative paths"
        );

        let (explicit_max_idx, implicit_max_index) =
            out.ref_map
                .mappings
                .iter()
                .map(|m| m.spec_index)
                .fold((0, 0), |(a, b), i| match i {
                    SpecIndex::ExplicitInRemote(idx) => (idx.max(a), b),
                    SpecIndex::Implicit(idx) => (a, idx.max(b)),
                });
        assert_eq!(
            explicit_max_idx,
            num_refspecs - 1,
            "mappings don't refer to non-existing explicit refspecs"
        );
        assert_eq!(
            implicit_max_index,
            &out.ref_map.extra_refspecs.len() - 1,
            "mappings don't refer to non-existing implicit refspecs"
        );
        let packed_refs = repo
            .refs
            .cached_packed_buffer()?
            .expect("packed refs should be present");
        assert_eq!(
            repo.refs.loose_iter()?.count(),
            2,
            "HEAD and an actual symbolic ref we received"
        );
        assert_eq!(
            packed_refs.iter()?.count(),
            14,
            "all non-symbolic refs should be stored, if reachable from our refs"
        );
        let sig = repo
            .head()?
            .log_iter()
            .all()?
            .expect("present")
            .next()
            .expect("one line")?
            .signature
            .to_owned();
        assert_eq!(sig.name, "no name configured during clone");
        assert_eq!(sig.email, "noEmailAvailable@example.com");

        match out.status {
            gix::remote::fetch::Status::Change { update_refs, .. } => {
                for edit in &update_refs.edits {
                    use gix_object::Exists;
                    match edit.change.new_value().expect("always set/no deletion") {
                        TargetRef::Symbolic(referent) => {
                            assert!(
                                repo.find_reference(referent).is_ok(),
                                "if we set up a symref, the target should exist by now"
                            );
                        }
                        TargetRef::Object(id) => {
                            assert!(repo.objects.exists(id), "part of the fetched pack");
                        }
                    }
                    let r = repo
                        .find_reference(edit.name.as_ref())
                        .unwrap_or_else(|_| panic!("didn't find created reference: {edit:?}"));
                    if r.name().category().expect("known") != gix_ref::Category::Tag {
                        assert!(r
                            .name()
                            .category_and_short_name()
                            .expect("computable")
                            .1
                            .starts_with_str(remote_name));
                        match r.target() {
                            TargetRef::Object(_) => {
                                let mut logs = r.log_iter();
                                assert_reflog(logs.all());
                            }
                            TargetRef::Symbolic(_) => {
                                // TODO: it *should* be possible to set the reflog here based on the referent if deref = true
                                //       when setting up the edits. But it doesn't seem to work. Also, some tests are
                                //       missing for `leaf_referent_previous_oid`.
                                assert!(
                                    !r.log_exists(),
                                    "symbolic refs don't have object ids, so they can't get \
                                      into the reflog as these need previous and new oid"
                                );
                            }
                        }
                    }
                }
                let mut out_of_graph_tags = Vec::new();
                for mapping in update_refs
                    .updates
                    .iter()
                    .enumerate()
                    .filter(|(_, update)| {
                        matches!(
                            update.mode,
                            gix::remote::fetch::refs::update::Mode::ImplicitTagNotSentByRemote
                        )
                    })
                    .map(|(idx, _)| &out.ref_map.mappings[idx])
                {
                    out_of_graph_tags.push(
                        mapping
                            .remote
                            .as_name()
                            .expect("tag always has a path")
                            .to_str()
                            .expect("valid UTF8"),
                    );
                }
                assert_eq!(
                    out_of_graph_tags,
                    &[
                        "refs/tags/annotated-detached-tag",
                        "refs/tags/annotated-future-tag",
                        "refs/tags/detached-tag",
                        "refs/tags/future-tag"
                    ]
                );
            }
            _ => unreachable!("clones are always causing changes and dry-runs aren't possible"),
        }

        let remote_head = repo
            .find_reference(&format!("refs/remotes/{remote_name}/HEAD"))
            .expect("remote HEAD present");
        assert_eq!(
            remote_head
                .target()
                .try_name()
                .expect("remote HEAD is symbolic")
                .as_bstr(),
            format!("refs/remotes/{remote_name}/main"),
            "it points to the local tracking branch of what the remote actually points to"
        );

        let head = repo.head()?;
        {
            let mut logs = head.log_iter();
            assert_reflog(logs.all());
        }

        let referent = head.try_into_referent().expect("symbolic ref is present");
        assert!(
            referent.id().object().is_ok(),
            "the object pointed to by HEAD was fetched as well"
        );
        assert_eq!(
            referent.name().as_bstr(),
            remote::repo("base").head_name()?.expect("symbolic").as_bstr(),
            "local clone always adopts the name of the remote"
        );

        let ref_name = referent.name();
        assert_eq!(
            referent
                .remote_name(gix::remote::Direction::Fetch)
                .expect("remote is set")
                .as_ref(),
            remote_name,
            "the remote branch information is fully configured"
        );
        assert_eq!(
            repo.branch_remote_ref_name(ref_name, gix::remote::Direction::Fetch)
                .expect("present")?
                .as_bstr(),
            "refs/heads/main"
        );

        {
            let mut logs = referent.log_iter();
            assert_reflog(logs.all());
        }
        Ok(())
    }

    fn assert_reflog(log: std::io::Result<Option<gix_ref::file::log::iter::Forward<'_>>>) {
        let lines = log
            .unwrap()
            .expect("log present")
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(lines.len(), 1, "just created");
        let line = &lines[0];
        assert!(
            line.message.starts_with(b"clone: from "),
            "{:?} unexpected",
            line.message
        );
        let path = gix_path::from_bstr(line.message.rsplit(|b| *b == b' ').next().expect("path").as_bstr());
        assert!(path.is_absolute(), "{path:?} must be absolute");
    }

    #[test]
    fn fetch_and_checkout() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let mut prepare = gix::clone::PrepareFetch::new(
            remote::repo("base").path(),
            tmp.path(),
            gix::create::Kind::WithWorktree,
            Default::default(),
            restricted(),
        )?;
        let (mut checkout, _out) =
            prepare.fetch_then_checkout(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;
        let (repo, _) = checkout.main_worktree(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;

        let index = repo.index()?;
        assert_eq!(index.entries().len(), 1, "All entries are known as per HEAD tree");

        assure_index_entries_on_disk(&index, repo.workdir().expect("non-bare"));
        Ok(())
    }
    #[test]
    fn fetch_and_checkout_specific_ref() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let remote_repo = remote::repo("base");
        let ref_to_checkout = "a";
        let mut prepare = gix::clone::PrepareFetch::new(
            remote_repo.path(),
            tmp.path(),
            gix::create::Kind::WithWorktree,
            Default::default(),
            restricted(),
        )?
        .with_ref_name(Some(ref_to_checkout))?;
        let (mut checkout, _out) =
            prepare.fetch_then_checkout(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;

        let (repo, _) = checkout.main_worktree(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;

        assert_eq!(
            repo.references()?.all()?.count() - 2,
            remote_repo.references()?.all()?.count(),
            "all references have been cloned, + remote HEAD + remote main (not listed in remote_repo)"
        );
        let checked_out_ref = repo.head_ref()?.expect("head points to ref");
        let remote_ref_name = format!("refs/heads/{ref_to_checkout}");
        assert_eq!(
            checked_out_ref.name().as_bstr(),
            remote_ref_name,
            "it's possible to checkout anything with that name, but here we have an ordinary branch"
        );

        assert_eq!(
            checked_out_ref
                .remote_ref_name(gix::remote::Direction::Fetch)
                .transpose()?
                .unwrap()
                .as_bstr(),
            remote_ref_name,
            "the merge configuration is using the given name"
        );

        let index = repo.index()?;
        assert_eq!(index.entries().len(), 1, "All entries are known as per HEAD tree");

        assure_index_entries_on_disk(&index, repo.workdir().expect("non-bare"));
        Ok(())
    }

    #[test]
    fn fetch_and_checkout_specific_non_existing() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let remote_repo = remote::repo("base");
        let ref_to_checkout = "does-not-exist";
        let mut prepare = gix::clone::PrepareFetch::new(
            remote_repo.path(),
            tmp.path(),
            gix::create::Kind::WithWorktree,
            Default::default(),
            restricted(),
        )?
        .with_ref_name(Some(ref_to_checkout))?;

        let err = prepare
            .fetch_then_checkout(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "The remote didn't have any ref that matched 'does-not-exist'",
            "we don't test this, but it's important that it determines this before receiving a pack"
        );
        Ok(())
    }

    #[test]
    fn fetch_succeeds_despite_remote_head_ref() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let remote_repo = remote::repo("head-ref");
        let mut prepare = gix::clone::PrepareFetch::new(
            remote_repo.path(),
            tmp.path(),
            gix::create::Kind::WithWorktree,
            Default::default(),
            restricted(),
        )?;

        let (mut checkout, _out) = prepare.fetch_then_checkout(gix::progress::Discard, &AtomicBool::default())?;
        let (repo, _) = checkout.main_worktree(gix::progress::Discard, &AtomicBool::default())?;
        assert!(repo.head().is_ok(), "we could handle the HEAD normaller");
        Ok(())
    }

    #[test]
    fn fetch_and_checkout_specific_annotated_tag() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let remote_repo = remote::repo("base");
        let ref_to_checkout = "annotated-detached-tag";
        let mut prepare = gix::clone::PrepareFetch::new(
            remote_repo.path(),
            tmp.path(),
            gix::create::Kind::WithWorktree,
            Default::default(),
            restricted(),
        )?
        .with_ref_name(Some(ref_to_checkout))?;
        let (mut checkout, _out) =
            prepare.fetch_then_checkout(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;

        let (repo, _) = checkout.main_worktree(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;

        assert_eq!(
            repo.references()?.all()?.count() - 1,
            remote_repo.references()?.all()?.count(),
            "all references have been cloned, + remote HEAD (not listed in remote_repo)"
        );
        let checked_out_ref = repo.head_ref()?.expect("head points to ref");
        let remote_ref_name = format!("refs/tags/{ref_to_checkout}");
        assert_eq!(
            checked_out_ref.name().as_bstr(),
            remote_ref_name,
            "it also works with tags"
        );

        assert_eq!(
            checked_out_ref
                .remote_ref_name(gix::remote::Direction::Fetch)
                .transpose()?,
            None,
            "there is no merge configuration for tags"
        );
        Ok(())
    }

    fn assure_index_entries_on_disk(index: &gix::worktree::Index, work_dir: &Path) {
        for entry in index.entries() {
            let entry_path = work_dir.join(gix_path::from_bstr(entry.path(index)));
            assert!(entry_path.is_file(), "{entry_path:?} not found on disk");
        }
    }

    #[test]
    fn fetch_and_checkout_empty_remote_repo() -> crate::Result {
        for version in [
            gix::protocol::transport::Protocol::V0,
            gix::protocol::transport::Protocol::V2,
        ] {
            let tmp = gix_testtools::tempfile::TempDir::new()?;
            let mut prepare = gix::clone::PrepareFetch::new(
                gix_testtools::scripted_fixture_read_only("make_empty_repo.sh")?,
                tmp.path(),
                gix::create::Kind::WithWorktree,
                Default::default(),
                restricted().config_overrides(Some(format!("protocol.version={}", version as u8))),
            )?;
            let (mut checkout, out) =
                prepare.fetch_then_checkout(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;
            let (repo, _) =
                checkout.main_worktree(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;

            assert!(!repo.index_path().is_file(), "newly initialized repos have no index");
            let head = repo.head()?;
            assert!(head.is_unborn());
            assert_eq!(repo.head_tree_id_or_empty()?, repo.empty_tree().id());

            assert!(
                head.log_iter().all()?.is_none(),
                "no reflog for unborn heads (as it needs non-null destination hash)"
            );

            let supports_unborn = out
                .handshake
                .capabilities
                .capability("ls-refs")
                .is_some_and(|cap| cap.supports("unborn").unwrap_or(false));
            if supports_unborn {
                assert_eq!(
                    head.referent_name().expect("present").as_bstr(),
                    "refs/heads/special",
                    "we pick up the name as present on the server, not the one we default to"
                );
            } else {
                assert_eq!(
                    head.referent_name().expect("present").as_bstr(),
                    "refs/heads/main",
                    "we simply keep our own post-init HEAD which defaults to the branch name we configured locally"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn fetch_only_without_configuration() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let (repo, out) = gix::clone::PrepareFetch::new(
            remote::repo("base").path(),
            tmp.path(),
            gix::create::Kind::Bare,
            Default::default(),
            restricted(),
        )?
        .fetch_only(gix::progress::Discard, &std::sync::atomic::AtomicBool::default())?;
        assert!(repo.find_remote("origin").is_ok(), "default remote name is 'origin'");
        match out.status {
            gix::remote::fetch::Status::Change { write_pack_bundle, .. } => {
                assert!(
                    write_pack_bundle.keep_path.is_none(),
                    "keep files aren't kept if refs are written"
                );
            }
            _ => unreachable!("a clone always carries a change"),
        }
        Ok(())
    }
}

#[test]
fn clone_and_early_persist_without_receive() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let repo = gix::clone::PrepareFetch::new(
        remote::repo("base").path(),
        tmp.path(),
        gix::create::Kind::Bare,
        Default::default(),
        restricted(),
    )?
    .persist();
    assert!(repo.is_bare(), "repo is now ours and remains");
    Ok(())
}

#[test]
fn clone_and_destination_must_be_empty() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    std::fs::write(tmp.path().join("file"), b"hello")?;
    match gix::clone::PrepareFetch::new(
        remote::repo("base").path(),
        tmp.path(),
        gix::create::Kind::Bare,
        Default::default(),
        restricted(),
    ) {
        Ok(_) => unreachable!("this should fail as the directory isn't empty"),
        Err(err) => assert!(err
            .to_string()
            .starts_with("Refusing to initialize the non-empty directory as ")),
    }
    Ok(())
}

#[test]
fn clone_bare_into_empty_directory_and_early_drop() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    // this breaks isolation, but shouldn't be affecting the test. If so, use isolation options for opening the repo.
    let prep = gix::clone::PrepareFetch::new(
        remote::repo("base").path(),
        tmp.path(),
        gix::create::Kind::Bare,
        Default::default(),
        restricted(),
    )?;
    let head = tmp.path().join("HEAD");
    assert!(head.is_file(), "now a bare basic repo is present");
    drop(prep);

    assert!(!head.is_file(), "we cleanup if the clone isn't followed through");
    Ok(())
}

#[test]
fn clone_into_empty_directory_and_early_drop() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let prep = gix::clone::PrepareFetch::new(
        remote::repo("base").path(),
        tmp.path(),
        gix::create::Kind::WithWorktree,
        Default::default(),
        restricted(),
    )?;
    let head = tmp.path().join(".git").join("HEAD");
    assert!(head.is_file(), "now a basic repo is present");
    drop(prep);

    assert!(!head.is_file(), "we cleanup if the clone isn't followed through");
    Ok(())
}
