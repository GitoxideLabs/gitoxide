use gix_mailmap::{Entry, Snapshot};
use gix_testtools::fixture_bytes;

#[test]
fn try_resolve() {
    let snapshot = Snapshot::from_bytes(&fixture_bytes("typical.txt"));
    assert_eq!(
        snapshot.try_resolve(signature("Foo", "Joe@example.com").to_ref()),
        Some(signature("Joe R. Developer", "joe@example.com")),
        "resolved signatures contain all original fields, and normalize the email as well to match the one that it was looked up with"
    );
    assert_eq!(
        snapshot.try_resolve(signature("Joe", "bugs@example.com").to_ref()),
        Some(signature("Joe R. Developer", "joe@example.com")),
        "name and email can be mapped specifically"
    );

    assert_eq!(
        snapshot.try_resolve(signature("Jane", "jane@laptop.(none)").to_ref()),
        Some(signature("Jane Doe", "jane@example.com")),
        "fix name and email by email"
    );
    assert_eq!(
        snapshot.try_resolve(signature("Jane", "jane@desktop.(none)").to_ref()),
        Some(signature("Jane Doe", "jane@example.com")),
        "fix name and email by other email"
    );

    assert_eq!(
        snapshot.try_resolve(signature("janE", "Bugs@example.com").to_ref()),
        Some(signature("Jane Doe", "jane@example.com")),
        "name and email can be mapped specifically, case insensitive matching of name"
    );
    assert_eq!(
        snapshot.resolve(signature("janE", "jane@ipad.(none)").to_ref()),
        signature("janE", "jane@example.com"),
        "an email can be mapped by name and email specifically, both match case-insensitively"
    );

    let sig = signature("Jane", "other@example.com");
    assert_eq!(snapshot.try_resolve(sig.to_ref()), None, "unmatched email");

    assert_eq!(
        snapshot.resolve(sig.to_ref()),
        sig,
        "resolution always works here, returning a copy of the original"
    );

    let sig = signature("Jean", "bugs@example.com");
    assert_eq!(
        snapshot.try_resolve(sig.to_ref()),
        None,
        "matched email, unmatched name"
    );
    assert_eq!(snapshot.resolve(sig.to_ref()), sig);

    assert_eq!(
        snapshot.entries(),
        &[
            Entry::change_name_and_email_by_name_and_email("Jane Doe", "jane@example.com", "Jane", "bugs@example.com"),
            Entry::change_name_and_email_by_name_and_email(
                "Joe R. Developer",
                "joe@example.com",
                "Joe",
                "bugs@example.com",
            ),
            Entry::change_name_and_email_by_email("Jane Doe", "jane@example.com", "jane@desktop.(none)"),
            Entry::change_email_by_name_and_email("jane@example.com", "Jane", "Jane@ipad.(none)"),
            Entry::change_name_and_email_by_email("Jane Doe", "jane@example.com", "jane@laptop.(none)"),
            Entry::change_name_by_email("Joe R. Developer", "joe@example.com"),
        ]
    );
}

#[test]
fn non_name_and_name_mappings_will_not_clash() {
    let entries = vec![
        // add mapping from email
        gix_mailmap::Entry::change_name_by_email("new-name", "old-email"),
        // add mapping from name and email
        gix_mailmap::Entry::change_name_and_email_by_name_and_email(
            "other-new-name",
            "other-new-email",
            "old-name",
            "old-email",
        ),
    ];
    for entries in [entries.clone().into_iter().rev().collect::<Vec<_>>(), entries] {
        let snapshot = Snapshot::new(entries);

        assert_eq!(
            snapshot.try_resolve(signature("replace-by-email", "Old-Email").to_ref()),
            Some(signature("new-name", "old-email")),
            "it can match by email only, and the email is normalized"
        );
        assert_eq!(
            snapshot.try_resolve(signature("old-name", "Old-Email").to_ref()),
            Some(signature("other-new-name", "other-new-email")),
            "it can match by email and name as well"
        );

        assert_eq!(
            snapshot.entries(),
            &[
                Entry::change_name_by_email("new-name", "old-email"),
                Entry::change_name_and_email_by_name_and_email(
                    "other-new-name",
                    "other-new-email",
                    "old-name",
                    "old-email"
                )
            ]
        );
    }
}

#[test]
fn overwrite_entries() {
    let snapshot = Snapshot::from_bytes(&fixture_bytes("overwrite.txt"));
    assert_eq!(
        snapshot.try_resolve(signature("does not matter", "old-a-email").to_ref()),
        Some(signature("A-overwritten", "old-a-email")),
        "email only by email"
    );

    assert_eq!(
        snapshot.try_resolve(signature("to be replaced", "old-b-EMAIL").to_ref()),
        Some(signature("B-overwritten", "new-b-email-overwritten")),
        "name and email by email"
    );

    assert_eq!(
        snapshot.try_resolve(signature("old-c", "old-C-email").to_ref()),
        Some(signature("C-overwritten", "new-c-email-overwritten")),
        "name and email by name and email"
    );

    assert_eq!(
        snapshot.try_resolve(signature("unchanged", "old-d-email").to_ref()),
        Some(signature("unchanged", "new-d-email-overwritten")),
        "email by email"
    );

    assert_eq!(
        snapshot.entries(),
        &[
            Entry::change_name_by_email("A-overwritten", "old-a-email"),
            Entry::change_name_and_email_by_email("B-overwritten", "new-b-email-overwritten", "old-b-email"),
            Entry::change_name_and_email_by_name_and_email(
                "C-overwritten",
                "new-c-email-overwritten",
                "old-C",
                "old-c-email"
            ),
            Entry::change_email_by_email("new-d-email-overwritten", "old-d-email")
        ]
    );
}

fn signature(name: &str, email: &str) -> gix_actor::Signature {
    gix_actor::Signature {
        name: name.into(),
        email: email.into(),
        time: b"42 +0800".into(),
    }
}
