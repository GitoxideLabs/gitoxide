use bstr::ByteSlice;
use gix_filter::{eol, eol::AutoCrlf};
use gix_merge::blob::{
    Pipeline, ResourceKind,
    pipeline::{self, Mode, WorktreeRoots},
};
use gix_object::tree::EntryKind;

use crate::blob::util::ObjectDb;

const ALL_MODES: [pipeline::Mode; 2] = [pipeline::Mode::ToGit, pipeline::Mode::Renormalize];

#[test]
fn without_transformation() -> crate::Result {
    for mode in ALL_MODES {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let mut filter = Pipeline::new(
            WorktreeRoots {
                common_ancestor_root: Some(tmp.path().to_owned()),
                ..Default::default()
            },
            gix_filter::Pipeline::default(),
            default_options(),
        );

        let does_not_matter = gix_hash::Kind::Sha1.null();
        let mut buf = Vec::new();
        let a_name = "a";
        let a_content = "a-content";
        std::fs::write(tmp.path().join(a_name), a_content.as_bytes())?;
        let out = filter.convert_to_mergeable(
            &does_not_matter,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::CommonAncestorOrBase,
            &mut |_, _| {},
            &gix_object::find::Never,
            mode,
            &mut buf,
        )?;
        assert_eq!(out, Some(pipeline::Data::Buffer));
        assert_eq!(buf.as_bstr(), a_content, "there is no transformations configured");

        let link_name = "link";
        gix_fs::symlink::create(a_name.as_ref(), &tmp.path().join(link_name))?;
        let err = filter
            .convert_to_mergeable(
                &does_not_matter,
                EntryKind::Link,
                link_name.into(),
                ResourceKind::CommonAncestorOrBase,
                &mut |_, _| {},
                &gix_object::find::Never,
                mode,
                &mut buf,
            )
            .unwrap_err();

        assert!(
            matches!(err, pipeline::convert_to_mergeable::Error::InvalidEntryKind {rela_path,actual}
                if rela_path == link_name && actual == EntryKind::Link)
        );
        assert_eq!(
            buf.len(),
            9,
            "input buffers are cleared only if we think they are going to be used"
        );
        drop(tmp);

        let mut db = ObjectDb::default();
        let b_content = "b-content";
        let id = db.insert(b_content)?;

        let out = filter.convert_to_mergeable(
            &id,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::CurrentOrOurs,
            &mut |_, _| {},
            &db,
            mode,
            &mut buf,
        )?;

        assert_eq!(out, Some(pipeline::Data::Buffer));
        assert_eq!(
            buf.as_bstr(),
            b_content,
            "there is no transformations configured, it fetched the data from the ODB"
        );

        let out = filter.convert_to_mergeable(
            &does_not_matter,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::OtherOrTheirs,
            &mut |_, _| {},
            &gix_object::find::Never,
            mode,
            &mut buf,
        )?;
        assert_eq!(out, None, "the lack of object in the database isn't a problem");

        let out = filter.convert_to_mergeable(
            &does_not_matter,
            EntryKind::Blob,
            "does not exist on disk".into(),
            ResourceKind::CommonAncestorOrBase,
            &mut |_, _| {},
            &gix_object::find::Never,
            mode,
            &mut buf,
        )?;
        assert_eq!(out, None, "the lack of file on disk is fine as well");
    }

    Ok(())
}

