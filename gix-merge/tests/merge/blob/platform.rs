use gix_worktree::stack::state::attributes;

use gix_merge::blob::Platform;

mod merge {
    use crate::blob::platform::new_platform;
    use crate::blob::util::ObjectDb;
    use bstr::{BStr, ByteSlice};
    use gix_merge::blob::builtin_driver::text::ConflictStyle;
    use gix_merge::blob::platform::builtin_merge::Pick;
    use gix_merge::blob::platform::DriverChoice;
    use gix_merge::blob::{builtin_driver, pipeline, platform, BuiltinDriver, Resolution, ResourceKind};
    use gix_object::tree::EntryKind;
    use std::process::Stdio;

    #[test]
    fn builtin_text_uses_binary_if_needed() -> crate::Result {
        let mut platform = new_platform(None, pipeline::Mode::ToGit);
        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "a".into(),
            ResourceKind::CommonAncestorOrBase,
            &gix_object::find::Never,
        )?;

        let mut db = ObjectDb::default();
        for (content, kind) in [
            ("ours", ResourceKind::CurrentOrOurs),
            ("theirs\0", ResourceKind::OtherOrTheirs),
        ] {
            let id = db.insert(content);
            platform.set_resource(
                id,
                EntryKind::Blob,
                "path matters only for attribute lookup".into(),
                kind,
                &db,
            )?;
        }
        let mut platform_ref = platform.prepare_merge(&db, Default::default())?;
        assert_eq!(
            platform_ref.driver,
            DriverChoice::BuiltIn(BuiltinDriver::Text),
            "it starts out at the default text driver"
        );

        let mut buf = Vec::new();
        let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
        assert_eq!(
            res,
            (Pick::Ours, Resolution::Conflict),
            "it detected the binary buffer, ran the binary merge with default conflict resolution"
        );

