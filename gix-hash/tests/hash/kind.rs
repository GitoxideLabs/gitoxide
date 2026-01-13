use gix_hash::{Kind, ObjectId};
use std::str::FromStr;

#[test]
fn from_str() {
    #[cfg(feature = "sha1")]
    for input in ["sha1", "SHA1", "SHA-1"] {
        assert_eq!(Kind::from_str(input).unwrap(), Kind::Sha1, "{input}");
    }
    #[cfg(feature = "sha256")]
    for input in ["sha256", "SHA256", "SHA-256"] {
        assert_eq!(Kind::from_str(input).unwrap(), Kind::Sha256, "{input}");
    }
}

#[test]
fn display() {
    #[cfg(feature = "sha1")]
    assert_eq!(
        Kind::Sha1.to_string(),
        "sha1",
        "Something that is compatible to core.objectFormat"
    );
    #[cfg(feature = "sha256")]
    assert_eq!(
        Kind::Sha256.to_string(),
        "sha256",
        "Something that is compatible to core.objectFormat"
    );
}

mod from_hex_len {
    use gix_hash::Kind;

    #[test]
    fn some_sha1() {
        assert_eq!(Kind::from_hex_len(0), Some(Kind::Sha1));
        assert_eq!(Kind::from_hex_len(10), Some(Kind::Sha1));
        assert_eq!(Kind::from_hex_len(20), Some(Kind::Sha1));
        assert_eq!(Kind::from_hex_len(40), Some(Kind::Sha1));
        #[cfg(feature = "sha256")]
        assert_eq!(Kind::from_hex_len(64), Some(Kind::Sha256));
        #[cfg(feature = "sha256")]
        assert_eq!(Kind::from_hex_len(41), Some(Kind::Sha256));
    }

    #[test]
    fn none_if_there_is_no_exact_fit() {
        assert_eq!(Kind::from_hex_len(65), None);
    }
}

#[test]
fn empty_blob() {
    let sha1 = Kind::Sha1;
    assert_eq!(sha1.empty_blob(), ObjectId::empty_blob(sha1));
}

#[test]
fn empty_tree() {
    let sha1 = Kind::Sha1;
    assert_eq!(sha1.empty_tree(), ObjectId::empty_tree(sha1));
}

#[test]
#[cfg(all(feature = "sha1", not(feature = "sha256")))]
fn shortest_sha1() {
    let shortest = Kind::shortest();
    assert_eq!(shortest, Kind::Sha1);
}

#[test]
#[cfg(all(not(feature = "sha1"), feature = "sha256"))]
fn shortest_sha256() {
    let shortest = Kind::shortest();
    assert_eq!(shortest, Kind::Sha256);
}

#[test]
#[cfg(all(feature = "sha1", feature = "sha256"))]
fn shortest_sha1_and_sha256() {
    let shortest = Kind::shortest();
    assert_eq!(shortest, Kind::Sha1);
}

#[test]
#[cfg(all(feature = "sha1", not(feature = "sha256")))]
fn longest_sha1() {
    let longest = Kind::longest();
    assert_eq!(longest, Kind::Sha1);
}

#[test]
#[cfg(all(not(feature = "sha1"), feature = "sha256"))]
fn longest_sha256() {
    let longest = Kind::longest();
    assert_eq!(longest, Kind::Sha256);
}

#[test]
#[cfg(all(feature = "sha1", feature = "sha256"))]
fn longest_sha1_and_sha256() {
    let longest = Kind::longest();
    assert_eq!(longest, Kind::Sha256);
}

#[test]
#[cfg(all(feature = "sha1", not(feature = "sha256")))]
fn all() {
    assert_eq!(Kind::all(), &[Kind::Sha1]);
}

#[test]
#[cfg(all(not(feature = "sha1"), feature = "sha256"))]
fn all() {
    assert_eq!(Kind::all(), &[Kind::Sha256]);
}

#[test]
#[cfg(all(feature = "sha1", feature = "sha256"))]
fn all() {
    assert_eq!(Kind::all(), &[Kind::Sha1, Kind::Sha256]);
}
