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

            index
                .write(gix_index::write::Options::default())
                .map_err(|err| err.into_error())?;
            assert!(index.checksum().is_some(), "checksum is adjusted after writing");
            assert!(index.path().is_file());
            assert_eq!(index.version(), expected_version);

            index.verify_integrity().map_err(|err| err.into_error())?;
        }
        Ok(())
    }
}