        platform_ref.options.resolve_binary_with = Some(builtin_driver::binary::ResolveWith::Theirs);
        let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
        assert_eq!(
            res,
            (Pick::Theirs, Resolution::Complete),
            "the auto-binary driver respects its own options"
        );
        Ok(())
    }

    #[test]
    fn builtin_with_conflict() -> crate::Result {
        let mut platform = new_platform(None, pipeline::Mode::ToGit);
        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "b".into(),
            ResourceKind::CommonAncestorOrBase,
            &gix_object::find::Never,
        )?;

        let mut db = ObjectDb::default();
        for (content, kind) in [
            ("ours", ResourceKind::CurrentOrOurs),
            ("theirs", ResourceKind::OtherOrTheirs),
        ] {
            let id = db.insert(content);
            platform.set_resource(id, EntryKind::Blob, "b".into(), kind, &db)?;
        }

        let mut platform_ref = platform.prepare_merge(&db, Default::default())?;
        assert_eq!(platform_ref.driver, DriverChoice::BuiltIn(BuiltinDriver::Text));
        let mut buf = Vec::new();
        let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
        assert_eq!(res, (Pick::Buffer, Resolution::Conflict));
        assert_eq!(
            buf.as_bstr(),
            r#"<<<<<<< current label
ours
=======
theirs
>>>>>>> other label
"#,
            "default options apply, hence the 'merge' style conflict"
        );
        platform_ref.options.text.conflict = builtin_driver::text::Conflict::Keep {
            style: ConflictStyle::Diff3,
            marker_size: 3,
        };
        let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
        assert_eq!(res, (Pick::Buffer, Resolution::Conflict));

        assert_eq!(
            buf.as_bstr(),
            r#"<<< current label
ours
||| ancestor label
b
===
theirs
>>> other label
"#,
            "options apply correctly"
        );

        platform_ref.options.text.conflict = builtin_driver::text::Conflict::ResolveWithOurs;
        let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
        assert_eq!(
            res,
            (Pick::Buffer, Resolution::Complete),
            "it's actually unclear now if there ever was a conflict, but we *could* compute it"
        );
        assert_eq!(buf.as_bstr(), "ours");

        platform_ref.options.text.conflict = builtin_driver::text::Conflict::ResolveWithTheirs;
        let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
        assert_eq!(res, (Pick::Buffer, Resolution::Complete));
        assert_eq!(buf.as_bstr(), "theirs");

        platform_ref.options.text.conflict = builtin_driver::text::Conflict::ResolveWithUnion;
        let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
        assert_eq!(res, (Pick::Buffer, Resolution::Complete));
        assert_eq!(buf.as_bstr(), "ours\ntheirs");

        platform_ref.driver = DriverChoice::BuiltIn(BuiltinDriver::Union);
        platform_ref.options.text.conflict = builtin_driver::text::Conflict::default();
        let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
        assert_eq!(res, (Pick::Buffer, Resolution::Complete));
        assert_eq!(buf.as_bstr(), "ours\ntheirs");

        platform_ref.driver = DriverChoice::BuiltIn(BuiltinDriver::Binary);
        let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
        assert_eq!(
            res,
            (Pick::Ours, Resolution::Conflict),
            "binary merges choose ours but conflict by default"
        );
        assert!(buf.is_empty(), "it tells us where to get the content from");
        assert_eq!(
            platform_ref.buffer_by_pick(res.0).unwrap().as_bstr(),
            "ours",
            "getting access to the content is simplified"
        );

        for (expected, expected_pick, resolve) in [
            ("ours", Pick::Ours, builtin_driver::binary::ResolveWith::Ours),
            ("theirs", Pick::Theirs, builtin_driver::binary::ResolveWith::Theirs),
            ("b\n", Pick::Ancestor, builtin_driver::binary::ResolveWith::Ancestor),
        ] {
            platform_ref.options.resolve_binary_with = Some(resolve);
            let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
            assert_eq!(res, (expected_pick, Resolution::Complete));
            assert_eq!(platform_ref.buffer_by_pick(res.0).unwrap().as_bstr(), expected);
        }

        Ok(())
    }

    #[test]
    fn with_external() -> crate::Result {
        let mut platform = new_platform(
            [gix_merge::blob::Driver {
                name: "b".into(),
                command:
                    "for arg in  %O %A %B %L %P %S %X %Y %F; do echo $arg >> \"%A\"; done; cat \"%O\" \"%B\" >> \"%A\""
                        .into(),
                ..Default::default()
            }],
            pipeline::Mode::ToGit,
        );
        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "b".into(),
            ResourceKind::CommonAncestorOrBase,
            &gix_object::find::Never,
        )?;

        let mut db = ObjectDb::default();
        for (content, kind) in [
            ("ours", ResourceKind::CurrentOrOurs),
            ("theirs", ResourceKind::OtherOrTheirs),
        ] {
            let id = db.insert(content);
            platform.set_resource(id, EntryKind::Blob, "b".into(), kind, &db)?;
        }

        let platform_ref = platform.prepare_merge(&db, Default::default())?;
        let mut buf = Vec::new();
        let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
        assert_eq!(res, (Pick::Buffer, Resolution::Complete), "merge drivers always merge ");
        let mut lines = cleaned_driver_lines(&buf)?;
        for tmp_file in lines.by_ref().take(3) {
            assert!(tmp_file.contains_str(&b".tmp"[..]), "{tmp_file}");
        }

        let lines: Vec<_> = lines.collect();
        assert_eq!(
            lines,
            [
                "7",
                "b",
                "ancestor label",
                "current label",
                "other label",
                "%F",
                "b",
                "theirs"
            ],
            "we handle word-splitting and definitely pick-up what's written into the %A buffer"
        );

        let id = db.insert("binary\0");
        platform.set_resource(id, EntryKind::Blob, "b".into(), ResourceKind::OtherOrTheirs, &db)?;
        let platform_ref = platform.prepare_merge(&db, Default::default())?;
        let res = platform_ref.merge(&mut buf, default_labels(), &Default::default())?;
        assert_eq!(
            res,
            (Pick::Buffer, Resolution::Complete),
            "merge drivers deal with binary themselves"
        );
        let mut lines = cleaned_driver_lines(&buf)?;
        for tmp_file in lines.by_ref().take(3) {
            assert!(tmp_file.contains_str(&b".tmp"[..]), "{tmp_file}");
        }
        let lines: Vec<_> = lines.collect();
        assert_eq!(
            lines,
            [
                "7",
                "b",
                "ancestor label",
                "current label",
                "other label",
                "%F",
                "b",
                "binary\0"
            ],
            "in this case, the binary lines are just taken verbatim"
        );

        Ok(())
    }

    #[test]
    fn missing_buffers_are_empty_buffers() -> crate::Result {
        let mut platform = new_platform(None, pipeline::Mode::ToGit);
        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "just-set".into(),
            ResourceKind::CommonAncestorOrBase,
            &gix_object::find::Never,
        )?;

        // Two deletions
        for kind in [ResourceKind::CurrentOrOurs, ResourceKind::OtherOrTheirs] {
            platform.set_resource(
                gix_hash::Kind::Sha1.null(),
                EntryKind::Blob,
                "does not matter for driver".into(),
                kind,
                &gix_object::find::Never,
            )?;
        }

        let platform_ref = platform.prepare_merge(&gix_object::find::Never, Default::default())?;

        let mut buf = Vec::new();
        let res = platform_ref.merge(&mut buf, Default::default(), &Default::default())?;
        assert_eq!(
            res,
            (Pick::Buffer, Resolution::Complete),
            "both versions are deleted, an actual merge happened"
        );
        assert!(
            buf.is_empty(),
            "the new buffer is considered empty, both sides were deleted, too"
        );

        let mut input = imara_diff::intern::InternedInput::new(&[][..], &[]);
        let res = platform_ref.builtin_merge(BuiltinDriver::Text, &mut buf, &mut input, Default::default());
        assert_eq!(
            res,
            Some((Pick::Buffer, Resolution::Complete)),
            "both versions are deleted"
        );
        assert!(buf.is_empty(), "the result is the same on direct invocation");

        let print_all = "for arg in $@ %O %A %B %L %P %S %X %Y %F; do echo $arg; done";
        let mut cmd = platform_ref.prepare_external_driver(print_all.into(), default_labels(), Default::default())?;
        let stdout = cmd.stdout(Stdio::piped()).output()?.stdout;
        let mut lines = cleaned_driver_lines(&stdout)?;
        for tmp_file in lines.by_ref().take(3) {
            assert!(tmp_file.contains_str(&b".tmp"[..]), "{tmp_file}");
        }
        let lines: Vec<_> = lines.collect();
        assert_eq!(
            lines,
            [
                "7",
                "does not matter for driver",
                "ancestor label",
                "current label",
                "other label",
                "%F"
            ],
            "word splitting is prevented thanks to proper quoting"
        );
        Ok(())
    }

    #[test]
    fn one_buffer_too_large() -> crate::Result {
        let mut platform = new_platform(None, pipeline::Mode::ToGit);
        platform.filter.options.large_file_threshold_bytes = 9;
        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "just-set".into(),
            ResourceKind::CommonAncestorOrBase,
            &gix_object::find::Never,
        )?;
        platform.filter.roots.other_root = platform.filter.roots.common_ancestor_root.clone();
        platform.filter.roots.current_root = platform.filter.roots.common_ancestor_root.clone();

        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "b".into(),
            ResourceKind::CurrentOrOurs,
            &gix_object::find::Never,
        )?;
        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "unspecified".into(),
            ResourceKind::OtherOrTheirs,
            &gix_object::find::Never,
        )?;

        let platform_ref = platform.prepare_merge(&gix_object::find::Never, Default::default())?;
        assert_eq!(platform_ref.other.data, platform::resource::Data::TooLarge { size: 12 });

        let mut out = Vec::new();
        let err = platform_ref
            .merge(&mut out, Default::default(), &Default::default())
            .unwrap_err();
        assert!(matches!(err, platform::merge::Error::ResourceTooLarge));

        let mut input = imara_diff::intern::InternedInput::new(&[][..], &[]);
        assert_eq!(
            platform_ref.builtin_merge(BuiltinDriver::Text, &mut out, &mut input, Default::default(),),
            None
        );

        let err = platform_ref
            .prepare_external_driver("bogus".into(), Default::default(), Default::default())
            .unwrap_err();
        assert!(matches!(
            err,
            platform::prepare_external_driver::Error::ResourceTooLarge { .. }
        ));
        Ok(())
    }

    fn cleaned_driver_lines(buf: &[u8]) -> std::io::Result<impl Iterator<Item = &BStr>> {
        let current_dir = gix_path::into_bstr(std::env::current_dir()?);
        Ok(buf
            .lines()
            .map(move |line| line.strip_prefix(current_dir.as_bytes()).unwrap_or(line).as_bstr()))
    }

    fn default_labels() -> builtin_driver::text::Labels<'static> {
        builtin_driver::text::Labels {
            ancestor: Some("ancestor label".into()),
            current: Some("current label".into()),
            other: Some("other label".into()),
        }
    }
}

