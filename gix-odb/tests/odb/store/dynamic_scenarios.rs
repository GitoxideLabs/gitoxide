use gix_object::Find as _;
use gix_odb::Header as _;

use crate::{
    Result,
    odb_fixture::{Component, Database, OdbFixture, Pack},
};

fn open(fixture: &OdbFixture, slots: u16) -> std::io::Result<gix_odb::Handle> {
    open_with_slots(fixture, gix_odb::store::init::Slots::Limit(slots))
}

fn open_with_slots(fixture: &OdbFixture, slots: gix_odb::store::init::Slots) -> std::io::Result<gix_odb::Handle> {
    gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        Vec::new(),
        gix_odb::store::init::Options {
            slots,
            ..Default::default()
        },
    )
}

fn assert_object(handle: &gix_odb::Handle, id: &gix_hash::oid) -> Result {
    let mut buffer = Vec::new();
    let object = handle.try_find(id, &mut buffer)?.expect("fixture object is available");
    assert_eq!(
        gix_object::compute_hash(id.kind(), object.kind, object.data)?,
        id,
        "the ODB returned bytes belonging to the requested object"
    );
    let header = handle.try_header(id)?.expect("the same object has a header");
    assert_eq!(header.kind(), object.kind, "header and object kinds agree");
    assert_eq!(header.size(), object.data.len() as u64, "header and object sizes agree");
    Ok(())
}

fn assert_missing(handle: &gix_odb::Handle, id: &gix_hash::oid) -> Result {
    let mut buffer = Vec::new();
    assert!(
        handle.try_find(id, &mut buffer)?.is_none(),
        "the object is not reachable in the current fixture state"
    );
    Ok(())
}

#[cfg(feature = "parallel")]
fn contended_lookup(
    first: gix_odb::HandleArc,
    second: gix_odb::HandleArc,
    id: gix_hash::ObjectId,
    pause_next_refresh: &std::sync::atomic::AtomicBool,
    point_rx: &crossbeam_channel::Receiver<gix_odb::store::init::debug::Point>,
    resume_tx: &crossbeam_channel::Sender<()>,
) -> (
    std::result::Result<bool, String>,
    std::result::Result<bool, String>,
    Vec<gix_odb::store::init::debug::Point>,
) {
    use std::{sync::atomic::Ordering, time::Duration};

    use gix_odb::store::init::debug::Point;

    let lookup = move |handle: gix_odb::HandleArc| {
        std::thread::spawn(move || {
            let mut buffer = Vec::new();
            handle
                .try_find(&id, &mut buffer)
                .map(|object| object.is_some())
                .map_err(|err| err.to_string())
        })
    };
    pause_next_refresh.store(true, Ordering::SeqCst);
    let first_thread = lookup(first);
    let mut observed = Vec::new();
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the first handle starts its refresh scan");
        observed.push(point);
        if matches!(point, Point::RefreshScanStarted) {
            break;
        }
    }
    let second_thread = lookup(second);
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the second handle attempts to acquire the refresh lock");
        observed.push(point);
        if matches!(point, Point::RefreshLocking) {
            break;
        }
    }
    resume_tx
        .send(())
        .expect("the first refresh is waiting for its release");
    let first = first_thread.join().expect("the first lookup does not panic");
    let second = second_thread.join().expect("the second lookup does not panic");
    observed.extend(point_rx.try_iter());
    (first, second, observed)
}

