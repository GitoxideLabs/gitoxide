use gix_diff::{
    rewrites::{Copies, CopySource},
    tree::{recorder::Location, visit::Relation},
    tree_with_rewrites::{Change, Options},
    Rewrites,
};
use gix_object::{bstr::BStr, TreeRefIter};

#[test]
fn empty_to_new_tree_without_rename_tracking() -> crate::Result {
    let (changes, _out) = collect_changes(None, "c1 - initial").expect("full path tracking is the default");
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Addition {
            location: "a",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "b",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "d",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "dir",
            relation: Some(
                Parent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o40000),
            id: Sha1(587ff082e0b98914788500eae5dd6a33f04883c9),
        },
        Addition {
            location: "dir/c",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
    ]
    "#);

    let (changes, _out) = collect_changes_opts(
        None,
        "c1 - initial",
        Options {
            location: Some(Location::FileName),
            ..Default::default()
        },
    )
    .expect("the path-options are respected - we only see the filename here");
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Addition {
            location: "a",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "b",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "d",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "dir",
            relation: Some(
                Parent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o40000),
            id: Sha1(587ff082e0b98914788500eae5dd6a33f04883c9),
        },
        Addition {
            location: "c",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
    ]
    "#);

    {
        let (lhs, rhs, mut cache, odb) = repo_with_trees(None, "c1 - initial")?;
        let err = gix_diff::tree_with_rewrites(
            TreeRefIter::from_bytes(&lhs),
            TreeRefIter::from_bytes(&rhs),
            &mut cache,
            &mut Default::default(),
            &odb,
            |_change| Err(std::io::Error::other("custom error")),
            Options::default(),
        )
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "The user-provided callback failed",
            "custom errors made visible and not squelched"
        );
    }
    Ok(())
}

#[test]
fn changes_against_modified_tree_with_filename_tracking() -> crate::Result {
    let (changes, _out) = collect_changes("c2", "c3-modification")?;

    insta::assert_debug_snapshot!(changes, @r#"
    [
        Modification {
            location: "a",
            previous_entry_mode: EntryMode(0o100644),
            previous_id: Sha1(78981922613b2afb6025042ff6bd878ac1994e85),
            entry_mode: EntryMode(0o100644),
            id: Sha1(b4f17b61de71d9b2e54ac9e62b1629ae2d97a6a7),
        },
        Modification {
            location: "dir",
            previous_entry_mode: EntryMode(0o40000),
            previous_id: Sha1(e5c63aefe4327cb1c780c71966b678ce8e4225da),
            entry_mode: EntryMode(0o40000),
            id: Sha1(c7ac5f82f536976f3561c9999b5f11e5893358be),
        },
        Modification {
            location: "dir/c",
            previous_entry_mode: EntryMode(0o100644),
            previous_id: Sha1(6695780ceb14b05e076a99bbd2babf34723b3464),
            entry_mode: EntryMode(0o100644),
            id: Sha1(40006fcef15a8853a1b7ae186d93b7d680fd29cf),
        },
    ]
    "#);
    let (changes, _out) = collect_changes_opts(
        "c2",
        "c3-modification",
        Options {
            location: Some(Location::FileName),
            ..Default::default()
        },
    )?;
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Modification {
            location: "a",
            previous_entry_mode: EntryMode(0o100644),
            previous_id: Sha1(78981922613b2afb6025042ff6bd878ac1994e85),
            entry_mode: EntryMode(0o100644),
            id: Sha1(b4f17b61de71d9b2e54ac9e62b1629ae2d97a6a7),
        },
        Modification {
            location: "dir",
            previous_entry_mode: EntryMode(0o40000),
            previous_id: Sha1(e5c63aefe4327cb1c780c71966b678ce8e4225da),
            entry_mode: EntryMode(0o40000),
            id: Sha1(c7ac5f82f536976f3561c9999b5f11e5893358be),
        },
        Modification {
            location: "c",
            previous_entry_mode: EntryMode(0o100644),
            previous_id: Sha1(6695780ceb14b05e076a99bbd2babf34723b3464),
            entry_mode: EntryMode(0o100644),
            id: Sha1(40006fcef15a8853a1b7ae186d93b7d680fd29cf),
        },
    ]
    "#);
    Ok(())
}

