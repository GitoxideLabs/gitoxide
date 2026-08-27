use std::{
    error::Error,
    ffi::OsStr,
    io::Write,
    path::Path,
    process::{Output, Stdio},
};

use gix_ref::{
    Target,
    store::{ReferenceExt as _, transaction::WriteStrategy},
    transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog},
};

type TestResult<T = ()> = Result<T, Box<dyn Error + Send + Sync>>;

fn update(name: &str, expected: PreviousValue, new: Target) -> RefEdit {
    RefEdit {
        change: Change::Update {
            log: LogChange {
                mode: RefLog::AndReference,
                force_create_reflog: true,
                message: format!("update {name}").into(),
            },
            expected,
            new,
        },
        name: name.try_into().expect("test names are valid"),
        deref: false,
    }
}

fn delete(name: &str, expected: PreviousValue, log: RefLog) -> RefEdit {
    RefEdit {
        change: Change::Delete { expected, log },
        name: name.try_into().expect("test names are valid"),
        deref: false,
    }
}

fn commit(store: &gix_ref::Store, edits: impl IntoIterator<Item = RefEdit>) -> TestResult<Vec<RefEdit>> {
    let mut time = gix_actor::date::parse::TimeBuf::default();
    Ok(store
        .transaction()
        .prepare(
            edits,
            gix_lock::acquire::Fail::Immediately,
            gix_lock::acquire::Fail::Immediately,
        )?
        .commit(crate::file::transaction::prepare_and_commit::committer().to_ref(&mut time))?)
}

fn create_stack(path: &Path) -> TestResult<gix_reftable::Stack> {
    Ok(gix_reftable::Stack::create(
        path,
        crate::fixture_hash_kind(),
        Default::default(),
        Default::default(),
    )?)
}

fn empty_store() -> TestResult<(gix_testtools::tempfile::TempDir, gix_ref::Store)> {
    let temp = gix_testtools::tempfile::TempDir::new()?;
    create_stack(&temp.path().join("reftable"))?;
    let store = gix_ref::Store::open_reftable(temp.path().to_owned(), crate::fixture_hash_kind())?;
    Ok((temp, store))
}

#[test]
fn atomically_replaces_a_parent_reference_with_its_child() -> TestResult {
    let (_temp, store) = empty_store()?;
    let commit_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    commit(
        &store,
        [update(
            "refs/heads/topic",
            PreviousValue::MustNotExist,
            Target::Object(commit_id),
        )],
    )?;

    commit(
        &store,
        [
            delete("refs/heads/topic", PreviousValue::MustExist, RefLog::AndReference),
            update(
                "refs/heads/topic/child",
                PreviousValue::MustNotExist,
                Target::Object(commit_id),
            ),
        ],
    )?;
    assert!(
        store.try_find("refs/heads/topic")?.is_none(),
        "the atomic replacement removes the parent reference"
    );
    assert_eq!(
        store.find("refs/heads/topic/child")?.target.try_id(),
        Some(commit_id.as_ref()),
        "the atomic replacement publishes the child reference"
    );
    Ok(())
}

#[test]
fn one_authoritative_stack_lock_covers_all_transaction_edits() -> TestResult {
    let (_temp, store) = empty_store()?;
    let commit_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let prepared = store.transaction().prepare(
        [update(
            "refs/heads/first",
            PreviousValue::MustNotExist,
            Target::Object(commit_id),
        )],
        gix_lock::acquire::Fail::Immediately,
        gix_lock::acquire::Fail::Immediately,
    )?;
    let competing = store.transaction().prepare(
        [update(
            "refs/heads/second",
            PreviousValue::MustNotExist,
            Target::Object(commit_id),
        )],
        gix_lock::acquire::Fail::Immediately,
        gix_lock::acquire::Fail::Immediately,
    );
    assert!(competing.is_err(), "another writer cannot bypass the held stack lock");
    assert_eq!(
        prepared.rollback().len(),
        1,
        "rolling back the lock-holding transaction returns its requested edit"
    );
    commit(
        &store,
        [update(
            "refs/heads/second",
            PreviousValue::MustNotExist,
            Target::Object(commit_id),
        )],
    )?;
    Ok(())
}

