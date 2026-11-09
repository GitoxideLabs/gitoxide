use std::process::Command;

use gix_diff::blob::{
    Algorithm, Platform, ResourceKind, pipeline, platform,
    platform::prepare_diff::{Operation, TextChange},
};
use gix_object::{
    Write,
    bstr::{BString, ByteSlice},
    tree::EntryKind,
};
use gix_testtools::TestResult;

use crate::{
    blob::pipeline::convert_to_diffable::default_options,
    hex_to_id,
    util::{insert, object_db},
};

#[test]
fn byte_lines_without_terminator() -> TestResult {
    let mut platform = new_platform(
        Some(gix_diff::blob::Driver {
            name: "a".into(),
            is_binary: Some(false),
            ..Default::default()
        }),
        pipeline::Mode::default(),
    );
    platform.filter.roots.old_root = None;
    let db = object_db();
    let cases: &[(&[u8], &[&[u8]])] = &[
        (b"", &[]),
        (b"\n", &[b""]),
        (b"\n\n", &[b"", b""]),
        (b"a", &[b"a"]),
        (b"a\n", &[b"a"]),
        (b"a\r\nb\nc\r", &[b"a", b"b", b"c\r"]),
        (b"\r\n", &[b""]),
        (b"\r\r\n", &[b"\r"]),
        (b"\xff\0\n\x80", &[b"\xff\0", b"\x80"]),
    ];
    for &(input, expected) in cases {
        let mut lines = platform::resource::ByteLinesWithoutTerminator::new(input);
        assert_eq!(
            lines.by_ref().collect::<Vec<_>>(),
            expected,
            "only LF and its immediately preceding CR are stripped from {input:?}"
        );
        assert_eq!(lines.next(), None, "the iterator remains exhausted");
        assert_eq!(lines.next(), None, "repeated calls remain exhausted");

        let blob_id = db.write_buf(gix_object::Kind::Blob, input)?;
        for kind in [ResourceKind::OldOrSource, ResourceKind::NewOrDestination] {
            platform.set_resource(Some(blob_id), EntryKind::Blob, "a".into(), kind, &db)?;
        }
        let out = platform.prepare_diff()?;
        let Operation::InternalDiff { change, .. } = out.operation else {
            panic!("explicitly forced text is diffable: {:?}", out.operation);
        };
        assert_eq!(
            change.patch_data(),
            (input, input),
            "patch bytes retain line terminators"
        );
        let input = change.patch_input();
        for tokens in [&input.before, &input.after] {
            assert_eq!(
                tokens.iter().map(|&token| input.interner[token]).collect::<Vec<_>>(),
                expected,
                "validated patch input strips only LF and its immediately preceding CR"
            );
        }
    }
    Ok(())
}

#[test]
fn missing_resources_keep_their_own_attributes() -> TestResult {
    let mut platform = new_platform(
        [gix_diff::blob::Driver {
            name: "a".into(),
            ..Default::default()
        }],
        pipeline::Mode::default(),
    );
    platform.filter.roots.old_root = None;
    for (path, kind) in [("a", ResourceKind::OldOrSource), ("b", ResourceKind::NewOrDestination)] {
        platform.set_resource(None, EntryKind::Blob, path.into(), kind, &gix_object::find::Never)?;
    }
    let (old, new) = platform.resources().expect("both resources were set");
    assert_eq!(old.data, platform::resource::Data::Missing);
    assert_eq!(new.data, platform::resource::Data::Missing);
    assert_eq!(
        old.driver_index,
        Some(0),
        "attributes belong to the missing resource's path"
    );
    assert_eq!(
        new.driver_index, None,
        "null IDs do not identify a shared attribute cache entry"
    );
    for path in ["c", "d", "e"] {
        platform.set_resource(
            None,
            EntryKind::Blob,
            path.into(),
            ResourceKind::NewOrDestination,
            &gix_object::find::Never,
        )?;
    }
    assert_eq!(
        platform.clear_resource_cache_keep_allocation(),
        1,
        "absent paths reuse one scratch buffer instead of accumulating cache entries"
    );

    Ok(())
}

