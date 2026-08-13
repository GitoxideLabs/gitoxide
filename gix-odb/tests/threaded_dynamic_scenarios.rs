use gix_object::Find as _;
use gix_odb::Header as _;

pub use gix_testtools::Result;
#[path = "tools/odb.rs"]
pub mod odb_fixture;
#[path = "tools/scenario.rs"]
mod support;
use odb_fixture::{Component, Database, OdbFixture, Pack};
use support::assert_object_once as assert_object;

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

#[test]
fn healthy_index_loads_do_not_block_independent_slots() -> Result {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    use gix_odb::store::init::debug::Point;

    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.install_pack(Database::Primary, Pack::B)?;
    let (locked_tx, locked_rx) = crossbeam_channel::unbounded();
    let (resume_tx, resume_rx) = crossbeam_channel::bounded(1);
    let pause_first_loader = AtomicBool::new(true);
    let store = Arc::new(gix_odb::Store::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        &mut std::iter::empty(),
        gix_odb::store::init::Options {
            debug: Some(gix_odb::store::init::debug::Options::new(move |point| {
                if let Point::IndexSlotLocked { slot } = point {
                    locked_tx.send(slot).expect("the test receives locked slots");
                    if pause_first_loader.swap(false, Ordering::SeqCst) {
                        resume_rx.recv().expect("the test releases the first loader");
                    }
                }
            })),
            ..Default::default()
        },
    )?);
    store.structure()?;
    let first = store.to_handle_arc();
    let second = first.clone();
    let object_id = fixture.manifest.pack(Pack::B).object_ids[0];
    let lookup = move |handle: gix_odb::store::Handle<Arc<gix_odb::Store>>| {
        std::thread::spawn(move || {
            handle
                .try_find(&object_id, &mut Vec::new())
                .map(|object| object.is_some())
        })
    };
    let first_thread = lookup(first);
    let first_slot = locked_rx.recv_timeout(Duration::from_secs(10));
    let second_thread = lookup(second);
    let second_slot = locked_rx.recv_timeout(Duration::from_secs(10));
    resume_tx
        .send(())
        .expect("the first loader can resume even if the second blocked");
    let first_result = first_thread.join().expect("the first lookup does not panic");
    let second_result = second_thread.join().expect("the second lookup does not panic");
    assert_ne!(
        first_slot.expect("the first loader locks a slot"),
        second_slot.expect("an independent slot loads before the first loader resumes"),
        "each loader owns a different index slot"
    );
    assert!(
        first_result? && second_result?,
        "both handles find the requested object"
    );
    Ok(())
}

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
    let (resume_observer_tx, resume_observer_rx) = crossbeam_channel::bounded(0);
    let pause_next_refresh = Arc::new(AtomicBool::new(false));
    let pause_next_observer = Arc::new(AtomicBool::new(false));
    let now = Arc::new(Mutex::new(Instant::now()));
    let debug = gix_odb::store::init::debug::Options::new({
        let pause_next_refresh = Arc::clone(&pause_next_refresh);
        let pause_next_observer = Arc::clone(&pause_next_observer);
        move |point| {
            point_tx
                .send(point)
                .expect("the test receives every synchronization point");
            if matches!(point, Point::RefreshScanStarted) && pause_next_refresh.swap(false, Ordering::SeqCst) {
                resume_rx.recv().expect("the test releases the refresh scan");
            }
            if matches!(point, Point::RefreshCompletionObserved) && pause_next_observer.swap(false, Ordering::SeqCst) {
                resume_observer_rx.recv().expect("the test releases the observer");
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
    let first_thread = lookup(handle.clone());
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
    pause_next_observer.store(true, Ordering::SeqCst);
    let second_thread = lookup(handle.clone());
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the second handle observes refresh completion");
        observed.push(point);
        if matches!(point, Point::RefreshCompletionObserved) {
            break;
        }
    }
    resume_tx.send(()).expect("the first refresh is waiting for release");
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the first handle completes its refresh scan");
        observed.push(point);
        if matches!(
            point,
            Point::RefreshScanCompleted {
                outcome: gix_odb::store::init::debug::LoadOutcome::Success
            }
        ) {
            break;
        }
    }
    resume_observer_tx
        .send(())
        .expect("the second observer is waiting for release");
    let first = first_thread.join().expect("the first lookup does not panic");
    let second = second_thread.join().expect("the second lookup does not panic");
    observed.extend(point_rx.try_iter());
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
fn invalidation_during_refresh_preserves_newly_published_packs() -> Result {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };

    use gix_odb::store::init::debug::Point;

    let mut fixture = OdbFixture::from_script()?;
    let (published_tx, published_rx) = crossbeam_channel::bounded(1);
    let (resume_tx, resume_rx) = crossbeam_channel::bounded(1);
    let pause_next_publication = Arc::new(AtomicBool::new(false));
    let now = Instant::now();
    let debug = gix_odb::store::init::debug::Options::new({
        let pause_next_publication = Arc::clone(&pause_next_publication);
        move |point| {
            if matches!(point, Point::IndexStatePublished) && pause_next_publication.swap(false, Ordering::SeqCst) {
                published_tx
                    .send(())
                    .expect("the test observes publication after scanning");
                resume_rx.recv().expect("the test resumes the refresh");
            }
        }
    })
    .with_clock(move || now);
    let store = Arc::new(gix_odb::Store::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        &mut std::iter::empty(),
        gix_odb::store::init::Options {
            debug: Some(debug),
            ..Default::default()
        },
    )?);
    store.structure()?;
    let mut first = store.to_handle_arc();
    first.refresh = gix_odb::store::RefreshMode::AfterDuration(Duration::from_secs(1));
    let second = first.clone();
    fixture.install_pack(Database::Primary, Pack::A)?;
    let object_id = fixture.manifest.pack(Pack::A).object_ids[0];
    pause_next_publication.store(true, Ordering::SeqCst);
    let refresh = std::thread::spawn(move || {
        first
            .try_find(&object_id, &mut Vec::new())
            .map(|object| object.is_some())
    });
    published_rx.recv_timeout(Duration::from_secs(10))?;

    fixture.install_pack(Database::Primary, Pack::B)?;
    let (started_tx, started_rx) = crossbeam_channel::bounded(1);
    let (invalidated_tx, invalidated_rx) = crossbeam_channel::bounded(1);
    let invalidation = std::thread::spawn(move || {
        started_tx.send(()).expect("the test observes the invalidating thread");
        store.mark_disk_state_stale();
        invalidated_tx
            .send(())
            .expect("the test observes completed invalidation");
    });
    started_rx.recv_timeout(Duration::from_secs(10))?;
    // An uncoordinated invalidation can finish while the scan is paused; a coordinated one waits for its release.
    let _ = invalidated_rx.recv_timeout(Duration::from_secs(1));
    resume_tx
        .send(())
        .expect("the refresh can complete before checking invalidation");
    assert!(
        refresh.join().expect("the refresh does not panic")?,
        "the refresh sees the pack published before its scan"
    );
    invalidation.join().expect("invalidation does not panic");
    let object_id = fixture.manifest.pack(Pack::B).object_ids[0];
    assert!(
        second.try_find(&object_id, &mut Vec::new())?.is_some(),
        "invalidation after the directory scan survives refresh completion without advancing the clock"
    );
    Ok(())
}
#[test]
fn an_index_load_is_registered_before_its_slot_is_reserved() -> Result {
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
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    let (point_tx, point_rx) = crossbeam_channel::unbounded();
    let (resume_loader_tx, resume_loader_rx) = crossbeam_channel::bounded(0);
    let (resume_waiter_tx, resume_waiter_rx) = crossbeam_channel::bounded(0);
    let pause_loader = Arc::new(AtomicBool::new(true));
    let pause_waiter = Arc::new(AtomicBool::new(true));
    let debug = gix_odb::store::init::debug::Options::new({
        let pause_loader = Arc::clone(&pause_loader);
        let pause_waiter = Arc::clone(&pause_waiter);
        move |point| {
            point_tx
                .send(point)
                .expect("the test receives every synchronization point");
            if matches!(point, Point::IndexLoadClaimed { .. }) && pause_loader.swap(false, Ordering::SeqCst) {
                resume_loader_rx.recv().expect("the test releases the index loader");
            } else if matches!(point, Point::IndexLoadWaiting | Point::SnapshotWaitingForIndexLoad)
                && pause_waiter.swap(false, Ordering::SeqCst)
            {
                resume_waiter_rx.recv().expect("the test releases the waiting lookup");
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

    let lookup = move |handle: gix_odb::HandleArc| {
        std::thread::spawn(move || {
            handle
                .try_header(&id)
                .map(|header| header.is_some())
                .map_err(|err| err.to_string())
        })
    };
    let first_thread = lookup(first);
    let mut observed = Vec::new();
    let active_loads = loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the first handle reserves the index slot");
        observed.push(point);
        if let Point::IndexLoadClaimed { active_loads, .. } = point {
            break active_loads;
        }
    };

    let second_thread = lookup(second);
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the second handle waits for the reserved index");
        observed.push(point);
        if matches!(point, Point::IndexLoadWaiting | Point::SnapshotWaitingForIndexLoad) {
            break;
        }
    }
    resume_loader_tx
        .send(())
        .expect("the first loader is waiting for its release");
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the first handle locks its claimed index slot");
        observed.push(point);
        if matches!(point, Point::IndexSlotLocked { .. }) {
            break;
        }
    }
    resume_waiter_tx
        .send(())
        .expect("the second lookup is waiting for its release");

    let first_found = first_thread
        .join()
        .expect("the first lookup does not panic")
        .expect("the first lookup succeeds");
    let second_found = second_thread
        .join()
        .expect("the second lookup does not panic")
        .expect("the waiting lookup succeeds");
    observed.extend(point_rx.try_iter());

    assert_eq!(
        active_loads, 1,
        "a reserved slot is already registered as an active load"
    );
    assert!(first_found, "the loading handle finds the packed object");
    assert!(second_found, "the waiting handle finds the packed object");
    assert_eq!(
        observed
            .iter()
            .filter(|point| matches!(
                point,
                Point::IndexLoadCompleted {
                    outcome: LoadOutcome::Success,
                    ..
                }
            ))
            .count(),
        1,
        "the contending lookups load the index once"
    );
    Ok(())
}
#[test]
fn a_handle_rechecks_its_snapshot_after_waiting_for_an_index_loader() -> Result {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    use gix_odb::store::init::debug::Point;

    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.install_pack(Database::Alternate, Pack::B)?;
    fixture.corrupt_index(Database::Alternate, Pack::B)?;
    fixture.set_alternate(true)?;

    let (point_tx, point_rx) = crossbeam_channel::unbounded();
    let (resume_waiter_tx, resume_waiter_rx) = crossbeam_channel::bounded(0);
    let pause_waiter = Arc::new(AtomicBool::new(false));
    let debug = gix_odb::store::init::debug::Options::new({
        let pause_waiter = Arc::clone(&pause_waiter);
        move |point| {
            point_tx
                .send(point)
                .expect("the test receives every synchronization point");
            if matches!(point, Point::IndexRetrySlotLocking { .. }) && pause_waiter.swap(false, Ordering::SeqCst) {
                resume_waiter_rx
                    .recv()
                    .expect("the test releases the stale handle's retry");
            }
        }
    });
    let mut handle = gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        Vec::new(),
        gix_odb::store::init::Options {
            slots: gix_odb::store::init::Slots::Limit(2),
            debug: Some(debug),
            ..Default::default()
        },
    )?
    .into_arc()?;
    assert_object(&handle, &fixture.manifest.pack(Pack::A).object_ids[0])?;
    let second_object_id = fixture.manifest.pack(Pack::B).object_ids[0];
    handle
        .try_header(&second_object_id)
        .expect_err("the second index records a failed load before it can be retried");
    handle.refresh_never();
    let stale = handle.clone();
    let loader = handle.clone();
    fixture.publish(Database::Alternate, Pack::B, Component::Index)?;
    point_rx.try_iter().for_each(drop);

    let lookup = move |handle: gix_odb::HandleArc| {
        std::thread::spawn(move || {
            handle
                .try_header(&second_object_id)
                .map(|header| header.is_some())
                .map_err(|err| err.to_string())
        })
    };
    pause_waiter.store(true, Ordering::SeqCst);
    let stale_thread = lookup(stale);
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the stale handle is about to lock the failed index for a retry");
        if matches!(point, Point::IndexRetrySlotLocking { .. }) {
            break;
        }
    }
    // Another loader publishes the repaired index while the stale handle is paused before acquiring its retry lock.
    let loaded = lookup(loader).join().expect("the loader does not panic");
    resume_waiter_tx
        .send(())
        .expect("the stale handle is waiting for its release");

    assert!(
        loaded.expect("the loader lookup succeeds"),
        "the loading handle finds the object"
    );
    assert!(
        stale_thread
            .join()
            .expect("the stale lookup does not panic")
            .expect("the stale lookup succeeds"),
        "the stale handle observes the index published while it waited"
    );
    Ok(())
}