#[test]
fn pristine_state_uses_the_authoritative_reftable_view() -> TestResult {
    let (_temp, store) = empty_store()?;
    let default_ref: &gix_ref::FullNameRef = "refs/heads/main".try_into()?;
    assert_eq!(store.is_pristine(default_ref)?, None, "an empty stack has no HEAD");
    commit(
        &store,
        [update(
            "HEAD",
            PreviousValue::MustNotExist,
            Target::Symbolic(default_ref.to_owned()),
        )],
    )?;
    assert_eq!(
        store.is_pristine(default_ref)?,
        Some(true),
        "a symbolic HEAD without its referent is pristine"
    );
    let commit_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    commit(
        &store,
        [update(
            "refs/heads/main",
            PreviousValue::MustNotExist,
            Target::Object(commit_id),
        )],
    )?;
    assert_eq!(
        store.is_pristine(default_ref)?,
        Some(false),
        "a born default branch makes the repository non-pristine"
    );
    Ok(())
}

#[test]
fn pristine_state_preserves_backend_failures() -> TestResult {
    let (temp, store) = empty_store()?;
    std::fs::write(temp.path().join("reftable/tables.list"), b"missing.ref\n")?;
    let default_ref: &gix_ref::FullNameRef = "refs/heads/main".try_into()?;

    let error = store
        .is_pristine(default_ref)
        .expect_err("a corrupt authoritative stack is not reported as uncertainty");
    assert_eq!(
        error.operation(),
        "inspect pristine reftable state",
        "the operation context identifies the failed pristine-state read"
    );
    assert!(
        std::error::Error::source(&error).is_some(),
        "the adapter error remains available through the source chain"
    );
    Ok(())
}

#[test]
fn log_only_deletion_preserves_the_ref_and_deletes_empty_markers() -> TestResult {
    let (temp, store) = empty_store()?;
    let commit_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    assert!(
        store.reflog_iter("refs/heads/missing")?.all()?.is_none(),
        "a missing reflog does not produce an empty iterator"
    );
    commit(
        &store,
        [update(
            "refs/heads/main",
            PreviousValue::MustNotExist,
            Target::Object(commit_id),
        )],
    )?;
    assert!(
        store.reflog_exists("refs/heads/main")?,
        "the forced update creates a reflog"
    );
    commit(
        &store,
        [delete("refs/heads/main", PreviousValue::MustExist, RefLog::Only)],
    )?;
    assert!(
        store.try_find("refs/heads/main")?.is_some(),
        "a log-only deletion preserves the reference"
    );
    assert!(
        !store.reflog_exists("refs/heads/main")?,
        "a log-only deletion hides the reflog"
    );

    let stack = gix_reftable::Stack::open(
        temp.path().join("reftable"),
        crate::fixture_hash_kind(),
        Default::default(),
        Default::default(),
    )?;
    let addition = stack.begin_addition(Default::default())?;
    let update_index = addition.next_update_index();
    addition.commit(
        &[],
        &[gix_reftable::LogRecord {
            ref_name: "refs/heads/empty".into(),
            update_index,
            value: gix_reftable::LogValue::Placeholder,
        }],
    )?;
    assert!(
        store.reflog_exists("refs/heads/empty")?,
        "a placeholder represents an existing empty reflog"
    );
    assert_eq!(
        store
            .reflog_iter("refs/heads/empty")?
            .all()?
            .expect("the empty reflog exists")
            .count(),
        0,
        "an empty reflog yields no update entries"
    );
    commit(&store, [delete("refs/heads/empty", PreviousValue::Any, RefLog::Only)])?;
    assert!(
        !store.reflog_exists("refs/heads/empty")?,
        "deleting the placeholder removes the empty reflog"
    );
    Ok(())
}

#[test]
fn compact_write_strategy_records_a_fully_peeled_tag() -> TestResult {
    let source = crate::scripted_fixture_read_only("make_packed_ref_repository.sh")?;
    let source_git_dir = source.join(".git");
    let files = gix_ref::Store::at(source_git_dir.clone(), crate::fixture_hash_kind());
    let tag_id = files.find("dt1")?.target.into_id();
    let peeled_id = files.find("main")?.target.into_id();
    let objects = gix_odb::at(source_git_dir.join("objects"), crate::fixture_hash_kind())?;
    let (_temp, store) = empty_store()?;
    let edit = update(
        "refs/tags/annotated",
        PreviousValue::MustNotExist,
        Target::Object(tag_id),
    );
    let mut time = gix_actor::date::parse::TimeBuf::default();
    store
        .transaction()
        .write_strategy(WriteStrategy::Compact {
            objects: Box::new(objects),
            remove_separate_source: true,
        })
        .prepare(
            [edit],
            gix_lock::acquire::Fail::Immediately,
            gix_lock::acquire::Fail::Immediately,
        )?
        .commit(crate::file::transaction::prepare_and_commit::committer().to_ref(&mut time))?;
    assert_eq!(
        store.find("refs/tags/annotated")?.peeled,
        Some(peeled_id),
        "compact writes persist the fully peeled tag target"
    );
    Ok(())
}

