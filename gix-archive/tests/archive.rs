/// Convert a hexadecimal hash into its corresponding `ObjectId` or _panic_.
fn hex_to_id(hex: &str) -> gix_hash::ObjectId {
    gix_hash::ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}

mod from_tree {
    use std::{io::Read, path::PathBuf};

    use gix_archive::Format;
    use gix_attributes::glob::pattern::Case;
    use gix_object::tree::EntryKind;
    use gix_testtools::bstr::ByteSlice;
    use gix_worktree::stack::state::attributes::Source;

    use crate::hex_to_id;

    #[cfg(target_pointer_width = "64")]
    const EXPECTED_BUFFER_LENGTH: usize = 551;
    #[cfg(target_pointer_width = "32")]
    const EXPECTED_BUFFER_LENGTH: usize = 479;

    #[test]
    fn basic_usage_internal() -> gix_testtools::Result {
        basic_usage(gix_archive::Format::InternalTransientNonPersistable, |buf| {
            assert_eq!(buf.len(), EXPECTED_BUFFER_LENGTH);

            let mut stream = gix_worktree_stream::Stream::from_read(std::io::Cursor::new(buf));
            let mut paths_and_modes = Vec::new();
            while let Some(mut entry) = stream.next_entry().expect("entry retrieval does not fail") {
                paths_and_modes.push((entry.relative_path().to_owned(), entry.mode.kind(), entry.id));
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf).expect("stream can always be read");
            }

            assert_eq!(
                paths_and_modes,
                &[
                    (
                        ".gitattributes".into(),
                        EntryKind::Blob,
                        hex_to_id("45c160c35c17ad264b96431cceb9793160396e99")
                    ),
                    (
                        "a".into(),
                        EntryKind::Blob,
                        hex_to_id("45b983be36b73c0788dc9cbcb76cbb80fc7bb057")
                    ),
                    (
                        "symlink-to-a".into(),
                        EntryKind::Link,
                        hex_to_id("2e65efe2a145dda7ee51d1741299f848e5bf752e")
                    ),
                    (
                        "dir/b".into(),
                        EntryKind::Blob,
                        hex_to_id("ab4a98190cf776b43cb0fe57cef231fb93fd07e6")
                    ),
                    (
                        "dir/subdir/exe".into(),
                        EntryKind::BlobExecutable,
                        hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
                    ),
                    (
                        "extra-file".into(),
                        EntryKind::Blob,
                        hex_to_id("0000000000000000000000000000000000000000")
                    ),
                    (
                        "extra-exe".into(),
                        if cfg!(windows) {
                            EntryKind::Blob
                        } else {
                            EntryKind::BlobExecutable
                        },
                        hex_to_id("0000000000000000000000000000000000000000")
                    ),
                    (
                        "extra-dir-empty".into(),
                        EntryKind::Tree,
                        hex_to_id("0000000000000000000000000000000000000000")
                    ),
                    (
                        "extra-dir/symlink-to-extra".into(),
                        EntryKind::Link,
                        hex_to_id("0000000000000000000000000000000000000000")
                    )
                ]
            );
            Ok(())
        })
    }

    #[test]
    #[cfg(feature = "tar")]
    fn basic_usage_tar() -> gix_testtools::Result {
        basic_usage(gix_archive::Format::Tar, |buf| {
            use tar::EntryType;
            let mut ar = tar::Archive::new(buf.as_slice());
            let mut out = Vec::new();
            for entry in ar.entries()? {
                let mut entry = entry?;
                let copied = std::io::copy(&mut entry, &mut std::io::sink())?;

                let header = entry.header();
                assert_eq!(
                    copied,
                    header.size()?,
                    "size field matches the size of the actual stream"
                );
                out.push((
                    entry.path_bytes().as_bstr().to_owned(),
                    header.entry_type(),
                    header.size()?,
                    header.mode()?,
                ));
            }
            assert_eq!(
                out,
                [
                    ("prefix/.gitattributes", EntryType::Regular, 56, 420),
                    ("prefix/a", EntryType::Regular, 3, 420),
                    ("prefix/symlink-to-a", EntryType::Symlink, 0, 420),
                    ("prefix/dir/b", EntryType::Regular, 3, 420),
                    ("prefix/dir/subdir/exe", EntryType::Regular, 0, 493),
                    ("prefix/extra-file", EntryType::Regular, 21, 420),
                    (
                        "prefix/extra-exe",
                        EntryType::Regular,
                        0,
                        if cfg!(windows) { 420 } else { 493 }
                    ),
                    ("prefix/extra-dir-empty", EntryType::Directory, 0, 420),
                    ("prefix/extra-dir/symlink-to-extra", EntryType::Symlink, 0, 420)
                ]
                .into_iter()
                .map(|(path, b, c, d)| (bstr::BStr::new(path).to_owned(), b, c, d))
                .collect::<Vec<_>>()
            );
            Ok(())
        })
    }

    #[test]
    #[cfg(feature = "tar_gz")]
    fn basic_usage_tar_gz() -> gix_testtools::Result {
        basic_usage(
            gix_archive::Format::TarGz {
                compression_level: Some(9),
            },
            |buf| {
                assert!(
                    buf.len() < 340,
                    "quite a bit smaller than uncompressed: {} < 340",
                    buf.len()
                );
                Ok(())
            },
        )
    }

    #[test]
    #[cfg(feature = "zip")]
    fn basic_usage_zip() -> gix_testtools::Result {
        basic_usage(
            gix_archive::Format::Zip {
                compression_level: Some(9),
            },
            |buf| {
                assert!(
                    buf.len() < 1400,
                    "much bigger than uncompressed for some reason (565): {} < 1400",
                    buf.len()
                );
                let ar = rawzip::ZipArchive::from_slice(buf.as_slice())?;
                assert_eq!(
                    {
                        let mut n: Vec<_> = Vec::new();
                        for entry_result in ar.entries() {
                            let entry = entry_result?;
                            n.push(String::from_utf8_lossy(entry.file_path().as_ref()).to_string());
                        }
                        n.sort();
                        n
                    },
                    &[
                        "prefix/.gitattributes",
                        "prefix/a",
                        "prefix/dir/b",
                        "prefix/dir/subdir/exe",
                        "prefix/extra-dir-empty/",
                        "prefix/extra-dir/symlink-to-extra",
                        "prefix/extra-exe",
                        "prefix/extra-file",
                        "prefix/symlink-to-a"
                    ]
                );
                
                // Find the symlink entry
                let ar = rawzip::ZipArchive::from_slice(buf.as_slice())?;
                let mut found_link = false;
                for entry_result in ar.entries() {
                    let entry = entry_result?;
                    if String::from_utf8_lossy(entry.file_path().as_ref()) == "prefix/symlink-to-a" {
                        assert!(!entry.is_dir());
                        let mode = entry.mode();
                        assert!(mode.is_symlink(), "symlinks are supported as well, but only on Unix");
                        assert_eq!(mode.value(), 0o120644, "the mode specifies what it should be");
                        let wayfinder = entry.wayfinder();
                        let zip_entry = ar.get_entry(wayfinder)?;
                        // For symlinks stored with Store compression, the data is uncompressed
                        let data = zip_entry.data();
                        assert_eq!(data.as_bstr(), "a");
                        found_link = true;
                        break;
                    }
                }
                assert!(found_link, "symlink entry should be found");
                Ok(())
            },
        )
    }

    fn basic_usage(
        format: gix_archive::Format,
        make_assertion: impl FnOnce(Vec<u8>) -> gix_testtools::Result,
    ) -> gix_testtools::Result {
        let (dir, head_tree, odb, mut cache) = basic()?;
        let mut stream = gix_worktree_stream::from_tree(
            head_tree,
            odb.clone(),
            noop_pipeline(),
            move |rela_path, mode, attrs| {
                cache
                    .at_entry(rela_path, Some(mode.into()), &odb)
                    .map(|entry| entry.matching_attributes(attrs))
                    .map(|_| ())
            },
        );
        stream
            .add_entry_from_path(&dir, &dir.join("extra-file"))?
            .add_entry_from_path(&dir, &dir.join("extra-exe"))?
            .add_entry_from_path(&dir, &dir.join("extra-dir-empty"))?
            .add_entry_from_path(&dir, &dir.join("extra-dir").join("symlink-to-extra"))?;

        let mut buf = Vec::new();
        if format == Format::InternalTransientNonPersistable {
            std::io::copy(&mut stream.into_read(), &mut buf)?;
        } else {
            if matches!(format, Format::Zip { .. }) {
                gix_archive::write_stream_seek(
                    &mut stream,
                    gix_worktree_stream::Stream::next_entry,
                    std::io::Cursor::new(&mut buf),
                    gix_archive::Options {
                        format,
                        tree_prefix: Some("prefix/".into()),
                        modification_time: 1820000000, // needs to be within a certain bound to be a valid MSDos time!
                    },
                )?;
            } else {
                gix_archive::write_stream(
                    &mut stream,
                    gix_worktree_stream::Stream::next_entry,
                    &mut buf,
                    gix_archive::Options {
                        format,
                        tree_prefix: Some("prefix/".into()),
                        modification_time: 120,
                    },
                )?;
            }
            assert!(
                stream.next_entry()?.is_none(),
                "stream is exhausted, all written to buf"
            );
        }
        make_assertion(buf).expect("all tests pass");
        Ok(())
    }

    fn basic() -> gix_testtools::Result<(PathBuf, gix_hash::ObjectId, gix_odb::HandleArc, gix_worktree::Stack)> {
        let dir = gix_testtools::scripted_fixture_read_only("basic.sh")?;

        let head = {
            let hex = std::fs::read(dir.join("head.hex"))?;
            gix_hash::ObjectId::from_hex(hex.trim())?
        };
        let odb = gix_odb::at(dir.join(".git").join("objects"))?;

        let mut collection = Default::default();
        let mut buf = Default::default();
        let attributes = gix_worktree::stack::state::Attributes::new(
            gix_attributes::Search::new_globals(None::<PathBuf>, &mut buf, &mut collection)?,
            None,
            Source::WorktreeThenIdMapping,
            collection,
        );
        let state = gix_worktree::stack::State::AttributesStack(attributes);
        let cache = gix_worktree::Stack::new(&dir, state, Case::Sensitive, Default::default(), Default::default());
        Ok((dir, head, odb.into_arc()?, cache))
    }

    fn noop_pipeline() -> gix_filter::Pipeline {
        gix_filter::Pipeline::new(Default::default(), Default::default())
    }
}
