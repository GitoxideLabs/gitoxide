fn store_with_packed_refs() -> crate::Result<gix_ref::Store> {
    let root = crate::scripted_fixture_read_only("make_packed_ref_repository.sh")?;
    Ok(gix_ref::Store::at(root.join(".git"), crate::fixture_hash_kind()))
}

fn edit(
    name: &str,
    expected: gix_ref::transaction::PreviousValue,
    new: gix_ref::Target,
    deref: bool,
) -> gix_ref::transaction::RefEdit {
    gix_ref::transaction::RefEdit {
        change: gix_ref::transaction::Change::Update {
            log: gix_ref::transaction::LogChange {
                mode: gix_ref::transaction::RefLog::AndReference,
                force_create_reflog: true,
                message: format!("update {name}").into(),
            },
            expected,
            new,
        },
        name: name.try_into().expect("contract reference names are valid"),
        deref,
    }
}

fn delete(name: &str) -> gix_ref::transaction::RefEdit {
    gix_ref::transaction::RefEdit {
        change: gix_ref::transaction::Change::Delete {
            expected: gix_ref::transaction::PreviousValue::MustExist,
            log: gix_ref::transaction::RefLog::AndReference,
        },
        name: name.try_into().expect("contract reference names are valid"),
        deref: false,
    }
}

fn commit(
    store: &gix_ref::Store,
    edits: impl IntoIterator<Item = gix_ref::transaction::RefEdit>,
) -> crate::Result<Vec<gix_ref::transaction::RefEdit>> {
    let mut signature_buf = gix_date::parse::TimeBuf::default();
    Ok(store
        .transaction()
        .prepare(
            edits,
            gix_lock::acquire::Fail::Immediately,
            gix_lock::acquire::Fail::Immediately,
        )?
        .commit(crate::file::transaction::prepare_and_commit::committer().to_ref(&mut signature_buf))?)
}

