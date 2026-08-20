use gix_hash::{ChangeId, Prefix};

use crate::hex_to_id;

macro_rules! assert_text_eq {
    ($value:expr, $text:expr) => {{
        let value = $value;
        let text = $text;
        let owned = text.to_owned();
        assert!(value == text, "the value compares with a string literal");
        assert!(text == value, "string-literal comparison is symmetric");
        assert!(&value == text, "a borrowed value compares with str");
        assert!(text == &value, "str comparison with a borrowed value is symmetric");
        assert!(value == owned, "the value compares with String");
        assert!(owned == value, "String comparison is symmetric");
    }};
}

macro_rules! assert_text_ne {
    ($value:expr, $text:expr) => {{
        let value = $value;
        let text = $text;
        assert!(value != text, "different text does not compare equal");
        assert!(text != value, "inequality is symmetric");
    }};
}

fn compare_all(object_hex: &str, reverse_hex: &str) {
    let id = hex_to_id(object_hex);
    let object_hex_upper = object_hex.to_ascii_uppercase();
    assert_text_eq!(id, object_hex);
    assert_text_eq!(id, object_hex_upper.as_str());
    assert_text_ne!(id, &object_hex[..object_hex.len() - 1]);
    let invalid_object_hex = format!("g{}", &object_hex[1..]);
    assert_text_ne!(id, invalid_object_hex.as_str());

    let borrowed = id.as_ref();
    assert!(borrowed == object_hex, "oid compares with str");
    assert!(object_hex == borrowed, "str comparison with oid is symmetric");
    assert!(borrowed == object_hex_upper, "oid compares with String");
    assert!(object_hex_upper == borrowed, "String comparison with oid is symmetric");
    assert!(
        borrowed != &object_hex[..object_hex.len() - 1],
        "oid requires the exact length"
    );
    assert!(borrowed != invalid_object_hex.as_str(), "oid rejects invalid hex");

    let change_id = ChangeId::from(id);
    let reverse_hex_upper = reverse_hex.to_ascii_uppercase();
    assert_text_eq!(change_id, reverse_hex);
    assert_text_eq!(change_id, reverse_hex_upper.as_str());
    assert_text_ne!(change_id, &reverse_hex[..reverse_hex.len() - 1]);
    let invalid_reverse_hex = format!("j{}", &reverse_hex[1..]);
    assert_text_ne!(change_id, invalid_reverse_hex.as_str());

    let prefix_len = 7;
    let prefix = Prefix::new(&id, prefix_len).expect("the requested prefix length is valid");
    let object_prefix = &object_hex[..prefix_len];
    let object_prefix_upper = object_prefix.to_ascii_uppercase();
    assert_text_eq!(prefix, object_prefix);
    assert_text_eq!(prefix, object_prefix_upper.as_str());
    assert_text_ne!(prefix, &object_hex[..prefix_len + 1]);
    assert_text_ne!(prefix, "abcdefg");

    assert_text_eq!(id.to_hex_with_len(prefix_len), object_prefix);
    assert_text_eq!(id.to_hex_with_len(prefix_len), object_prefix_upper.as_str());
    assert_text_ne!(id.to_hex_with_len(prefix_len), &object_hex[..prefix_len + 1]);
    assert_text_ne!(id.to_hex_with_len(prefix_len), "abcdefg");

    let reverse_prefix = &reverse_hex[..prefix_len];
    let reverse_prefix_upper = reverse_prefix.to_ascii_uppercase();
    assert_text_eq!(change_id.to_reverse_hex_with_len(prefix_len), reverse_prefix);
    assert_text_eq!(
        change_id.to_reverse_hex_with_len(prefix_len),
        reverse_prefix_upper.as_str()
    );
    assert_text_ne!(
        change_id.to_reverse_hex_with_len(prefix_len),
        &reverse_hex[..prefix_len + 1]
    );
    assert_text_ne!(change_id.to_reverse_hex_with_len(prefix_len), "abcdefg");
}

#[test]
fn compares_sha1_with_text() {
    compare_all(
        "0123456789abcdef0123456789abcdef01234567",
        "zyxwvutsrqponmlkzyxwvutsrqponmlkzyxwvuts",
    );
}

#[test]
#[cfg(feature = "sha256")]
fn compares_sha256_with_text() {
    compare_all(
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "zyxwvutsrqponmlkzyxwvutsrqponmlkzyxwvutsrqponmlkzyxwvutsrqponmlk",
    );
}
