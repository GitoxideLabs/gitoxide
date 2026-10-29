use std::{path::Path, sync::atomic::AtomicBool};

use gix::{
    ObjectId, Repository,
    push::{Lease, Options, Outcome, Rejection, Status, Update},
};
use gix_testtools::{TestResult, tempfile::TempDir};

fn setup() -> TestResult<(Repository, TempDir)> {
    let repo = crate::remote::repo("base");
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    Ok((repo, destination))
}

fn update(name: &str, new_id: Option<ObjectId>) -> TestResult<Update> {
    Ok(Update {
        destination: name.try_into()?,
        new_id,
        lease: Lease::Any,
        force: false,
    })
}

fn push(repo: &Repository, path: &Path, updates: Vec<Update>, options: Options<'_>) -> gix::Result<Outcome> {
    repo.remote_at(gix::path::into_bstr(path.to_owned())?)?
        .connect(gix::remote::Direction::Push)?
        .push(
            updates,
            options,
            gix::progress::Discard,
            |_, _| Ok(()),
            &AtomicBool::default(),
        )
}

fn remote_id(path: &Path, name: &str) -> TestResult<Option<ObjectId>> {
    let repo = gix::open_opts(path, gix::open::Options::isolated())?;
    Ok(repo.try_find_reference(name)?.map(|reference| reference.id().detach()))
}

#[test]
fn native_push_creates_fast_forwards_tags_and_deletes() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let (repo, destination) = setup()?;
    let old_id = repo.find_reference("refs/heads/g")?.id().detach();
    let tip_id = repo.head_id()?.detach();
    let tag_id = repo.find_reference("refs/tags/b-tag")?.id().detach();
    let outcome = push(
        &repo,
        destination.path(),
        vec![update("refs/heads/main", Some(old_id))?],
        Options::default(),
    )?;
    assert!(matches!(outcome.updates[0].status, Status::Updated { forced: false }));
    assert_eq!(remote_id(destination.path(), "refs/heads/main")?, Some(old_id));
    let outcome = push(
        &repo,
        destination.path(),
        vec![
            update("refs/heads/main", Some(tip_id))?,
            update("refs/tags/v1", Some(tag_id))?,
        ],
        Options::default(),
    )?;
    assert!(
        outcome
            .updates
            .iter()
            .all(|update| matches!(update.status, Status::Updated { forced: false }))
    );
    assert_eq!(remote_id(destination.path(), "refs/heads/main")?, Some(tip_id));
    assert_eq!(remote_id(destination.path(), "refs/tags/v1")?, Some(tag_id));
    gix_testtools::git(destination.path(), "fsck --strict --no-reflogs")?;

    let git_unchanged = gix_testtools::git_command(repo.git_dir())
        .args(["push", "--dry-run", "--porcelain"])
        .arg(format!("--force-with-lease=refs/heads/main:{old_id}"))
        .arg(destination.path())
        .arg(format!("{tip_id}:refs/heads/main"))
        .output()?;
    assert!(
        git_unchanged.status.success(),
        "Git ignores a stale lease for an unchanged tip"
    );

    let mut unchanged = update("refs/heads/main", Some(tip_id))?;
    unchanged.lease = Lease::MustMatch(Some(old_id));
    let outcome = push(&repo, destination.path(), vec![unchanged], Options::default())?;
    assert_eq!(
        outcome.updates[0].status,
        Status::UpToDate,
        "unchanged tips succeed before checking leases, like Git"
    );
    let outcome = push(
        &repo,
        destination.path(),
        vec![update("refs/tags/v1", None)?],
        Options::default(),
    )?;
    assert!(matches!(outcome.updates[0].status, Status::Updated { .. }));
    assert_eq!(remote_id(destination.path(), "refs/tags/v1")?, None);
    Ok(())
}

