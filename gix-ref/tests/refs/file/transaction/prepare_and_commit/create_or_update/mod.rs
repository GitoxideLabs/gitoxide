use gix_hash::ObjectId;
use gix_lock::acquire::Fail;
use gix_object::bstr::{BString, ByteSlice};
use gix_ref::{
    file::{
        transaction::{self, PackedRefs},
        ReferenceExt,
    },
    store::WriteReflog,
    transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
    Target,
};
use std::error::Error;

use crate::{
    file::{
        store_with_packed_refs, store_writable,
        transaction::prepare_and_commit::{
            committer, create_at, create_symbolic_at, delete_at, empty_store, log_line, reflog_lines,
        },
        EmptyCommit,
    },
    hex_to_id,
};

mod collisions;

#[test]
fn intermediate_directories_are_removed_on_rollback() -> crate::Result {
    for explicit_rollback in [false, true] {
        let (dir, store) = empty_store()?;

        let transaction = store.transaction().prepare(
            [create_at("refs/heads/a/b/ref"), create_at("refs/heads/a/c/ref")],
            Fail::Immediately,
            Fail::Immediately,
        )?;

        assert!(
            dir.path().join("refs/heads/a/b").exists(),
            "lock files have been created in their place to avoid concurrent modification"
        );
        assert!(dir.path().join("refs/heads/a/c").exists());

        if explicit_rollback {
            transaction.rollback();
        } else {
            drop(transaction);
        }

        assert!(!dir.path().join("refs/heads").exists());
        assert!(!dir.path().join("refs").exists(), "we go all in right now and also remove the refs directory. 'git' might not do that, but it's not a problem either");
    }
    Ok(())
}

#[test]
fn reference_with_equally_named_empty_or_non_empty_directory_already_in_place_can_potentially_recover() -> crate::Result
{
    for is_empty in &[true, false] {
        let (dir, store) = empty_store()?;
        let head_dir = dir.path().join("HEAD");
        std::fs::create_dir_all(head_dir.join("a").join("b").join("also-empty"))?;
        if !*is_empty {
            std::fs::write(head_dir.join("file.ext"), "".as_bytes())?;
        }

        let edits = store
            .transaction()
            .prepare(
                Some(RefEdit {
                    change: Change::Update {
                        log: LogChange::default(),
                        expected: PreviousValue::MustNotExist,
                        new: Target::Symbolic("refs/heads/main".try_into().unwrap()),
                    },
                    name: "HEAD".try_into()?,
                    deref: false,
                }),
                Fail::Immediately,
                Fail::Immediately,
            )?
            .commit(committer().to_ref());
        if *is_empty {
            let edits = edits?;
            assert!(
                store.try_find_loose(edits[0].name.as_ref())?.is_some(),
                "HEAD was created despite a directory being in the way"
            );
        } else {
            match edits {
                #[cfg_attr(target_os = "windows", allow(unused_variables))]
                Err(transaction::commit::Error::LockCommit { source, full_name }) => {
                    assert_eq!(full_name, "HEAD");
                    #[cfg(not(windows))]
                    assert_eq!(source.to_string(), "Directory not empty");
                }
                _ => unreachable!("other errors shouldn't happen here"),
            }
        }
    }
    Ok(())
}

#[test]
fn reference_with_old_value_must_exist_when_creating_it() -> crate::Result {
    let (_keep, store) = empty_store()?;

    let new_target = Target::Object(gix_hash::Kind::Sha1.null());
    let res = store.transaction().prepare(
        Some(RefEdit {
            change: Change::Update {
                log: LogChange::default(),
                new: new_target.clone(),
                expected: PreviousValue::MustExist,
            },
            name: "HEAD".try_into()?,
            deref: false,
        }),
        Fail::Immediately,
        Fail::Immediately,
    );

    match res {
        Err(transaction::prepare::Error::MustExist { full_name, expected }) => {
            assert_eq!(full_name, "HEAD");
            assert_eq!(expected, new_target);
        }
        _ => unreachable!("unexpected result"),
    }
    Ok(())
}

