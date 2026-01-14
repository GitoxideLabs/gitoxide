use std::str::FromStr;

use gix_diff::{
    index::Change,
    rewrites::{Copies, CopySource},
    Rewrites,
};
use gix_object::bstr::BStr;

#[test]
fn empty_to_new_tree_without_rename_tracking() -> crate::Result {
    let changes = collect_changes_no_renames(None, "c1 - initial").expect("really just an addition - nothing to track");
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Addition {
            location: "a",
            index: 0,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "b",
            index: 1,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "d",
            index: 2,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "dir/c",
            index: 3,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
    ]
    "#);

    {
        let (lhs, rhs, _cache, _odb, mut pathspec) = repo_with_indices(None, "c1 - initial", None)?;
        let err = gix_diff::index(
            &lhs,
            &rhs,
            |_change| Err(std::io::Error::other("custom error")),
            None::<gix_diff::index::RewriteOptions<'_, gix_odb::Handle>>,
            &mut pathspec,
            &mut |_, _, _, _| true,
        )
        .unwrap_err();
        assert_eq!(
            format!("{err:?}"),
            r#"Callback(Custom { kind: Other, error: "custom error" })"#,
            "custom errors made visible and not squelched"
        );
    }
    Ok(())
}

#[test]
fn changes_against_modified_tree_with_filename_tracking() -> crate::Result {
    let changes = collect_changes_no_renames("c2", "c3-modification")?;
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Modification {
            location: "a",
            previous_index: 0,
            previous_entry_mode: Mode(
                FILE,
            ),
            previous_id: Sha1(78981922613b2afb6025042ff6bd878ac1994e85),
            index: 0,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(b4f17b61de71d9b2e54ac9e62b1629ae2d97a6a7),
        },
        Modification {
            location: "dir/c",
            previous_index: 3,
            previous_entry_mode: Mode(
                FILE,
            ),
            previous_id: Sha1(6695780ceb14b05e076a99bbd2babf34723b3464),
            index: 3,
            entry_mode: Mode(
                FILE,
            ),
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
                Some(Rewrites {
                    percentage,
                    track_empty,
                    ..Default::default()
                }),
            )?;
            let actual: Vec<_> = changes
                .into_iter()
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
            Some(Rewrites {
                percentage,
                ..Default::default()
            }),
        ).expect("errors can only happen with IO or ODB access fails");
            insta::assert_debug_snapshot!(changes, @r#"
            [
                Modification {
                    location: "b",
                    previous_index: 0,
                    previous_entry_mode: Mode(
                        FILE,
                    ),
                    previous_id: Sha1(61780798228d17af2d34fce4cfbdf35556832472),
                    index: 0,
                    entry_mode: Mode(
                        FILE,
                    ),
                    id: Sha1(54781fa52cf133fa9d0bf59cfe2ef2621b5ad29f),
                },
                Deletion {
                    location: "dir/c",
                    index: 5,
                    entry_mode: Mode(
                        FILE,
                    ),
                    id: Sha1(40006fcef15a8853a1b7ae186d93b7d680fd29cf),
                },
                Addition {
                    location: "dir/c-moved",
                    index: 5,
                    entry_mode: Mode(
                        FILE,
                    ),
                    id: Sha1(f01e8ddf5adc56985b9a1cda6d7c7ef9e3abe034),
                },
            ]
            "#);
            let out = out.expect("tracking enabled");
            assert_eq!(out.num_similarity_checks, if percentage.is_some() { 1 } else { 0 });
            assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0);
            assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0);
        }
    }

    let (changes, out) = collect_changes_opts(
        "r2-ambiguous",
        "r3-simple",
        Some(Rewrites {
            percentage: Some(0.6),
            limit: 1, // has no effect as it's just one item here.
            ..Default::default()
        }),
    )
    .expect("it found all items at the cut-off point, similar to git");

    insta::assert_debug_snapshot!(changes, @r#"
    [
        Modification {
            location: "b",
            previous_index: 0,
            previous_entry_mode: Mode(
                FILE,
            ),
            previous_id: Sha1(61780798228d17af2d34fce4cfbdf35556832472),
            index: 0,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(54781fa52cf133fa9d0bf59cfe2ef2621b5ad29f),
        },
        Rewrite {
            source_location: "dir/c",
            source_index: 5,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(40006fcef15a8853a1b7ae186d93b7d680fd29cf),
            location: "dir/c-moved",
            index: 5,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(f01e8ddf5adc56985b9a1cda6d7c7ef9e3abe034),
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
        Some(Rewrites {
            limit: 1, // prevent fuzzy tracking from happening
            ..Default::default()
        }),
    )?;
    assert_eq!(
        changes.iter().filter(|c| matches!(c, Change::Rewrite { .. })).count(),
        0,
        "fuzzy tracking is effectively disabled due to limit"
    );

    use gix_diff::index::ChangeRef;

    let actual_locations: Vec<_> = changes.iter().map(ChangeRef::location).collect();
    assert_eq!(actual_locations, ["f1", "f1-renamed", "f2", "f2-renamed"]);

    let actual_indices: Vec<_> = changes.iter().map(ChangeRef::index).collect();
    assert_eq!(actual_indices, [6, 6, 7, 7]);

    use gix_index::entry::Mode;

    let actual_entry_modes: Vec<_> = changes.iter().map(ChangeRef::entry_mode).collect();
    assert_eq!(actual_entry_modes, [Mode::FILE, Mode::FILE, Mode::FILE, Mode::FILE]);

    let actual_ids: Vec<_> = changes.iter().map(ChangeRef::id).collect();
    assert_eq!(
        actual_ids,
        [
            gix_hash::ObjectId::from_str("f00c965d8307308469e537302baa73048488f162")?,
            gix_hash::ObjectId::from_str("683cfcc0f47566c332aa45d81c5cc98acb4aab49")?,
            gix_hash::ObjectId::from_str("3bb459b831ea471b9cd1cbb7c6d54a74251a711b")?,
            gix_hash::ObjectId::from_str("0a805f8e02d72bd354c1f00607906de2e49e00d6")?,
        ]
    );

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
        Some(Rewrites {
            copies: Some(Copies {
                source: CopySource::FromSetOfModifiedFiles,
                percentage: None,
            }),
            limit: 1, // the limit isn't actually used for identity based checks
            ..Default::default()
        }),
    )?;
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Rewrite {
            source_location: "base",
            source_index: 3,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(f00c965d8307308469e537302baa73048488f162),
            location: "c1",
            index: 4,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(f00c965d8307308469e537302baa73048488f162),
            copy: true,
        },
        Rewrite {
            source_location: "base",
            source_index: 3,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(f00c965d8307308469e537302baa73048488f162),
            location: "c2",
            index: 5,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(f00c965d8307308469e537302baa73048488f162),
            copy: true,
        },
        Rewrite {
            source_location: "base",
            source_index: 3,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(f00c965d8307308469e537302baa73048488f162),
            location: "dir/c3",
            index: 9,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(f00c965d8307308469e537302baa73048488f162),
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
        Some(Rewrites {
            copies: Some(Copies::default()),
            ..Default::default()
        }),
    )?;
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Rewrite {
            source_location: "base",
            source_index: 3,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            location: "c4",
            index: 6,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            copy: true,
        },
        Rewrite {
            source_location: "base",
            source_index: 3,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            location: "c5",
            index: 7,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(08fe19ca4d2f79624f35333157d610811efc1aed),
            copy: true,
        },
        Rewrite {
            source_location: "base",
            source_index: 3,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            location: "dir/c6",
            index: 12,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(cf7a729ca69bfabd0995fc9b083e86a18215bd91),
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
        Some(Rewrites {
            copies: Some(Copies::default()),
            ..Default::default()
        }),
    )?;
    assert_eq!(
        changes.iter().filter(|c| matches!(c, Change::Rewrite { .. })).count(),
        0,
        "needs --find-copies-harder to detect rewrites here"
    );
    let actual: Vec<_> = changes.iter().map(gix_diff::index::ChangeRef::location).collect();
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
        Some(Rewrites {
            copies: Some(Copies {
                source: CopySource::FromSetOfModifiedFilesAndAllSources,
                ..Default::default()
            }),
            ..Default::default()
        }),
    )?;

    // As the full-tree traversal order is different, it sees candidates in different order.
    // Let's keep this as expectations, as in future there might be a candidate-based search that considers filenames
    // or similarity in names.
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Rewrite {
            source_location: "base",
            source_index: 3,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            location: "c6",
            index: 8,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            copy: true,
        },
        Rewrite {
            source_location: "r/c3di",
            source_index: 12,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(cf7a729ca69bfabd0995fc9b083e86a18215bd91),
            location: "c7",
            index: 9,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(cf7a729ca69bfabd0995fc9b083e86a18215bd91),
            copy: true,
        },
        Rewrite {
            source_location: "c5",
            source_index: 7,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(08fe19ca4d2f79624f35333157d610811efc1aed),
            location: "newly-added",
            index: 19,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(97b3d1a5707f8a11fa5fa8bc6c3bd7b3965601fd),
            copy: true,
        },
        Modification {
            location: "b",
            previous_index: 0,
            previous_entry_mode: Mode(
                FILE,
            ),
            previous_id: Sha1(54781fa52cf133fa9d0bf59cfe2ef2621b5ad29f),
            index: 0,
            entry_mode: Mode(
                FILE,
            ),
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
        Some(Rewrites {
            copies: Some(Copies {
                source: CopySource::FromSetOfModifiedFilesAndAllSources,
                ..Default::default()
            }),
            limit: 2, // similarity checks can't be made that way
            track_empty: false,
            ..Default::default()
        }),
    )?;

    // Again, it finds a different first match for the rewrite compared to tree-traversal, expected for now.
    insta::assert_debug_snapshot!(changes, @r#"
    [
        Rewrite {
            source_location: "base",
            source_index: 3,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            location: "c6",
            index: 8,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(3bb459b831ea471b9cd1cbb7c6d54a74251a711b),
            copy: true,
        },
        Rewrite {
            source_location: "r/c3di",
            source_index: 12,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(cf7a729ca69bfabd0995fc9b083e86a18215bd91),
            location: "c7",
            index: 9,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(cf7a729ca69bfabd0995fc9b083e86a18215bd91),
            copy: true,
        },
        Modification {
            location: "b",
            previous_index: 0,
            previous_entry_mode: Mode(
                FILE,
            ),
            previous_id: Sha1(54781fa52cf133fa9d0bf59cfe2ef2621b5ad29f),
            index: 0,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(f198d0640214092732566fb00543163845c8252c),
        },
        Addition {
            location: "newly-added",
            index: 19,
            entry_mode: Mode(
                FILE,
            ),
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
    assert_eq!(out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 21);

    Ok(())
}

#[test]
fn realistic_renames_by_identity() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "r1-base",
        "r1-change",
        Some(Rewrites {
            copies: Some(Copies::default()),
            limit: 1,
            track_empty: true,
            ..Default::default()
        }),
    )?;

    insta::assert_debug_snapshot!(changes.into_iter().collect::<Vec<_>>(), @r#"
    [
        Rewrite {
            source_location: "git-index/src/file.rs",
            source_index: 18,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "git-index/src/file/mod.rs",
            index: 19,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            copy: false,
        },
        Addition {
            location: "git-index/tests/index/file/access.rs",
            index: 45,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Modification {
            location: "git-index/tests/index/file/mod.rs",
            previous_index: 45,
            previous_entry_mode: Mode(
                FILE,
            ),
            previous_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            index: 46,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(8ba3a16384aacc37d01564b28401755ce8053f51),
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
fn realistic_renames_disabled() -> crate::Result {
    let changes = collect_changes_no_renames("r1-base", "r1-change")?;
    insta::assert_debug_snapshot!(changes.into_iter().collect::<Vec<_>>(), @r#"
    [
        Deletion {
            location: "git-index/src/file.rs",
            index: 18,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "git-index/src/file/mod.rs",
            index: 19,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "git-index/tests/index/file/access.rs",
            index: 45,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Modification {
            location: "git-index/tests/index/file/mod.rs",
            previous_index: 45,
            previous_entry_mode: Mode(
                FILE,
            ),
            previous_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            index: 46,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(8ba3a16384aacc37d01564b28401755ce8053f51),
        },
    ]
    "#);
    Ok(())
}

#[test]
fn realistic_renames_disabled_3() -> crate::Result {
    let changes = collect_changes_no_renames("r3-base", "r3-change")?;

    insta::assert_debug_snapshot!(changes.into_iter().collect::<Vec<_>>(), @r#"
    [
        Addition {
            location: "src/ein.rs",
            index: 0,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "src/gix.rs",
            index: 1,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "src/plumbing-cli.rs",
            index: 0,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "src/porcelain-cli.rs",
            index: 4,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
    ]
    "#);

    Ok(())
}

#[test]
fn realistic_renames_by_identity_3() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "r3-base",
        "r3-change",
        Some(Rewrites {
            copies: Some(Copies::default()),
            limit: 1,
            track_empty: true,
            ..Default::default()
        }),
    )?;

    insta::assert_debug_snapshot!(changes.into_iter().collect::<Vec<_>>(), @r#"
    [
        Rewrite {
            source_location: "src/plumbing-cli.rs",
            source_index: 0,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "src/ein.rs",
            index: 0,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            copy: false,
        },
        Rewrite {
            source_location: "src/porcelain-cli.rs",
            source_index: 4,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            location: "src/gix.rs",
            index: 1,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
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

#[test]
fn realistic_renames_2() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "r2-base",
        "r2-change",
        Some(Rewrites {
            copies: Some(Copies::default()),
            track_empty: false,
            ..Default::default()
        }),
    )?;

    // We cannot capture renames if track-empty is disabled, as these are actually empty,
    // and we can't take directory-shortcuts here (i.e. tracking knows no directories here
    // as is the case with trees where we traverse breadth-first.
    insta::assert_debug_snapshot!(changes.into_iter().collect::<Vec<_>>(), @r#"
    [
        Deletion {
            location: "git-sec/CHANGELOG.md",
            index: 3,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/Cargo.toml",
            index: 4,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/src/identity.rs",
            index: 5,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/src/lib.rs",
            index: 6,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/src/permission.rs",
            index: 7,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/src/trust.rs",
            index: 8,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/tests/identity/mod.rs",
            index: 9,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Deletion {
            location: "git-sec/tests/sec.rs",
            index: 10,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/CHANGELOG.md",
            index: 231,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/Cargo.toml",
            index: 232,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/src/identity.rs",
            index: 233,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/src/lib.rs",
            index: 234,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/src/permission.rs",
            index: 235,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/src/trust.rs",
            index: 236,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/tests/identity/mod.rs",
            index: 237,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
        },
        Addition {
            location: "gix-sec/tests/sec.rs",
            index: 238,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
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

#[test]
fn realistic_renames_3_without_identity() -> crate::Result {
    let (changes, out) = collect_changes_opts(
        "r4-base",
        "r4-dir-rename-non-identity",
        Some(Rewrites {
            copies: None,
            percentage: None,
            limit: 0,
            track_empty: false,
        }),
    )?;

    // We don't actually track directory renames, only files show up.
    insta::assert_debug_snapshot!(changes.into_iter().collect::<Vec<_>>(), @r#"
    [
        Rewrite {
            source_location: "src/plumbing/options.rs",
            source_index: 4,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(00750edc07d6415dcc07ae0351e9397b0222b7ba),
            location: "src/plumbing-renamed/options/mod.rs",
            index: 4,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(00750edc07d6415dcc07ae0351e9397b0222b7ba),
            copy: false,
        },
        Rewrite {
            source_location: "src/plumbing/mod.rs",
            source_index: 3,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(0cfbf08886fca9a91cb753ec8734c84fcbe52c9f),
            location: "src/plumbing-renamed/mod.rs",
            index: 3,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(0cfbf08886fca9a91cb753ec8734c84fcbe52c9f),
            copy: false,
        },
        Rewrite {
            source_location: "src/plumbing/main.rs",
            source_index: 2,
            source_entry_mode: Mode(
                FILE,
            ),
            source_id: Sha1(d00491fd7e5bb6fa28c517a0bb32b8b506539d4d),
            location: "src/plumbing-renamed/main.rs",
            index: 2,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(d00491fd7e5bb6fa28c517a0bb32b8b506539d4d),
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

    let (changes, _out) = collect_changes_opts_with_pathspec(
        "r4-base",
        "r4-dir-rename-non-identity",
        Some(Rewrites {
            copies: None,
            percentage: None,
            limit: 0,
            track_empty: false,
        }),
        Some("src/plumbing/m*"),
    )?;

    // Pathspecs are applied in advance, which affects rename tracking.
    insta::assert_debug_snapshot!(changes.into_iter().collect::<Vec<_>>(), @r#"
    [
        Deletion {
            location: "src/plumbing/main.rs",
            index: 2,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(d00491fd7e5bb6fa28c517a0bb32b8b506539d4d),
        },
        Deletion {
            location: "src/plumbing/mod.rs",
            index: 3,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(0cfbf08886fca9a91cb753ec8734c84fcbe52c9f),
        },
    ]
    "#);

    let (changes, _out) = collect_changes_opts_with_pathspec(
        "r4-base",
        "r4-dir-rename-non-identity",
        Some(Rewrites {
            copies: None,
            percentage: None,
            limit: 0,
            track_empty: false,
        }),
        Some("src/plumbing-renamed/m*"),
    )?;
    // One can also get the other side of the rename
    insta::assert_debug_snapshot!(changes.into_iter().collect::<Vec<_>>(), @r#"
    [
        Addition {
            location: "src/plumbing-renamed/main.rs",
            index: 2,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(d00491fd7e5bb6fa28c517a0bb32b8b506539d4d),
        },
        Addition {
            location: "src/plumbing-renamed/mod.rs",
            index: 3,
            entry_mode: Mode(
                FILE,
            ),
            id: Sha1(0cfbf08886fca9a91cb753ec8734c84fcbe52c9f),
        },
    ]
    "#);

    Ok(())
}

#[test]
fn unmerged_entries_and_intent_to_add() -> crate::Result {
    let (changes, _out) = collect_changes_opts(
        "r4-dir-rename-non-identity",
        ".git/index",
        Some(Rewrites {
            copies: None,
            percentage: None,
            limit: 0,
            track_empty: false,
        }),
    )?;

    // Intent-to-add is transparent. And unmerged entries aren't emitted either, along with
    // their sibling paths.
    // All that with rename tracking…
    insta::assert_debug_snapshot!(changes.into_iter().collect::<Vec<_>>(), @"[]");

    let changes = collect_changes_no_renames("r4-dir-rename-non-identity", ".git/index")?;
    // …or without
    insta::assert_debug_snapshot!(changes.into_iter().collect::<Vec<_>>(), @"[]");

    let (index, _, _, _, _) = repo_with_indices(".git/index", ".git/index", None)?;
    assert_eq!(
        index.entry_by_path("will-add".into()).map(|e| e.id),
        Some(hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")),
        "the file is there, but we don't see it"
    );

    Ok(())
}

mod util {
    use std::{
        convert::Infallible,
        path::{Path, PathBuf},
    };

    use gix_diff::rewrites;

    fn repo_workdir() -> crate::Result<PathBuf> {
        gix_testtools::scripted_fixture_read_only_standalone("make_diff_for_rewrites_repo.sh")
    }

    pub fn repo_with_indices(
        lhs: impl Into<Option<&'static str>>,
        rhs: impl Into<Option<&'static str>>,
        patterns: impl IntoIterator<Item = &'static str>,
    ) -> gix_testtools::Result<(
        gix_index::State,
        gix_index::State,
        gix_diff::blob::Platform,
        gix_odb::Handle,
        gix_pathspec::Search,
    )> {
        let root = repo_workdir()?;
        let odb = gix_odb::at(root.join(".git/objects"))?;
        let lhs = read_index(&odb, &root, lhs.into())?;
        let rhs = read_index(&odb, &root, rhs.into())?;

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
        let pathspecs = gix_pathspec::Search::from_specs(
            patterns
                .into_iter()
                .map(|p| gix_pathspec::Pattern::from_bytes(p.as_bytes(), Default::default()).expect("valid pattern")),
            None,
            &root,
        )?;
        Ok((lhs, rhs, cache, odb, pathspecs))
    }

    pub fn collect_changes_no_renames(
        lhs: impl Into<Option<&'static str>>,
        rhs: impl Into<Option<&'static str>>,
    ) -> gix_testtools::Result<Vec<gix_diff::index::Change>> {
        Ok(collect_changes_opts(lhs, rhs, None)?.0)
    }

    pub fn collect_changes_opts(
        lhs: impl Into<Option<&'static str>>,
        rhs: impl Into<Option<&'static str>>,
        options: Option<gix_diff::Rewrites>,
    ) -> gix_testtools::Result<(Vec<gix_diff::index::Change>, Option<rewrites::Outcome>)> {
        collect_changes_opts_with_pathspec(lhs, rhs, options, None)
    }

    pub fn collect_changes_opts_with_pathspec(
        lhs: impl Into<Option<&'static str>>,
        rhs: impl Into<Option<&'static str>>,
        options: Option<gix_diff::Rewrites>,
        patterns: impl IntoIterator<Item = &'static str>,
    ) -> gix_testtools::Result<(Vec<gix_diff::index::Change>, Option<rewrites::Outcome>)> {
        let (from, to, mut cache, odb, mut pathspecs) = repo_with_indices(lhs, rhs, patterns)?;
        let mut out = Vec::new();
        let rewrites_info = gix_diff::index(
            &from,
            &to,
            |change| -> Result<_, Infallible> {
                out.push(change.into_owned());
                Ok(std::ops::ControlFlow::Continue(()))
            },
            options.map(|rewrites| gix_diff::index::RewriteOptions {
                rewrites,
                resource_cache: &mut cache,
                find: &odb,
            }),
            &mut pathspecs,
            &mut |_, _, _, _| false,
        )?;
        Ok((out, rewrites_info))
    }

    fn read_index(
        odb: impl gix_object::Find,
        root: &Path,
        tree: Option<&str>,
    ) -> gix_testtools::Result<gix_index::State> {
        let Some(tree) = tree else {
            return Ok(gix_index::State::new(gix_hash::Kind::Sha1));
        };
        if tree == ".git/index" {
            Ok(gix_index::File::at(root.join(tree), gix_hash::Kind::Sha1, false, Default::default())?.into())
        } else {
            let tree_id_path = root.join(tree).with_extension("tree");
            let hex_id = std::fs::read_to_string(&tree_id_path).map_err(|err| {
                std::io::Error::other(format!("Could not read '{}': {}", tree_id_path.display(), err))
            })?;
            let tree_id = gix_hash::ObjectId::from_hex(hex_id.trim().as_bytes())?;
            Ok(gix_index::State::from_tree(&tree_id, odb, Default::default())?)
        }
    }
}
use util::{collect_changes_no_renames, collect_changes_opts, collect_changes_opts_with_pathspec, repo_with_indices};

use crate::hex_to_id;
