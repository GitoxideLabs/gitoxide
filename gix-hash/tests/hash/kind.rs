use gix_hash::{Kind, ObjectId};

mod from_hex_len {
    use gix_hash::Kind;

    #[test]
    fn some_sha1() {
        assert_eq!(Kind::from_hex_len(0), Some(Kind::Sha1));
        assert_eq!(Kind::from_hex_len(10), Some(Kind::Sha1));
        assert_eq!(Kind::from_hex_len(20), Some(Kind::Sha1));
        assert_eq!(Kind::from_hex_len(40), Some(Kind::Sha1));
    }

    #[test]
    fn none_if_there_is_no_fit() {
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