#[test]
fn push_enforces_leases_and_git_update_rules() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let (repo, destination) = setup()?;
    let old_id = repo.find_reference("refs/heads/g")?.id().detach();
    let tip_id = repo.head_id()?.detach();
    push(
        &repo,
        destination.path(),
        vec![
            update("refs/heads/main", Some(tip_id))?,
            update("refs/tags/v1", Some(tip_id))?,
        ],
        Options::default(),
    )?;
    let git_rejected = gix_testtools::git_command(repo.git_dir())
        .args(["push", "--dry-run", "--porcelain"])
        .arg(destination.path())
        .args([format!("{old_id}:refs/heads/main"), format!("{old_id}:refs/tags/v1")])
        .output()?;
    assert!(!git_rejected.status.success(), "Git rejects these unforced changes");
    let git_status = String::from_utf8_lossy(&git_rejected.stdout);
    assert!(git_status.contains("non-fast-forward"), "{git_status}");
    assert!(git_status.contains("already exists"), "{git_status}");
    let outcome = push(
        &repo,
        destination.path(),
        vec![
            update("refs/heads/main", Some(old_id))?,
            update("refs/tags/v1", Some(old_id))?,
        ],
        Options::default(),
    )?;
    assert_eq!(outcome.updates[0].status, Status::Rejected(Rejection::NonFastForward));
    assert_eq!(outcome.updates[1].status, Status::Rejected(Rejection::AlreadyExists));

    let mut stale = update("refs/heads/main", Some(old_id))?;
    stale.lease = Lease::MustMatch(Some(old_id));
    stale.force = true;
    let outcome = push(&repo, destination.path(), vec![stale.clone()], Options::default())?;
    assert_eq!(
        outcome.updates[0].status,
        Status::Rejected(Rejection::StaleLease),
        "force never bypasses an explicit lease"
    );
    let git_leased = gix_testtools::git_command(repo.git_dir())
        .args(["push", "--dry-run", "--porcelain"])
        .arg(format!("--force-with-lease=refs/heads/main:{tip_id}"))
        .arg(destination.path())
        .arg(format!("{old_id}:refs/heads/main"))
        .output()?;
    assert!(
        git_leased.status.success(),
        "a matching lease also permits Git's forced update"
    );
    stale.lease = Lease::MustMatch(Some(tip_id));
    stale.force = false;
    let outcome = push(&repo, destination.path(), vec![stale], Options::default())?;
    assert_eq!(
        outcome.updates[0].status,
        Status::Updated { forced: true },
        "a matching lease permits a non-fast-forward"
    );
    assert_eq!(remote_id(destination.path(), "refs/heads/main")?, Some(old_id));

    let blob_id = repo
        .find_reference("refs/heads/g")?
        .peel_to_commit()?
        .tree()?
        .iter()
        .next()
        .expect("the tree has a blob")?
        .id()
        .detach();
    let mut non_commit = update("refs/heads/blob", Some(blob_id))?;
    non_commit.force = true;
    let outcome = push(
        &repo,
        destination.path(),
        vec![non_commit, update("refs/custom/blob", Some(blob_id))?],
        Options::default(),
    )?;
    assert_eq!(outcome.updates[0].status, Status::Rejected(Rejection::NonCommitBranch));
    assert!(matches!(outcome.updates[1].status, Status::Updated { .. }));
    Ok(())
}

