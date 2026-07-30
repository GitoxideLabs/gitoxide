use gix_object::Find as _;
use gix_odb::Header as _;

use crate::{
    Result,
    odb_fixture::{Component, Database, OdbFixture, Pack},
};

fn open(fixture: &OdbFixture, slots: u16) -> std::io::Result<gix_odb::Handle> {
    gix_odb::at_opts(
        fixture.objects_dir(Database::Primary),
        fixture.manifest.object_hash,
        Vec::new(),
        gix_odb::store::init::Options {
            slots: gix_odb::store::init::Slots::Given(slots),
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
fn malformed_index_can_be_restored() -> Result {
    let mut fixture = OdbFixture::from_script()?;
    fixture.install_pack(Database::Primary, Pack::A)?;
    fixture.corrupt_index(Database::Primary, Pack::A)?;
    assert!(!fixture.is_valid(), "the helper tracks the malformed component");
    let handle = open(&fixture, 4)?;
    let id = fixture.manifest.pack(Pack::A).object_ids[0];
    let mut buffer = Vec::new();
    let _ = handle.try_find(&id, &mut buffer);

    fixture.publish(Database::Primary, Pack::A, Component::Index)?;
    assert!(fixture.is_valid(), "restoring the generated index makes the ODB valid");
    assert_object(&open(&fixture, 4)?, &id)?;
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
