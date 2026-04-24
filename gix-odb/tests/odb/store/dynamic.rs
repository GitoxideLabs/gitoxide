use std::process::Command;

use gix_hash::ObjectId;
use gix_object::{Exists, FindExt, Write};
use gix_odb::{store, store::iter::Ordering, Header};
use gix_testtools::fixture_path_standalone;

use crate::{hex_to_id, odb::db};

fn all_orderings() -> [Ordering; 2] {
    [
        Ordering::PackLexicographicalThenLooseLexicographical,
        Ordering::PackAscendingOffsetThenLooseLexicographical,
    ]
}

/// indices, multi-pack-index, loose odb
fn db_with_all_object_sources() -> crate::Result<(gix_odb::Handle, gix_testtools::tempfile::TempDir)> {
    let objects_dir = gix_testtools::tempfile::tempdir()?;
    gix_testtools::copy_recursively_into_existing_dir(fixture_path_standalone("objects"), &objects_dir)?;

    let mut multi_pack_index = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(objects_dir.path().join("pack/multi-pack-index"))?;
    gix_odb::pack::multi_index::write_from_index_paths(
        vec![
            fixture_path_standalone("objects/pack/pack-a2bf8e71d8c18879e499335762dd95119d93d9f1.idx"),
            fixture_path_standalone("objects/pack/pack-c0438c19fb16422b6bbcce24387b3264416d485b.idx"),
        ],
        &mut multi_pack_index,
        &mut gix_features::progress::Discard,
        &std::sync::atomic::AtomicBool::default(),
        gix_odb::pack::multi_index::write::Options {
            object_hash: gix_hash::Kind::Sha1,
        },
    )?;
    Ok((gix_odb::at(objects_dir.path())?, objects_dir))
}

#[test]
fn multi_index_access() -> crate::Result {
    let dir = gix_testtools::scripted_fixture_writable_standalone("make_repo_multi_index.sh")?;
    let handle = gix_odb::at(dir.path().join(".git/objects"))?;

    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 0,
            open_reachable_indices: 0,
            known_reachable_indices: 0,
            open_reachable_packs: 0,
            known_packs: 0,
            unused_slots: 32,
            loose_dbs: 0,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "it starts out knowing nothing, it's completely lazy"
    );

    for order in all_orderings() {
        let mut count = 0;
        let mut buf = Vec::new();
        for oid in handle.iter()?.with_ordering(order) {
            let oid = oid?;
            assert!(handle.exists(&oid));
            let obj = handle.find(&oid, &mut buf)?;
            let hdr = handle.try_header(&oid)?.expect("exists");
            assert_eq!(hdr.kind(), obj.kind);
            assert_eq!(hdr.size(), obj.data.len() as u64);
            count += 1;
        }
        assert_eq!(count, 1732);
    }

    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 1,
            open_reachable_indices: 1,
            known_reachable_indices: 1,
            open_reachable_packs: 15,
            known_packs: 15,
            unused_slots: 31,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "it opened only a single multi-index and its pack - hard to see it's actually a multi-index as it's just one index anyway…"
    );

    let non_existing_to_trigger_refresh = hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    handle.exists(&non_existing_to_trigger_refresh);

    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 2,
            open_reachable_indices: 1,
            known_reachable_indices: 1,
            open_reachable_packs: 15,
            known_packs: 15,
            unused_slots: 31,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "A miss means just another refresh with no other change"
    );

    filetime::set_file_mtime(
        handle.store_ref().path().join("pack/multi-pack-index"),
        filetime::FileTime::now(),
    )?;
    handle.exists(&non_existing_to_trigger_refresh);

    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 2 + 1 /*legit refresh with changes*/ + 1 /*a refresh attempt with no changes, causing 'contains()' to give up*/,
            open_reachable_indices: 1,
            known_reachable_indices: 1,
            open_reachable_packs: 0,
            known_packs: 15,
            unused_slots: 31,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "everything seems to remain as it was, even though we moved our multi-index to a new slot and removed the old one"
    );

    assert_eq!(handle.store_ref().structure()?.len(), 2);
    assert_eq!(
        handle.store_ref().alternate_db_paths()?.len(),
        0,
        "there are no alternates"
    );
    Ok(())
}

