use gix_hash::{ChangeId, ObjectId};

use crate::hex_to_id;

#[test]
fn formats_and_parses_exactly_like_jujutsu() -> gix_testtools::Result {
    let object_hex = "0123456789abcdef0123456789abcdef01234567";
    let reverse_hex = "zyxwvutsrqponmlkzyxwvutsrqponmlkzyxwvuts";
    let object_id = hex_to_id(object_hex);
    let change_id = ChangeId::from(object_id);

    assert_eq!(change_id.to_string(), reverse_hex, "Display uses JJ's reverse alphabet");
    assert_eq!(
        change_id.to_reverse_hex().to_string(),
        reverse_hex,
        "explicit formatting matches Display"
    );
    assert_eq!(
        change_id.to_reverse_hex_with_len(17).to_string(),
        &reverse_hex[..17],
        "shortening counts reverse-hex digits"
    );
    assert_eq!(
        change_id.to_reverse_hex_with_len(usize::MAX).to_string(),
        reverse_hex,
        "lengths beyond the ID are clamped"
    );
    assert_eq!(
        ChangeId::from_reverse_hex(reverse_hex.as_bytes())?,
        change_id,
        "reverse hex decodes to the original bytes"
    );
    assert_eq!(
        reverse_hex.to_ascii_uppercase().parse::<ChangeId>()?,
        change_id,
        "JJ accepts uppercase reverse-hex digits"
    );

    let roundtrip: ObjectId = change_id.into();
    assert_eq!(roundtrip, object_id, "conversion does not alter the object hash");
    let from_borrowed: ChangeId = roundtrip.as_ref().into();
    assert_eq!(from_borrowed, change_id, "borrowed object IDs convert symmetrically");
    Ok(())
}

#[test]
#[cfg(feature = "sha256")]
fn formats_sha256_with_the_same_jj_algorithm() -> gix_testtools::Result {
    let object_hex = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let reverse_hex = "zyxwvutsrqponmlkzyxwvutsrqponmlkzyxwvutsrqponmlkzyxwvutsrqponmlk";
    let change_id = ChangeId::from(hex_to_id(object_hex));

    assert_eq!(
        change_id.to_string(),
        reverse_hex,
        "the algorithm is independent of hash kind"
    );
    assert_eq!(
        reverse_hex.parse::<ChangeId>()?,
        change_id,
        "SHA-256 reverse hex round-trips"
    );
    Ok(())
}

#[test]
fn rejects_invalid_reverse_hex() {
    let invalid_character = format!("j{}", "z".repeat(39));
    assert_eq!(
        ChangeId::from_reverse_hex(invalid_character.as_bytes())
            .unwrap_err()
            .to_string(),
        "Invalid character encountered",
        "characters outside JJ's k-z alphabet are rejected"
    );
    assert_eq!(
        ChangeId::from_reverse_hex(b"zzy").unwrap_err().to_string(),
        "A hash sized 3 hexadecimal characters is invalid",
        "full change IDs require a supported object hash length"
    );
}
