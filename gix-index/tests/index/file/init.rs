mod at_or_new {
    use crate::Fixture::Generated;

    #[test]
    fn opens_existing() {
        gix_index::File::at_or_default(
            Generated("v4_more_files_IEOT").to_path(),
            gix_testtools::object_hash(),
            false,
            Default::default(),
        )
        .expect("file exists and can be opened");
    }

    #[test]
    fn create_empty_in_memory_state_if_file_does_not_exist() {
        let index = gix_index::File::at_or_default(
            "__definitely no file that exists ever__",
            gix_testtools::object_hash(),
            false,
            Default::default(),
        )
        .expect("file is defaulting to a new one");
        assert!(!index.path().is_file(), "the file wasn't created yet");
        assert_eq!(
            index.object_hash(),
            gix_testtools::object_hash(),
            "object hash is respected"
        );
        assert_eq!(index.entries().len(), 0, "index is empty");
    }
}

mod from_state {
    use gix_index::Version::{V2, V3};

    use crate::Fixture::*;

    #[test]
    fn writes_data_to_disk_and_is_a_valid_index() -> gix_testtools::Result {
        let fixtures = [
            (Loose("extended-flags"), V3),
            (Generated("v2"), V2),
            (Generated("v2_empty"), V2),
            (Generated("v2_more_files"), V2),
            (Generated("v2_all_file_kinds"), V2),
            (Generated("v4_more_files_IEOT"), V2),
        ];

        for (fixture, expected_version) in fixtures {
            // Loose fixtures are pre-created and only exist as SHA-1 variants.
            if gix_testtools::object_hash() != gix_hash::Kind::Sha1 && matches!(fixture, Loose(_)) {
                continue;
            }

            let tmp = gix_testtools::tempfile::TempDir::new()?;
            let new_index_path = tmp.path().join(fixture.to_name());
            assert!(!new_index_path.exists());

            let index = gix_index::File::at(
                fixture.to_path(),
                gix_testtools::object_hash(),
                false,
                Default::default(),
            )?;
            let mut index = gix_index::File::from_state(index.into(), new_index_path.clone());
            assert!(index.checksum().is_none());
            assert_eq!(index.path(), new_index_path);

            index.write(gix_index::write::Options::default())?;
            assert!(index.checksum().is_some(), "checksum is adjusted after writing");
            assert!(index.path().is_file());
            assert_eq!(index.version(), expected_version);

            index.verify_integrity()?;
        }
        Ok(())
    }
}

#[test]
fn truncated_files_return_errors_before_checksum_verification() -> gix_testtools::Result {
    let directory = gix_testtools::tempfile::TempDir::new()?;
    let path = directory.path().join("index");
    for object_hash in [gix_hash::Kind::Sha1, gix_hash::Kind::Sha256] {
        let mut empty_index = b"DIRC\0\0\0\x02\0\0\0\0".to_vec();
        empty_index.resize(empty_index.len() + object_hash.len_in_bytes(), 0);
        for data in std::iter::once(&b"broken"[..]).chain((0..empty_index.len()).map(|length| &empty_index[..length])) {
            std::fs::write(&path, data)?;
            for skip_hash in [false, true] {
                let error = gix_index::File::at(&path, object_hash, skip_hash, Default::default())
                    .expect_err("a truncated index must return an error with either checksum policy");
                if !data.is_empty() {
                    assert!(
                        matches!(
                            error,
                            gix_index::file::init::Error::Decode(gix_index::decode::Error::Header(
                                gix_index::decode::header::Error::Corrupt(_)
                            ))
                        ),
                        "a short mapped index is diagnosed as a corrupt header: {error}"
                    );
                }
                assert!(
                    gix_index::File::at_or_default(&path, object_hash, skip_hash, Default::default()).is_err(),
                    "a corrupt existing index must not be treated as a missing index"
                );
            }
        }
        std::fs::write(&path, empty_index)?;
        assert!(
            gix_index::File::at(&path, object_hash, false, Default::default())?
                .entries()
                .is_empty(),
            "a complete header and null checksum still represent a valid empty index"
        );
    }
    Ok(())
}