fn assert_with_handles(handle: &gix_odb::Handle, assertion: impl Fn(&gix_odb::Handle) -> Result + Sync) -> Result {
    let threads = test_threads()?;
    if threads == 1 {
        return assertion(handle);
    }

    let barrier = std::sync::Barrier::new(threads);
    std::thread::scope(|scope| {
        let workers = (1..threads)
            .map(|_| {
                let handle = handle.clone();
                let barrier = &barrier;
                let assertion = &assertion;
                scope.spawn(move || {
                    barrier.wait();
                    assertion(&handle)
                })
            })
            .collect::<Vec<_>>();

        barrier.wait();
        let mut outcome = assertion(handle);
        for worker in workers {
            let worker = worker.join().expect("a contending assertion does not panic");
            if outcome.is_ok() {
                outcome = worker;
            }
        }
        outcome
    })
}

#[path = "odb/store/dynamic_scenarios.rs"]
mod scenarios;

fn test_threads() -> Result<usize> {
    Ok(match std::env::var_os("GIX_ODB_TEST_THREADS") {
        Some(value) => value
            .into_string()
            .map_err(|_| std::io::Error::other("GIX_ODB_TEST_THREADS must be valid UTF-8"))?
            .parse::<std::num::NonZeroUsize>()
            .map_err(|err| std::io::Error::other(format!("invalid GIX_ODB_TEST_THREADS: {err}")))?
            .get(),
        None => 1,
    })
}