#[test]
fn renames_by_identity() -> crate::Result {
    for (from, to, expected, assert_msg, track_empty) in [
        (
            "c3-modification",
            "r1-identity",
            vec![BStr::new("a"), "dir/a-moved".into()],
            "one rename and nothing else",
            false,
        ),
        (
            "c4 - add identical files",
            "r2-ambiguous",
            vec![
                "s1".into(),
                "b1".into(),
                "s2".into(),
                "b2".into(),
                "s3".into(),
                "z".into(),
            ],
            "multiple possible sources decide by ordering everything lexicographically",
            true,
        ),
        (
            "c4 - add identical files",
            "r2-ambiguous",
            vec![],
            "nothing is tracked with `track_empty = false`",
            false,
        ),
        (
            "c5 - add links",
            "r4-symlinks",
            vec!["link-1".into(), "renamed-link-1".into()],
            "symlinks are only tracked by identity",
            false,
        ),
        (
            "r1-identity",
            "c4 - add identical files",
            vec![],
            "not having any renames is OK as well",
            false,
        ),
        (
            "tc1-identity",
            "tc1-identity",
            vec![],
            "copy tracking is off by default",
            false,
        ),
    ] {
        for percentage in [None, Some(0.5)] {
            let (changes, out) = collect_changes_opts(
                from,
                to,
                Options {
                    location: Some(Location::Path),
                    rewrites: Some(Rewrites {
                        percentage,
                        track_empty,
                        ..Default::default()
                    }),
                },
            )?;
            let actual: Vec<_> = changes
                .into_iter()
                .filter(|c| !c.entry_mode().is_tree())
                .flat_map(|c| match c {
                    Change::Rewrite {
                        source_location,
                        location,
                        copy,
                        ..
                    } => {
                        assert!(!copy);
                        vec![source_location, location]
                    }
                    _ => vec![],
                })
                .collect();

            assert_eq!(actual, expected, "{assert_msg}");
            #[cfg(not(windows))]
            assert_eq!(
                out.expect("present as rewrites are configured").num_similarity_checks,
                0,
                "there are no fuzzy checks in if everything was resolved by identity only"
            );
        }
    }
    Ok(())
}

#[test]
fn rename_by_similarity() -> crate::Result {
    insta::allow_duplicates! {
    for percentage in [
        None,
        Some(0.76), /*cutoff point where git stops seeing it as equal */
    ] {
        let (changes, out) = collect_changes_opts(
            "r2-ambiguous",
            "r3-simple",
            Options {
                location: Some(Location::Path),
                rewrites: Some(Rewrites {
                    percentage,
                    ..Default::default()
                }),
            },
        ).expect("errors can only happen with IO or ODB access fails");
        insta::assert_debug_snapshot!(
            changes,
            @r#"
        [
            Modification {
                location: "b",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(61780798228d17af2d34fce4cfbdf35556832472),
                entry_mode: EntryMode(0o100644),
                id: Sha1(54781fa52cf133fa9d0bf59cfe2ef2621b5ad29f),
            },
            Modification {
                location: "dir",
                previous_entry_mode: EntryMode(0o40000),
                previous_id: Sha1(d1622e275dbb2cb3215a0bdcd2fc77273891f360),
                entry_mode: EntryMode(0o40000),
                id: Sha1(6602e61ea053525e4907e155c0b3da3a269e1385),
            },
            Deletion {
                location: "dir/c",
                relation: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(40006fcef15a8853a1b7ae186d93b7d680fd29cf),
            },
            Addition {
                location: "dir/c-moved",
                relation: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(f01e8ddf5adc56985b9a1cda6d7c7ef9e3abe034),
            },
        ]
        "#
            );
            let out = out.expect("tracking enabled");
            assert_eq!(out.num_similarity_checks, if percentage.is_some() { 1 } else { 0 });
            assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0);
            assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);
        }
    }

    let (changes, out) = collect_changes_opts(
        "r2-ambiguous",
        "r3-simple",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                percentage: Some(0.6),
                limit: 1, // has no effect as it's just one item here.
                ..Default::default()
            }),
        },
    )
    .expect("it found all items at the cut-off point, similar to git");

    insta::assert_debug_snapshot!(changes, @r#"
    [
        Modification {
            location: "b",
            previous_entry_mode: EntryMode(0o100644),
            previous_id: Sha1(61780798228d17af2d34fce4cfbdf35556832472),
            entry_mode: EntryMode(0o100644),
            id: Sha1(54781fa52cf133fa9d0bf59cfe2ef2621b5ad29f),
        },
        Modification {
            location: "dir",
            previous_entry_mode: EntryMode(0o40000),
            previous_id: Sha1(d1622e275dbb2cb3215a0bdcd2fc77273891f360),
            entry_mode: EntryMode(0o40000),
            id: Sha1(6602e61ea053525e4907e155c0b3da3a269e1385),
        },
        Rewrite {
            source_location: "dir/c",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(40006fcef15a8853a1b7ae186d93b7d680fd29cf),
            diff: Some(
                DiffLineStats {
                    removals: 0,
                    insertions: 1,
                    before: 2,
                    after: 3,
                    similarity: 0.65,
                },
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(f01e8ddf5adc56985b9a1cda6d7c7ef9e3abe034),
            location: "dir/c-moved",
            relation: None,
            copy: false,
        },
    ]
    "#);

    let out = out.expect("tracking enabled");
    assert_eq!(out.num_similarity_checks, 1);
    assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0);
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);
    Ok(())
}

#[test]
fn renames_by_similarity_with_limit() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "c6",
        "r5",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                limit: 1, // prevent fuzzy tracking from happening
                ..Default::default()
            }),
        },
    )?;
    assert_eq!(
        changes.iter().filter(|c| matches!(c, Change::Rewrite { .. })).count(),
        0,
        "fuzzy tracking is effectively disabled due to limit"
    );
    let actual: Vec<_> = changes.iter().map(Change::location).collect();
    assert_eq!(actual, ["f1", "f1-renamed", "f2", "f2-renamed"]);

    let out = out.expect("tracking enabled");
    assert_eq!(out.num_similarity_checks, 0);
    assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 4);
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);

    Ok(())
}