#[test]
fn multi_index_alloc_limit_bytes_falls_back_to_plain_indices() -> crate::Result {
    let dir = gix_testtools::scripted_fixture_writable_standalone("make_repo_multi_index.sh")?;
    let handle = gix_odb::at_opts(
        dir.path().join(".git/objects"),
        Vec::new(),
        gix_odb::store::init::Options {
            alloc_limit_bytes: Some(1),
            ..Default::default()
        },
    )?;

    let mut count = 0;
    for oid in handle.iter()? {
        oid?;
        count += 1;
    }

    assert_eq!(
        count, 1732,
        "packed objects remain reachable by falling back to plain indices"
    );
    let metrics = handle.store_ref().metrics();
    assert!(
        metrics.known_reachable_indices > 1,
        "a multi-index open blocked by the allocation limit must fall back to loading plain indices"
    );
    assert_eq!(metrics.known_packs, 15);
    Ok(())
}

#[test]
fn multi_index_keep_open() -> crate::Result {
    let dir = gix_testtools::scripted_fixture_writable_standalone("make_repo_multi_index.sh")?;
    let (stable_handle, handle) = {
        let mut stable_handle = gix_odb::at(dir.path().join(".git/objects"))?;
        let handle = stable_handle.clone();
        stable_handle.prevent_pack_unload();
        (stable_handle, handle)
    };
    let oid = handle.iter()?.next().expect("first oid")?;

    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 2,
            num_refreshes: 1,
            open_reachable_indices: 1,
            known_reachable_indices: 1,
            open_reachable_packs: 0,
            known_packs: 15,
            unused_slots: 31,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "it opened the multi-pack index for iteration"
    );
    let mut buf = Vec::new();
    use gix_pack::Find;
    let location = stable_handle
        .location_by_oid(&oid, &mut buf)
        .expect("oid exists and is packed");

    let non_existing_to_trigger_refresh = hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    filetime::set_file_mtime(
        handle.store_ref().path().join("pack/multi-pack-index"),
        filetime::FileTime::now(),
    )?;
    gix_object::Exists::exists(&handle, &non_existing_to_trigger_refresh);

    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 2,
            num_refreshes: 3,
            open_reachable_indices: 1,
            known_reachable_indices: 1,
            open_reachable_packs: 0, /*no pack is open anymore at least as seen from the index*/
            known_packs: 15,
            unused_slots: 30,
            loose_dbs: 1,
            unreachable_indices: 1,
            unreachable_packs: 1
        },
        "now there is an unreachable index and pack which is still loaded, but whose pack hasn't been loaded"
    );

    assert!(
        gix_odb::pack::Find::entry_by_location(&stable_handle, &location).is_some(),
        "the entry can still be found even though the location is invalid"
    );
    assert_eq!(handle.store_ref().structure()?.len(), 2);
    Ok(())
}

#[test]
fn write() -> crate::Result {
    let dir = gix_testtools::tempfile::tempdir()?;
    let mut handle = gix_odb::at(dir.path())?;
    // It should refresh once even if the refresh mode is never, just to initialize the index
    handle.refresh_never();

    let written_id = handle.write_buf(gix_object::Kind::Blob, b"hello world")?;
    assert_eq!(written_id, hex_to_id("95d09f2b10159347eece71399a7e2e907ea3df4f"));
    Ok(())
}

#[test]
fn alternate_dbs_query() -> crate::Result {
    let dir = gix_testtools::scripted_fixture_read_only_standalone("make_alternates_odb.sh")?;
    let handle = gix_odb::at(dir.join(".git/objects"))?;

    let alternates = handle.store_ref().alternate_db_paths()?;
    assert_eq!(alternates.len(), 1, "exactly one alternate");
    assert_eq!(
        alternates[0]
            .ancestors()
            .nth(2)
            .expect("alternate repo work-dir")
            .file_name()
            .expect("present"),
        "object_source",
        "the alternate object location is well-known"
    );
    Ok(())
}

