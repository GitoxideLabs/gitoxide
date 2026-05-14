use gix_diff::blob::{
    Algorithm, Platform, ResourceKind, pipeline, platform,
    platform::{prepare_diff, prepare_diff::Operation},
};
use gix_object::{
    bstr::{BString, ByteSlice},
    tree::EntryKind,
};
use gix_worktree::stack::state::attributes;

use crate::{blob::pipeline::convert_to_diffable::default_options, hex_to_id, util::ObjectDb};

#[test]
fn resources_of_worktree_and_odb_and_check_link() -> crate::Result {
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

    let mut db = ObjectDb::default();
    let a_content = "a-content";
    let id = db.insert(a_content)?;
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

fn comparable_ext_diff(
    cmd: Result<
        gix_diff::blob::platform::prepare_diff_command::Command,
        gix_diff::blob::platform::prepare_diff_command::Error,
    >,
) -> String {
    let cmd = cmd.expect("no error");
    let tokens = shell_words::split(&format!("{:?}", *cmd)).expect("parses fine");
    let ofs = if cfg!(windows) { 3 } else { 0 }; // Windows doesn't show env vars
    tokens
        .into_iter()
        .enumerate()
        .filter_map(|(idx, s)| {
            (idx != (5 - ofs) && idx != (8 - ofs))
                .then_some(s)
                .or_else(|| Some("<tmp-path>".into()))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn diff_binary() -> crate::Result {
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

    let mut db = ObjectDb::default();
    let a_content = "b";
    let id = db.insert(a_content)?;
    platform.set_resource(id, EntryKind::Blob, "b".into(), ResourceKind::NewOrDestination, &db)?;

    let out = platform.prepare_diff()?;
    assert!(
        matches!(out.operation, Operation::SourceOrDestinationIsBinary),
        "one binary resource is enough to skip diffing entirely"
    );

    match platform.prepare_diff_command("test".into(), Default::default(), 0, 1) {
        Err(err) => assert_eq!(
            err.to_string(),
            "Binary resources can't be diffed with an external command (as we don't have the data anymore)"
        ),
        Ok(_) => unreachable!("must error"),
    }

    Ok(())
}

#[test]
fn diff_performed_despite_external_command() -> crate::Result {
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

    let mut db = ObjectDb::default();
    let a_content = "b";
    let id = db.insert(a_content)?;
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
fn diff_skipped_due_to_external_command_and_enabled_option() -> crate::Result {
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

    let mut db = ObjectDb::default();
    let a_content = "b";
    let id = db.insert(a_content)?;
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
fn source_and_destination_do_not_exist() -> crate::Result {
    let mut platform = new_platform(None, pipeline::Mode::default());
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

    assert!(matches!(
        platform.prepare_diff(),
        Err(prepare_diff::Error::SourceAndDestinationRemoved)
    ));

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
fn invalid_resource_types() {
    let mut platform = new_platform(None, pipeline::Mode::default());
    for (mode, name) in [(EntryKind::Commit, "Commit"), (EntryKind::Tree, "Tree")] {
        assert_eq!(
            platform
                .set_resource(
                    gix_hash::Kind::Sha1.null(),
                    mode,
                    "a".into(),
                    ResourceKind::NewOrDestination,
                    &gix_object::find::Never,
                )
                .unwrap_err()
                .to_string(),
            format!("Can only diff blobs and links, not {name}")
        );
    }
}

fn new_platform(
    drivers: impl IntoIterator<Item = gix_diff::blob::Driver>,
    mode: gix_diff::blob::pipeline::Mode,
) -> Platform {
    let root = crate::scripted_fixture_read_only("make_blob_repo.sh").expect("valid fixture");
    let attributes = gix_worktree::Stack::new(
        &root,
        gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::new(
            Default::default(),
            None,
            attributes::Source::WorktreeThenIdMapping,
            Default::default(),
        )),
        gix_worktree::glob::pattern::Case::Sensitive,
        Vec::new(),
        Vec::new(),
    );
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
