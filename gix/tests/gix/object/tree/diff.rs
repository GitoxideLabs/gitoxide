use std::convert::Infallible;

use gix::object::{blob::diff::lines, tree::diff::Change};
use gix_object::{bstr::ByteSlice, tree::EntryKind};

use crate::named_repo;

#[test]
fn changes_against_tree_modified() -> crate::Result {
    let repo = named_repo("make_diff_repo.sh")?;
    let from = tree_named(&repo, "@^{/c3-modification}~1");
    let to = tree_named(&repo, ":/c3-modification");
    let mut cache = repo.diff_resource_cache(gix_diff::blob::pipeline::Mode::ToGit, Default::default())?;

    let expected_modifications = [
        (EntryKind::Blob, "a\n", EntryKind::Blob, "a\na1\n"),
        (EntryKind::Tree, "", EntryKind::Tree, ""),
        (EntryKind::Blob, "dir/c\n", EntryKind::Blob, "dir/c\ndir/c1\n"),
    ];
    let mut i = 0;

    from.changes()?
        .for_each_to_obtain_tree(&to, |change| -> Result<_, Infallible> {
            let (expected_previous_entry_mode, expected_previous_data, expected_entry_mode, expected_data) =
                expected_modifications[i];

            assert!(
                !change.location().is_empty(),
                "without configuration the location field is set"
            );
            match change {
                Change::Modification {
                    previous_entry_mode,
                    previous_id,
                    entry_mode,
                    id,
                    ..
                } => {
                    assert_eq!(previous_entry_mode.kind(), expected_previous_entry_mode);
                    assert_eq!(entry_mode.kind(), expected_entry_mode);

                    if matches!(entry_mode.kind(), EntryKind::Tree) {
                        i += 1;
                        return Ok(Default::default());
                    }

                    assert_eq!(previous_id.object().unwrap().data.as_bstr(), expected_previous_data);
                    assert_eq!(id.object().unwrap().data.as_bstr(), expected_data);
                }
                Change::Rewrite { .. } | Change::Deletion { .. } | Change::Addition { .. } => {
                    unreachable!("only modification is expected")
                }
            }

            let mut diff = change.diff(&mut cache).expect("objects available");
            let count = diff.line_counts().expect("no diff error").expect("no binary blobs");
            assert_eq!(count.insertions, 1);
            assert_eq!(count.removals, 0);
            diff.lines(|hunk| {
                match hunk {
                    lines::Change::Deletion { .. } => unreachable!("there was no deletion"),
                    lines::Change::Addition { lines } => {
                        assert_eq!(lines.len(), 1);
                        assert_eq!(
                            lines[0],
                            expected_data.as_bytes()[expected_previous_data.len()..]
                                .as_bstr()
                                .trim(),
                            "diffed lines don't have newlines anymore"
                        );
                    }
                    lines::Change::Modification { .. } => unreachable!("there was no modification"),
                }
                Ok::<_, Infallible>(())
            })
            .expect("infallible");

            i += 1;
            Ok(Default::default())
        })?;
    assert_eq!(i, 3);

    let actual = repo.diff_tree_to_tree(&from, &to, None)?;
    insta::assert_debug_snapshot!(actual, @r#"
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

    assert_eq!(
        from.changes()?.stats(&to)?,
        gix::object::tree::diff::Stats {
            lines_added: 2,
            lines_removed: 0,
            files_changed: 2,
        },
        "two files with one added line each"
    );

    Ok(())
}

mod track_rewrites {
    use std::collections::HashMap;
    use std::convert::Infallible;

    use gix::{
        diff::{
            rewrites::{Copies, CopySource},
            Rewrites,
        },
        object::tree::diff::Change,
    };
    use gix_ref::bstr::BStr;

    use crate::object::tree::diff::tree_named;
    use crate::util::named_subrepo_opts;

    #[test]
    #[cfg_attr(
        windows,
        ignore = "Fails on some Window systems, like the fixture doesn't get set up correctly."
    )]
    fn jj_realistic_needs_to_be_more_clever() -> crate::Result {
        let repo = named_subrepo_opts("make_diff_repos.sh", "jj-trackcopy-1", gix::open::Options::isolated())?;

        let mut expected = HashMap::<&BStr, (&BStr, u32)>::new();
        expected.insert(
            "cli/src/commands/file/chmod.rs".into(),
            ("cli/src/commands/chmod.rs".into(), 90),
        );
        expected.insert(
            "cli/src/commands/file/print.rs".into(),
            ("cli/src/commands/cat.rs".into(), 90),
        );
        expected.insert(
            "cli/tests/test_file_chmod_command.rs".into(),
            ("cli/tests/test_chmod_command.rs".into(), 88),
        );
        expected.insert(
            "cli/tests/test_file_print_command.rs".into(),
            ("cli/tests/test_cat_command.rs".into(), 77),
        );

        let from = tree_named(&repo, "@~2");
        let to = tree_named(&repo, "@~1");
        let rewrites = Rewrites {
            copies: Some(Copies {
                source: CopySource::FromSetOfModifiedFiles,
                percentage: Some(0.5),
            }),
            limit: 1000,
            percentage: Some(0.5),
            track_empty: false,
        };
        let out = from
            .changes()?
            .options(|opts| {
                opts.track_rewrites(rewrites.into());
            })
            .for_each_to_obtain_tree(&to, |change| -> Result<_, Infallible> {
                if let Change::Rewrite {
                    source_location,
                    diff: Some(diff),
                    location,
                    ..
                } = change
                {
                    // Round to percentage points to avoid floating point error.
                    let similarity = (diff.similarity * 100.0) as u32;
                    let v = expected.remove(location);
                    assert_eq!(v, Some((source_location, similarity)));
                }
                Ok(Default::default())
            })?;

        assert_eq!(expected, HashMap::new());
        let out = out.expect("tracking enabled");
        assert_eq!(
            out.num_similarity_checks, 21,
            "this probably increases once the algorithm improves"
        );
        assert_eq!(
            out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0,
            "limit disabled"
        );
        assert_eq!(
            out.num_similarity_checks_skipped_for_copy_tracking_due_to_limit, 0,
            "limit disabled"
        );

        let actual: Vec<_> = repo
            .diff_tree_to_tree(
                &from,
                &to,
                Some(gix::diff::Options::default().with_rewrites(Some(rewrites))),
            )?
            .into_iter()
            .filter(|c| !c.entry_mode().is_tree())
            .collect();
        insta::assert_debug_snapshot!(actual, @r#"
        [
            Rewrite {
                source_location: "cli/src/commands/cat.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: None,
                source_id: Sha1(f09e8b0e6bf963d8d6d5b578fea48ff4c9b723fb),
                diff: Some(
                    DiffLineStats {
                        removals: 3,
                        insertions: 20,
                        before: 131,
                        after: 148,
                        similarity: 0.900421,
                    },
                ),
                entry_mode: EntryMode(0o100644),
                id: Sha1(081093be2ba0d2be62d14363f43859355bee2aa2),
                location: "cli/src/commands/file/print.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_cat_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: None,
                source_id: Sha1(8c80364c37b7fc364778efb4214575536e6a1df4),
                diff: Some(
                    DiffLineStats {
                        removals: 17,
                        insertions: 12,
                        before: 123,
                        after: 118,
                        similarity: 0.77923656,
                    },
                ),
                entry_mode: EntryMode(0o100644),
                id: Sha1(45bb2cf6b7fa96a39c95301f619ca3e4cc3eb0f3),
                location: "cli/tests/test_file_print_command.rs",
                relation: None,
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_chmod_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: None,
                source_id: Sha1(c24ae8e04f53b84e09838d232b3e8c0167ccc010),
                diff: Some(
                    DiffLineStats {
                        removals: 13,
                        insertions: 19,
                        before: 244,
                        after: 250,
                        similarity: 0.88720536,
                    },
                ),
                entry_mode: EntryMode(0o100644),
                id: Sha1(8defe631bc82bf35a53cd25083f85664516f412f),
                location: "cli/tests/test_file_chmod_command.rs",
                relation: None,
                copy: false,
            },
            Rewrite {
                source_location: "cli/src/commands/chmod.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: None,
                source_id: Sha1(8f55dec5b81779d23539fa7146d713cc42df70f4),
                diff: Some(
                    DiffLineStats {
                        removals: 0,
                        insertions: 17,
                        before: 124,
                        after: 141,
                        similarity: 0.9060576,
                    },
                ),
                entry_mode: EntryMode(0o100644),
                id: Sha1(94f78deb408d181ccea9da574d0e45ac32a98092),
                location: "cli/src/commands/file/chmod.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Modification {
                location: "CHANGELOG.md",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(f4cb24f79ec2549a3a8a5028d4c43d953f74137d),
                entry_mode: EntryMode(0o100644),
                id: Sha1(5a052b7fb0919218b2ecddffbb341277bd443a5c),
            },
            Addition {
                location: "cli/src/commands/file/mod.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                entry_mode: EntryMode(0o100644),
                id: Sha1(d67f782327ea286136b8532eaf9a509806a87e83),
            },
            Modification {
                location: "cli/src/commands/mod.rs",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(e7e8c4f00412aa9bc9898f396ef9a7597aa64756),
                entry_mode: EntryMode(0o100644),
                id: Sha1(e3a9ec4524d27aa7035a38fd7c5db414809623c4),
            },
            Modification {
                location: "cli/tests/cli-reference@.md.snap",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(5c1985fc3c89a8d0edaedc23f76feb7f5c4cc962),
                entry_mode: EntryMode(0o100644),
                id: Sha1(92853cde19b20cadd74113ea3566c87d4def591b),
            },
            Modification {
                location: "cli/tests/runner.rs",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(a008cb19a57bd44a5a054fced38384b09c9243fc),
                entry_mode: EntryMode(0o100644),
                id: Sha1(5253f0ff160e8b7001a7bd271ca4a07968ff81a3),
            },
            Modification {
                location: "cli/tests/test_acls.rs",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(e7e8f15d7f4c0c50aad13b0f82a632e3d55c33c6),
                entry_mode: EntryMode(0o100644),
                id: Sha1(f644e4c8dd0be6fbe5493b172ce10839bcd9e25c),
            },
            Modification {
                location: "cli/tests/test_diffedit_command.rs",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(85e7db4f01d8be8faa7a020647273399f815f597),
                entry_mode: EntryMode(0o100644),
                id: Sha1(fd57f61e92d4d49b4920c08c3522c066cb03ecd2),
            },
            Modification {
                location: "cli/tests/test_fix_command.rs",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(16ab056981c9ca40cdd4d298feb70510cc3ced37),
                entry_mode: EntryMode(0o100644),
                id: Sha1(e0baefc79038fed0bcf56f2d8c3588a26d5bf985),
            },
            Modification {
                location: "cli/tests/test_global_opts.rs",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(44f49aec05b7dc920cf1f1a554016e74b06ee1c8),
                entry_mode: EntryMode(0o100644),
                id: Sha1(a0c0340e495fa759c0b705dd46cee322aa0d80c8),
            },
            Modification {
                location: "cli/tests/test_immutable_commits.rs",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(ba61cefef4328f126283f25935aab2d04ae2016e),
                entry_mode: EntryMode(0o100644),
                id: Sha1(3d7598b4e4c570eef701f40853ef3e3b0fb224f7),
            },
            Modification {
                location: "cli/tests/test_move_command.rs",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(cbd36dbc76760ed41c968f369b470b45c176dabe),
                entry_mode: EntryMode(0o100644),
                id: Sha1(ac9ad5761637cd731abe1bf4a075fedda7bfc61f),
            },
            Modification {
                location: "cli/tests/test_new_command.rs",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(3e03295d9b4654adccb6cd625376c36d4d38fb3d),
                entry_mode: EntryMode(0o100644),
                id: Sha1(a03b50a8a9c23c68d641b51b7c887ea088cd0d2b),
            },
            Modification {
                location: "cli/tests/test_squash_command.rs",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(f921d5bc423586194bd73419f9814ff072212faa),
                entry_mode: EntryMode(0o100644),
                id: Sha1(ff1c247d4312adb5b372c6d9ff93fa71846ca527),
            },
            Modification {
                location: "cli/tests/test_unsquash_command.rs",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha1(0dcc138981223171df13d35444c7aaee4b502c6f),
                entry_mode: EntryMode(0o100644),
                id: Sha1(b8b29cc0ca0176fafaa97c7421a10ed116bcba8a),
            },
        ]
        "#);

        Ok(())
    }

    #[test]
    #[cfg_attr(
        windows,
        ignore = "Fails on some Window systems, like the fixture doesn't get set up correctly."
    )]
    fn jj_realistic_directory_rename() -> crate::Result {
        let repo = named_subrepo_opts("make_diff_repos.sh", "jj-trackcopy-1", gix::open::Options::isolated())?;

        let from = tree_named(&repo, "@~1");
        let to = tree_named(&repo, "@");
        let actual: Vec<_> = repo
            .diff_tree_to_tree(
                &from,
                &to,
                Some(gix::diff::Options::default().with_rewrites(Some(Rewrites::default()))),
            )?
            .into_iter()
            .collect();
        insta::assert_debug_snapshot!(actual, @r#"
        [
            Rewrite {
                source_location: "cli",
                source_entry_mode: EntryMode(0o40000),
                source_relation: Some(
                    Parent(
                        2,
                    ),
                ),
                source_id: Sha1(f203064a6a81df47498fb415a2064a8ec568ed67),
                diff: None,
                entry_mode: EntryMode(0o40000),
                id: Sha1(f203064a6a81df47498fb415a2064a8ec568ed67),
                location: "c",
                relation: Some(
                    Parent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/src/commands/file/print.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(081093be2ba0d2be62d14363f43859355bee2aa2),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(081093be2ba0d2be62d14363f43859355bee2aa2),
                location: "c/src/commands/file/print.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/src/commands/file",
                source_entry_mode: EntryMode(0o40000),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(0f3bc154b577b84fb5ce31383e25acc99c2f24a5),
                diff: None,
                entry_mode: EntryMode(0o40000),
                id: Sha1(0f3bc154b577b84fb5ce31383e25acc99c2f24a5),
                location: "c/src/commands/file",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests",
                source_entry_mode: EntryMode(0o40000),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(17be3b367831653883a36a2f2a8dea418b8d96b7),
                diff: None,
                entry_mode: EntryMode(0o40000),
                id: Sha1(17be3b367831653883a36a2f2a8dea418b8d96b7),
                location: "c/tests",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_immutable_commits.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(3d7598b4e4c570eef701f40853ef3e3b0fb224f7),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(3d7598b4e4c570eef701f40853ef3e3b0fb224f7),
                location: "c/tests/test_immutable_commits.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_file_print_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(45bb2cf6b7fa96a39c95301f619ca3e4cc3eb0f3),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(45bb2cf6b7fa96a39c95301f619ca3e4cc3eb0f3),
                location: "c/tests/test_file_print_command.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/runner.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(5253f0ff160e8b7001a7bd271ca4a07968ff81a3),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(5253f0ff160e8b7001a7bd271ca4a07968ff81a3),
                location: "c/tests/runner.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/src",
                source_entry_mode: EntryMode(0o40000),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(80e5b08f25f75c2050afbcb794e8434f4cf082f1),
                diff: None,
                entry_mode: EntryMode(0o40000),
                id: Sha1(80e5b08f25f75c2050afbcb794e8434f4cf082f1),
                location: "c/src",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_file_chmod_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(8defe631bc82bf35a53cd25083f85664516f412f),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(8defe631bc82bf35a53cd25083f85664516f412f),
                location: "c/tests/test_file_chmod_command.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/cli-reference@.md.snap",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(92853cde19b20cadd74113ea3566c87d4def591b),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(92853cde19b20cadd74113ea3566c87d4def591b),
                location: "c/tests/cli-reference@.md.snap",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/src/commands/file/chmod.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(94f78deb408d181ccea9da574d0e45ac32a98092),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(94f78deb408d181ccea9da574d0e45ac32a98092),
                location: "c/src/commands/file/chmod.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_new_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(a03b50a8a9c23c68d641b51b7c887ea088cd0d2b),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(a03b50a8a9c23c68d641b51b7c887ea088cd0d2b),
                location: "c/tests/test_new_command.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_global_opts.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(a0c0340e495fa759c0b705dd46cee322aa0d80c8),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(a0c0340e495fa759c0b705dd46cee322aa0d80c8),
                location: "c/tests/test_global_opts.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_move_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(ac9ad5761637cd731abe1bf4a075fedda7bfc61f),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(ac9ad5761637cd731abe1bf4a075fedda7bfc61f),
                location: "c/tests/test_move_command.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_unsquash_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(b8b29cc0ca0176fafaa97c7421a10ed116bcba8a),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(b8b29cc0ca0176fafaa97c7421a10ed116bcba8a),
                location: "c/tests/test_unsquash_command.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/src/commands/file/mod.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(d67f782327ea286136b8532eaf9a509806a87e83),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(d67f782327ea286136b8532eaf9a509806a87e83),
                location: "c/src/commands/file/mod.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_fix_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(e0baefc79038fed0bcf56f2d8c3588a26d5bf985),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(e0baefc79038fed0bcf56f2d8c3588a26d5bf985),
                location: "c/tests/test_fix_command.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/src/commands/mod.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(e3a9ec4524d27aa7035a38fd7c5db414809623c4),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(e3a9ec4524d27aa7035a38fd7c5db414809623c4),
                location: "c/src/commands/mod.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/src/commands",
                source_entry_mode: EntryMode(0o40000),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(f414de88468352d59c129d0e7686fb1e1f387929),
                diff: None,
                entry_mode: EntryMode(0o40000),
                id: Sha1(f414de88468352d59c129d0e7686fb1e1f387929),
                location: "c/src/commands",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_acls.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(f644e4c8dd0be6fbe5493b172ce10839bcd9e25c),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(f644e4c8dd0be6fbe5493b172ce10839bcd9e25c),
                location: "c/tests/test_acls.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_diffedit_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(fd57f61e92d4d49b4920c08c3522c066cb03ecd2),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(fd57f61e92d4d49b4920c08c3522c066cb03ecd2),
                location: "c/tests/test_diffedit_command.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_squash_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: Some(
                    ChildOfParent(
                        2,
                    ),
                ),
                source_id: Sha1(ff1c247d4312adb5b372c6d9ff93fa71846ca527),
                diff: None,
                entry_mode: EntryMode(0o100644),
                id: Sha1(ff1c247d4312adb5b372c6d9ff93fa71846ca527),
                location: "c/tests/test_squash_command.rs",
                relation: Some(
                    ChildOfParent(
                        1,
                    ),
                ),
                copy: false,
            },
        ]
        "#);
        Ok(())
    }
}

fn tree_named(repo: &gix::Repository, rev_spec: impl AsRef<str>) -> gix::Tree {
    repo.rev_parse_single(rev_spec.as_ref())
        .unwrap()
        .object()
        .unwrap()
        .peel_to_kind(gix::object::Kind::Tree)
        .unwrap()
        .into_tree()
}