fn assert_cohort(handles: &mut [gix_odb::Handle], assertion: impl Fn(&gix_odb::Handle) -> Result + Sync) -> Result {
    if handles.len() == 1 {
        return assertion(&handles[0]);
    }
    let barrier = std::sync::Barrier::new(handles.len());
    std::thread::scope(|scope| {
        let workers = handles
            .iter_mut()
            .map(|handle| {
                let barrier = &barrier;
                let assertion = &assertion;
                scope.spawn(move || {
                    barrier.wait();
                    assertion(handle)
                })
            })
            .collect::<Vec<_>>();

        let mut outcome = Ok(());
        for worker in workers {
            let worker = worker.join().expect("a persistent contending assertion does not panic");
            if outcome.is_ok() {
                outcome = worker;
            }
        }
        outcome
    })
}
#[test]
fn pack_removal_during_loading_is_observed_by_all_waiters() -> Result {
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
    let pause_loader = Arc::new(AtomicBool::new(true));
    let debug = gix_odb::store::init::debug::Options::new({
        let pause_loader = Arc::clone(&pause_loader);
        move |point| {
            point_tx
                .send(point)
                .expect("the test receives every synchronization point");
            if matches!(point, Point::PackSlotLocked { .. }) && pause_loader.swap(false, Ordering::SeqCst) {
                resume_rx.recv().expect("the test releases the pack loader");
            }
        }
    });
    let handle = gix_odb::at_opts(
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
        handle.packed_object_count()?,
        fixture.manifest.pack(Pack::A).object_ids.len() as u64,
        "the index is loaded while its pack stays unopened"
    );
    point_rx.try_iter().for_each(drop);

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
    let first_thread = lookup(handle.clone());
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the first lookup locks the pack slot");
        if matches!(point, Point::PackSlotLocked { .. }) {
            break;
        }
    }
    let second_thread = lookup(handle.clone());
    loop {
        let point = point_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("the second lookup waits for the pack slot");
        if matches!(point, Point::PackSlotLocking { .. }) {
            break;
        }
    }

    fixture.remove(Database::Primary, Pack::A, Component::Pack)?;
    resume_tx.send(()).expect("the pack loader is waiting for release");
    assert!(
        !first_thread
            .join()
            .expect("the first lookup does not panic")
            .expect("a removed pack is a miss"),
        "the first handle observes the maintenance removal"
    );
    assert!(
        !second_thread
            .join()
            .expect("the second lookup does not panic")
            .expect("a removed pack is a shared miss"),
        "the waiting handle observes the shared missing-pack state"
    );
    assert_eq!(
        point_rx
            .try_iter()
            .filter(|point| matches!(
                point,
                Point::PackLoadCompleted {
                    outcome: LoadOutcome::Failure,
                    ..
                }
            ))
            .count(),
        1,
        "contending handles attempt to open the removed pack once"
    );
    Ok(())
}

