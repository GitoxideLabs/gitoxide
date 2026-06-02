use std::sync::Arc;

use gix_hash::ObjectId;
use gix_odb::store::init::Options as OdbOptions;
use gix_packetline::blocking_io::encode::{data_to_write, delim_to_write, flush_to_write};
use gix_ref::file::Store;
use gix_ref::store::init::Options as RefOptions;
use gix_serve::refs::collect_refs;
use gix_serve::serve::serve_upload_pack;
use gix_transport::server::blocking_io::connection::Connection;
use gix_transport::{Protocol, Service};

fn ref_store(script: &str) -> Store {
    let path = gix_testtools::scripted_fixture_read_only_standalone(script).expect("fixture");
    Store::at(
        path.join(".git"),
        RefOptions {
            write_reflog: gix_ref::store::WriteReflog::Disable,
            object_hash: gix_hash::Kind::Sha1,
            precompose_unicode: false,
            prohibit_windows_device_names: false,
        },
    )
}

fn odb(script: &str) -> gix_odb::HandleArc {
    let path = gix_testtools::scripted_fixture_read_only_standalone(script).expect("fixture");
    let store =
        gix_odb::Store::at_opts(path.join(".git/objects"), &mut None.into_iter(), OdbOptions::default()).expect("odb");
    let mut cache = Arc::new(store).to_cache_arc();
    cache.prevent_pack_unload();
    cache
}

fn build_v1_want_done(oid: &ObjectId) -> Vec<u8> {
    let mut buf = Vec::new();
    data_to_write(format!("want {}\n", oid.to_hex()).as_bytes(), &mut buf).unwrap();
    flush_to_write(&mut buf).unwrap();
    data_to_write(b"done\n", &mut buf).unwrap();
    flush_to_write(&mut buf).unwrap();
    buf
}

fn build_v2_fetch_input(wants: &[ObjectId]) -> Vec<u8> {
    let mut buf = Vec::new();
    data_to_write(b"command=fetch\n", &mut buf).unwrap();
    delim_to_write(&mut buf).unwrap();
    for oid in wants {
        data_to_write(format!("want {}\n", oid.to_hex()).as_bytes(), &mut buf).unwrap();
    }
    data_to_write(b"done\n", &mut buf).unwrap();
    flush_to_write(&mut buf).unwrap();
    buf
}

fn assert_has_valid_pack(output: &[u8]) {
    let pack_pos = output.windows(4).position(|w| w == b"PACK");
    assert!(pack_pos.is_some(), "output should contain pack data");

    let pack_start = pack_pos.unwrap();
    assert_eq!(
        &output[pack_start + 4..pack_start + 8],
        &[0, 0, 0, 2],
        "pack format version 2"
    );

    let num_entries = u32::from_be_bytes(output[pack_start + 8..pack_start + 12].try_into().unwrap());
    assert!(num_entries > 0, "pack should contain objects");
}

#[test]
fn v1_fresh_clone() {
    let store = ref_store("make_repo_simple.sh");
    let db = odb("make_repo_simple.sh");
    let refs = collect_refs(&store).unwrap();
    let main_oid = refs.iter().find(|r| r.name == "refs/heads/main").unwrap().object_id;

    let input = build_v1_want_done(&main_oid);
    let mut output = Vec::new();
    let mut conn = Connection::new(
        &input[..],
        &mut output,
        Service::UploadPack,
        "/repo.git",
        Protocol::V1,
        false,
    );

    serve_upload_pack(&store, db, &mut conn, Protocol::V1).unwrap();

    assert!(!output.is_empty());
    let nak_pos = output.windows(3).position(|w| w == b"NAK");
    let pack_pos = output.windows(4).position(|w| w == b"PACK").unwrap();
    assert!(nak_pos.is_some());
    assert!(nak_pos.unwrap() < pack_pos);
    assert_has_valid_pack(&output);
}

#[test]
fn v1_empty_wants() {
    let store = ref_store("make_repo_simple.sh");
    let db = odb("make_repo_simple.sh");

    let mut input = Vec::new();
    flush_to_write(&mut input).unwrap();
    let mut output = Vec::new();
    let mut conn = Connection::new(
        &input[..],
        &mut output,
        Service::UploadPack,
        "/repo.git",
        Protocol::V1,
        false,
    );

    serve_upload_pack(&store, db, &mut conn, Protocol::V1).unwrap();

    assert!(!output.is_empty());
    assert!(!output.windows(4).any(|w| w == b"PACK"));
}

#[test]
fn v2_fresh_clone() {
    let store = ref_store("make_repo_simple.sh");
    let db = odb("make_repo_simple.sh");
    let refs = collect_refs(&store).unwrap();
    let main_oid = refs.iter().find(|r| r.name == "refs/heads/main").unwrap().object_id;

    let input = build_v2_fetch_input(&[main_oid]);
    let mut output = Vec::new();
    let mut conn = Connection::new(
        &input[..],
        &mut output,
        Service::UploadPack,
        "/repo.git",
        Protocol::V2,
        false,
    );

    serve_upload_pack(&store, db, &mut conn, Protocol::V2).unwrap();

    assert!(!output.is_empty());
    assert_has_valid_pack(&output);
}