#[test]
fn cached_resources_preserve_the_supplied_mode() -> TestResult {
    let mut platform = new_platform(None, pipeline::Mode::default());
    platform.filter.roots.old_root = None;
    let objects = object_db();
    let blob_id = insert(&objects, "content")?;
    for blob_id in [None, Some(gix_testtools::object_hash().null()), Some(blob_id)] {
        for (mode, kind) in [
            (EntryKind::Blob, ResourceKind::OldOrSource),
            (EntryKind::BlobExecutable, ResourceKind::NewOrDestination),
        ] {
            platform.set_resource(blob_id, mode, "a".into(), kind, &objects)?;
        }
        let (old, new) = platform.resources().expect("both resources were set");
        assert_eq!(old.id, blob_id.as_deref().filter(|id| !id.is_null()));
        assert_eq!(new.id, old.id);
        assert_eq!(old.mode, EntryKind::Blob);
        assert_eq!(
            new.mode,
            EntryKind::BlobExecutable,
            "a shared content cache must not replace the supplied mode"
        );
    }
    Ok(())
}

#[test]
fn worktree_resources_at_the_same_path_have_distinct_roots() -> TestResult {
    let mut platform = new_platform(None, pipeline::Mode::default());
    platform.filter.roots.new_root = platform.filter.roots.old_root.as_ref().map(|root| root.join(".git"));
    for (blob_id, kind) in [
        (None, ResourceKind::OldOrSource),
        (
            Some(gix_testtools::object_hash().null()),
            ResourceKind::NewOrDestination,
        ),
    ] {
        platform.set_resource(
            blob_id,
            EntryKind::Blob,
            "config".into(),
            kind,
            &gix_object::find::Never,
        )?;
    }
    let (old, new) = platform.resources().expect("both resources were set");
    assert_eq!(
        (old.id, new.id),
        (None, None),
        "unknown IDs are normalized independently of content presence"
    );
    assert_eq!(
        old.data,
        platform::resource::Data::Missing,
        "config is absent from the worktree root"
    );
    assert!(
        new.data.as_slice().is_some_and(|data| !data.is_empty()),
        "the fixture's .git directory contains config"
    );
    platform.clear_resource_cache();
    platform.filter.roots.new_root = platform.filter.roots.old_root.clone();
    for (blob_id, kind) in [
        (None, ResourceKind::OldOrSource),
        (
            Some(gix_testtools::object_hash().null()),
            ResourceKind::NewOrDestination,
        ),
    ] {
        platform.set_resource(blob_id, EntryKind::Blob, "a".into(), kind, &gix_object::find::Never)?;
    }
    assert_eq!(
        platform.clear_resource_cache_keep_allocation(),
        1,
        "equal worktree roots still share their cached content"
    );

    Ok(())
}

