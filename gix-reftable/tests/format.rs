#[test]
fn git_offset_varints_round_trip_boundaries() {
    for value in [0, 1, 127, 128, 255, 256, 16_383, 16_384, u32::MAX as u64, u64::MAX] {
        let mut encoded = Vec::new();
        gix_reftable::format::varint::encode(value, &mut encoded);
        let (actual, consumed) = gix_reftable::format::varint::decode(&encoded).expect("encoded values decode");
        assert_eq!(actual, value, "the decoded integer is unchanged");
        assert_eq!(consumed, encoded.len(), "the decoder consumes exactly one varint");
    }
}

#[test]
fn malformed_varints_fail_without_overflow() {
    assert!(
        gix_reftable::format::varint::decode(&[]).is_err(),
        "an empty buffer does not contain a varint"
    );
    assert!(
        gix_reftable::format::varint::decode(&[0x80]).is_err(),
        "an unterminated continuation byte is rejected"
    );
    assert!(
        gix_reftable::format::varint::decode(&[0xff; 32]).is_err(),
        "an overlong varint is rejected before overflowing"
    );
}

#[test]
fn file_parse_errors_retain_the_table_path() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let table_path = temp.path().join("corrupt.ref");
    std::fs::write(&table_path, b"not a reftable")?;

    let error = gix_reftable::Table::read(&table_path, gix_reftable::Limits::default())
        .expect_err("malformed on-disk table is rejected");
    assert!(
        matches!(&error, gix_reftable::Error::Parse { path, .. } if path == &table_path),
        "the outer error identifies the corrupt member path: {error:?}"
    );
    assert!(
        std::error::Error::source(&error).is_some(),
        "the path context retains the underlying format error"
    );
    Ok(())
}
