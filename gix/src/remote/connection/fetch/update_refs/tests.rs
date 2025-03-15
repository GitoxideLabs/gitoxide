pub fn restricted() -> crate::open::Options {
    crate::open::Options::isolated().config_overrides(["user.name=gitoxide", "user.email=gitoxide@localhost"])
}

/// Convert a hexadecimal hash into its corresponding `ObjectId` or _panic_.
fn hex_to_id(hex: &str) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

mod update {
    use gix_testtools::Result;

    use super::hex_to_id;
    use crate as gix;

    fn base_repo_path() -> String {
        gix::path::realpath(
            gix_testtools::scripted_fixture_read_only("make_remote_repos.sh")
                .unwrap()
                .join("base"),
        )
        .unwrap()
        .to_string_lossy()
        .into_owned()
    }

    fn repo(name: &str) -> gix::Repository {
        let dir = gix_testtools::scripted_fixture_read_only_with_args_single_archive(
            "make_fetch_repos.sh",
            [base_repo_path()],
        )
        .unwrap();
        gix::open_opts(dir.join(name), restricted()).unwrap()
    }
    fn named_repo(name: &str) -> gix::Repository {
        let dir = gix_testtools::scripted_fixture_read_only("make_remote_repos.sh").unwrap();
        gix::open_opts(dir.join(name), restricted()).unwrap()
    }
    fn repo_rw(name: &str) -> (gix::Repository, gix_testtools::tempfile::TempDir) {
        let dir = gix_testtools::scripted_fixture_writable_with_args_single_archive(
            "make_fetch_repos.sh",
            [base_repo_path()],
            gix_testtools::Creation::ExecuteScript,
        )
        .unwrap();
        let repo = gix::open_opts(dir.path().join(name), restricted()).unwrap();
        (repo, dir)
    }
    use gix_ref::{
        transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
        Target, TargetRef,
    };

    use crate::{
        bstr::BString,
        remote::{
            fetch,
            fetch::{
                refmap::Mapping,
                refmap::Source,
                refmap::SpecIndex,
                refs::{tests::restricted, update::TypeChange},
                RefLogMessage,
            },
        },
    };