#[cfg(feature = "parallel")]
#[test]
fn debug_hooks_coordinate_contending_failed_index_loaders() -> Result {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    use gix_odb::store::init::debug::{LoadOutcome, Point};

    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.corrupt_index(Database::Primary, Pack::A)?;
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    let (point_tx, point_rx) = crossbeam_channel::unbounded();
    let (resume_tx, resume_rx) = crossbeam_channel::bounded(0);
    let pause_first_loader = Arc::new(AtomicBool::new(true));
    let debug = gix_odb::store::init::debug::Options::new({
        let pause_first_loader = Arc::clone(&pause_first_loader);
        move |point| {
            point_tx
                .send(point)
                .expect("the test receives every synchronization point");
            if matches!(point, Point::IndexLoadClaimed { .. }) && pause_first_loader.swap(false, Ordering::SeqCst) {
                resume_rx.recv().expect("the test releases the first index loader");
            }
        }
    });
    let first = gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        Vec::new(),
        gix_odb::store::init::Options {
            slots: gix_odb::store::init::Slots::Limit(1),
            debug: Some(debug),
            ..Default::default()
        },
    )?
    .into_arc()?;
    let second = first.clone();
    let recovery = first.clone();

    let lookup = move |handle: gix_odb::HandleArc| {
        std::thread::spawn(move || {
            let mut buffer = Vec::new();
            handle
                .try_find(&id, &mut buffer)
                .map(|object| object.is_some())
                .map_err(|err| err.to_string())
        })
    };
    let first_thread = lookup(first);
    let mut observed = Vec::new();
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the first loader reaches its claimed-index synchronization point");
        observed.push(point);
        if matches!(point, Point::IndexLoadClaimed { .. }) {
            break;
        }
    }

    let second_thread = lookup(second);
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the second loader reaches snapshot contention");
        observed.push(point);
        if matches!(point, Point::SnapshotWaitingForIndexLoad) {
            break;
        }
    }
    resume_tx.send(()).expect("the first loader is waiting for its release");

    let first_error = first_thread
        .join()
        .expect("the first loader does not panic")
        .expect_err("the first loader reports the malformed index");
    let second_error = second_thread
        .join()
        .expect("the second loader does not panic")
        .expect_err("the waiting loader reports the malformed index");
    assert_eq!(
        first_error, second_error,
        "all handles observe the same cached index-load error"
    );
    observed.extend(point_rx.try_iter());
    assert_eq!(
        observed
            .iter()
            .filter(|point| matches!(point, Point::IndexLoadClaimed { .. }))
            .count(),
        1,
        "only one thread claims the malformed index"
    );
    assert_eq!(
        observed
            .iter()
            .filter(|point| matches!(point, Point::IndexSlotLocked { .. }))
            .count(),
        1,
        "the claimed slot is locked exactly once"
    );
    assert_eq!(
        observed
            .iter()
            .filter(|point| matches!(
                point,
                Point::IndexLoadCompleted {
                    outcome: LoadOutcome::Failure,
                    ..
                }
            ))
            .count(),
        1,
        "the shared malformed index is opened exactly once"
    );
    assert_eq!(
        observed
            .iter()
            .filter(|point| matches!(point, Point::IndexStatePublished))
            .count(),
        1,
        "initial discovery publishes one index state"
    );

    let mut buffer = Vec::new();
    assert_eq!(
        recovery
            .try_find(&id, &mut buffer)
            .expect_err("the unchanged malformed index remains an error")
            .to_string(),
        first_error,
        "later lookups observe the cached error"
    );
    assert!(
        point_rx
            .try_iter()
            .all(|point| !matches!(point, Point::IndexLoadCompleted { .. })),
        "the unchanged malformed index is not loaded again"
    );

    fixture.publish(Database::Primary, Pack::A, Component::Index)?;
    assert!(
        recovery
            .try_find(&id, &mut buffer)
            .expect("the replacement index loads")
            .is_some(),
        "the shared store recovers after the malformed index is replaced"
    );
    assert_eq!(
        point_rx
            .try_iter()
            .filter(|point| matches!(
                point,
                Point::IndexLoadCompleted {
                    outcome: LoadOutcome::Success,
                    ..
                }
            ))
            .count(),
        1,
        "the replacement index is loaded exactly once"
    );
    Ok(())
}

#[cfg(feature = "parallel")]
#[test]
fn debug_hooks_coalesce_successful_refreshes() -> Result {
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };

    use gix_odb::store::init::debug::Point;

    let mut fixture = OdbFixture::from_script()?;
    let (point_tx, point_rx) = crossbeam_channel::unbounded();
    let (resume_tx, resume_rx) = crossbeam_channel::bounded(0);
    let pause_next_refresh = Arc::new(AtomicBool::new(false));
    let now = Arc::new(Mutex::new(Instant::now()));
    let debug = gix_odb::store::init::debug::Options::new({
        let pause_next_refresh = Arc::clone(&pause_next_refresh);
        move |point| {
            point_tx
                .send(point)
                .expect("the test receives every synchronization point");
            if matches!(point, Point::RefreshScanStarted) && pause_next_refresh.swap(false, Ordering::SeqCst) {
                resume_rx.recv().expect("the test releases the refresh scan");
            }
        }
    })
    .with_clock({
        let now = Arc::clone(&now);
        move || *now.lock().expect("the test clock isn't poisoned")
    });
    let mut handle = gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        Vec::new(),
        gix_odb::store::init::Options {
            slots: gix_odb::store::init::Slots::Limit(4),
            debug: Some(debug),
            ..Default::default()
        },
    )?
    .into_arc()?;
    assert_eq!(
        handle.store_ref().structure()?.len(),
        1,
        "initialization discovers only the primary loose-object database"
    );
    assert_eq!(
        handle.store_ref().metrics().num_refreshes,
        1,
        "initialization scans the empty ODB once"
    );

    fixture.install_pack(Database::Primary, Pack::A)?;
    point_rx.try_iter().for_each(drop);
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    let (first, second, observed) = contended_lookup(
        handle.clone(),
        handle.clone(),
        id,
        &pause_next_refresh,
        &point_rx,
        &resume_tx,
    );
    assert!(
        first.expect("the first lookup succeeds"),
        "the first handle finds the new object"
    );
    assert!(
        second.expect("the waiting lookup succeeds"),
        "the waiting handle observes the refreshed state"
    );
    assert_eq!(
        observed
            .iter()
            .filter(|point| matches!(point, Point::RefreshScanStarted))
            .count(),
        1,
        "contending handles scan the changed ODB once"
    );
    assert_eq!(
        handle.store_ref().metrics().num_refreshes,
        2,
        "the changed ODB adds one refresh"
    );

    let refresh_after = Duration::from_secs(1);
    handle.refresh = gix_odb::store::RefreshMode::AfterDuration(refresh_after);
    {
        let mut now = now.lock().expect("the test clock isn't poisoned");
        *now += refresh_after;
    }
    point_rx.try_iter().for_each(drop);
    let (first, second, observed) = contended_lookup(
        handle.clone(),
        handle.clone(),
        fixture.manifest.missing_id(),
        &pause_next_refresh,
        &point_rx,
        &resume_tx,
    );
    assert!(!first.expect("the first miss succeeds"), "the object remains absent");
    assert!(
        !second.expect("the waiting miss succeeds"),
        "the waiting handle shares the unchanged refresh"
    );
    assert_eq!(
        observed
            .iter()
            .filter(|point| matches!(point, Point::RefreshScanStarted))
            .count(),
        1,
        "contending handles scan the unchanged ODB once at the shared deadline"
    );
    assert_eq!(
        handle.store_ref().metrics().num_refreshes,
        3,
        "the unchanged ODB adds one refresh"
    );
    Ok(())
}