#[test]
fn copies_by_identity() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "c7",
        "tc1-identity",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                copies: Some(Copies {
                    source: CopySource::FromSetOfModifiedFiles,
                    percentage: None,
                }),
                limit: 1, // the limit isn't actually used for identity based checks
                ..Default::default()
            }),
        },
    )?;
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Modification {
            location: "dir",
            previous_entry_mode: EntryMode(0o40000),
            previous_id: Sha1(6602e61ea053525e4907e155c0b3da3a269e1385),
            entry_mode: EntryMode(0o40000),
            id: Sha1(f01fd5b4d733a4ae749cbb58a828cdb3f342f298),
        },
        Rewrite {
            source_location: "base",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(f00c965d8307308469e537302baa73048488f162),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(f00c965d8307308469e537302baa73048488f162),
            location: "c1",
            relation: None,
            copy: true,
        },
        Rewrite {
            source_location: "base",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(f00c965d8307308469e537302baa73048488f162),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(f00c965d8307308469e537302baa73048488f162),
            location: "c2",
            relation: None,
            copy: true,
        },
        Rewrite {
            source_location: "base",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(f00c965d8307308469e537302baa73048488f162),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(f00c965d8307308469e537302baa73048488f162),
            location: "dir/c3",
            relation: None,
            copy: true,
        },
    ]
    "#);
    let out = out.expect("tracking enabled");
    assert_eq!(out.num_similarity_checks, 0);
    assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0);
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);

    Ok(())
}

#[test]
fn copies_by_similarity() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "tc1-identity",
        "tc2-similarity",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                copies: Some(Copies::default()),
                ..Default::default()
            }),
        },
    )?;
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Modification {
            location: "dir",
            previous_entry_mode: EntryMode(0o40000),
            previous_id: Sha1(f01fd5b4d733a4ae749cbb58a828cdb3f342f298),
            entry_mode: EntryMode(0o40000),
            id: Sha1(1d7e20e07562a54af0408fd2669b0c56a6faa6f0),
        },
        Rewrite {
            source_location: "base",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            location: "c4",
            relation: None,
            copy: true,
        },
        Rewrite {
            source_location: "base",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            diff: Some(
                DiffLineStats {
                    removals: 0,
                    insertions: 1,
                    before: 11,
                    after: 12,
                    similarity: 0.8888889,
                },
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(08fe19ca4d2f79624f35333157d610811efc1aed),
            location: "c5",
            relation: None,
            copy: true,
        },
        Rewrite {
            source_location: "base",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            diff: Some(
                DiffLineStats {
                    removals: 0,
                    insertions: 1,
                    before: 11,
                    after: 12,
                    similarity: 0.8888889,
                },
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(cf7a729ca69bfabd0995fc9b083e86a18215bd91),
            location: "dir/c6",
            relation: None,
            copy: true,
        },
    ]
    "#);

    let out = out.expect("tracking enabled");
    assert_eq!(
        out.num_similarity_checks, 2,
        "two are similar, the other one is identical"
    );
    assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0);
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);

    Ok(())
}

#[test]
fn copies_in_entire_tree_by_similarity() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "tc2-similarity",
        "tc3-find-harder",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                copies: Some(Copies::default()),
                ..Default::default()
            }),
        },
    )?;
    assert_eq!(
        changes.iter().filter(|c| matches!(c, Change::Rewrite { .. })).count(),
        0,
        "needs --find-copies-harder to detect rewrites here"
    );
    let actual: Vec<_> = changes.iter().map(Change::location).collect();
    assert_eq!(actual, ["b", "c6", "c7", "newly-added"]);

    let out = out.expect("tracking enabled");
    assert_eq!(
        out.num_similarity_checks, 3,
        "it does have some candidates, probably for rename tracking"
    );
    assert_eq!(
        out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0,
        "no limit configured"
    );
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);

    let (changes, out) = collect_changes_opts(
        "tc2-similarity",
        "tc3-find-harder",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                copies: Some(Copies {
                    source: CopySource::FromSetOfModifiedFilesAndAllSources,
                    ..Default::default()
                }),
                ..Default::default()
            }),
        },
    )?;
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Rewrite {
            source_location: "base",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            location: "c6",
            relation: None,
            copy: true,
        },
        Rewrite {
            source_location: "dir/c6",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(cf7a729ca69bfabd0995fc9b083e86a18215bd91),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(cf7a729ca69bfabd0995fc9b083e86a18215bd91),
            location: "c7",
            relation: None,
            copy: true,
        },
        Rewrite {
            source_location: "c5",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(08fe19ca4d2f79624f35333157d610811efc1aed),
            diff: Some(
                DiffLineStats {
                    removals: 0,
                    insertions: 3,
                    before: 12,
                    after: 15,
                    similarity: 0.75,
                },
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(97b3d1a5707f8a11fa5fa8bc6c3bd7b3965601fd),
            location: "newly-added",
            relation: None,
            copy: true,
        },
        Modification {
            location: "b",
            previous_entry_mode: EntryMode(0o100644),
            previous_id: Sha1(54781fa52cf133fa9d0bf59cfe2ef2621b5ad29f),
            entry_mode: EntryMode(0o100644),
            id: Sha1(f198d0640214092732566fb00543163845c8252c),
        },
    ]
    "#);
    let out = out.expect("tracking enabled");
    assert_eq!(out.num_similarity_checks, 4);
    assert_eq!(
        out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0,
        "no limit configured"
    );
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);

    Ok(())
}