mod prepare_merge {
    use crate::blob::platform::new_platform;
    use gix_merge::blob::platform::{resource, DriverChoice};
    use gix_merge::blob::{builtin_driver, pipeline, BuiltinDriver, ResourceKind};
    use gix_object::tree::EntryKind;

    #[test]
    fn ancestor_and_current_and_other_do_not_exist() -> crate::Result {
        let mut platform = new_platform(None, pipeline::Mode::ToGit);
        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "also-missing".into(),
            ResourceKind::CommonAncestorOrBase,
            &gix_object::find::Never,
        )?;

        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "can't-be-found-in-odb".into(),
            ResourceKind::CurrentOrOurs,
            &gix_object::find::Never,
        )?;
        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::BlobExecutable,
            "can't-be-found-in-odb".into(),
            ResourceKind::OtherOrTheirs,
            &gix_object::find::Never,
        )?;

        let state = platform
            .prepare_merge(&gix_object::find::Never, Default::default())
            .expect("no validation is done here, let the caller inspect");
        assert_eq!(state.ancestor.data, resource::Data::Missing);
        assert_eq!(state.current.data, resource::Data::Missing);
        assert_eq!(state.other.data, resource::Data::Missing);
        Ok(())
    }

    #[test]
    fn driver_selection() -> crate::Result {
        let mut platform = new_platform(
            [
                gix_merge::blob::Driver {
                    name: "union".into(),
                    ..Default::default()
                },
                gix_merge::blob::Driver {
                    name: "to proof it will be sorted".into(),
                    ..Default::default()
                },
                gix_merge::blob::Driver {
                    name: "b".into(),
                    recursive: Some("for-recursion".into()),
                    ..Default::default()
                },
                gix_merge::blob::Driver {
                    name: "for-recursion".into(),
                    recursive: Some("should not be looked up".into()),
                    ..Default::default()
                },
            ],
            pipeline::Mode::ToGit,
        );
        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "just-set".into(),
            ResourceKind::CommonAncestorOrBase,
            &gix_object::find::Never,
        )?;

        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "does not matter for driver".into(),
            ResourceKind::CurrentOrOurs,
            &gix_object::find::Never,
        )?;
        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::BlobExecutable,
            "also does not matter for driver".into(),
            ResourceKind::OtherOrTheirs,
            &gix_object::find::Never,
        )?;

        let prepared = platform.prepare_merge(&gix_object::find::Never, Default::default())?;
        assert_eq!(
            prepared.driver,
            DriverChoice::BuiltIn(BuiltinDriver::Text),
            "`merge` attribute means text"
        );

        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "unset".into(),
            ResourceKind::CommonAncestorOrBase,
            &gix_object::find::Never,
        )?;
        let prepared = platform.prepare_merge(&gix_object::find::Never, Default::default())?;
        assert_eq!(
            prepared.driver,
            DriverChoice::BuiltIn(BuiltinDriver::Text),
            "`-merge` attribute means binary, but it looked up 'current' which is still at some bogus worktree path"
        );

        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "unset".into(),
            ResourceKind::CurrentOrOurs,
            &gix_object::find::Never,
        )?;
        let prepared = platform.prepare_merge(&gix_object::find::Never, Default::default())?;
        assert_eq!(
            prepared.driver,
            DriverChoice::BuiltIn(BuiltinDriver::Binary),
            "`-merge` attribute means binary"
        );

        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "unspecified".into(),
            ResourceKind::CurrentOrOurs,
            &gix_object::find::Never,
        )?;
        let prepared = platform.prepare_merge(&gix_object::find::Never, Default::default())?;
        assert_eq!(
            prepared.driver,
            DriverChoice::BuiltIn(BuiltinDriver::Text),
            "`!merge` attribute means the hardcoded default"
        );

        platform.options.default_driver = Some("union".into());
        let prepared = platform.prepare_merge(&gix_object::find::Never, Default::default())?;
        let expected_idx = 3;
        assert_eq!(
            prepared.driver,
            DriverChoice::Index(expected_idx),
            "`!merge` attribute will also pick up the 'merge.default' configuration, and find the name in passed drivers first.\
            Note that the index is 1, even though it was 0 when passing the drivers - they are sorted by name."
        );
        assert_eq!(platform.drivers()[expected_idx].name, "union");

        platform.options.default_driver = Some("binary".into());
        let prepared = platform.prepare_merge(&gix_object::find::Never, Default::default())?;
        assert_eq!(
            prepared.driver,
            DriverChoice::BuiltIn(BuiltinDriver::Binary),
            "`!merge` attribute will also pick up the 'merge.default' configuration, non-overridden builtin filters work as well"
        );

        platform.options.default_driver = Some("Binary".into());
        let prepared = platform.prepare_merge(&gix_object::find::Never, Default::default())?;
        assert_eq!(
            prepared.driver,
            DriverChoice::BuiltIn(BuiltinDriver::Text),
            "'merge.default' is case-sensitive"
        );

        platform.set_resource(
            gix_hash::Kind::Sha1.null(),
            EntryKind::Blob,
            "b".into(),
            ResourceKind::CurrentOrOurs,
            &gix_object::find::Never,
        )?;
        let prepared = platform.prepare_merge(&gix_object::find::Never, Default::default())?;
        let expected_idx = 0;
        assert_eq!(prepared.driver, DriverChoice::Index(expected_idx));
        assert_eq!(
            platform.drivers()[expected_idx].name,
            "b",
            "by default, even if recursive is specified, it doesn't look it up"
        );

        let prepared = platform.prepare_merge(
            &gix_object::find::Never,
            gix_merge::blob::platform::merge::Options {
                is_virtual_ancestor: true,
                resolve_binary_with: None,
                ..Default::default()
            },
        )?;
        let expected_idx = 1;
        assert_eq!(prepared.driver, DriverChoice::Index(expected_idx),);
        assert_eq!(
            prepared.options.resolve_binary_with,
            Some(builtin_driver::binary::ResolveWith::Ours),
            "it automatically adjusts the merge mode for binary operations to work for bases"
        );
        assert_eq!(
            platform.drivers()[expected_idx].name,
            "for-recursion",
            "It looks up the final driver, including recursion, it only looks it up once though"
        );
        Ok(())
    }
}