    #[test]
    fn various_valid_updates() {
        let repo = repo("two-origins");
        for (spec, expected_mode, reflog_message, detail) in [
            (
                "refs/heads/main:refs/remotes/origin/main",
                fetch::refs::update::Mode::NoChangeNeeded,
                Some("no update will be performed"),
                "these refs are en-par since the initial clone",
            ),
            (
                "refs/heads/main",
                fetch::refs::update::Mode::NoChangeNeeded,
                None,
                "without local destination ref there is nothing to do for us, ever (except for FETCH_HEADs) later",
            ),
            (
                "refs/heads/main:refs/remotes/origin/new-main",
                fetch::refs::update::Mode::New,
                Some("storing ref"),
                "the destination branch doesn't exist and needs to be created",
            ),
            (
                "refs/heads/main:refs/heads/feature",
                fetch::refs::update::Mode::New,
                Some("storing head"),
                "reflog messages are specific to the type of branch stored, to some limited extend",
            ),
            (
                "refs/heads/main:refs/tags/new-tag",
                fetch::refs::update::Mode::New,
                Some("storing tag"),
                "reflog messages are specific to the type of branch stored, to some limited extend",
            ),
            (
                "+refs/heads/main:refs/remotes/origin/new-main",
                fetch::refs::update::Mode::New,
                Some("storing ref"),
                "just to validate that we really are in dry-run mode, or else this ref would be present now",
            ),
            (
                "+refs/heads/main:refs/remotes/origin/g",
                fetch::refs::update::Mode::FastForward,
                Some("fast-forward (guessed in dry-run)"),
                "a forced non-fastforward (main goes backwards), but dry-run calls it fast-forward",
            ),
            (
                "+refs/heads/main:refs/tags/b-tag",
                fetch::refs::update::Mode::Forced,
                Some("updating tag"),
                "tags can only be forced",
            ),
            (
                "refs/heads/main:refs/tags/b-tag",
                fetch::refs::update::Mode::RejectedTagUpdate,
                None,
                "otherwise a tag is always refusing itself to be overwritten (no-clobber)",
            ),
            (
                "+refs/remotes/origin/g:refs/heads/main",
                fetch::refs::update::Mode::RejectedCurrentlyCheckedOut {
                    worktree_dirs: vec![repo.workdir().expect("present").to_owned()],
                },
                None,
                "checked out branches cannot be written, as it requires a merge of sorts which isn't done here",
            ),
            (
                "ffffffffffffffffffffffffffffffffffffffff:refs/heads/invalid-source-object",
                fetch::refs::update::Mode::RejectedSourceObjectNotFound {
                    id: hex_to_id("ffffffffffffffffffffffffffffffffffffffff"),
                },
                None,
                "checked out branches cannot be written, as it requires a merge of sorts which isn't done here",
            ),
            (
                "refs/remotes/origin/g:refs/heads/not-currently-checked-out",
                fetch::refs::update::Mode::FastForward,
                Some("fast-forward (guessed in dry-run)"),
                "a fast-forward only fast-forward situation, all good",
            ),
        ] {
            let (mapping, specs) = mapping_from_spec(spec, &repo);
            let out = fetch::refs::update(
                &repo,
                prefixed("action"),
                &mapping,
                &specs,
                &[],
                fetch::Tags::None,
                reflog_message.map_or(fetch::DryRun::No, |_| fetch::DryRun::Yes),
                fetch::WritePackedRefs::Never,
            )
            .unwrap();

            assert_eq!(
                out.updates,
                vec![fetch::refs::Update {
                    type_change: None,
                    mode: expected_mode.clone(),
                    edit_index: reflog_message.map(|_| 0),
                }],
                "{spec:?}: {detail}"
            );
            assert_eq!(out.edits.len(), reflog_message.map_or(0, |_| 1));
            if let Some(reflog_message) = reflog_message {
                let edit = &out.edits[0];
                match &edit.change {
                    Change::Update { log, new, .. } => {
                        assert_eq!(
                            log.message,
                            format!("action: {reflog_message}"),
                            "{spec}: reflog messages are specific and we emulate git word for word"
                        );
                        let remote_ref = repo
                            .find_reference(specs[0].to_ref().source().expect("always present"))
                            .unwrap();
                        assert_eq!(
                            new.id(),
                            remote_ref.target().id(),
                            "remote ref provides the id to set in the local reference"
                        );
                    }
                    _ => unreachable!("only updates"),
                }
            }
        }
    }

    #[test]
    fn checked_out_branches_in_worktrees_are_rejected_with_additional_information() -> Result {
        let root = gix_path::realpath(gix_testtools::scripted_fixture_read_only_with_args_single_archive(
            "make_fetch_repos.sh",
            [base_repo_path()],
        )?)?;
        let repo = root.join("worktree-root");
        let repo = gix::open_opts(repo, restricted())?;
        for (branch, path_from_root) in [
            ("main", "worktree-root"),
            ("wt-a-nested", "prev/wt-a-nested"),
            ("wt-a", "wt-a"),
            ("nested-wt-b", "wt-a/nested-wt-b"),
            ("wt-c-locked", "wt-c-locked"),
            ("wt-deleted", "wt-deleted"),
        ] {
            let spec = format!("refs/heads/main:refs/heads/{branch}");
            let (mappings, specs) = mapping_from_spec(&spec, &repo);
            let out = fetch::refs::update(
                &repo,
                prefixed("action"),
                &mappings,
                &specs,
                &[],
                fetch::Tags::None,
                fetch::DryRun::Yes,
                fetch::WritePackedRefs::Never,
            )?;

            assert_eq!(
                out.updates,
                vec![fetch::refs::Update {
                    mode: fetch::refs::update::Mode::RejectedCurrentlyCheckedOut {
                        worktree_dirs: vec![root.join(path_from_root)],
                    },
                    type_change: None,
                    edit_index: None,
                }],
                "{spec}: checked-out checks are done before checking if a change would actually be required (here it isn't)"
            );
            assert_eq!(out.edits.len(), 0);
        }
        Ok(())
    }

