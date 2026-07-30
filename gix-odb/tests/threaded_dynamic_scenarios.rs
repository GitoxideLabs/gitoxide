use gix_object::Find as _;

pub use gix_testtools::Result;
#[path = "tools/odb.rs"]
pub mod odb_fixture;
#[path = "tools/scenario.rs"]
mod support;
use odb_fixture::{Component, Database, OdbFixture, Pack};
use support::*;

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
