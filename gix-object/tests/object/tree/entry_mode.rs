use gix_object::tree::{EntryKind, EntryMode};

#[test]
fn size_in_bytes() {
    assert_eq!(
        std::mem::size_of::<EntryMode>(),
        4,
        "it should not change without notice"
    );
}

#[test]
fn is_methods() {
    fn mode(kind: EntryKind) -> EntryMode {
        kind.into()
    }

    assert!(mode(EntryKind::Blob).is_blob());
    assert!(EntryMode::from_bytes(b"100645").unwrap().is_blob());
    assert_eq!(EntryMode::from_bytes(b"100645").unwrap().kind(), EntryKind::Blob);
    assert!(!EntryMode::from_bytes(b"100675").unwrap().is_executable());
    assert!(EntryMode::from_bytes(b"100700").unwrap().is_executable());
    assert_eq!(
        EntryMode::from_bytes(b"100700").unwrap().kind(),
        EntryKind::BlobExecutable
    );
    assert!(!mode(EntryKind::Blob).is_link());
    assert!(mode(EntryKind::BlobExecutable).is_blob());
    assert!(mode(EntryKind::BlobExecutable).is_executable());
    assert!(mode(EntryKind::Blob).is_blob_or_symlink());
    assert!(mode(EntryKind::BlobExecutable).is_blob_or_symlink());

    assert!(!mode(EntryKind::Link).is_blob());
    assert!(mode(EntryKind::Link).is_link());
    assert!(EntryMode::from_bytes(b"121234").unwrap().is_link());
    assert_eq!(EntryMode::from_bytes(b"121234").unwrap().kind(), EntryKind::Link);
    assert!(mode(EntryKind::Link).is_blob_or_symlink());
    assert!(mode(EntryKind::Tree).is_tree());
    assert!(EntryMode::from_bytes(b"040101").unwrap().is_tree());
    assert_eq!(EntryMode::from_bytes(b"040101").unwrap().kind(), EntryKind::Tree);
    assert!(EntryMode::from_bytes(b"40101").unwrap().is_tree());
    assert_eq!(EntryMode::from_bytes(b"40101").unwrap().kind(), EntryKind::Tree);
    assert!(mode(EntryKind::Commit).is_commit());
    assert!(EntryMode::from_bytes(b"167124").unwrap().is_commit());
    assert_eq!(EntryMode::from_bytes(b"167124").unwrap().kind(), EntryKind::Commit);
    assert_eq!(
        EntryMode::from_bytes(b"000000").unwrap().kind(),
        EntryKind::Commit,
        "commit is really 'anything else' as `kind()` can't fail"
    );
}

#[test]
fn normalized() -> gix_error::TestResult {
    for (raw, kind) in [
        ("40000", EntryKind::Tree),
        ("040000", EntryKind::Tree),
        ("40777", EntryKind::Tree),
        ("040777", EntryKind::Tree),
        ("100644", EntryKind::Blob),
        ("100600", EntryKind::Blob),
        ("100010", EntryKind::Blob),
        ("100755", EntryKind::BlobExecutable),
        ("100700", EntryKind::BlobExecutable),
        ("100100", EntryKind::BlobExecutable),
        ("120000", EntryKind::Link),
        ("120777", EntryKind::Link),
        ("160000", EntryKind::Commit),
        ("167124", EntryKind::Commit),
        ("140000", EntryKind::Commit),
        ("140777", EntryKind::Commit),
        ("170000", EntryKind::Commit),
        ("060000", EntryKind::Commit),
        ("000000", EntryKind::Commit),
    ] {
        let mode = EntryMode::from_bytes(raw.as_bytes()).expect("valid octal mode");
        let normalized = mode.normalized();
        assert_eq!(
            normalized,
            EntryMode::from(kind),
            "{raw}: normalization agrees with kind()"
        );
        assert_eq!(normalized.value(), kind as u16, "{raw}: the numeric mode is canonical");
        assert_eq!(
            normalized.as_bytes(&mut Default::default()),
            kind.as_octal_str(),
            "{raw}: serialization discards noncanonical permissions and tree padding"
        );
        assert_eq!(
            normalized.normalized(),
            normalized,
            "{raw}: normalization is idempotent"
        );
        assert_eq!(
            mode.value(),
            u16::from_str_radix(raw, 8)?,
            "{raw}: normalization leaves the original numeric mode unchanged"
        );
    }
    Ok(())
}

#[test]
fn as_bytes() {
    let mut buf = Default::default();
    for (mode, expected) in [
        (EntryMode::from(EntryKind::Tree), EntryKind::Tree.as_octal_str()),
        (EntryKind::Blob.into(), EntryKind::Blob.as_octal_str()),
        (
            EntryKind::BlobExecutable.into(),
            EntryKind::BlobExecutable.as_octal_str(),
        ),
        (EntryKind::Link.into(), EntryKind::Link.as_octal_str()),
        (EntryKind::Commit.into(), EntryKind::Commit.as_octal_str()),
        (
            EntryMode::from_bytes(b"100744 ".as_ref()).expect("valid"),
            "100744".into(),
        ),
        (
            EntryMode::from_bytes(b"100644 ".as_ref()).expect("valid"),
            "100644".into(),
        ),
        (
            EntryMode::from_bytes(b"040000".as_ref()).expect("valid"),
            "040000".into(),
        ),
        (EntryMode::from_bytes(b"40000".as_ref()).expect("valid"), "40000".into()),
    ] {
        assert_eq!(mode.as_bytes(&mut buf), expected);
    }
}
