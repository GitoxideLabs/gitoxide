use std::{path::PathBuf, sync::atomic::AtomicBool};

use gix_hash::ObjectId;

mod commit;
mod encode;
mod object_ref;
mod tag;
mod tree;

#[test]
fn compute_hash() {
    let hk = gix_hash::Kind::Sha1;
    assert_eq!(
        gix_object::compute_hash(hk, gix_object::Kind::Blob, &[]).expect("empty hash doesn’t collide"),
        gix_hash::ObjectId::empty_blob(hk)
    );
    assert_eq!(
        gix_object::compute_hash(hk, gix_object::Kind::Tree, &[]).expect("empty hash doesn’t collide"),
        gix_hash::ObjectId::empty_tree(hk)
    );
}

#[test]
fn compute_stream_hash() {
    let hk = gix_hash::Kind::Sha1;
    assert_eq!(
        gix_object::compute_stream_hash(
            hk,
            gix_object::Kind::Blob,
            &mut &[][..],
            0,
            &mut gix_features::progress::Discard,
            &AtomicBool::default()
        )
        .expect("in-memory works"),
        gix_hash::ObjectId::empty_blob(hk)
    );
    assert_eq!(
        gix_object::compute_stream_hash(
            hk,
            gix_object::Kind::Tree,
            &mut &[][..],
            0,
            &mut gix_features::progress::Discard,
            &AtomicBool::default()
        )
        .expect("in-memory works"),
        gix_hash::ObjectId::empty_tree(hk)
    );
}

use gix_testtools::Result;

#[cfg(not(windows))]
fn fixup(v: Vec<u8>) -> Vec<u8> {
    v
}

#[cfg(windows)]
fn fixup(v: Vec<u8>) -> Vec<u8> {
    // Git checks out text files with line ending conversions, git itself will of course not put '\r\n' anywhere,
    // so that wouldn't be expected in an object and doesn't have to be parsed.
    use bstr::ByteSlice;
    v.replace(b"\r\n", "\n")
}

pub fn fixture(path: &str) -> PathBuf {
    PathBuf::from("tests/fixtures").join(path)
}

fn fixture_bytes(path: &str) -> Vec<u8> {
    fixup(std::fs::read(fixture(path)).unwrap())
}

fn fixture_name(kind: &str, path: &str) -> Vec<u8> {
    fixup(fixture_bytes(PathBuf::from(kind).join(path).to_str().unwrap()))
}

#[test]
fn size_in_memory() {
    let actual = std::mem::size_of::<gix_object::Object>();
    assert!(
        actual <= 272,
        "{actual} <= 272: Prevent unexpected growth of what should be lightweight objects"
    );
}

fn hex_to_id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

fn signature(time: &[u8]) -> gix_actor::SignatureRef<'_> {
    use gix_object::bstr::ByteSlice;
    gix_actor::SignatureRef {
        name: b"Sebastian Thiel".as_bstr(),
        email: b"sebastian.thiel@icloud.com".as_bstr(),
        time: time.as_bstr(),
    }
}

fn linus_signature(time: &[u8]) -> gix_actor::SignatureRef<'_> {
    use gix_object::bstr::ByteSlice;
    gix_actor::SignatureRef {
        name: b"Linus Torvalds".as_bstr(),
        email: b"torvalds@linux-foundation.org".as_bstr(),
        time: time.as_bstr(),
    }
}