#[cfg(feature = "parallel")]
#[test]
fn debug_hooks_share_refresh_errors_with_waiters() -> Result {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };

    use gix_odb::store::init::debug::{LoadOutcome, Point};

    let mut fixture = OdbFixture::from_script()?;
    let (point_tx, point_rx) = crossbeam_channel::unbounded();
    let (resume_tx, resume_rx) = crossbeam_channel::bounded(0);
    let pause_next_refresh = Arc::new(AtomicBool::new(false));
    let debug = gix_odb::store::init::debug::Options::new({
        let pause_next_refresh = Arc::clone(&pause_next_refresh);
        move |point| {
            point_tx
                .send(point)
                .expect("the test receives every synchronization point");
            if matches!(point, Point::RefreshScanStarted) && pause_next_refresh.swap(false, Ordering::SeqCst) {
                resume_rx.recv().expect("the test releases the refresh scan");
            }
        }
    });
    let handle = gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        Vec::new(),
        gix_odb::store::init::Options {
            slots: gix_odb::store::init::Slots::Limit(1),
            debug: Some(debug),
            ..Default::default()
        },
    )?
    .into_arc()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.install_pack(Database::Primary, Pack::B)?;
    point_rx.try_iter().for_each(drop);
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    let (first, second, observed) = contended_lookup(
        handle.clone(),
        handle.clone(),
        id,
        &pause_next_refresh,
        &point_rx,
        &resume_tx,
    );
    let first = first.expect_err("one slot cannot hold both discovered indices");
    let second = second.expect_err("the waiting handle observes the same refresh failure");
    assert_eq!(first, second, "both handles receive the shared refresh error");
    assert_eq!(
        observed
            .iter()
            .filter(|point| matches!(point, Point::RefreshScanStarted))
            .count(),
        1,
        "the failed refresh scans the ODB once"
    );
    assert_eq!(
        observed
            .iter()
            .filter(|point| matches!(
                point,
                Point::RefreshScanCompleted {
                    outcome: LoadOutcome::Failure
                }
            ))
            .count(),
        1,
        "the single scan publishes one failed outcome"
    );
    assert_eq!(
        handle.store_ref().metrics().num_refreshes,
        1,
        "the failed initialization scans once"
    );

    fixture.remove_pack(Database::Primary, Pack::B)?;
    assert_object(&handle, &id)?;
    assert_eq!(
        handle.store_ref().metrics().num_refreshes,
        2,
        "a later lookup retries after the disk state changes"
    );
    Ok(())
}

#[test]
fn fixture_actions_build_a_valid_odb_from_empty() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    assert!(fixture.is_valid(), "the empty object directories are valid");
    assert!(fixture.reachable_ids().is_empty(), "the active ODB starts empty");

    fixture.install_pack(Database::Primary, Pack::A)?;
    assert!(fixture.is_valid(), "a complete pack pair is valid");
    assert_eq!(
        fixture.reachable_ids().len(),
        fixture.manifest.pack(Pack::A).object_ids.len(),
        "the manifest describes every reachable object in the installed pack"
    );

    let handle = open(&fixture, 4)?;
    for id in &fixture.manifest.pack(Pack::A).object_ids {
        assert_object(&handle, id)?;
    }

    fixture.remove_pack(Database::Primary, Pack::A)?;
    assert!(
        fixture.is_valid(),
        "removing all components restores an empty valid ODB"
    );
    Ok(())
}

