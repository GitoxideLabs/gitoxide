use std::{path::Path, sync::atomic::AtomicBool};

use gix::{
    ObjectId, Repository,
    bstr::ByteSlice,
    push::{Lease, Options, Status, Update},
};
use gix_testtools::TestResult;

fn object_id(root: &Path, name: &str) -> TestResult<ObjectId> {
    Ok(ObjectId::from_hex(
        std::fs::read_to_string(root.join(name))?.trim().as_bytes(),
    )?)
}

fn push(repo: &Repository, destination: &Path, object_id: ObjectId) -> (gix::Result<gix::push::Outcome>, Vec<u8>) {
    push_options(repo, destination, object_id, Options::default())
}

fn push_options(
    repo: &Repository,
    destination: &Path,
    object_id: ObjectId,
    options: Options<'_>,
) -> (gix::Result<gix::push::Outcome>, Vec<u8>) {
    let mut output = Vec::new();
    let result = repo
        .remote_at(gix::path::into_bstr(destination).expect("test path"))
        .and_then(|remote| {
            remote.push(
                [Update {
                    destination: "refs/heads/topic".try_into().expect("valid name"),
                    new_id: Some(object_id),
                    lease: Lease::Any,
                    force: false,
                }],
                options,
                gix::progress::Discard,
                |_, bytes| {
                    output.extend_from_slice(bytes);
                    Ok(())
                },
                &AtomicBool::default(),
            )
        });
    (result, output)
}

#[test]
fn receive_fsck_policy_matches_git_without_weakening_connectivity() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_receive_fsck.sh")?;
    let source = gix::open_opts(root.join("source"), gix::open::Options::isolated())?;
    for (case, object, config, accepted, diagnostic) in [
        ("default", "legacy", vec![], false, "missingEmail"),
        (
            "warning",
            "legacy",
            vec![("receive.fsck.missingEmail", "warn")],
            true,
            "missingEmail",
        ),
        (
            "ignore",
            "legacy",
            vec![("receive.fsck.missingEmail", "ignore")],
            true,
            "",
        ),
        (
            "no-general-fallback",
            "legacy",
            vec![("fsck.missingEmail", "ignore")],
            false,
            "missingEmail",
        ),
        (
            "disabled",
            "legacy",
            vec![("receive.fsckObjects", "false"), ("transfer.fsckObjects", "true")],
            true,
            "",
        ),
        ("valid", "valid", vec![], true, ""),
        ("strict-warning", "dot", vec![], false, "hasDot"),
        (
            "demoted-warning",
            "dot",
            vec![("receive.fsck.hasDot", "warn")],
            true,
            "hasDot",
        ),
        ("special-blob", "special", vec![], false, "gitmodulesUrl"),
    ] {
        let tip_id = object_id(&root, object)?;
        for native in [false, true] {
            let destination = gix_testtools::tempfile::tempdir()?;
            gix_testtools::git(destination.path(), "init --bare")?;
            gix_testtools::git(destination.path(), "config receive.fsckObjects true")?;
            for (key, value) in &config {
                let status = gix_testtools::git_command(destination.path())
                    .args(["config", key, value])
                    .status()?;
                assert!(status.success());
            }
            let output = if native {
                let (result, output) = push(&source, destination.path(), tip_id);
                let result = result?;
                assert_eq!(
                    matches!(result.updates[0].status, Status::Updated { .. }),
                    accepted,
                    "{case}"
                );
                if !accepted {
                    assert!(
                        result
                            .response
                            .as_ref()
                            .is_some_and(|report| report.unpack_error.is_some()),
                        "fsck rejects the complete incoming pack"
                    );
                }
                output
            } else {
                let output = gix_testtools::git_command(source.git_dir())
                    .arg("push")
                    .arg(destination.path())
                    .arg(format!("{tip_id}:refs/heads/topic"))
                    .output()?;
                assert_eq!(
                    output.status.success(),
                    accepted,
                    "Git baseline: {case}: {}",
                    output.stderr.as_bstr()
                );
                output.stderr
            };
            assert!(
                output.as_bstr().contains_str(diagnostic),
                "{case} native={native}: {}",
                output.as_bstr()
            );
            let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
            assert_eq!(receiver.try_find_reference("refs/heads/topic")?.is_some(), accepted);
            assert_eq!(
                receiver.has_object(tip_id),
                accepted,
                "rejected incoming objects remain quarantined: {case}"
            );
        }
    }
    Ok(())
}

