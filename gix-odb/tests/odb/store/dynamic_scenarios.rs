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
    assert!(
        handle.try_find(&id, &mut buffer)?.is_none(),
        "a malformed index cannot provide its objects"
    );

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
fn a_failed_pack_load_is_shared_and_recovers_after_replacement() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    let mut first = open(&fixture, 4)?;
    let mut second = first.clone();
    first.refresh_never();
    second.refresh_never();
    assert_eq!(
        first.packed_object_count()?,
        fixture.manifest.pack(Pack::A).object_ids.len() as u64,
        "loading the index leaves its pack unopened"
    );

    fixture.corrupt_pack(Database::Primary, Pack::A)?;
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    let mut buffer = Vec::new();
    let first_err = first
        .try_find(&id, &mut buffer)
        .expect_err("the first handle observes the malformed pack");
    let second_err = second
        .try_find(&id, &mut buffer)
        .expect_err("the second handle observes the shared pack-load failure");
    assert_eq!(
        first_err.to_string(),
        second_err.to_string(),
        "shared handles report the same pack-load failure"
    );

    fixture.publish(Database::Primary, Pack::A, Component::Pack)?;
    assert_object(&second, &id)?;
    assert_object(&first, &id)?;
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