#[test]
fn stale_handles_interleave_pack_publication_and_removal() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let first = open(&fixture, 8)?;
    let second = first.clone();
    let a = fixture.manifest.pack(Pack::A).object_ids[0];
    let b = fixture.manifest.pack(Pack::B).object_ids[0];
    assert_object(&first, &a)?;

    fixture.publish(Database::Primary, Pack::B, Component::Pack)?;
    assert_missing(&second, &b)?;
    fixture.publish(Database::Primary, Pack::B, Component::ReverseIndex)?;
    fixture.publish(Database::Primary, Pack::B, Component::Index)?;
    assert_object(&second, &b)?;
    assert_object(&first, &a)?;

    fixture.remove(Database::Primary, Pack::A, Component::Index)?;
    assert!(
        !fixture.is_valid(),
        "component-wise deletion exposes its intermediate state"
    );
    assert_missing(&second, &fixture.manifest.missing_id())?;
    fixture.remove_pack(Database::Primary, Pack::A)?;
    assert!(fixture.is_valid(), "the completed removal is valid again");

    let current = open(&fixture, 8)?;
    assert_missing(&current, &a)?;
    assert_object(&first, &b)?;
    Ok(())
}

#[test]
fn stale_handles_follow_multi_index_rewrites() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.install_pack(Database::Primary, Pack::B)?;
    fixture.write_multi_index(Database::Primary, &[Pack::A, Pack::B])?;
    let first = open(&fixture, 8)?;
    let second = first.clone();
    let a = fixture.manifest.pack(Pack::A).object_ids[0];
    let c = fixture.manifest.pack(Pack::C).object_ids[0];
    assert_object(&first, &a)?;

    fixture.install_pack(Database::Primary, Pack::C)?;
    fixture.write_multi_index(Database::Primary, &[Pack::A, Pack::B, Pack::C])?;
    assert_object(&second, &c)?;

    fixture.write_multi_index(Database::Primary, &[Pack::B, Pack::C])?;
    fixture.remove_pack(Database::Primary, Pack::A)?;
    assert_missing(&second, &fixture.manifest.missing_id())?;
    let current = open(&fixture, 8)?;
    assert_missing(&current, &a)?;
    assert_object(&current, &c)?;
    Ok(())
}

#[test]
fn a_multi_index_subset_and_standalone_index_use_their_own_packs() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.install_pack(Database::Primary, Pack::B)?;
    fixture.write_multi_index(Database::Primary, &[Pack::A])?;
    fixture.publish(Database::Alternate, Pack::A, Component::Index)?;
    fixture.publish(Database::Alternate, Pack::C, Component::Index)?;
    fixture.install_pack(Database::Alternate, Pack::B)?;
    fixture.write_multi_index(Database::Alternate, &[Pack::A, Pack::C])?;
    fixture.set_alternate(true)?;

    let handle = open(&fixture, 16)?;
    assert_object(&handle, &fixture.manifest.pack(Pack::A).object_ids[0])?;
    assert_object(&handle, &fixture.manifest.pack(Pack::B).object_ids[0])?;
    Ok(())
}

#[test]
fn alternates_can_change_while_handles_are_alive() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Alternate, Pack::C)?;
    let first = open(&fixture, 8)?;
    let second = first.clone();
    let id = fixture.manifest.pack(Pack::C).object_ids[0];
    assert_missing(&first, &id)?;

    fixture.set_alternate(true)?;
    assert_object(&second, &id)?;

    fixture.set_alternate(false)?;
    assert_missing(&second, &fixture.manifest.missing_id())?;
    let current = open(&fixture, 8)?;
    assert_missing(&current, &id)?;
    Ok(())
}

#[test]
fn malformed_index_can_be_restored_for_an_existing_handle() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.corrupt_index(Database::Primary, Pack::A)?;
    assert!(!fixture.is_valid(), "the helper tracks the malformed component");
    let handle = open(&fixture, 4)?;
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    let mut buffer = Vec::new();
    handle
        .try_find(&id, &mut buffer)
        .expect_err("a malformed index is reported to the caller");

    fixture.publish(Database::Primary, Pack::A, Component::Index)?;
    assert!(fixture.is_valid(), "restoring the generated index makes the ODB valid");
    assert_missing(&handle, &fixture.manifest.missing_id())?;
    assert_object(&handle, &id)?;
    Ok(())
}

#[test]
fn a_pack_missing_on_first_access_is_shared_and_recovers() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let mut first = open(&fixture, 4)?;
    let mut second = first.clone();
    let strict = first.clone();
    first.refresh_never();
    second.refresh_never();
    assert_eq!(
        strict.packed_object_count()?,
        fixture.manifest.pack(Pack::A).object_ids.len() as u64,
        "loading the index does not require opening its pack"
    );

    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    fixture.remove(Database::Primary, Pack::A, Component::Pack)?;
    assert_missing(&first, &id)?;
    assert_missing(&second, &id)?;
    assert_eq!(
        strict.store_ref().metrics().num_refreshes,
        1,
        "never-refresh handles share the missing pack state without scanning the directory"
    );

    fixture.publish(Database::Primary, Pack::A, Component::Pack)?;
    assert_missing(&strict, &fixture.manifest.missing_id())?;
    assert_object(&strict, &id)?;
    assert_object(&first, &id)?;
    assert_object(&second, &id)?;
    Ok(())
}