#[test]
fn receive_fsck_skip_list_and_existing_special_blobs_match_git() -> TestResult {
    use gix::objs::{FindExt, Write};
    let root = gix_testtools::scripted_fixture_read_only("make_receive_fsck.sh")?;
    let source = gix::open_opts(root.join("source"), gix::open::Options::isolated())?;
    for native in [false, true] {
        for existing_special in [false, true] {
            let destination = gix_testtools::tempfile::tempdir()?;
            gix_testtools::git(destination.path(), "init --bare")?;
            gix_testtools::git(destination.path(), "config transfer.fsckObjects true")?;
            let tip_id = object_id(&root, if existing_special { "special" } else { "legacy" })?;
            if existing_special {
                let blob_id = object_id(&root, "modules")?;
                let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
                let mut buffer = Vec::new();
                let blob = source.find(&blob_id, &mut buffer)?;
                receiver.objects.write_buf(blob.kind, blob.data)?;
                let output = gix_testtools::git_command(destination.path())
                    .args(["update-ref", "refs/tags/existing", &blob_id.to_string()])
                    .output()?;
                assert!(output.status.success());
            } else {
                std::fs::write(
                    destination.path().join("fsck.skip"),
                    format!("# historical object\n{tip_id}\n"),
                )?;
                gix_testtools::git(destination.path(), "config receive.fsck.skipList fsck.skip")?;
            }
            let accepted = !existing_special;
            let output = if native {
                let (result, output) = push(&source, destination.path(), tip_id);
                assert_eq!(matches!(result?.updates[0].status, Status::Updated { .. }), accepted);
                output
            } else {
                let output = gix_testtools::git_command(source.git_dir())
                    .arg("push")
                    .arg(destination.path())
                    .arg(format!("{tip_id}:refs/heads/topic"))
                    .output()?;
                assert_eq!(output.status.success(), accepted, "{}", output.stderr.as_bstr());
                output.stderr
            };
            if existing_special {
                assert!(
                    output.as_bstr().contains_str("gitmodulesUrl"),
                    "existing blobs are checked when newly referenced by a special tree entry: {}",
                    output.as_bstr()
                );
            }
        }
    }
    Ok(())
}

#[test]
fn receive_fsck_config_validation_matches_git() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_receive_fsck.sh")?;
    let source = gix::open_opts(root.join("source"), gix::open::Options::isolated())?;
    let tip_id = object_id(&root, "valid")?;
    for (name, value, enabled, accepted, diagnostic) in [
        ("futureMessage", "nonsense", true, true, "skipping unknown msg id"),
        ("missingEmail", "nonsense", true, false, ""),
        ("missingEmail", "nonsense", false, false, ""),
        ("nulInHeader", "warn", true, false, ""),
        ("nulInHeader", "warn", false, true, ""),
        ("skipList", ":(optional)missing-list", true, true, ""),
        ("skipList", "missing-list", false, true, ""),
        ("skipList", "missing-list", true, false, ""),
    ] {
        for native in [false, true] {
            let destination = gix_testtools::tempfile::tempdir()?;
            gix_testtools::git(destination.path(), "init --bare")?;
            let output = gix_testtools::git_command(destination.path())
                .args(["config", "receive.fsckObjects", if enabled { "true" } else { "false" }])
                .output()?;
            assert!(output.status.success());
            let output = gix_testtools::git_command(destination.path())
                .args(["config", &format!("receive.fsck.{name}"), value])
                .output()?;
            assert!(output.status.success());
            let output = if native {
                let (result, output) = push(&source, destination.path(), tip_id);
                assert_eq!(
                    result.is_ok_and(|result| matches!(result.updates[0].status, Status::Updated { .. })),
                    accepted,
                    "{name}={value} enabled={enabled}"
                );
                output
            } else {
                let output = gix_testtools::git_command(source.git_dir())
                    .arg("push")
                    .arg(destination.path())
                    .arg(format!("{tip_id}:refs/heads/topic"))
                    .output()?;
                assert_eq!(
                    output.status.success(),
                    accepted,
                    "{name}={value} enabled={enabled}: {}",
                    output.stderr.as_bstr()
                );
                output.stderr
            };
            assert!(output.as_bstr().contains_str(diagnostic), "{}", output.as_bstr());
        }
    }
    Ok(())
}

