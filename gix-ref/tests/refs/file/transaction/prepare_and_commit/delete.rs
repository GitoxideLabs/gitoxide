use crate::{
    file::{
        store_writable,
        transaction::prepare_and_commit::{committer, empty_store},
    },
    hex_to_id,
};
use gix_date::parse::TimeBuf;
use gix_lock::acquire::Fail;
use gix_ref::file::transaction::prepare::Error;
use gix_ref::transaction::LogChange;
use gix_ref::{
    FullName, Reference, Target,
    file::ReferenceExt,
    transaction::{Change, PreviousValue, RefEdit, RefLog},
};

#[test]
fn delete_a_ref_which_is_gone_succeeds() -> crate::Result {
    let (_keep, store) = empty_store()?;
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::Any,
                    log: RefLog::AndReference,
                },
                name: "DOES_NOT_EXIST".try_into()?,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;
    assert_eq!(edits.len(), 1);
    Ok(())
}

#[test]
fn delete_a_ref_which_is_gone_but_must_exist_fails() -> crate::Result {
    let (_keep, store) = empty_store()?;
    let res = store.transaction().prepare(
        Some(RefEdit {
            change: Change::Delete {
                expected: PreviousValue::MustExist,
                log: RefLog::AndReference,
            },
            name: "DOES_NOT_EXIST".try_into()?,
            deref: false,
        }),
        Fail::Immediately,
        Fail::Immediately,
    );
    match res {
        Ok(_) => unreachable!("must exist, but it doesn't actually exist"),
        Err(err) => assert_eq!(
            err.to_string(),
            "The reference \"DOES_NOT_EXIST\" for deletion did not exist or could not be parsed"
        ),
    }
    Ok(())
}

#[test]
fn delete_ref_and_reflog_on_symbolic_no_deref() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.find_loose("HEAD")?;
    assert!(head.log_exists(&store));
    let _main = store.find_loose("main")?;

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::MustExist,
                    log: RefLog::AndReference,
                },
                name: head.name.clone(),
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert_eq!(
        edits,
        vec![RefEdit {
            change: Change::Delete {
                expected: PreviousValue::MustExistAndMatch(Target::Symbolic("refs/heads/main".try_into()?)),
                log: RefLog::AndReference,
            },
            name: head.name,
            deref: false
        }],
        "the previous value was updated with the actual one"
    );
    assert!(
        store.reflog_iter_rev("HEAD", &mut [0u8; 128])?.is_none(),
        "reflog was deleted"
    );
    assert!(store.try_find_loose("HEAD")?.is_none(), "ref was deleted");
    assert!(store.try_find_loose("main")?.is_some(), "referent still exists");
    Ok(())
}

#[test]
fn delete_ref_with_incorrect_previous_value_fails() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.find_loose("HEAD")?;
    assert!(head.log_exists(&store));

    let res = store.transaction().prepare(
        Some(RefEdit {
            change: Change::Delete {
                expected: PreviousValue::MustExistAndMatch(Target::Symbolic("refs/heads/main".try_into()?)),
                log: RefLog::Only,
            },
            name: head.name,
            deref: true,
        }),
        Fail::Immediately,
        Fail::Immediately,
    );

    match res {
        Err(err) => {
            assert_eq!(
                err.to_string(),
                format!(
                    "The reference \"refs/heads/main\" should have content ref: refs/heads/main, actual content was {}",
                    hex_to_id("02a7a22d90d7c02fb494ed25551850b868e634f0")
                )
            );
        }
        Ok(_) => unreachable!("must be err"),
    }
    // everything stays as is
    let head = store.find_loose("HEAD")?;
    assert!(head.log_exists(&store));
    let main = store.find_loose("main").expect("referent still exists");
    assert!(main.log_exists(&store));
    Ok(())
}

#[test]
fn delete_reflog_only_of_symbolic_no_deref() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.find_loose("HEAD")?;
    assert!(head.log_exists(&store));

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::MustExistAndMatch(Target::Symbolic("refs/heads/main".try_into()?)),
                    log: RefLog::Only,
                },
                name: head.name,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert_eq!(edits.len(), 1);
    let head: Reference = store.find_loose("HEAD")?.into();
    assert!(!head.log_exists(&store));
    let main = store.find_loose("main").expect("referent still exists");
    assert!(main.log_exists(&store), "log is untouched, too");
    assert_eq!(
        main.target,
        head.follow(&store).expect("a symref")?.target,
        "head points to main"
    );
    Ok(())
}