#[test]
fn multi_index_rewrite_during_refresh_publishes_one_current_state() -> Result {
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
    fixture.write_multi_index(Database::Primary, &[Pack::A])?;
    let (point_tx, point_rx) = crossbeam_channel::unbounded();
    let (resume_tx, resume_rx) = crossbeam_channel::bounded(0);
    let pause_refresh = Arc::new(AtomicBool::new(false));
    let debug = gix_odb::store::init::debug::Options::new({
        let pause_refresh = Arc::clone(&pause_refresh);
        move |point| {
            point_tx
                .send(point)
                .expect("the test receives every synchronization point");
            if matches!(point, Point::RefreshScanStarted) && pause_refresh.swap(false, Ordering::SeqCst) {
                resume_rx.recv().expect("the test releases the refresh scan");
            }
        }
    });
    let handle = gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        Vec::new(),
        gix_odb::store::init::Options {
            slots: gix_odb::store::init::Slots::Growable { initial: 1 },
            debug: Some(debug),
            ..Default::default()
        },
    )?
    .into_arc()?;
    assert_object(&handle, &fixture.manifest.pack(Pack::A).object_ids[0])?;
    fixture.install_pack(Database::Primary, Pack::B)?;
    point_rx.try_iter().for_each(drop);

    let id = fixture.manifest.pack(Pack::B).object_ids[0];
    pause_refresh.store(true, Ordering::SeqCst);
    let lookup = std::thread::spawn({
        let handle = handle.clone();
        move || {
            let mut buffer = Vec::new();
            handle
                .try_find(&id, &mut buffer)
                .map(|object| object.is_some())
                .map_err(|err| err.to_string())
        }
    });
    loop {
        if matches!(
            point_rx
                .recv_timeout(Duration::from_secs(10))
                .expect("the lookup starts its refresh scan"),
            Point::RefreshScanStarted
        ) {
            break;
        }
    }
    fixture.write_multi_index(Database::Primary, &[Pack::A, Pack::B])?;
    resume_tx.send(()).expect("the refresh scan is waiting for release");
    assert!(
        lookup
            .join()
            .expect("the lookup does not panic")
            .expect("the lookup succeeds after maintenance"),
        "the handle finds the object after the concurrent MIDX rewrite"
    );
    let observed = point_rx.try_iter().collect::<Vec<_>>();
    assert_eq!(
        observed
            .iter()
            .filter(|point| matches!(
                point,
                Point::RefreshScanCompleted {
                    outcome: LoadOutcome::Success
                }
            ))
            .count(),
        1,
        "the lookup completes one successful refresh"
    );
    assert_eq!(
        handle.store_ref().metrics().known_reachable_indices,
        1,
        "the rewritten MIDX is the sole reachable index"
    );
    Ok(())
}