#[test]
fn fsck_skip_list_does_not_allow_missing_connectivity() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_receive_fsck.sh")?;
    let source = gix::open_opts(root.join("source"), gix::open::Options::isolated())?;
    let tip_id = object_id(&root, "missing")?;
    let tree_id = object_id(&root, "missing-tree")?;
    for native in [false, true] {
        let destination = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(destination.path(), "init --bare")?;
        gix_testtools::git(destination.path(), "config receive.fsckObjects true")?;
        std::fs::write(destination.path().join("skip"), format!("{tip_id}\n{tree_id}\n"))?;
        gix_testtools::git(destination.path(), "config receive.fsck.skipList skip")?;
        if native {
            let (result, _) = push(&source, destination.path(), tip_id);
            result.expect_err("skipped structural validation does not imply connectivity");
        } else {
            let output = gix_testtools::git_command(source.git_dir())
                .arg("push")
                .arg(destination.path())
                .arg(format!("{tip_id}:refs/heads/topic"))
                .output()?;
            assert!(!output.status.success());
        }
        let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
        assert!(receiver.try_find_reference("refs/heads/topic")?.is_none());
        assert!(
            !receiver.has_object(tip_id),
            "incomplete incoming graphs are never migrated"
        );
    }
    Ok(())
}

#[test]
#[cfg(unix)]
fn fsck_rejection_precedes_hooks_and_quarantine_migration() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let root = gix_testtools::scripted_fixture_read_only("make_receive_fsck.sh")?;
    let source = gix::open_opts(root.join("source"), gix::open::Options::isolated())?;
    let tip_id = object_id(&root, "special")?;
    let tree_id = object_id(&root, "modules-tree")?;
    for native in [false, true] {
        let destination = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(destination.path(), "init --bare")?;
        gix_testtools::git(destination.path(), "config receive.fsckObjects true")?;
        // Skipping the tree's diagnostics must still discover the unsafe blob.
        std::fs::write(destination.path().join("skip"), format!("{tree_id}\n"))?;
        gix_testtools::git(destination.path(), "config receive.fsck.skipList skip")?;
        let hook = destination.path().join("hooks/pre-receive");
        std::fs::write(&hook, "#!/bin/sh\nprintf invoked >hook-ran\n")?;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
        if native {
            let (result, output) = push(&source, destination.path(), tip_id);
            assert!(!matches!(result?.updates[0].status, Status::Updated { .. }));
            assert!(output.as_bstr().contains_str("gitmodulesUrl"));
        } else {
            let output = gix_testtools::git_command(source.git_dir())
                .arg("push")
                .arg(destination.path())
                .arg(format!("{tip_id}:refs/heads/topic"))
                .output()?;
            assert!(!output.status.success(), "{}", output.stderr.as_bstr());
        }
        assert!(
            !destination.path().join("hook-ran").exists(),
            "fsck finishes before pre-receive"
        );
        let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
        assert!(!receiver.has_object(tip_id));
        assert!(!receiver.has_object(tree_id));
        assert!(
            !std::fs::read_dir(destination.path().join("objects"))?
                .any(|entry| entry.is_ok_and(|entry| entry.file_name().to_string_lossy().starts_with("incoming-"))),
            "quarantine is removed after fsck rejection"
        );
    }
    Ok(())
}

