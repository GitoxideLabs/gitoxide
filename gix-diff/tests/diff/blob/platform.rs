use crate::Result;
use gix_diff::blob::{Algorithm, Platform, ResourceKind, pipeline, platform, platform::prepare_diff::Operation};
use gix_object::{
    bstr::{BString, ByteSlice},
    tree::EntryKind,
};

use crate::{
    blob::pipeline::convert_to_diffable::default_options,
    hex_to_id,
    util::{insert, object_db},
};

#[test]
fn resources_of_worktree_and_odb_and_check_link() -> Result {
    let mut platform = new_platform(
        Some(gix_diff::blob::Driver {
            name: "a".into(),
            ..Default::default()
        }),
        gix_diff::blob::pipeline::Mode::default(),
    );
    platform.set_resource(
        gix_hash::Kind::Sha1.null(),
        EntryKind::Blob,
        "a".into(),
        ResourceKind::OldOrSource,
        &gix_object::find::Never,
    )?;

    let db = object_db();
    let a_content = "a-content";
    let id = insert(&db, a_content)?;
    platform.set_resource(
        id,
        EntryKind::BlobExecutable,
        "a".into(),
        ResourceKind::NewOrDestination,
        &db,
    )?;

    let (old, new) = platform.resources().expect("previously set source and destination");
    assert_eq!(old.data.as_slice().expect("present").as_bstr(), "a\n");
    assert_eq!(old.driver_index, Some(0));
    assert_eq!(old.mode, EntryKind::Blob);
    assert!(old.id.is_null(), "id is used verbatim");
    assert_eq!(old.rela_path, "a", "location is kept directly as provided");
    assert_eq!(new.data.as_slice().expect("present").as_bstr(), a_content);
    assert_eq!(new.driver_index, Some(0));
    assert_eq!(new.mode, EntryKind::BlobExecutable);
    let new_id = hex_to_id(
        "4c469b6c8c4486fdc9ded9d597d8f6816a455707",
        "061557fb6e67e1be5d41f8e83962699fce2a7d168ccfb2b5743d27ab06038da8",
    );
    assert_eq!(new.id, new_id);
    assert_eq!(new.rela_path, "a", "location is kept directly as provided");

    let out = platform.prepare_diff()?;
    assert_eq!(
        out.operation,
        Operation::InternalDiff {
            algorithm: Algorithm::Histogram
        },
        "it ends up with the default, as it's not overridden anywhere"
    );

    assert_eq!(
        comparable_ext_diff(platform.prepare_diff_command(
            "test".into(),
            gix_diff::command::Context {
                git_dir: Some(".".into()),
                ..Default::default()
            },
            2,
            3
        )),
        format!(
            "{}test a <tmp-path> 0000000000000000000000000000000000000000 100644 <tmp-path> {new_id} 100755",
            if !cfg!(windows) {
                "GIT_DIFF_PATH_COUNTER=3 GIT_DIFF_PATH_TOTAL=3 GIT_DIR=. "
            } else {
                ""
            }
        ),
        "in this case, there is no rename-to field as last argument, it's based on the resource paths being different"
    );

    let command = platform.prepare_diff_command("test --flag".into(), Default::default(), 0, 1)?;
    assert!(
        command
            .get_args()
            .any(|arg| arg.to_string_lossy().contains("test --flag")),
        "configured command lines with arguments are interpreted by a shell"
    );

    platform.set_resource(id, EntryKind::Link, "a".into(), ResourceKind::NewOrDestination, &db)?;

    // Double-inserts are fine.
    platform.set_resource(id, EntryKind::Link, "a".into(), ResourceKind::NewOrDestination, &db)?;
    let (old, new) = platform.resources().expect("previously set source and destination");
    assert_eq!(
        old.data.as_slice().expect("present").as_bstr(),
        "a\n",
        "the source is still the same"
    );
    assert_eq!(old.mode, EntryKind::Blob);
    assert_eq!(
        new.mode,
        EntryKind::Link,
        "but the destination has changed as is now a link"
    );
    assert_eq!(
        new.data.as_slice().expect("present").as_bstr(),
        a_content,
        "despite the same content"
    );
    assert_eq!(new.id, new_id);
    assert_eq!(new.rela_path, "a");

    let out = platform.prepare_diff()?;
    assert_eq!(
        out.operation,
        Operation::InternalDiff {
            algorithm: Algorithm::Histogram
        },
        "it would still diff, despite this being blob-with-link now. But that's fine."
    );

    assert_eq!(
        comparable_ext_diff(platform.prepare_diff_command(
            "test".into(),
            gix_diff::command::Context {
                git_dir: Some(".".into()),
                ..Default::default()
            },
            0,
            1
        )),
        format!(
            "{}test a <tmp-path> 0000000000000000000000000000000000000000 100644 <tmp-path> {new_id} 120000",
            if !cfg!(windows) {
                r#"GIT_DIFF_PATH_COUNTER=1 GIT_DIFF_PATH_TOTAL=1 GIT_DIR=. "#
            } else {
                ""
            }
        ),
        "Also obvious that symlinks are definitely special, but it's what git does as well"
    );

    assert_eq!(
        platform.clear_resource_cache_keep_allocation(),
        3,
        "some buffers are retained and reused"
    );
    assert_eq!(
        platform.resources(),
        None,
        "clearing the cache voids resources and one has to set it up again"
    );

    assert_eq!(
        platform.clear_resource_cache_keep_allocation(),
        2,
        "doing this again keeps 2 buffers"
    );
    assert_eq!(
        platform.clear_resource_cache_keep_allocation(),
        2,
        "no matter what - after all we need at least two resources for a diff"
    );

    platform.clear_resource_cache();
    assert_eq!(
        platform.clear_resource_cache_keep_allocation(),
        0,
        "after a proper clearing, the free-list is also emptied, and it won't be recreated"
    );

    Ok(())
}