#[test]
fn copies_in_entire_tree_by_similarity_with_limit() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "tc2-similarity",
        "tc3-find-harder",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                copies: Some(Copies {
                    source: CopySource::FromSetOfModifiedFilesAndAllSources,
                    ..Default::default()
                }),
                limit: 2, // similarity checks can't be made that way
                track_empty: false,
                ..Default::default()
            }),
        },
    )?;
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Rewrite {
            source_location: "base",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            location: "c6",
            relation: None,
            copy: true,
        },
        Rewrite {
            source_location: "dir/c6",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(cf7a729ca69bfabd0995fc9b083e86a18215bd91),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(cf7a729ca69bfabd0995fc9b083e86a18215bd91),
            location: "c7",
            relation: None,
            copy: true,
        },
        Modification {
            location: "b",
            previous_entry_mode: EntryMode(0o100644),
            previous_id: Sha1(54781fa52cf133fa9d0bf59cfe2ef2621b5ad29f),
            entry_mode: EntryMode(0o100644),
            id: Sha1(f198d0640214092732566fb00543163845c8252c),
        },
        Addition {
            location: "newly-added",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(97b3d1a5707f8a11fa5fa8bc6c3bd7b3965601fd),
        },
    ]
    "#);

    let out = out.expect("tracking enabled");
    assert_eq!(out.num_similarity_checks, 0, "similarity checks can't run");
    assert_eq!(
        out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0,
        "no limit configured"
    );
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 19);

    Ok(())
}

#[test]
fn copies_by_similarity_with_limit() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "tc1-identity",
        "tc2-similarity",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                copies: Some(Copies::default()),
                limit: 1,
                ..Default::default()
            }),
        },
    )?;

    insta::assert_debug_snapshot!(changes, @r#"
    [
        Modification {
            location: "dir",
            previous_entry_mode: EntryMode(0o40000),
            previous_id: Sha1(f01fd5b4d733a4ae749cbb58a828cdb3f342f298),
            entry_mode: EntryMode(0o40000),
            id: Sha1(1d7e20e07562a54af0408fd2669b0c56a6faa6f0),
        },
        Rewrite {
            source_location: "base",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            location: "c4",
            relation: None,
            copy: true,
        },
        Addition {
            location: "c5",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(08fe19ca4d2f79624f35333157d610811efc1aed),
        },
        Addition {
            location: "dir/c6",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(cf7a729ca69bfabd0995fc9b083e86a18215bd91),
        },
    ]
    "#);

    let out = out.expect("tracking enabled");
    assert_eq!(out.num_similarity_checks, 0);
    assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0);
    assert_eq!(
        out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 2,
        "limit prevents any similarity check from being performed, and identity fails in most places"
    );

    Ok(())
}

#[test]
fn realistic_renames_by_identity() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "r1-base",
        "r1-change",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                copies: Some(Copies::default()),
                limit: 1,
                track_empty: true,
                ..Default::default()
            }),
        },
    )?;

    insta::assert_debug_snapshot!(changes.into_iter().filter(|c| !c.entry_mode().is_tree()).collect::<Vec<_>>(), @r#"
    [
        Rewrite {
            source_location: "git-index/src/file.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "git-index/src/file/mod.rs",
            relation: None,
            copy: false,
        },
        Addition {
            location: "git-index/tests/index/file/access.rs",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Modification {
            location: "git-index/tests/index/file/mod.rs",
            previous_entry_mode: EntryMode(0o100644),
            previous_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            entry_mode: EntryMode(0o100644),
            id: Sha1(8ba3a16384aacc37d01564b28401755ce8053f51),
        },
    ]
    "#);

    #[cfg(not(windows))]
    {
        let actual = std::fs::read_to_string(repo_workdir()?.join("baseline.with-renames"))?;
        let expected = r#"commit 0231f5093bd3d760e7ee82984e0453da80e05c87
Author: author <author@example.com>
Date:   Sat Jan 1 00:00:00 2000 +0000

    r1-change

diff --git a/git-index/src/file.rs b/git-index/src/file/mod.rs
similarity index 100%
rename from git-index/src/file.rs
rename to git-index/src/file/mod.rs
diff --git a/git-index/tests/index/file/access.rs b/git-index/tests/index/file/access.rs
new file mode 100644
index 0000000..e69de29
diff --git a/git-index/tests/index/file/mod.rs b/git-index/tests/index/file/mod.rs
index e69de29..8ba3a16 100644
--- a/git-index/tests/index/file/mod.rs
+++ b/git-index/tests/index/file/mod.rs
@@ -0,0 +1 @@
+n
"#;
        assert_eq!(actual, expected);
    }

    let out = out.expect("tracking enabled");
    assert_eq!(out.num_similarity_checks, 1);
    assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0);
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);

    Ok(())
}