#[test]
fn reference_with_explicit_value_must_match_the_value_on_update() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.try_find_loose("HEAD")?.expect("head exists already");
    let target = head.target;

    let res = store.transaction().prepare(
        Some(RefEdit {
            change: Change::Update {
                log: LogChange::default(),
                new: Target::Object(gix_hash::Kind::Sha1.null()),
                expected: PreviousValue::MustExistAndMatch(Target::Object(hex_to_id(
                    "28ce6a8b26aa170e1de65536fe8abe1832bd3242",
                ))),
            },
            name: "HEAD".try_into()?,
            deref: false,
        }),
        Fail::Immediately,
        Fail::Immediately,
    );
    match res {
        Err(transaction::prepare::Error::ReferenceOutOfDate { full_name, actual, .. }) => {
            assert_eq!(full_name, "HEAD");
            assert_eq!(actual, target);
        }
        _ => unreachable!("unexpected result"),
    }
    Ok(())
}

#[test]
fn the_existing_must_match_constraint_allow_non_existing_references_to_be_created() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let expected = PreviousValue::ExistingMustMatch(Target::Object(ObjectId::empty_tree(gix_hash::Kind::Sha1)));
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Update {
                    log: LogChange::default(),
                    new: Target::Object(gix_hash::Kind::Sha1.null()),
                    expected: expected.clone(),
                },
                name: "refs/heads/new".try_into()?,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref())?;

    assert_eq!(
        edits,
        vec![RefEdit {
            change: Change::Update {
                log: LogChange::default(),
                new: Target::Object(gix_hash::Kind::Sha1.null()),
                expected,
            },
            name: "refs/heads/new".try_into()?,
            deref: false,
        }]
    );
    Ok(())
}

#[test]
fn the_existing_must_match_constraint_requires_existing_references_to_have_the_given_value_to_cause_failure_on_mismatch(
) -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.try_find_loose("HEAD")?.expect("head exists already");
    let target = head.target;

    let res = store.transaction().prepare(
        Some(RefEdit {
            change: Change::Update {
                log: LogChange::default(),
                new: Target::Object(gix_hash::Kind::Sha1.null()),
                expected: PreviousValue::ExistingMustMatch(Target::Object(hex_to_id(
                    "28ce6a8b26aa170e1de65536fe8abe1832bd3242",
                ))),
            },
            name: "HEAD".try_into()?,
            deref: false,
        }),
        Fail::Immediately,
        Fail::Immediately,
    );
    match res {
        Err(transaction::prepare::Error::ReferenceOutOfDate { full_name, actual, .. }) => {
            assert_eq!(full_name, "HEAD");
            assert_eq!(actual, target);
        }
        _ => unreachable!("unexpected result"),
    }
    Ok(())
}

#[test]
fn reference_with_must_not_exist_constraint_cannot_be_created_if_it_exists_already() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.try_find_loose("HEAD")?.expect("head exists already");
    let target = head.target;

    let res = store
        .transaction()
        .prepare(Some(create_at("HEAD")), Fail::Immediately, Fail::Immediately);
    match res {
        Err(transaction::prepare::Error::MustNotExist { full_name, actual, .. }) => {
            assert_eq!(full_name, "HEAD");
            assert_eq!(actual, target);
        }
        _ => unreachable!("unexpected result"),
    }
    Ok(())
}

#[test]
fn namespaced_updates_or_deletions_are_transparent_and_not_observable() -> crate::Result {
    let (_keep, mut store) = empty_store()?;
    store.namespace = gix_ref::namespace::expand("foo")?.into();
    let actual = vec![
        delete_at("refs/for/deletion"),
        create_symbolic_at("HEAD", "refs/heads/hello"),
    ];
    let edits = store
        .transaction()
        .prepare(actual.clone(), Fail::Immediately, Fail::Immediately)?
        .commit(committer().to_ref())?;

    assert_eq!(edits, actual);
    Ok(())
}

#[test]
fn reference_with_must_exist_constraint_must_exist_already_with_any_value() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.try_find_loose("HEAD")?.expect("head exists already");
    let target = head.target;
    let previous_reflog_count = reflog_lines(&store, "HEAD")?.len();

    let new_target = Target::Object(ObjectId::empty_tree(gix_hash::Kind::Sha1));
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Update {
                    log: LogChange::default(),
                    new: new_target.clone(),
                    expected: PreviousValue::MustExist,
                },
                name: "HEAD".try_into()?,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref())?;

    assert_eq!(
        edits,
        vec![RefEdit {
            change: Change::Update {
                log: LogChange::default(),
                new: new_target,
                expected: PreviousValue::MustExistAndMatch(target)
            },
            name: "HEAD".try_into()?,
            deref: false,
        }]
    );

    assert_eq!(
        reflog_lines(&store, "HEAD")?.len(),
        previous_reflog_count + 1,
        "a new reflog is added"
    );
    Ok(())
}

