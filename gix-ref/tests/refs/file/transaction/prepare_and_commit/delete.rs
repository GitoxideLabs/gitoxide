use crate::{
    file::{
        store_writable,
        transaction::prepare_and_commit::{committer, empty_store},
    },
    hex_to_id,
};
use gix_date::parse::TimeBuf;
use gix_lock::acquire::Fail;
use gix_ref::{
    FullName, Reference, Target,
    file::ReferenceExt,
    transaction::{PreviousValue, RefEdit, RefLog},
};
use gix_testtools::TestResult;

#[test]
fn deleting_the_last_loose_reference_preserves_refs_directory_like_git() -> TestResult {
    for (case, name) in [("flat", "refs/topic"), ("nested", "refs/heads/a/b/topic")] {
        let fixture = crate::scripted_fixture_writable("keep_refs_directory.sh")?;
        let git_dir = fixture.path().join(format!("delete-{case}/.git"));
        let expected_git_dir = fixture.path().join(format!("git-delete-{case}/.git"));
        assert!(
            expected_git_dir.join("refs").is_dir(),
            "{case}: Git preserves the refs root"
        );
        let store = gix_ref::file::Store::at(git_dir.clone(), crate::fixture_hash_kind());
        store
            .transaction()
            .prepare(
                [RefEdit::delete(name.try_into()?, PreviousValue::MustExist)],
                Fail::Immediately,
                Fail::Immediately,
            )?
            .commit(None)?;
        assert!(store.try_find(name)?.is_none(), "{case}: the reference was deleted");
        assert!(
            git_dir.join("refs").is_dir(),
            "{case}: deletion must preserve the refs root"
        );
        assert_eq!(
            std::fs::read_dir(git_dir.join("refs"))?.count(),
            0,
            "{case}: no reference, lock, or empty descendant directories remain"
        );
        gix_discover::is_git(&git_dir)?;
    }
    Ok(())
}

#[test]
fn delete_a_ref_which_is_gone_succeeds() -> TestResult {
    let (_keep, store) = empty_store()?;
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit::delete("DOES_NOT_EXIST".try_into()?, PreviousValue::Any)),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;
    assert_eq!(edits.len(), 1);
    Ok(())
}

#[test]
fn delete_a_ref_which_is_gone_but_must_exist_fails() -> TestResult {
    let (_keep, store) = empty_store()?;
    let res = store.transaction().prepare(
        Some(RefEdit::delete("DOES_NOT_EXIST".try_into()?, PreviousValue::MustExist)),
        Fail::Immediately,
        Fail::Immediately,
    );
    let err = res.expect_err("the reference must exist");
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[(&store.git_dir().to_string_lossy(), "<git-dir>")]), "delete a ref which is gone but must exist fails", @r#"
    Could not prepare reference edit, reference="DOES_NOT_EXIST"

    Caused by:
        0: The reference to delete must exist
    "#);
    assert!(err.is_not_found(), "delete a ref which is gone but must exist fails");
    Ok(())
}

#[test]
fn delete_ref_and_reflog_on_symbolic_no_deref() -> TestResult {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.find_loose("HEAD")?;
    assert!(head.log_exists(&store)?);
    let _main = store.find_loose("main")?;

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit::delete(head.name.clone(), PreviousValue::MustExist)),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert_eq!(
        edits,
        vec![RefEdit::delete(
            head.name,
            PreviousValue::MustExistAndMatch(Target::Symbolic("refs/heads/main".try_into()?)),
        )],
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
fn delete_ref_with_incorrect_previous_value_fails() -> TestResult {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.find_loose("HEAD")?;
    assert!(head.log_exists(&store)?);

    let res = store.transaction().prepare(
        Some(
            RefEdit::delete_with_log(
                head.name,
                PreviousValue::MustExistAndMatch(Target::Symbolic("refs/heads/main".try_into()?)),
                RefLog::Only,
            )
            .with_deref(true),
        ),
        Fail::Immediately,
        Fail::Immediately,
    );

    let err = res.expect_err("the expected target differs");
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[]), "reference deletion retains the expected and actual target when they differ", @r#"
    Could not prepare reference edit, reference="HEAD", referent="refs/heads/main"

    Caused by:
        0: Expected reference content ref: refs/heads/main
        1: The reference "refs/heads/main" changed to Oid(1)
    "#);
    assert!(err.is_conflict(), "the stale target must be refreshed and reconciled");
    assert!(
        !err.can_retry(),
        "retrying an unchanged expected target cannot resolve the conflict"
    );
    let stale = err
        .downcast_any_ref::<gix_ref::file::transaction::prepare::ReferenceOutOfDate>()
        .expect("stale reference recovery signal");
    assert_eq!(stale.full_name, "refs/heads/main");
    assert_eq!(
        stale.actual,
        Target::Object(hex_to_id("02a7a22d90d7c02fb494ed25551850b868e634f0"))
    );
    // everything stays as is
    let head = store.find_loose("HEAD")?;
    assert!(head.log_exists(&store)?);
    let main = store.find_loose("main")?;
    assert!(main.log_exists(&store)?);
    Ok(())
}