#[test]
fn realistic_renames_disabled() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "r1-base",
        "r1-change",
        Options {
            location: Some(Location::Path),
            rewrites: None,
        },
    )?;

    insta::assert_debug_snapshot!(changes.into_iter().filter(|c| !c.entry_mode().is_tree()).collect::<Vec<_>>(), @r#"
    [
        Deletion {
            location: "git-index/src/file.rs",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "git-index/src/file/mod.rs",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "git-index/tests/index/file/access.rs",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Modification {
            location: "git-index/tests/index/file/mod.rs",
            previous_entry_mode: EntryMode(0o100644),
            previous_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            entry_mode: EntryMode(0o100644),
            id: Sha1(8ba3a16384aacc37d01564b28401755ce8053f51),
        },
    ]
    "#);

    #[cfg(not(windows))]
    {
        let actual = std::fs::read_to_string(repo_workdir()?.join("baseline.no-renames"))?;
        let expected = r#"commit 0231f5093bd3d760e7ee82984e0453da80e05c87
Author: author <author@example.com>
Date:   Sat Jan 1 00:00:00 2000 +0000

    r1-change

diff --git a/git-index/src/file.rs b/git-index/src/file.rs
deleted file mode 100644
index e69de29..0000000
diff --git a/git-index/src/file/mod.rs b/git-index/src/file/mod.rs
new file mode 100644
index 0000000..e69de29
diff --git a/git-index/tests/index/file/access.rs b/git-index/tests/index/file/access.rs
new file mode 100644
index 0000000..e69de29
diff --git a/git-index/tests/index/file/mod.rs b/git-index/tests/index/file/mod.rs
index e69de29..8ba3a16 100644
--- a/git-index/tests/index/file/mod.rs
+++ b/git-index/tests/index/file/mod.rs
@@ -0,0 +1 @@
+n
"#;
        assert_eq!(actual, expected);
    }

    assert_eq!(out, None, "tracking is disabled completely");
    Ok(())
}

#[test]
fn realistic_renames_disabled_2() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "r2-base",
        "r2-change",
        Options {
            location: Some(Location::Path),
            rewrites: None,
        },
    )?;

    // Directories are associated with their children, making a bundling possible.
    insta::assert_debug_snapshot!(changes.into_iter()
                                     .filter(|c| !c.entry_mode().is_tree() ||
                                                  c.relation().is_some_and(|r| matches!(r, Relation::Parent(_)))
                                     ).collect::<Vec<_>>(), @r#"
    [
        Deletion {
            location: "git-sec",
            relation: Some(
                Parent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o40000),
            id: Sha1(0026010e87631065a2739f627622feb14f903fd4),
        },
        Addition {
            location: "gix-sec",
            relation: Some(
                Parent(
                    2,
                ),
            ),
            entry_mode: EntryMode(0o40000),
            id: Sha1(0026010e87631065a2739f627622feb14f903fd4),
        },
        Deletion {
            location: "git-sec/CHANGELOG.md",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/Cargo.toml",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/CHANGELOG.md",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/Cargo.toml",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/src/identity.rs",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/src/lib.rs",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/src/permission.rs",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/src/trust.rs",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/tests/sec.rs",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/src/identity.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/src/lib.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/src/permission.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/src/trust.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/tests/sec.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/tests/identity/mod.rs",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/tests/identity/mod.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
    ]
    "#);

    #[cfg(not(windows))]
    {
        let expected = r#"commit d78c63c5ea3149040767e4387e7fc743cda118fd
Author: author <author@example.com>
Date:   Sat Jan 1 00:00:00 2000 +0000

    r2-change

diff --git a/git-sec/CHANGELOG.md b/git-sec/CHANGELOG.md
deleted file mode 100644
index e69de29..0000000
diff --git a/git-sec/Cargo.toml b/git-sec/Cargo.toml
deleted file mode 100644
index e69de29..0000000
diff --git a/git-sec/src/identity.rs b/git-sec/src/identity.rs
deleted file mode 100644
index e69de29..0000000
diff --git a/git-sec/src/lib.rs b/git-sec/src/lib.rs
deleted file mode 100644
index e69de29..0000000
diff --git a/git-sec/src/permission.rs b/git-sec/src/permission.rs
deleted file mode 100644
index e69de29..0000000
diff --git a/git-sec/src/trust.rs b/git-sec/src/trust.rs
deleted file mode 100644
index e69de29..0000000
diff --git a/git-sec/tests/identity/mod.rs b/git-sec/tests/identity/mod.rs
deleted file mode 100644
index e69de29..0000000
diff --git a/git-sec/tests/sec.rs b/git-sec/tests/sec.rs
deleted file mode 100644
index e69de29..0000000
diff --git a/gix-sec/CHANGELOG.md b/gix-sec/CHANGELOG.md
new file mode 100644
index 0000000..e69de29
diff --git a/gix-sec/Cargo.toml b/gix-sec/Cargo.toml
new file mode 100644
index 0000000..e69de29
diff --git a/gix-sec/src/identity.rs b/gix-sec/src/identity.rs
new file mode 100644
index 0000000..e69de29
diff --git a/gix-sec/src/lib.rs b/gix-sec/src/lib.rs
new file mode 100644
index 0000000..e69de29
diff --git a/gix-sec/src/permission.rs b/gix-sec/src/permission.rs
new file mode 100644
index 0000000..e69de29
diff --git a/gix-sec/src/trust.rs b/gix-sec/src/trust.rs
new file mode 100644
index 0000000..e69de29
diff --git a/gix-sec/tests/identity/mod.rs b/gix-sec/tests/identity/mod.rs
new file mode 100644
index 0000000..e69de29
diff --git a/gix-sec/tests/sec.rs b/gix-sec/tests/sec.rs
new file mode 100644
index 0000000..e69de29
"#;
        assert_eq!(
            std::fs::read_to_string(repo_workdir()?.join("baseline-2.no-renames"))?,
            expected
        );
    }

    assert_eq!(out, None, "tracking is disabled completely");
    Ok(())
}

#[test]
fn realistic_renames_disabled_3() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "r3-base",
        "r3-change",
        Options {
            location: Some(Location::Path),
            rewrites: None,
        },
    )?;

    insta::assert_debug_snapshot!(changes.into_iter().filter(|c| !c.entry_mode().is_tree()).collect::<Vec<_>>(), @r#"
    [
        Addition {
            location: "src/ein.rs",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "src/gix.rs",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "src/plumbing-cli.rs",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "src/porcelain-cli.rs",
            relation: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
    ]
    "#);

    #[cfg(not(windows))]
    {
        let expected = r#"commit 0cf7a4fe3ad6c49ae7beb394a1c1df7cc5173ce4
Author: author <author@example.com>
Date:   Sat Jan 1 00:00:00 2000 +0000

    r3-change

diff --git a/src/ein.rs b/src/ein.rs
new file mode 100644
index 0000000..e69de29
diff --git a/src/gix.rs b/src/gix.rs
new file mode 100644
index 0000000..e69de29
diff --git a/src/plumbing-cli.rs b/src/plumbing-cli.rs
deleted file mode 100644
index e69de29..0000000
diff --git a/src/porcelain-cli.rs b/src/porcelain-cli.rs
deleted file mode 100644
index e69de29..0000000
"#;

        assert_eq!(
            std::fs::read_to_string(repo_workdir()?.join("baseline-3.no-renames"))?,
            expected
        );
    }

    assert_eq!(out, None, "tracking is disabled completely");
    Ok(())
}

#[test]
fn realistic_renames_by_identity_3() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "r3-base",
        "r3-change",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                copies: Some(Copies::default()),
                limit: 1,
                track_empty: true,
                ..Default::default()
            }),
        },
    )?;

    insta::assert_debug_snapshot!(changes.into_iter().filter(|c| !c.entry_mode().is_tree()).collect::<Vec<_>>(), @r#"
    [
        Rewrite {
            source_location: "src/plumbing-cli.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "src/ein.rs",
            relation: None,
            copy: false,
        },
        Rewrite {
            source_location: "src/porcelain-cli.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: None,
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "src/gix.rs",
            relation: None,
            copy: false,
        },
    ]
    "#);

    #[cfg(not(windows))]
    {
        let expected = r#"commit 0cf7a4fe3ad6c49ae7beb394a1c1df7cc5173ce4
Author: author <author@example.com>
Date:   Sat Jan 1 00:00:00 2000 +0000

    r3-change

diff --git a/src/plumbing-cli.rs b/src/ein.rs
similarity index 100%
rename from src/plumbing-cli.rs
rename to src/ein.rs
diff --git a/src/porcelain-cli.rs b/src/gix.rs
similarity index 100%
rename from src/porcelain-cli.rs
rename to src/gix.rs
"#;
        assert_eq!(
            std::fs::read_to_string(repo_workdir()?.join("baseline-3.with-renames"))?,
            expected
        );
    }

    let out = out.expect("tracking enabled");
    assert_eq!(
        out.num_similarity_checks, 0,
        "similarity checks disabled, and not necessary"
    );
    assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0);
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);

    Ok(())
}