#[test]
fn tracking_lease_includes_matches_git_for_raw_ids_and_destination_reflogs() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    for hash in gix::hash::Kind::all() {
        let fixture = gix_testtools::scripted_fixture_read_only_with_args("make_push_tracking.sh", [hash.to_string()])?;
        let source = fixture.join("source");
        let destination = fixture.join("destination");
        let repo = gix::open_opts(&source, gix::open::Options::isolated())?;
        let remote = repo.find_remote("origin")?.with_push_url(destination.as_path())?;
        let new_id = repo.find_reference("refs/heads/source")?.id().detach();
        for (branch, included) in [
            ("included", true),
            ("descendant", true),
            ("boundary", true),
            ("too-old", false),
            ("not-included", false),
            ("no-local", false),
        ] {
            for force_if_includes in [false, true] {
                let mut request = update(&format!("refs/heads/{branch}"), Some(new_id))?;
                request.lease = Lease::Tracking { force_if_includes };
                let options = Options {
                    dry_run: true,
                    ..Options::default()
                };
                let outcome = remote.push(
                    [request.clone()],
                    options,
                    gix::progress::Discard,
                    |_, _| Ok(()),
                    &AtomicBool::new(false),
                )?;
                let wire = remote.connect(gix::remote::Direction::Push)?.push(
                    [request],
                    options,
                    gix::progress::Discard,
                    |_, _| Ok(()),
                    &AtomicBool::new(false),
                )?;
                assert_eq!(
                    wire.updates, outcome.updates,
                    "local and transport planners share tracking-lease policy"
                );
                if force_if_includes && !included {
                    assert_eq!(outcome.updates[0].status, Status::Rejected(Rejection::RemoteUpdated));
                }
                for source_spec in [new_id.to_string(), "source".into()] {
                    let mut git = gix_testtools::git_command(&source);
                    git.args(["push", "--dry-run", "--porcelain", "--no-verify", "--force-with-lease"]);
                    if force_if_includes {
                        git.arg("--force-if-includes");
                    }
                    let git = git
                        .arg("origin")
                        .arg(format!("{source_spec}:refs/heads/{branch}"))
                        .output()?;
                    let should_succeed = !force_if_includes || included;
                    assert_eq!(
                        git.status.success(),
                        should_succeed,
                        "Git oracle for {branch}, source={source_spec}, includes={force_if_includes}: {git:?}"
                    );
                    assert_eq!(
                        matches!(outcome.updates[0].status, Status::WouldUpdate { forced: true }),
                        should_succeed,
                        "native push agrees for {branch}, source={source_spec}, includes={force_if_includes}"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn tracking_leases_handle_absence_deletion_staleness_and_literal_first_mapping() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable_with_args(
        "make_push_tracking.sh",
        ["sha1"],
        gix_testtools::Creation::Execute,
    )?;
    let repo = gix::open_opts(fixture.path().join("source"), gix::open::Options::isolated())?;
    let destination = fixture.path().join("destination");
    let remote = repo.find_remote("origin")?.with_push_url(destination.as_path())?;
    let rewritten_id = repo.find_reference("refs/heads/source")?.id().detach();
    let old_id = repo.find_reference("refs/remotes/origin/included")?.id().detach();
    let request = |branch: &str, id: Option<ObjectId>, force_if_includes| -> TestResult<Update> {
        let mut update = update(&format!("refs/heads/{branch}"), id)?;
        update.lease = Lease::Tracking { force_if_includes };
        Ok(update)
    };
    let push = |remote: &gix::Remote<'_>, update| {
        remote.push(
            [update],
            Options::default(),
            gix::progress::Discard,
            |_, _| Ok(()),
            &AtomicBool::new(false),
        )
    };
    let outcome = push(&remote, request("fresh", Some(rewritten_id), true)?)?;
    assert_eq!(
        outcome.updates[0].status,
        Status::Updated { forced: false },
        "no tracking tip means expected absence, with no reflog check"
    );
    let outcome = push(&remote, request("fresh", None, false)?)?;
    assert_eq!(
        outcome.updates[0].status,
        Status::Rejected(Rejection::StaleLease),
        "expected absence protects a newly created remote ref"
    );
    let outcome = push(&remote, request("fresh", Some(rewritten_id), true)?)?;
    assert_eq!(
        outcome.updates[0].status,
        Status::UpToDate,
        "unchanged tips bypass even a missing tracking lease"
    );

    let outcome = push(&remote, request("included", Some(rewritten_id), true)?)?;
    assert_eq!(outcome.updates[0].status, Status::Updated { forced: true });
    let outcome = push(&remote, request("included", Some(old_id), true)?)?;
    assert_eq!(
        outcome.updates[0].status,
        Status::Rejected(Rejection::StaleLease),
        "the stale lease takes precedence over reachability"
    );

    let outcome = push(&remote, request("not-included", None, true)?)?;
    assert_eq!(
        outcome.updates[0].status,
        Status::Rejected(Rejection::RemoteUpdated),
        "Git's includes check also applies to deletion when requested"
    );
    let outcome = push(&remote, request("not-included", None, false)?)?;
    assert!(
        matches!(outcome.updates[0].status, Status::Updated { .. }),
        "a matching tracking lease is enough to protect deletion"
    );
    assert_eq!(remote_id(&destination, "refs/heads/not-included")?, None);

    for specs in [
        vec!["refs/heads/*:refs/missing/*", "refs/heads/*:refs/remotes/origin/*"],
        vec!["descendant:refs/remotes/origin/descendant"],
        vec!["refs/heads/*:refs/remotes/origin/*", "^refs/heads/descendant"],
        Vec::new(),
    ] {
        let mut selected = remote.clone();
        selected.replace_refspecs(specs.iter().copied(), gix::remote::Direction::Fetch)?;
        let outcome = push(&selected, request("descendant", Some(rewritten_id), false)?)?;
        assert_eq!(
            outcome.updates[0].status,
            Status::Rejected(Rejection::StaleLease),
            "mapping selection is literal, first-match and exclusion-aware: {specs:?}"
        );
    }
    Ok(())
}

#[test]
#[cfg(feature = "revision")]
fn exact_refspec_updates_resolve_revisions_and_reject_implicit_destinations() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only_with_args("make_push_tracking.sh", ["sha1"])?;
    let repo = gix::open_opts(fixture.join("source"), gix::open::Options::isolated())?;
    let tip = repo.find_reference("refs/heads/source")?.id().detach();
    let parent = repo.rev_parse_single("source^")?.detach();
    for (spec, destination, new_id, force) in [
        ("source:refs/heads/topic", "refs/heads/topic", Some(tip), false),
        ("+source^:refs/heads/topic", "refs/heads/topic", Some(parent), true),
        ("refs/heads/source", "refs/heads/source", Some(tip), false),
        (":refs/heads/topic", "refs/heads/topic", None, false),
        (
            "source:refs/for/main%topic=test",
            "refs/for/main%topic=test",
            Some(tip),
            false,
        ),
    ] {
        let parsed = gix::refspec::parse(spec.into(), gix::refspec::parse::Operation::Push)?;
        let update = Update::from_refspec(&repo, parsed)?;
        assert_eq!(update.destination, destination);
        assert_eq!(update.new_id, new_id, "{spec}");
        assert_eq!(update.force, force, "{spec}");
        assert_eq!(update.lease, Lease::Any, "constructing an update does not add a lease");
    }
    for spec in [":", "source:topic", "refs/heads/*:refs/heads/*", "^refs/heads/source"] {
        let parsed = gix::refspec::parse(spec.into(), gix::refspec::parse::Operation::Push)?;
        assert!(
            Update::from_refspec(&repo, parsed)
                .expect_err("implicit or patterned destinations need advertisement-aware selection")
                .is_unsupported(),
            "{spec}"
        );
    }
    let missing = gix::refspec::parse("absent:refs/heads/topic".into(), gix::refspec::parse::Operation::Push)?;
    assert!(
        Update::from_refspec(&repo, missing)
            .expect_err("missing source refs are errors")
            .is_not_found()
    );
    Ok(())
}

#[test]
fn tracking_updates_follow_confirmed_results_and_preserve_remote_outcomes_on_local_error() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable_with_args(
        "make_push_tracking.sh",
        ["sha1"],
        gix_testtools::Creation::Execute,
    )?;
    let repo = gix::open_opts(
        fixture.path().join("source"),
        gix::open::Options::isolated().config_overrides(["core.filesRefLockTimeout=0"]),
    )?;
    let destination = fixture.path().join("destination");
    let remote = repo.find_remote("origin")?.with_push_url(destination.as_path())?;
    let old_id = repo.find_reference("refs/remotes/origin/included")?.id().detach();
    let new_id = repo.find_reference("refs/heads/source")?.id().detach();
    let mut request = update("refs/heads/included", Some(new_id))?;
    request.force = true;
    let push = |request: Update, dry_run| {
        remote.push(
            [request],
            Options {
                dry_run,
                ..Options::default()
            },
            gix::progress::Discard,
            |_, _| Ok(()),
            &AtomicBool::new(false),
        )
    };
    let dry_run = push(request.clone(), true)?;
    assert!(
        remote.update_tracking_refs(&dry_run)?.is_empty(),
        "dry runs never write tracking refs"
    );
    let confirmed = push(request.clone(), false)?;
    assert_eq!(
        repo.find_reference("refs/remotes/origin/included")?.id(),
        old_id,
        "pushing leaves local changes to the explicit helper"
    );
    let lock = repo.git_dir().join("refs/remotes/origin/included.lock");
    std::fs::write(&lock, "held by another writer\n")?;
    assert!(
        remote.update_tracking_refs(&confirmed).is_err(),
        "local lock errors remain separate from the successful remote result"
    );
    std::fs::remove_file(lock)?;
    assert_eq!(confirmed.updates[0].status, Status::Updated { forced: true });
    assert_eq!(remote_id(&destination, "refs/heads/included")?, Some(new_id));
    let up_to_date_dry_run = push(request.clone(), true)?;
    assert_eq!(up_to_date_dry_run.updates[0].status, Status::UpToDate);
    assert!(
        remote.update_tracking_refs(&up_to_date_dry_run)?.is_empty(),
        "even up-to-date dry runs must leave stale local tracking tips untouched"
    );
    assert_eq!(repo.find_reference("refs/remotes/origin/included")?.id(), old_id);
    let up_to_date = push(request, false)?;
    let edits = remote.update_tracking_refs(&up_to_date)?;
    assert_eq!(edits.len(), 1);
    assert_eq!(
        repo.find_reference("refs/remotes/origin/included")?.id(),
        new_id,
        "up-to-date remote results refresh stale tracking refs"
    );
    let mut log = Vec::new();
    let last = repo
        .refs
        .reflog_iter("refs/remotes/origin/included", &mut log)?
        .expect("tracking reflog")
        .last()
        .expect("tracking update log entry")?;
    assert_eq!(last.message, "update by push");

    let rejected = push(update("refs/heads/not-included", Some(new_id))?, false)?;
    assert!(matches!(rejected.updates[0].status, Status::Rejected(_)));
    assert!(remote.update_tracking_refs(&rejected)?.is_empty());
    assert_eq!(repo.find_reference("refs/remotes/origin/not-included")?.id(), old_id);
    let deleted = push(update("refs/heads/not-included", None)?, false)?;
    remote.update_tracking_refs(&deleted)?;
    assert!(repo.try_find_reference("refs/remotes/origin/not-included")?.is_none());
    assert!(
        !repo.refs.reflog_exists("refs/remotes/origin/not-included")?,
        "deletions remove their tracking reflogs"
    );
    Ok(())
}

#[test]
fn tracking_updates_honor_each_proc_receive_report() -> TestResult {
    use gix::protocol::push::{RefStatus, Report, Response};

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_writable_with_args(
        "make_push_tracking.sh",
        ["sha1"],
        gix_testtools::Creation::Execute,
    )?;
    let repo = gix::open_opts(fixture.path().join("source"), gix::open::Options::isolated())?;
    let remote = repo
        .find_remote("origin")?
        .with_push_url(fixture.path().join("destination").as_path())?;
    let new_id = repo.find_reference("refs/heads/source")?.id().detach();
    let reported_id = repo.find_reference("refs/remotes/origin/included")?.id().detach();
    let mut outcome = remote.push(
        [update("refs/for/main", Some(new_id))?],
        Options::default(),
        gix::progress::Discard,
        |_, _| Ok(()),
        &AtomicBool::new(false),
    )?;
    // A proc-receive server can report several actual destinations for one command.
    outcome.response = Some(Response {
        unpack_error: None,
        refs: vec![
            RefStatus {
                name: "refs/for/main".try_into()?,
                error: None,
                report: Report {
                    ref_name: Some("refs/heads/reported".try_into()?),
                    new_id: Some(reported_id),
                    ..Report::default()
                },
            },
            RefStatus {
                name: "refs/for/main".try_into()?,
                error: None,
                report: Report {
                    ref_name: Some("refs/heads/descendant".try_into()?),
                    new_id: Some(repo.object_hash().null()),
                    ..Report::default()
                },
            },
        ],
    });
    let mut missing_object = outcome.clone();
    missing_object.response.as_mut().expect("reports were just set").refs[1]
        .report
        .new_id = Some(ObjectId::from_hex(b"1111111111111111111111111111111111111111")?);
    assert!(
        remote
            .update_tracking_refs(&missing_object)
            .expect_err("reported objects must exist locally")
            .is_not_found()
    );
    assert!(
        repo.try_find_reference("refs/remotes/origin/reported")?.is_none(),
        "an invalid later report prevents earlier tracking edits too"
    );

    let branch_remote = repo
        .remote_at("unused")?
        .with_refspecs(["refs/heads/*:refs/heads/*"], gix::remote::Direction::Fetch)?;
    let blob_id = repo.write_blob(b"reported non-commit")?.detach();
    let git_rejected = gix_testtools::git_command(repo.git_dir())
        .args(["update-ref", "refs/heads/reported", &blob_id.to_string()])
        .output()?;
    assert!(!git_rejected.status.success(), "Git rejects non-commit branch tips");
    let mut non_commit = outcome.clone();
    non_commit.response.as_mut().expect("reports were just set").refs[0]
        .report
        .new_id = Some(blob_id);
    assert!(
        branch_remote.update_tracking_refs(&non_commit).is_err(),
        "tracking mappings to local branches require commits, like Git's reference updates"
    );
    assert!(repo.try_find_reference("refs/heads/reported")?.is_none());

    let edits = remote.update_tracking_refs(&outcome)?;
    assert_eq!(edits.len(), 2);
    assert_eq!(
        repo.find_reference("refs/remotes/origin/reported")?.id(),
        reported_id,
        "reported refname and ID override the submitted command"
    );
    assert!(
        repo.try_find_reference("refs/remotes/origin/descendant")?.is_none(),
        "reported null IDs delete tracking refs"
    );
    assert!(
        repo.try_find_reference("refs/remotes/origin/main")?.is_none(),
        "the submitted destination is not used when reports redirect it"
    );
    Ok(())
}

#[test]
fn dry_run_and_atomic_local_rejections_do_not_change_refs() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let (repo, destination) = setup()?;
    let tip_id = repo.head_id()?.detach();
    let outcome = push(
        &repo,
        destination.path(),
        vec![update("refs/heads/dry", Some(tip_id))?],
        Options {
            dry_run: true,
            ..Options::default()
        },
    )?;
    assert_eq!(outcome.updates[0].status, Status::WouldUpdate { forced: false });
    assert_eq!(remote_id(destination.path(), "refs/heads/dry")?, None);

    let mut stale = update("refs/heads/stale", Some(tip_id))?;
    stale.lease = Lease::MustMatch(Some(tip_id));
    let outcome = push(
        &repo,
        destination.path(),
        vec![update("refs/heads/good", Some(tip_id))?, stale],
        Options {
            atomic: true,
            ..Options::default()
        },
    )?;
    assert_eq!(outcome.updates[0].status, Status::Rejected(Rejection::Atomic));
    assert_eq!(outcome.updates[1].status, Status::Rejected(Rejection::StaleLease));
    assert_eq!(remote_id(destination.path(), "refs/heads/good")?, None);
    Ok(())
}