fn exercise_backend_contract(mut store: gix_ref::Store) -> crate::Result {
    let first_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let second_id = crate::hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    let absent_id = crate::hex_to_id("4c3f4cce493d7beb45012e478021b5f65295e5a3");
    let main_name = "refs/heads/main";
    let main_full_name: &gix_ref::FullNameRef = main_name.try_into()?;
    let alias_name = "refs/heads/alias";

    commit(
        &store,
        [
            edit(
                main_name,
                gix_ref::transaction::PreviousValue::MustNotExist,
                gix_ref::Target::Object(first_id),
                false,
            ),
            edit(
                "refs/heads/topic",
                gix_ref::transaction::PreviousValue::MustNotExist,
                gix_ref::Target::Object(first_id),
                false,
            ),
            edit(
                alias_name,
                gix_ref::transaction::PreviousValue::MustNotExist,
                gix_ref::Target::Symbolic(main_name.try_into()?),
                false,
            ),
            edit(
                "HEAD",
                gix_ref::transaction::PreviousValue::MustNotExist,
                gix_ref::Target::Symbolic(main_name.try_into()?),
                false,
            ),
        ],
    )?;

    assert_eq!(
        store.find("main")?.target.try_id(),
        Some(first_id.as_ref()),
        "partial direct-reference lookup returns the stored object ID"
    );
    assert_eq!(
        store.find(alias_name)?.target.try_name(),
        Some(main_full_name),
        "full-name lookup preserves a symbolic reference target"
    );
    assert!(
        store.try_find("refs/heads/missing")?.is_none(),
        "optional lookup reports a missing reference as absent"
    );

    let prefix: &gix_path::RelativePath = b"refs/heads/".try_into()?;
    let names = store
        .iter()?
        .prefixed(prefix)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|reference| reference.name)
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [alias_name, main_name, "refs/heads/topic"],
        "prefix iteration is complete and sorted"
    );
    let pseudo = store
        .iter()?
        .pseudo()?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|reference| reference.name)
        .collect::<Vec<_>>();
    assert!(
        pseudo.iter().any(|name| name == "HEAD"),
        "pseudo-ref iteration includes HEAD"
    );

    let stale_edit = edit(
        main_name,
        gix_ref::transaction::PreviousValue::MustExistAndMatch(gix_ref::Target::Object(absent_id)),
        gix_ref::Target::Object(second_id),
        false,
    );
    assert!(
        store
            .transaction()
            .prepare(
                [stale_edit],
                gix_lock::acquire::Fail::Immediately,
                gix_lock::acquire::Fail::Immediately,
            )
            .is_err(),
        "compare-and-swap rejects an outdated expected value"
    );
    assert_eq!(
        store.find(main_name)?.target.try_id(),
        Some(first_id.as_ref()),
        "a rejected compare-and-swap leaves the reference unchanged"
    );

    commit(
        &store,
        [edit(
            main_name,
            gix_ref::transaction::PreviousValue::MustExistAndMatch(gix_ref::Target::Object(first_id)),
            gix_ref::Target::Object(second_id),
            false,
        )],
    )?;
    assert_eq!(
        store.find(main_name)?.target.try_id(),
        Some(second_id.as_ref()),
        "a matching compare-and-swap publishes the new object ID"
    );

    commit(
        &store,
        [edit(
            alias_name,
            gix_ref::transaction::PreviousValue::MustExistAndMatch(gix_ref::Target::Object(second_id)),
            gix_ref::Target::Object(first_id),
            true,
        )],
    )?;
    assert_eq!(
        store.find(main_name)?.target.try_id(),
        Some(first_id.as_ref()),
        "a dereferencing edit updates the symbolic reference's referent"
    );
    assert_eq!(
        store.find(alias_name)?.target.try_name(),
        Some(main_full_name),
        "a dereferencing edit leaves the symbolic reference intact"
    );

    let mut logs = store.reflog_iter(main_name)?;
    let forward = logs
        .all()?
        .expect("forced reflog creation makes the log available")
        .collect::<Result<Vec<_>, _>>()?;
    let reverse = logs
        .rev()?
        .expect("the same reflog is available in reverse")
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        forward.iter().map(|line| line.new_oid).rev().collect::<Vec<_>>(),
        reverse.iter().map(|line| line.new_oid).collect::<Vec<_>>(),
        "forward and reverse reflog traversal describe the same history"
    );

    commit(&store, [delete(alias_name)])?;
    assert!(
        store.try_find(alias_name)?.is_none(),
        "symbolic references can be deleted"
    );
    assert!(
        store.try_find(main_name)?.is_some(),
        "deleting a symbolic ref does not delete its target"
    );

    let namespace = gix_ref::namespace::expand("tenant")?.to_owned();
    store.replace_namespace(Some(namespace));
    commit(
        &store,
        [edit(
            "refs/heads/scoped",
            gix_ref::transaction::PreviousValue::MustNotExist,
            gix_ref::Target::Object(first_id),
            false,
        )],
    )?;
    assert!(
        store.try_find("scoped")?.is_some(),
        "namespaced writes are visible in the namespace"
    );
    let mut unnamespaced = store.clone();
    unnamespaced.replace_namespace(None);
    assert!(
        unnamespaced.try_find("scoped")?.is_none(),
        "namespaced writes do not leak into the unnamespaced view"
    );
    store.verify()?;
    store.optimize(
        gix_ref::store::maintenance::Options::default(),
        gix_lock::acquire::Fail::Immediately,
    )?;
    store.force_refresh()?;
    Ok(())
}

#[test]
fn files_adapter_satisfies_the_backend_contract() -> crate::Result {
    let fixture = gix_testtools::tempfile::TempDir::new()?;
    exercise_backend_contract(gix_ref::Store::at(
        fixture.path().to_owned(),
        crate::fixture_hash_kind(),
    ))
}