#[test]
fn delete_reflog_only_of_symbolic_no_deref() -> TestResult {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.find_loose("HEAD")?;
    assert!(head.log_exists(&store)?);

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit::delete_with_log(
                head.name,
                PreviousValue::MustExistAndMatch(Target::Symbolic("refs/heads/main".try_into()?)),
                RefLog::Only,
            )),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert_eq!(edits.len(), 1);
    let head: Reference = store.find_loose("HEAD")?.into();
    assert!(!head.log_exists(&store)?);
    let main = store.find_loose("main")?;
    assert!(main.log_exists(&store)?, "log is untouched, too");
    assert_eq!(
        main.target,
        head.follow(&store).expect("a symref")?.target,
        "head points to main"
    );
    Ok(())
}

#[test]
fn delete_reflog_only_of_symbolic_with_deref() -> TestResult {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let head = store.find_loose("HEAD")?;
    assert!(head.log_exists(&store)?);

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit::delete_with_log(head.name, PreviousValue::MustExist, RefLog::Only).with_deref(true)),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert_eq!(edits.len(), 2);
    let head: Reference = store.find_loose("HEAD")?.into();
    assert!(!head.log_exists(&store)?);
    let main = store.find_loose("main")?;
    assert!(!main.log_exists(&store)?, "log is removed");
    assert_eq!(
        main.target,
        head.follow(&store).expect("a symref")?.target,
        "head points to main"
    );
    Ok(())
}

