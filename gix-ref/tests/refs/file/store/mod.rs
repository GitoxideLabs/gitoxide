use gix_date::parse::TimeBuf;
use gix_lock::acquire::Fail;
use gix_ref::{
    Target,
    file::transaction::PackedRefs,
    store::WriteReflog,
    transaction::{Change, LogChange, PreviousValue, RefEdit},
};

use crate::file::{
    EmptyCommit,
    transaction::prepare_and_commit::{committer, create_at},
};

mod access;
mod find;
mod iter;
mod reflog;

#[test]
fn precompose_unicode_journey() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let precomposed_a = "ä";
    let decomposed_a = "a\u{308}";
    let root = tmp.path().join(decomposed_a);
    std::fs::create_dir(&root)?;

    let store_decomposed = gix_ref::file::Store::at(
        root,
        gix_ref::store::init::Options {
            write_reflog: WriteReflog::Always,
            object_hash: crate::fixture_hash_kind(),
            precompose_unicode: false,
            ..Default::default()
        },
    );
    assert!(!store_decomposed.precompose_unicode);

    let decomposed_ref = format!("refs/heads/{decomposed_a}");
    let mut buf = TimeBuf::default();
    store_decomposed
        .transaction()
        .prepare(Some(create_at(&decomposed_ref)), Fail::Immediately, Fail::Immediately)?
        .commit(committer().to_ref(&mut buf))?;

    let r = store_decomposed.iter()?.all()?.next().expect("created one ref")?;
    assert_eq!(r.name.as_bstr(), decomposed_ref, "no transformation happens by default");

    // For some reason, `tmpfs` can qualify as decomposing, but it then doesn't really work the same as expected here
    // to the point where the precomposed iteration can't find a single reference.
    let fs_folds_decomposed = gix_fs::Capabilities::probe(tmp.path()).precompose_unicode;
    if !fs_folds_decomposed || !cfg!(target_vendor = "apple") {
        // For file-access to work, we need a filesystem for which precomposed == decomposed.
        return Ok(());
    }

    let store_precomposed = gix_ref::file::Store::at(
        tmp.path().join(precomposed_a), // it's important that root paths are also precomposed then.
        gix_ref::store::init::Options {
            write_reflog: WriteReflog::Always,
            object_hash: crate::fixture_hash_kind(),
            precompose_unicode: true,
            ..Default::default()
        },
    );

    let precomposed_ref = format!("refs/heads/{precomposed_a}");
    let r = store_precomposed.iter()?.all()?.next().expect("created one ref")?;
    assert_eq!(
        r.name.as_bstr(),
        precomposed_ref,
        "it transforms all refs it sees to precomposed format, in order to unify it with what we store in packed-refs (precomposed)"
    );

    assert_eq!(
        store_precomposed.find(precomposed_a)?.name.as_bstr(),
        precomposed_ref,
        "can find as precomposed, even though on disk is decomposed it is decomposed"
    );
    assert_eq!(
        store_precomposed.find(decomposed_a)?.name.as_bstr(),
        decomposed_ref,
        "can find as decomposed, and it keeps it as is to not violate expectations of the returned name being equal to the input (when comparing as bytes)"
    );

    let decomposed_u = "u\u{308}";
    let decomposed_ref = format!("refs/heads/{decomposed_u}");
    let edits = store_precomposed
        .transaction()
        .prepare(Some(create_at(&decomposed_ref)), Fail::Immediately, Fail::Immediately)?
        .commit(committer().to_ref(&mut buf))?;
    assert_eq!(
        edits[0].name.as_bstr(),
        decomposed_ref,
        "it doesn't alter the composition style to allow input and output to remain unchanged"
    );

    assert_eq!(
        store_decomposed
            .iter()?
            .all()?
            .nth(1)
            .expect("two refs")?
            .name
            .shorten(),
        decomposed_u,
        "the ref name isn't transformed in any way and left decomposed on disk as well, making sure internal loose/packed-ref interactions work reliably"
    );

    assert!(
        store_precomposed.cached_packed_buffer()?.is_none(),
        "no packed-refs yet"
    );
    let edits = store_precomposed
        .transaction()
        .packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(
            Box::new(EmptyCommit),
        ))
        .prepare(
            // Intentionally use the decomposed versions of their names
            store_decomposed
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
        .commit(committer().to_ref(&mut buf))?;
    assert!(
        store_precomposed.cached_packed_buffer()?.is_some(),
        "refs were written into the packed-refs file"
    );

    assert_eq!(store_precomposed.loose_iter()?.count(), 0, "all loose refs are gone");
    assert_eq!(edits.len(), 2);
    assert_eq!(
        edits[0].name.shorten(),
        decomposed_a,
        "composition stays the same for consistency"
    );
    assert_eq!(
        edits[1].name.shorten(),
        decomposed_u,
        "composition stays the same for consistency"
    );

    assert_eq!(
        store_decomposed.find(precomposed_a)?.name.shorten(),
        precomposed_a,
        "the decomposed store can only find what's in packed-refs verbatim"
    );
    assert!(
        store_decomposed.try_find(decomposed_a)?.is_none(),
        "decomposed inputs don't match in packed-refs"
    );
    assert_eq!(
        store_precomposed.find(precomposed_a)?.name.shorten(),
        precomposed_a,
        "we find what's in the packed-refs file, which is native to packed-refs, just to keep lookups simple as otherwise\
         we would have to search lots of variants for decomposed and precomposed versions of the same item (but would this be a problem as it's rare?)"
    );
    assert_eq!(
        store_precomposed.find(decomposed_a)?.name.shorten(),
        decomposed_a,
        "despite the input being decomposed, we find the ref (in packed-refs) as precomposed, but return it just like we inserted it"
    );
    assert!(
        store_decomposed.reflog_exists(decomposed_ref.as_str())?,
        "these are stored as files, which always works on a filesystem that supports it"
    );
    assert!(store_precomposed.reflog_exists(decomposed_ref.as_str())?);

    let edits = store_precomposed
        .transaction()
        .prepare(
            // A symref pointing to a decomposed name.
            Some(RefEdit {
                change: Change::Update {
                    log: LogChange::default(),
                    expected: PreviousValue::MustNotExist,
                    new: Target::Symbolic(decomposed_ref.clone().try_into().expect("valid name")),
                },
                name: "HEAD".try_into().expect("valid name"),
                deref: false,
            }),
            Fail::Immediately,
            Fail::Immediately,
        )?
        .commit(committer().to_ref(&mut buf))?;
    assert_eq!(
        edits[0].change.new_value().unwrap().try_name().unwrap().as_bstr(),
        decomposed_ref,
        "the composition doesn't change, it matches the original edit"
    );
    assert_eq!(
        store_decomposed
            .find("HEAD")
            .expect("exists")
            .target
            .try_name()
            .unwrap()
            .as_bstr(),
        decomposed_ref,
        "on disk it's stored in the original format as well"
    );

    let precomposed_u = "ü";
    for namespace in [decomposed_u, precomposed_u] {
        let mut store_precomposed_with_namespace = store_precomposed.clone();
        store_precomposed_with_namespace.namespace = Some(gix_ref::namespace::expand(namespace)?.clone());

        // these edits are loose refs
        let edits = store_precomposed_with_namespace
            .transaction()
            .prepare(
                [create_at(&format!("refs/heads/{decomposed_a}"))],
                Fail::Immediately,
                Fail::Immediately,
            )?
            .commit(committer().to_ref(&mut buf))?;
        assert_eq!(
            edits[0].name.shorten(),
            decomposed_a,
            "composition stays the same for consistency"
        );

        let reference = store_precomposed_with_namespace.find(decomposed_a)?;
        assert_eq!(
            reference.name.shorten(),
            decomposed_a,
            "on disk we find anything as precomposition is transparent"
        );
        assert!(store_precomposed_with_namespace.reflog_exists(reference.name.as_ref())?);

        let reference = store_precomposed_with_namespace.find(precomposed_a)?;
        assert_eq!(
            reference.name.shorten(),
            precomposed_a,
            "on disk we find anything as precomposition is transparent"
        );

        // and these go straight to packed-refs
        let edits = store_precomposed_with_namespace
            .transaction()
            .packed_refs(PackedRefs::DeletionsAndNonSymbolicUpdatesRemoveLooseSourceReference(
                Box::new(EmptyCommit),
            ))
            .prepare(
                [create_at(&format!("refs/heads/{decomposed_u}"))],
                Fail::Immediately,
                Fail::Immediately,
            )?
            .commit(committer().to_ref(&mut buf))?;
        assert_eq!(
            edits[0].name.shorten(),
            decomposed_u,
            "composition stays the same for consistency"
        );

        let reference = store_precomposed_with_namespace.find(decomposed_u)?;
        assert_eq!(
            reference.name.shorten(),
            decomposed_u,
            "despite the input being decomposed, we find the ref (in packed-refs) as precomposed, but return it just like we inserted it"
        );
        assert!(store_precomposed_with_namespace.reflog_exists(reference.name.as_ref())?);

        let reference = store_precomposed_with_namespace.find(precomposed_u)?;
        assert_eq!(
            reference.name.shorten(),
            precomposed_u,
            "despite decomposed namespace, we find the precomposed version as well, even though it's packed"
        );
        assert!(store_precomposed_with_namespace.reflog_exists(reference.name.as_ref())?);
    }

    Ok(())
}