#[test]
fn resources_of_worktree_and_odb_and_check_link() -> TestResult {
    let mut platform = new_platform(
        Some(gix_diff::blob::Driver {
            name: "a".into(),
            ..Default::default()
        }),
        gix_diff::blob::pipeline::Mode::default(),
    );
    platform.set_resource(
        None,
        EntryKind::Blob,
        "a".into(),
        ResourceKind::OldOrSource,
        &gix_object::find::Never,
    )?;

    let db = object_db();
    let a_content = "a-content";
    let id = insert(&db, a_content)?;
    platform.set_resource(
        Some(id),
        EntryKind::BlobExecutable,
        "a".into(),
        ResourceKind::NewOrDestination,
        &db,
    )?;

    let (old, new) = platform.resources().expect("previously set source and destination");
    assert_eq!(old.data.as_slice().expect("present").as_bstr(), "a\n");
    assert_eq!(old.driver_index, Some(0));
    assert_eq!(old.mode, EntryKind::Blob);
    assert!(old.id.is_none(), "an unavailable ID is exposed as None");
    assert_eq!(old.rela_path, "a", "location is kept directly as provided");
    assert_eq!(new.data.as_slice().expect("present").as_bstr(), a_content);
    assert_eq!(new.driver_index, Some(0));
    assert_eq!(new.mode, EntryKind::BlobExecutable);
    let new_id = hex_to_id(
        "4c469b6c8c4486fdc9ded9d597d8f6816a455707",
        "061557fb6e67e1be5d41f8e83962699fce2a7d168ccfb2b5743d27ab06038da8",
    );
    assert_eq!(new.id, Some(new_id.as_ref()));
    assert_eq!(new.rela_path, "a", "location is kept directly as provided");

    let out = platform.prepare_diff()?;
    assert!(
        matches!(
            out.operation,
            Operation::InternalDiff {
                algorithm: Algorithm::Histogram,
                change: TextChange::Modification(_)
            }
        ),
        "it ends up with the default, as it's not overridden anywhere"
    );

    let null_id = gix_testtools::object_hash().null();
    let program = Command::try_from(gix_diff::command::prepare("test"))?;
    let program = program.get_program().to_string_lossy();
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
            "{}{program} a <tmp-path> {null_id} 100644 <tmp-path> {new_id} 100755",
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

    platform.set_resource(
        Some(id),
        EntryKind::Link,
        "a".into(),
        ResourceKind::NewOrDestination,
        &db,
    )?;

    // Double-inserts are fine.
    platform.set_resource(
        Some(id),
        EntryKind::Link,
        "a".into(),
        ResourceKind::NewOrDestination,
        &db,
    )?;
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
    assert_eq!(new.id, Some(new_id.as_ref()));
    assert_eq!(new.rela_path, "a");

    let out = platform.prepare_diff()?;
    assert!(
        matches!(
            out.operation,
            Operation::InternalDiff {
                algorithm: Algorithm::Histogram,
                change: TextChange::Modification(_)
            }
        ),
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
            "{}{program} a <tmp-path> {null_id} 100644 <tmp-path> {new_id} 120000",
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
fn diff_binary() -> TestResult {
    let mut platform = new_platform(
        Some(gix_diff::blob::Driver {
            name: "a".into(),
            is_binary: Some(true),
            ..Default::default()
        }),
        gix_diff::blob::pipeline::Mode::default(),
    );
    platform.set_resource(
        None,
        EntryKind::Blob,
        "a".into(),
        ResourceKind::OldOrSource,
        &gix_object::find::Never,
    )?;

    let db = object_db();
    let a_content = "b";
    let id = insert(&db, a_content)?;
    platform.set_resource(
        Some(id),
        EntryKind::Blob,
        "b".into(),
        ResourceKind::NewOrDestination,
        &db,
    )?;

    let out = platform.prepare_diff()?;
    assert!(
        matches!(out.operation, Operation::SourceOrDestinationIsBinary),
        "one binary resource is enough to skip diffing entirely"
    );

    match platform.prepare_diff_command("test".into(), Default::default(), 0, 1) {
        Err(err) => {
            insta::assert_debug_snapshot!(err, "diff binary", @"Binary resources can't be diffed with an external command (as we don't have the data anymore)");
            assert!(
                err.is_validation(),
                "external diff requires resources with available data"
            );
        }
        Ok(_) => unreachable!("must error"),
    }

    Ok(())
}

#[test]
fn diff_performed_despite_external_command() -> TestResult {
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
        None,
        EntryKind::Blob,
        "a".into(),
        ResourceKind::OldOrSource,
        &gix_object::find::Never,
    )?;

    let db = object_db();
    let a_content = "b";
    let id = insert(&db, a_content)?;
    platform.set_resource(
        Some(id),
        EntryKind::Blob,
        "b".into(),
        ResourceKind::NewOrDestination,
        &db,
    )?;

    let out = platform.prepare_diff()?;
    assert!(
        matches!(
            out.operation,
            Operation::InternalDiff {
                algorithm: Algorithm::Myers,
                change: TextChange::Modification(_)
            }
        ),
        "by default, we prepare for internal diffs, unless external commands are enabled.\
         The caller could still obtain the command from here if they wanted to, as well.\
         Also, the algorithm is overridden by the source."
    );
    Ok(())
}

#[test]
fn diff_skipped_due_to_external_command_and_enabled_option() -> TestResult {
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
        None,
        EntryKind::Blob,
        "a".into(),
        ResourceKind::OldOrSource,
        &gix_object::find::Never,
    )?;

    let db = object_db();
    let a_content = "b";
    let id = insert(&db, a_content)?;
    platform.set_resource(
        Some(id),
        EntryKind::Blob,
        "b".into(),
        ResourceKind::NewOrDestination,
        &db,
    )?;

    let out = platform.prepare_diff()?;
    let Operation::ExternalCommand {
        command: actual_command,
        change,
    } = out.operation
    else {
        panic!(
            "an enabled external command receives validated text: {:?}",
            out.operation
        );
    };
    assert_eq!(
        actual_command, command,
        "now we provide all information that is needed to run the overridden diff command"
    );
    assert!(
        matches!(change, TextChange::Modification(_)),
        "both resources exist and are text"
    );
    assert_eq!(
        change.patch_data(),
        (b"a\n".as_slice(), a_content.as_bytes()),
        "external commands carry the same validated bytes as internal diffs"
    );
    Ok(())
}