#[test]
fn a_loaded_index_recovers_after_its_pack_and_index_are_republished() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let mut stale = open(&fixture, 4)?;
    let current = stale.clone();
    stale.refresh_never();
    assert_eq!(
        current.packed_object_count()?,
        fixture.manifest.pack(Pack::A).object_ids.len() as u64,
        "the shared store loads the index before its files are replaced"
    );

    fixture.remove(Database::Primary, Pack::A, Component::Pack)?;
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    assert_missing(&stale, &id)?;
    fixture.publish(Database::Primary, Pack::A, Component::Index)?;
    fixture.publish(Database::Primary, Pack::A, Component::Pack)?;

    assert_missing(&current, &fixture.manifest.missing_id())?;
    assert_object(&current, &id)?;
    Ok(())
}

#[cfg(feature = "parallel")]
#[test]
fn a_failed_pack_load_is_shared_and_recovers_after_replacement() -> Result {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    use gix_odb::store::init::debug::{LoadOutcome, Point};

    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let (point_tx, point_rx) = crossbeam_channel::unbounded();
    let (resume_tx, resume_rx) = crossbeam_channel::bounded(0);
    let pause_first_loader = Arc::new(AtomicBool::new(true));
    let debug = gix_odb::store::init::debug::Options::new({
        let pause_first_loader = Arc::clone(&pause_first_loader);
        move |point| {
            point_tx
                .send(point)
                .expect("the test receives every synchronization point");
            if matches!(point, Point::PackSlotLocked { .. }) && pause_first_loader.swap(false, Ordering::SeqCst) {
                resume_rx.recv().expect("the test releases the first pack loader");
            }
        }
    });
    let first = gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        Vec::new(),
        gix_odb::store::init::Options {
            slots: gix_odb::store::init::Slots::Limit(4),
            debug: Some(debug),
            ..Default::default()
        },
    )?
    .into_arc()?;
    let second = first.clone();
    let recovery = first.clone();
    assert_eq!(
        first.packed_object_count()?,
        fixture.manifest.pack(Pack::A).object_ids.len() as u64,
        "loading the index leaves its pack unopened"
    );
    point_rx.try_iter().for_each(drop);

    fixture.corrupt_pack(Database::Primary, Pack::A)?;
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    let lookup = move |handle: gix_odb::HandleArc| {
        std::thread::spawn(move || {
            let mut buffer = Vec::new();
            handle
                .try_find(&id, &mut buffer)
                .map(|object| object.is_some())
                .map_err(|err| err.to_string())
        })
    };
    let first_thread = lookup(first);
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the first loader acquires the pack slot");
        if matches!(point, Point::PackSlotLocked { .. }) {
            break;
        }
    }
    let second_thread = lookup(second);
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the second loader attempts to acquire the pack slot");
        if matches!(point, Point::PackSlotLocking { .. }) {
            break;
        }
    }
    resume_tx.send(()).expect("the first loader is waiting for its release");

    let first_err = first_thread
        .join()
        .expect("the first loader does not panic")
        .expect_err("the first handle observes the malformed pack");
    let second_err = second_thread
        .join()
        .expect("the second loader does not panic")
        .expect_err("the second handle observes the shared pack-load failure");
    assert_eq!(
        first_err, second_err,
        "shared handles report the same pack-load failure"
    );
    let observed: Vec<_> = point_rx.try_iter().collect();
    assert_eq!(
        observed
            .iter()
            .filter(|point| matches!(
                point,
                Point::PackLoadCompleted {
                    outcome: LoadOutcome::Failure,
                    ..
                }
            ))
            .count(),
        1,
        "the shared malformed pack is opened exactly once"
    );

    let mut buffer = Vec::new();
    assert_eq!(
        recovery
            .try_find(&id, &mut buffer)
            .expect_err("the unchanged malformed pack remains an error")
            .to_string(),
        first_err,
        "later lookups observe the cached error"
    );
    assert!(
        point_rx
            .try_iter()
            .all(|point| !matches!(point, Point::PackLoadCompleted { .. })),
        "the unchanged malformed pack is not loaded again"
    );
    fixture.publish(Database::Primary, Pack::A, Component::Pack)?;
    assert_object(&recovery, &id)?;
    assert_eq!(
        point_rx
            .try_iter()
            .filter(|point| matches!(
                point,
                Point::PackLoadCompleted {
                    outcome: LoadOutcome::Success,
                    ..
                }
            ))
            .count(),
        1,
        "the replacement pack is loaded exactly once"
    );
    Ok(())
}

#[test]
fn slot_exhaustion_keeps_the_loaded_pack_usable() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let handle = open(&fixture, 1)?;
    let a = fixture.manifest.pack(Pack::A).object_ids[0];
    let b = fixture.manifest.pack(Pack::B).object_ids[0];
    assert_object(&handle, &a)?;

    fixture.install_pack(Database::Primary, Pack::B)?;
    let mut buffer = Vec::new();
    let err = handle
        .try_find(&b, &mut buffer)
        .expect_err("one slot cannot hold two indices");
    assert!(
        err.to_string().contains("slot"),
        "slot exhaustion is reported through the lookup error: {err}"
    );
    assert_object(&handle, &a)?;
    Ok(())
}