mod set_resource {
    use crate::blob::platform::new_platform;
    use gix_merge::blob::{pipeline, ResourceKind};
    use gix_object::tree::EntryKind;

    #[test]
    fn invalid_resource_types() {
        let mut platform = new_platform(None, pipeline::Mode::ToGit);
        for (mode, name) in [(EntryKind::Commit, "Commit"), (EntryKind::Tree, "Tree")] {
            assert_eq!(
                platform
                    .set_resource(
                        gix_hash::Kind::Sha1.null(),
                        mode,
                        "a".into(),
                        ResourceKind::OtherOrTheirs,
                        &gix_object::find::Never,
                    )
                    .unwrap_err()
                    .to_string(),
                format!("Can only diff blobs, not {name}")
            );
        }
    }
}

fn new_platform(
    drivers: impl IntoIterator<Item = gix_merge::blob::Driver>,
    filter_mode: gix_merge::blob::pipeline::Mode,
) -> Platform {
    let root = gix_testtools::scripted_fixture_read_only("make_blob_repo.sh").expect("valid fixture");
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
    let filter = gix_merge::blob::Pipeline::new(
        gix_merge::blob::pipeline::WorktreeRoots {
            common_ancestor_root: Some(root.clone()),
            ..Default::default()
        },
        gix_filter::Pipeline::default(),
        Default::default(),
    );
    Platform::new(
        filter,
        filter_mode,
        attributes,
        drivers.into_iter().collect(),
        Default::default(),
    )
}
