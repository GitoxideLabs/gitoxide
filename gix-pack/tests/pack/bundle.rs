mod locate {
    use bstr::ByteSlice;
    use gix_features::zlib;
    use gix_object::Kind;
    use gix_odb::pack;

    use crate::{SMALL_PACK_INDEX, fixture_path, hex_to_id};

    fn locate<'a>(hex_id: &str, out: &'a mut Vec<u8>) -> gix_object::Data<'a> {
        let bundle = pack::Bundle::at(fixture_path(SMALL_PACK_INDEX), gix_hash::Kind::Sha1).expect("pack and idx");
        bundle
            .find(
                &hex_to_id(hex_id),
                out,
                &mut zlib::Inflate::default(),
                &mut pack::cache::Never,
            )
            .expect("read success")
            .expect("id present")
            .0
    }

    mod locate_and_verify {
        use gix_features::zlib;
        use gix_odb::pack;

        use crate::{PACKS_AND_INDICES, fixture_path};

        #[test]
        fn all() -> Result<(), Box<dyn std::error::Error>> {
            for (index_path, data_path) in PACKS_AND_INDICES {
                // both paths are equivalent
                pack::Bundle::at(fixture_path(index_path), gix_hash::Kind::Sha1)?;
                let bundle = pack::Bundle::at(fixture_path(data_path), gix_hash::Kind::Sha1)?;

                let mut buf = Vec::new();
                for entry in bundle.index.iter() {
                    let (obj, _location) = bundle
                        .find(
                            &entry.oid,
                            &mut buf,
                            &mut zlib::Inflate::default(),
                            &mut pack::cache::Never,
                        )?
                        .expect("id present");
                    obj.verify_checksum(&entry.oid)?;
                }
            }
            Ok(())
        }
    }

    #[test]
    fn blob() -> Result<(), Box<dyn std::error::Error>> {
        let mut out = Vec::new();
        let obj = locate("bd46bb3f5bb4ca5431770c4fde0735fb89d382f3", &mut out);

        assert_eq!(
            obj.data.as_bstr(),
            b"GitPython is a python library used to interact with Git repositories.\n\nHi there\n".as_bstr()
        );
        assert_eq!(obj.kind, Kind::Blob);
        let object = obj.decode()?;
        assert_eq!(object.kind(), Kind::Blob);
        assert_eq!(object.as_blob().expect("blob").data, obj.data);
        Ok(())
    }

    #[test]
    fn tree() -> Result<(), Box<dyn std::error::Error>> {
        let mut out = Vec::new();
        let obj = locate("e90926b07092bccb7bf7da445fae6ffdfacf3eae", &mut out);

        assert_eq!(obj.kind, Kind::Tree);
        assert_eq!(obj.decode()?.kind(), Kind::Tree);
        Ok(())
    }

    #[test]
    fn commit() -> Result<(), Box<dyn std::error::Error>> {
        let mut out = Vec::new();
        let obj = locate("779c5451ba9fe210ffd1f55db202e55f51acecac", &mut out);

        assert_eq!(obj.kind, Kind::Commit);
        assert_eq!(obj.decode()?.kind(), Kind::Commit);
        Ok(())
    }
}

#[cfg(all(not(feature = "wasm"), feature = "streaming-input"))]
mod write_to_directory {
    use std::{fs, path::Path, sync::atomic::AtomicBool};

    use gix_features::progress;
    use gix_odb::pack;
    use gix_testtools::tempfile::TempDir;

    use crate::{SMALL_PACK, SMALL_PACK_INDEX, fixture_path};

    fn expected_outcome() -> Result<pack::bundle::write::Outcome, Box<dyn std::error::Error>> {
        Ok(pack::bundle::write::Outcome {
            index: pack::index::write::Outcome {
                index_version: pack::index::Version::V2,
                index_hash: gix_hash::ObjectId::from_hex(b"544a7204a55f6e9cacccf8f6e191ea8f83575de3")?,
                data_hash: gix_hash::ObjectId::from_hex(b"0f3ea84cd1bba10c2a03d736a460635082833e59")?,
                num_objects: 42,
            },
            pack_version: pack::data::Version::V2,
            index_path: None,
            data_path: None,
            keep_path: None,
            object_hash: gix_hash::Kind::Sha1,
        })
    }

    #[test]
    fn without_providing_one() -> Result<(), Box<dyn std::error::Error>> {
        let res = write_pack(None::<&Path>, SMALL_PACK)?;
        assert_eq!(res, expected_outcome()?);
        assert_eq!(
            res.index.index_hash,
            pack::index::File::at(fixture_path(SMALL_PACK_INDEX), gix_hash::Kind::Sha1)?.index_checksum()
        );
        assert!(res.to_bundle().is_none());
        Ok(())
    }

    #[test]
    fn given_a_directory() -> Result<(), Box<dyn std::error::Error>> {
        let dir = TempDir::new()?;
        let mut res = write_pack(Some(&dir), SMALL_PACK)?;
        let (index_path, data_path, keep_path) = (res.index_path.take(), res.data_path.take(), res.keep_path.take());
        assert_eq!(res, expected_outcome()?);
        let mut sorted_entries = fs::read_dir(&dir)?.filter_map(Result::ok).collect::<Vec<_>>();
        sorted_entries.sort_by_key(fs::DirEntry::file_name);
        assert_eq!(
            sorted_entries.len(),
            3,
            "we want a pack and the corresponding index and the keep file"
        );

        let pack_hash = res.index.data_hash.to_hex();
        assert_eq!(file_name(&sorted_entries[0]), format!("pack-{pack_hash}.idx"));
        assert_eq!(Some(sorted_entries[0].path()), index_path);
        assert_eq!(file_name(&sorted_entries[1]), format!("pack-{pack_hash}.keep"));
        assert_eq!(Some(sorted_entries[1].path()), keep_path);
        assert_eq!(file_name(&sorted_entries[2]), format!("pack-{pack_hash}.pack"));
        assert_eq!(Some(sorted_entries[2].path()), data_path);

        res.index_path = index_path;
        assert!(res.to_bundle().transpose()?.is_some());
        Ok(())
    }

    fn file_name(entry: &fs::DirEntry) -> String {
        entry.path().file_name().unwrap().to_str().unwrap().to_owned()
    }

    fn write_pack(
        directory: Option<impl AsRef<Path>>,
        pack_file: &str,
    ) -> Result<pack::bundle::write::Outcome, Box<dyn std::error::Error>> {
        let pack_file = fs::File::open(fixture_path(pack_file))?;
        static SHOULD_INTERRUPT: AtomicBool = AtomicBool::new(false);
        pack::Bundle::write_to_directory_eagerly(
            Box::new(pack_file),
            None,
            directory,
            &mut progress::Discard,
            &SHOULD_INTERRUPT,
            None::<gix_object::find::Never>,
            pack::bundle::write::Options {
                thread_limit: None,
                iteration_mode: pack::data::input::Mode::Verify,
                index_version: pack::index::Version::V2,
                object_hash: gix_hash::Kind::Sha1,
            },
        )
        .map_err(Into::into)
    }
}
