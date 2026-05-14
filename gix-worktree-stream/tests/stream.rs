/// Convert a hexadecimal hash into its corresponding `ObjectId` or _panic_.
fn hex_to_id(hex_sha1: &str, hex_sha256: &str) -> gix_hash::ObjectId {
    match gix_testtools::object_hash() {
        gix_hash::Kind::Sha1 => gix_hash::ObjectId::from_hex(hex_sha1.as_bytes()).expect("40 bytes hex"),
        gix_hash::Kind::Sha256 => gix_hash::ObjectId::from_hex(hex_sha256.as_bytes()).expect("64 bytes hex"),
        _ => unimplemented!(),
    }
}

mod from_tree {
    use std::{
        convert::Infallible,
        io::{Error, Read, Write},
        path::PathBuf,
        sync::Arc,
    };

    use gix_attributes::glob::pattern::Case;
    use gix_hash::oid;
    use gix_object::{Data, bstr::ByteSlice, tree::EntryKind};
    use gix_worktree::stack::state::attributes::Source;
    use std::sync::LazyLock;

    use crate::hex_to_id;

    #[derive(Clone)]
    struct FailObjectRetrieval;

    impl gix_object::Find for FailObjectRetrieval {
        fn try_find<'a>(
            &self,
            _id: &oid,
            _buffer: &'a mut Vec<u8>,
        ) -> Result<Option<Data<'a>>, gix_object::find::Error> {
            Err(Box::new(Error::other("object retrieval failed")))
        }
    }

    #[test]
    fn can_receive_err_if_root_is_not_found() {
        let mut stream = gix_worktree_stream::from_tree(
            gix_testtools::object_hash().null(),
            FailObjectRetrieval,
            mutating_pipeline(false),
            |_, _, _| -> Result<_, Infallible> { unreachable!("must not be called") },
        );
        let err = stream.next_entry().unwrap_err();
        assert_eq!(err.to_string(), "Could not find a tree to traverse");
    }

    #[test]
    fn can_receive_err_if_attribute_not_found() -> gix_testtools::Result {
        let (_dir, head_tree, odb, _cache) = basic()?;
        let mut stream = gix_worktree_stream::from_tree(head_tree, odb, mutating_pipeline(false), |_, _, _| {
            Err(Error::other("attribute retrieval failed"))
        });
        let err = stream.next_entry().unwrap_err();
        assert_eq!(
            err.to_string(),
            "Could not query attributes for path \".gitattributes\""
        );
        Ok(())
    }

    #[test]
    fn will_provide_all_information_and_respect_export_ignore() -> gix_testtools::Result {
        let (dir, head_tree, odb, mut cache) = basic()?;
        let mut stream = gix_worktree_stream::from_tree(
            head_tree,
            odb.clone(),
            mutating_pipeline(true),
            move |rela_path, mode, attrs| {
                cache
                    .at_entry(rela_path, Some(mode.into()), &odb)
                    .map(|entry| entry.matching_attributes(attrs))
                    .map(|_| ())
            },
        );
        let object_hash = gix_testtools::object_hash();
        stream
            .add_entry_from_path(&dir, &dir.join("extra-file"), object_hash)?
            .add_entry_from_path(&dir, &dir.join("extra-bigfile"), object_hash)?
            .add_entry_from_path(&dir, &dir.join("extra-exe"), object_hash)?
            .add_entry_from_path(&dir, &dir.join("extra-dir-empty"), object_hash)?
            .add_entry_from_path(&dir, &dir.join("extra-dir").join("symlink-to-extra"), object_hash)?;

        let tee_read = TeeToMemory {
            read: stream.into_read(),
            write: Default::default(),
        };
        let copy = tee_read.write.clone();
        let mut paths_and_modes = Vec::new();
        let mut stream = gix_worktree_stream::Stream::from_read(tee_read);

        while let Some(mut entry) = stream.next_entry().expect("entry retrieval does not fail") {
            paths_and_modes.push((entry.relative_path().to_owned(), entry.mode.kind(), entry.id));
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf).expect("stream can always be read");
            if !buf.is_empty() && entry.mode.is_blob() {
                if entry.relative_path().contains_str("extra") {
                    assert!(
                        buf.find_byte(b'\r').is_none(),
                        "extra-files are not processed in any way"
                    );
                } else if !entry.relative_path().contains_str("big") {
                    assert!(
                        buf.find_byte(b'\r').is_some(),
                        "'{}' did not contain a carriage return as sign of having been filtered",
                        buf.as_bstr()
                    );
                    if entry.relative_path().ends_with_str(b"streamed") {
                        assert_eq!(buf.as_bstr(), "➡streamed-by-driver\r\n");
                    }
                }
            }
        }

        assert_eq!(
            paths_and_modes,
            &[
                (
                    ".gitattributes".into(),
                    EntryKind::Blob,
                    hex_to_id(
                        "45c160c35c17ad264b96431cceb9793160396e99",
                        "e34ae8e2c517230c6c613202a955b5f2095567830de5c3bb7081d72f1af393dd"
                    )
                ),
                (
                    "a".into(),
                    EntryKind::Blob,
                    hex_to_id(
                        "45b983be36b73c0788dc9cbcb76cbb80fc7bb057",
                        "96c18f0297e38d01f4b2dacddea4259aea6b2961eb0822bd2c0c3f6029030045"
                    )
                ),
                (
                    "bigfile".into(),
                    EntryKind::Blob,
                    hex_to_id(
                        "4995fde49ed64e043977e22539f66a0d372dd129",
                        "527a170c0c29520af58bf61a383c4e99a7a7c63c0475cbb630cf98d38d6c0dce"
                    )
                ),
                (
                    "symlink-to-a".into(),
                    EntryKind::Link,
                    hex_to_id(
                        "2e65efe2a145dda7ee51d1741299f848e5bf752e",
                        "eb337bcee2061c5313c9a1392116b6c76039e9e30d71467ae359b36277e17dc7"
                    )
                ),
                (
                    "dir/.gitattributes".into(),
                    EntryKind::Blob,
                    hex_to_id(
                        "81b9a375276405703e05be6cecf0fc1c8b8eed64",
                        "4b74474eca46ad7e1807afe8855e7a511d132b314d3d0f481bb55a331b249fb7"
                    )
                ),
                (
                    "dir/b".into(),
                    EntryKind::Blob,
                    hex_to_id(
                        "ab4a98190cf776b43cb0fe57cef231fb93fd07e6",
                        "cf84f9ae227fa5c481dacd97f6d6506de727dabd30377c6907f623ef1192637a"
                    )
                ),
                (
                    "dir/subdir/exe".into(),
                    EntryKind::BlobExecutable,
                    hex_to_id(
                        "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391",
                        "473a0f4c3be8a93681a267e3b1e9a7dcda1185436fe141f7749120a303721813"
                    )
                ),
                (
                    "dir/subdir/streamed".into(),
                    EntryKind::Blob,
                    hex_to_id(
                        "08991f58f4de5d85b61c0f87f3ac053c79d0e739",
                        "8c13ad7df9686daf00357f34700922485802cb0be33e684ec5171f7d0d8a84fd"
                    )
                ),
                ("extra-file".into(), EntryKind::Blob, object_hash.null()),
                ("extra-bigfile".into(), EntryKind::Blob, object_hash.null()),
                (
                    "extra-exe".into(),
                    if cfg!(windows) {
                        EntryKind::Blob
                    } else {
                        EntryKind::BlobExecutable
                    },
                    object_hash.null()
                ),
                ("extra-dir-empty".into(), EntryKind::Tree, object_hash.null()),
                ("extra-dir/symlink-to-extra".into(), EntryKind::Link, object_hash.null())
            ]
        );

        #[cfg(target_pointer_width = "64")]
        let expected_buffer_length: usize = match gix_testtools::object_hash() {
            gix_hash::Kind::Sha1 => 320302,
            gix_hash::Kind::Sha256 => 320458,
            _ => unimplemented!(),
        };
        #[cfg(target_pointer_width = "32")]
        let expected_buffer_length: usize = match gix_testtools::object_hash() {
            gix_hash::Kind::Sha1 => 320198,
            gix_hash::Kind::Sha256 => todo!("let the test fail on CI and add the value here"),
            _ => unimplemented!(),
        };

        assert_eq!(
            copy.lock().len(),
            expected_buffer_length,
            "keep track of file size changes of the streaming format"
        );

        let mut copied_stream =
            gix_worktree_stream::Stream::from_read(std::io::Cursor::new(copy.lock().as_bytes().to_owned()));
        let mut copied_paths_and_modes = Vec::new();
        let mut buf = Vec::new();
        while let Some(mut entry) = copied_stream.next_entry().expect("entry retrieval does not fail") {
            copied_paths_and_modes.push((entry.relative_path().to_owned(), entry.mode.kind(), entry.id));
            buf.clear();
            entry.read_to_end(&mut buf).expect("stream can always be read");
        }
        assert_eq!(
            copied_paths_and_modes, paths_and_modes,
            "a stream copy yields exactly the same result"
        );
        Ok(())
    }

    #[test]
    fn can_drop_entry_without_reading_it() -> gix_testtools::Result {
        let (_dir, head_tree, odb, mut cache) = basic()?;
        let mut stream = gix_worktree_stream::from_tree(
            head_tree,
            odb.clone(),
            mutating_pipeline(false),
            move |rela_path, mode, attrs| {
                cache
                    .at_entry(rela_path, Some(mode.into()), &odb)
                    .map(|entry| entry.matching_attributes(attrs))
                    .map(|_| ())
            },
        );

        drop(stream.next_entry().expect("entry retrieval does not fail"));
        Ok(())
    }

    fn basic() -> gix_testtools::Result<(PathBuf, gix_hash::ObjectId, gix_odb::HandleArc, gix_worktree::Stack)> {
        let dir = gix_testtools::scripted_fixture_read_only("basic.sh")?;

        let head = {
            let hex = std::fs::read(dir.join("head.hex"))?;
            gix_hash::ObjectId::from_hex(hex.trim())?
        };
        let odb = gix_odb::at_opts(
            dir.join(".git").join("objects"),
            Vec::new(),
            gix_odb::store::init::Options {
                object_hash: gix_testtools::object_hash(),
                ..Default::default()
            },
        )?;

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

    fn mutating_pipeline(driver: bool) -> gix_filter::Pipeline {
        gix_filter::Pipeline::new(
            Default::default(),
            gix_filter::pipeline::Options {
                drivers: if driver { vec![driver_with_process()] } else { vec![] },
                eol_config: gix_filter::eol::Configuration {
                    auto_crlf: gix_filter::eol::AutoCrlf::Enabled,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
    }

    pub(crate) fn driver_with_process() -> gix_filter::Driver {
        let mut exe = DRIVER.to_string_lossy().into_owned();
        if cfg!(windows) {
            exe = exe.replace('\\', "/");
        }
        gix_filter::Driver {
            name: "arrow".into(),
            clean: None,
            smudge: None,
            process: Some((exe + " process").into()),
            required: true,
        }
    }

    static DRIVER: LazyLock<PathBuf> =
        LazyLock::new(|| gix_testtools::build_example_for_test("gix-filter", "arrow", env!("CARGO_TARGET_TMPDIR")));

    struct TeeToMemory<R> {
        read: R,
        write: Arc<parking_lot::Mutex<Vec<u8>>>,
    }

    impl<R> std::io::Read for TeeToMemory<R>
    where
        R: std::io::Read,
    {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let nb = self.read.read(buf)?;
            self.write.lock().write_all(&buf[..nb])?;
            Ok(nb)
        }
    }
}
