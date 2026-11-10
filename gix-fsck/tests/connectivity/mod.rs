use gix_error::Result;
use gix_fsck::Connectivity;
use gix_hash::ObjectId;
use gix_hashtable::HashMap;
use gix_object::Kind;
use std::sync::LazyLock;

use crate::hex_to_id;

fn check_missing<'a>(repo_name: &str, commits: impl IntoIterator<Item = &'a ObjectId>) -> HashMap<ObjectId, Kind> {
    let db = {
        let fixture_path = gix_testtools::scripted_fixture_read_only("make_test_repos.sh")
            .expect("fixture path")
            .join(repo_name)
            .join(".git")
            .join("objects");
        let mut db = gix_odb::at(fixture_path, gix_testtools::object_hash()).expect("valid odb");
        db.refresh_never();
        db
    };

    let mut missing: HashMap<ObjectId, Kind> = HashMap::default();
    let record_missing_and_assert_no_duplicate = |oid: &ObjectId, kind: Kind| {
        missing.try_insert(*oid, kind).expect("no duplicate oid");
    };

    let mut check = Connectivity::new(db, record_missing_and_assert_no_duplicate);
    for commit in commits.into_iter() {
        check.check_commit(commit).expect("commit is present");
    }
    missing
}

fn hex_to_ids<'a>(hex_ids: impl IntoIterator<Item = &'a str>) -> Vec<ObjectId> {
    hex_ids.into_iter().map(hex_to_id).collect()
}

fn hex_to_objects<'a>(hex_ids: impl IntoIterator<Item = &'a str>, kind: Kind) -> HashMap<ObjectId, Kind> {
    hex_to_ids(hex_ids).into_iter().map(|id| (id, kind)).collect()
}

// Get a `&Vec<ObjectID` for each commit in the test fixture repository
fn all_commits() -> &'static [ObjectId] {
    static ALL_COMMITS: LazyLock<Vec<ObjectId>> = LazyLock::new(|| {
        hex_to_ids([
            "ebed23648b19484cb1f340c4ee04dda08479188a",
            "8ff6d0f8891c3cb22827be142cc64606121d47b3",
            "734c926856a328d1168ffd7088532e0d1ad19bbe",
        ])
    });
    &ALL_COMMITS
}

#[test]
fn no_missing() {
    // The "base" repo is the original, and has every object present
    assert_eq!(check_missing("base", all_commits()), HashMap::default());
}

#[test]
fn blob_existence_errors_are_not_missing_objects() -> gix_error::TestResult {
    struct CorruptExistence(gix_odb::Handle);

    impl gix_object::Find for CorruptExistence {
        fn try_find<'a>(&self, id: &gix_hash::oid, buffer: &'a mut Vec<u8>) -> Result<Option<gix_object::Data<'a>>> {
            gix_object::Find::try_find(&self.0, id, buffer)
        }
    }

    impl gix_object::Exists for CorruptExistence {
        fn exists(&self, _id: &gix_hash::oid) -> Result<bool> {
            Err(gix_error::message("index could not be loaded").corrupted_error())
        }
    }

    let objects = gix_testtools::scripted_fixture_read_only("make_test_repos.sh")?.join("base/.git/objects");
    let db = CorruptExistence(gix_odb::at(objects, gix_testtools::object_hash())?);
    let mut check = Connectivity::new(db, |_, _| panic!("lookup failures must not report missing objects"));
    for _ in 0..2 {
        let err = check
            .check_commit(&all_commits()[0])
            .expect_err("failed blob existence checks must stop checking, including on retry");
        assert!(
            err.is_corrupted(),
            "the original lookup error classification is preserved"
        );
        assert_eq!(
            err.to_string(),
            "index could not be loaded",
            "the lookup diagnostic is preserved"
        );
    }
    Ok(())
}

#[test]
fn missing_blobs() {
    // The "blobless" repo is cloned with `--filter=blob:none`, and is missing one blob
    let expected = hex_to_objects(
        [
            "4cdeaab5b01f9a9fbbb2fb6c08404cf12b7bdab1",
            "c18147dc648481eeb65dc5e66628429a64843327",
        ],
        Kind::Blob,
    );
    assert_eq!(check_missing("blobless", all_commits()), expected);
}

#[test]
fn missing_trees() {
    // The "treeless" repo is cloned with `--filter=tree:0`, and is missing two trees
    // NOTE: This repo is also missing a blob, but we have no way of knowing that, as the tree referencing it is missing
    let expected = hex_to_objects(
        [
            "20317ffa7614f49b2702a057bf2833918ea9fd24",
            "fc264b3b6875a46e9031483aeb9994a1b897ffd3",
        ],
        Kind::Tree,
    );
    assert_eq!(check_missing("treeless", all_commits()), expected);
}