#[test]
#[cfg(unix)]
fn receive_hooks_push_options_and_atomic_reports_are_preserved() -> TestResult {
    use std::os::unix::fs::PermissionsExt;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let (repo, destination) = setup()?;
    gix_testtools::git(destination.path(), "config receive.advertisePushOptions true")?;
    for (name, script) in [
        (
            "pre-receive",
            r#"#!/bin/sh
[ "$GIT_PUSH_OPTION_COUNT" = 2 ] || exit 1
[ "$GIT_PUSH_OPTION_0" = "" ] || exit 1
[ "$GIT_PUSH_OPTION_1" = ci.skip ] || exit 1
echo 'push options accepted' >&2
cat >/dev/null
"#,
        ),
        (
            "update",
            r#"#!/bin/sh
if [ "$1" = refs/heads/reject ]; then
    echo 'branch denied by hook' >&2
    exit 1
fi
"#,
        ),
    ] {
        let hook = destination.path().join("hooks").join(name);
        std::fs::write(&hook, script)?;
        std::fs::set_permissions(hook, std::fs::Permissions::from_mode(0o755))?;
    }
    let tip_id = repo.head_id()?.detach();
    let push_options = ["".into(), "ci.skip".into()];
    for atomic in [true, false] {
        let mut diagnostics = Vec::new();
        let remote = repo.remote_at(gix::path::into_bstr(destination.path().to_owned())?)?;
        let outcome = remote.connect(gix::remote::Direction::Push)?.push(
            vec![
                update("refs/heads/good", Some(tip_id))?,
                update("refs/heads/reject", Some(tip_id))?,
            ],
            Options {
                atomic,
                push_options: &push_options,
                ..Options::default()
            },
            gix::progress::Discard,
            |_, data| {
                diagnostics.extend_from_slice(data);
                Ok(())
            },
            &AtomicBool::default(),
        )?;
        assert!(String::from_utf8_lossy(&diagnostics).contains("push options accepted"));
        assert!(String::from_utf8_lossy(&diagnostics).contains("branch denied by hook"));
        assert!(matches!(
            outcome.updates[1].status,
            Status::Rejected(Rejection::Server(_))
        ));
        assert_eq!(remote_id(destination.path(), "refs/heads/reject")?, None);
        if atomic {
            assert!(matches!(
                outcome.updates[0].status,
                Status::Rejected(Rejection::Server(_))
            ));
            assert_eq!(remote_id(destination.path(), "refs/heads/good")?, None);
        } else {
            assert!(matches!(outcome.updates[0].status, Status::Updated { .. }));
            assert_eq!(remote_id(destination.path(), "refs/heads/good")?, Some(tip_id));
        }
    }
    Ok(())
}