#[test]
fn realistic_renames_2() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "r2-base",
        "r2-change",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                copies: Some(Copies::default()),
                limit: 1,
                track_empty: false,
                ..Default::default()
            }),
        },
    )?;

    // Look how nicely it captures and associates this directory rename.
    insta::assert_debug_snapshot!(changes.into_iter()
                                     .filter(|c| !c.entry_mode().is_tree() ||
                                                  c.relation().is_some_and(|r| matches!(r, Relation::Parent(_)))
                                     ).collect::<Vec<_>>(), @r#"
    [
        Rewrite {
            source_location: "git-sec",
            source_entry_mode: EntryMode(0o40000),
            source_relation: Some(
                Parent(
                    1,
                ),
            ),
            source_id: Sha1(0026010e87631065a2739f627622feb14f903fd4),
            diff: None,
            entry_mode: EntryMode(0o40000),
            id: Sha1(0026010e87631065a2739f627622feb14f903fd4),
            location: "gix-sec",
            relation: Some(
                Parent(
                    2,
                ),
            ),
            copy: false,
        },
        Rewrite {
            source_location: "git-sec/CHANGELOG.md",
            source_entry_mode: EntryMode(0o100644),
            source_relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "gix-sec/CHANGELOG.md",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            copy: false,
        },
        Rewrite {
            source_location: "git-sec/Cargo.toml",
            source_entry_mode: EntryMode(0o100644),
            source_relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "gix-sec/Cargo.toml",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            copy: false,
        },
        Rewrite {
            source_location: "git-sec/src/identity.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "gix-sec/src/identity.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            copy: false,
        },
        Rewrite {
            source_location: "git-sec/src/lib.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "gix-sec/src/lib.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            copy: false,
        },
        Rewrite {
            source_location: "git-sec/src/permission.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "gix-sec/src/permission.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            copy: false,
        },
        Rewrite {
            source_location: "git-sec/src/trust.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "gix-sec/src/trust.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            copy: false,
        },
        Rewrite {
            source_location: "git-sec/tests/sec.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "gix-sec/tests/sec.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            copy: false,
        },
        Rewrite {
            source_location: "git-sec/tests/identity/mod.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "gix-sec/tests/identity/mod.rs",
            relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            copy: false,
        },
    ]
    "#);

    #[cfg(not(windows))]
    {
        let expected = r#"commit d78c63c5ea3149040767e4387e7fc743cda118fd
Author: author <author@example.com>
Date:   Sat Jan 1 00:00:00 2000 +0000

    r2-change

diff --git a/git-sec/CHANGELOG.md b/gix-sec/CHANGELOG.md
similarity index 100%
rename from git-sec/CHANGELOG.md
rename to gix-sec/CHANGELOG.md
diff --git a/git-sec/Cargo.toml b/gix-sec/Cargo.toml
similarity index 100%
rename from git-sec/Cargo.toml
rename to gix-sec/Cargo.toml
diff --git a/git-sec/src/identity.rs b/gix-sec/src/identity.rs
similarity index 100%
rename from git-sec/src/identity.rs
rename to gix-sec/src/identity.rs
diff --git a/git-sec/src/lib.rs b/gix-sec/src/lib.rs
similarity index 100%
rename from git-sec/src/lib.rs
rename to gix-sec/src/lib.rs
diff --git a/git-sec/src/permission.rs b/gix-sec/src/permission.rs
similarity index 100%
rename from git-sec/src/permission.rs
rename to gix-sec/src/permission.rs
diff --git a/git-sec/src/trust.rs b/gix-sec/src/trust.rs
similarity index 100%
rename from git-sec/src/trust.rs
rename to gix-sec/src/trust.rs
diff --git a/git-sec/tests/identity/mod.rs b/gix-sec/tests/identity/mod.rs
similarity index 100%
rename from git-sec/tests/identity/mod.rs
rename to gix-sec/tests/identity/mod.rs
diff --git a/git-sec/tests/sec.rs b/gix-sec/tests/sec.rs
similarity index 100%
rename from git-sec/tests/sec.rs
rename to gix-sec/tests/sec.rs
"#;
        assert_eq!(
            std::fs::read_to_string(repo_workdir()?.join("baseline-2.with-renames"))?,
            expected
        );
    }

    let out = out.expect("tracking enabled");
    assert_eq!(
        out.num_similarity_checks, 0,
        "similarity checks disabled, and not necessary"
    );
    assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0);
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);

    Ok(())
}