#[test]
fn object_replacement() -> crate::Result {
    let dir = gix_testtools::scripted_fixture_read_only_standalone("make_replaced_history.sh")?;
    let handle = gix_odb::at(dir.join(".git/objects"))?;
    let mut buf = Vec::new();
    let short_history_link = hex_to_id("434e5a872d6738d1fffd1e11e52a1840b73668c6");
    let third_commit = handle.find_commit(&short_history_link, &mut buf)?;

    let orphan_of_new_history = hex_to_id("0703c317e28068f39834ae61e7ab941b7d672322");
    assert_eq!(
        third_commit.parents().collect::<Vec<_>>(),
        vec![orphan_of_new_history],
        "no replacements are known by default, hence this is the replaced commit, not the replacement"
    );
    drop(third_commit);
    let hdr = handle.try_header(&short_history_link)?.expect("present");
    assert_eq!(hdr.kind(), gix_object::Kind::Commit);
    assert_eq!(
        handle.find(&short_history_link, &mut buf)?.data.len() as u64,
        hdr.size()
    );

    let orphan = handle.find_commit(&orphan_of_new_history, &mut buf)?;
    assert_eq!(orphan.parents.len(), 0);
    let hdr = handle.try_header(&orphan_of_new_history)?.expect("present");
    assert_eq!(hdr.kind(), gix_object::Kind::Commit);

    let long_history_tip = hex_to_id("71f537d9d78bf6ae89a29a17e54b95a914d3d2ef");
    let unrelated_mapping = (
        ObjectId::null(handle.store_ref().object_hash()),
        ObjectId::null(handle.store_ref().object_hash()),
    );

    let mut handle = gix_odb::at_opts(
        dir.join(".git/objects"),
        vec![(short_history_link, long_history_tip), unrelated_mapping],
        gix_odb::store::init::Options { ..Default::default() },
    )?;
    drop(orphan);

    let replaced = handle.find_commit(&short_history_link, &mut buf)?;
    let long_history_second_id = hex_to_id("753ccf815e7b69c9147db5bbf633fe5f7da24ad7");
    assert_eq!(
        replaced.parents().collect::<Vec<_>>(),
        vec![long_history_second_id],
        "replacements are applied by default when present"
    );
    let hdr = handle.try_header(&short_history_link)?.expect("present");
    assert_eq!(hdr.kind(), gix_object::Kind::Commit);
    drop(replaced);
    assert_eq!(
        handle.find(&short_history_link, &mut buf)?.data.len() as u64,
        hdr.size()
    );

    handle.ignore_replacements = true;
    let not_replaced = handle.find_commit(&short_history_link, &mut buf)?;
    assert_eq!(
        not_replaced.parents().collect::<Vec<_>>(),
        vec![orphan_of_new_history],
        "no replacements are made if explicitly disabled"
    );
    let hdr = handle.try_header(&short_history_link)?.expect("present");
    assert_eq!(hdr.kind(), gix_object::Kind::Commit);
    drop(not_replaced);
    assert_eq!(
        handle.find(&short_history_link, &mut buf)?.data.len() as u64,
        hdr.size()
    );

    assert_eq!(
        handle.store_ref().replacements().collect::<Vec<_>>(),
        vec![unrelated_mapping, (short_history_link, long_history_tip)],
        "one can query the list of replacements"
    );

    // TODO: mapping to non-existing object (can happen if replace-refs are pushed but related history isn't fetched)
    Ok(())
}