#[test]
fn source_and_destination_do_not_exist() -> TestResult {
    let mut platform = new_platform(None, pipeline::Mode::default());
    let err = platform.prepare_diff().expect_err("neither resource has been set");
    assert!(
        matches!(err.error(), platform::prepare_diff::Error::SourceOrDestinationUnset),
        "unset resources have a distinct recovery variant"
    );
    assert!(err.is_validation(), "unset resources are invalid input");
    assert!(
        err.classify()
            .next()
            .expect("validation classification")
            .error()
            .is::<platform::prepare_diff::Error>(),
        "the constant marker classifies the preparation error"
    );
    platform.set_resource(
        None,
        EntryKind::Blob,
        "missing".into(),
        ResourceKind::OldOrSource,
        &gix_object::find::Never,
    )?;

    platform.set_resource(
        None,
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

    assert_eq!(
        platform.prepare_diff()?.operation,
        Operation::NeitherExists,
        "removed resources are a successful no-op, unlike resources that were never set"
    );

    let program = Command::try_from(gix_diff::command::prepare("test"))?;
    assert_eq!(
        format!(
            "{:?}",
            *platform.prepare_diff_command(
                "test".into(),
                gix_diff::command::Context {
                    git_dir: Some(".".into()),
                    ..Default::default()
                },
                0,
                1
            )?
        ),
        format!(
            r#"{}{program:?} "missing" "/dev/null" "." "." "/dev/null" "." "." "a""#,
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
fn validated_text_changes_cover_resource_states() -> TestResult {
    use platform::resource::Data;

    let db = object_db();
    let states = [
        ("missing", None, "a", Data::Missing),
        (
            "missing with null ID",
            Some(gix_testtools::object_hash().null()),
            "a",
            Data::Missing,
        ),
        (
            "empty",
            Some(insert(&db, "")?),
            "a",
            Data::Buffer {
                buf: b"",
                is_derived: false,
            },
        ),
        (
            "text",
            Some(insert(&db, "text\n")?),
            "a",
            Data::Buffer {
                buf: b"text\n",
                is_derived: false,
            },
        ),
        ("binary", Some(insert(&db, "binary\0")?), "a", Data::Binary { size: 7 }),
        (
            "derived text",
            Some(insert(&db, "text\0\n")?),
            "b",
            Data::Buffer {
                buf: b"text\n",
                is_derived: true,
            },
        ),
        (
            "derived empty",
            Some(insert(&db, "\0")?),
            "b",
            Data::Buffer {
                buf: b"",
                is_derived: true,
            },
        ),
    ];
    for external in [false, true] {
        let command = "external-diff-not-executed";
        let mut platform = new_platform(
            [
                gix_diff::blob::Driver {
                    name: "a".into(),
                    command: Some(command.into()),
                    algorithm: Some(Algorithm::Myers),
                    ..Default::default()
                },
                gix_diff::blob::Driver {
                    name: "b".into(),
                    command: Some(command.into()),
                    algorithm: Some(Algorithm::Myers),
                    is_binary: Some(true),
                    binary_to_text_command: Some("tr -d '\\000' <".into()),
                },
            ],
            pipeline::Mode::ToWorktreeAndBinaryToText,
        );
        platform.filter.roots.old_root = None;
        platform.options.skip_internal_diff_if_external_is_configured = external;
        for &(old_name, old_blob_id, old_path, old_data) in &states {
            for &(new_name, new_blob_id, new_path, new_data) in &states {
                let case = format!("{old_name} -> {new_name}, external={external}");
                platform.set_resource(
                    old_blob_id,
                    EntryKind::Blob,
                    old_path.into(),
                    ResourceKind::OldOrSource,
                    &db,
                )?;
                platform.set_resource(
                    new_blob_id,
                    EntryKind::Blob,
                    new_path.into(),
                    ResourceKind::NewOrDestination,
                    &db,
                )?;
                let out = platform.prepare_diff()?;
                assert_eq!(out.old.data, old_data, "source state: {case}");
                assert_eq!(out.new.data, new_data, "destination state: {case}");
                assert_eq!(
                    out.old_or_new_is_derived,
                    old_data.is_derived() || new_data.is_derived(),
                    "conversion provenance includes either resource: {case}"
                );
                if matches!((old_data, new_data), (Data::Missing, Data::Missing)) {
                    assert_eq!(out.operation, Operation::NeitherExists, "no text is fabricated: {case}");
                    continue;
                }
                if matches!(old_data, Data::Binary { .. }) || matches!(new_data, Data::Binary { .. }) {
                    assert_eq!(
                        out.operation,
                        Operation::SourceOrDestinationIsBinary,
                        "binary resources never yield validated text, even with an external command: {case}"
                    );
                    continue;
                }
                let change = match out.operation {
                    Operation::InternalDiff { algorithm, change } => {
                        assert!(!external, "configured external commands take precedence: {case}");
                        assert_eq!(
                            algorithm,
                            Algorithm::Myers,
                            "source driver selects the algorithm: {case}"
                        );
                        change
                    }
                    Operation::ExternalCommand {
                        command: actual,
                        change,
                    } => {
                        assert!(external, "external diff must be enabled explicitly: {case}");
                        assert_eq!(actual, command, "source driver supplies the command: {case}");
                        change
                    }
                    other => panic!("expected validated text for {case}, got {other:?}"),
                };
                let (old, new) = match change {
                    TextChange::Addition { new } => (None, Some(new)),
                    TextChange::Deletion { old } => (Some(old), None),
                    TextChange::Modification(pair) => (Some(pair.before()), Some(pair.after())),
                };
                for (text, data) in [(old, old_data), (new, new_data)] {
                    assert_eq!(
                        text.map(|text| (text.as_slice(), text.is_derived())),
                        data.as_slice().map(|buf| (buf, data.is_derived())),
                        "only missing sides are absent; empty buffers retain their provenance: {case}"
                    );
                }
                assert_eq!(
                    change.patch_data(),
                    (
                        old_data.as_slice().unwrap_or_default(),
                        new_data.as_slice().unwrap_or_default()
                    ),
                    "only patch rendering maps absent sides to empty bytes: {case}"
                );
                let input = change.patch_input();
                for (tokens, data) in [(&input.before, old_data), (&input.after, new_data)] {
                    assert_eq!(
                        tokens.iter().map(|&token| input.interner[token]).collect::<Vec<_>>(),
                        data.as_slice().unwrap_or_default().lines().collect::<Vec<_>>(),
                        "patch input uses validated text and strips line terminators: {case}"
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn one_unset_resource_is_not_a_missing_resource() -> TestResult {
    let db = object_db();
    let mut platform = new_platform(None, pipeline::Mode::default());
    platform.filter.roots.old_root = None;
    for kind in [ResourceKind::OldOrSource, ResourceKind::NewOrDestination] {
        for blob_id in [
            None,
            Some(gix_testtools::object_hash().null()),
            Some(insert(&db, "")?),
            Some(insert(&db, "text\n")?),
            Some(insert(&db, "binary\0")?),
        ] {
            platform.clear_resource_cache();
            platform.set_resource(blob_id, EntryKind::Blob, "a".into(), kind, &db)?;
            let err = platform.prepare_diff().expect_err("one resource was never set");
            assert!(
                matches!(err.error(), platform::prepare_diff::Error::SourceOrDestinationUnset),
                "an unset side is an error even when the other side is missing, empty, text, or binary: {kind:?}"
            );
            assert!(err.is_validation(), "unset resources remain invalid input");
        }
    }
    Ok(())
}

#[test]
fn invalid_resource_types() -> TestResult {
    let mut error_snapshots = Vec::new();
    let mut platform = new_platform(None, pipeline::Mode::default());
    for mode in [EntryKind::Commit, EntryKind::Tree] {
        platform.set_resource(
            None,
            EntryKind::Blob,
            "a".into(),
            ResourceKind::NewOrDestination,
            &gix_object::find::Never,
        )?;
        let err = platform
            .set_resource(
                None,
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
fn resource_setup_errors_retain_paths_causes_and_clear_the_failed_side() -> TestResult {
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
                platform.set_resource(None, EntryKind::Blob, "a".into(), side, &gix_object::find::Never)?;
            }
            let err = platform
                .set_resource(
                    Some(missing_blob_id),
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
                    assert_eq!(
                        err.iter_errors().count(),
                        2,
                        "only the resource context and the original missing-object diagnostic remain"
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
        gix_filter::Pipeline::new(Default::default(), gix_testtools::object_hash(), Default::default()),
        drivers.into_iter().collect(),
        default_options(),
    );
    Platform::new(Default::default(), filter, mode, attributes)
}
