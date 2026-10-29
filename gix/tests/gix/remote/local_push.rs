use std::{path::Path, sync::atomic::AtomicBool};

use gix::{
    ObjectId, Repository,
    push::{Lease, Options, Outcome, Rejection, Status, Update},
};
use gix_testtools::TestResult;

fn update(name: &str, new_id: Option<ObjectId>) -> TestResult<Update> {
    Ok(Update {
        destination: name.try_into()?,
        new_id,
        lease: Lease::Any,
        force: false,
    })
}

fn push(repo: &Repository, path: &Path, updates: Vec<Update>, options: Options<'_>) -> gix::Result<Outcome> {
    repo.remote_at(gix::path::into_bstr(path.to_owned())?)?.push(
        updates,
        options,
        gix::progress::Discard,
        |_, _| Ok(()),
        &AtomicBool::default(),
    )
}

fn read_id(path: &Path, name: &str) -> TestResult<Option<ObjectId>> {
    let repo = gix::open_opts(path, gix::open::Options::isolated())?;
    Ok(repo.try_find_reference(name)?.map(|reference| reference.id().detach()))
}

#[test]
fn local_push_transfers_objects_and_updates_refs_without_git_in_path() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    let tip_id = repo.head_id()?.detach();
    let old_id = repo.find_reference("refs/heads/g")?.id().detach();
    let path = std::env::var_os("PATH");
    let path_guard = gix_testtools::Env::new().set("PATH", destination.path().to_string_lossy());
    let outcome = push(
        &repo,
        destination.path(),
        vec![update("refs/heads/topic", Some(old_id))?],
        Options::default(),
    )?;
    assert_eq!(outcome.updates[0].status, Status::Updated { forced: false });
    let outcome = push(
        &repo,
        destination.path(),
        vec![update("refs/heads/topic", Some(tip_id))?],
        Options::default(),
    )?;
    assert_eq!(outcome.updates[0].status, Status::Updated { forced: false });
    assert_eq!(read_id(destination.path(), "refs/heads/topic")?, Some(tip_id));
    let mut leased = update("refs/heads/topic", Some(old_id))?;
    leased.lease = Lease::MustMatch(Some(tip_id));
    let outcome = push(&repo, destination.path(), vec![leased], Options::default())?;
    assert_eq!(outcome.updates[0].status, Status::Updated { forced: true });
    let outcome = push(
        &repo,
        destination.path(),
        vec![update("refs/heads/topic", None)?],
        Options::default(),
    )?;
    assert!(matches!(outcome.updates[0].status, Status::Updated { .. }));
    assert_eq!(read_id(destination.path(), "refs/heads/topic")?, None);
    drop(path_guard);
    assert_eq!(std::env::var_os("PATH"), path);
    gix_testtools::git(destination.path(), "fsck --strict --no-reflogs")?;
    Ok(())
}

#[test]
fn local_receive_policy_overrides_force_and_atomic_rejects_every_update() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init --bare")?;
    let tip_id = repo.head_id()?.detach();
    let old_id = repo.find_reference("refs/heads/g")?.id().detach();
    push(
        &repo,
        destination.path(),
        vec![update("refs/heads/topic", Some(tip_id))?],
        Options::default(),
    )?;
    gix_testtools::git(destination.path(), "config receive.denyNonFastForwards true")?;
    let mut forced = update("refs/heads/topic", Some(old_id))?;
    forced.force = true;
    let outcome = push(
        &repo,
        destination.path(),
        vec![forced, update("refs/heads/new", Some(tip_id))?],
        Options {
            atomic: true,
            ..Options::default()
        },
    )?;
    assert!(
        outcome
            .updates
            .iter()
            .all(|update| matches!(update.status, Status::Rejected(Rejection::Server(_))))
    );
    assert_eq!(read_id(destination.path(), "refs/heads/topic")?, Some(tip_id));
    assert_eq!(read_id(destination.path(), "refs/heads/new")?, None);
    gix_testtools::git(destination.path(), "config receive.denyDeletes true")?;
    let outcome = push(
        &repo,
        destination.path(),
        vec![update("refs/heads/topic", None)?],
        Options::default(),
    )?;
    assert!(matches!(
        outcome.updates[0].status,
        Status::Rejected(Rejection::Server(_))
    ));
    assert_eq!(read_id(destination.path(), "refs/heads/topic")?, Some(tip_id));
    Ok(())
}

#[test]
fn checked_out_branches_and_hidden_refs_are_protected() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let destination = gix_testtools::tempfile::tempdir()?;
    gix_testtools::git(destination.path(), "init")?;
    let tip_id = repo.head_id()?.detach();
    let outcome = push(
        &repo,
        destination.path(),
        vec![update("refs/heads/main", Some(tip_id))?],
        Options::default(),
    )?;
    assert!(matches!(
        outcome.updates[0].status,
        Status::Rejected(Rejection::Server(_))
    ));
    assert_eq!(read_id(destination.path(), "refs/heads/main")?, None);
    gix_testtools::git(destination.path(), "config receive.hideRefs refs/heads/private")?;
    let outcome = push(
        &repo,
        destination.path(),
        vec![
            update("refs/heads/private/secret", Some(tip_id))?,
            update("refs/heads/public", Some(tip_id))?,
        ],
        Options::default(),
    )?;
    assert!(matches!(
        outcome.updates[0].status,
        Status::Rejected(Rejection::Server(_))
    ));
    assert!(matches!(outcome.updates[1].status, Status::Updated { .. }));
    assert_eq!(read_id(destination.path(), "refs/heads/private/secret")?, None);
    assert_eq!(read_id(destination.path(), "refs/heads/public")?, Some(tip_id));
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

#[test]
fn reference_lock_failure_is_per_ref_unless_atomic() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let repo = crate::remote::repo("base");
    let tip_id = repo.head_id()?.detach();
    for atomic in [false, true] {
        let destination = gix_testtools::tempfile::tempdir()?;
        gix_testtools::git(destination.path(), "init --bare")?;
        gix_testtools::git(destination.path(), "config core.filesRefLockTimeout 0")?;
        std::fs::write(destination.path().join("refs/heads/locked.lock"), b"concurrent writer")?;
        let outcome = push(
            &repo,
            destination.path(),
            vec![
                update("refs/heads/free", Some(tip_id))?,
                update("refs/heads/locked", Some(tip_id))?,
            ],
            Options {
                atomic,
                ..Options::default()
            },
        )?;
        assert!(matches!(
            outcome.updates[1].status,
            Status::Rejected(Rejection::Server(_))
        ));
        assert_eq!(read_id(destination.path(), "refs/heads/locked")?, None);
        assert_eq!(
            read_id(destination.path(), "refs/heads/free")?,
            (!atomic).then_some(tip_id)
        );
    }
    Ok(())
}

#[path = "local_shallow_push.rs"]
mod shallow;

#[cfg(unix)]
#[path = "push_to_checkout.rs"]
mod checkout_hook;