#[test]
fn binary_below_large_file_threshold() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let mut filter = Pipeline::new(
        WorktreeRoots {
            current_root: Some(tmp.path().to_owned()),
            ..Default::default()
        },
        gix_filter::Pipeline::default(),
        pipeline::Options {
            large_file_threshold_bytes: 5,
        },
    );

    let does_not_matter = gix_hash::Kind::Sha1.null();
    let mut buf = Vec::new();
    let a_name = "a";
    let binary_content = "a\0b";
    std::fs::write(tmp.path().join(a_name), binary_content.as_bytes())?;
    let out = filter.convert_to_mergeable(
        &does_not_matter,
        EntryKind::BlobExecutable,
        a_name.into(),
        ResourceKind::CurrentOrOurs,
        &mut |_, _| {},
        &gix_object::find::Never,
        pipeline::Mode::ToGit,
        &mut buf,
    )?;
    assert_eq!(out, Some(pipeline::Data::Buffer), "binary data can still be merged");
    assert_eq!(buf.as_bstr(), binary_content);

    let mut db = ObjectDb::default();
    let id = db.insert(binary_content)?;
    let out = filter.convert_to_mergeable(
        &id,
        EntryKind::Blob,
        a_name.into(),
        ResourceKind::OtherOrTheirs,
        &mut |_, _| {},
        &db,
        pipeline::Mode::ToGit,
        &mut buf,
    )?;
    assert_eq!(out, Some(pipeline::Data::Buffer));
    assert_eq!(buf.as_bstr(), binary_content);

    Ok(())
}

#[test]
fn above_large_file_threshold() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let mut filter = gix_merge::blob::Pipeline::new(
        WorktreeRoots {
            current_root: Some(tmp.path().to_owned()),
            ..Default::default()
        },
        gix_filter::Pipeline::default(),
        pipeline::Options {
            large_file_threshold_bytes: 4,
        },
    );

    let does_not_matter = gix_hash::Kind::Sha1.null();
    let mut buf = Vec::new();
    let a_name = "a";
    let large_content = "hello";
    std::fs::write(tmp.path().join(a_name), large_content.as_bytes())?;
    let out = filter.convert_to_mergeable(
        &does_not_matter,
        EntryKind::BlobExecutable,
        a_name.into(),
        ResourceKind::CurrentOrOurs,
        &mut |_, _| {},
        &gix_object::find::Never,
        pipeline::Mode::ToGit,
        &mut buf,
    )?;
    assert_eq!(
        out,
        Some(pipeline::Data::TooLarge { size: 5 }),
        "it indicates that the file is too large"
    );
    assert_eq!(buf.len(), 0, "it should avoid querying that data in the first place");

    drop(tmp);
    let mut db = ObjectDb::default();
    let id = db.insert(large_content)?;

    let out = filter.convert_to_mergeable(
        &id,
        EntryKind::Blob,
        a_name.into(),
        ResourceKind::CommonAncestorOrBase,
        &mut |_, _| {},
        &db,
        pipeline::Mode::ToGit,
        &mut buf,
    )?;

    assert_eq!(out, Some(pipeline::Data::TooLarge { size: 5 }));
    assert_eq!(
        buf.len(),
        0,
        "it won't have queried the blob, first it checks the header"
    );

    Ok(())
}

#[test]
fn non_existing() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let mut filter = Pipeline::new(
        WorktreeRoots {
            common_ancestor_root: Some(tmp.path().to_owned()),
            ..Default::default()
        },
        gix_filter::Pipeline::default(),
        default_options(),
    );

    let null = gix_hash::Kind::Sha1.null();
    let mut buf = vec![1];
    let a_name = "a";
    assert!(
        !tmp.path().join(a_name).exists(),
        "precondition: worktree file doesn't exist"
    );
    let out = filter.convert_to_mergeable(
        &null,
        EntryKind::Blob,
        a_name.into(),
        ResourceKind::CommonAncestorOrBase,
        &mut |_, _| {},
        &gix_object::find::Never,
        pipeline::Mode::ToGit,
        &mut buf,
    )?;
    assert_eq!(
        out, None,
        "it's OK for a resource to not exist on disk - they'd then count as deleted"
    );
    assert_eq!(buf.len(), 0, "always cleared");

    drop(tmp);

    buf.push(1);
    let out = filter.convert_to_mergeable(
        &null,
        EntryKind::Blob,
        a_name.into(),
        ResourceKind::OtherOrTheirs,
        &mut |_, _| {},
        &gix_object::find::Never,
        pipeline::Mode::ToGit,
        &mut buf,
    )?;

    assert_eq!(
        out, None,
        "the root path isn't configured and the object database returns nothing"
    );
    assert_eq!(buf.len(), 0, "it's always cleared before any potential use");

    let some_id = gix_hash::ObjectId::from_hex(b"45c160c35c17ad264b96431cceb9793160396e99")?;
    let err = filter
        .convert_to_mergeable(
            &some_id,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::OtherOrTheirs,
            &mut |_, _| {},
            &gix_object::find::Never,
            pipeline::Mode::ToGit,
            &mut buf,
        )
        .unwrap_err();
    assert!(
        matches!(
            err,
            gix_merge::blob::pipeline::convert_to_mergeable::Error::FindObject(
                gix_object::find::existing_object::Error::NotFound { .. }
            ),
        ),
        "missing object database ids are always an error (even though missing objects on disk are allowed)"
    );
    Ok(())
}