#[test]
fn never_refresh_does_not_retry_failed_initial_scan() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.install_pack(Database::Primary, Pack::B)?;
    let mut never = open(&fixture, 1)?;
    let strict = never.clone();
    never.refresh_never();
    let missing = fixture.manifest.missing_id();
    let mut buffer = Vec::new();

    let err = never
        .try_find(&missing, &mut buffer)
        .expect_err("one slot cannot hold two indices during lazy initialization");
    assert!(
        err.to_string().contains("slot"),
        "the initial slot exhaustion is reported through the lookup error: {err}"
    );
    assert_eq!(
        never.store_ref().metrics().num_refreshes,
        1,
        "the initial lookup scans the pack directory once"
    );

    assert_missing(&never, &missing)?;
    assert_eq!(
        never.store_ref().metrics().num_refreshes,
        1,
        "the never-refresh handle does not retry the failed scan"
    );

    fixture.remove_pack(Database::Primary, Pack::B)?;
    assert_object(&strict, &fixture.manifest.pack(Pack::A).object_ids[0])?;
    assert_eq!(
        strict.store_ref().metrics().num_refreshes,
        2,
        "a strict shared handle can reconcile once the remaining index fits"
    );
    Ok(())
}

#[test]
fn refresh_after_duration_is_shared_by_handles_until_the_deadline() -> Result {
    use std::{
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    let mut fixture = OdbFixture::from_script()?;
    let now = Arc::new(Mutex::new(Instant::now()));
    let debug = gix_odb::store::init::debug::Options::new(|_| {}).with_clock({
        let now = Arc::clone(&now);
        move || *now.lock().expect("the test clock isn't poisoned")
    });
    let mut first = gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        Vec::new(),
        gix_odb::store::init::Options {
            slots: gix_odb::store::init::Slots::Growable { initial: 1 },
            debug: Some(debug),
            ..Default::default()
        },
    )?;
    let refresh_after = Duration::from_secs(1);
    first.refresh = gix_odb::store::RefreshMode::AfterDuration(refresh_after);
    let second = first.clone();
    let id = fixture.manifest.pack(Pack::A).object_ids[0];

    assert_eq!(
        first.packed_object_count()?,
        0,
        "the empty store initializes without packed objects"
    );
    assert_eq!(
        first.store_ref().metrics().num_refreshes,
        1,
        "initialization scans the shared store once"
    );
    fixture.install_pack(Database::Primary, Pack::A)?;
    assert_object(&second, &id)?;
    assert_eq!(
        first.store_ref().metrics().num_refreshes,
        2,
        "initialization leaves the first miss-triggered refresh available to discover published objects"
    );
    assert_missing(&first, &fixture.manifest.missing_id())?;
    assert_eq!(
        first.store_ref().metrics().num_refreshes,
        2,
        "the other handle shares the freshness window established by the refresh"
    );
    fixture.install_pack(Database::Primary, Pack::B)?;
    let newly_published = fixture.manifest.pack(Pack::B).object_ids[0];
    assert_missing(&second, &newly_published)?;
    first.store_ref().mark_disk_state_stale();
    assert_object(&second, &newly_published)?;
    assert_eq!(
        first.store_ref().metrics().num_refreshes,
        3,
        "marking a known mutation stale makes the next lookup refresh without performing eager I/O"
    );

    {
        let mut now = now.lock().expect("the test clock isn't poisoned");
        *now += refresh_after;
    }
    assert_missing(&second, &fixture.manifest.missing_id())?;
    assert_eq!(
        first.store_ref().metrics().num_refreshes,
        4,
        "the first handle to miss at the deadline refreshes the shared store"
    );
    Ok(())
}

#[test]
fn a_failed_refresh_does_not_renew_the_freshness_window() -> Result {
    use std::{
        sync::{Arc, Mutex},
        time::{Duration, Instant},
    };

    let mut fixture = OdbFixture::from_script()?;
    let now = Arc::new(Mutex::new(Instant::now()));
    let debug = gix_odb::store::init::debug::Options::new(|_| {}).with_clock({
        let now = Arc::clone(&now);
        move || *now.lock().expect("the test clock isn't poisoned")
    });
    let mut handle = gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        Vec::new(),
        gix_odb::store::init::Options {
            slots: gix_odb::store::init::Slots::Limit(1),
            debug: Some(debug),
            ..Default::default()
        },
    )?;
    let refresh_after = Duration::from_secs(1);
    handle.refresh = gix_odb::store::RefreshMode::AfterDuration(refresh_after);
    assert_eq!(
        handle.packed_object_count()?,
        0,
        "the empty store initializes without packed objects"
    );

    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.install_pack(Database::Primary, Pack::B)?;
    let mut buffer = Vec::new();
    handle
        .try_find(&fixture.manifest.missing_id(), &mut buffer)
        .expect_err("one slot cannot hold both newly discovered indices");
    assert_eq!(
        handle.store_ref().metrics().num_refreshes,
        2,
        "the failed refresh is attempted once"
    );

    fixture.remove_pack(Database::Primary, Pack::B)?;
    assert_object(&handle, &fixture.manifest.pack(Pack::A).object_ids[0])?;
    assert_eq!(
        handle.store_ref().metrics().num_refreshes,
        3,
        "the next lookup retries because failure did not renew freshness"
    );
    Ok(())
}