#[test]
fn realistic_renames_3_without_identity() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "r4-base",
        "r4-dir-rename-non-identity",
        Options {
            location: Some(Location::Path),
            rewrites: Some(Rewrites {
                copies: None,
                percentage: None,
                limit: 0,
                track_empty: false,
            }),
        },
    )?;

    // Look how nicely it captures and associates this directory rename.
    insta::assert_debug_snapshot!(changes.into_iter()
                                     .filter(|c| !c.entry_mode().is_tree() ||
                                                  c.relation().is_some_and(|r| matches!(r, Relation::Parent(_)))
                                     ).collect::<Vec<_>>(), @r#"
    [
        Rewrite {
            source_location: "src/plumbing/options.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            source_id: Sha1(00750edc07d6415dcc07ae0351e9397b0222b7ba),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(00750edc07d6415dcc07ae0351e9397b0222b7ba),
            location: "src/plumbing-renamed/options/mod.rs",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            copy: false,
        },
        Rewrite {
            source_location: "src/plumbing/mod.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            source_id: Sha1(0cfbf08886fca9a91cb753ec8734c84fcbe52c9f),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(0cfbf08886fca9a91cb753ec8734c84fcbe52c9f),
            location: "src/plumbing-renamed/mod.rs",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            copy: false,
        },
        Rewrite {
            source_location: "src/plumbing/main.rs",
            source_entry_mode: EntryMode(0o100644),
            source_relation: Some(
                ChildOfParent(
                    2,
                ),
            ),
            source_id: Sha1(d00491fd7e5bb6fa28c517a0bb32b8b506539d4d),
            diff: None,
            entry_mode: EntryMode(0o100644),
            id: Sha1(d00491fd7e5bb6fa28c517a0bb32b8b506539d4d),
            location: "src/plumbing-renamed/main.rs",
            relation: Some(
                ChildOfParent(
                    1,
                ),
            ),
            copy: false,
        },
        Rewrite {
            source_location: "src/plumbing",
            source_entry_mode: EntryMode(0o40000),
            source_relation: Some(
                Parent(
                    2,
                ),
            ),
            source_id: Sha1(b9d41dcdbd92fcab2fb6594d04f2ad99b3472621),
            diff: None,
            entry_mode: EntryMode(0o40000),
            id: Sha1(202702465d7bb291153629dc2e8b353afe9cbdae),
            location: "src/plumbing-renamed",
            relation: Some(
                Parent(
                    1,
                ),
            ),
            copy: false,
        },
    ]
    "#);

    let out = out.expect("tracking enabled");
    assert_eq!(
        out.num_similarity_checks, 0,
        "similarity checks disabled, and not necessary"
    );
    assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0);
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);

    Ok(())
}