#[test]
fn reference_with_must_not_exist_constraint_may_exist_already_if_the_new_value_matches_the_existing_one(
) -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.try_find_loose("HEAD")?.expect("head exists already");
    let target = head.target;
    let previous_reflog_count = reflog_lines(&store, "HEAD")?.len();

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Update {
                    log: LogChange::default(),
                    new: target.clone(),
                    expected: PreviousValue::MustNotExist,
                },
                name: "HEAD".try_into()?,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref())?;

    assert_eq!(
        edits,
        vec![RefEdit {
            change: Change::Update {
                log: LogChange::default(),
                new: target.clone(),
                expected: PreviousValue::MustExistAndMatch(target)
            },
            name: "HEAD".try_into()?,
            deref: false,
        }]
    );

    assert_eq!(
        reflog_lines(&store, "HEAD")?.len(),
        previous_reflog_count,
        "no new reflog is actually added"
    );
    Ok(())
}

#[test]
fn cancellation_after_preparation_leaves_no_change() -> crate::Result {
    let (dir, store) = empty_store()?;

    let tx = store.transaction();

    assert_eq!(
        std::fs::read_dir(dir.path())?.count(),
        0,
        "nothing happens before preparation"
    );

    let tx = tx.prepare(
        Some(create_symbolic_at("HEAD", "refs/heads/main")),
        Fail::Immediately,
        Fail::Immediately,
    )?;
    assert_eq!(std::fs::read_dir(dir.path())?.count(), 1, "the lock file was created");

    drop(tx);
    assert_eq!(std::fs::read_dir(dir.path())?.count(), 0, "everything vanished");
    Ok(())
}

#[test]
fn symbolic_reference_writes_reflog_if_previous_value_is_set() -> crate::Result {
    let (_keep, store) = empty_store()?;
    let referent = "refs/heads/alt-main";
    assert!(
        store.try_find_loose(referent)?.is_none(),
        "the reference does not exist"
    );
    let log = LogChange {
        mode: RefLog::AndReference,
        force_create_reflog: false,
        message: "message".into(),
    };
    let new_head_value = Target::Symbolic(referent.try_into().unwrap());
    let new_oid = hex_to_id("28ce6a8b26aa170e1de65536fe8abe1832bd3242");
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Update {
                    log,
                    new: new_head_value,
                    expected: PreviousValue::ExistingMustMatch(Target::Object(new_oid)),
                },
                name: "refs/heads/symbolic".try_into()?,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref())?;
    assert_eq!(edits.len(), 1, "no split was performed");
    let head = store.find_loose(&edits[0].name)?;
    assert_eq!(head.name.as_bstr(), "refs/heads/symbolic");
    assert_eq!(head.kind(), gix_ref::Kind::Symbolic);
    assert_eq!(
        head.target.to_ref().try_name().map(gix_ref::FullNameRef::as_bstr),
        Some(referent.as_bytes().as_bstr())
    );
    assert!(
        head.log_exists(&store),
        "reflog is written for new symbolic ref with information about the peeled target id, \
         as special accommodation for the state during clone to allow us to get a peeled id into the log"
    );
    assert!(store.try_find_loose(referent)?.is_none(), "referent wasn't created");

    Ok(())
}

