use std::collections::BTreeMap;

use gix_lock::acquire::Fail;
use gix_object::bstr::BStr;
use gix_ref::{
    Target,
    file::Store,
    log::Line,
    transaction::{Change, PreviousValue, RefEdit},
};
use gix_testtools::{Creation, TestResult, scripted_fixture_read_only_with_args, scripted_fixture_writable_with_args};

const SCRIPT: &str = "make_ref_rename.sh";

#[test]
fn destination_constraints_are_checked_before_changing_the_source_or_logs() -> TestResult {
    for storage in ["loose", "packed", "both"] {
        for (state, constraint, succeeds) in [
            ("plain", "missing", true),
            ("destination", "missing", false),
            ("destination-same-target", "missing", false),
            ("symbolic-destination", "missing", false),
            ("broken-destination", "missing", false),
            ("destination", "matching", true),
            ("destination", "mismatching", false),
            ("plain", "optional-matching", true),
            ("destination", "optional-matching", true),
            ("plain", "existing", false),
            ("destination", "existing", true),
        ] {
            let (_keep, store) = writable("topic", "renamed", storage, state, "detached")?;
            let before = observe(&store, "refs/heads/topic", "refs/heads/renamed")?;
            let destination_target = before.refs.get("refs/heads/renamed");
            let expected = match constraint {
                "missing" => PreviousValue::MustNotExist,
                "existing" => PreviousValue::MustExist,
                "matching" => PreviousValue::MustExistAndMatch(destination_target.expect("destination exists").clone()),
                "mismatching" => PreviousValue::MustExistAndMatch(before.refs["refs/heads/topic"].clone()),
                "optional-matching" => PreviousValue::ExistingMustMatch(
                    destination_target.unwrap_or(&before.refs["refs/heads/topic"]).clone(),
                ),
                _ => unreachable!("test constraints are exhaustive"),
            };
            let result = store.rename(
                "refs/heads/topic".try_into()?,
                "refs/heads/renamed".try_into()?,
                expected,
                Some(committer()?),
                BStr::new("rename"),
                Fail::Immediately,
                Fail::Immediately,
            );
            assert_eq!(result.is_ok(), succeeds, "{state}, {storage}, {constraint}: {result:?}");
            if succeeds {
                assert!(
                    store.try_find("refs/heads/topic")?.is_none(),
                    "the accepted rename removes its source"
                );
                assert_eq!(
                    store.find("refs/heads/renamed")?.target,
                    before.refs["refs/heads/topic"],
                    "the accepted rename preserves the source target"
                );
            } else {
                assert_eq!(
                    observe(&store, "refs/heads/topic", "refs/heads/renamed")?,
                    before,
                    "{state}, {storage}, {constraint}: reject before altering refs or reflogs"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn directory_file_conflicts_and_packed_sources_match_git() -> TestResult {
    for storage in ["loose", "packed", "both"] {
        for (source, destination) in [
            ("topic", "renamed"),
            ("topic", "Topic"),
            ("topic", "topic/child"),
            ("topic/child", "topic"),
            ("topic/nested/child", "topic"),
            ("topic", "topic"),
        ] {
            assert_matches_git(source, destination, storage, "plain")?;
        }
    }
    Ok(())
}

#[test]
fn destination_replacement_and_reflog_history_match_git() -> TestResult {
    for state in [
        "destination",
        "stale-destination-log",
        "no-source-log",
        "no-source-log-destination",
        "no-source-log-stale-destination-log",
        "no-logs",
        "empty-source-log-directory",
        "empty-source-log-directory-destination",
    ] {
        for storage in ["loose", "packed", "both"] {
            assert_matches_git("topic", "renamed", storage, state)?;
        }
    }
    Ok(())
}

#[test]
fn reflog_history_is_preserved_byte_for_byte_even_when_entries_are_malformed() -> TestResult {
    let (_keep, store) = writable("topic", "renamed", "loose", "opaque-source-log", "detached")?;
    let baseline = scripted_fixture_read_only_with_args(
        SCRIPT,
        ["topic", "renamed", "loose", "opaque-source-log", "detached", "git"],
    )?;
    assert_eq!(
        std::fs::read_to_string(baseline.join("status"))?.trim(),
        "0",
        "Git renames a ref with opaque history"
    );
    let history = std::fs::read(store.git_dir().join("logs/refs/heads/topic"))?;
    rename(&store, "refs/heads/topic", "refs/heads/renamed")?;
    let actual = std::fs::read(store.git_dir().join("logs/refs/heads/renamed"))?;
    assert!(
        actual.starts_with(&history),
        "preserve every source reflog byte in its original order"
    );
    assert_eq!(
        actual,
        std::fs::read(baseline.join(".git/logs/refs/heads/renamed"))?,
        "source history and appended rename entry exactly match Git"
    );
    Ok(())
}

#[test]
fn reflog_messages_normalize_ascii_whitespace_and_preserve_other_bytes() -> TestResult {
    for (message, expected) in [
        (
            b" \t first\nsecond\r message \t".as_slice(),
            b"first second message".as_slice(),
        ),
        (b"\t\xff\xc2\xa0 text\n".as_slice(), b"\xff\xc2\xa0 text".as_slice()),
    ] {
        let (_keep, store) = writable("topic", "renamed", "loose", "plain", "detached")?;
        let mut history = reflog(&store, "refs/heads/topic")?.expect("source has history");
        let commit_id = store.find("refs/heads/topic")?.target.id().to_owned();
        let mut line = rename_line(commit_id, "refs/heads/topic", "refs/heads/renamed")?;
        line.message = expected.into();
        history.push(line);
        store.rename(
            "refs/heads/topic".try_into()?,
            "refs/heads/renamed".try_into()?,
            PreviousValue::Any,
            Some(committer()?),
            BStr::new(message),
            Fail::Immediately,
            Fail::Immediately,
        )?;
        assert_eq!(
            reflog(&store, "refs/heads/renamed")?,
            Some(history),
            "normalize only ASCII whitespace in the appended message"
        );
    }
    Ok(())
}

#[test]
fn invalid_log_inputs_are_rejected_before_changing_refs_or_history() -> TestResult {
    for (committer, message) in [(None, b"rename".as_slice()), (Some(committer()?), b"a\0b".as_slice())] {
        let (_keep, store) = writable("topic", "renamed", "loose", "plain", "detached")?;
        let before = observe(&store, "refs/heads/topic", "refs/heads/renamed")?;
        assert!(
            store
                .rename(
                    "refs/heads/topic".try_into()?,
                    "refs/heads/renamed".try_into()?,
                    PreviousValue::Any,
                    committer,
                    BStr::new(message),
                    Fail::Immediately,
                    Fail::Immediately,
                )
                .is_err(),
            "reject a missing required identity or an embedded NUL"
        );
        assert_eq!(
            observe(&store, "refs/heads/topic", "refs/heads/renamed")?,
            before,
            "invalid input preserves the entire original state"
        );
    }
    let (_keep, store) = writable("topic", "renamed", "loose", "tag", "detached")?;
    store.rename(
        "refs/tags/topic".try_into()?,
        "refs/tags/renamed".try_into()?,
        PreviousValue::Any,
        None,
        BStr::new("rename"),
        Fail::Immediately,
        Fail::Immediately,
    )?;
    assert!(
        !store.reflog_exists("refs/tags/renamed")?,
        "an identity is optional when no log is written"
    );
    Ok(())
}

#[test]
fn namespace_conflicts_and_invalid_sources_leave_the_store_unchanged() -> TestResult {
    for storage in ["loose", "packed"] {
        for (source, destination, state) in [
            ("topic", "renamed", "missing-source"),
            ("topic", "renamed", "symbolic-source"),
            ("topic", "renamed", "broken-source"),
            ("topic/child", "topic", "sibling"),
            ("topic", "renamed/child", "destination-parent"),
            ("topic", "renamed", "destination-child"),
        ] {
            let (_keep, store) = writable(source, destination, storage, state, "detached")?;
            let source = format!("refs/heads/{source}");
            let destination = format!("refs/heads/{destination}");
            let before = observe(&store, &source, &destination)?;
            assert!(
                rename(&store, &source, &destination).is_err(),
                "{state}, {storage}: reject the rename"
            );
            assert_eq!(
                observe(&store, &source, &destination)?,
                before,
                "{state}, {storage}: a rejected rename preserves refs and complete reflogs"
            );
        }
    }
    Ok(())
}

#[test]
fn symbolic_destinations_are_replaced_without_dereferencing() -> TestResult {
    for storage in ["loose", "packed", "both"] {
        for state in [
            "symbolic-destination",
            "broken-destination",
            "cyclic-destination",
            "malformed-target-destination",
        ] {
            let (_keep, store) = writable("topic", "renamed", storage, state, "detached")?;
            let baseline =
                scripted_fixture_read_only_with_args(SCRIPT, ["topic", "renamed", storage, state, "detached", "git"])?;
            assert_eq!(
                std::fs::read_to_string(baseline.join("status"))?.trim(),
                "0",
                "Git replaces the destination without following its target: {state}, {storage}"
            );
            let names = ["refs/heads/topic", "refs/heads/renamed", "refs/heads/other", "HEAD"];
            let before = observe_names(&store, names)?;
            let edits = rename(&store, "refs/heads/topic", "refs/heads/renamed")?;
            assert_outcome(&edits, &before, "refs/heads/topic", "refs/heads/renamed");
            let baseline = Store::at(baseline.join(".git"), crate::fixture_hash_kind());
            assert_eq!(
                observe_names(&store, names)?,
                observe_names(&baseline, names)?,
                "replace the destination and preserve source history exactly like Git: {state}, {storage}"
            );
            for name in ["broken", "cycle"] {
                let path = format!("refs/heads/{name}");
                if baseline.git_dir().join(&path).exists() {
                    assert_eq!(
                        std::fs::read(store.git_dir().join(&path))?,
                        std::fs::read(baseline.git_dir().join(path))?,
                        "preserve the unresolvable target's raw contents: {state}, {storage}"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn malformed_destination_is_replaced_with_source_history() -> TestResult {
    let (_keep, store) = writable("topic", "renamed", "loose", "malformed-destination", "detached")?;
    let target = store.find("refs/heads/topic")?.target;
    let mut history = reflog(&store, "refs/heads/topic")?.expect("source has history");
    history.push(rename_line(
        target.id().to_owned(),
        "refs/heads/topic",
        "refs/heads/renamed",
    )?);
    rename(&store, "refs/heads/topic", "refs/heads/renamed")?;
    assert!(store.try_find("refs/heads/topic")?.is_none(), "remove the source");
    assert_eq!(
        store.find("refs/heads/renamed")?.target,
        target,
        "replace invalid destination contents"
    );
    assert_eq!(
        reflog(&store, "refs/heads/renamed")?,
        Some(history),
        "preserve source history and append the rename"
    );
    Ok(())
}

#[test]
fn malformed_loose_destination_preserves_its_hidden_packed_ref_like_git() -> TestResult {
    for storage in ["packed", "both"] {
        for state in [
            "malformed-packed-destination",
            "no-source-log-malformed-packed-destination",
            "malformed-packed-destination-same-target",
            "no-source-log-malformed-packed-destination-same-target",
        ] {
            let (_keep, store) = writable("topic", "renamed", storage, state, "detached")?;
            let baseline =
                scripted_fixture_read_only_with_args(SCRIPT, ["topic", "renamed", storage, state, "detached", "git"])?;
            assert_eq!(
                std::fs::read_to_string(baseline.join("status"))?.trim(),
                "0",
                "Git replaces a malformed loose overlay: {storage}, {state}"
            );
            let packed_destination_id = store
                .open_packed_buffer()?
                .expect("packed refs exist")
                .find("refs/heads/renamed")?
                .target();
            rename(&store, "refs/heads/topic", "refs/heads/renamed")?;
            assert_eq!(
                store
                    .open_packed_buffer()?
                    .expect("packed destination survives")
                    .find("refs/heads/renamed")?
                    .target(),
                packed_destination_id,
                "retain the old packed destination under the replacement loose ref"
            );
            let baseline = Store::at(baseline.join(".git"), crate::fixture_hash_kind());
            assert_eq!(
                observe(&store, "refs/heads/topic", "refs/heads/renamed")?,
                observe(&baseline, "refs/heads/topic", "refs/heads/renamed")?,
                "{storage}, {state}: source history replaces destination history only when it exists"
            );
        }
    }
    Ok(())
}

#[test]
fn malformed_refs_are_checked_for_name_conflicts_without_parsing_their_targets() -> TestResult {
    for (source, destination, state, malformed_ref, succeeds) in [
        (
            "topic",
            "renamed/child",
            "malformed-parent",
            "refs/heads/renamed",
            false,
        ),
        (
            "topic",
            "renamed",
            "malformed-child",
            "refs/heads/renamed/blocker",
            false,
        ),
        (
            "topic/child",
            "topic",
            "malformed-child",
            "refs/heads/topic/blocker",
            false,
        ),
        ("topic", "renamed", "malformed-unrelated", "refs/heads/unrelated", true),
    ] {
        let (_keep, store) = writable(source, destination, "loose", state, "source")?;
        let source = format!("refs/heads/{source}");
        let destination = format!("refs/heads/{destination}");
        let target = store.find(source.as_str())?.target;
        let source_history = reflog(&store, &source)?;
        let head_history = reflog(&store, "HEAD")?;
        let result = rename(&store, &source, &destination);
        assert_eq!(
            result.is_ok(),
            succeeds,
            "{state}, {source} -> {destination}: {result:?}"
        );
        assert_eq!(
            std::fs::read(store.git_dir().join(malformed_ref))?,
            b"broken\n",
            "preserve the malformed ref's bytes"
        );
        if succeeds {
            assert!(
                store.try_find(source.as_str())?.is_none(),
                "delete the source despite an unrelated malformed ref"
            );
            assert_eq!(
                store.find(destination.as_str())?.target,
                target,
                "preserve the renamed target"
            );
        } else {
            assert_eq!(
                store.find(source.as_str())?.target,
                target,
                "reject namespace conflicts before deleting the source"
            );
            assert_eq!(
                reflog(&store, &source)?,
                source_history,
                "preserve complete source history"
            );
            assert_eq!(
                reflog(&store, "HEAD")?,
                head_history,
                "reject namespace conflicts before logging a HEAD deletion"
            );
        }
    }
    Ok(())
}

#[test]
fn head_is_not_retargeted_but_its_reflog_records_reference_deletion() -> TestResult {
    // These are refs_rename_ref() results, before porcelain retargets HEAD.
    for (destination, state, head, deletion) in [
        ("renamed", "plain", "source", Some("refs/heads/topic")),
        ("topic", "plain", "source", Some("refs/heads/topic")),
        ("renamed", "destination", "destination", Some("refs/heads/renamed")),
        (
            "renamed",
            "symbolic-destination",
            "destination",
            Some("refs/heads/other"),
        ),
        ("renamed", "broken-destination", "destination", Some("-")),
        ("renamed", "plain", "destination", None),
    ] {
        let (_keep, store) = writable("topic", destination, "loose", state, head)?;
        let destination = format!("refs/heads/{destination}");
        let before = observe(&store, "refs/heads/topic", &destination)?;
        let mut expected_head_log = before.logs.get("HEAD").cloned().unwrap_or_default();
        if let Some(deleted) = deletion {
            let previous = if deleted == "-" {
                crate::fixture_hash_kind().null()
            } else {
                before.refs[deleted].id().to_owned()
            };
            let mut entry = rename_line(previous, "refs/heads/topic", &destination)?;
            entry.new_oid = crate::fixture_hash_kind().null();
            if head != "source" {
                entry.message.clear();
            }
            expected_head_log.push(entry);
        }
        rename(&store, "refs/heads/topic", &destination)?;
        assert_eq!(
            store.find("HEAD")?.target,
            before.refs["HEAD"],
            "HEAD keeps its symbolic target"
        );
        assert_eq!(
            reflog(&store, "HEAD")?.unwrap_or_default(),
            expected_head_log,
            "HEAD={head}, destination={destination}: log only the applicable reference deletion"
        );
    }
    Ok(())
}

#[test]
fn namespaces_apply_to_refs_reflogs_and_packed_deletion() -> TestResult {
    for storage in ["loose", "packed", "both"] {
        for (source, destination) in [("topic", "topic/child"), ("topic/child", "topic")] {
            let (_keep, mut store) = writable(source, destination, storage, "namespace", "detached")?;
            let source = format!("refs/heads/{source}");
            let destination = format!("refs/heads/{destination}");
            let outer_target = store.find(source.as_str())?.target;
            let outer_log = reflog(&store, &source)?;
            store.namespace = Some(gix_ref::namespace::expand("scope/nested")?);
            let source_target = store.find(source.as_str())?.target;
            let mut expected = reflog(&store, &source)?.expect("namespaced source has history");
            expected.push(rename_line(source_target.id().to_owned(), &source, &destination)?);
            rename(&store, &source, &destination)?;
            assert!(
                store.try_find(source.as_str())?.is_none(),
                "remove the namespaced source, including packed refs"
            );
            assert_eq!(
                store.find(destination.as_str())?.target,
                source_target,
                "preserve the namespaced target"
            );
            assert_eq!(
                reflog(&store, &destination)?,
                Some(expected),
                "move the namespaced reflog in order"
            );
            store.namespace = None;
            assert_eq!(store.find(source.as_str())?.target, outer_target, "outer refs survive");
            assert_eq!(reflog(&store, &source)?, outer_log, "outer reflogs survive");
        }
    }
    Ok(())
}

#[test]
fn disabled_reflog_writes_preserve_history_without_adding_an_entry() -> TestResult {
    for state in ["plain", "no-logs"] {
        let (_keep, mut store) = writable("topic", "renamed", "loose", state, "detached")?;
        let history = reflog(&store, "refs/heads/topic")?;
        store.write_reflog = gix_ref::store::WriteReflog::Disable;
        rename(&store, "refs/heads/topic", "refs/heads/renamed")?;
        assert_eq!(
            reflog(&store, "refs/heads/renamed")?,
            history,
            "the store's write policy suppresses new log entries"
        );
    }
    Ok(())
}

#[test]
fn tags_obey_reflog_creation_policy_and_preserve_existing_history() -> TestResult {
    for (state, policy) in [
        ("tag", gix_ref::store::WriteReflog::Normal),
        ("tag", gix_ref::store::WriteReflog::Always),
        ("tag-log", gix_ref::store::WriteReflog::Normal),
    ] {
        let (_keep, mut store) = writable("topic", "renamed", "loose", state, "detached")?;
        let target = store.find("refs/tags/topic")?.target;
        let mut expected = reflog(&store, "refs/tags/topic")?;
        if policy == gix_ref::store::WriteReflog::Always || expected.is_some() {
            expected.get_or_insert_with(Vec::new).push(rename_line(
                target.id().to_owned(),
                "refs/tags/topic",
                "refs/tags/renamed",
            )?);
        }
        store.write_reflog = policy;
        rename(&store, "refs/tags/topic", "refs/tags/renamed")?;
        assert_eq!(
            store.find("refs/tags/renamed")?.target,
            target,
            "preserve a non-branch target"
        );
        assert_eq!(
            reflog(&store, "refs/tags/renamed")?,
            expected,
            "{state}, {policy:?}: preserve history and obey log creation policy"
        );
    }
    Ok(())
}

#[test]
fn linked_worktree_uses_shared_refs_and_its_private_head_reflog() -> TestResult {
    let (_keep, main) = writable("topic", "renamed", "packed", "worktree", "detached")?;
    let store = Store::for_linked_worktree(
        main.git_dir().join("worktrees/linked"),
        main.git_dir().to_owned(),
        crate::fixture_hash_kind(),
    );
    let target = store.find("refs/heads/topic")?.target;
    let mut history = reflog(&store, "refs/heads/topic")?.expect("shared source has history");
    let mut head_history = reflog(&store, "HEAD")?.expect("private HEAD has history");
    let main_head_history = reflog(&main, "HEAD")?;
    let entry = rename_line(target.id().to_owned(), "refs/heads/topic", "refs/heads/renamed")?;
    history.push(entry.clone());
    let mut deletion = entry;
    deletion.new_oid = crate::fixture_hash_kind().null();
    head_history.push(deletion);
    rename(&store, "refs/heads/topic", "refs/heads/renamed")?;
    assert!(
        main.try_find("refs/heads/topic")?.is_none(),
        "remove the packed source from the common directory"
    );
    assert_eq!(
        main.find("refs/heads/renamed")?.target,
        target,
        "create the destination in the common directory"
    );
    assert_eq!(
        reflog(&main, "refs/heads/renamed")?,
        Some(history),
        "move the shared source history"
    );
    assert_eq!(
        store.find("HEAD")?.target,
        Target::Symbolic("refs/heads/topic".try_into()?),
        "leave the worktree HEAD target for porcelain"
    );
    assert_eq!(
        reflog(&store, "HEAD")?,
        Some(head_history),
        "log the deletion in the private HEAD log"
    );
    assert_eq!(
        reflog(&main, "HEAD")?,
        main_head_history,
        "the main worktree HEAD log is unchanged"
    );
    Ok(())
}

#[test]
fn lock_failures_preserve_references_history_and_other_writers_locks() -> TestResult {
    for (state, storage, lock, head) in [
        ("source-lock", "loose", "refs/heads/topic.lock", "detached"),
        ("destination-lock", "loose", "refs/heads/renamed.lock", "detached"),
        ("packed-lock", "packed", "packed-refs.lock", "detached"),
        ("head-lock", "loose", "HEAD.lock", "source"),
    ] {
        let (_keep, store) = writable("topic", "renamed", storage, state, head)?;
        let before = observe(&store, "refs/heads/topic", "refs/heads/renamed")?;
        assert!(
            rename(&store, "refs/heads/topic", "refs/heads/renamed").is_err(),
            "{state}: another writer owns the lock"
        );
        assert_eq!(
            observe(&store, "refs/heads/topic", "refs/heads/renamed")?,
            before,
            "{state}: recover the original ref and log"
        );
        assert_eq!(
            std::fs::read(store.git_dir().join(lock))?,
            b"preserve this lock\n",
            "{state}: never remove another writer's lock"
        );
    }
    Ok(())
}

#[test]
fn a_reflog_move_failure_restores_the_source() -> TestResult {
    for storage in ["loose", "packed", "both"] {
        let (_keep, store) = writable("topic", "renamed", storage, "reflog-directory", "detached")?;
        let target = store.find("refs/heads/topic")?.target;
        let source_history = reflog(&store, "refs/heads/topic")?;
        let blocker = std::fs::read(store.git_dir().join("logs/refs/heads/renamed/child"))?;
        assert!(
            rename(&store, "refs/heads/topic", "refs/heads/renamed").is_err(),
            "a surviving child reflog cannot be overwritten"
        );
        assert_eq!(
            store.find("refs/heads/topic")?.target,
            target,
            "{storage}: recover the source target"
        );
        assert_eq!(
            reflog(&store, "refs/heads/topic")?,
            source_history,
            "{storage}: recover complete source history"
        );
        assert!(
            store.try_find("refs/heads/renamed")?.is_none(),
            "the destination was not created"
        );
        assert_eq!(
            std::fs::read(store.git_dir().join("logs/refs/heads/renamed/child"))?,
            blocker,
            "the blocking reflog survives"
        );
    }
    Ok(())
}

#[test]
fn final_reference_publication_failure_restores_the_source_and_its_exact_history() -> TestResult {
    for storage in ["loose", "packed", "both"] {
        let (_keep, store) = writable("topic/child", "topic", storage, "publication-blocker", "detached")?;
        let target = store.find("refs/heads/topic/child")?.target;
        let history = std::fs::read(store.git_dir().join("logs/refs/heads/topic/child"))?;
        let err = rename(&store, "refs/heads/topic/child", "refs/heads/topic")
            .expect_err("the surviving invalid ref name blocks final publication");
        assert!(
            format!("{err:?}").contains("Could not commit reference"),
            "{storage}: failure occurs during final reference publication: {err:?}"
        );
        assert_eq!(
            store.find("refs/heads/topic/child")?.target,
            target,
            "restore the source after destination log installation"
        );
        assert_eq!(
            std::fs::read(store.git_dir().join("logs/refs/heads/topic/child"))?,
            history,
            "restore exact source log bytes without retaining the failed rename entry"
        );
        assert_eq!(
            std::fs::read(store.git_dir().join("refs/heads/topic/.keep"))?,
            b"preserve this blocker\n",
            "never delete the blocking file"
        );
        assert!(
            !store.git_dir().join("logs/refs/heads/topic").is_file(),
            "remove the installed destination log so the source log directory can be restored"
        );
    }
    Ok(())
}

#[test]
fn source_reflog_directory_failures_preserve_the_source_and_destination_logs() -> TestResult {
    for state in [
        "empty-source-log-directory-stale-destination-log",
        "nonempty-source-log-directory",
    ] {
        let (_keep, store) = writable("topic", "renamed", "loose", state, "detached")?;
        let baseline =
            scripted_fixture_read_only_with_args(SCRIPT, ["topic", "renamed", "loose", state, "detached", "git"])?;
        assert_ne!(
            std::fs::read_to_string(baseline.join("status"))?.trim(),
            "0",
            "Git rejects {state}"
        );
        let target = store.find("refs/heads/topic")?.target;
        let destination_history = reflog(&store, "refs/heads/renamed")?;
        let source_log = store.git_dir().join("logs/refs/heads/topic");
        let child_history = if state == "nonempty-source-log-directory" {
            Some(std::fs::read(source_log.join("child"))?)
        } else {
            None
        };
        assert!(
            rename(&store, "refs/heads/topic", "refs/heads/renamed").is_err(),
            "{state}: preserve incompatible reflog paths"
        );
        assert_eq!(
            store.find("refs/heads/topic")?.target,
            target,
            "{state}: restore the source target"
        );
        assert_eq!(
            reflog(&store, "refs/heads/renamed")?,
            destination_history,
            "{state}: retain the old destination history"
        );
        assert!(source_log.is_dir(), "{state}: restore the source log directory");
        if let Some(history) = child_history {
            assert_eq!(
                std::fs::read(source_log.join("child"))?,
                history,
                "restore every source log directory entry"
            );
        } else {
            assert_eq!(
                std::fs::read_dir(source_log)?.count(),
                0,
                "the restored log directory remains empty"
            );
        }
    }
    Ok(())
}

#[test]
fn disabled_reflog_writes_move_a_nonempty_source_log_directory() -> TestResult {
    let (_keep, mut store) = writable("topic", "renamed", "loose", "nonempty-source-log-directory", "detached")?;
    let target = store.find("refs/heads/topic")?.target;
    let history = std::fs::read(store.git_dir().join("logs/refs/heads/topic/child"))?;
    store.write_reflog = gix_ref::store::WriteReflog::Disable;
    rename(&store, "refs/heads/topic", "refs/heads/renamed")?;
    assert!(
        store.try_find("refs/heads/topic")?.is_none(),
        "remove the source reference"
    );
    assert!(
        !store.git_dir().join("logs/refs/heads/topic").exists(),
        "remove the original log directory"
    );
    assert_eq!(
        store.find("refs/heads/renamed")?.target,
        target,
        "preserve the source target"
    );
    assert_eq!(
        std::fs::read(store.git_dir().join("logs/refs/heads/renamed/child"))?,
        history,
        "preserve source log directory contents without appending an entry"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn source_reflog_symlinks_are_rejected_without_following_them() -> TestResult {
    let (_keep, store) = writable("topic", "renamed", "loose", "plain", "detached")?;
    let source_log = store.git_dir().join("logs/refs/heads/topic");
    let history_file = source_log.with_file_name("source-history");
    let history = std::fs::read(&source_log)?;
    // CopyFromReadOnly dereferences symlinks, so install this filesystem blocker after copying.
    std::fs::rename(&source_log, &history_file)?;
    std::os::unix::fs::symlink("source-history", &source_log)?;
    let before = observe(&store, "refs/heads/topic", "refs/heads/renamed")?;
    assert!(
        rename(&store, "refs/heads/topic", "refs/heads/renamed").is_err(),
        "Git rejects a source reflog symlink"
    );
    assert_eq!(
        observe(&store, "refs/heads/topic", "refs/heads/renamed")?,
        before,
        "preserve refs and original history"
    );
    assert_eq!(
        std::fs::read_link(source_log)?,
        std::path::Path::new("source-history"),
        "leave the symlink untouched"
    );
    assert_eq!(
        std::fs::read(history_file)?,
        history,
        "never write through the source reflog symlink"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn destination_reflog_symlinks_match_gits_history_replacement_rules() -> TestResult {
    for state in [
        "stale-destination-log",
        "no-source-log-stale-destination-log",
        "no-source-log-destination",
    ] {
        let (_keep, store) = writable("topic", "renamed", "loose", state, "detached")?;
        let destination_log = store.git_dir().join("logs/refs/heads/renamed");
        let history_file = destination_log.with_file_name("destination-history");
        let original_bytes = std::fs::read(&destination_log)?;
        let source_history = reflog(&store, "refs/heads/topic")?;
        let follow_symlink = state == "no-source-log-stale-destination-log";
        let mut expected = source_history.unwrap_or(if follow_symlink {
            reflog(&store, "refs/heads/renamed")?.expect("stale destination has history")
        } else {
            Vec::new()
        });
        expected.push(rename_line(
            store.find("refs/heads/topic")?.target.id().to_owned(),
            "refs/heads/topic",
            "refs/heads/renamed",
        )?);
        // CopyFromReadOnly dereferences symlinks, so recreate only the link after copying.
        std::fs::rename(&destination_log, &history_file)?;
        std::os::unix::fs::symlink("destination-history", &destination_log)?;
        rename(&store, "refs/heads/topic", "refs/heads/renamed")?;
        assert_eq!(
            reflog(&store, "refs/heads/renamed")?,
            Some(expected),
            "{state}: preserve the applicable history"
        );
        assert_eq!(
            std::fs::symlink_metadata(&destination_log)?.is_symlink(),
            follow_symlink,
            "{state}: only append through a stale log when neither source history nor a destination ref replaces it"
        );
        if !follow_symlink {
            assert_eq!(
                std::fs::read(history_file)?,
                original_bytes,
                "{state}: replacing the symlink does not modify its target"
            );
        }
    }
    Ok(())
}

fn assert_matches_git(source: &str, destination: &str, storage: &str, state: &str) -> TestResult {
    let (_keep, store) = writable(source, destination, storage, state, "detached")?;
    let baseline =
        scripted_fixture_read_only_with_args(SCRIPT, [source, destination, storage, state, "detached", "git"])?;
    assert_eq!(
        std::fs::read_to_string(baseline.join("status"))?.trim(),
        "0",
        "Git baseline succeeds: {source} -> {destination}, {storage}, {state}"
    );
    let baseline = Store::at(baseline.join(".git"), crate::fixture_hash_kind());
    let source = format!("refs/heads/{source}");
    let destination = format!("refs/heads/{destination}");
    let mut before = observe(&store, &source, &destination)?;
    if let Some(reference) = store.try_find(destination.as_str())? {
        before.refs.insert(destination.clone(), reference.target);
    }
    let edits = rename(&store, &source, &destination)?;
    assert_outcome(&edits, &before, &source, &destination);
    assert_eq!(
        observe(&store, &source, &destination)?,
        observe(&baseline, &source, &destination)?,
        "Git-compatible refs and complete, ordered reflogs: {source} -> {destination}, {storage}, {state}"
    );
    Ok(())
}

fn assert_outcome(edits: &[RefEdit], before: &State, source: &str, destination: &str) {
    assert_eq!(edits.len(), 2, "report the source deletion and destination update");
    assert_eq!(edits[0].name.as_bstr(), source, "the deletion identifies the source");
    assert!(
        matches!(edits[0].change, Change::Delete { .. }),
        "report a deletion first"
    );
    assert_eq!(
        edits[0].change.previous_value(),
        Some(before.refs[source].to_ref()),
        "report the actual source target"
    );
    assert_eq!(
        edits[1].name.as_bstr(),
        destination,
        "the update identifies the destination"
    );
    assert_eq!(
        edits[1].change.previous_value(),
        before.refs.get(destination).map(Target::to_ref),
        "report the overwritten target, including symbolic refs and same-name renames"
    );
    assert_eq!(
        edits[1].change.new_value(),
        Some(before.refs[source].to_ref()),
        "report the preserved source target"
    );
    assert!(
        edits.iter().all(|edit| !edit.deref),
        "rename operates on these exact names"
    );
}

fn writable(
    source: &str,
    destination: &str,
    storage: &str,
    state: &str,
    head: &str,
) -> TestResult<(gix_testtools::tempfile::TempDir, Store)> {
    let directory = scripted_fixture_writable_with_args(
        SCRIPT,
        [source, destination, storage, state, head, "setup"],
        Creation::CopyFromReadOnly,
    )?;
    let store = Store::at(directory.path().join(".git"), crate::fixture_hash_kind());
    Ok((directory, store))
}

fn rename(store: &Store, source: &str, destination: &str) -> TestResult<Vec<RefEdit>> {
    let message = format!("Branch: renamed {source} to {destination}");
    Ok(store.rename(
        source.try_into()?,
        destination.try_into()?,
        PreviousValue::Any,
        Some(committer()?),
        BStr::new(&message),
        Fail::Immediately,
        Fail::Immediately,
    )?)
}

fn committer() -> TestResult<gix_actor::SignatureRef<'static>> {
    Ok(gix_actor::SignatureRef::from_bytes(
        b"committer <committer@example.com> 946771200 +0000",
    )?)
}

fn rename_line(commit_id: gix_hash::ObjectId, source: &str, destination: &str) -> TestResult<Line> {
    Ok(Line {
        previous_oid: commit_id,
        new_oid: commit_id,
        signature: committer()?.to_owned()?,
        message: format!("Branch: renamed {source} to {destination}").into(),
    })
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
        if let Some(history) = reflog(store, name)? {
            logs.insert(name.to_owned(), history);
        }
    }
    Ok(State { refs, logs })
}

fn reflog(store: &Store, name: &str) -> TestResult<Option<Vec<Line>>> {
    if !store.reflog_exists(name)? {
        return Ok(None);
    }
    let mut buffer = Vec::new();
    store
        .reflog_iter(name, &mut buffer)?
        .map(|lines| lines.map(|line| line.map(Line::from)).collect::<Result<_, _>>())
        .transpose()
        .map_err(Into::into)
}