#[test]
fn receiver_ancestry_checks_honor_metadata_exemptions() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_receive_fsck.sh")?;
    let source = gix::open_opts(root.join("source"), gix::open::Options::isolated())?;
    let base_id = object_id(&root, "valid")?;
    let tip_id = object_id(&root, "legacy-child")?;
    for native in [false, true] {
        let destination = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(destination.path(), "init --bare")?;
        gix_testtools::git(destination.path(), "config receive.fsckObjects true")?;
        gix_testtools::git(destination.path(), "config receive.fsck.missingEmail ignore")?;
        gix_testtools::git(destination.path(), "config receive.denyNonFastForwards true")?;
        for object_id in [base_id, tip_id] {
            if native {
                let (result, _) = push(&source, destination.path(), object_id);
                assert!(matches!(result?.updates[0].status, Status::Updated { forced: false }));
            } else {
                let output = gix_testtools::git_command(source.git_dir())
                    .arg("push")
                    .arg(destination.path())
                    .arg(format!("{object_id}:refs/heads/topic"))
                    .output()?;
                assert!(output.status.success(), "{}", output.stderr.as_bstr());
            }
        }
    }
    Ok(())
}

#[test]
fn dry_run_does_not_load_the_receivers_fsck_skip_lists() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_receive_fsck.sh")?;
    let source = gix::open_opts(root.join("source"), gix::open::Options::isolated())?;
    let tip_id = object_id(&root, "valid")?;
    for native in [false, true] {
        let destination = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(destination.path(), "init --bare")?;
        gix_testtools::git(destination.path(), "config receive.fsckObjects true")?;
        gix_testtools::git(destination.path(), "config receive.fsck.skipList missing-file")?;
        gix_testtools::git(destination.path(), "config receive.fsck.nulInHeader warn")?;
        if native {
            let (result, _) = push_options(
                &source,
                destination.path(),
                tip_id,
                Options {
                    dry_run: true,
                    ..Options::default()
                },
            );
            assert!(matches!(result?.updates[0].status, Status::WouldUpdate { .. }));
        } else {
            let output = gix_testtools::git_command(source.git_dir())
                .args(["push", "--dry-run"])
                .arg(destination.path())
                .arg(format!("{tip_id}:refs/heads/topic"))
                .output()?;
            assert!(output.status.success(), "{}", output.stderr.as_bstr());
        }
        let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
        assert!(receiver.try_find_reference("refs/heads/topic")?.is_none());
        assert!(!receiver.has_object(tip_id));
    }
    Ok(())
}

#[test]
fn skip_lists_accumulate_and_any_fsck_error_rejects_all_commands() -> TestResult {
    let root = gix_testtools::scripted_fixture_read_only("make_receive_fsck.sh")?;
    let source = gix::open_opts(root.join("source"), gix::open::Options::isolated())?;
    let base_id = object_id(&root, "legacy")?;
    let tip_id = object_id(&root, "legacy-descendant")?;
    let valid_id = object_id(&root, "valid")?;
    for list_count in 0..=2 {
        for native in [false, true] {
            let destination = gix_testtools::tempfile::tempdir()?;
            gix_testtools::git(destination.path(), "init --bare")?;
            gix_testtools::git(destination.path(), "config receive.fsckObjects true")?;
            for (name, object_id) in [("first", base_id), ("second", tip_id)].into_iter().take(list_count) {
                std::fs::write(destination.path().join(name), format!("{object_id}\n"))?;
                let output = gix_testtools::git_command(destination.path())
                    .args(["config", "--add", "receive.fsck.skipList", name])
                    .output()?;
                assert!(output.status.success());
            }
            let accepted = list_count == 2;
            if native {
                let outcome = source.remote_at(gix::path::into_bstr(destination.path())?)?.push(
                    [("refs/heads/valid", valid_id), ("refs/heads/legacy", tip_id)].map(|(name, object_id)| Update {
                        destination: name.try_into().expect("valid test ref"),
                        new_id: Some(object_id),
                        lease: Lease::Any,
                        force: false,
                    }),
                    Options::default(),
                    gix::progress::Discard,
                    |_, _| Ok(()),
                    &AtomicBool::default(),
                )?;
                for update in outcome.updates {
                    assert_eq!(
                        matches!(update.status, Status::Updated { .. }),
                        accepted,
                        "fsck applies to the complete pack regardless of atomic mode"
                    );
                }
                assert_eq!(
                    outcome.response.expect("actual transfer").unpack_error.is_none(),
                    accepted
                );
            } else {
                let output = gix_testtools::git_command(source.git_dir())
                    .arg("push")
                    .arg(destination.path())
                    .args([
                        format!("{valid_id}:refs/heads/valid"),
                        format!("{tip_id}:refs/heads/legacy"),
                    ])
                    .output()?;
                assert_eq!(output.status.success(), accepted, "{}", output.stderr.as_bstr());
            }
            let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
            assert_eq!(receiver.try_find_reference("refs/heads/valid")?.is_some(), accepted);
            assert_eq!(receiver.try_find_reference("refs/heads/legacy")?.is_some(), accepted);
            assert_eq!(receiver.has_object(tip_id), accepted);
        }
    }
    Ok(())
}

