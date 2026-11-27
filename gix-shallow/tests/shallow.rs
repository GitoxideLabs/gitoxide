mod read {
    use gix_error::TestResult;
    use gix_hash::{Kind, ObjectId};

    #[test]
    fn splits_long_lines_at_git_record_boundaries() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("shallow");

        for hash in [Kind::Sha1, Kind::Sha256] {
            let first = vec![b'1'; hash.len_in_hex()];
            let second = vec![b'2'; hash.len_in_hex()];
            let mut input = second.clone();
            input.resize(1023, b'x');
            input.extend_from_slice(&first);
            std::fs::write(&path, input)?;

            let commits = gix_shallow::read(&path, hash)?
                .expect("both chunks start with a valid object ID")
                .into_iter()
                .collect::<Vec<_>>();
            assert_eq!(
                commits,
                [ObjectId::from_hex(&first)?, ObjectId::from_hex(&second)?],
                "the continuation is parsed as another record even without a final newline"
            );
        }
        Ok(())
    }

    #[test]
    fn counts_the_newline_toward_the_record_length() -> TestResult {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("shallow");
        let mut input = vec![b'1'; Kind::Sha1.len_in_hex()];
        input.resize(1022, b'x');
        input.push(b'\n');
        std::fs::write(&path, &input)?;
        assert!(
            gix_shallow::read(&path, Kind::Sha1)?.is_some(),
            "the newline fits in the same record as the object ID"
        );

        input[1022] = b'x';
        input.push(b'\n');
        std::fs::write(&path, &input)?;
        assert!(
            gix_shallow::read(&path, Kind::Sha1).is_err(),
            "a newline beyond the record boundary is a separate, invalid record"
        );
        Ok(())
    }
}