#[test]
fn files_adapter_rejects_unsupported_reflog_expiry() -> crate::Result {
    let fixture = gix_testtools::tempfile::TempDir::new()?;
    let store = gix_ref::Store::at(fixture.path().to_owned(), crate::fixture_hash_kind());
    let err = store
        .optimize(
            gix_ref::store::maintenance::Options {
                expire_reflogs_before: Some(u64::MAX),
                ..Default::default()
            },
            gix_lock::acquire::Fail::Immediately,
        )
        .expect_err("files-backed reflog expiry must not be silently ignored");
    assert_eq!(
        err.operation(),
        "expire reference logs",
        "the backend-neutral error identifies the unsupported operation"
    );
    assert_eq!(
        err.to_string(),
        "Could not expire reference logs",
        "display renders the failed operation without duplicating the adapter-specific source"
    );
    let source = std::error::Error::source(&err).expect("backend errors retain their adapter-specific source");
    assert!(
        source.to_string().contains("not supported by the files backend"),
        "the unsupported backend capability remains visible: {source}"
    );
    Ok(())
}

#[test]
fn files_adapter_force_refreshes_packed_refs_through_the_opaque_store() -> crate::Result {
    let fixture = gix_testtools::tempfile::TempDir::new()?;
    let git_dir = fixture.path();
    let packed_refs = git_dir.join("packed-refs");
    let first_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let second_id = crate::hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    let write_packed_ref = |object_id| {
        std::fs::write(
            &packed_refs,
            format!("# pack-refs with: peeled fully-peeled sorted\n{object_id} refs/heads/main\n"),
        )
    };

    write_packed_ref(first_id)?;
    let store = gix_ref::Store::at(git_dir.to_owned(), crate::fixture_hash_kind());
    assert_eq!(
        store.find("main")?.target.try_id(),
        Some(first_id.as_ref()),
        "the initial lookup caches the first packed reference"
    );

    write_packed_ref(second_id)?;
    store.force_refresh()?;
    assert_eq!(
        store.find("main")?.target.try_id(),
        Some(second_id.as_ref()),
        "an explicit refresh observes a replacement packed reference"
    );

    std::fs::remove_file(packed_refs)?;
    store.force_refresh()?;
    assert!(
        store.try_find("main")?.is_none(),
        "an explicit refresh clears the cache after packed-refs is removed"
    );
    Ok(())
}

#[test]
fn files_adapter_supports_lookup_through_the_opaque_store() -> crate::Result {
    let store = store_with_packed_refs()?;

    let main = store.find("main")?;
    assert_eq!(
        main.name, "refs/heads/main",
        "partial-name lookup uses Git's precedence rules"
    );
    assert!(
        store.try_find("refs/heads/does-not-exist")?.is_none(),
        "missing references are represented by None"
    );
    Ok(())
}

#[test]
fn files_adapter_supports_sorted_iteration_through_the_opaque_store() -> crate::Result {
    let store = store_with_packed_refs()?;

    let mut names = Vec::new();
    let mut errors = 0;
    for reference in store.iter()?.all()? {
        match reference {
            Ok(reference) => names.push(reference.name),
            Err(_) => errors += 1,
        }
    }
    assert!(
        names.windows(2).all(|pair| pair[0] < pair[1]),
        "the backend contract yields references in strictly sorted order"
    );
    assert!(
        names.iter().any(|name| name == "refs/heads/main"),
        "iteration includes loose or packed local branches"
    );
    assert!(errors > 0, "malformed references remain visible as iterator errors");
    Ok(())
}

#[test]
fn files_adapter_commits_edits_through_the_opaque_store() -> crate::Result {
    let fixture = crate::scripted_fixture_writable("make_ref_repository.sh")?;
    let store = gix_ref::Store::at(fixture.path().join(".git"), crate::fixture_hash_kind());
    let edit = crate::file::transaction::prepare_and_commit::create_at("refs/heads/through-store");
    let mut signature_buf = gix_date::parse::TimeBuf::default();

    let edits = store
        .transaction()
        .prepare(
            [edit],
            gix_lock::acquire::Fail::Immediately,
            gix_lock::acquire::Fail::Immediately,
        )?
        .commit(crate::file::transaction::prepare_and_commit::committer().to_ref(&mut signature_buf))?;

    assert_eq!(edits.len(), 1, "one requested edit produces one committed edit");
    assert_eq!(
        store.find("through-store")?.name,
        "refs/heads/through-store",
        "the committed edit is immediately visible through the same store"
    );
    Ok(())
}

