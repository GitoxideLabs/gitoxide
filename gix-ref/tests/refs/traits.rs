use std::{rc::Rc, sync::Arc};

use gix_lock::acquire::Fail;
use gix_ref::{
    transaction::{Change, LogChange, PreviousValue, RefEdit},
    FullNameRef, PartialNameRef, StoreMutate, StoreRead, StoreReadExt, Target,
};

#[test]
fn try_find_success_and_miss() -> crate::Result {
    let store = crate::file::store_with_packed_refs()?;

    let present: &PartialNameRef = "main".try_into()?;
    assert!(StoreRead::try_find(&store, present).expect("lookup succeeds").is_some());

    let missing: &PartialNameRef = "this-definitely-does-not-exist".try_into()?;
    assert!(StoreRead::try_find(&store, missing).expect("lookup succeeds").is_none());
    Ok(())
}

#[test]
fn find_miss_returns_exn() -> crate::Result {
    let store = crate::file::store_with_packed_refs()?;
    let missing: &PartialNameRef = "this-definitely-does-not-exist".try_into()?;

    let err = StoreReadExt::find(&store, missing).expect_err("must report not-found");
    assert!(
        err.to_string().contains("could not be found"),
        "the missing-reference condition is reported through Exn"
    );
    Ok(())
}

#[test]
fn iter_all_works_via_trait() -> crate::Result {
    let store = crate::file::store_with_packed_refs()?;
    let platform = StoreRead::iter(&store).expect("iterator platform can be created");
    assert!(
        platform.all()?.next().is_some(),
        "fixture has at least one reference in iteration"
    );
    Ok(())
}

#[test]
fn reflog_apis_work_via_trait() -> crate::Result {
    let store = crate::file::store_at("make_repo_for_reflog.sh")?;
    let head: &FullNameRef = "HEAD".try_into()?;
    let missing: &FullNameRef = "refs/heads/does-not-exist".try_into()?;

    assert!(StoreRead::reflog_exists(&store, head).expect("lookup succeeds"));
    assert!(!StoreRead::reflog_exists(&store, missing).expect("lookup succeeds"));

    let mut buf = Vec::new();
    assert!(
        StoreRead::reflog_iter(&store, head, &mut buf)
            .expect("iteration works")
            .is_some(),
        "HEAD has a reflog in the fixture"
    );
    assert!(
        StoreRead::reflog_iter(&store, missing, &mut buf)
            .expect("iteration works")
            .is_none(),
        "missing refs have no reflog"
    );

    let mut reverse_buf = [0u8; 512];
    assert!(
        StoreRead::reflog_iter_rev(&store, head, &mut reverse_buf)
            .expect("reverse iteration works")
            .is_some(),
        "HEAD has a reverse reflog iterator"
    );
    assert!(
        StoreRead::reflog_iter_rev(&store, missing, &mut reverse_buf)
            .expect("reverse iteration works")
            .is_none(),
        "missing refs have no reflog"
    );
    Ok(())
}

#[test]
fn transaction_via_trait_is_usable() -> crate::Result {
    let dir = gix_testtools::scripted_fixture_writable_standalone("make_repo_for_reflog.sh")?;
    let store = gix_ref::file::Store::at(
        dir.path().join(".git"),
        gix_ref::store::init::Options {
            write_reflog: gix_ref::store::WriteReflog::Disable,
            ..Default::default()
        },
    );

    let expected = crate::hex_to_id("28ce6a8b26aa170e1de65536fe8abe1832bd3242");
    StoreMutate::transaction(&store)
        .expect("transaction opens")
        .prepare(
            [RefEdit {
                change: Change::Update {
                    log: LogChange::default(),
                    expected: PreviousValue::MustNotExist,
                    new: Target::Object(expected),
                },
                name: "refs/heads/trait-created".try_into()?,
                deref: false,
            }],
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(None)?;

    let created: &PartialNameRef = "refs/heads/trait-created".try_into()?;
    let reference = StoreReadExt::find(&store, created).expect("newly created reference is readable");
    assert_eq!(reference.target.id(), expected.as_ref());
    Ok(())
}

#[test]
fn blanket_impls_compile_for_shared_owners() -> crate::Result {
    fn use_read_api<T: StoreRead + ?Sized>(store: &T) {
        let partial: &PartialNameRef = "main".try_into().expect("valid");
        let _ = StoreRead::try_find(store, partial).expect("read works");
        let _ = StoreRead::iter(store).expect("iter works");
    }

    fn use_mutate_api<T: StoreMutate + ?Sized>(store: &T) {
        let _ = StoreMutate::transaction(store).expect("transaction can be obtained");
    }

    let store = crate::file::store_with_packed_refs()?;
    use_read_api(&store);
    use_read_api(&Rc::new(store.clone()));
    use_read_api(&Arc::new(store.clone()));
    use_read_api(&Box::new(store.clone()));

    use_mutate_api(&store);
    use_mutate_api(&Rc::new(store.clone()));
    use_mutate_api(&Arc::new(store.clone()));
    use_mutate_api(&Box::new(store));
    Ok(())
}
