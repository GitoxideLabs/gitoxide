use std::{
    sync::atomic::AtomicBool,
    time::{Duration, SystemTime},
};

use gix_object::Write;
use gix_odb::loose::{Store, prune::Options};
use gix_testtools::TestResult;

const CUTOFF: u64 = 1_700_000_000;

fn options(dry_run: bool) -> Options {
    Options {
        expire: SystemTime::UNIX_EPOCH + Duration::from_secs(CUTOFF),
        dry_run,
    }
}

fn set_time(store: &Store, object_id: &gix_hash::oid, seconds: u64) -> TestResult {
    filetime::set_file_mtime(
        store.object_path(object_id),
        filetime::FileTime::from_unix_time(seconds as i64, 0),
    )?;
    Ok(())
}

#[test]
fn expiry_and_reachability_match_git_for_both_object_formats() -> TestResult {
    let fixture = gix_testtools::scripted_fixture_writable("make_prune_repos.sh")?;
    for object_hash in [gix_hash::Kind::Sha1, gix_hash::Kind::Sha256] {
        let path = fixture.path().join(object_hash.to_string());
        let store = Store::at(path.join("objects"), object_hash);
        let keep_id = store.write_buf(gix_object::Kind::Blob, b"reachable")?;
        let old_id = store.write_buf(gix_object::Kind::Blob, b"old")?;
        let boundary_id = store.write_buf(gix_object::Kind::Blob, b"at cutoff")?;
        let recent_id = store.write_buf(gix_object::Kind::Blob, b"recent")?;
        for object_id in [keep_id, old_id] {
            set_time(&store, &object_id, CUTOFF - 1)?;
        }
        set_time(&store, &boundary_id, CUTOFF)?;
        set_time(&store, &recent_id, CUTOFF + 1)?;
        gix_testtools::git(&path, &format!("update-ref refs/tags/keep {keep_id}"))?;
        let expected = gix_testtools::git(&path, &format!("prune --dry-run --expire=@{CUTOFF}"))?;
        let mut expected_ids: Vec<_> = expected
            .lines()
            .map(|line| line.split_once(' ').expect("Git lists ID and kind").0.to_owned())
            .collect();
        expected_ids.sort();
        let mut pruned_ids = vec![old_id.to_string(), boundary_id.to_string()];
        pruned_ids.sort();
        assert_eq!(
            expected_ids, pruned_ids,
            "Git prunes unprotected objects at or before the cutoff"
        );
        let expected_bytes =
            store.object_path(&old_id).metadata()?.len() + store.object_path(&boundary_id).metadata()?.len();
        let dry_run = store.prune(|object_id| object_id == keep_id, options(true), &AtomicBool::default())?;
        assert_eq!(dry_run.pruned_objects, 2, "the same two objects are selected");
        assert_eq!(dry_run.pruned_bytes, expected_bytes, "sizes count compressed files");
        assert!(
            store.contains(&old_id) && store.contains(&boundary_id),
            "dry runs retain every object"
        );
        let actual = store.prune(|object_id| object_id == keep_id, options(false), &AtomicBool::default())?;
        assert_eq!(actual, dry_run, "the actual prune removes the dry-run candidates");
        assert!(
            store.contains(&keep_id) && store.contains(&recent_id),
            "reachable and recent objects remain"
        );
        assert!(
            !store.contains(&old_id) && !store.contains(&boundary_id),
            "only the expired unreachable objects are deleted"
        );
        assert!(
            gix_testtools::git(&path, &format!("prune --dry-run --expire=@{CUTOFF}"))?.is_empty(),
            "Git finds no remaining prune candidates"
        );
    }
    Ok(())
}

#[test]
fn freshening_and_concurrent_deletion_are_observed_after_reachability_lookup() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let store = Store::at(directory.path(), gix_testtools::object_hash());
    let freshened_id = store.write_buf(gix_object::Kind::Blob, b"freshened")?;
    let removed_id = store.write_buf(gix_object::Kind::Blob, b"removed concurrently")?;
    set_time(&store, &freshened_id, CUTOFF - 1)?;
    set_time(&store, &removed_id, CUTOFF - 1)?;
    let result = store.prune(
        |object_id| {
            if object_id == freshened_id {
                set_time(&store, object_id, CUTOFF + 1).expect("freshen the object before its final metadata check");
            } else {
                std::fs::remove_file(store.object_path(object_id)).expect("simulate concurrent removal");
            }
            false
        },
        options(false),
        &AtomicBool::default(),
    )?;
    assert_eq!(
        result.pruned_objects, 0,
        "freshened and already-removed objects are not counted as pruned"
    );
    assert!(
        store.contains(&freshened_id),
        "concurrent freshening protects the object"
    );
    Ok(())
}

#[test]
fn cancellation_and_unrecognized_paths_do_not_remove_objects() -> TestResult {
    let directory = gix_testtools::tempfile::tempdir()?;
    let store = Store::at(directory.path(), gix_testtools::object_hash());
    let object_id = store.write_buf(gix_object::Kind::Blob, b"object")?;
    set_time(&store, &object_id, CUTOFF - 1)?;
    let err = store
        .prune(|_| false, options(false), &AtomicBool::new(true))
        .expect_err("cancel before touching objects");
    assert!(
        err.is_cancelled(),
        "interruptions carry the cancellation classification"
    );
    assert!(store.contains(&object_id), "cancellation precedes deletion");
    let unknown = directory.path().join("aa/not-an-object");
    std::fs::create_dir_all(unknown.parent().expect("fanout parent"))?;
    std::fs::write(&unknown, b"keep")?;
    let nested = directory
        .path()
        .join("unexpected")
        .join(&object_id.to_string()[..2])
        .join(&object_id.to_string()[2..]);
    std::fs::create_dir_all(nested.parent().expect("nested parent"))?;
    std::fs::write(&nested, b"keep")?;
    store.prune(|_| true, options(false), &AtomicBool::default())?;
    assert!(
        unknown.is_file() && nested.is_file(),
        "unrecognized paths are not pruned"
    );
    Ok(())
}

#[test]
#[cfg(unix)]
fn symbolic_links_and_directories_are_preserved() -> TestResult {
    use std::os::unix::fs::symlink;
    let directory = gix_testtools::tempfile::tempdir()?;
    let store = Store::at(directory.path().join("objects"), gix_testtools::object_hash());
    std::fs::create_dir(store.path())?;
    let object_id = store.write_buf(gix_object::Kind::Blob, b"object")?;
    let object_path = store.object_path(&object_id);
    let target = directory.path().join("external");
    std::fs::rename(&object_path, &target)?;
    symlink(&target, &object_path)?;
    assert_eq!(
        store
            .prune(|_| false, options(false), &AtomicBool::default())?
            .pruned_objects,
        0,
        "symlinks are not object files for pruning"
    );
    assert!(
        object_path.is_symlink() && target.is_file(),
        "both the link and target remain"
    );
    std::fs::remove_file(&object_path)?;
    std::fs::create_dir(&object_path)?;
    assert_eq!(
        store
            .prune(|_| false, options(false), &AtomicBool::default())?
            .pruned_objects,
        0,
        "object-shaped directories remain untouched"
    );
    Ok(())
}
