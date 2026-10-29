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
