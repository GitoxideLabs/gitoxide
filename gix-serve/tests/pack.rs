use std::sync::Arc;

use gix_odb::store::init::Options as OdbOptions;
use gix_ref::file::Store;
use gix_serve::{pack::generate_pack, refs::collect_refs};

fn store_from(script: &str) -> Store {
    let path = gix_testtools::scripted_fixture_read_only_standalone(script).expect("fixture script should run");
    Store::at(
        path.join(".git"),
        gix_ref::store::init::Options {
            write_reflog: gix_ref::store::WriteReflog::Disable,
            object_hash: gix_hash::Kind::Sha1,
            precompose_unicode: false,
            prohibit_windows_device_names: false,
        },
    )
}

fn odb_from(script: &str) -> gix_odb::HandleArc {
    let path = gix_testtools::scripted_fixture_read_only_standalone(script).expect("fixture script should run");
    let store = gix_odb::Store::at_opts(path.join(".git/objects"), &mut None.into_iter(), OdbOptions::default())
        .expect("odb should open");
    let mut cache = Arc::new(store).to_cache_arc();
    cache.prevent_pack_unload();
    cache
}

#[test]
fn pack_starts_with_magic_and_version() {
    let db = odb_from("make_repo_simple.sh");
    let refs = collect_refs(&store_from("make_repo_simple.sh")).unwrap();
    let main_oid = refs.iter().find(|r| r.name == "refs/heads/main").unwrap().object_id;

    let mut buf = Vec::new();
    generate_pack(db, &[main_oid], &[], &mut buf).unwrap();

    assert!(buf.len() > 12, "pack should have header + entries + checksum");
    assert_eq!(&buf[..4], b"PACK", "magic bytes");
    assert_eq!(&buf[4..8], &[0, 0, 0, 2], "version 2");
}

#[test]
fn pack_has_nonzero_entry_count() {
    let db = odb_from("make_repo_simple.sh");
    let refs = collect_refs(&store_from("make_repo_simple.sh")).unwrap();
    let main_oid = refs.iter().find(|r| r.name == "refs/heads/main").unwrap().object_id;

    let mut buf = Vec::new();
    generate_pack(db, &[main_oid], &[], &mut buf).unwrap();

    let num_entries = u32::from_be_bytes(buf[8..12].try_into().unwrap());
    assert!(num_entries > 0, "pack should contain at least one object");
}

#[test]
fn pack_ends_with_20_byte_checksum() {
    let db = odb_from("make_repo_simple.sh");
    let refs = collect_refs(&store_from("make_repo_simple.sh")).unwrap();
    let main_oid = refs.iter().find(|r| r.name == "refs/heads/main").unwrap().object_id;

    let mut buf = Vec::new();
    generate_pack(db, &[main_oid], &[], &mut buf).unwrap();

    assert!(buf.len() >= 32, "pack needs header + at least one entry + checksum");
    let checksum = &buf[buf.len() - 20..];
    assert!(checksum.iter().any(|&b| b != 0), "checksum should not be all zeros");
}