#[test]
fn linked_and_explicit_other_worktree_names_route_to_their_own_stacks() -> TestResult {
    let temp = gix_testtools::tempfile::TempDir::new()?;
    let common = temp.path().join("common");
    let current = common.join("worktrees/w1");
    let other = common.join("worktrees/w2");
    create_stack(&common.join("reftable"))?;
    create_stack(&current.join("reftable"))?;
    create_stack(&other.join("reftable"))?;
    let store =
        gix_ref::Store::open_reftable_for_linked_worktree(current.clone(), common.clone(), crate::fixture_hash_kind())?;
    let shared_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let current_id = crate::hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    let other_id = crate::hex_to_id("4c3f4cce493d7beb45012e478021b5f65295e5a3");
    commit(
        &store,
        [
            update(
                "refs/heads/main",
                PreviousValue::MustNotExist,
                Target::Object(shared_id),
            ),
            update("HEAD", PreviousValue::MustNotExist, Target::Object(current_id)),
            update(
                "refs/bisect/current",
                PreviousValue::MustNotExist,
                Target::Object(current_id),
            ),
            update(
                "main-worktree/HEAD",
                PreviousValue::MustNotExist,
                Target::Object(shared_id),
            ),
            update(
                "main-worktree/refs/bisect/main-only",
                PreviousValue::MustNotExist,
                Target::Object(shared_id),
            ),
            update(
                "worktrees/w2/refs/bisect/bad",
                PreviousValue::MustNotExist,
                Target::Object(other_id),
            ),
            update(
                "worktrees/w2/HEAD",
                PreviousValue::MustNotExist,
                Target::Symbolic("worktrees/w2/refs/bisect/bad".try_into()?),
            ),
        ],
    )?;

    assert_eq!(
        store.find("HEAD")?.target.try_id(),
        Some(current_id.as_ref()),
        "unqualified HEAD resolves in the current worktree stack"
    );
    assert_eq!(
        store.find("main-worktree/HEAD")?.target.try_id(),
        Some(shared_id.as_ref()),
        "main-worktree HEAD resolves in the main worktree stack"
    );
    assert_eq!(
        store.find("worktrees/w2/refs/heads/main")?.target.try_id(),
        Some(shared_id.as_ref()),
        "shared refs ignore the explicit worktree selector"
    );
    let mut other_head = store.find("worktrees/w2/HEAD")?;
    assert_eq!(
        other_head.follow_to_object(&store)?,
        other_id,
        "an explicit other-worktree symbolic HEAD follows within that worktree"
    );
    assert_eq!(
        other_head.name, "worktrees/w2/refs/bisect/bad",
        "following retains the caller-visible explicit worktree name"
    );
    let stable = store.snapshot()?;
    let other_stack = gix_reftable::Stack::open(
        other.join("reftable"),
        crate::fixture_hash_kind(),
        Default::default(),
        Default::default(),
    )?;
    let addition = other_stack.begin_addition(Default::default())?;
    let update_index = addition.next_update_index();
    addition.commit(
        &[gix_reftable::RefRecord {
            name: "refs/bisect/bad".into(),
            update_index,
            value: gix_reftable::RefValue::Direct(current_id),
        }],
        &[],
    )?;
    assert_eq!(
        stable
            .try_find("worktrees/w2/refs/bisect/bad")?
            .expect("the other-worktree ref exists when first accessed")
            .target
            .try_id(),
        Some(current_id.as_ref()),
        "an other-worktree stack is pinned lazily on first access"
    );
    let addition = other_stack.begin_addition(Default::default())?;
    let update_index = addition.next_update_index();
    addition.commit(
        &[gix_reftable::RefRecord {
            name: "refs/bisect/bad".into(),
            update_index,
            value: gix_reftable::RefValue::Direct(shared_id),
        }],
        &[],
    )?;
    assert_eq!(
        stable
            .try_find("worktrees/w2/refs/bisect/bad")?
            .expect("the first-access generation remains cached")
            .target
            .try_id(),
        Some(current_id.as_ref()),
        "later publications do not change an already-open other-worktree snapshot"
    );
    assert_eq!(
        store.find("worktrees/w2/refs/bisect/bad")?.target.try_id(),
        Some(shared_id.as_ref()),
        "a new store snapshot observes the later generation"
    );
    assert!(
        store.try_find("refs/bisect/main-only")?.is_none(),
        "a main-worktree private reference is absent from the current-worktree namespace"
    );
    assert!(
        store.try_find("main-worktree/refs/bisect/main-only")?.is_some(),
        "the explicit main-worktree selector exposes its private reference"
    );

    let names = store
        .iter()?
        .all()?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|reference| reference.name)
        .collect::<Vec<_>>();
    assert!(
        names.iter().any(|name| name == "refs/heads/main"),
        "iteration includes shared references"
    );
    assert!(
        names.iter().any(|name| name == "refs/bisect/current"),
        "iteration includes current-worktree private references"
    );
    assert!(
        !names.iter().any(|name| name == "refs/bisect/main-only"),
        "iteration excludes main-worktree private references from the current view"
    );
    Ok(())
}