fn exercise_compact_strategy(remove_separate_source: bool) -> crate::Result {
    let fixture = crate::scripted_fixture_writable("make_ref_repository.sh")?;
    let git_dir = fixture.path().join(".git");
    let loose_path = git_dir.join("refs/heads/main");
    let store = gix_ref::Store::at(git_dir.clone(), crate::fixture_hash_kind());
    let target = store.find("main")?.target;
    let object_id = target.try_id().expect("the fixture's main branch is direct").to_owned();
    let objects = crate::file::odb_at(git_dir.join("objects"))?;
    let mut signature_buf = gix_date::parse::TimeBuf::default();

    store
        .transaction()
        .write_strategy(gix_ref::store::transaction::WriteStrategy::Compact {
            objects: Box::new(objects),
            remove_separate_source,
        })
        .prepare(
            [edit(
                "refs/heads/main",
                gix_ref::transaction::PreviousValue::MustExistAndMatch(target.clone()),
                target,
                false,
            )],
            gix_lock::acquire::Fail::Immediately,
            gix_lock::acquire::Fail::Immediately,
        )?
        .commit(crate::file::transaction::prepare_and_commit::committer().to_ref(&mut signature_buf))?;

    let packed = std::fs::read_to_string(git_dir.join("packed-refs"))?;
    assert!(
        packed
            .lines()
            .any(|line| line == format!("{object_id} refs/heads/main")),
        "the compact strategy publishes the direct reference in packed-refs"
    );
    assert_eq!(
        loose_path.exists(),
        !remove_separate_source,
        "the compact strategy applies its requested loose-source policy"
    );
    assert_eq!(
        store.find("main")?.target.try_id(),
        Some(object_id.as_ref()),
        "the logical reference remains visible after physical compaction"
    );
    Ok(())
}

#[test]
fn compact_strategy_can_retain_the_loose_source() -> crate::Result {
    exercise_compact_strategy(false)
}

#[test]
fn compact_strategy_can_remove_the_loose_source() -> crate::Result {
    exercise_compact_strategy(true)
}

#[test]
fn compact_strategy_preserves_the_loose_source_if_packed_refs_cannot_be_published() -> crate::Result {
    let fixture = crate::scripted_fixture_writable("make_ref_repository.sh")?;
    let git_dir = fixture.path().join(".git");
    let loose_path = git_dir.join("refs/heads/main");
    let store = gix_ref::Store::at(git_dir.clone(), crate::fixture_hash_kind());
    let target = store.find("main")?.target;
    let objects = crate::file::odb_at(git_dir.join("objects"))?;
    let transaction = store
        .transaction()
        .write_strategy(gix_ref::store::transaction::WriteStrategy::Compact {
            objects: Box::new(objects),
            remove_separate_source: true,
        })
        .prepare(
            [edit(
                "refs/heads/main",
                gix_ref::transaction::PreviousValue::MustExistAndMatch(target.clone()),
                target,
                false,
            )],
            gix_lock::acquire::Fail::Immediately,
            gix_lock::acquire::Fail::Immediately,
        )?;

    std::fs::create_dir(git_dir.join("packed-refs"))?;
    let mut signature_buf = gix_date::parse::TimeBuf::default();
    transaction
        .commit(crate::file::transaction::prepare_and_commit::committer().to_ref(&mut signature_buf))
        .expect_err("a directory at packed-refs prevents the compact representation from being published");
    assert!(
        loose_path.is_file(),
        "the loose source is retained when publishing packed-refs fails"
    );
    Ok(())
}