#[test]
fn a_missing_known_pack_bypasses_the_refresh_deadline() -> Result {
    use std::time::Duration;

    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let mut handle = open(&fixture, 1)?;
    handle.refresh = gix_odb::store::RefreshMode::AfterDuration(Duration::MAX);
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    assert_eq!(
        handle.packed_object_count()?,
        fixture.manifest.pack(Pack::A).object_ids.len() as u64,
        "the index is known while its pack remains unopened"
    );
    fixture.remove(Database::Primary, Pack::A, Component::Pack)?;

    assert_missing(&handle, &id)?;
    assert_eq!(
        handle.store_ref().metrics().num_refreshes,
        2,
        "losing a pack named by a known index forces structural reconciliation"
    );
    Ok(())
}

#[test]
fn a_midx_rewrite_does_not_require_a_spare_slot() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.write_multi_index(Database::Primary, &[Pack::A])?;
    let handle = open(&fixture, 1)?;
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    assert_object(&handle, &id)?;

    fixture.write_multi_index(Database::Primary, &[Pack::A])?;
    assert_missing(&handle, &fixture.manifest.missing_id())?;
    assert_object(&handle, &id)?;
    assert_eq!(
        handle.store_ref().metrics().known_reachable_indices,
        1,
        "rewriting the only reachable index stays within the configured limit"
    );
    Ok(())
}

#[test]
fn slot_exhaustion_during_midx_rewrite_keeps_the_loaded_pack_usable() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let handle = open(&fixture, 1)?;
    let a = fixture.manifest.pack(Pack::A).object_ids[0];
    let b = fixture.manifest.pack(Pack::B).object_ids[9];
    assert_missing(&handle, &fixture.manifest.missing_id())?;

    fixture.install_pack(Database::Primary, Pack::B)?;
    fixture.write_multi_index(Database::Primary, &[Pack::A])?;
    let err = handle
        .try_header(&b)
        .expect_err("one slot cannot hold the MIDX and the remaining standalone index");
    assert!(
        err.to_string().contains("slot"),
        "slot exhaustion is reported through the header lookup error: {err}"
    );
    assert_object(&handle, &a)?;
    Ok(())
}

#[test]
fn a_multi_index_can_replace_a_standalone_index_in_the_same_slot() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let handle = open(&fixture, 1)?;
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    assert_eq!(
        handle.packed_object_count()?,
        fixture.manifest.pack(Pack::A).object_ids.len() as u64,
        "the standalone index is loaded without opening its pack"
    );

    fixture.remove(Database::Primary, Pack::A, Component::Pack)?;
    fixture.write_multi_index(Database::Primary, &[Pack::A])?;
    assert_missing(&handle, &id)?;

    fixture.publish(Database::Primary, Pack::A, Component::Pack)?;
    assert_object(&handle, &id)?;
    Ok(())
}

#[test]
fn growable_slots_expand_without_invalidating_existing_handles() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    let first = open_with_slots(&fixture, gix_odb::store::init::Slots::Growable { initial: 1 })?;
    let second = first.clone();
    let mut stable = first.clone();
    stable.prevent_pack_unload();
    assert_eq!(
        first.store_ref().metrics().num_refreshes,
        0,
        "opening a growable store does not scan the pack directory"
    );

    fixture.install_pack(Database::Primary, Pack::A)?;
    let a = fixture.manifest.pack(Pack::A).object_ids[0];
    assert_object(&first, &a)?;
    let mut buffer = Vec::new();
    let location = gix_odb::pack::Find::location_by_oid(&stable, &a, &mut buffer)
        .expect("the stable handle locates the first pack");

    fixture.install_pack(Database::Primary, Pack::B)?;
    fixture.install_pack(Database::Primary, Pack::C)?;
    let b = fixture.manifest.pack(Pack::B).object_ids[0];
    let c = fixture.manifest.pack(Pack::C).object_ids[0];
    assert_object(&second, &c)?;
    assert_object(&first, &b)?;
    assert_object(&stable, &a)?;
    assert!(
        gix_odb::pack::Find::entry_by_location(&stable, &location).is_some(),
        "growing the slot map preserves stable pack locations"
    );
    assert_eq!(
        first.store_ref().metrics().known_reachable_indices,
        3,
        "all standalone indices are represented after growth"
    );
    Ok(())
}

