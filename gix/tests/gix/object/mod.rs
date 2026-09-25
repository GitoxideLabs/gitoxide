mod blob;
mod commit;
mod tree;

use gix_testtools::size_ok;

#[test]
fn public_errors_use_the_crate_result_and_preserve_diagnostics() -> crate::Result {
    let repo = crate::basic_repo()?;
    let mut commit = repo.head_commit()?;
    commit.data = b"invalid commit".to_vec();
    let tag = gix::ObjectDetached {
        id: repo.object_hash().null(),
        kind: gix::objs::Kind::Tag,
        data: b"invalid tag".to_vec(),
    }
    .attach(&repo)
    .into_tag();
    let tree = gix::Tree::from_data(repo.object_hash().null(), b"invalid tree".to_vec(), &repo);
    let entry = tree.iter().next().expect("malformed entry is reported").map(|_| ());
    let results: [gix::Result<()>; 13] = [
        commit.message().map(|_| ()),
        commit.message_raw().map(|_| ()),
        commit.decode().map(|_| ()),
        commit.author().map(|_| ()),
        commit.committer().map(|_| ()),
        commit.tree_id().map(|_| ()),
        commit.signature().map(|_| ()),
        tag.decode().map(|_| ()),
        tag.target_id().map(|_| ()),
        tag.tagger().map(|_| ()),
        tree.decode().map(|_| ()),
        entry,
        gix::objs::Tree::try_from(tree).map(|_| ()),
    ];
    for result in results {
        let error = result.expect_err("malformed objects must fail to decode");
        assert!(
            error.downcast_any_ref::<gix_error::Message>().is_some(),
            "decoding errors retain their concrete plumbing diagnostic"
        );
    }

    let result: gix::Result<gix::Commit<'_>> = repo.head_tree_id()?.object()?.try_into_commit();
    assert!(
        result.expect_err("a tree cannot be a commit").is_validation(),
        "conversion errors retain their validation classification"
    );
    Ok(())
}

#[test]
fn failed_object_conversions_return_the_original_object() -> crate::Result {
    use gix::objs::Kind;

    let repo = crate::basic_repo()?;
    let object_id = repo.head_commit()?.id;
    for kind in [Kind::Blob, Kind::Commit, Kind::Tag, Kind::Tree] {
        let object = gix::ObjectDetached {
            id: object_id,
            kind,
            data: b"object bytes".to_vec(),
        }
        .attach(&repo);
        let data_ptr = object.data.as_ptr();
        let object: gix::Object<'_> = match kind {
            Kind::Blob => gix::Commit::try_from(object).err().expect("a blob is not a commit"),
            Kind::Commit => gix::Tag::try_from(object).err().expect("a commit is not a tag"),
            Kind::Tag => gix::Tree::try_from(object).err().expect("a tag is not a tree"),
            Kind::Tree => gix::Blob::try_from(object).err().expect("a tree is not a blob"),
        };
        assert_eq!(object.id, object_id, "the original object ID is returned");
        assert_eq!(object.kind, kind, "the original object kind is returned");
        assert_eq!(object.data, b"object bytes", "the original object data is returned");
        assert_eq!(object.data.as_ptr(), data_ptr, "the original allocation is returned");
        assert!(std::ptr::eq(object.repo, &repo), "the original repository is retained");
    }
    Ok(())
}

#[test]
fn object_ref_size_in_memory() {
    let actual = std::mem::size_of::<gix::Object<'_>>();
    let sha1 = 56;
    let sha256_extra = 16;
    let expected = sha1 + sha256_extra;
    assert!(
        size_ok(actual, expected),
        "the size of this structure should not change unexpectedly: {actual} <~ {expected}"
    );
}

#[test]
fn oid_size_in_memory() {
    let actual = std::mem::size_of::<gix::Id<'_>>();
    let sha1 = 32;
    let sha256_extra = 16;
    let expected = sha1 + sha256_extra;
    assert!(
        size_ok(actual, expected),
        "the size of this structure should not change unexpectedly: {actual} <~ {expected}"
    );
}