#[test]
fn replacing_a_ref_with_its_child_in_one_transaction_is_rejected() -> TestResult {
    let (_keep, store) = store_writable("make_repo_for_reflog.sh")?;
    let old = store.find_loose("old")?;

    let new_name: FullName = "refs/heads/old/new".try_into()?;
    let err = store
        .transaction()
        .prepare(
            [
                RefEdit::delete(old.name.clone(), PreviousValue::MustExist).with_deref(true),
                RefEdit::update(new_name.clone(), old.target.clone(), PreviousValue::MustNotExist, "").with_deref(true),
            ],
            Fail::Immediately,
            Fail::Immediately,
        )
        .expect_err("transactions reject directory/file name replacement");
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&(err), &[]), "transactions reject namespace conflicts before publication", @r#"
    Could not prepare reference edit, reference="refs/heads/old/new"

    Caused by:
        0: The reference name conflicts with another reference, reference="refs/heads/old"
    "#);

    assert!(
        err.is_validation(),
        "reject conflicting reference names independently of filesystem layout"
    );

    let edits = store
        .transaction()
        .prepare(
            [RefEdit::delete(old.name, PreviousValue::MustExist).with_deref(true)],
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;
    assert_eq!(edits.len(), 1, "First delete to make space");

    let edits = store
        .transaction()
        .prepare(
            [RefEdit::update(new_name.clone(), old.target, PreviousValue::MustNotExist, "").with_deref(true)],
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
fn delete_broken_ref_that_must_exist_fails_as_it_is_no_valid_ref() -> TestResult {
    let (_keep, store) = empty_store()?;
    std::fs::write(store.git_dir().join("HEAD"), b"broken")?;
    assert!(store.try_find_loose("HEAD").is_err(), "the ref is truly broken");

    let res = store.transaction().prepare(
        Some(RefEdit::delete("HEAD".try_into()?, PreviousValue::MustExist).with_deref(true)),
        Fail::Immediately,
        Fail::Immediately,
    );
    let err = res.expect_err("a valid existing reference is required");
    insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&err, &[(&store.git_dir().to_string_lossy(), "<git-dir>")]), "delete broken ref that must exist fails as it is no valid ref", @r#"
    Could not prepare reference edit, reference="HEAD"

    Caused by:
        0: The reference to delete must exist
    "#);
    assert!(
        err.is_not_found(),
        "delete broken ref that must exist fails as it is no valid ref"
    );
    Ok(())
}

#[test]
fn non_existing_can_be_deleted_with_the_may_exist_match_constraint() -> TestResult {
    let (_keep, store) = empty_store()?;
    let previous_value =
        PreviousValue::ExistingMustMatch(Target::Object(hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03")));
    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit::delete("refs/heads/not-there".try_into()?, previous_value.clone()).with_deref(true)),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert_eq!(
        edits,
        vec![RefEdit::delete("refs/heads/not-there".try_into()?, previous_value,)]
    );
    Ok(())
}

#[test]
/// Based on https://github.com/git/git/blob/master/refs/files-backend.c#L514:L515
fn delete_broken_ref_that_may_not_exist_works_even_in_deref_mode() -> TestResult {
    let (_keep, store) = empty_store()?;
    std::fs::write(store.git_dir().join("HEAD"), b"broken")?;
    assert!(store.try_find_loose("HEAD").is_err(), "the ref is truly broken");

    let edits = store
        .transaction()
        .prepare(
            Some(RefEdit::delete("HEAD".try_into()?, PreviousValue::Any).with_deref(true)),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut TimeBuf::default()))?;

    assert!(store.try_find_loose("HEAD")?.is_none(), "the ref was deleted");
    assert_eq!(edits, vec![RefEdit::delete("HEAD".try_into()?, PreviousValue::Any,)]);
    Ok(())
}

#[test]
fn store_write_mode_has_no_effect_and_reflogs_are_always_deleted() -> TestResult {
    for reflog_writemode in &[
        gix_ref::store::WriteReflog::Normal,
        gix_ref::store::WriteReflog::Disable,
    ] {
        let (_keep, mut store) = store_writable("make_repo_for_reflog.sh")?;
        store.write_reflog = *reflog_writemode;
        assert!(store.find_loose("HEAD")?.log_exists(&store)?);
        assert!(store.open_packed_buffer()?.is_none(), "there is no pack");

        let edits = store
            .transaction()
            .prepare(
                Some(RefEdit::delete_with_log(
                    "HEAD".try_into()?,
                    PreviousValue::Any,
                    RefLog::Only,
                )),
                Fail::Immediately,
                Fail::Immediately,
            )?
            .commit(committer().to_ref(&mut TimeBuf::default()))?;
        assert_eq!(edits.len(), 1);
        assert!(!store.find_loose("HEAD")?.log_exists(&store)?, "log was deleted");
        assert!(store.open_packed_buffer()?.is_none(), "there still is no pack");
    }
    Ok(())
}

#[test]
fn packed_refs_are_consulted_when_determining_previous_value_of_ref_to_be_deleted_and_are_deleted_from_packed_ref_file()
-> TestResult {
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
            Some(RefEdit::delete(
                "refs/heads/main".try_into()?,
                PreviousValue::MustExistAndMatch(Target::Object(old_id)),
            )),
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
fn a_loose_ref_with_old_value_check_and_outdated_packed_refs_value_deletes_both_refs() -> TestResult {
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
            Some(RefEdit::delete(
                branch.name,
                PreviousValue::MustExistAndMatch(Target::Object(branch_id)),
            )),
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
fn all_contained_references_deletes_the_packed_ref_file_too() -> TestResult {
    for mode in ["must-exist", "may-exist"] {
        let (_keep, store) = store_writable("make_packed_ref_repository.sh")?;
        let edits = store
            .transaction()
            .prepare(
                store.open_packed_buffer()?.expect("packed-refs").iter()?.map(|r| {
                    let r = r.expect("valid ref");
                    RefEdit::delete(
                        r.name.into(),
                        match mode {
                            "must-exist" => PreviousValue::MustExistAndMatch(Target::Object(r.target())),
                            "may-exist" => PreviousValue::ExistingMustMatch(Target::Object(r.target())),
                            _ => unimplemented!("unknown mode: {}", mode),
                        },
                    )
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