#[test]
fn pushes_an_in_memory_commit_with_an_implicit_empty_tree() -> TestResult {
    use gix_object::Exists;

    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let mut commit = crate::remote::repo("base").head_commit()?.decode()?.to_owned()?;
    let repo =
        crate::named_subrepo_opts("make_basic_repo.sh", "unborn", gix::open::Options::isolated())?.with_object_memory();
    commit.tree = ObjectId::empty_tree(repo.object_hash());
    commit.parents.clear();
    let commit_id = repo.write_object(&commit)?.detach();
    assert!(
        !repo.objects.exists(&commit.tree),
        "the empty tree is synthesized by the repository"
    );
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    let outcome = push(
        &repo,
        destination.path(),
        vec![update("refs/heads/main", Some(commit_id))?],
        Options::default(),
    )?;
    assert!(matches!(outcome.updates[0].status, Status::Updated { .. }));
    assert_eq!(remote_id(destination.path(), "refs/heads/main")?, Some(commit_id));
    gix_testtools::git(destination.path(), "fsck --strict --no-reflogs")?;
    Ok(())
}

#[test]
fn deletion_of_absent_refs_is_checked_by_the_receiver() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    gix_testtools::git(destination.path(), "config receive.hideRefs refs/heads/private")?;
    let outcome = push(
        &repo,
        destination.path(),
        vec![update("refs/heads/private", None)?, update("refs/heads/absent", None)?],
        Options::default(),
    )?;
    assert_eq!(
        outcome.updates[0].status,
        Status::Rejected(Rejection::Server("deny deleting a hidden ref".into()))
    );
    assert_eq!(outcome.updates[1].status, Status::Updated { forced: false });
    assert_eq!(
        outcome.response.expect("deletions receive a server report").refs.len(),
        2
    );
    Ok(())
}
