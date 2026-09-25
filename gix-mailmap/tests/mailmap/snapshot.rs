use bstr::ByteSlice;
use gix_date::parse::TimeBuf;
use gix_mailmap::{Entry, Snapshot};
use gix_testtools::fixture_bytes;

#[test]
fn try_resolve() {
    let snapshot = Snapshot::from_bytes(&fixture_bytes("typical.txt"));
    let mut buf = TimeBuf::default();
    assert_eq!(
        snapshot.try_resolve(signature("Foo", "Joe@example.com").to_ref(&mut buf)),
        Some(signature("Joe R. Developer", "joe@example.com")),
        "resolved signatures contain all original fields, and normalize the email as well to match the one that it was looked up with"
    );
    assert_eq!(
        snapshot.try_resolve(signature("Joe", "bugs@example.com").to_ref(&mut buf)),
        Some(signature("Joe R. Developer", "joe@example.com")),
        "name and email can be mapped specifically"
    );

    assert_eq!(
        snapshot.try_resolve(signature("Jane", "jane@laptop.(none)").to_ref(&mut buf)),
        Some(signature("Jane Doe", "jane@example.com")),
        "fix name and email by email"
    );
    assert_eq!(
        snapshot.try_resolve(signature("Jane", "jane@desktop.(none)").to_ref(&mut buf)),
        Some(signature("Jane Doe", "jane@example.com")),
        "fix name and email by other email"
    );

    assert_eq!(
        snapshot.try_resolve(signature("janE", "Bugs@example.com").to_ref(&mut buf)),
        Some(signature("Jane Doe", "jane@example.com")),
        "name and email can be mapped specifically, case insensitive matching of name"
    );
    assert_eq!(
        snapshot.resolve(signature("janE", "jane@ipad.(none)").to_ref(&mut buf)),
        signature("janE", "jane@example.com"),
        "an email can be mapped by name and email specifically, both match case-insensitively"
    );

    let sig = signature("Jane", "other@example.com");
    assert_eq!(snapshot.try_resolve(sig.to_ref(&mut buf)), None, "unmatched email");

    assert_eq!(
        snapshot.resolve(sig.to_ref(&mut buf)),
        sig,
        "resolution always works here, returning a copy of the original"
    );

    let sig = signature("Jean", "bugs@example.com");
    assert_eq!(
        snapshot.try_resolve(sig.to_ref(&mut buf)),
        None,
        "matched email, unmatched name"
    );
    assert_eq!(snapshot.resolve(sig.to_ref(&mut buf)), sig);

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
fn empty_emails_can_be_mapped() {
    let snapshot = Snapshot::from_bytes(b"Canonical Name <canonical@example.com> <>");
    let mut buf = TimeBuf::default();

    assert_eq!(
        snapshot.try_resolve(signature("Any Name", "").to_ref(&mut buf)),
        Some(signature("Canonical Name", "canonical@example.com")),
        "an empty email matches an empty old email"
    );
    assert_eq!(
        snapshot.try_resolve(signature("Any Name", "other@example.com").to_ref(&mut buf)),
        None,
        "the empty-email mapping does not match a non-empty email"
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
    let mut buf = TimeBuf::default();
    for entries in [entries.clone().into_iter().rev().collect::<Vec<_>>(), entries] {
        let snapshot = Snapshot::new(entries);

        assert_eq!(
            snapshot.try_resolve(signature("replace-by-email", "Old-Email").to_ref(&mut buf)),
            Some(signature("new-name", "old-email")),
            "it can match by email only, and the email is normalized"
        );
        assert_eq!(
            snapshot.try_resolve(signature("old-name", "Old-Email").to_ref(&mut buf)),
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
    let mut buf = TimeBuf::default();
    assert_eq!(
        snapshot.try_resolve(signature("does not matter", "old-a-email").to_ref(&mut buf)),
        Some(signature("A-overwritten", "old-a-email")),
        "email only by email"
    );

    assert_eq!(
        snapshot.try_resolve(signature("to be replaced", "old-b-EMAIL").to_ref(&mut buf)),
        Some(signature("B-overwritten", "new-b-email-overwritten")),
        "name and email by email"
    );

    assert_eq!(
        snapshot.try_resolve(signature("old-c", "old-C-email").to_ref(&mut buf)),
        Some(signature("C-overwritten", "new-c-email-overwritten")),
        "name and email by name and email"
    );

    assert_eq!(
        snapshot.try_resolve(signature("unchanged", "old-d-email").to_ref(&mut buf)),
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

#[test]
fn invalid_entries_are_ignored() {
    let invalid = Entry::default();
    assert_eq!(
        Snapshot::new([invalid, invalid]),
        Snapshot::default(),
        "entries with neither a new name nor a new email must not create mappings"
    );

    let entries = [
        invalid,
        Entry::change_name_by_email("First", "old"),
        invalid,
        Entry::change_email_by_email("new", "old"),
        invalid,
        Entry::change_name_by_email("Last", "old"),
        invalid,
    ];
    let expected = Snapshot::new([Entry::change_name_and_email_by_email("Last", "new", "old")]);
    for split in 0..=entries.len() {
        let mut snapshot = Snapshot::new(entries[..split].iter().copied());
        snapshot.merge(entries[split..].iter().copied());
        assert_eq!(
            snapshot, expected,
            "invalid entries must not affect valid update order, including a merge at entry {split}"
        );

        snapshot.merge([invalid, invalid]);
        assert_eq!(
            snapshot, expected,
            "an invalid-only merge must leave existing mappings unchanged"
        );
    }
}

#[test]
fn partial_updates_match_git() -> gix_testtools::Result {
    assert_matches_git("partial-updates")
}

#[test]
fn case_folding_with_non_utf8_keys_matches_git() -> gix_testtools::Result {
    assert_matches_git("case-folding-with-non-utf8-keys")
}

#[test]
fn large_reverse_ordered_mailmaps() -> gix_testtools::Result {
    use std::fmt::Write;

    const COUNT: usize = 100_000;
    for match_name in [false, true] {
        let mut input = String::new();
        for index in (0..COUNT).rev() {
            if match_name {
                writeln!(input, "New {index:06} <new> Old {index:06} <old>")?;
            } else {
                writeln!(input, "New {index:06} <old-{index:06}>")?;
            }
        }

        let start = std::time::Instant::now();
        let snapshot = Snapshot::from_bytes(input.as_bytes());
        eprintln!("{COUNT} entries, match_name={match_name}: {:?}", start.elapsed());
        assert_eq!(snapshot.iter().count(), COUNT, "every distinct key is retained");
        for index in [0, COUNT / 2, COUNT - 1] {
            let old_name = format!("Old {index:06}");
            let old_email = if match_name {
                "old".to_owned()
            } else {
                format!("old-{index:06}")
            };
            let mut buf = TimeBuf::default();
            assert_eq!(
                snapshot.resolve(signature(&old_name, &old_email).to_ref(&mut buf)),
                signature(&format!("New {index:06}"), if match_name { "new" } else { &old_email }),
                "keys at the beginning, middle and end remain searchable"
            );
        }
    }
    Ok(())
}

/// Assert that the fixture's contacts in `name` resolve to its Git-generated baseline byte for byte.
///
/// The fixture script supplies the mailmap, newline-separated `Name <email>` contacts, and output
/// of `git check-mailmap --stdin`; this helper only reads the shared fixture. For every split of
/// the parsed entries, build a snapshot from the prefix and merge the suffix, ensuring that both
/// one-shot construction and incremental updates agree with Git on mapping precedence and lookup.
fn assert_matches_git(name: &str) -> gix_testtools::Result {
    let dir = gix_testtools::scripted_fixture_read_only("make_mailmap_baseline.sh")?.join(name);
    let mailmap = std::fs::read(dir.join(".mailmap"))?;
    let contacts = std::fs::read(dir.join("contacts"))?;
    let expected = std::fs::read(dir.join("baseline.git"))?;

    let entries = gix_mailmap::parse(&mailmap).collect::<Result<Vec<_>, _>>()?;
    for split in 0..=entries.len() {
        let mut snapshot = Snapshot::new(entries[..split].iter().copied());
        snapshot.merge(entries[split..].iter().copied());
        let mut actual = Vec::new();
        for line in contacts.lines() {
            let contact = gix_actor::IdentityRef::from_bytes(line)?;
            let resolved = snapshot.resolve_cow(gix_actor::SignatureRef {
                name: contact.name,
                email: contact.email,
                time: "0 +0000",
            });
            gix_actor::IdentityRef {
                name: resolved.name.as_ref(),
                email: resolved.email.as_ref(),
            }
            .write_to(&mut actual)?;
            actual.push(b'\n');
        }
        assert_eq!(
            actual.as_bstr(),
            expected.as_bstr(),
            "mapping precedence and lookup must match Git, including a merge at entry {split}"
        );
    }
    Ok(())
}

fn signature(name: &str, email: &str) -> gix_actor::Signature {
    gix_actor::Signature {
        name: name.into(),
        email: email.into(),
        time: gix_date::parse_header("42 +0800").unwrap(),
    }
}