#[test]
fn store_configuration_and_locations_are_backend_neutral() -> crate::Result {
    let fixture = crate::scripted_fixture_read_only("make_ref_repository.sh")?;
    let git_dir = fixture.join(".git");
    let mut store = gix_ref::Store::at(git_dir.clone(), crate::fixture_hash_kind());

    assert_eq!(
        store.git_dir(),
        git_dir,
        "the adapter reports its worktree-local location"
    );
    assert!(
        store.common_dir().is_none(),
        "ordinary repositories have no separate common directory"
    );
    assert_eq!(
        store.write_reflog(),
        gix_ref::store::WriteReflog::Normal,
        "new stores use the normal reflog policy"
    );
    store.set_write_reflog(gix_ref::store::WriteReflog::Disable);
    assert_eq!(
        store.write_reflog(),
        gix_ref::store::WriteReflog::Disable,
        "the backend-neutral setter updates the reflog policy"
    );

    let namespace = gix_ref::namespace::expand("tenant")?.to_owned();
    assert!(
        store.replace_namespace(Some(namespace.clone())).is_none(),
        "installing the first namespace returns no previous namespace"
    );
    assert_eq!(
        store.namespace(),
        Some(&namespace),
        "the configured namespace is observable through the common store"
    );
    assert_eq!(
        store.replace_namespace(None),
        Some(namespace),
        "clearing a namespace returns the namespace that was installed"
    );
    Ok(())
}

#[test]
fn files_adapter_reads_reflogs_through_the_opaque_store() -> crate::Result {
    let fixture = crate::scripted_fixture_read_only("make_repo_for_reflog.sh")?;
    let store = gix_ref::Store::at(fixture.join(".git"), crate::fixture_hash_kind());

    assert!(store.reflog_exists("HEAD")?, "the fixture contains a HEAD reflog");
    let mut platform = store.reflog_iter("HEAD")?;
    let lines = platform
        .all()?
        .expect("the reflog exists")
        .collect::<Result<Vec<_>, _>>()?;
    assert!(
        !lines.is_empty(),
        "all reflog entries are returned through the common iterator"
    );
    Ok(())
}

#[test]
fn symbolic_references_follow_through_a_backend_neutral_snapshot() -> crate::Result {
    use gix_ref::store::ReferenceExt as _;

    let store = store_with_packed_refs()?;
    let expected_id = store
        .find("main")?
        .target
        .try_id()
        .expect("main is a direct reference")
        .to_owned();
    let mut head = store.find("HEAD")?;

    assert_eq!(
        head.follow_to_object(&store)?,
        expected_id,
        "symbolic reference traversal reaches the direct object ID"
    );
    assert_eq!(
        head.name, "refs/heads/main",
        "following updates the cursor to the direct referent"
    );
    Ok(())
}

#[test]
fn files_snapshot_reads_loose_references_live() -> crate::Result {
    let fixture = gix_testtools::tempfile::TempDir::new()?;
    let loose_path = fixture.path().join("refs/heads/main");
    std::fs::create_dir_all(loose_path.parent().expect("a reference path has a parent"))?;
    let first_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let second_id = crate::hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    std::fs::write(&loose_path, format!("{}\n", first_id.to_hex()))?;
    let store = gix_ref::Store::at(fixture.path().to_owned(), crate::fixture_hash_kind());
    let snapshot = store.snapshot()?;
    assert_eq!(
        snapshot
            .try_find("refs/heads/main")?
            .expect("the loose reference exists")
            .target
            .try_id(),
        Some(first_id.as_ref()),
        "the initial loose value is visible through the snapshot"
    );

    std::fs::write(&loose_path, format!("{}\n", second_id.to_hex()))?;
    assert_eq!(
        snapshot
            .try_find("refs/heads/main")?
            .expect("the loose reference still exists")
            .target
            .try_id(),
        Some(second_id.as_ref()),
        "loose references remain live while aggregate state is pinned"
    );
    Ok(())
}