#[test]
fn windows_device_name_is_illegal_with_enabled_windows_protections() -> crate::Result {
    let (_keep, mut store) = empty_store()?;
    store.prohibit_windows_device_names = true;
    let log_ignored = LogChange {
        mode: RefLog::AndReference,
        force_create_reflog: false,
        message: "ignored".into(),
    };

    let new = Target::Object(hex_to_id("28ce6a8b26aa170e1de65536fe8abe1832bd3242"));
    for invalid_name in ["refs/heads/CON", "refs/CON/still-invalid"] {
        let err = store
            .transaction()
            .prepare(
                Some(RefEdit {
                    change: Change::Update {
                        log: log_ignored.clone(),
                        new: new.clone(),
                        expected: PreviousValue::Any,
                    },
                    name: invalid_name.try_into()?,
                    deref: false,
                }),
                Fail::Immediately,
                Fail::Immediately,
            )
            .unwrap_err();
        assert_eq!(
            err.source().expect("inner").to_string(),
            format!("Illegal use of reserved Windows device name in \"{invalid_name}\""),
            "it's notable that the check also kicks in when the previous value doesn't matter - we expect a 'read' to happen anyway \
            - it can't be optimized away as the previous value is stored in the transaction result right now."
        );
    }

    #[cfg(not(windows))]
    {
        store.prohibit_windows_device_names = false;
        let _prepared_transaction = store.transaction().prepare(
            Some(RefEdit {
                change: Change::Update {
                    log: log_ignored.clone(),
                    new,
                    expected: PreviousValue::Any,
                },
                name: "refs/heads/CON".try_into()?,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?;
    }

    Ok(())
}

#[test]
fn symbolic_head_missing_referent_then_update_referent() -> crate::Result {
    for reflog_writemode in &[WriteReflog::Normal, WriteReflog::Disable, WriteReflog::Always] {
        let (_keep, mut store) = empty_store()?;
        store.write_reflog = *reflog_writemode;
        let referent = "refs/heads/alt-main";
        assert!(
            store.try_find_loose(referent)?.is_none(),
            "the reference does not exist"
        );
        let log_ignored = LogChange {
            mode: RefLog::AndReference,
            force_create_reflog: false,
            message: "ignored".into(),
        };
        let new_head_value = Target::Symbolic(referent.try_into().unwrap());
        let edits = store
            .transaction()
            .prepare(
                Some(RefEdit {
                    change: Change::Update {
                        log: log_ignored.clone(),
                        new: new_head_value.clone(),
                        expected: PreviousValue::MustNotExist,
                    },
                    name: "HEAD".try_into()?,
                    deref: false,
                }),
                Fail::Immediately,
                Fail::Immediately,
            )?
            .commit(committer().to_ref())?;
        assert_eq!(
            edits,
            vec![RefEdit {
                change: Change::Update {
                    log: log_ignored.clone(),
                    new: new_head_value.clone(),
                    expected: PreviousValue::MustNotExist,
                },
                name: "HEAD".try_into()?,
                deref: false,
            }],
            "no split was performed"
        );

        let head = store.find_loose(&edits[0].name)?;
        assert_eq!(head.name.as_bstr(), "HEAD");
        assert_eq!(head.kind(), gix_ref::Kind::Symbolic);
        assert_eq!(
            std::fs::read_to_string(store.git_dir().join("HEAD"))?,
            "ref: refs/heads/alt-main\n",
            "note the newline - symbolic refs really want a newline just like git does it, otherwise some tools may break"
        );
        assert_eq!(
            head.target.to_ref().try_name().map(gix_ref::FullNameRef::as_bstr),
            Some(referent.as_bytes().as_bstr())
        );
        assert!(!head.log_exists(&store), "no reflog is written for symbolic ref");
        assert!(store.try_find_loose(referent)?.is_none(), "referent wasn't created");

        let new_oid = hex_to_id("28ce6a8b26aa170e1de65536fe8abe1832bd3242");
        let new = Target::Object(new_oid);
        let log = LogChange {
            message: "an actual change".into(),
            mode: RefLog::AndReference,
            force_create_reflog: false,
        };
        let edits = store
            .transaction()
            .prepare(
                Some(RefEdit {
                    change: Change::Update {
                        log: log.clone(),
                        new: new.clone(),
                        expected: PreviousValue::Any,
                    },
                    name: "HEAD".try_into()?,
                    deref: true,
                }),
                Fail::Immediately,
                Fail::Immediately,
            )?
            .commit(committer().to_ref())?;

        assert_eq!(
            edits,
            vec![
                RefEdit {
                    change: Change::Update {
                        log: {
                            let mut l = log.clone();
                            l.mode = RefLog::Only;
                            l
                        },
                        new: new.clone(),
                        expected: PreviousValue::MustExistAndMatch(new_head_value.clone()),
                    },
                    name: "HEAD".try_into()?,
                    deref: false,
                },
                RefEdit {
                    change: Change::Update {
                        log,
                        new: new.clone(),
                        expected: PreviousValue::Any, // there is no previous value, so we can't put `MustExistAndMatch` here.
                    },
                    name: referent.try_into()?,
                    deref: false,
                }
            ]
        );

        let head = store.find_loose("HEAD")?;
        assert_eq!(
            head.kind(),
            gix_ref::Kind::Symbolic,
            "head is still symbolic, not detached"
        );
        assert_eq!(
            head.target.to_ref().try_name().map(gix_ref::FullNameRef::as_bstr),
            Some(referent.as_bytes().as_bstr()),
            "it still points to the referent"
        );

        let referent_ref = store.find_loose(referent)?;
        assert_eq!(referent_ref.kind(), gix_ref::Kind::Object, "referent is a peeled ref");
        assert_eq!(
            referent_ref.target.to_ref().try_id(),
            Some(new_oid.as_ref()),
            "referent points to desired hash"
        );

        let mut buf = Vec::new();
        for ref_name in &["HEAD", referent] {
            match reflog_writemode {
                WriteReflog::Normal | WriteReflog::Always => {
                    let expected_line = log_line(gix_hash::Kind::Sha1.null(), new_oid, "an actual change");
                    assert_eq!(reflog_lines(&store, ref_name)?, vec![expected_line]);
                }
                WriteReflog::Disable => {
                    assert!(
                        store.reflog_iter(*ref_name, &mut buf)?.is_none(),
                        "nothing is ever written if its disabled"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
/// Writing a peeled ref to which head points to doesn't update HEAD on the fly even though that might be what's would
/// be needed to keep the reflog consistent
fn write_reference_to_which_head_points_to_does_not_update_heads_reflog_even_though_it_should() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.find_loose("HEAD")?;
    let referent = head.target.to_ref().try_name().expect("symbolic ref").to_owned();
    let previous_head_reflog = reflog_lines(&store, "HEAD")?;

    let new_id = hex_to_id("01dd4e2a978a9f5bd773dae6da7aa4a5ac1cdbbc");
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: false,
                        message: "".into(),
                    },
                    expected: PreviousValue::MustExist,
                    new: Target::Object(new_id),
                },
                name: referent.as_bstr().try_into()?,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref())?;

    assert_eq!(edits.len(), 1, "HEAD wasn't update");
    assert_eq!(
        edits,
        vec![RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: false,
                    message: "".into(),
                },
                expected: PreviousValue::MustExistAndMatch(Target::Object(hex_to_id(
                    "02a7a22d90d7c02fb494ed25551850b868e634f0"
                ))),
                new: Target::Object(new_id),
            },
            name: referent.as_bstr().try_into()?,
            deref: false,
        }]
    );
    assert_eq!(
        reflog_lines(&store, "HEAD")?,
        previous_head_reflog,
        "nothing changed in the heads reflog"
    );

    let expected_line = log_line(hex_to_id("02a7a22d90d7c02fb494ed25551850b868e634f0"), new_id, "");
    assert_eq!(
        reflog_lines(&store, &referent.to_string())?
            .last()
            .expect("at least one line"),
        &expected_line,
        "referent line matches the expected one"
    );
    Ok(())
}

#[test]
fn packed_refs_are_looked_up_when_checking_existing_values() -> crate::Result {
    let (_keep, store) = store_writable("make_packed_ref_repository.sh")?;
    assert!(
        store.try_find_loose("main")?.is_none(),
        "no loose main available, it's packed"
    );
    let new_id = hex_to_id("0000000000000000000000000000000000000001");
    let old_id = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Update {
                    log: LogChange {
                        mode: RefLog::AndReference,
                        force_create_reflog: false,
                        message: "for pack".into(),
                    },
                    expected: PreviousValue::MustExistAndMatch(Target::Object(old_id)),
                    new: Target::Object(new_id),
                },
                name: "refs/heads/main".try_into()?,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref())?;

    assert_eq!(edits.len(), 1, "only one edit was performed in the loose refs store");

    let packed = store.open_packed_buffer().unwrap().expect("packed refs is available");
    assert_eq!(
            packed.find("main")?.target(),
            old_id,
            "packed refs aren't rewritten, the change goes into the loose ref instead which shadows packed refs of same name"
        );
    assert_eq!(
        store.find_loose("main")?.target.try_id(),
        Some(new_id.as_ref()),
        "the new id was written to the loose ref"
    );
    Ok(())
}

#[test]
fn packed_refs_creation_with_tag_loop_are_not_handled_and_cannot_exist_due_to_object_hashes() {
    // Tag loops cannot exist as you cannot create them thanks to hashing.
}

#[test]
fn packed_refs_creation_with_packed_refs_mode_prune_removes_original_loose_refs() -> crate::Result {
    let (_keep, store) = store_writable("make_ref_repository.sh")?;
    assert!(
        store.open_packed_buffer()?.is_none(),
        "there should be no packed refs to start out with"
    );
    let odb = gix_odb::at(store.git_dir().join("objects"))?;
    let edits = store
        .transaction()
        .packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(
            Box::new(odb),
        ))
        .prepare(
            store
                .loose_iter()?
                .filter_map(|r| r.ok().filter(|r| r.kind() == gix_ref::Kind::Object))
                .map(|r| RefEdit {
                    change: Change::Update {
                        log: LogChange::default(),
                        expected: PreviousValue::MustExistAndMatch(r.target.clone()),
                        new: r.target,
                    },
                    name: r.name,
                    deref: false,
                }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref())?;

    assert_eq!(
        edits.len(),
        11,
        "there are a certain amount of loose refs that are packed"
    );

    assert!(
        store
            .loose_iter()?
            .filter_map(Result::ok)
            .all(|r| r.kind() == gix_ref::Kind::Symbolic),
        "only symbolic refs are left"
    );

    let other_store = store_with_packed_refs()?;
    let expected_pack_data: BString = std::fs::read(other_store.packed_refs_path())?.into();
    let actual_packed_data: BString = std::fs::read(store.packed_refs_path())?.into();
    assert_eq!(
        actual_packed_data, expected_pack_data,
        "both gitoxide and git must agree on the packed refs file perfectly"
    );
    Ok(())
}

#[test]
fn packed_refs_creation_with_packed_refs_mode_leave_keeps_original_loose_refs() -> crate::Result {
    let (_keep, store) = store_writable("make_packed_ref_repository_for_overlay.sh")?;
    let branch = store.find("newer-as-loose")?;
    let packed = store.open_packed_buffer()?.expect("packed-refs");
    assert_ne!(
        packed.find("newer-as-loose")?.target(),
        branch.target.try_id().expect("peeled"),
        "the packed ref is outdated"
    );
    let previous_reflog_entries = branch.log_iter(&store).all()?.expect("log").count();
    let previous_packed_refs = packed.iter()?.filter_map(Result::ok).count();

    let edits = store.loose_iter()?.map(|r| r.expect("valid ref")).map(|r| RefEdit {
        change: Change::Update {
            log: LogChange::default(),
            expected: PreviousValue::MustExistAndMatch(r.target.clone()),
            new: r.target,
        },
        name: r.name,
        deref: false,
    });

    let edits = store
        .transaction()
        .packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdates(Box::new(EmptyCommit)))
        .prepare(edits, Fail::Immediately, Fail::Immediately)?
        .commit(committer().to_ref())?;
    assert_eq!(
            edits.len(),
            2,
            "it claims to have performed all desired operations, even though some don't make it into the pack as 'side-car'"
        );

    assert_eq!(
        store.loose_iter()?.filter_map(Result::ok).count(),
        edits.len(),
        "the amount of loose refs didn't change and having symbolic ones isn't a problem"
    );
    assert_eq!(
        branch.log_iter(&store).all()?.expect("log").count(),
        previous_reflog_entries,
        "reflog isn't adjusted as there is no change"
    );

    let packed = store.open_packed_buffer()?.expect("packed-refs");
    assert_eq!(
        packed.iter()?.filter_map(Result::ok).count(),
        previous_packed_refs,
        "the amount of packed refs doesn't change"
    );
    assert_eq!(
        packed.find("newer-as-loose")?.target(),
        store.find("newer-as-loose")?.target.into_id(),
        "the packed ref is now up to date and the loose ref definitely still exists"
    );
    Ok(())
}

#[test]
fn packed_refs_deletion_in_deletions_and_updates_mode() -> crate::Result {
    let (_keep, store) = store_writable("make_packed_ref_repository.sh")?;
    assert!(
        store.try_find_loose("refs/heads/d1")?.is_none(),
        "no loose d1 available, it's packed"
    );
    let odb = gix_odb::at(store.git_dir().join("objects"))?;
    let old_id = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let edits = store
        .transaction()
        .packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdates(Box::new(odb)))
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::MustExistAndMatch(Target::Object(old_id)),
                    log: RefLog::AndReference,
                },
                name: "refs/heads/d1".try_into()?,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref())?;

    assert_eq!(edits.len(), 1, "only one edit was performed in the packed refs store");

    let packed = store.open_packed_buffer().unwrap().expect("packed refs is available");
    assert!(
        packed.try_find("refs/heads/d1")?.is_none(),
        "d1 should be removed from packed refs"
    );
    Ok(())
}