#[test]
fn worktree_filter() -> crate::Result {
    let tmp = gix_testtools::tempfile::TempDir::new()?;
    let filter = gix_filter::Pipeline::new(
        Default::default(),
        gix_filter::pipeline::Options {
            eol_config: eol::Configuration {
                auto_crlf: AutoCrlf::Enabled,
                ..Default::default()
            },
            ..Default::default()
        },
    );
    let mut filter = gix_merge::blob::Pipeline::new(
        WorktreeRoots {
            common_ancestor_root: Some(tmp.path().to_owned()),
            ..Default::default()
        },
        filter,
        default_options(),
    );

    let mut db = ObjectDb::default();
    let a_name = "a";
    let mut buf = Vec::new();
    let a_content = "a-content\r\n";
    std::fs::write(tmp.path().join(a_name), a_content.as_bytes())?;
    for mode in ALL_MODES {
        let does_not_matter = gix_hash::Kind::Sha1.null();
        let out = filter.convert_to_mergeable(
            &does_not_matter,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::CommonAncestorOrBase,
            &mut |_, _| {},
            &gix_object::find::Never,
            mode,
            &mut buf,
        )?;
        assert_eq!(out, Some(pipeline::Data::Buffer));
        assert_eq!(
            buf.as_bstr(),
            "a-content\n",
            "worktree files need to be converted back to what's stored in Git"
        );

        let id = db.insert(a_content)?;
        let out = filter.convert_to_mergeable(
            &id,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::CommonAncestorOrBase,
            &mut |_, _| {},
            &db,
            mode,
            &mut buf,
        )?;
        assert_eq!(out, Some(pipeline::Data::Buffer));
        match mode {
            Mode::ToGit => {
                assert_eq!(
                    buf.as_bstr(),
                    "a-content\r\n",
                    "if an object with CRLF already exists, we don't 'renormalize' it, it's a feature"
                );
            }
            Mode::Renormalize => {
                assert_eq!(
                    buf.as_bstr(),
                    "a-content\n",
                    "we can also do it if the file exists both on disk and is known to the ODB"
                );
            }
        }
    }

    drop(tmp);

    let b_content = "b-content\n";
    let id = db.insert(b_content)?;

    let out = filter.convert_to_mergeable(
        &id,
        EntryKind::Blob,
        a_name.into(),
        ResourceKind::CurrentOrOurs,
        &mut |_, _| {},
        &db,
        pipeline::Mode::ToGit,
        &mut buf,
    )?;

    assert_eq!(out, Some(pipeline::Data::Buffer));
    assert_eq!(buf.as_bstr(), b_content, "no work is done for what's already in Git");

    let mut db = ObjectDb::default();
    let b_content = "b-content\r\n";
    let id = db.insert(b_content)?;
    let out = filter.convert_to_mergeable(
        &id,
        EntryKind::Blob,
        a_name.into(),
        ResourceKind::OtherOrTheirs,
        &mut |_, _| {},
        &db,
        pipeline::Mode::Renormalize,
        &mut buf,
    )?;

    assert_eq!(out, Some(pipeline::Data::Buffer));
    assert_eq!(
        buf.as_bstr(),
        "b-content\n",
        "we see what would have been stored if the file was checked out and checked in again.\
        It explicitly ignores what's in Git already (or it wouldn't do anyting)"
    );

    Ok(())
}

fn default_options() -> pipeline::Options {
    pipeline::Options {
        large_file_threshold_bytes: 0,
    }
}