#[test]
fn making_a_stale_handle_stable_discards_invalid_pack_ids() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let current = open(&fixture, 1)?;
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    assert_object(&current, &id)?;
    let mut stable = current.clone();

    fixture.write_multi_index(Database::Primary, &[Pack::A])?;
    assert_missing(&current, &fixture.manifest.missing_id())?;

    stable.prevent_pack_unload();
    let mut buffer = Vec::new();
    let location = gix_odb::pack::Find::location_by_oid(&stable, &id, &mut buffer)
        .expect("the stable handle locates the object through the current MIDX");
    assert!(
        gix_odb::pack::Find::location_by_oid(&stable, &fixture.manifest.pack(Pack::C).object_ids[0], &mut buffer)
            .is_none(),
        "a miss may refresh the stable handle without invalidating its location"
    );
    assert!(
        gix_odb::pack::Find::entry_by_location(&stable, &location).is_some(),
        "the location obtained after stability was enabled remains valid"
    );
    Ok(())
}

#[test]
fn a_midx_rewrite_does_not_reuse_a_stable_standalone_pack_id() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.install_pack(Database::Primary, Pack::B)?;
    fixture.write_multi_index(Database::Primary, &[Pack::A])?;
    let mut stable = open(&fixture, 2)?;
    stable.prevent_pack_unload();
    let mut buffer = Vec::new();
    let b = fixture.manifest.pack(Pack::B).object_ids[0];
    let location = gix_odb::pack::Find::location_by_oid(&stable, &b, &mut buffer)
        .expect("the standalone pack is available before the MIDX absorbs it");

    fixture.write_multi_index(Database::Primary, &[Pack::A, Pack::B])?;
    assert!(
        gix_odb::pack::Find::location_by_oid(&stable, &fixture.manifest.pack(Pack::C).object_ids[0], &mut buffer)
            .is_none(),
        "slot exhaustion during the rewrite is reported as a miss by the infallible location API"
    );
    assert!(
        gix_odb::pack::Find::entry_by_location(&stable, &location).is_some(),
        "the standalone pack ID remains mapped after the failed rewrite"
    );
    Ok(())
}

#[test]
fn a_stable_location_keeps_a_deleted_midx_pack_available_across_refresh() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.publish(Database::Primary, Pack::A, Component::Pack)?;
    fixture.publish(Database::Primary, Pack::A, Component::Index)?;
    fixture.write_multi_index(Database::Primary, &[Pack::A])?;
    let mut stable = open(&fixture, 16)?;
    stable.prevent_pack_unload();
    let mut buffer = Vec::new();
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    let location = gix_odb::pack::Find::location_by_oid(&stable, &id, &mut buffer)
        .expect("the stable handle locates the object through the MIDX");

    fixture.remove(Database::Primary, Pack::A, Component::Pack)?;
    assert!(
        gix_odb::pack::Find::location_by_oid(&stable, &fixture.manifest.pack(Pack::C).object_ids[0], &mut buffer)
            .is_none(),
        "a lookup miss refreshes the stable handle after the pack was deleted"
    );

    assert!(
        gix_odb::pack::Find::entry_by_location(&stable, &location).is_some(),
        "the deleted pack remains available by its previously returned location"
    );
    Ok(())
}

#[test]
fn iteration_objects_are_readable_after_an_unchanged_index_gets_its_pack_back() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let current = open(&fixture, 4)?;
    let mut stale = current.clone();
    stale.refresh_never();
    assert_eq!(
        current.packed_object_count()?,
        fixture.manifest.pack(Pack::A).object_ids.len() as u64,
        "the shared store loads the index before its pack disappears"
    );

    fixture.remove(Database::Primary, Pack::A, Component::Pack)?;
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    assert_missing(&stale, &id)?;
    fixture.publish(Database::Primary, Pack::A, Component::Pack)?;

    for id in current.iter()? {
        assert_object(&current, &id?)?;
    }
    Ok(())
}

#[test]
fn a_multi_index_with_a_missing_pack_does_not_refresh_forever() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.publish(Database::Primary, Pack::A, Component::Index)?;
    fixture.install_pack(Database::Primary, Pack::B)?;
    fixture.write_multi_index(Database::Primary, &[Pack::A, Pack::B])?;
    let handle = open(&fixture, 4)?;

    assert_missing(&handle, &fixture.manifest.pack(Pack::A).object_ids[0])?;
    assert_object(&handle, &fixture.manifest.pack(Pack::B).object_ids[0])?;
    Ok(())
}

#[test]
fn disk_sized_slots_can_grow_beyond_the_opening_estimate() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    let handle = open_with_slots(
        &fixture,
        gix_odb::store::init::Slots::AsNeededByDiskState {
            multiplier: 1.0,
            minimum: 1,
        },
    )?;

    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.install_pack(Database::Primary, Pack::B)?;
    assert_object(&handle, &fixture.manifest.pack(Pack::B).object_ids[0])?;
    assert_eq!(
        handle.store_ref().metrics().known_reachable_indices,
        2,
        "a disk-sized store grows when later maintenance adds more indices than estimated"
    );
    Ok(())
}