#[test]
fn contains() {
    let handle = db();

    assert!(handle.exists(&hex_to_id("37d4e6c5c48ba0d245164c4e10d5f41140cab980"))); // loose object
    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 1,
            open_reachable_indices: 0,
            known_reachable_indices: 3,
            open_reachable_packs: 0,
            known_packs: 3,
            unused_slots: 29,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "it only refreshed the file list, yielding the loose db to find this object, but no pack was opened yet"
    );

    // pack c043, the biggest one
    assert!(handle.exists(&hex_to_id("dd25c539efbb0ab018caa4cda2d133285634e9b5")));

    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 1,
            open_reachable_indices: 1,
            known_reachable_indices: 3,
            open_reachable_packs: 0,
            known_packs: 3,
            unused_slots: 29,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "it loaded the biggest back only, which is the first in the list"
    );

    // pack, the smallest one
    // The new handle should make no difference.
    #[allow(clippy::redundant_clone)]
    let mut new_handle = handle.clone();
    assert!(new_handle.exists(&hex_to_id("501b297447a8255d3533c6858bb692575cdefaa0")));
    assert_eq!(
        new_handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 2,
            num_refreshes: 1,
            open_reachable_indices: 3,
            known_reachable_indices: 3,
            open_reachable_packs: 0,
            known_packs: 3,
            unused_slots: 29,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "when asking for an object in the smallest pack, all in between packs are also loaded."
    );

    assert!(!new_handle.exists(&hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")));
    assert_eq!(
        new_handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 2,
            num_refreshes: 2,
            open_reachable_indices: 3,
            known_reachable_indices: 3,
            open_reachable_packs: 0,
            known_packs: 3,
            unused_slots: 29,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "trigger refreshes each time there is an object miss"
    );

    new_handle.refresh_never();
    assert!(!new_handle.exists(&hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")));
    assert_eq!(
        new_handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 2,
            num_refreshes: 2,
            open_reachable_indices: 3,
            known_reachable_indices: 3,
            open_reachable_packs: 0,
            known_packs: 3,
            unused_slots: 29,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "if no refreshes are allowed, there is no additional refresh"
    );

    assert_eq!(handle.store_ref().structure().unwrap().len(), 4);
}

#[test]
fn lookup() {
    let mut handle = db();

    fn can_locate(db: &gix_odb::Handle, hex_id: &str) {
        let id = hex_to_id(hex_id);
        assert!(db.exists(&id));

        let mut buf = Vec::new();
        let obj = db.find(&id, &mut buf).expect("exists");

        let hdr = db.try_header(&id).unwrap().expect("exists");
        assert_eq!(obj.kind, hdr.kind());
        assert_eq!(obj.data.len() as u64, hdr.size());
    }
    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 0,
            open_reachable_indices: 0,
            known_reachable_indices: 0,
            open_reachable_packs: 0,
            known_packs: 0,
            unused_slots: 32,
            loose_dbs: 0,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "nothing happened yet, the store is totally lazy"
    );

    // pack (in sort order inherent to the store)
    can_locate(&handle, "501b297447a8255d3533c6858bb692575cdefaa0"); // pack 11fd
    can_locate(&handle, "4dac9989f96bc5b5b1263b582c08f0c5f0b58542"); // pack a2bf
    can_locate(&handle, "dd25c539efbb0ab018caa4cda2d133285634e9b5"); // pack c043

    let mut all_loaded = gix_odb::store::Metrics {
        num_handles: 1,
        num_refreshes: 1,
        open_reachable_indices: 3,
        known_reachable_indices: 3,
        open_reachable_packs: 3,
        known_packs: 3,
        unused_slots: 29,
        loose_dbs: 1,
        unreachable_indices: 0,
        unreachable_packs: 0,
    };
    assert_eq!(
        handle.store_ref().metrics(),
        all_loaded,
        "all packs and indices are loaded"
    );

    // Loose
    can_locate(&handle, "37d4e6c5c48ba0d245164c4e10d5f41140cab980");

    assert!(matches!(
        handle.refresh_mode(),
        store::RefreshMode::AfterAllIndicesLoaded
    ));
    assert!(!handle.exists(&hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")));

    all_loaded.num_refreshes += 1;
    assert_eq!(
        handle.store_ref().metrics(),
        all_loaded,
        "it tried to refresh once to see if the missing object is there then"
    );

    handle.refresh_never();
    let previous_refresh_count = all_loaded.num_refreshes;
    assert!(!handle.exists(&hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")));
    assert_eq!(
        handle.store_ref().metrics().num_refreshes,
        previous_refresh_count,
        "it didn't try to refresh the on-disk state after failing to find the object."
    );
}

fn assert_all_indices_loaded(handle: &gix_odb::Handle, num_refreshes: usize, open_reachable_indices: usize) {
    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes,
            open_reachable_indices,
            known_reachable_indices: 2,
            open_reachable_packs: 0,
            known_packs: 3,
            unused_slots: 30,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "all indices must be loaded and searched to assure unambiguous object ids"
    );
}

#[test]
fn packed_object_count_causes_all_indices_to_be_loaded() {
    let (handle, _tmp) = db_with_all_object_sources().unwrap();

    assert_eq!(handle.packed_object_count().unwrap(), 139);
    assert_all_indices_loaded(&handle, 1, 2);
}

mod disambiguate_prefix {
    use std::cmp::Ordering;

    use gix_odb::store::prefix::disambiguate::Candidate;

    use crate::{
        odb::{hex_to_id, store::dynamic::all_orderings},
        store::dynamic::{assert_all_indices_loaded, db_with_all_object_sources},
    };

    #[test]
    fn unambiguous_hex_lengths_yield_prefixes_of_exactly_the_given_length() -> crate::Result {
        let (mut handle, _tmp) = db_with_all_object_sources()?;
        handle.refresh.never();

        let hex_lengths = &[5, 7, 40];
        for order in all_orderings() {
            for (index, oid) in handle.iter()?.with_ordering(order).map(Result::unwrap).enumerate() {
                let hex_len = hex_lengths[index % hex_lengths.len()];
                let prefix = handle
                    .disambiguate_prefix(Candidate::new(oid, hex_len)?)?
                    .expect("object exists");
                assert_eq!(prefix.hex_len(), hex_len);
                assert_eq!(prefix.cmp_oid(&oid), Ordering::Equal);
            }
        }
        assert_all_indices_loaded(&handle, 1, 2);
        Ok(())
    }

    #[test]
    fn returns_disambiguated_prefixes_when_needed() {
        let (handle, _tmp) = db_with_all_object_sources().unwrap();
        let id = hex_to_id("a7065b5e971a6d8b55875d8cf634a3a37202ab23");
        let prefix = handle
            .disambiguate_prefix(Candidate::new(id, 4).unwrap())
            .unwrap()
            .expect("object exists");

        assert_eq!(prefix.hex_len(), 5, "the hex_len was increased to disambiguate");
        assert_eq!(prefix.cmp_oid(&id), Ordering::Equal);
        assert_all_indices_loaded(&handle, 2, 2);
    }

    #[test]
    fn no_work_is_done_for_unambiguous_potential_prefixes() {
        let (handle, _tmp) = db_with_all_object_sources().unwrap();
        let id = hex_to_id("a7065b5e971a6d8b55875d8cf634a3a37202ab23");
        let potential_prefix = Candidate::new(id, 40).unwrap();
        assert!(
            handle
                .disambiguate_prefix(potential_prefix)
                .unwrap()
                .expect("object exists")
                == potential_prefix.to_prefix(),
        );

        assert_eq!(
            handle.store_ref().metrics(),
            gix_odb::store::Metrics {
                num_handles: 1,
                num_refreshes: 1,
                open_reachable_indices: 1,
                known_reachable_indices: 2,
                known_packs: 3,
                unused_slots: 30,
                loose_dbs: 1,
                ..Default::default()
            },
            "early bailout, without doing any real work except for a contains() check"
        );
    }

    #[test]
    fn returns_none_if_id_does_not_exist() {
        let (handle, _tmp) = db_with_all_object_sources().unwrap();
        let null = gix_hash::ObjectId::null(gix_hash::Kind::Sha1);
        assert!(handle
            .disambiguate_prefix(Candidate::new(null, 7).unwrap())
            .unwrap()
            .is_none());
        assert_all_indices_loaded(&handle, 2, 2);
    }
}

mod iter {
    use gix_odb::store::iter::Ordering;

    use crate::odb::{
        db,
        store::dynamic::{all_orderings, db_with_all_object_sources},
    };

    #[test]
    fn iteration_ordering_is_effective() -> crate::Result {
        assert_eq!(all_orderings().len(), 2, "new orderings cause this test to be reviewed");
        for (handle, _tmp) in [db_with_all_object_sources().map(|(a, b)| (a, Some(b)))?, (db(), None)] {
            let (mut a, mut b): (Vec<_>, Vec<_>) = (
                handle
                    .iter()?
                    .with_ordering(Ordering::PackLexicographicalThenLooseLexicographical)
                    .map(Result::unwrap)
                    .collect(),
                handle
                    .iter()?
                    .with_ordering(Ordering::PackAscendingOffsetThenLooseLexicographical)
                    .map(Result::unwrap)
                    .collect(),
            );
            assert_eq!(a.len(), b.len(), "count isn't affected by ordering");
            assert_ne!(a, b, "ordering is different");

            a.sort();
            b.sort();
            assert_eq!(a, b, "both sets contain the same object ids");
        }
        Ok(())
    }
}

mod lookup_prefix {
    use std::collections::HashSet;

    use maplit::hashset;

    use crate::{
        odb::{hex_to_id, store::dynamic::all_orderings},
        store::dynamic::{assert_all_indices_loaded, db_with_all_object_sources},
    };

    #[test]
    fn returns_none_for_prefixes_without_any_match() {
        let (handle, _tmp) = db_with_all_object_sources().unwrap();
        let prefix = gix_hash::Prefix::new(&gix_hash::ObjectId::null(gix_hash::Kind::Sha1), 7).unwrap();
        assert!(handle.lookup_prefix(prefix, None).unwrap().is_none());
        assert_all_indices_loaded(&handle, 2, 2);

        let mut candidates = HashSet::default();
        assert!(handle.lookup_prefix(prefix, Some(&mut candidates)).unwrap().is_none());
        assert!(candidates.is_empty(), "no candidates available here either");
        assert_all_indices_loaded(&handle, 3, 2);
    }

    #[test]
    fn returns_some_err_for_prefixes_with_more_than_one_match() {
        let (handle, _tmp) = db_with_all_object_sources().unwrap();
        let input_id = hex_to_id("a7065b5e971a6d8b55875d8cf634a3a37202ab23");
        let prefix = gix_hash::Prefix::new(&input_id, 4).unwrap();
        assert_eq!(
            handle.lookup_prefix(prefix, None).unwrap(),
            Some(Err(())),
            "there are two objects with that prefix"
        );
        assert_all_indices_loaded(&handle, 1, 1);

        let mut candidates = HashSet::default();
        assert_eq!(
            handle.lookup_prefix(prefix, Some(&mut candidates)).unwrap(),
            Some(Err(())),
            "the error is the same"
        );
        assert_eq!(
            candidates,
            hashset! {input_id, hex_to_id("a706d7cd20fc8ce71489f34b50cf01011c104193")},
            "all candidates are returned though"
        );
        assert_all_indices_loaded(&handle, 2, 2);
    }

    #[test]
    fn iterable_objects_can_be_looked_up_with_varying_prefix_lengths() -> crate::Result {
        let (mut handle, _tmp) = db_with_all_object_sources()?;
        handle.refresh.never();

        let hex_lengths = &[5, 7, 40];
        for order in all_orderings() {
            for (index, oid) in handle.iter()?.with_ordering(order).map(Result::unwrap).enumerate() {
                for mut candidates in [None, Some(HashSet::default())] {
                    let hex_len = hex_lengths[index % hex_lengths.len()];
                    let prefix = gix_hash::Prefix::new(&oid, hex_len)?;
                    assert_eq!(
                        handle
                            .lookup_prefix(prefix, candidates.as_mut())?
                            .expect("object exists")
                            .expect("unambiguous"),
                        oid
                    );
                    if let Some(candidates) = candidates {
                        assert_eq!(candidates, hashset! {oid});
                    }
                }
            }
        }
        assert_all_indices_loaded(&handle, 1, 2);
        Ok(())
    }
}

#[test]
fn missing_objects_triggers_everything_is_loaded() {
    let handle = db();
    assert!(!handle.exists(&hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")));

    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 2,
            open_reachable_indices: 3,
            known_reachable_indices: 3,
            open_reachable_packs: 0,
            known_packs: 3,
            unused_slots: 29,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "first refresh triggered by on-disk check, second refresh triggered to see if something changed, contains() only sees indices"
    );

    let mut buf = Vec::new();
    assert!(handle
        .find(&hex_to_id("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"), &mut buf)
        .is_err());

    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 3,
            open_reachable_indices: 3,
            known_reachable_indices: 3,
            open_reachable_packs: 0,
            known_packs: 3,
            unused_slots: 29,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "there are still no packs opened as no index contained the object"
    );
}

#[test]
fn iterate_over_a_bunch_of_loose_and_packed_objects() -> crate::Result {
    let (db, _tmp) = db_with_all_object_sources()?;
    for order in all_orderings() {
        let iter = db.iter()?.with_ordering(order);
        assert_eq!(
            iter.size_hint(),
            (139, None),
            "we only count packs and have no upper bound"
        );
        assert_eq!(iter.count(), 146, "it sees the correct amount of objects");
        for id in db.iter()? {
            assert!(db.exists(&id?), "each object exists");
        }
    }
    Ok(())
}

#[test]
fn auto_refresh_with_and_without_id_stability() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    assert!(
        Command::new("git")
            .arg("-C")
            .arg(tmp.path())
            .arg("init")
            .arg("--bare")
            .status()?
            .success(),
        "git should work"
    );
    gix_testtools::copy_recursively_into_existing_dir(
        fixture_path_standalone("objects/pack"),
        tmp.path().join("objects/pack"),
    )?;
    let hide_pack = |name: &str| {
        let stem = tmp.path().join("objects/pack").join(name);
        std::fs::rename(stem.with_extension("idx"), stem.with_extension("idx.bak")).unwrap();
        std::fs::rename(stem.with_extension("pack"), stem.with_extension("pack.bak")).unwrap();
    };
    let unhide_pack = |name: &str| {
        let stem = tmp.path().join("objects/pack").join(name);
        std::fs::rename(stem.with_extension("idx.bak"), stem.with_extension("idx")).unwrap();
        std::fs::rename(stem.with_extension("pack.bak"), stem.with_extension("pack")).unwrap();
    };
    hide_pack("pack-11fdfa9e156ab73caae3b6da867192221f2089c2");
    hide_pack("pack-a2bf8e71d8c18879e499335762dd95119d93d9f1");

    let handle = gix_odb::at(tmp.path().join("objects"))?;
    let mut buf = Vec::new();
    assert!(
        handle
            .find(&hex_to_id("dd25c539efbb0ab018caa4cda2d133285634e9b5"), &mut buf)
            .is_ok(),
        "can find object in existing pack at pack-c0438c19fb16422b6bbcce24387b3264416d485b.idx"
    );
    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 1,
            open_reachable_indices: 1,
            known_reachable_indices: 1,
            open_reachable_packs: 1,
            known_packs: 1,
            unused_slots: 31,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "one pack was opened"
    );

    hide_pack("pack-c0438c19fb16422b6bbcce24387b3264416d485b");
    unhide_pack("pack-11fdfa9e156ab73caae3b6da867192221f2089c2");
    assert!(
        handle
            .find(&hex_to_id("501b297447a8255d3533c6858bb692575cdefaa0"), &mut buf)
            .is_ok(),
        "now finding the object in the new pack"
    );
    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 2,
            open_reachable_indices: 1,
            known_reachable_indices: 1,
            open_reachable_packs: 1,
            known_packs: 1,
            unused_slots: 31,
            loose_dbs: 1,
            unreachable_indices: 0,
            unreachable_packs: 0
        },
        "the old pack was removed, the new was loaded"
    );

    {
        use gix_pack::Find;
        let mut stable_handle = handle.clone();
        stable_handle.prevent_pack_unload();
        let location = stable_handle
            .location_by_oid(&hex_to_id("501b297447a8255d3533c6858bb692575cdefaa0"), &mut buf)
            .expect("object exists");
        assert!(
            stable_handle.entry_by_location(&location).is_some(),
            "entries can be found by location as the pack is definitely still loaded, the index didn't change"
        );

        hide_pack("pack-11fdfa9e156ab73caae3b6da867192221f2089c2");
        unhide_pack("pack-a2bf8e71d8c18879e499335762dd95119d93d9f1");

        assert!(
            stable_handle
                .location_by_oid(&hex_to_id("4dac9989f96bc5b5b1263b582c08f0c5f0b58542"), &mut buf)
                .is_some(),
            "it finds the object in the newly unhidden pack, which also triggers a refresh providing it with new indices"
        );
        assert_eq!(
            handle.store_ref().metrics(),
            gix_odb::store::Metrics {
                num_handles: 2,
                num_refreshes: 3,
                open_reachable_indices: 1,
                known_reachable_indices: 1,
                open_reachable_packs: 1,
                known_packs: 1,
                unused_slots: 30,
                loose_dbs: 1,
                unreachable_indices: 1,
                unreachable_packs: 1
            },
            "the removed pack is still loaded"
        );
        assert!(
            stable_handle.entry_by_location(&location).is_some(),
            "it finds the old removed location (still loaded) on the old id, it's still cached in the handle, too"
        );
        assert!(
            stable_handle.clone().entry_by_location(&location).is_some(),
            "handles without any internal cache also work"
        );
    }

    hide_pack("pack-a2bf8e71d8c18879e499335762dd95119d93d9f1");
    unhide_pack("pack-c0438c19fb16422b6bbcce24387b3264416d485b");

    assert!(
        handle
            .find(&hex_to_id("dd25c539efbb0ab018caa4cda2d133285634e9b5"), &mut buf)
            .is_ok(),
        "new pack is loaded, previously loaded is forgotten, lack of cache triggers refresh"
    );
    assert_eq!(
        handle.store_ref().metrics(),
        gix_odb::store::Metrics {
            num_handles: 1,
            num_refreshes: 4,
            open_reachable_indices: 1,
            known_reachable_indices: 1,
            open_reachable_packs: 1,
            known_packs: 1,
            unused_slots: 30,
            loose_dbs: 1,
            unreachable_indices: 1,
            unreachable_packs: 1
        },
        "garbaged slots aren't reclaimed until there is the need. Keeping indices open despite them not being accessible anymore."
    );
    Ok(())
}

mod verify {
    use std::sync::atomic::AtomicBool;

    use gix_features::progress;
    use gix_testtools::fixture_path_standalone;

    use crate::store::dynamic::db;

    #[test]
    fn integrity() {
        let handle = db();
        let outcome = handle
            .store_ref()
            .verify_integrity(&mut progress::Discard, &AtomicBool::new(false), Default::default())
            .unwrap();
        assert_eq!(outcome.index_statistics.len(), 3, "there are only three packs to check");
        assert_eq!(
            outcome.index_statistics[0].path,
            fixture_path_standalone("objects/pack/pack-c0438c19fb16422b6bbcce24387b3264416d485b.idx")
        );
        assert_eq!(
            outcome.index_statistics[1].path,
            fixture_path_standalone("objects/pack/pack-a2bf8e71d8c18879e499335762dd95119d93d9f1.idx")
        );
        assert_eq!(
            outcome.index_statistics[2].path,
            fixture_path_standalone("objects/pack/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.idx")
        );
        assert_eq!(
            outcome.loose_object_stores,
            vec![gix_odb::store::verify::integrity::LooseObjectStatistics {
                path: fixture_path_standalone("objects"),
                statistics: gix_odb::loose::verify::integrity::Statistics { num_objects: 7 }
            }]
        );

        assert_eq!(
            handle.store_ref().metrics(),
            gix_odb::store::Metrics {
                num_handles: 1,
                num_refreshes: 1,
                open_reachable_indices: 0,
                known_reachable_indices: 3,
                open_reachable_packs: 0,
                known_packs: 3,
                unused_slots: 29,
                loose_dbs: 1,
                unreachable_indices: 0,
                unreachable_packs: 0
            },
            "verification only discovers files on disk but won't cause them to be opened permanently"
        );
    }
}