#[test]
fn delete_reflog_only_of_symbolic_with_deref() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.find_loose("HEAD")?;
    assert!(head.log_exists(&store));

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::MustExist,
                    log: RefLog::Only,
                },
                name: head.name,
                deref: true,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert_eq!(edits.len(), 2);
    let head: Reference = store.find_loose("HEAD")?.into();
    assert!(!head.log_exists(&store));
    let main = store.find_loose("main").expect("referent still exists");
    assert!(!main.log_exists(&store), "log is removed");
    assert_eq!(
        main.target,
        head.follow(&store).expect("a symref")?.target,
        "head points to main"
    );
    Ok(())
}

#[test]
fn rename_a_to_a_slash_b_in_one_transaction() -> crate::Result {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let old = store.find_loose("old")?;

    let new_name: FullName = "refs/heads/old/new".try_into()?;
    let err = store
        .transaction()
        .prepare(
            [
                RefEdit {
                    change: Change::Delete {
                        expected: PreviousValue::MustExist,
                        log: RefLog::AndReference,
                    },
                    name: old.name.clone(),
                    deref: true,
                },
                RefEdit {
                    change: Change::Update {
                        expected: PreviousValue::MustNotExist,
                        log: LogChange::default(),
                        new: old.target.clone(),
                    },
                    name: new_name.clone(),
                    deref: true,
                },
            ],
            Fail::Immediately,
            Fail::Immediately,
        )
        .unwrap_err();

    match err {
        #[cfg(unix)]
        Error::Io(err) => {
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::NotADirectory,
                "For now this isn't supported in the same transaction."
            );
        }
        #[cfg(windows)]
        Error::LockAcquire { .. } => {
            // It's bad that the error differs on Windows, but then again, doing this kind of pseudo-db on a filesystem
            // probably is never going to be great, so let's wait for ref-tables.
        }
        err => unreachable!("unexpected error variant: {err:?}"),
    }

    let edits = store
        .transaction()
        .prepare(
            [RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::MustExist,
                    log: RefLog::AndReference,
                },
                name: old.name,
                deref: true,
            }],
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;
    assert_eq!(edits.len(), 1, "First delete to make space");

    let edits = store
        .transaction()
        .prepare(
            [RefEdit {
                change: Change::Update {
                    expected: PreviousValue::MustNotExist,
                    log: LogChange::default(),
                    new: old.target,
                },
                name: new_name.clone(),
                deref: true,
            }],
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;
    assert_eq!(edits.len(), 1, "Then create the new ref in place of the old");
    assert!(
        store.try_find_loose(new_name.as_ref())?.is_some(),
        "must have created the new reference"
    );
    Ok(())
}

#[test]
/// Based on https://github.com/git/git/blob/master/refs/files-backend.c#L514:L515
fn delete_broken_ref_that_must_exist_fails_as_it_is_no_valid_ref() -> crate::Result {
    let (_keep, store) = empty_store()?;
    std::fs::write(store.git_dir().join("HEAD"), b"broken")?;
    assert!(store.try_find_loose("HEAD").is_err(), "the ref is truly broken");

    let res = store.transaction().prepare(
        Some(RefEdit {
            change: Change::Delete {
                expected: PreviousValue::MustExist,
                log: RefLog::AndReference,
            },
            name: "HEAD".try_into()?,
            deref: true,
        }),
        Fail::Immediately,
        Fail::Immediately,
    );
    match res {
        Err(err) => {
            assert_eq!(
                err.to_string(),
                "The reference \"HEAD\" for deletion did not exist or could not be parsed"
            );
        }
        Ok(_) => unreachable!("expected error"),
    }
    Ok(())
}

#[test]
fn non_existing_can_be_deleted_with_the_may_exist_match_constraint() -> crate::Result {
    let (_keep, store) = empty_store()?;
    let previous_value =
        PreviousValue::ExistingMustMatch(Target::Object(hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03")));
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    expected: previous_value.clone(),
                    log: RefLog::AndReference,
                },
                name: "refs/heads/not-there".try_into()?,
                deref: true,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert_eq!(
        edits,
        vec![RefEdit {
            change: Change::Delete {
                expected: previous_value,
                log: RefLog::AndReference,
            },
            name: "refs/heads/not-there".try_into()?,
            deref: false,
        }]
    );
    Ok(())
}

#[test]
/// Based on https://github.com/git/git/blob/master/refs/files-backend.c#L514:L515
fn delete_broken_ref_that_may_not_exist_works_even_in_deref_mode() -> crate::Result {
    let (_keep, store) = empty_store()?;
    std::fs::write(store.git_dir().join("HEAD"), b"broken")?;
    assert!(store.try_find_loose("HEAD").is_err(), "the ref is truly broken");

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::Any,
                    log: RefLog::AndReference,
                },
                name: "HEAD".try_into()?,
                deref: true,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert!(store.try_find_loose("HEAD")?.is_none(), "the ref was deleted");
    assert_eq!(
        edits,
        vec![RefEdit {
            change: Change::Delete {
                expected: PreviousValue::Any,
                log: RefLog::AndReference,
            },
            name: "HEAD".try_into()?,
            deref: false,
        }]
    );
    Ok(())
}