#[test]
fn symbolic_cycle_reports_the_reference_repeated_by_the_cycle() -> crate::Result {
    use gix_ref::store::ReferenceExt as _;

    let fixture = gix_testtools::tempfile::TempDir::new()?;
    let refs_dir = fixture.path().join("refs");
    std::fs::create_dir_all(&refs_dir)?;
    std::fs::write(refs_dir.join("entry"), "ref: refs/cycle-a\n")?;
    std::fs::write(refs_dir.join("cycle-a"), "ref: refs/cycle-b\n")?;
    std::fs::write(refs_dir.join("cycle-b"), "ref: refs/cycle-a\n")?;
    let store = gix_ref::Store::at(fixture.path().to_owned(), crate::fixture_hash_kind());
    let mut reference = store.find("refs/entry")?;

    let err = reference
        .follow_to_object(&store)
        .expect_err("the symbolic reference chain contains a cycle");
    match err {
        gix_ref::store::peel::to_object::Error::Cycle { reference } => assert_eq!(
            reference, "refs/cycle-a",
            "the error identifies the repeated reference instead of the chain's entry point"
        ),
        other => panic!("expected a cycle error, got {other:?}"),
    }
    Ok(())
}

#[test]
fn annotated_tags_peel_through_the_backend_neutral_store() -> crate::Result {
    use gix_ref::store::ReferenceExt as _;

    let root = crate::scripted_fixture_read_only("make_packed_ref_repository.sh")?;
    let git_dir = root.join(".git");
    let store = gix_ref::Store::at(git_dir.clone(), crate::fixture_hash_kind());
    let objects = gix_odb::at(git_dir.join("objects"), crate::fixture_hash_kind())?;
    let expected_id = store.find("main")?.target.into_id();
    let mut annotated_tag = store.find("dt1")?;
    assert_ne!(
        annotated_tag.target.try_id(),
        Some(expected_id.as_ref()),
        "the fixture's annotated tag initially points at a tag object"
    );

    assert_eq!(
        annotated_tag.peel_to_id(&store, &objects)?,
        expected_id,
        "peeling follows the annotated tag to its final object ID"
    );
    assert_eq!(
        annotated_tag.target.try_id(),
        Some(expected_id.as_ref()),
        "peeling updates the reference cursor to the peeled object ID"
    );
    Ok(())
}

#[test]
fn linked_worktree_routing_is_available_through_the_opaque_store() -> crate::Result {
    use gix_ref::store::ReferenceExt as _;

    let root = crate::scripted_fixture_read_only_with_args("make_worktree_repo.sh", ["packed"])?;
    let (git_dir, _worktree) = gix_discover::upwards(&root.join("w1"))?
        .0
        .into_repository_and_work_tree_directories();
    let common_dir = git_dir.join("../..");
    let store = gix_ref::Store::for_linked_worktree(git_dir.clone(), common_dir.clone(), crate::fixture_hash_kind());
    let mut current_head = store.find("HEAD")?;
    let mut main_head = store.find("main-worktree/HEAD")?;

    assert_eq!(
        store.git_dir(),
        git_dir,
        "a linked-worktree store retains its private Git directory"
    );
    assert_eq!(
        store.common_dir(),
        Some(common_dir.as_path()),
        "a linked-worktree store retains its shared common directory"
    );
    assert_ne!(
        current_head.follow_to_object(&store)?,
        main_head.follow_to_object(&store)?,
        "current and main worktree HEAD route to their distinct private refs"
    );
    assert!(
        store.try_find("worktrees/w-detached/refs/bisect/bad")?.is_some(),
        "explicit other-worktree lookups route through the common store"
    );
    Ok(())
}

#[test]
#[cfg(feature = "parallel")]
fn is_send_and_sync() {
    fn assert_type<T: Send + Sync>(_t: T) {}
    let store = store_with_packed_refs().expect("fixture-backed store can be created");
    assert_type(&store);
    assert_type(store);
}
