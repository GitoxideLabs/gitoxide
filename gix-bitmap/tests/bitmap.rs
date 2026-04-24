mod fuzzed {
    #[test]
    fn ewah_artifacts_run_fuzzer() {
        for path in artifact_paths("ewah") {
            let data = std::fs::read(path).expect("artifact is readable");
            let _ = gix_bitmap::ewah::decode(&data);
        }
    }

    #[test]
    fn runaway_run_length_is_rejected() {
        let (bitmap, rest) = gix_bitmap::ewah::decode(include_bytes!(
            "../fuzz/artifacts/ewah/slow-unit-ac817962d1a6c123d4d1f73860f5b779423ed171"
        ))
        .expect("fixture must decode");

        assert!(rest.is_empty(), "fixture should be fully consumed");
        assert_eq!(
            bitmap.for_each_set_bit(|_| Some(())),
            None,
            "impossible run lengths must be rejected instead of iterating unboundedly"
        );
    }

    #[test]
    fn non_zero_padding_bits_in_last_literal_word_are_rejected() {
        let bitmap = gix_bitmap::ewah::Vec::from_bits(&[false]).expect("small test fixtures must fit into u32");
        let mut data = Vec::new();
        bitmap
            .write_to(&mut data)
            .expect("writing a valid test fixture to bytes must succeed");
        let header_size = 4 + 4;
        // Skip the fixed-size header and the RLW word, then flip bit 33 in the first literal word
        // so the bitmap stays logically empty while the serialized padding becomes invalid.
        let literal_word_offset = header_size + 8;
        let mut literal_word = u64::from_be_bytes(
            data[literal_word_offset..literal_word_offset + 8]
                .try_into()
                .expect("literal word"),
        );
        literal_word |= 1u64 << 33;
        data[literal_word_offset..literal_word_offset + 8].copy_from_slice(&literal_word.to_be_bytes());

        let (bitmap, rest) = gix_bitmap::ewah::decode(&data).expect("fixture must decode");

        assert!(rest.is_empty(), "fixture should be fully consumed");
        assert_eq!(
            bitmap.for_each_set_bit(|_| Some(())),
            None,
            "set bits outside the declared bit length must be rejected"
        );
    }

    #[test]
    fn literal_only_bitmaps_preserve_all_set_bits() {
        for bits in [
            vec![],
            vec![false],
            vec![true],
            vec![true, false, true, false, true],
            (0..64).map(|idx| idx % 3 == 0).collect(),
            (0..65).map(|idx| idx == 0 || idx == 63 || idx == 64).collect(),
            (0..130).map(|idx| idx == 1 || idx == 64 || idx == 129).collect(),
        ] {
            let bitmap = gix_bitmap::ewah::Vec::from_bits(&bits).expect("small test fixtures must fit into u32");
            let mut encoded = Vec::new();
            let expected: Vec<_> = bits
                .iter()
                .enumerate()
                .filter_map(|(idx, bit)| bit.then_some(idx))
                .collect();

            bitmap
                .write_to(&mut encoded)
                .expect("writing a valid test fixture to bytes must succeed");
            let (bitmap, rest) = gix_bitmap::ewah::decode(&encoded).expect("serialized test fixture must decode");
            let mut actual = Vec::new();

            assert!(rest.is_empty(), "serialized test fixture should be fully consumed");
            assert_eq!(
                bitmap.num_bits(),
                bits.len(),
                "fixture should preserve the declared bit length"
            );
            assert_eq!(
                bitmap.for_each_set_bit(|idx| {
                    actual.push(idx);
                    Some(())
                }),
                Some(()),
                "well-formed literal-only bitmaps must iterate successfully"
            );
            assert_eq!(
                actual, expected,
                "iteration should report exactly the set bits from the source bitmap"
            );
        }
    }

    #[test]
    fn zero_padding_bits_in_last_literal_word_are_accepted() {
        let bitmap =
            gix_bitmap::ewah::Vec::from_bits(&[true, false, true]).expect("small test fixtures must fit into u32");
        let mut actual = Vec::new();

        assert_eq!(
            bitmap.for_each_set_bit(|idx| {
                actual.push(idx);
                Some(())
            }),
            Some(()),
            "unused bits in the last partial word are valid when they are zero"
        );
        assert_eq!(actual, vec![0, 2], "iteration should ignore zero padding bits");
    }

    fn artifact_paths(target: &str) -> Vec<std::path::PathBuf> {
        std::fs::read_dir(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("fuzz/artifacts")
                .join(target),
        )
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect()
    }
}