#[test]
fn fsck_exempts_advertised_history_instead_of_all_existing_objects() -> TestResult {
    use gix::objs::{FindExt, Write};

    let root = gix_testtools::scripted_fixture_read_only("make_receive_fsck.sh")?;
    let source = gix::open_opts(root.join("source"), gix::open::Options::isolated())?;
    for (preloaded, incoming, advertised, hidden, accepted, diagnostic) in [
        ("legacy", "legacy", false, false, false, "missingEmail"),
        ("dot-tree", "dot", false, false, false, "hasDot"),
        ("legacy", "valid-descendant", true, false, true, ""),
        ("legacy", "valid-descendant", true, true, false, "missingEmail"),
    ] {
        let stored_id = object_id(&root, preloaded)?;
        let tip_id = object_id(&root, incoming)?;
        for native in [false, true] {
            let destination = gix_testtools::tempfile::tempdir()?;
            gix_testtools::git(destination.path(), "init --bare")?;
            gix_testtools::git(destination.path(), "config receive.fsckObjects true")?;
            let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
            let mut buffer = Vec::new();
            for entry in gix_traverse::object::BreadthFirst::new([stored_id], &source).with_object_validation(false) {
                let entry = entry?;
                let object = source.find(&entry.id, &mut buffer)?;
                receiver.objects.write_buf(object.kind, object.data)?;
            }
            if advertised {
                let output = gix_testtools::git_command(destination.path())
                    .args(["update-ref", "refs/heads/old", &stored_id.to_string()])
                    .output()?;
                assert!(output.status.success());
            }
            if hidden {
                gix_testtools::git(destination.path(), "config receive.hideRefs refs/heads/old")?;
            }
            let output = if native {
                let (result, output) = push(&source, destination.path(), tip_id);
                assert_eq!(
                    matches!(result?.updates[0].status, Status::Updated { .. }),
                    accepted,
                    "{preloaded} advertised={advertised} hidden={hidden}"
                );
                output
            } else {
                let output = gix_testtools::git_command(source.git_dir())
                    .arg("push")
                    .arg(destination.path())
                    .arg(format!("{tip_id}:refs/heads/topic"))
                    .output()?;
                assert_eq!(
                    output.status.success(),
                    accepted,
                    "{preloaded} advertised={advertised} hidden={hidden}: {}",
                    output.stderr.as_bstr()
                );
                output.stderr
            };
            assert!(output.as_bstr().contains_str(diagnostic), "{}", output.as_bstr());
            let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
            assert_eq!(receiver.try_find_reference("refs/heads/topic")?.is_some(), accepted);
        }
    }
    Ok(())
}

