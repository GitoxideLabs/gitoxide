use crate::fixture_path;

#[test]
fn encode_decode_roundtrip() -> crate::Result {
    let buf = std::fs::read(fixture_path(
        "objects/pack/pack-11fdfa9e156ab73caae3b6da867192221f2089c2.pack",
    ))?;
    let expected_encoded_header = &buf[..12];
    let (version, num_objects) = gix_pack::data::header::decode(expected_encoded_header.try_into()?)?;
    let actual_encoded_header = gix_pack::data::header::encode(version, num_objects);
    assert_eq!(actual_encoded_header, expected_encoded_header);
    Ok(())
}