#[test]
fn store_write_mode_has_no_effect_and_reflogs_are_always_deleted() -> crate::Result {
    for reflog_writemode in &[
        gix_ref::store::WriteReflog::Normal,
        gix_ref::store::WriteReflog::Disable,
    ] {
        let (_keep, mut store) = store_writable("make_repo_for_reflog.sh")?;
        store.write_reflog = *reflog_writemode;
        assert!(store.find_loose("HEAD")?.log_exists(&store));
        assert!(store.open_packed_buffer()?.is_none(), "there is no pack");

        let edits = store
            .transaction()
            .prepare(
                Some(RefEdit {
                    change: Change::Delete {
                        expected: PreviousValue::Any,
                        log: RefLog::Only,
                    },
                    name: "HEAD".try_into()?,
                    deref: false,
                }),
                Fail::Immediately,
                Fail::Immediately,
            )?
            .commit(committer().to_ref(&mut TimeBuf::default()))?;
        assert_eq!(edits.len(), 1);
        assert!(!store.find_loose("HEAD")?.log_exists(&store), "log was deleted");
        assert!(store.open_packed_buffer()?.is_none(), "there still is no pack");
    }
    Ok(())
}

#[test]
fn packed_refs_are_consulted_when_determining_previous_value_of_ref_to_be_deleted_and_are_deleted_from_packed_ref_file()
-> crate::Result {
    let (_keep, store) = store_writable("make_packed_ref_repository.sh")?;
    assert!(
        store.try_find_loose("main")?.is_none(),
        "no loose main available, it's packed"
    );
    assert!(
        store.open_packed_buffer()?.expect("packed").try_find("main")?.is_some(),
        "packed main is available"
    );

    let old_id = hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::MustExistAndMatch(Target::Object(old_id)),
                    log: RefLog::AndReference,
                },
                name: "refs/heads/main".try_into()?,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert_eq!(edits.len(), 1, "an edit was performed in the packed refs store");
    let packed = store.open_packed_buffer()?.expect("packed ref present");
    assert!(packed.try_find("main")?.is_none(), "no main present after deletion");
    Ok(())
}

#[test]
fn a_loose_ref_with_old_value_check_and_outdated_packed_refs_value_deletes_both_refs() -> crate::Result {
    let (_keep, store) = store_writable("make_packed_ref_repository_for_overlay.sh")?;
    let packed = store.open_packed_buffer()?.expect("packed-refs");
    let branch = store.find("newer-as-loose")?;
    let branch_id = branch.target.try_id().map(ToOwned::to_owned).expect("peeled");
    assert_ne!(
        packed.find("newer-as-loose")?.target(),
        branch_id,
        "the packed ref is outdated"
    );

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit {
                change: Change::Delete {
                    expected: PreviousValue::MustExistAndMatch(Target::Object(branch_id)),
                    log: RefLog::AndReference,
                },
                name: branch.name,
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert_eq!(
        edits.len(),
        1,
        "only one edit even though technically two places were changed"
    );
    assert!(
        store.try_find("newer-as-loose")?.is_none(),
        "reference is deleted everywhere"
    );
    Ok(())
}

#[test]
fn all_contained_references_deletes_the_packed_ref_file_too() -> crate::Result {
    for mode in ["must-exist", "may-exist"] {
        let (_keep, store) = store_writable("make_packed_ref_repository.sh")?;
        let edits = store
            .transaction()
            .prepare(
                store.open_packed_buffer()?.expect("packed-refs").iter()?.map(|r| {
                    let r = r.expect("valid ref");
                    RefEdit {
                        change: Change::Delete {
                            expected: match mode {
                                "must-exist" => PreviousValue::MustExistAndMatch(Target::Object(r.target())),
                                "may-exist" => PreviousValue::ExistingMustMatch(Target::Object(r.target())),
                                _ => unimplemented!("unknown mode: {}", mode),
                            },
                            log: RefLog::AndReference,
                        },
                        name: r.name.into(),
                        deref: false,
                    }
                }),
                Fail::Immediately,
                Fail::Immediately,
            )?
            .commit(committer().to_ref(&mut TimeBuf::default()))?;

        assert!(!store.packed_refs_path().is_file(), "packed-refs was entirely removed");

        let packed = store.open_packed_buffer()?;
        assert!(packed.is_none(), "it won't make up packed refs");
        for edit in edits {
            assert!(store.try_find(&edit.name)?.is_none(), "delete ref cannot be found");
        }
    }
    Ok(())
}