#[test]
fn misplaced_shared_records_do_not_shadow_the_common_stack_and_fail_verification() -> TestResult {
    let temp = gix_testtools::tempfile::TempDir::new()?;
    let common = temp.path().join("common");
    let current = common.join("worktrees/w1");
    create_stack(&common.join("reftable"))?;
    let current_stack = create_stack(&current.join("reftable"))?;
    let store = gix_ref::Store::open_reftable_for_linked_worktree(current, common, crate::fixture_hash_kind())?;
    let shared_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let misplaced_id = crate::hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    commit(
        &store,
        [update(
            "refs/heads/main",
            PreviousValue::MustNotExist,
            Target::Object(shared_id),
        )],
    )?;
    let addition = current_stack.begin_addition(Default::default())?;
    let update_index = addition.next_update_index();
    addition.commit(
        &[gix_reftable::RefRecord {
            name: "refs/heads/main".into(),
            update_index,
            value: gix_reftable::RefValue::Direct(misplaced_id),
        }],
        &[],
    )?;

    assert_eq!(
        store.find("refs/heads/main")?.target.try_id(),
        Some(shared_id.as_ref()),
        "an exact shared-reference read routes to the common stack"
    );
    let iterated = store
        .iter()?
        .all()?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .find(|reference| reference.name == "refs/heads/main")
        .expect("iteration includes the shared reference");
    assert_eq!(
        iterated.target.try_id(),
        Some(shared_id.as_ref()),
        "iteration applies the same common-stack routing as an exact read"
    );
    let error = store
        .verify()
        .expect_err("verification rejects a shared record stored in a per-worktree stack");
    let source = std::error::Error::source(&error).expect("backend errors retain their adapter-specific source");
    assert!(
        source.to_string().contains("is not worktree-private"),
        "the source identifies the misplaced record: {source}"
    );
    Ok(())
}

#[test]
fn explicit_worktree_paths_honor_precomposition_and_device_name_options() -> TestResult {
    let temp = gix_testtools::tempfile::TempDir::new()?;
    let common = temp.path().join("common");
    let current = common.join("worktrees/w1");
    let precomposed = "caf\u{e9}";
    let decomposed = "cafe\u{301}";
    create_stack(&common.join("reftable"))?;
    create_stack(&current.join("reftable"))?;
    let other_stack = create_stack(&common.join("worktrees").join(precomposed).join("reftable"))?;
    let target_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let addition = other_stack.begin_addition(Default::default())?;
    let update_index = addition.next_update_index();
    addition.commit(
        &[gix_reftable::RefRecord {
            name: "HEAD".into(),
            update_index,
            value: gix_reftable::RefValue::Direct(target_id),
        }],
        &[],
    )?;
    let store = gix_ref::Store::open_reftable_for_linked_worktree_opts(
        current,
        common,
        crate::fixture_hash_kind(),
        gix_ref::store::init::Options {
            precompose_unicode: true,
            prohibit_windows_device_names: true,
            ..Default::default()
        },
    )?;

    let stable = store.snapshot()?;
    let decomposed_head = format!("worktrees/{decomposed}/HEAD");
    assert_eq!(
        stable
            .try_find(decomposed_head.as_str())?
            .expect("the precomposed worktree stack contains HEAD")
            .target
            .try_id(),
        Some(target_id.as_ref()),
        "precomposition selects the linked-worktree directory used on disk"
    );
    let later_id = crate::hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    let addition = other_stack.begin_addition(Default::default())?;
    let update_index = addition.next_update_index();
    addition.commit(
        &[gix_reftable::RefRecord {
            name: "HEAD".into(),
            update_index,
            value: gix_reftable::RefValue::Direct(later_id),
        }],
        &[],
    )?;
    let precomposed_head = format!("worktrees/{precomposed}/HEAD");
    assert_eq!(
        stable
            .try_find(precomposed_head.as_str())?
            .expect("the alternate spelling addresses the same stack")
            .target
            .try_id(),
        Some(target_id.as_ref()),
        "alternate Unicode spellings share one first-access snapshot"
    );
    assert_eq!(
        store.find(precomposed_head.as_str())?.target.try_id(),
        Some(later_id.as_ref()),
        "a fresh snapshot observes the later stack generation"
    );
    let error = store
        .try_find("worktrees/NUL/HEAD")
        .expect_err("reserved Windows device names are rejected before path access");
    let source = std::error::Error::source(&error).expect("the backend error retains its adapter source");
    assert!(
        source.to_string().contains("reserved Windows device name"),
        "the source identifies the rejected path component: {source}"
    );
    Ok(())
}