fn comparable_ext_diff(cmd: gix_error::Result<gix_diff::blob::platform::prepare_diff_command::Command>) -> String {
    let cmd = cmd.expect("no error");
    let command = format!("{:?}", *cmd);
    let parsed = gix_diff::command::parse::command_line(command.as_str().into()).expect("parses fine");
    let env_len = parsed.env.len();
    parsed
        .env
        .into_iter()
        .map(|(name, value)| {
            format!(
                "{name}={}",
                value.into_string().expect("parsing a UTF-8 command preserves UTF-8")
            )
        })
        .chain(
            std::iter::once(parsed.command)
                .chain(parsed.args)
                .map(|arg| arg.into_string().expect("parsing a UTF-8 command preserves UTF-8")),
        )
        .enumerate()
        .filter_map(|(idx, s)| {
            (idx != env_len + 2 && idx != env_len + 5)
                .then_some(s)
                .or_else(|| Some("<tmp-path>".into()))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn diff_binary() -> Result {
    let mut platform = new_platform(
        Some(gix_diff::blob::Driver {
            name: "a".into(),
            is_binary: Some(true),
            ..Default::default()
        }),
        gix_diff::blob::pipeline::Mode::default(),
    );
    platform.set_resource(
        gix_hash::Kind::Sha1.null(),
        EntryKind::Blob,
        "a".into(),
        ResourceKind::OldOrSource,
        &gix_object::find::Never,
    )?;

    let db = object_db();
    let a_content = "b";
    let id = insert(&db, a_content)?;
    platform.set_resource(id, EntryKind::Blob, "b".into(), ResourceKind::NewOrDestination, &db)?;

    let out = platform.prepare_diff()?;
    assert!(
        matches!(out.operation, Operation::SourceOrDestinationIsBinary),
        "one binary resource is enough to skip diffing entirely"
    );

    match platform.prepare_diff_command("test".into(), Default::default(), 0, 1) {
        Err(err) => {
            insta::assert_debug_snapshot!(err, "diff binary", @"Binary resources can't be diffed with an external command (as we don't have the data anymore)");
        }
        Ok(_) => unreachable!("must error"),
    }

    Ok(())
}

#[test]
fn diff_performed_despite_external_command() -> Result {
    let mut platform = new_platform(
        Some(gix_diff::blob::Driver {
            name: "a".into(),
            command: Some("something-to-be-ignored".into()),
            algorithm: Some(Algorithm::Myers),
            ..Default::default()
        }),
        gix_diff::blob::pipeline::Mode::default(),
    );
    platform.set_resource(
        gix_hash::Kind::Sha1.null(),
        EntryKind::Blob,
        "a".into(),
        ResourceKind::OldOrSource,
        &gix_object::find::Never,
    )?;

    let db = object_db();
    let a_content = "b";
    let id = insert(&db, a_content)?;
    platform.set_resource(id, EntryKind::Blob, "b".into(), ResourceKind::NewOrDestination, &db)?;

    let out = platform.prepare_diff()?;
    assert!(
        matches!(
            out.operation,
            Operation::InternalDiff {
                algorithm: Algorithm::Myers
            }
        ),
        "by default, we prepare for internal diffs, unless external commands are enabled.\
         The caller could still obtain the command from here if they wanted to, as well.\
         Also, the algorithm is overridden by the source."
    );
    Ok(())
}

#[test]
fn diff_skipped_due_to_external_command_and_enabled_option() -> Result {
    let command: BString = "something-to-be-ignored".into();
    let mut platform = new_platform(
        Some(gix_diff::blob::Driver {
            name: "a".into(),
            command: Some(command.clone()),
            algorithm: Some(Algorithm::Myers),
            ..Default::default()
        }),
        gix_diff::blob::pipeline::Mode::default(),
    );
    platform.options.skip_internal_diff_if_external_is_configured = true;

    platform.set_resource(
        gix_hash::Kind::Sha1.null(),
        EntryKind::Blob,
        "a".into(),
        ResourceKind::OldOrSource,
        &gix_object::find::Never,
    )?;

    let db = object_db();
    let a_content = "b";
    let id = insert(&db, a_content)?;
    platform.set_resource(id, EntryKind::Blob, "b".into(), ResourceKind::NewOrDestination, &db)?;

    let out = platform.prepare_diff()?;
    assert_eq!(
        out.operation,
        Operation::ExternalCommand {
            command: command.as_ref()
        },
        "now we provide all information that is needed to run the overridden diff command"
    );
    Ok(())
}

#[test]
fn source_and_destination_do_not_exist() -> Result {
    let mut platform = new_platform(None, pipeline::Mode::default());
    let err = platform.prepare_diff().expect_err("neither resource has been set");
    assert!(
        matches!(err.error(), platform::prepare_diff::Error::SourceOrDestinationUnset),
        "unset resources have a distinct recovery variant"
    );
    assert!(err.is_validation(), "unset resources are invalid input");
    platform.set_resource(
        gix_hash::Kind::Sha1.null(),
        EntryKind::Blob,
        "missing".into(),
        ResourceKind::OldOrSource,
        &gix_object::find::Never,
    )?;

    platform.set_resource(
        gix_hash::Kind::Sha1.null(),
        EntryKind::BlobExecutable,
        "a".into(),
        ResourceKind::NewOrDestination,
        &gix_object::find::Never,
    )?;

    let (old, new) = platform.resources().expect("previously set source and destination");
    assert_eq!(old.data, platform::resource::Data::Missing);
    assert_eq!(old.driver_index, None);
    assert_eq!(old.mode, EntryKind::Blob);
    assert_eq!(new.data, platform::resource::Data::Missing);
    assert_eq!(new.driver_index, None);
    assert_eq!(new.mode, EntryKind::BlobExecutable);

    let err = platform.prepare_diff().expect_err("both resources are missing");
    assert!(
        matches!(err.error(), platform::prepare_diff::Error::SourceAndDestinationRemoved),
        "removed resources are distinguishable from resources that were never set"
    );
    assert!(err.is_validation(), "two removed resources are invalid input");
    assert!(
        err.classify()
            .next()
            .expect("validation classification")
            .error()
            .is::<platform::prepare_diff::Error>(),
        "the constant marker classifies the preparation error"
    );
    insta::assert_debug_snapshot!(err, "source and destination do not exist", @"Tried to diff resources that are both considered removed");

    assert_eq!(
        format!(
            "{:?}",
            *platform
                .prepare_diff_command(
                    "test".into(),
                    gix_diff::command::Context {
                        git_dir: Some(".".into()),
                        ..Default::default()
                    },
                    0,
                    1
                )
                .expect("resources set")
        ),
        format!(
            r#"{}"test" "missing" "/dev/null" "." "." "/dev/null" "." "." "a""#,
            if !cfg!(windows) {
                r#"GIT_DIFF_PATH_COUNTER="1" GIT_DIFF_PATH_TOTAL="1" GIT_DIR="." "#
            } else {
                Default::default()
            }
        )
    );
    Ok(())
}

#[test]
fn invalid_resource_types() -> Result {
    let mut error_snapshots = Vec::new();
    let mut platform = new_platform(None, pipeline::Mode::default());
    for mode in [EntryKind::Commit, EntryKind::Tree] {
        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "a".into(),
            ResourceKind::NewOrDestination,
            &gix_object::find::Never,
        )?;
        let err = platform
            .set_resource(
                gix_hash::Kind::Sha1.null(),
                mode,
                "a".into(),
                ResourceKind::NewOrDestination,
                &gix_object::find::Never,
            )
            .expect_err("trees and commits are not diffable resources");
        assert!(
            matches!(err.error(), platform::set_resource::Error::InvalidMode { mode: actual } if *actual == mode),
            "the unsupported mode is available for recovery"
        );
        assert!(err.is_validation(), "invalid modes are classified as invalid input");
        assert!(
            platform.resource(ResourceKind::NewOrDestination).is_none(),
            "an invalid mode clears the previously set resource"
        );
        error_snapshots.push(gix_testtools::redact_debug_snapshot(&err, &[]));
    }
    insta::assert_debug_snapshot!(error_snapshots, "invalid resource types", @"
    [
        Can only diff blobs and links, not Commit,
        Can only diff blobs and links, not Tree,
    ]
    ");
    let err = platform
        .set_resource_by_change(
            gix_diff::tree_with_rewrites::ChangeRef::Addition {
                location: "a".into(),
                entry_mode: EntryKind::Tree.into(),
                relation: None,
                id: gix_hash::Kind::Sha1.null(),
            },
            &gix_object::find::Never,
        )
        .err()
        .expect("tree changes cannot be diffed as blobs");
    assert!(
        matches!(
            err.error(),
            platform::set_resource::Error::InvalidMode { mode: EntryKind::Tree }
        ),
        "setting resources by change shares the same typed recovery contract"
    );
    Ok(())
}

#[test]
fn resource_setup_errors_retain_paths_causes_and_clear_the_failed_side() -> Result {
    use platform::set_resource::Error;

    let mut platform = new_platform(None, pipeline::Mode::default());
    platform.filter.roots.old_root = None;
    let missing_blob_id = gix_testtools::object_hash().empty_blob();
    for kind in [ResourceKind::OldOrSource, ResourceKind::NewOrDestination] {
        for rela_path in [
            b"../invalid-\xff".as_bstr(),
            b"missing".as_bstr(),
            // Only Unix can convert a non-UTF-8 Git path for attribute lookup.
            #[cfg(unix)]
            b"missing-\xff".as_bstr(),
        ] {
            for side in [ResourceKind::OldOrSource, ResourceKind::NewOrDestination] {
                platform.set_resource(
                    gix_testtools::object_hash().null(),
                    EntryKind::Blob,
                    "a".into(),
                    side,
                    &gix_object::find::Never,
                )?;
            }
            let err = platform
                .set_resource(
                    missing_blob_id,
                    EntryKind::Blob,
                    rela_path,
                    kind,
                    &gix_object::find::Never,
                )
                .expect_err("the path is invalid or the object is absent");
            match err.error() {
                Error::Attributes {
                    kind: actual,
                    rela_path: path,
                } => {
                    assert_eq!(*actual, kind, "the attribute failure retains the resource side");
                    assert_eq!(path, rela_path, "the invalid path retains its original bytes");
                    assert!(
                        rela_path.starts_with(b"../"),
                        "only the invalid path fails attribute setup"
                    );
                    assert!(
                        err.downcast_any_ref::<std::io::Error>().is_some(),
                        "the attribute failure retains its underlying I/O error"
                    );
                }
                Error::ConvertToDiffable {
                    kind: actual,
                    rela_path: path,
                } => {
                    assert_eq!(*actual, kind, "the conversion failure retains the resource side");
                    assert_eq!(path, rela_path, "the resource path retains its original bytes");
                    assert!(!rela_path.starts_with(b"../"), "a valid path reaches conversion");
                    assert!(err.is_not_found(), "the missing object's classification is retained");
                    assert!(
                        err.downcast_any_ref::<gix_error::Message>().is_some(),
                        "the conversion failure retains the callee's diagnostic"
                    );
                }
                other => panic!("unexpected resource setup error: {other:?}"),
            }
            assert!(platform.resource(kind).is_none(), "failed setup clears the resource");
            assert!(
                matches!(
                    platform.prepare_diff().expect_err("a resource was cleared").error(),
                    platform::prepare_diff::Error::SourceOrDestinationUnset
                ),
                "a failed setup cannot leave stale resources available for diffing"
            );
        }
    }
    Ok(())
}

fn new_platform(
    drivers: impl IntoIterator<Item = gix_diff::blob::Driver>,
    mode: gix_diff::blob::pipeline::Mode,
) -> Platform {
    let root = crate::scripted_fixture_read_only("make_blob_repo.sh").expect("valid fixture");
    let attributes = crate::blob::new_attributes_stack(&root);
    let filter = gix_diff::blob::Pipeline::new(
        pipeline::WorktreeRoots {
            old_root: Some(root.clone()),
            new_root: None,
        },
        gix_filter::Pipeline::default(),
        drivers.into_iter().collect(),
        default_options(),
    );
    Platform::new(Default::default(), filter, mode, attributes)
}
