use crate::Result;
use crate::fixture_path;

#[test]
fn unsupported_version_is_not_invalid_input_or_corruption() {
    let mut header = gix_pack::data::header::encode(gix_pack::data::Version::V2, 0);
    header[4..8].copy_from_slice(&4u32.to_be_bytes());
    let err = gix_pack::data::header::decode(&header).expect_err("version 4 requires another implementation");
    assert!(err.is_unsupported(), "the caller can switch pack-reading strategies");
    assert!(
        !err.is_validation() && !err.is_corrupted(),
        "an unknown version is not malformed data"
    );
    assert_eq!(
        err.to_string(),
        "Unsupported pack version: 4",
        "the diagnostic is unchanged"
    );
}

#[test]
fn encode_decode_roundtrip() -> Result {
    let buf = std::fs::read(fixture_path(
        "objects/pack/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.pack",
    ))?;
    let expected_encoded_header = &buf[..gix_pack::data::header::SIZE];
    let (version, num_objects) = gix_pack::data::header::decode(expected_encoded_header.try_into()?)?;
    let actual_encoded_header = gix_pack::data::header::encode(version, num_objects);
    assert_eq!(actual_encoded_header, expected_encoded_header);
    Ok(())
}
