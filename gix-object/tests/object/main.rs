use std::{path::PathBuf, sync::atomic::AtomicBool};

use gix_hash::ObjectId;
use gix_object::WriteTo;

mod commit;
mod encode;
mod object_ref;
mod tag;
mod tree;

#[test]
fn compute_hash() {
    for hk in gix_hash::Kind::all() {
        assert_eq!(
            gix_object::compute_hash(*hk, gix_object::Kind::Blob, &[]).expect("empty hash doesn’t collide"),
            gix_hash::ObjectId::empty_blob(*hk)
        );
        assert_eq!(
            gix_object::compute_hash(*hk, gix_object::Kind::Tree, &[]).expect("empty hash doesn’t collide"),
            gix_hash::ObjectId::empty_tree(*hk)
        );
    }
}

#[test]
fn compute_stream_hash() {
    for hk in gix_hash::Kind::all() {
        assert_eq!(
            gix_object::compute_stream_hash(
                *hk,
                gix_object::Kind::Blob,
                &mut &[][..],
                0,
                &mut gix_features::progress::Discard,
                &AtomicBool::default()
            )
            .expect("in-memory works"),
            gix_hash::ObjectId::empty_blob(*hk)
        );
        assert_eq!(
            gix_object::compute_stream_hash(
                *hk,
                gix_object::Kind::Tree,
                &mut &[][..],
                0,
                &mut gix_features::progress::Discard,
                &AtomicBool::default()
            )
            .expect("in-memory works"),
            gix_hash::ObjectId::empty_tree(*hk)
        );
    }
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

pub fn fixture_hash_kind() -> gix_hash::Kind {
    gix_testtools::object_hash()
}

fn fixture_bytes(path: &str) -> Vec<u8> {
    fixup(std::fs::read(fixture(path)).unwrap())
}

pub(crate) fn fuzz_artifact_paths(target: &str) -> Vec<PathBuf> {
    let mut paths = std::fs::read_dir(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("fuzz/artifacts")
            .join(target),
    )
    .expect("artifact directory exists")
    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
    .collect::<Vec<_>>();
    paths.sort();
    paths
}

fn fixture_name(kind: &str, path: &str) -> Vec<u8> {
    fixup(fixture_bytes(PathBuf::from(kind).join(path).to_str().unwrap()))
}

/// Return the object id expected in fixture assertions for the active fixture hash kind.
///
/// Object fixtures in this test module are authored as SHA-1 data and are rewritten on demand for
/// SHA-256 runs. This helper mirrors that rewrite on the expectation side so parsing tests can use
/// one set of source ids for both hash kinds.
pub fn fixture_oid(hex: &str) -> ObjectId {
    let oid = hex_to_id(hex);
    translate_fixture_oid(oid, fixture_hash_kind())
}

fn translate_fixture_oid(oid: ObjectId, kind: gix_hash::Kind) -> ObjectId {
    match kind {
        gix_hash::Kind::Sha1 => oid,
        kind => {
            let mut hasher = gix_hash::hasher(kind);
            hasher.update(oid.as_bytes());
            hasher.try_finalize().expect("sha256 hashing is available for tests")
        }
    }
}

/// Load an object fixture and, if needed, rewrite its SHA-1 object ids for the active fixture hash kind.
pub fn object_fixture(path: &str) -> Result<Vec<u8>> {
    if let Some(path) = path.strip_prefix("tree/") {
        tree_fixture(path)
    } else if let Some(path) = path.strip_prefix("commit/") {
        commit_fixture(path)
    } else if let Some(path) = path.strip_prefix("tag/") {
        tag_fixture(path)
    } else {
        Ok(fixture_bytes(path))
    }
}

/// Load a tree fixture and, if needed, rewrite its embedded entry ids for the active fixture hash kind.
///
/// The on-disk `tree/*.tree` fixtures contain SHA-1-sized ids. For SHA-256 test runs we parse the
/// SHA-1 fixture, rewrite each entry id into the synthetic SHA-256 ids produced by [`fixture_oid()`],
/// and re-encode the tree so parsers see correctly-sized object ids.
pub fn tree_fixture(path: &str) -> Result<Vec<u8>> {
    let fixture = fixture_name("tree", path);
    match fixture_hash_kind() {
        gix_hash::Kind::Sha1 => Ok(fixture),
        kind => {
            let mut tree: gix_object::Tree = gix_object::TreeRef::from_bytes(&fixture, gix_hash::Kind::Sha1)?.into();
            for entry in &mut tree.entries {
                entry.oid = translate_fixture_oid(entry.oid, kind);
            }
            let mut out = Vec::with_capacity(
                fixture.len() + tree.entries.len() * (kind.len_in_bytes() - gix_hash::Kind::Sha1.len_in_bytes()),
            );
            tree.write_to(&mut out)?;
            Ok(out)
        }
    }
}

fn commit_fixture(path: &str) -> Result<Vec<u8>> {
    let fixture = fixture_name("commit", path);
    match fixture_hash_kind() {
        gix_hash::Kind::Sha1 => Ok(fixture),
        kind => {
            let mut commit = gix_object::CommitRef::from_bytes(&fixture, gix_hash::Kind::Sha1)?.into_owned()?;
            commit.tree = translate_fixture_oid(commit.tree, kind);
            for parent in &mut commit.parents {
                *parent = translate_fixture_oid(*parent, kind);
            }

            let mut out = Vec::with_capacity(
                fixture.len()
                    + (1 + commit.parents.len()) * (kind.len_in_bytes() - gix_hash::Kind::Sha1.len_in_bytes()),
            );
            commit.write_to(&mut out)?;
            Ok(out)
        }
    }
}

fn tag_fixture(path: &str) -> Result<Vec<u8>> {
    let fixture = fixture_name("tag", path);
    match fixture_hash_kind() {
        gix_hash::Kind::Sha1 => Ok(fixture),
        kind => {
            let mut tag = gix_object::TagRef::from_bytes(&fixture, gix_hash::Kind::Sha1)?.into_owned()?;
            tag.target = translate_fixture_oid(tag.target, kind);

            let mut out = Vec::with_capacity(fixture.len() + kind.len_in_bytes() - gix_hash::Kind::Sha1.len_in_bytes());
            tag.write_to(&mut out)?;
            Ok(out)
        }
    }
}

pub fn generated_tree_root_id() -> Result<ObjectId> {
    let root = gix_testtools::scripted_fixture_read_only("make_trees.sh")?;
    let tree = std::fs::read(root.join("tree.baseline"))?;
    Ok(gix_object::compute_hash(
        fixture_hash_kind(),
        gix_object::Kind::Tree,
        &tree,
    )?)
}

pub fn normalize_tree_snapshot(input: &str) -> String {
    gix_testtools::normalize_hashes(input).0
}

#[test]
fn size_in_memory() {
    let actual = std::mem::size_of::<gix_object::Object>();
    let sha1 = 272;
    let sha256_extra = 16;
    let expected = sha1 + sha256_extra;
    assert!(
        actual <= expected,
        "{actual} <= {expected}: Prevent unexpected growth of what should be lightweight objects"
    );
}

fn hex_to_id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

fn signature(time: &str) -> gix_actor::SignatureRef<'_> {
    use gix_object::bstr::ByteSlice;
    gix_actor::SignatureRef {
        name: b"Sebastian Thiel".as_bstr(),
        email: b"sebastian.thiel@icloud.com".as_bstr(),
        time,
    }
}

fn linus_signature(time: &str) -> gix_actor::SignatureRef<'_> {
    use gix_object::bstr::ByteSlice;
    gix_actor::SignatureRef {
        name: b"Linus Torvalds".as_bstr(),
        email: b"torvalds@linux-foundation.org".as_bstr(),
        time,
    }
}