mod util {
    use std::{
        convert::Infallible,
        path::{Path, PathBuf},
    };

    use gix_diff::rewrites;
    use gix_object::{FindExt, TreeRefIter};

    pub fn repo_workdir() -> crate::Result<PathBuf> {
        gix_testtools::scripted_fixture_read_only_standalone("make_diff_for_rewrites_repo.sh")
    }

    pub fn repo_with_trees(
        lhs: impl Into<Option<&'static str>>,
        rhs: impl Into<Option<&'static str>>,
    ) -> gix_testtools::Result<(Vec<u8>, Vec<u8>, gix_diff::blob::Platform, gix_odb::Handle)> {
        let root = repo_workdir()?;
        let odb = gix_odb::at(root.join(".git/objects"))?;
        let lhs = read_tree(&odb, &root, lhs.into())?;
        let rhs = read_tree(&odb, &root, rhs.into())?;

        let cache = gix_diff::blob::Platform::new(
            Default::default(),
            gix_diff::blob::Pipeline::new(Default::default(), Default::default(), Vec::new(), Default::default()),
            Default::default(),
            gix_worktree::Stack::new(
                &root,
                gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::default()),
                Default::default(),
                Vec::new(),
                Vec::new(),
            ),
        );
        Ok((lhs, rhs, cache, odb))
    }

    pub fn collect_changes(
        lhs: impl Into<Option<&'static str>>,
        rhs: impl Into<Option<&'static str>>,
    ) -> gix_testtools::Result<(Vec<gix_diff::tree_with_rewrites::Change>, Option<rewrites::Outcome>)> {
        let options = gix_diff::tree_with_rewrites::Options {
            location: Some(gix_diff::tree::recorder::Location::Path),
            rewrites: None,
        };
        collect_changes_opts(lhs, rhs, options)
    }

    pub fn collect_changes_opts(
        lhs: impl Into<Option<&'static str>>,
        rhs: impl Into<Option<&'static str>>,
        options: gix_diff::tree_with_rewrites::Options,
    ) -> gix_testtools::Result<(Vec<gix_diff::tree_with_rewrites::Change>, Option<rewrites::Outcome>)> {
        let (from, to, mut cache, odb) = repo_with_trees(lhs, rhs)?;
        let mut out = Vec::new();
        let rewrites_info = gix_diff::tree_with_rewrites(
            TreeRefIter::from_bytes(&from),
            TreeRefIter::from_bytes(&to),
            &mut cache,
            &mut Default::default(),
            &odb,
            |change| -> Result<_, Infallible> {
                out.push(change.into_owned());
                Ok(std::ops::ControlFlow::Continue(()))
            },
            options,
        )?;
        Ok((out, rewrites_info))
    }

    fn read_tree(odb: &dyn gix_object::Find, root: &Path, tree: Option<&str>) -> gix_testtools::Result<Vec<u8>> {
        let Some(tree) = tree else { return Ok(Vec::new()) };
        let tree_id_path = root.join(tree).with_extension("tree");
        let hex_id = std::fs::read_to_string(&tree_id_path)
            .map_err(|err| std::io::Error::other(format!("Could not read '{}': {}", tree_id_path.display(), err)))?;
        let tree_id = gix_hash::ObjectId::from_hex(hex_id.trim().as_bytes())?;
        let mut buf = Vec::new();
        odb.find_tree(&tree_id, &mut buf)?;
        Ok(buf)
    }
}
use util::{collect_changes, collect_changes_opts, repo_with_trees, repo_workdir};