#[test]
fn normal_reflog_creation_uses_the_public_name_under_a_namespace() -> TestResult {
    let (_temp, mut store) = empty_store()?;
    store.replace_namespace(Some(gix_ref::namespace::expand("tenant")?.to_owned()));
    let target_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let mut edit = update(
        "refs/heads/scoped",
        PreviousValue::MustNotExist,
        Target::Object(target_id),
    );
    let Change::Update { log, .. } = &mut edit.change else {
        unreachable!("the helper always creates an update")
    };
    log.force_create_reflog = false;
    commit(&store, [edit])?;

    assert!(
        store.reflog_exists("refs/heads/scoped")?,
        "normal policy autocreates a branch reflog before applying the namespace prefix"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn linked_worktree_symlinks_cannot_escape_reads_or_maintenance() -> TestResult {
    use std::os::unix::fs::symlink;

    let temp = gix_testtools::tempfile::TempDir::new()?;
    let common = temp.path().join("common");
    let current = common.join("worktrees/w1");
    create_stack(&common.join("reftable"))?;
    create_stack(&current.join("reftable"))?;
    let outside = create_stack(&temp.path().join("outside-reftable"))?;
    outside.begin_addition(Default::default())?.commit(
        &[gix_reftable::RefRecord {
            name: "HEAD".into(),
            update_index: 1,
            value: gix_reftable::RefValue::Direct(crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03")),
        }],
        &[],
    )?;
    outside.begin_addition(Default::default())?.commit(
        &[gix_reftable::RefRecord {
            name: "HEAD".into(),
            update_index: 2,
            value: gix_reftable::RefValue::Direct(crate::hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")),
        }],
        &[],
    )?;
    let escaped = common.join("worktrees/escaped");
    std::fs::create_dir_all(&escaped)?;
    symlink(outside.directory(), escaped.join("reftable"))?;
    let store = gix_ref::Store::open_reftable_for_linked_worktree(current, common, crate::fixture_hash_kind())?;

    assert!(
        store.try_find("worktrees/escaped/HEAD").is_err(),
        "explicit reads reject a linked-worktree stack symlink"
    );
    assert!(
        store
            .optimize(
                gix_ref::store::maintenance::Options::default(),
                gix_lock::acquire::Fail::Immediately,
            )
            .is_err(),
        "maintenance rejects a linked-worktree stack symlink"
    );
    assert_eq!(
        outside.snapshot()?.members().len(),
        2,
        "rejected maintenance does not compact storage outside the worktree boundary"
    );
    Ok(())
}

#[test]
fn maintenance_verifies_and_optimizes_every_worktree_stack() -> TestResult {
    let temp = gix_testtools::tempfile::TempDir::new()?;
    let common = temp.path().join("common");
    let current = common.join("worktrees/w1");
    let other = common.join("worktrees/w2");
    for directory in [&common, &current, &other] {
        create_stack(&directory.join("reftable"))?;
    }
    let store =
        gix_ref::Store::open_reftable_for_linked_worktree(current.clone(), common.clone(), crate::fixture_hash_kind())?;
    let first_object_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    let second_object_id = crate::hex_to_id("4c3f4cce493d7beb45012e478021b5f65295e5a3");
    let edits = |expected: PreviousValue, target_object_id: gix_hash::ObjectId| {
        [
            update("refs/heads/shared", expected.clone(), Target::Object(target_object_id)),
            update(
                "refs/bisect/current",
                expected.clone(),
                Target::Object(target_object_id),
            ),
            update(
                "worktrees/w2/refs/bisect/other",
                expected,
                Target::Object(target_object_id),
            ),
        ]
    };
    commit(&store, edits(PreviousValue::MustNotExist, first_object_id))?;
    commit(&store, edits(PreviousValue::Any, second_object_id))?;

    let common_stack = create_stack(&common.join("reftable"))?;
    let abandoned = common.join("reftable/0x000000000001-0x000000000001-abandoned.ref");
    let listed = common_stack.snapshot()?.members()[0].file_name.clone();
    std::fs::copy(common.join("reftable").join(listed), &abandoned)?;
    store.verify()?;
    store.optimize(
        gix_ref::store::maintenance::Options {
            expire_reflogs_before: Some(u64::MAX),
            keep_latest_reflog_entries: 1,
            cleanup_abandoned: true,
        },
        gix_lock::acquire::Fail::Immediately,
    )?;

    for (directory, reflog) in [
        (&common, b"refs/heads/shared".as_slice()),
        (&current, b"refs/bisect/current".as_slice()),
        (&other, b"refs/bisect/other".as_slice()),
    ] {
        let stack = create_stack(&directory.join("reftable"))?;
        let snapshot = stack.snapshot()?;
        assert_eq!(snapshot.members().len(), 1, "{} was compacted", directory.display());
        assert_eq!(snapshot.logs_for(reflog).len(), 1, "the newest reflog entry is kept");
    }
    assert!(!abandoned.exists(), "unreachable complete tables are cleaned up");
    store.verify()?;
    Ok(())
}

#[test]
fn verification_rejects_an_invalid_symbolic_target() -> TestResult {
    let (temp, store) = empty_store()?;
    let stack = create_stack(&temp.path().join("reftable"))?;
    let addition = stack.begin_addition(Default::default())?;
    let update_index = addition.next_update_index();
    addition.commit(
        &[gix_reftable::RefRecord {
            name: "refs/heads/invalid".into(),
            update_index,
            value: gix_reftable::RefValue::Symbolic("not-a-full-reference".into()),
        }],
        &[],
    )?;

    let err = store.verify().expect_err("invalid symbolic targets fail verification");
    assert_eq!(
        err.operation(),
        "verify reftable reference storage",
        "the backend-neutral error identifies the failed operation"
    );
    let source = std::error::Error::source(&err).expect("backend errors retain their adapter-specific source");
    assert!(
        source.to_string().contains("invalid symbolic target"),
        "the source chain identifies the violated invariant: {source}"
    );
    Ok(())
}

#[test]
fn verification_rejects_directory_file_conflicts_in_prebuilt_tables() -> TestResult {
    let (temp, store) = empty_store()?;
    let stack = create_stack(&temp.path().join("reftable"))?;
    let addition = stack.begin_addition(Default::default())?;
    let update_index = addition.next_update_index();
    let target_object_id = crate::hex_to_id("134385f6d781b7e97062102c6a483440bfda2a03");
    addition.commit(
        &[
            gix_reftable::RefRecord {
                name: "refs/heads/topic".into(),
                update_index,
                value: gix_reftable::RefValue::Direct(target_object_id),
            },
            gix_reftable::RefRecord {
                name: "refs/heads/topic/child".into(),
                update_index,
                value: gix_reftable::RefValue::Direct(target_object_id),
            },
        ],
        &[],
    )?;

    let error = store
        .verify()
        .expect_err("verification rejects a prebuilt directory-file conflict");
    let source = std::error::Error::source(&error).expect("backend errors retain their adapter-specific source");
    assert!(
        source.to_string().contains("conflicts with reference"),
        "the source identifies both sides of the name conflict: {source}"
    );
    Ok(())
}

#[test]
fn verification_is_read_only_and_does_not_wait_for_the_publication_lock() -> TestResult {
    let (temp, store) = empty_store()?;
    let list_path = temp.path().join("reftable/tables.list");
    let _publication_lock =
        gix_lock::File::acquire_to_update_resource(&list_path, gix_lock::acquire::Fail::Immediately, None)?;

    store.verify()?;
    Ok(())
}

fn git<I, S>(repo: Option<&Path>, args: I) -> TestResult<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    gix_testtools::isolated_git_output(repo, args)
}

fn git_ok<I, S>(repo: Option<&Path>, args: I) -> TestResult<Output>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    gix_testtools::isolated_git_output_checked(repo, args)
}

#[test]
fn git_and_the_adapter_consume_each_others_transactions() -> TestResult {
    if gix_testtools::should_skip_as_git_version_is_smaller_than(2, 45, 0) {
        return Ok(());
    }
    let temp = gix_testtools::tempfile::TempDir::new()?;
    let hash = match crate::fixture_hash_kind() {
        gix_hash::Kind::Sha1 => "sha1",
        gix_hash::Kind::Sha256 => "sha256",
        _ => return Ok(()),
    };
    let object_format = format!("--object-format={hash}");
    git_ok(
        None,
        [
            OsStr::new("init"),
            OsStr::new("--quiet"),
            OsStr::new("--initial-branch=main"),
            OsStr::new("--ref-format=reftable"),
            OsStr::new(&object_format),
            temp.path().as_os_str(),
        ],
    )?;
    git_ok(
        Some(temp.path()),
        [
            "-c",
            "user.name=Adapter Test",
            "-c",
            "user.email=adapter@example.com",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "initial",
        ],
    )?;
    let head = String::from_utf8(git_ok(Some(temp.path()), ["rev-parse", "HEAD"])?.stdout)?;
    let head_id = gix_hash::ObjectId::from_hex(head.trim().as_bytes())?;
    let store = gix_ref::Store::open_reftable(temp.path().join(".git"), crate::fixture_hash_kind())?;
    commit(
        &store,
        [update(
            "refs/heads/from-adapter",
            PreviousValue::MustNotExist,
            Target::Object(head_id),
        )],
    )?;
    let resolved = git_ok(Some(temp.path()), ["rev-parse", "refs/heads/from-adapter"])?;
    assert_eq!(
        String::from_utf8(resolved.stdout)?.trim(),
        head.trim(),
        "Git resolves a reference written through the adapter"
    );
    let reflog = git_ok(
        Some(temp.path()),
        ["reflog", "show", "--format=%gs", "refs/heads/from-adapter"],
    )?;
    assert_eq!(
        String::from_utf8(reflog.stdout)?.trim(),
        "update refs/heads/from-adapter",
        "Git reads the adapter-written reflog message"
    );
    commit(
        &store,
        [delete(
            "refs/heads/from-adapter",
            PreviousValue::MustExist,
            RefLog::Only,
        )],
    )?;
    assert!(
        !git(Some(temp.path()), ["reflog", "exists", "refs/heads/from-adapter"])?
            .status
            .success(),
        "Git observes the adapter-written reflog tombstones"
    );
    git_ok(Some(temp.path()), ["show-ref", "--verify", "refs/heads/from-adapter"])?;

    git_ok(Some(temp.path()), ["update-ref", "refs/heads/from-git", head.trim()])?;
    assert_eq!(
        store.find("refs/heads/from-git")?.target.try_id(),
        Some(head_id.as_ref()),
        "the adapter resolves a reference written by Git"
    );

    let mut child = gix_testtools::isolated_git_command(Some(temp.path()))
        .args(["update-ref", "--stdin"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or("git update-ref stdin was unavailable")?
        .write_all(format!("delete refs/heads/from-git {head_id}\n").as_bytes())?;
    let output = child.wait_with_output()?;
    assert!(
        output.status.success(),
        "Git transaction failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        store.try_find("refs/heads/from-git")?.is_none(),
        "the adapter observes a Git-written deletion"
    );
    Ok(())
}

#[test]
fn git_and_the_adapter_agree_on_linked_worktree_routing() -> TestResult {
    if gix_testtools::should_skip_as_git_version_is_smaller_than(2, 45, 0) {
        return Ok(());
    }
    let temp = gix_testtools::tempfile::TempDir::new()?;
    let repository = temp.path().join("repository");
    let first_worktree = temp.path().join("worktree-one");
    let second_worktree = temp.path().join("worktree-two");
    let hash = match crate::fixture_hash_kind() {
        gix_hash::Kind::Sha1 => "sha1",
        gix_hash::Kind::Sha256 => "sha256",
        _ => return Ok(()),
    };
    let object_format = format!("--object-format={hash}");
    git_ok(
        None,
        [
            OsStr::new("init"),
            OsStr::new("--quiet"),
            OsStr::new("--initial-branch=main"),
            OsStr::new("--ref-format=reftable"),
            OsStr::new(&object_format),
            repository.as_os_str(),
        ],
    )?;
    git_ok(
        Some(&repository),
        [
            "-c",
            "user.name=Adapter Test",
            "-c",
            "user.email=adapter@example.com",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            "initial",
        ],
    )?;
    git_ok(
        Some(&repository),
        [
            OsStr::new("worktree"),
            OsStr::new("add"),
            OsStr::new("--quiet"),
            OsStr::new("-b"),
            OsStr::new("worktree-one"),
            first_worktree.as_os_str(),
            OsStr::new("HEAD"),
        ],
    )?;
    git_ok(
        Some(&repository),
        [
            OsStr::new("worktree"),
            OsStr::new("add"),
            OsStr::new("--quiet"),
            OsStr::new("-b"),
            OsStr::new("worktree-two"),
            second_worktree.as_os_str(),
            OsStr::new("HEAD"),
        ],
    )?;
    let current_git_dir = Path::new(
        std::str::from_utf8(&git_ok(Some(&first_worktree), ["rev-parse", "--absolute-git-dir"])?.stdout)?.trim(),
    )
    .to_owned();
    let common_dir = Path::new(
        std::str::from_utf8(
            &git_ok(
                Some(&first_worktree),
                ["rev-parse", "--path-format=absolute", "--git-common-dir"],
            )?
            .stdout,
        )?
        .trim(),
    )
    .to_owned();
    let other_git_dir = Path::new(
        std::str::from_utf8(&git_ok(Some(&second_worktree), ["rev-parse", "--absolute-git-dir"])?.stdout)?.trim(),
    )
    .to_owned();
    let other_name = other_git_dir
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or("Git produced a non-UTF-8 worktree id")?;
    let head = String::from_utf8(git_ok(Some(&repository), ["rev-parse", "HEAD"])?.stdout)?;
    let head_id = gix_hash::ObjectId::from_hex(head.trim().as_bytes())?;
    let store =
        gix_ref::Store::open_reftable_for_linked_worktree(current_git_dir, common_dir, crate::fixture_hash_kind())?;
    commit(
        &store,
        [
            update(
                "refs/heads/from-adapter-shared",
                PreviousValue::MustNotExist,
                Target::Object(head_id),
            ),
            update(
                "refs/bisect/from-adapter-current",
                PreviousValue::MustNotExist,
                Target::Object(head_id),
            ),
            update(
                &format!("worktrees/{other_name}/refs/bisect/from-adapter-other"),
                PreviousValue::MustNotExist,
                Target::Object(head_id),
            ),
        ],
    )?;

    for worktree in [&first_worktree, &second_worktree] {
        let shared = git_ok(Some(worktree), ["rev-parse", "refs/heads/from-adapter-shared"])?;
        assert_eq!(
            String::from_utf8(shared.stdout)?.trim(),
            head.trim(),
            "Git resolves the adapter-written shared reference from every worktree"
        );
    }
    let current = git_ok(Some(&first_worktree), ["rev-parse", "refs/bisect/from-adapter-current"])?;
    assert_eq!(
        String::from_utf8(current.stdout)?.trim(),
        head.trim(),
        "Git resolves the adapter-written current-worktree reference"
    );
    assert!(
        !git(
            Some(&second_worktree),
            ["rev-parse", "--verify", "refs/bisect/from-adapter-current"]
        )?
        .status
        .success(),
        "current-worktree refs do not leak to another worktree"
    );
    let other = git_ok(Some(&second_worktree), ["rev-parse", "refs/bisect/from-adapter-other"])?;
    assert_eq!(
        String::from_utf8(other.stdout)?.trim(),
        head.trim(),
        "Git resolves the adapter-written other-worktree reference"
    );

    git_ok(
        Some(&second_worktree),
        ["update-ref", "refs/bisect/from-git", head.trim()],
    )?;
    let explicit_other_ref = format!("worktrees/{other_name}/refs/bisect/from-git");
    assert_eq!(
        store.find(explicit_other_ref.as_str())?.target.try_id(),
        Some(head_id.as_ref()),
        "the adapter resolves a Git-written explicit other-worktree reference"
    );
    assert_ne!(
        store.find("HEAD")?.target,
        store.find("main-worktree/HEAD")?.target,
        "the linked worktree and main worktree retain distinct HEAD targets"
    );
    Ok(())
}