    #[test]
    fn unborn_remote_branches_can_be_created_locally_if_they_are_new() -> Result {
        let repo = named_repo("unborn");
        let (mappings, specs) = mapping_from_spec("HEAD:refs/remotes/origin/HEAD", &repo);
        assert_eq!(mappings.len(), 1);
        let out = fetch::refs::update(
            &repo,
            prefixed("action"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::Yes,
            fetch::WritePackedRefs::Never,
        )?;
        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::New,
                type_change: None,
                edit_index: Some(0)
            }]
        );
        assert_eq!(out.edits.len(), 1, "we are OK with creating unborn refs");
        Ok(())
    }

    #[test]
    fn unborn_remote_branches_can_update_local_unborn_branches() -> Result {
        let repo = named_repo("unborn");
        let (mappings, specs) = mapping_from_spec("HEAD:refs/heads/existing-unborn-symbolic", &repo);
        assert_eq!(mappings.len(), 1);
        let out = fetch::refs::update(
            &repo,
            prefixed("action"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::Yes,
            fetch::WritePackedRefs::Never,
        )?;
        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::NoChangeNeeded,
                type_change: None,
                edit_index: Some(0)
            }]
        );
        assert_eq!(out.edits.len(), 1, "we are OK with updating unborn refs");
        assert_eq!(
            out.edits[0],
            RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: false,
                        message: "action: change unborn ref".into(),
                    },
                    expected: PreviousValue::MustExistAndMatch(Target::Symbolic(
                        "refs/heads/main".try_into().expect("valid"),
                    )),
                    new: Target::Symbolic("refs/heads/main".try_into().expect("valid")),
                },
                name: "refs/heads/existing-unborn-symbolic".try_into().expect("valid"),
                deref: false,
            }
        );

        let (mappings, specs) = mapping_from_spec("HEAD:refs/heads/existing-unborn-symbolic-other", &repo);
        assert_eq!(mappings.len(), 1);
        let out = fetch::refs::update(
            &repo,
            prefixed("action"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::Yes,
            fetch::WritePackedRefs::Never,
        )?;
        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::Forced,
                type_change: None,
                edit_index: Some(0)
            }]
        );
        assert_eq!(
            out.edits.len(),
            1,
            "we are OK with creating unborn refs even without actually forcing it"
        );
        assert_eq!(
            out.edits[0],
            RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: false,
                        message: "action: change unborn ref".into(),
                    },
                    expected: PreviousValue::MustExistAndMatch(Target::Symbolic(
                        "refs/heads/other".try_into().expect("valid"),
                    )),
                    new: Target::Symbolic("refs/heads/main".try_into().expect("valid")),
                },
                name: "refs/heads/existing-unborn-symbolic-other".try_into().expect("valid"),
                deref: false,
            }
        );
        Ok(())
    }

    #[test]
    fn remote_symbolic_refs_with_locally_unavailable_target_result_in_valid_peeled_branches() -> Result {
        let remote_repo = named_repo("one-commit-with-symref");
        let local_repo = named_repo("unborn");
        let (mappings, specs) = mapping_from_spec("refs/heads/symbolic:refs/heads/new", &remote_repo);
        assert_eq!(mappings.len(), 1);

        let out = fetch::refs::update(
            &local_repo,
            prefixed("action"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::Yes,
            fetch::WritePackedRefs::Never,
        )?;
        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::New,
                type_change: None,
                edit_index: Some(0)
            }]
        );
        assert_eq!(out.edits.len(), 1);
        let target = Target::Object(hex_to_id("66f16e4e8baf5c77bb6d0484495bebea80e916ce"));
        assert_eq!(
            out.edits[0],
            RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: false,
                        message: "action: storing head".into(),
                    },
                    expected: PreviousValue::ExistingMustMatch(target.clone()),
                    new: target,
                },
                name: "refs/heads/new".try_into().expect("valid"),
                deref: false,
            },
            "we create local-refs whose targets aren't present yet, even though the remote knows them.\
             This leaves the caller with assuring all refs are mentioned in mappings."
        );
        Ok(())
    }

    #[test]
    fn remote_symbolic_refs_with_locally_unavailable_target_dont_overwrite_valid_local_branches() -> Result {
        let remote_repo = named_repo("one-commit-with-symref");
        let local_repo = named_repo("one-commit-with-symref-missing-branch");
        let (mappings, specs) = mapping_from_spec("refs/heads/unborn:refs/heads/valid-locally", &remote_repo);
        assert_eq!(mappings.len(), 1);

        let out = fetch::refs::update(
            &local_repo,
            prefixed("action"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::Yes,
            fetch::WritePackedRefs::Never,
        )?;
        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::RejectedToReplaceWithUnborn,
                type_change: None,
                edit_index: None
            }]
        );
        assert_eq!(out.edits.len(), 0);
        Ok(())
    }

    #[test]
    fn unborn_remote_refs_dont_overwrite_valid_local_refs() -> Result {
        let remote_repo = named_repo("unborn");
        let local_repo = named_repo("one-commit-with-symref");
        let (mappings, specs) =
            mapping_from_spec("refs/heads/existing-unborn-symbolic:refs/heads/branch", &remote_repo);
        assert_eq!(mappings.len(), 1);

        let out = fetch::refs::update(
            &local_repo,
            prefixed("action"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::Yes,
            fetch::WritePackedRefs::Never,
        )?;
        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::RejectedToReplaceWithUnborn,
                type_change: None,
                edit_index: None
            }],
            "we don't overwrite locally present refs with unborn ones for safety"
        );
        assert_eq!(out.edits.len(), 0);
        Ok(())
    }

    #[test]
    fn local_symbolic_refs_can_be_overwritten() {
        let repo = repo("two-origins");
        for (source, destination, expected_update, expected_edit) in [
            (
                // attempt to overwrite HEAD isn't possible as the matching engine will normalize the path. That way, `HEAD`
                // can never be set. This is by design (of git) and we follow it.
                "refs/heads/symbolic",
                "HEAD",
                fetch::refs::Update {
                    mode: fetch::refs::update::Mode::New,
                    type_change: None,
                    edit_index: Some(0),
                },
                Some(RefEdit {
                    change: Change::Update {
                        log: LogChange {
                            mode: RefLog::AndReference,
                            force_create_reflog: false,
                            message: "action: storing head".into(),
                        },
                        expected: PreviousValue::ExistingMustMatch(Target::Symbolic(
                            "refs/heads/main".try_into().expect("valid"),
                        )),
                        new: Target::Symbolic("refs/heads/main".try_into().expect("valid")),
                    },
                    name: "refs/heads/HEAD".try_into().expect("valid"),
                    deref: false,
                }),
            ),
            (
                // attempt to overwrite checked out branch fails
                "refs/remotes/origin/b", // strange, but the remote-refs are simulated and based on local refs
                "refs/heads/main",
                fetch::refs::Update {
                    mode: fetch::refs::update::Mode::RejectedCurrentlyCheckedOut {
                        worktree_dirs: vec![repo.workdir().expect("present").to_owned()],
                    },
                    type_change: None,
                    edit_index: None,
                },
                None,
            ),
            (
                // symbolic becomes direct
                "refs/heads/main",
                "refs/heads/symbolic",
                fetch::refs::Update {
                    mode: fetch::refs::update::Mode::NoChangeNeeded,
                    type_change: Some(TypeChange::SymbolicToDirect),
                    edit_index: Some(0),
                },
                Some(RefEdit {
                    change: Change::Update {
                        log: LogChange {
                            mode: RefLog::AndReference,
                            force_create_reflog: false,
                            message: "action: no update will be performed".into(),
                        },
                        expected: PreviousValue::MustExistAndMatch(Target::Symbolic(
                            "refs/heads/main".try_into().expect("valid"),
                        )),
                        new: Target::Object(hex_to_id("f99771fe6a1b535783af3163eba95a927aae21d5")),
                    },
                    name: "refs/heads/symbolic".try_into().expect("valid"),
                    deref: false,
                }),
            ),
            (
                // direct becomes symbolic
                "refs/heads/symbolic",
                "refs/remotes/origin/a",
                fetch::refs::Update {
                    mode: fetch::refs::update::Mode::NoChangeNeeded,
                    type_change: Some(TypeChange::DirectToSymbolic),
                    edit_index: Some(0),
                },
                Some(RefEdit {
                    change: Change::Update {
                        log: LogChange {
                            mode: RefLog::AndReference,
                            force_create_reflog: false,
                            message: "action: no update will be performed".into(),
                        },
                        expected: PreviousValue::MustExistAndMatch(Target::Object(hex_to_id(
                            "f99771fe6a1b535783af3163eba95a927aae21d5",
                        ))),
                        new: Target::Symbolic("refs/heads/main".try_into().expect("valid")),
                    },
                    name: "refs/remotes/origin/a".try_into().expect("valid"),
                    deref: false,
                }),
            ),
            (
                // symbolic to symbolic (same)
                "refs/heads/symbolic",
                "refs/heads/symbolic",
                fetch::refs::Update {
                    mode: fetch::refs::update::Mode::NoChangeNeeded,
                    type_change: None,
                    edit_index: Some(0),
                },
                Some(RefEdit {
                    change: Change::Update {
                        log: LogChange {
                            mode: RefLog::AndReference,
                            force_create_reflog: false,
                            message: "action: no update will be performed".into(),
                        },
                        expected: PreviousValue::MustExistAndMatch(Target::Symbolic(
                            "refs/heads/main".try_into().expect("valid"),
                        )),
                        new: Target::Symbolic("refs/heads/main".try_into().expect("valid")),
                    },
                    name: "refs/heads/symbolic".try_into().expect("valid"),
                    deref: false,
                }),
            ),
        ] {
            let (mappings, specs) = mapping_from_spec(&format!("{source}:{destination}"), &repo);
            assert_eq!(mappings.len(), 1);
            let out = fetch::refs::update(
                &repo,
                prefixed("action"),
                &mappings,
                &specs,
                &[],
                fetch::Tags::None,
                fetch::DryRun::Yes,
                fetch::WritePackedRefs::Never,
            )
            .unwrap();

            assert_eq!(out.edits.len(), usize::from(expected_edit.is_some()));
            assert_eq!(out.updates, vec![expected_update]);
            if let Some(expected) = expected_edit {
                assert_eq!(out.edits, vec![expected]);
            }
        }
    }

    #[test]
    fn remote_symbolic_refs_can_always_be_set_as_there_is_no_scenario_where_it_could_be_nonexisting_and_rejected() {
        let repo = repo("two-origins");
        let (mut mappings, specs) = mapping_from_spec("refs/heads/symbolic:refs/remotes/origin/new", &repo);
        mappings.push(Mapping {
            remote: Source::Ref(gix_protocol::handshake::Ref::Direct {
                full_ref_name: "refs/heads/main".into(),
                object: hex_to_id("f99771fe6a1b535783af3163eba95a927aae21d5"),
            }),
            local: Some("refs/heads/symbolic".into()),
            spec_index: SpecIndex::ExplicitInRemote(0),
        });
        let out = fetch::refs::update(
            &repo,
            prefixed("action"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::Yes,
            fetch::WritePackedRefs::Never,
        )
        .unwrap();

        assert_eq!(out.edits.len(), 2, "symbolic refs are handled just like any other ref");
        assert_eq!(
            out.updates,
            vec![
                fetch::refs::Update {
                    mode: fetch::refs::update::Mode::New,
                    type_change: None,
                    edit_index: Some(0)
                },
                fetch::refs::Update {
                    mode: fetch::refs::update::Mode::NoChangeNeeded,
                    type_change: Some(TypeChange::SymbolicToDirect),
                    edit_index: Some(1)
                }
            ],
        );
        let edit = &out.edits[0];
        match &edit.change {
            Change::Update { log, new, .. } => {
                assert_eq!(log.message, "action: storing ref");
                assert!(
                    new.try_name().is_some(),
                    "remote falls back to peeled id as it's the only thing we seem to have locally, it won't refer to a non-existing local ref"
                );
            }
            _ => unreachable!("only updates"),
        }
    }

    #[test]
    fn local_direct_refs_are_written_with_symbolic_ones() {
        let repo = repo("two-origins");
        let (mappings, specs) = mapping_from_spec("refs/heads/symbolic:refs/heads/not-currently-checked-out", &repo);
        let out = fetch::refs::update(
            &repo,
            prefixed("action"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::Yes,
            fetch::WritePackedRefs::Never,
        )
        .unwrap();

        assert_eq!(out.edits.len(), 1);
        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::NoChangeNeeded,
                type_change: Some(fetch::refs::update::TypeChange::DirectToSymbolic),
                edit_index: Some(0)
            }],
        );
    }

    #[test]
    fn remote_refs_cannot_map_to_local_head() {
        let repo = repo("two-origins");
        let (mappings, specs) = mapping_from_spec("refs/heads/main:HEAD", &repo);
        let out = fetch::refs::update(
            &repo,
            prefixed("action"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::Yes,
            fetch::WritePackedRefs::Never,
        )
        .unwrap();

        assert_eq!(out.edits.len(), 1);
        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::New,
                type_change: None,
                edit_index: Some(0),
            }],
        );
        let edit = &out.edits[0];
        match &edit.change {
            Change::Update { log, new, .. } => {
                assert_eq!(log.message, "action: storing head");
                assert!(
                    new.try_id().is_some(),
                    "remote is peeled, so local will be peeled as well"
                );
            }
            _ => unreachable!("only updates"),
        }
        assert_eq!(
            edit.name.as_bstr(),
            "refs/heads/HEAD",
            "it's not possible to refer to the local HEAD with refspecs"
        );
    }

    #[test]
    fn remote_symbolic_refs_can_be_written_locally_and_point_to_tracking_branch() {
        let repo = repo("two-origins");
        let (mut mappings, specs) = mapping_from_spec("HEAD:refs/remotes/origin/new-HEAD", &repo);
        mappings.push(Mapping {
            remote: Source::Ref(gix_protocol::handshake::Ref::Direct {
                full_ref_name: "refs/heads/main".into(),
                object: hex_to_id("f99771fe6a1b535783af3163eba95a927aae21d5"),
            }),
            local: Some("refs/remotes/origin/main".into()),
            spec_index: SpecIndex::ExplicitInRemote(0),
        });
        let out = fetch::refs::update(
            &repo,
            prefixed("action"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::Yes,
            fetch::WritePackedRefs::Never,
        )
        .unwrap();

        assert_eq!(
            out.updates,
            vec![
                fetch::refs::Update {
                    mode: fetch::refs::update::Mode::New,
                    type_change: None,
                    edit_index: Some(0),
                },
                fetch::refs::Update {
                    mode: fetch::refs::update::Mode::NoChangeNeeded,
                    type_change: None,
                    edit_index: Some(1),
                }
            ],
        );
        assert_eq!(out.edits.len(), 2);
        let edit = &out.edits[0];
        match &edit.change {
            Change::Update { log, new, .. } => {
                assert_eq!(log.message, "action: storing ref");
                assert_eq!(
                    new.try_name().expect("symbolic ref").as_bstr(),
                    "refs/remotes/origin/main",
                    "remote is symbolic, so local will be symbolic as well, but is rewritten to tracking branch"
                );
            }
            _ => unreachable!("only updates"),
        }
        assert_eq!(edit.name.as_bstr(), "refs/remotes/origin/new-HEAD",);
    }

    #[test]
    fn non_fast_forward_is_rejected_but_appears_to_be_fast_forward_in_dryrun_mode() {
        let repo = repo("two-origins");
        let (mappings, specs) = mapping_from_spec("refs/heads/main:refs/remotes/origin/g", &repo);
        let reflog_message: BString = "very special".into();
        let out = fetch::refs::update(
            &repo,
            RefLogMessage::Override {
                message: reflog_message.clone(),
            },
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::Yes,
            fetch::WritePackedRefs::Never,
        )
        .unwrap();

        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::FastForward,
                type_change: None,
                edit_index: Some(0),
            }],
            "The caller has to be aware and note that dry-runs can't know about fast-forwards as they don't have remote objects"
        );
        assert_eq!(out.edits.len(), 1);
        let edit = &out.edits[0];
        match &edit.change {
            Change::Update { log, .. } => {
                assert_eq!(log.message, reflog_message);
            }
            _ => unreachable!("only updates"),
        }
    }

    #[test]
    fn non_fast_forward_is_rejected_if_dry_run_is_disabled() {
        let (repo, _tmp) = repo_rw("two-origins");
        let (mappings, specs) = mapping_from_spec("refs/remotes/origin/g:refs/heads/not-currently-checked-out", &repo);
        let out = fetch::refs::update(
            &repo,
            prefixed("action"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::No,
            fetch::WritePackedRefs::Never,
        )
        .unwrap();

        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::RejectedNonFastForward,
                type_change: None,
                edit_index: None,
            }]
        );
        assert_eq!(out.edits.len(), 0);

        let (mappings, specs) = mapping_from_spec("refs/heads/main:refs/remotes/origin/g", &repo);
        let out = fetch::refs::update(
            &repo,
            prefixed("prefix"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::No,
            fetch::WritePackedRefs::Never,
        )
        .unwrap();

        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::FastForward,
                type_change: None,
                edit_index: Some(0),
            }]
        );
        assert_eq!(out.edits.len(), 1);
        let edit = &out.edits[0];
        match &edit.change {
            Change::Update { log, .. } => {
                assert_eq!(log.message, format!("prefix: {}", "fast-forward"));
            }
            _ => unreachable!("only updates"),
        }
    }

    #[test]
    fn fast_forwards_are_called_out_even_if_force_is_given() {
        let (repo, _tmp) = repo_rw("two-origins");
        let (mappings, specs) = mapping_from_spec("+refs/heads/main:refs/remotes/origin/g", &repo);
        let out = fetch::refs::update(
            &repo,
            prefixed("prefix"),
            &mappings,
            &specs,
            &[],
            fetch::Tags::None,
            fetch::DryRun::No,
            fetch::WritePackedRefs::Never,
        )
        .unwrap();

        assert_eq!(
            out.updates,
            vec![fetch::refs::Update {
                mode: fetch::refs::update::Mode::FastForward,
                type_change: None,
                edit_index: Some(0),
            }]
        );
        assert_eq!(out.edits.len(), 1);
        let edit = &out.edits[0];
        match &edit.change {
            Change::Update { log, .. } => {
                assert_eq!(log.message, format!("prefix: {}", "fast-forward"));
            }
            _ => unreachable!("only updates"),
        }
    }

    fn mapping_from_spec(
        spec: &str,
        remote_repo: &gix::Repository,
    ) -> (Vec<fetch::refmap::Mapping>, Vec<gix::refspec::RefSpec>) {
        let spec = gix_refspec::parse(spec.into(), gix_refspec::parse::Operation::Fetch).unwrap();
        let group = gix_refspec::MatchGroup::from_fetch_specs(Some(spec));
        let references = remote_repo.references().unwrap();
        let mut references: Vec<_> = references.all().unwrap().map(|r| into_remote_ref(r.unwrap())).collect();
        references.push(into_remote_ref(remote_repo.find_reference("HEAD").unwrap()));
        let mappings = group
            .match_lhs(references.iter().map(remote_ref_to_item))
            .mappings
            .into_iter()
            .map(|m| fetch::refmap::Mapping {
                remote: m.item_index.map_or_else(
                    || match m.lhs {
                        gix_refspec::match_group::SourceRef::ObjectId(id) => fetch::refmap::Source::ObjectId(id),
                        _ => unreachable!("not a ref, must be id: {:?}", m),
                    },
                    |idx| fetch::refmap::Source::Ref(references[idx].clone()),
                ),
                local: m.rhs.map(std::borrow::Cow::into_owned),
                spec_index: SpecIndex::ExplicitInRemote(m.spec_index),
            })
            .collect();
        (mappings, vec![spec.to_owned()])
    }

    fn into_remote_ref(mut r: gix::Reference<'_>) -> gix_protocol::handshake::Ref {
        let full_ref_name = r.name().as_bstr().into();
        match r.target() {
            TargetRef::Object(id) => gix_protocol::handshake::Ref::Direct {
                full_ref_name,
                object: id.into(),
            },
            TargetRef::Symbolic(name) => {
                let target = name.as_bstr().into();
                match r.peel_to_id_in_place() {
                    Ok(id) => gix_protocol::handshake::Ref::Symbolic {
                        full_ref_name,
                        target,
                        tag: None,
                        object: id.detach(),
                    },
                    Err(_) => gix_protocol::handshake::Ref::Unborn { full_ref_name, target },
                }
            }
        }
    }

    fn remote_ref_to_item(r: &gix_protocol::handshake::Ref) -> gix_refspec::match_group::Item<'_> {
        let (full_ref_name, target, object) = r.unpack();
        static NULL: gix_hash::ObjectId = gix_hash::Kind::Sha1.null();
        gix_refspec::match_group::Item {
            full_ref_name,
            target: target.unwrap_or(NULL.as_ref()),
            object,
        }
    }

    fn prefixed(action: &str) -> RefLogMessage {
        RefLogMessage::Prefixed { action: action.into() }
    }
}
