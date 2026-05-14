pub(crate) mod convert_to_diffable {

    use gix_diff::blob::{
        ResourceKind, pipeline,
        pipeline::{Options, WorktreeRoots},
    };
    use gix_filter::{eol, eol::AutoCrlf};
    use gix_object::{bstr::ByteSlice, tree::EntryKind};
    use gix_worktree::stack::state::attributes;

    use crate::util::ObjectDb;

    #[test]
    fn simple() -> crate::Result {
        for mode in [
            pipeline::Mode::ToWorktreeAndBinaryToText,
            pipeline::Mode::ToGit,
            pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
        ] {
            let tmp = gix_testtools::tempfile::TempDir::new()?;
            let mut filter = gix_diff::blob::Pipeline::new(
                WorktreeRoots {
                    old_root: Some(tmp.path().to_owned()),
                    new_root: None,
                },
                gix_filter::Pipeline::default(),
                vec![],
                default_options(),
            );

            let does_not_matter = gix_hash::Kind::Sha1.null();
            let mut buf = Vec::new();
            let a_name = "a";
            let a_content = "a-content";
            std::fs::write(tmp.path().join(a_name), a_content.as_bytes())?;
            let out = filter.convert_to_diffable(
                &does_not_matter,
                EntryKind::Blob,
                a_name.into(),
                ResourceKind::OldOrSource,
                &mut |_, _| {},
                &gix_object::find::Never,
                mode,
                &mut buf,
            )?;
            assert!(out.driver_index.is_none(), "there was no driver");
            assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
            assert_eq!(buf.as_bstr(), a_content, "there is no transformations configured");

            let link_name = "link";
            gix_fs::symlink::create(a_name.as_ref(), &tmp.path().join(link_name))?;
            let out = filter.convert_to_diffable(
                &does_not_matter,
                EntryKind::Link,
                link_name.into(),
                ResourceKind::OldOrSource,
                &mut |_, _| {},
                &gix_object::find::Never,
                mode,
                &mut buf,
            )?;

            assert!(out.driver_index.is_none());
            assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
            assert_eq!(
                buf.as_bstr(),
                a_name,
                "links are just files with a different mode, with its content pointing to the target"
            );
            drop(tmp);

            let mut db = ObjectDb::default();
            let b_content = "b-content";
            let id = db.insert(b_content)?;

            let out = filter.convert_to_diffable(
                &id,
                EntryKind::Blob,
                a_name.into(),
                ResourceKind::NewOrDestination,
                &mut |_, _| {},
                &db,
                mode,
                &mut buf,
            )?;

            assert!(out.driver_index.is_none(), "there was no driver");
            assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
            assert_eq!(buf.as_bstr(), b_content, "there is no transformations configured");
        }

        Ok(())
    }

    #[test]
    fn binary_below_large_file_threshold() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let mut filter = gix_diff::blob::Pipeline::new(
            WorktreeRoots {
                old_root: None,
                new_root: Some(tmp.path().to_owned()),
            },
            gix_filter::Pipeline::default(),
            vec![],
            gix_diff::blob::pipeline::Options {
                large_file_threshold_bytes: 5,
                ..default_options()
            },
        );

        let does_not_matter = gix_hash::Kind::Sha1.null();
        let mut buf = Vec::new();
        let a_name = "a";
        let large_content = "a\0b";
        std::fs::write(tmp.path().join(a_name), large_content.as_bytes())?;
        let out = filter.convert_to_diffable(
            &does_not_matter,
            EntryKind::BlobExecutable,
            a_name.into(),
            ResourceKind::NewOrDestination,
            &mut |_, _| {},
            &gix_object::find::Never,
            pipeline::Mode::default(),
            &mut buf,
        )?;
        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, Some(pipeline::Data::Binary { size: 3 }), "detected in buffer");
        assert_eq!(buf.len(), 0, "it should avoid querying that data in the first place");

        let mut db = ObjectDb::default();
        let id = db.insert(large_content)?;
        let out = filter.convert_to_diffable(
            &id,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::OldOrSource,
            &mut |_, _| {},
            &db,
            pipeline::Mode::default(),
            &mut buf,
        )?;

        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, Some(pipeline::Data::Binary { size: 3 }));
        assert_eq!(buf.len(), 0, "it should avoid querying that data in the first place");

        Ok(())
    }

    #[test]
    fn above_large_file_threshold() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let mut filter = gix_diff::blob::Pipeline::new(
            WorktreeRoots {
                old_root: None,
                new_root: Some(tmp.path().to_owned()),
            },
            gix_filter::Pipeline::default(),
            vec![],
            gix_diff::blob::pipeline::Options {
                large_file_threshold_bytes: 4,
                ..default_options()
            },
        );

        let does_not_matter = gix_hash::Kind::Sha1.null();
        let mut buf = Vec::new();
        let a_name = "a";
        let large_content = "hello";
        std::fs::write(tmp.path().join(a_name), large_content.as_bytes())?;
        let out = filter.convert_to_diffable(
            &does_not_matter,
            EntryKind::BlobExecutable,
            a_name.into(),
            ResourceKind::NewOrDestination,
            &mut |_, _| {},
            &gix_object::find::Never,
            pipeline::Mode::default(),
            &mut buf,
        )?;
        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, Some(pipeline::Data::Binary { size: 5 }));
        assert_eq!(buf.len(), 0, "it should avoid querying that data in the first place");

        // On windows, this test fails as it needs the link target to exist, and it was
        // hard to make it exist with a relative path, strangely enough.
        // For worktree checkouts, this works, that's all that matters for now.
        if !cfg!(windows) {
            let link_name = "link";
            gix_fs::symlink::create(large_content.as_ref(), &tmp.path().join(link_name))?;
            let out = filter.convert_to_diffable(
                &does_not_matter,
                EntryKind::Link,
                link_name.into(),
                ResourceKind::NewOrDestination,
                &mut |_, _| {},
                &gix_object::find::Never,
                pipeline::Mode::default(),
                &mut buf,
            )?;

            assert!(out.driver_index.is_none());
            assert_eq!(
                out.data,
                Some(pipeline::Data::Buffer { is_derived: false }),
                "links are always read and never considered large"
            );
            assert_eq!(buf.as_bstr(), large_content);
        }
        drop(tmp);

        let mut db = ObjectDb::default();
        let id = db.insert(large_content)?;

        let out = filter.convert_to_diffable(
            &id,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::OldOrSource,
            &mut |_, _| {},
            &db,
            pipeline::Mode::default(),
            &mut buf,
        )?;

        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, Some(pipeline::Data::Binary { size: 5 }));
        assert_eq!(buf.len(), 0, "it should avoid querying that data in the first place");

        Ok(())
    }

    #[test]
    fn non_existing() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let mut filter = gix_diff::blob::Pipeline::new(
            WorktreeRoots {
                old_root: Some(tmp.path().to_owned()),
                new_root: None,
            },
            gix_filter::Pipeline::default(),
            vec![],
            default_options(),
        );

        let null = gix_hash::Kind::Sha1.null();
        let mut buf = vec![1];
        let a_name = "a";
        assert!(
            !tmp.path().join(a_name).exists(),
            "precondition: worktree file doesn't exist"
        );
        let out = filter.convert_to_diffable(
            &null,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::OldOrSource,
            &mut |_, _| {},
            &gix_object::find::Never,
            pipeline::Mode::default(),
            &mut buf,
        )?;
        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, None);
        assert_eq!(buf.len(), 0, "always cleared");

        buf.push(1);
        let out = filter.convert_to_diffable(
            &null,
            EntryKind::Link,
            "link".into(),
            ResourceKind::OldOrSource,
            &mut |_, _| {},
            &gix_object::find::Never,
            pipeline::Mode::default(),
            &mut buf,
        )?;
        assert!(out.driver_index.is_none());
        assert_eq!(out.data, None);
        assert_eq!(buf.len(), 0, "always cleared");

        drop(tmp);

        buf.push(1);
        let out = filter.convert_to_diffable(
            &null,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::NewOrDestination,
            &mut |_, _| {},
            &gix_object::find::Never,
            pipeline::Mode::default(),
            &mut buf,
        )?;

        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, None);
        assert_eq!(buf.len(), 0, "it's always cleared before any potential use");

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
        let mut filter = gix_diff::blob::Pipeline::new(
            WorktreeRoots {
                old_root: Some(tmp.path().to_owned()),
                new_root: None,
            },
            filter,
            vec![],
            default_options(),
        );

        let does_not_matter = gix_hash::Kind::Sha1.null();
        let mut buf = Vec::new();
        let a_name = "a";
        let a_content = "a-content\n";
        std::fs::write(tmp.path().join(a_name), a_content.as_bytes())?;
        let out = filter.convert_to_diffable(
            &does_not_matter,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::OldOrSource,
            &mut |_, _| {},
            &gix_object::find::Never,
            pipeline::Mode::default(),
            &mut buf,
        )?;
        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
        assert_eq!(
            buf.as_bstr(),
            a_content,
            "worktree files are assumed to be filtered already, and are verbatim"
        );

        let b_name = "b";
        let b_content = "a\r\nb";
        std::fs::write(tmp.path().join(b_name), b_content.as_bytes())?;
        let out = filter.convert_to_diffable(
            &does_not_matter,
            EntryKind::Blob,
            b_name.into(),
            ResourceKind::OldOrSource,
            &mut |_, _| {},
            &gix_object::find::Never,
            pipeline::Mode::ToGit,
            &mut buf,
        )?;
        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
        assert_eq!(
            buf.as_bstr(),
            "a\nb",
            "worktree files are converted back to git if the mode needs it"
        );

        // On windows, this test fails as it needs the link target to exist, and it's kind of impossible
        // to test what we want to test apparently.
        if !cfg!(windows) {
            let link_name = "link";
            let link_content = "hello\n";
            gix_fs::symlink::create(link_content.as_ref(), &tmp.path().join(link_name))?;
            let out = filter.convert_to_diffable(
                &does_not_matter,
                EntryKind::Link,
                link_name.into(),
                ResourceKind::OldOrSource,
                &mut |_, _| {},
                &gix_object::find::Never,
                pipeline::Mode::default(),
                &mut buf,
            )?;

            assert!(out.driver_index.is_none());
            assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
            assert_eq!(
                buf.as_bstr(),
                link_content,
                "links aren't put through worktree filters, otherwise it would have its newlines replaced"
            );
        }
        drop(tmp);

        let mut db = ObjectDb::default();
        let b_content = "b-content\n";
        let id = db.insert(b_content)?;

        let out = filter.convert_to_diffable(
            &id,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::NewOrDestination,
            &mut |_, _| {},
            &db,
            pipeline::Mode::default(),
            &mut buf,
        )?;

        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
        assert_eq!(buf.as_bstr(), "b-content\r\n", "LF to CRLF by worktree filtering");

        let mut db = ObjectDb::default();
        let b_content = "b\n";
        let id = db.insert(b_content)?;
        let out = filter.convert_to_diffable(
            &id,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::NewOrDestination,
            &mut |_, _| {},
            &db,
            pipeline::Mode::ToGit,
            &mut buf,
        )?;

        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
        assert_eq!(buf.as_bstr(), "b\n", "no filtering was performed at all");

        Ok(())
    }

    #[test]
    fn binary_by_buffer_inspection() -> crate::Result {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let root = crate::scripted_fixture_read_only("make_blob_repo.sh")?;
        let mut attributes = gix_worktree::Stack::new(
            root,
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
        let mut filter = gix_diff::blob::Pipeline::new(
            WorktreeRoots {
                old_root: Some(tmp.path().to_owned()),
                new_root: None,
            },
            gix_filter::Pipeline::default(),
            vec![gix_diff::blob::Driver {
                name: "c".into(),
                binary_to_text_command: Some(r"printf '\0'; cat <".into()),
                ..Default::default()
            }],
            default_options(),
        );

        let does_not_matter = gix_hash::Kind::Sha1.null();
        let mut buf = Vec::new();
        let a_name = "a";
        let a_content = "a\0b";
        std::fs::write(tmp.path().join(a_name), a_content.as_bytes())?;
        let out = filter.convert_to_diffable(
            &does_not_matter,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::OldOrSource,
            &mut |_, _| {},
            &gix_object::find::Never,
            pipeline::Mode::default(),
            &mut buf,
        )?;
        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, Some(pipeline::Data::Binary { size: 3 }));
        assert_eq!(buf.len(), 0, "binary files aren't stored, even if we read them");

        // LINK with null-bytes can't be created, and generally we ignore a lot of checks on links
        // for good reason. Hard to test.
        drop(tmp);

        let mut db = ObjectDb::default();
        let b_content = "b-co\0ntent\n";
        let id = db.insert(b_content)?;

        let out = filter.convert_to_diffable(
            &id,
            EntryKind::Blob,
            a_name.into(),
            ResourceKind::NewOrDestination,
            &mut |_, _| {},
            &db,
            pipeline::Mode::default(),
            &mut buf,
        )?;

        assert!(out.driver_index.is_none(), "there was no driver");
        assert_eq!(out.data, Some(pipeline::Data::Binary { size: 11 }));
        assert_eq!(buf.len(), 0, "buffers are cleared even if we read them");

        let platform = attributes.at_entry("c", None, &gix_object::find::Never)?;

        let id = db.insert("b")?;
        let out = filter.convert_to_diffable(
            &id,
            EntryKind::Blob,
            "c".into(),
            ResourceKind::NewOrDestination,
            &mut |_, out| {
                let _ = platform.matching_attributes(out);
            },
            &db,
            pipeline::Mode::default(),
            &mut buf,
        )?;

        assert_eq!(out.driver_index, Some(0));
        assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: true }));
        assert_eq!(
            buf.as_bstr(),
            "\0b",
            "if binary-text-conversion is set, we don't care if it outputs null-bytes, let everything pass"
        );

        Ok(())
    }

    #[test]
    fn with_driver() -> crate::Result {
        let root = crate::scripted_fixture_read_only("make_blob_repo.sh")?;
        let command = "echo to-text; cat <";
        let mut attributes = gix_worktree::Stack::new(
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
        let mut filter = gix_diff::blob::Pipeline::new(
            WorktreeRoots {
                old_root: Some(root.clone()),
                new_root: None,
            },
            gix_filter::Pipeline::default(),
            vec![
                gix_diff::blob::Driver {
                    name: "a".into(),
                    binary_to_text_command: Some(command.into()),
                    ..Default::default()
                },
                gix_diff::blob::Driver {
                    name: "b".into(),
                    is_binary: Some(true),
                    ..Default::default()
                },
                gix_diff::blob::Driver {
                    name: "c".into(),
                    binary_to_text_command: Some(command.into()),
                    is_binary: Some(true),
                    ..Default::default()
                },
                gix_diff::blob::Driver {
                    name: "d".into(),
                    binary_to_text_command: Some(command.into()),
                    ..Default::default()
                },
                gix_diff::blob::Driver {
                    name: "missing".into(),
                    ..Default::default()
                },
            ],
            default_options(),
        );

        let mut db = ObjectDb::default();
        let null = gix_hash::Kind::Sha1.null();
        let mut buf = Vec::new();
        let platform = attributes.at_entry("a", None, &gix_object::find::Never)?;
        let worktree_modes = [
            pipeline::Mode::ToWorktreeAndBinaryToText,
            pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
        ];
        let all_modes = [
            pipeline::Mode::ToGit,
            pipeline::Mode::ToWorktreeAndBinaryToText,
            pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
        ];
        for mode in worktree_modes {
            let out = filter.convert_to_diffable(
                &null,
                EntryKind::Blob,
                "a".into(),
                ResourceKind::OldOrSource,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &gix_object::find::Never,
                mode,
                &mut buf,
            )?;
            assert_eq!(out.driver_index, Some(0));
            assert_eq!(
                out.data,
                Some(pipeline::Data::Buffer {
                    is_derived: !matches!(mode, pipeline::Mode::ToGit)
                })
            );
            assert_eq!(buf.as_bstr(), "to-text\na\n", "filter was applied");
        }

        let out = filter.convert_to_diffable(
            &null,
            EntryKind::Blob,
            "a".into(),
            ResourceKind::OldOrSource,
            &mut |_, out| {
                let _ = platform.matching_attributes(out);
            },
            &gix_object::find::Never,
            pipeline::Mode::ToGit,
            &mut buf,
        )?;
        assert_eq!(out.driver_index, Some(0));
        assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
        assert_eq!(buf.as_bstr(), "a\n", "unconditionally use git according to mode");

        let id = db.insert("a\n")?;
        for mode in worktree_modes {
            let out = filter.convert_to_diffable(
                &id,
                EntryKind::Blob,
                "a".into(),
                ResourceKind::NewOrDestination,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &db,
                mode,
                &mut buf,
            )?;
            assert_eq!(out.driver_index, Some(0));
            assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: true }));
            assert_eq!(buf.as_bstr(), "to-text\na\n", "filter was applied");
        }

        let out = filter.convert_to_diffable(
            &id,
            EntryKind::Blob,
            "a".into(),
            ResourceKind::NewOrDestination,
            &mut |_, out| {
                let _ = platform.matching_attributes(out);
            },
            &db,
            pipeline::Mode::ToGit,
            &mut buf,
        )?;
        assert_eq!(out.driver_index, Some(0));
        assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
        assert_eq!(
            buf.as_bstr(),
            "a\n",
            "no filter was applied in this mode, also when using the ODB"
        );

        let platform = attributes.at_entry("missing", None, &gix_object::find::Never)?;
        for mode in all_modes {
            buf.push(1);
            let out = filter.convert_to_diffable(
                &null,
                EntryKind::Link,
                "missing".into(), /* does not actually exist */
                ResourceKind::OldOrSource,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &gix_object::find::Never,
                mode,
                &mut buf,
            )?;
            assert_eq!(out.driver_index, Some(4), "despite missing, we get driver information");
            assert_eq!(out.data, None);
            assert_eq!(buf.len(), 0, "always cleared");

            buf.push(1);
            let out = filter.convert_to_diffable(
                &null,
                EntryKind::Link,
                "missing".into(), /* does not actually exist */
                ResourceKind::NewOrDestination,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &gix_object::find::Never,
                mode,
                &mut buf,
            )?;
            assert_eq!(out.driver_index, Some(4), "despite missing, we get driver information");
            assert_eq!(out.data, None);
            assert_eq!(buf.len(), 0, "always cleared");

            buf.push(1);
            let id = db.insert("link-target")?;
            let out = filter.convert_to_diffable(
                &id,
                EntryKind::Link,
                "missing".into(),
                ResourceKind::NewOrDestination,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &db,
                mode,
                &mut buf,
            )?;
            assert_eq!(out.driver_index, Some(4), "despite missing, we get driver information");
            assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
            assert_eq!(
                buf.as_bstr(),
                "link-target",
                "no matter what, links always look the same."
            );
        }

        let platform = attributes.at_entry("b", None, &gix_object::find::Never)?;
        for mode in all_modes {
            buf.push(1);
            let out = filter.convert_to_diffable(
                &null,
                EntryKind::Blob,
                "b".into(),
                ResourceKind::OldOrSource,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &gix_object::find::Never,
                mode,
                &mut buf,
            )?;

            assert_eq!(out.driver_index, Some(1));
            assert_eq!(
                out.data,
                Some(pipeline::Data::Binary { size: 2 }),
                "binary value comes from driver, and it's always respected with worktree source"
            );
            assert_eq!(buf.len(), 0, "it's always cleared before any potential use");
        }

        let id = db.insert("b\n")?;
        for mode in all_modes {
            buf.push(1);
            let out = filter.convert_to_diffable(
                &id,
                EntryKind::Blob,
                "b".into(),
                ResourceKind::NewOrDestination,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &db,
                mode,
                &mut buf,
            )?;

            assert_eq!(out.driver_index, Some(1));
            assert_eq!(
                out.data,
                Some(pipeline::Data::Binary { size: 2 }),
                "binary value comes from driver, and it's always respected with DB source"
            );
            assert_eq!(buf.len(), 0, "it's always cleared before any potential use");
        }

        let platform = attributes.at_entry("c", None, &gix_object::find::Never)?;
        for mode in worktree_modes {
            let out = filter.convert_to_diffable(
                &null,
                EntryKind::Blob,
                "c".into(),
                ResourceKind::OldOrSource,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &gix_object::find::Never,
                mode,
                &mut buf,
            )?;
            assert_eq!(out.driver_index, Some(2));
            assert_eq!(
                out.data,
                Some(pipeline::Data::Buffer {
                    is_derived: !matches!(mode, pipeline::Mode::ToGit)
                })
            );
            assert_eq!(
                buf.as_bstr(),
                "to-text\nc\n",
                "filter was applied, it overrides binary=true"
            );
        }

        let id = db.insert("c\n")?;
        for mode in worktree_modes {
            let out = filter.convert_to_diffable(
                &id,
                EntryKind::Blob,
                "c".into(),
                ResourceKind::NewOrDestination,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &db,
                mode,
                &mut buf,
            )?;
            assert_eq!(out.driver_index, Some(2));
            assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: true }));
            assert_eq!(
                buf.as_bstr(),
                "to-text\nc\n",
                "filter was applied, it overrides binary=true"
            );
        }

        let platform = attributes.at_entry("unset", None, &gix_object::find::Never)?;
        for mode in all_modes {
            let out = filter.convert_to_diffable(
                &null,
                EntryKind::Blob,
                "unset".into(),
                ResourceKind::OldOrSource,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &gix_object::find::Never,
                mode,
                &mut buf,
            )?;
            assert_eq!(
                out.driver_index, None,
                "no driver is associated, as `diff` is explicitly unset"
            );
            assert_eq!(
                out.data,
                Some(pipeline::Data::Binary { size: 6 }),
                "unset counts as binary"
            );
            assert_eq!(buf.len(), 0);
        }

        let id = db.insert("unset\n")?;
        for mode in all_modes {
            let out = filter.convert_to_diffable(
                &id,
                EntryKind::Blob,
                "unset".into(),
                ResourceKind::NewOrDestination,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &db,
                mode,
                &mut buf,
            )?;
            assert_eq!(
                out.driver_index, None,
                "no driver is associated, as `diff` is explicitly unset"
            );
            assert_eq!(
                out.data,
                Some(pipeline::Data::Binary { size: 6 }),
                "unset counts as binary"
            );
            assert_eq!(buf.len(), 0);
        }

        let platform = attributes.at_entry("d", None, &gix_object::find::Never)?;
        let id = db.insert("d-in-db")?;
        for mode in worktree_modes {
            let out = filter.convert_to_diffable(
                &null,
                EntryKind::Blob,
                "d".into(),
                ResourceKind::OldOrSource,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &gix_object::find::Never,
                mode,
                &mut buf,
            )?;
            assert_eq!(out.driver_index, Some(3));
            assert_eq!(
                out.data,
                Some(pipeline::Data::Buffer {
                    is_derived: !matches!(mode, pipeline::Mode::ToGit)
                })
            );
            assert_eq!(
                buf.as_bstr(),
                "to-text\nd\n",
                "the worktree + text conversion was triggered for worktree source"
            );

            let out = filter.convert_to_diffable(
                &id,
                EntryKind::Blob,
                "d".into(),
                ResourceKind::NewOrDestination,
                &mut |_, out| {
                    let _ = platform.matching_attributes(out);
                },
                &db,
                mode,
                &mut buf,
            )?;
            assert_eq!(out.driver_index, Some(3));
            assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: true }));
            assert_eq!(
                buf.as_bstr(),
                "to-text\nd-in-db",
                "the worktree + text conversion was triggered for db source"
            );
        }

        let platform = attributes.at_entry("e-no-attr", None, &gix_object::find::Never)?;
        let out = filter.convert_to_diffable(
            &null,
            EntryKind::Blob,
            "e-no-attr".into(),
            ResourceKind::OldOrSource,
            &mut |_, out| {
                let _ = platform.matching_attributes(out);
            },
            &gix_object::find::Never,
            pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
            &mut buf,
        )?;
        assert_eq!(out.driver_index, None);
        assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
        assert_eq!(
            buf.as_bstr(),
            "e\n",
            "no text filter, so git conversion was applied for worktree source"
        );

        let id = db.insert("e-in-db")?;
        let out = filter.convert_to_diffable(
            &id,
            EntryKind::Blob,
            "e-no-attr".into(),
            ResourceKind::NewOrDestination,
            &mut |_, out| {
                let _ = platform.matching_attributes(out);
            },
            &db,
            pipeline::Mode::ToGitUnlessBinaryToTextIsPresent,
            &mut buf,
        )?;
        assert_eq!(out.driver_index, None);
        assert_eq!(out.data, Some(pipeline::Data::Buffer { is_derived: false }));
        assert_eq!(
            buf.as_bstr(),
            "e-in-db",
            "no text filter, so git conversion was applied for ODB source"
        );

        Ok(())
    }

    pub(crate) fn default_options() -> Options {
        Options {
            large_file_threshold_bytes: 0,
            fs: gix_fs::Capabilities {
                precompose_unicode: false,
                ignore_case: false,
                executable_bit: true,
                symlink: true,
            },
        }
    }
}
