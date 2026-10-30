//! Git-compatible transaction baselines, kept distinct from dedicated branch renames.
//! Only store construction selects a backend; setup and observations use logical refs and reflogs.

use std::collections::BTreeMap;

use gix_lock::acquire::Fail;
use gix_ref::{
    Target,
    file::Store,
    log::Line,
    transaction::{PreviousValue, RefEdit},
};
use gix_testtools::{Creation, TestResult, scripted_fixture_read_only_with_args, scripted_fixture_writable_with_args};

#[test]
fn namespace_conflicts_are_rejected_before_commit_in_loose_and_packed_storage() -> TestResult {
    let script = "make_ref_namespace_transaction_baseline.sh";
    for (storage, namespace) in [
        ("loose", "-"),
        ("packed", "-"),
        ("both", "-"),
        ("packed", "scope/nested"),
    ] {
        for state in ["existing", "new"] {
            for (first, second, succeeds) in [
                ("refs/heads/topic", "refs/heads/topic/child", false),
                ("refs/heads/topic/child", "refs/heads/topic", false),
                ("refs/heads/topic", "refs/heads/renamed", true),
            ] {
                for order in ["first-first", "second-first"] {
                    let args = [first, second, state, storage, namespace, order, "setup"];
                    let fixture = scripted_fixture_writable_with_args(script, args, Creation::CopyFromReadOnly)?;
                    let mut baseline_args = args;
                    baseline_args[6] = "git";
                    let baseline = scripted_fixture_read_only_with_args(script, baseline_args)?;
                    let git_succeeds = std::fs::read_to_string(baseline.join("status"))?.trim() == "0";
                    let context = format!("{first}, {second}, {state}, {storage}, {namespace}, {order}");
                    assert_eq!(git_succeeds, succeeds, "Git baseline: {context}");
                    let mut store = Store::at(fixture.path().join(".git"), crate::fixture_hash_kind());
                    store.write_reflog = gix_ref::store::WriteReflog::Disable;
                    store.namespace = (namespace != "-")
                        .then(|| gix_ref::namespace::expand(namespace))
                        .transpose()?;
                    let mut baseline = Store::at(baseline.join(".git"), crate::fixture_hash_kind());
                    baseline.namespace = store.namespace.clone();
                    let commit_id = gix_hash::ObjectId::from_hex(
                        std::fs::read_to_string(fixture.path().join("commit-id"))?
                            .trim()
                            .as_bytes(),
                    )?;
                    let mut edits = vec![RefEdit::update(
                        second.try_into()?,
                        commit_id,
                        PreviousValue::MustNotExist,
                        "create",
                    )];
                    if state == "new" {
                        edits.insert(
                            0,
                            RefEdit::update(first.try_into()?, commit_id, PreviousValue::MustNotExist, "create"),
                        );
                    }
                    if order == "second-first" {
                        edits.reverse();
                    }
                    let transaction = store.transaction().prepare(edits, Fail::Immediately, Fail::Immediately);
                    assert_eq!(
                        transaction.is_ok(),
                        succeeds,
                        "namespace conflicts fail during preparation: {context}"
                    );
                    if let Ok(transaction) = transaction {
                        transaction.commit(None)?;
                    }
                    assert_eq!(
                        observe(&store, first, second)?,
                        observe(&baseline, first, second)?,
                        "retain the same refs and logs as Git: {context}"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn head_reflog_baseline_for_direct_updates_and_symbolic_retargeting() -> TestResult {
    let script = "make_ref_head_transaction_baseline.sh";
    for storage in ["loose", "packed", "both"] {
        for (mode, scenario) in [
            ("symref", "existing"),
            ("symref", "source-unborn"),
            ("symref", "destination-unborn"),
            ("symref", "both-unborn"),
            ("symref", "same-existing"),
            ("symref", "same-unborn"),
            ("symref-referent", "existing"),
            ("symref-referent", "destination-unborn"),
            ("direct", "changed"),
            ("direct", "no-op"),
            ("direct", "unrelated-changed"),
            ("direct", "unrelated-no-op"),
        ] {
            let fixture = scripted_fixture_writable_with_args(
                script,
                [mode, scenario, storage, "setup"],
                Creation::CopyFromReadOnly,
            )?;
            let baseline = scripted_fixture_read_only_with_args(script, [mode, scenario, storage, "git"])?;
            let store = Store::at(fixture.path().join(".git"), crate::fixture_hash_kind());
            let new_target = std::fs::read_to_string(fixture.path().join("new-target"))?;
            let (name, new) = if mode.starts_with("symref") {
                (
                    if mode == "symref" { "HEAD" } else { "refs/heads/source" },
                    Target::Symbolic(new_target.trim().try_into()?),
                )
            } else {
                (
                    "refs/heads/source",
                    Target::Object(gix_hash::ObjectId::from_hex(new_target.trim().as_bytes())?),
                )
            };
            let committer = gix_actor::SignatureRef::from_bytes(b"committer <committer@example.com> 946771200 +0000")?;
            assert_eq!(
                std::fs::read_to_string(baseline.join("status"))?.trim(),
                "0",
                "Git baseline: {mode}, {scenario}, {storage}"
            );
            let edit = RefEdit::update(name.try_into()?, new, PreviousValue::Any, "change");
            store
                .transaction()
                .prepare([edit], Fail::Immediately, Fail::Immediately)?
                .commit(Some(committer))?;
            let baseline = Store::at(baseline.join(".git"), crate::fixture_hash_kind());
            assert_eq!(
                observe(&store, "refs/heads/source", "refs/heads/destination")?,
                observe(&baseline, "refs/heads/source", "refs/heads/destination")?,
                "{mode}, {scenario}, {storage}: refs and complete reflogs match Git"
            );
        }
    }
    Ok(())
}

#[test]
fn explicit_head_edits_take_precedence_over_automatic_reflog_updates() -> TestResult {
    for storage in ["loose", "packed", "both"] {
        for symbolic in [false, true] {
            for head_first in [false, true] {
                let fixture = scripted_fixture_writable_with_args(
                    "make_ref_head_transaction_baseline.sh",
                    ["direct", "changed", storage, "setup"],
                    Creation::CopyFromReadOnly,
                )?;
                let store = Store::at(fixture.path().join(".git"), crate::fixture_hash_kind());
                let commit_id = store.find("refs/heads/destination")?.target.id().to_owned();
                let head_target = if symbolic {
                    Target::Symbolic("refs/heads/destination".try_into()?)
                } else {
                    Target::Object(commit_id)
                };
                let before = observe(&store, "refs/heads/source", "refs/heads/destination")?;
                let mut edits = vec![
                    RefEdit::update("refs/heads/source".try_into()?, commit_id, PreviousValue::Any, "branch"),
                    RefEdit::update(
                        "HEAD".try_into()?,
                        head_target.clone(),
                        PreviousValue::Any,
                        "explicit HEAD",
                    ),
                ];
                if head_first {
                    edits.reverse();
                }
                let committer =
                    gix_actor::SignatureRef::from_bytes(b"committer <committer@example.com> 946771200 +0000")?;
                let performed = store
                    .transaction()
                    .prepare(edits, Fail::Immediately, Fail::Immediately)?
                    .commit(Some(committer))?;
                assert_eq!(performed.len(), 2, "perform only the caller's non-dereferencing edits");
                let after = observe(&store, "refs/heads/source", "refs/heads/destination")?;
                assert_eq!(after.refs["HEAD"], head_target, "preserve the explicit raw HEAD target");
                assert_eq!(
                    after.refs["refs/heads/source"],
                    Target::Object(commit_id),
                    "update the branch too"
                );
                assert_eq!(
                    after.logs["HEAD"].len(),
                    before.logs["HEAD"].len() + 1,
                    "write one HEAD entry"
                );
                let head_entry = after.logs["HEAD"]
                    .last()
                    .expect("the explicit edit appended a HEAD entry");
                assert_eq!(head_entry.new_oid, commit_id, "log the explicit HEAD target");
                assert_eq!(head_entry.message, "explicit HEAD", "retain the caller's HEAD message");
            }
        }
    }
    Ok(())
}

#[test]
fn symbolic_reference_transactions_with_unresolvable_targets() -> TestResult {
    let script = "make_ref_symref_transaction_baseline.sh";
    for storage in ["loose", "packed", "both"] {
        for corruption in ["malformed", "cycle"] {
            for scenario in [
                "delete",
                "delete-head",
                "delete-expected",
                "replace-direct",
                "replace-symbolic",
                "replace-symbolic-expected",
                "new-target",
            ] {
                let fixture = scripted_fixture_writable_with_args(
                    script,
                    [scenario, corruption, storage, "setup"],
                    Creation::CopyFromReadOnly,
                )?;
                let baseline = scripted_fixture_read_only_with_args(script, [scenario, corruption, storage, "git"])?;
                let store = Store::at(fixture.path().join(".git"), crate::fixture_hash_kind());
                let alias = "refs/heads/alias";
                let expected = if scenario.ends_with("-expected") {
                    PreviousValue::MustExistAndMatch(store.find(alias)?.target)
                } else {
                    PreviousValue::Any
                };
                let edit = if scenario.starts_with("delete") {
                    RefEdit::delete(alias.try_into()?, expected)
                } else {
                    let new = if scenario == "replace-direct" {
                        store.find("refs/heads/destination")?.target
                    } else {
                        Target::Symbolic(
                            if scenario == "new-target" {
                                "refs/heads/broken"
                            } else {
                                "refs/heads/destination"
                            }
                            .try_into()?,
                        )
                    };
                    RefEdit::update(alias.try_into()?, new, expected, "change")
                };
                let succeeds = !scenario.ends_with("-expected");
                let context = format!("{scenario}, {corruption}, {storage}");
                assert_eq!(
                    std::fs::read_to_string(baseline.join("status"))?.trim() == "0",
                    succeeds,
                    "Git baseline: {context}"
                );
                if !succeeds {
                    let mut no_logs = store.clone();
                    no_logs.write_reflog = gix_ref::store::WriteReflog::Disable;
                    assert!(
                        no_logs
                            .transaction()
                            .prepare([edit.clone()], Fail::Immediately, Fail::Immediately)
                            .is_err(),
                        "symbolic constraints require resolution even with reflogs disabled: {context}"
                    );
                }
                let transaction =
                    store
                        .transaction()
                        .reflog_message("change")
                        .prepare([edit], Fail::Immediately, Fail::Immediately);
                assert_eq!(
                    transaction.is_ok(),
                    succeeds,
                    "only an expected old target requires successful resolution: {context}"
                );
                if let Ok(transaction) = transaction {
                    let committer =
                        gix_actor::SignatureRef::from_bytes(b"committer <committer@example.com> 946771200 +0000")?;
                    transaction.commit(Some(committer))?;
                }
                let baseline = Store::at(baseline.join(".git"), crate::fixture_hash_kind());
                let names = [alias, "HEAD", "refs/heads/source", "refs/heads/destination"];
                assert_eq!(
                    observe_names(&store, names)?,
                    observe_names(&baseline, names)?,
                    "affected refs and complete reflogs match Git despite unresolvable targets: {context}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn replacement_baseline() -> TestResult {
    for storage in ["loose", "packed", "both"] {
        for initial_logs in ["normal", "missing"] {
            replacement_baseline_for(storage, initial_logs)?;
        }
    }
    Ok(())
}

fn replacement_baseline_for(storage: &str, initial_logs: &str) -> TestResult {
    let script = "make_ref_transaction_baseline.sh";
    // Success columns: gix transaction, Git update-ref transaction, Git branch -m.
    // Git's branch rename handles both directions; ordinary transactions reject both.
    for (source_branch, destination_branch, blocker, successes) in [
        ("topic", "renamed", None, [true, true, true]),
        ("topic/child", "topic", None, [false, false, true]),
        ("topic/nested/child", "topic", None, [false, false, true]),
        ("topic", "topic/child", None, [false, false, true]),
        ("topic/child", "topic", Some("topic/sibling"), [false; 3]),
        ("topic", "renamed", Some("renamed"), [false; 3]),
    ] {
        let setup_args = [
            source_branch,
            destination_branch,
            blocker.unwrap_or("-"),
            "setup",
            "delete-first",
            storage,
            initial_logs,
        ];
        let setup = scripted_fixture_read_only_with_args(script, setup_args)?;
        let initial_store = Store::at(setup.join(".git"), crate::fixture_hash_kind());
        let source = format!("refs/heads/{source_branch}");
        let destination = format!("refs/heads/{destination_branch}");
        let commit_id = initial_store.find(source.as_str())?.target.id().to_owned();
        let initial = observe(&initial_store, &source, &destination)?;
        let committer = gix_actor::SignatureRef::from_bytes(b"committer <committer@example.com> 946771200 +0000")?;

        for (operation, succeeds) in ["gix", "update-ref", "branch"].into_iter().zip(successes) {
            for update_first in [false, true] {
                if operation == "branch" && update_first {
                    continue;
                }
                let context = format!(
                    "{source_branch} -> {destination_branch}, {blocker:?}, {operation}, {storage}, {initial_logs} logs, update first: {update_first}"
                );
                let (directory, _keep) = if operation == "gix" {
                    let fixture = scripted_fixture_writable_with_args(script, setup_args, Creation::CopyFromReadOnly)?;
                    (fixture.path().to_owned(), Some(fixture))
                } else {
                    let mut args = setup_args;
                    args[3] = operation;
                    args[4] = if update_first { "update-first" } else { "delete-first" };
                    (scripted_fixture_read_only_with_args(script, args)?, None)
                };
                let store = Store::at(directory.join(".git"), crate::fixture_hash_kind());
                let mut expected = initial.clone();

                let (success, diagnostic) = if operation == "gix" {
                    let mut edits = [
                        RefEdit::delete(
                            source.as_str().try_into()?,
                            PreviousValue::MustExistAndMatch(commit_id.into()),
                        ),
                        RefEdit::update(
                            destination.as_str().try_into()?,
                            commit_id,
                            PreviousValue::MustNotExist,
                            "rename",
                        ),
                    ];
                    if update_first {
                        edits.reverse();
                    }
                    let result = store
                        .transaction()
                        .reflog_message("rename")
                        .prepare(edits, Fail::Immediately, Fail::Immediately)
                        .and_then(|transaction| transaction.commit(Some(committer)));
                    (result.is_ok(), format!("{result:?}"))
                } else {
                    let status: i32 = std::fs::read_to_string(directory.join("status"))?.trim().parse()?;
                    (
                        status == 0,
                        String::from_utf8_lossy(&std::fs::read(directory.join("stderr"))?).into_owned(),
                    )
                };
                assert_eq!(success, succeeds, "recorded outcome: {context}\n{diagnostic}");

                if succeeds {
                    expected.refs.remove(&source).expect("source exists before replacement");
                    expected.refs.insert(destination.clone(), Target::Object(commit_id));
                    let source_log = expected.logs.remove(&source).unwrap_or_default();
                    assert_eq!(
                        source_log.len(),
                        if initial_logs == "normal" { 2 } else { 0 },
                        "the source has the requested history: {context}"
                    );
                    let mut entry = Line {
                        previous_oid: crate::fixture_hash_kind().null(),
                        new_oid: commit_id,
                        signature: committer.to_owned()?,
                        message: "rename".into(),
                    };
                    let destination_log = if operation == "branch" {
                        entry.previous_oid = commit_id;
                        entry.message = format!("Branch: renamed {source} to {destination}").into();
                        expected
                            .refs
                            .insert("HEAD".into(), Target::Symbolic(destination.as_str().try_into()?));
                        let head_log = expected.logs.entry("HEAD".into()).or_default();
                        let mut deletion = entry.clone();
                        deletion.new_oid = crate::fixture_hash_kind().null();
                        head_log.push(deletion);
                        let mut creation = entry.clone();
                        creation.previous_oid = crate::fixture_hash_kind().null();
                        head_log.push(creation);
                        let mut history = source_log;
                        history.push(entry);
                        history
                    } else {
                        // Delete/create loses the source history and leaves HEAD pointing at the deleted branch.
                        let mut deletion = entry.clone();
                        deletion.previous_oid = commit_id;
                        deletion.new_oid = crate::fixture_hash_kind().null();
                        expected.logs.entry("HEAD".into()).or_default().push(deletion);
                        vec![entry]
                    };
                    expected.logs.insert(destination.clone(), destination_log);
                }
                assert_eq!(
                    observe(&store, &source, &destination)?,
                    expected,
                    "record refs, HEAD and full reflogs, including unchanged state on rejection: {context}"
                );
            }
        }
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct State {
    refs: BTreeMap<String, Target>,
    logs: BTreeMap<String, Vec<Line>>,
}

fn observe(store: &Store, source: &str, destination: &str) -> TestResult<State> {
    let refs = store
        .iter()?
        .all()?
        .map(|reference| reference.map(|reference| reference.name.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    observe_names(
        store,
        refs.iter().map(String::as_str).chain([source, destination, "HEAD"]),
    )
}

fn observe_names<'a>(store: &Store, names: impl IntoIterator<Item = &'a str>) -> TestResult<State> {
    let mut refs = BTreeMap::new();
    let mut logs = BTreeMap::new();
    for name in names {
        if let Some(reference) = store.try_find(name)? {
            refs.insert(name.to_owned(), reference.target);
        }
        let mut buffer = Vec::new();
        if store.reflog_exists(name)? {
            let lines = store
                .reflog_iter(name, &mut buffer)?
                .expect("the observed reflog exists")
                .map(|line| line.map(Line::from))
                .collect::<Result<Vec<_>, _>>()?;
            logs.insert(name.to_owned(), lines);
        }
    }
    Ok(State { refs, logs })
}