#[test]
fn retransmitted_objects_must_match_existing_receiver_bytes_even_without_fsck() -> TestResult {
    use gix::objs::{FindExt, Write};

    let root = gix_testtools::scripted_fixture_read_only("make_receive_fsck.sh")?;
    let source = gix::open_opts(root.join("source"), gix::open::Options::isolated())?;
    let tip_id = object_id(&root, "valid")?;
    let mut source_buffer = Vec::new();
    let source_object = source.find(&tip_id, &mut source_buffer)?;
    let mut changed_commit = source_object.data.to_vec();
    changed_commit.extend_from_slice(b"different message bytes");
    for fsck in [false, true] {
        for (kind, bytes) in [
            (gix::objs::Kind::Blob, b"corrupt receiver payload".as_slice()),
            (gix::objs::Kind::Commit, changed_commit.as_slice()),
        ] {
            for native in [false, true] {
                let destination = gix_testtools::tempfile::tempdir()?;
                gix_testtools::git(destination.path(), "init --bare")?;
                let output = gix_testtools::git_command(destination.path())
                    .args(["config", "receive.fsckObjects", if fsck { "true" } else { "false" }])
                    .output()?;
                assert!(output.status.success());
                // Force Git's pack collision checking even for this tiny fixture.
                gix_testtools::git(destination.path(), "config receive.unpackLimit 0")?;
                let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
                receiver.objects.write_buf_with_known_id(kind, bytes, tip_id)?;
                if native {
                    let (result, _) = push(&source, destination.path(), tip_id);
                    result.expect_err("a retransmitted object must agree with stored data independently of fsck");
                } else {
                    let output = gix_testtools::git_command(source.git_dir())
                        .arg("push")
                        .arg(destination.path())
                        .arg(format!("{tip_id}:refs/heads/topic"))
                        .output()?;
                    assert!(!output.status.success(), "{}", output.stderr.as_bstr());
                }
                let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
                assert!(receiver.try_find_reference("refs/heads/topic")?.is_none());
            }
        }
    }
    Ok(())
}

#[test]
fn incomplete_pushes_preserve_shallow_receiver_integrity() -> TestResult {
    use gix::objs::{FindExt, Write};

    let root = gix_testtools::scripted_fixture_read_only("make_receive_fsck.sh")?;
    let source = gix::open_opts(root.join("source"), gix::open::Options::isolated())?;
    let parent_id = object_id(&root, "valid")?;
    let shallow_id = object_id(&root, "valid-child")?;
    for native in [false, true] {
        let destination = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(destination.path(), "init --bare")?;
        gix_testtools::git(destination.path(), "config receive.fsckObjects true")?;
        let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
        let mut buffer = Vec::new();
        // Store the advertised child and its tree, but deliberately omit its parent.
        for entry in gix_traverse::object::BreadthFirst::new([shallow_id], &source).with_shallow_commits([shallow_id]) {
            let entry = entry?;
            let object = source.find(&entry.id, &mut buffer)?;
            receiver.objects.write_buf(object.kind, object.data)?;
        }
        std::fs::write(destination.path().join("shallow"), format!("{shallow_id}\n"))?;
        let output = gix_testtools::git_command(destination.path())
            .args(["update-ref", "refs/heads/shallow", &shallow_id.to_string()])
            .output()?;
        assert!(output.status.success());
        assert!(!receiver.has_object(parent_id));
        if native {
            let (result, _) = push(&source, destination.path(), parent_id);
            result.expect_err(
                "receiver connectivity rejects an ancestor omitted by the sender's advertised-history exclusions",
            );
        } else {
            let output = gix_testtools::git_command(source.git_dir())
                .arg("push")
                .arg(destination.path())
                .arg(format!("{parent_id}:refs/heads/topic"))
                .output()?;
            assert!(!output.status.success(), "{}", output.stderr.as_bstr());
        }
        let receiver = gix::open_opts(destination.path(), gix::open::Options::isolated())?;
        assert!(receiver.try_find_reference("refs/heads/topic")?.is_none());
        assert!(
            !receiver.has_object(parent_id),
            "an incomplete push does not publish an object or reference"
        );
        assert_eq!(
            receiver.find_reference("refs/heads/shallow")?.target().try_id(),
            Some(shallow_id.as_ref())
        );
        assert_eq!(
            std::fs::read_to_string(destination.path().join("shallow"))?,
            format!("{shallow_id}\n")
        );
    }
    Ok(())
}