#[test]
fn stable_location_remains_readable_while_slot_growth_is_published() -> Result {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    use gix_odb::store::init::debug::Point;

    let mut fixture = OdbFixture::from_script()?;
    let (point_tx, point_rx) = crossbeam_channel::unbounded();
    let (resume_tx, resume_rx) = crossbeam_channel::bounded(0);
    let pause_publication = Arc::new(AtomicBool::new(false));
    let debug = gix_odb::store::init::debug::Options::new({
        let pause_publication = Arc::clone(&pause_publication);
        move |point| {
            point_tx
                .send(point)
                .expect("the test receives every synchronization point");
            if matches!(point, Point::IndexStatePublished) && pause_publication.swap(false, Ordering::SeqCst) {
                resume_rx.recv().expect("the test releases catalog publication");
            }
        }
    });
    let handle = gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        Vec::new(),
        gix_odb::store::init::Options {
            slots: gix_odb::store::init::Slots::Growable { initial: 1 },
            debug: Some(debug),
            ..Default::default()
        },
    )?
    .into_arc()?;
    let mut stable = handle.clone();
    stable.prevent_pack_unload();
    fixture.install_pack(Database::Primary, Pack::A)?;
    let a = fixture.manifest.pack(Pack::A).object_ids[0];
    assert_object(&stable, &a)?;
    let mut buffer = Vec::new();
    let location = gix_odb::pack::Find::location_by_oid(&stable, &a, &mut buffer)?
        .expect("the stable handle locates the first pack");

    fixture.install_pack(Database::Primary, Pack::B)?;
    fixture.install_pack(Database::Primary, Pack::C)?;
    let c = fixture.manifest.pack(Pack::C).object_ids[0];
    point_rx.try_iter().for_each(drop);
    pause_publication.store(true, Ordering::SeqCst);
    let growth = std::thread::spawn({
        let handle = handle.clone();
        move || {
            let mut buffer = Vec::new();
            handle
                .try_find(&c, &mut buffer)
                .map(|object| object.is_some())
                .map_err(|err| err.to_string())
        }
    });
    loop {
        if matches!(
            point_rx
                .recv_timeout(Duration::from_secs(10))
                .expect("slot growth publishes a new catalog"),
            Point::IndexStatePublished
        ) {
            break;
        }
    }
    assert!(
        gix_odb::pack::Find::entry_by_location(&stable, &location).is_some(),
        "the old stable location remains readable while the grown catalog is being published"
    );
    resume_tx.send(()).expect("catalog publication is waiting for release");
    assert!(
        growth
            .join()
            .expect("the growing lookup does not panic")
            .expect("the growing lookup succeeds"),
        "the lookup finds the object added during slot growth"
    );
    assert_eq!(
        handle.store_ref().metrics().known_reachable_indices,
        3,
        "the grown catalog exposes every installed index"
    );
    assert!(
        gix_odb::pack::Find::entry_by_location(&stable, &location).is_some(),
        "the old stable location remains readable after publication completes"
    );
    Ok(())
}
#[test]
fn stale_handles_mix_operations_after_pack_publication() -> Result {
    use std::{sync::Arc, time::Duration};

    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let handle = scenarios::open(&fixture, 4)?.into_arc()?;
    let a = fixture.manifest.pack(Pack::A).object_ids[0];
    assert_object(&handle, &a)?;

    let mut stable = handle.clone();
    stable.prevent_pack_unload();
    let mut buffer = Vec::new();
    let location = gix_odb::pack::Find::location_by_oid(&stable, &a, &mut buffer)?
        .expect("the stable handle records a location before the mutation");

    let refresh = gix_odb::store::RefreshMode::AfterDuration(Duration::MAX);
    let mut find = handle.clone();
    let mut header = handle.clone();
    let mut prefix = handle.clone();
    let mut iterate = handle.clone();
    let mut missing = handle.clone();
    for handle in [&mut find, &mut header, &mut prefix, &mut iterate, &mut missing] {
        handle.refresh = refresh;
    }
    fixture.install_pack(Database::Primary, Pack::B)?;
    let b = fixture.manifest.pack(Pack::B).object_ids[0];
    handle.store_ref().mark_disk_state_stale();
    let refreshes_before = handle.store_ref().metrics().num_refreshes;

    let barrier = Arc::new(std::sync::Barrier::new(6));
    let (result_tx, result_rx) = crossbeam_channel::bounded(6);
    let workers = [
        std::thread::spawn({
            let barrier = Arc::clone(&barrier);
            let result_tx = result_tx.clone();
            move || {
                barrier.wait();
                let result = assert_object(&find, &b).map_err(|err| err.to_string());
                result_tx.send(result).expect("the result receiver stays alive");
            }
        }),
        std::thread::spawn({
            let barrier = Arc::clone(&barrier);
            let result_tx = result_tx.clone();
            move || {
                barrier.wait();
                let result = header
                    .try_header(&b)
                    .map_err(|err| err.to_string())
                    .and_then(|header| header.ok_or_else(|| "the published object has a header".to_owned()))
                    .map(|_| ());
                result_tx.send(result).expect("the result receiver stays alive");
            }
        }),
        std::thread::spawn({
            let barrier = Arc::clone(&barrier);
            let result_tx = result_tx.clone();
            move || {
                barrier.wait();
                let result = gix_hash::Prefix::new(&b, b.kind().len_in_hex())
                    .map_err(|err| err.to_string())
                    .and_then(|prefix_value| prefix.lookup_prefix(prefix_value, None).map_err(|err| err.to_string()))
                    .and_then(|outcome| match outcome {
                        Some(Ok(id)) if id == b => Ok(()),
                        other => Err(format!("exact prefix resolves to the published object, got {other:?}")),
                    });
                result_tx.send(result).expect("the result receiver stays alive");
            }
        }),
        std::thread::spawn({
            let barrier = Arc::clone(&barrier);
            let result_tx = result_tx.clone();
            move || {
                barrier.wait();
                let result = iterate
                    .iter()
                    .map_err(|err| err.to_string())
                    .and_then(|objects| {
                        objects
                            .collect::<std::result::Result<Vec<_>, _>>()
                            .map_err(|err| err.to_string())
                    })
                    .and_then(|ids| {
                        ids.contains(&a)
                            .then_some(())
                            .ok_or_else(|| "iteration retains the object known before publication".to_owned())
                    });
                result_tx.send(result).expect("the result receiver stays alive");
            }
        }),
        std::thread::spawn({
            let barrier = Arc::clone(&barrier);
            let result_tx = result_tx.clone();
            let missing_id = fixture.manifest.missing_id();
            move || {
                barrier.wait();
                let result = scenarios::assert_missing_once(&missing, &missing_id).map_err(|err| err.to_string());
                result_tx.send(result).expect("the result receiver stays alive");
            }
        }),
        std::thread::spawn({
            let barrier = Arc::clone(&barrier);
            let result_tx = result_tx.clone();
            move || {
                barrier.wait();
                let entry = gix_odb::pack::Find::entry_by_location(&stable, &location);
                let mut buffer = Vec::new();
                let current = gix_odb::pack::Find::location_by_oid(&stable, &a, &mut buffer)
                    .expect("the stable location lookup succeeds");
                let result = (entry.is_some() && current.is_some())
                    .then_some(())
                    .ok_or_else(|| "the stable handle retains and resolves its pre-mutation location".to_owned());
                result_tx.send(result).expect("the result receiver stays alive");
            }
        }),
    ];
    drop(result_tx);
    let mut outcome = Ok(());
    for _ in 0..workers.len() {
        let result = result_rx
            .recv_timeout(Duration::from_secs(10))
            .expect("every mixed operation completes within the liveness bound");
        if outcome.is_ok() {
            outcome = result.map_err(std::io::Error::other);
        }
    }
    for worker in workers {
        worker.join().expect("a mixed-operation worker does not panic");
    }
    outcome?;
    assert_eq!(
        handle.store_ref().metrics().num_refreshes,
        refreshes_before + 1,
        "all stale operations share one refresh of the known disk mutation"
    );
    Ok(())
}
