use gix_object::Find as _;
use gix_odb::Header as _;

use crate::{
    Result,
    odb_fixture::{Component, Database, OdbFixture, Pack},
};
#[path = "../../tools/scenario.rs"]
mod support;
use support::*;

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

fn assert_missing(handle: &gix_odb::Handle, id: &gix_hash::oid) -> Result {
    let mut buffer = Vec::new();
    assert!(
        handle.try_find(id, &mut buffer)?.is_none(),
        "the object is not reachable in the current fixture state"
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
fn a_missing_index_recovers_without_an_mtime_change() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let mut handle = open(&fixture, 4)?;
    handle.refresh_never();
    handle.store_ref().structure()?;

    let index_path = fixture.component_path(Database::Primary, Pack::A, Component::Index);
    let hidden_index_path = index_path.with_extension("hidden");
    let original_mtime = std::fs::metadata(&index_path)?.modified()?;
    let object_id = fixture.manifest.pack(Pack::A).object_ids[0];
    // Hide the index after discovery but before its first load, retaining its original timestamp.
    std::fs::rename(&index_path, &hidden_index_path)?;
    assert_missing(&handle, &object_id)?;
    std::fs::rename(&hidden_index_path, &index_path)?;
    assert_eq!(
        std::fs::metadata(&index_path)?.modified()?,
        original_mtime,
        "rediscovery must recover the same file without a timestamp change"
    );

    handle.refresh = gix_odb::store::RefreshMode::AfterAllIndicesLoaded;
    assert_object(&handle, &object_id)?;
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
fn copied_stores_preserve_slot_growth_policy() -> Result {
    use gix_odb::store::init::Slots;

    for slots in [
        Slots::Growable { initial: 0 },
        Slots::Growable { initial: 1 },
        Slots::AsNeededByDiskState {
            multiplier: 1.0,
            minimum: 0,
        },
        Slots::Limit(0),
        Slots::Limit(1),
    ] {
        let mut fixture = OdbFixture::from_script()?;
        let source = open_with_slots(&fixture, slots)?;
        let copy = std::sync::Arc::new(gix_odb::Store::try_from(source.store_ref())?).to_cache_arc();
        let converted = source.into_arc()?;
        for (pack_index, pack) in [Pack::A, Pack::B].into_iter().enumerate() {
            fixture.install_pack(Database::Primary, pack)?;
            let object_id = fixture.manifest.pack(pack).object_ids[0];
            for handle in [&copy, &converted] {
                let mut buffer = Vec::new();
                let result = handle.try_find(&object_id, &mut buffer);
                if matches!(slots, Slots::Limit(limit) if pack_index >= usize::from(limit)) {
                    let err = result.expect_err("copying preserves an explicit slot limit");
                    assert!(
                        err.to_string().contains("slot"),
                        "the limit causes slot exhaustion: {err}"
                    );
                } else {
                    assert!(result?.is_some(), "a copied {slots:?} store can discover the new pack");
                }
            }
        }
    }
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
    let location = gix_odb::pack::Find::location_by_oid(&stable, &a, &mut buffer)?
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
    let location = gix_odb::pack::Find::location_by_oid(&stable, &id, &mut buffer)?
        .expect("the stable handle locates the object through the current MIDX");
    assert!(
        gix_odb::pack::Find::location_by_oid(&stable, &fixture.manifest.pack(Pack::C).object_ids[0], &mut buffer)?
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
    let location = gix_odb::pack::Find::location_by_oid(&stable, &b, &mut buffer)?
        .expect("the standalone pack is available before the MIDX absorbs it");

    fixture.write_multi_index(Database::Primary, &[Pack::A, Pack::B])?;
    assert!(
        gix_odb::pack::Find::location_by_oid(&stable, &fixture.manifest.pack(Pack::C).object_ids[0], &mut buffer)?
            .is_none(),
        "the missing object remains absent after the rewrite"
    );
    assert!(
        gix_odb::pack::Find::entry_by_location(&stable, &location).is_some(),
        "the standalone pack ID remains mapped after the failed rewrite"
    );
    Ok(())
}

#[test]
fn stable_clones_share_locations_discovered_after_cloning() -> Result {
    use gix_pack::Find;

    for use_multi_index in [false, true] {
        for use_find in [false, true] {
            let mut fixture = OdbFixture::from_script()?;
            fixture.install_pack(Database::Primary, Pack::A)?;
            if use_multi_index {
                fixture.write_multi_index(Database::Primary, &[Pack::A])?;
            }
            let mut parent = open(&fixture, 4)?.into_arc()?;
            parent.prevent_pack_unload();
            let sibling = parent.clone();
            let producer = parent.clone();
            let object_id = fixture.manifest.pack(Pack::A).object_ids[0];
            let location = std::thread::spawn(move || {
                let mut buffer = Vec::new();
                if use_find {
                    Find::try_find(&producer, &object_id, &mut buffer)
                        .expect("the clone reads a packed object")
                        .expect("the fixture object is present")
                        .1
                } else {
                    producer
                        .location_by_oid(&object_id, &mut buffer)
                        .expect("the clone locates a packed object")
                }
                .expect("the clone obtains a pack location")
            })
            .join()
            .expect("the producing clone does not panic");

            for remove_pack in [false, true] {
                if remove_pack {
                    fixture.remove(Database::Primary, Pack::A, Component::Pack)?;
                    fixture.remove(Database::Primary, Pack::A, Component::Index)?;
                    if use_multi_index {
                        fixture.remove_multi_index(Database::Primary)?;
                    }
                }
                for consumer in [&parent, &sibling] {
                    if remove_pack {
                        assert!(
                            consumer
                                .location_by_oid(&fixture.manifest.missing_id(), &mut Vec::new())?
                                .is_none(),
                            "a miss refreshes the consumer after maintenance"
                        );
                    }
                    assert!(
                        consumer.entry_by_location(&location).is_some(),
                        "an existing clone retains locations discovered by another, even after maintenance"
                    );
                    assert!(
                        consumer
                            .pack_offsets_and_oid(location.pack_id)?
                            .expect("the retained index is shared too")
                            .contains(&(location.pack_offset, object_id)),
                        "the shared index maps the retained entry back to its object"
                    );
                }
            }
        }
    }
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
    let location = gix_odb::pack::Find::location_by_oid(&stable, &id, &mut buffer)?
        .expect("the stable handle locates the object through the MIDX");

    fixture.remove(Database::Primary, Pack::A, Component::Pack)?;
    assert!(
        gix_odb::pack::Find::location_by_oid(&stable, &fixture.manifest.pack(Pack::C).object_ids[0], &mut buffer)?
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
