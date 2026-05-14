use gix_date::parse::TimeBuf;
use gix_lock::acquire::Fail;
use gix_ref::{
    Target,
    file::transaction::PackedRefs,
    transaction::{Change, LogChange, PreviousValue, RefEdit},
};

use crate::{
    file::{
        EmptyCommit,
        transaction::prepare_and_commit::{committer, create_at, create_symbolic_at, delete_at, empty_store},
    },
    hex_to_id,
};

fn case_sensitive(tmp_dir: &std::path::Path) -> bool {
    std::fs::write(tmp_dir.join("config"), "").expect("can create file once");
    !gix_fs::Capabilities::probe(tmp_dir).ignore_case
}

#[test]
fn conflicting_creation_without_packed_refs() -> crate::Result {
    let (dir, store) = empty_store()?;
    let res = store.transaction().prepare(
        [create_at("refs/a"), create_at("refs/A")],
        Fail::Immediately,
        Fail::Immediately,
    );

    let case_sensitive = case_sensitive(dir.path());
    match res {
        Ok(_) if case_sensitive => {}
        Ok(_) if !case_sensitive => panic!("should fail as 'a' and 'A' clash"),
        Err(err) if case_sensitive => panic!("should work as case sensitivity allows 'a' and 'A' to coexist: {err:?}"),
        Err(err) if !case_sensitive => {
            assert_eq!(err.to_string(), "A lock could not be obtained for reference \"refs/A\"");
        }
        _ => unreachable!("actually everything is covered"),
    }
    Ok(())
}

#[test]
fn non_conflicting_creation_without_packed_refs_work() -> crate::Result {
    let (_dir, store) = empty_store()?;
    let ongoing = store
        .transaction()
        .prepare([create_at("refs/new")], Fail::Immediately, Fail::Immediately)
        .unwrap();

    let t2 = store.transaction().prepare(
        [create_at("refs/non-conflicting")],
        Fail::Immediately,
        Fail::Immediately,
    )?;

    let mut buf = TimeBuf::default();
    t2.commit(committer().to_ref(&mut buf))?;
    ongoing.commit(committer().to_ref(&mut buf))?;

    assert!(store.reflog_exists("refs/new")?);
    assert!(store.reflog_exists("refs/non-conflicting")?);

    Ok(())
}

#[test]
fn packed_refs_lock_is_mandatory_for_multiple_ongoing_transactions_even_if_one_does_not_need_it() -> crate::Result {
    let (_dir, store) = empty_store()?;
    let ref_name = "refs/a";
    let _t1 = store
        .transaction()
        .packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(
            Box::new(EmptyCommit),
        ))
        .prepare([create_at(ref_name)], Fail::Immediately, Fail::Immediately)?;

    let t2res = store
        .transaction()
        .prepare([delete_at(ref_name)], Fail::Immediately, Fail::Immediately);
    assert_eq!(
        &t2res.unwrap_err().to_string()[..54],
        "The lock for the packed-ref file could not be obtained",
        "if packed-refs are about to be created, other transactions always acquire a packed-refs lock as to not miss anything"
    );
    Ok(())
}

#[test]
fn conflicting_creation_into_packed_refs() -> crate::Result {
    let (_dir, store) = empty_store()?;
    let mut buf = TimeBuf::default();
    store
        .transaction()
        .packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(
            Box::new(EmptyCommit),
        ))
        .prepare(
            [
                create_at("refs/a"),
                create_at("refs/A"),
                create_symbolic_at("refs/symbolic", "refs/heads/target"),
            ],
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut buf))?;

    assert_eq!(
        store.cached_packed_buffer()?.expect("created").iter()?.count(),
        2,
        "packed-refs can store everything in case-insensitive manner"
    );
    assert_eq!(
        store.loose_iter()?.count(),
        1,
        "symbolic refs can't be packed and stay loose"
    );
    assert!(store.reflog_exists("refs/a")?);
    assert!(store.reflog_exists("refs/A")?);
    assert!(!store.reflog_exists("refs/symbolic")?, "and they can't have reflogs");

    // The following works because locks aren't actually obtained if there would be no change.
    // Otherwise there would be a conflict on case-insensitive filesystems
    let null = crate::fixture_hash_kind().null();
    store
        .transaction()
        .packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(
            Box::new(EmptyCommit),
        ))
        .prepare(
            [
                RefEdit {
                    change: Change::Update {
                        log: LogChange::default(),
                        expected: PreviousValue::Any,
                        new: Target::Object(null),
                    },
                    name: "refs/a".try_into().expect("valid"),
                    deref: false,
                },
                RefEdit {
                    change: Change::Update {
                        log: LogChange::default(),
                        expected: PreviousValue::MustExistAndMatch(Target::Object(hex_to_id(
                            "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
                        ))),
                        new: Target::Object(null),
                    },
                    name: "refs/A".try_into().expect("valid"),
                    deref: false,
                },
            ],
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut buf))?;
    assert_eq!(store.iter()?.all()?.count(), 3);

    {
        let _ongoing = store
            .transaction()
            .prepare([create_at("refs/new")], Fail::Immediately, Fail::Immediately)?;

        let t2res = store.transaction().prepare(
            [create_at("refs/non-conflicting")],
            Fail::Immediately,
            Fail::Immediately,
        );

        assert_eq!(
            &t2res.unwrap_err().to_string()[..40],
            "The lock for the packed-ref file could n",
            "packed-refs files will always be locked if they are present as we have to look up their content"
        );
    }

    {
        let _ongoing = store
            .transaction()
            .prepare([delete_at("refs/a")], Fail::Immediately, Fail::Immediately)?;

        let t2res = store
            .transaction()
            .prepare([delete_at("refs/A")], Fail::Immediately, Fail::Immediately);

        assert_eq!(
            &t2res.unwrap_err().to_string()[..40],
            "The lock for the packed-ref file could n",
            "once again, packed-refs save the day"
        );
    }

    // Create a loose ref at a path
    assert_eq!(store.loose_iter()?.count(), 1, "a symref");
    store
        .transaction()
        .prepare(
            [RefEdit {
                change: Change::Update {
                    log: LogChange::default(),
                    expected: PreviousValue::Any,
                    new: Target::Symbolic("refs/heads/does-not-matter".try_into().expect("valid")),
                },
                name: "refs/a".try_into().expect("valid"),
                deref: false,
            }],
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut buf))?;
    assert_eq!(
        store.loose_iter()?.count(),
        2,
        "we created a loose ref, overlaying the packed one, and have a symbolic one"
    );

    store
        .transaction()
        .prepare(
            [delete_at("refs/a"), delete_at("refs/A"), delete_at("refs/symbolic")],
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut buf))?;

    assert_eq!(
        store.iter()?.all()?.count(),
        0,
        "we deleted our only two packed refs and one loose ref with the same name"
    );
    assert!(!store.reflog_exists("refs/a")?);
    assert!(!store.reflog_exists("refs/A")?);
    Ok(())
}
