use gix_testtools::TestResult;

use gix::object::{blob::diff::lines, tree::diff::Change};
use gix_object::{bstr::ByteSlice, tree::EntryKind};

use crate::named_repo;

#[test]
fn changes_against_tree_modified() -> TestResult {
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

    from.changes()?.for_each_to_obtain_tree(&to, |change| {
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
                    return Ok(std::ops::ControlFlow::Continue(()));
                }

                assert_eq!(previous_id.object().unwrap().data.as_bstr(), expected_previous_data);
                assert_eq!(id.object().unwrap().data.as_bstr(), expected_data);
            }
            Change::Rewrite { .. } | Change::Deletion { .. } | Change::Addition { .. } => {
                unreachable!("only modification is expected")
            }
        }

        let mut diff = change.diff(&mut cache)?;
        let count = diff.line_counts()?.expect("no binary blobs");
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
            Ok(())
        })?;

        i += 1;
        Ok(std::ops::ControlFlow::Continue(()))
    })?;
    assert_eq!(i, 3);

    let actual = repo.diff_tree_to_tree(&from, &to, None)?;
    match repo.object_hash() {
        gix::hash::Kind::Sha1 => insta::assert_debug_snapshot!(actual, @r#"
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
        "#),
        gix::hash::Kind::Sha256 => insta::assert_debug_snapshot!(actual, @r#"
        [
            Modification {
                location: "a",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha256(f8625e43f9e04f24291f77cdbe4c71b3c2a3b0003f60419b3ed06a058d766c8b),
                entry_mode: EntryMode(0o100644),
                id: Sha256(6eba3a291223ebd59db7f157409b70bb1f929be92ed8df0fba98cb81852ff378),
            },
            Modification {
                location: "dir",
                previous_entry_mode: EntryMode(0o40000),
                previous_id: Sha256(0d8c11bd683002c6f7c29c94e952cc3ebe2e0e10bd781846285998a509ebd746),
                entry_mode: EntryMode(0o40000),
                id: Sha256(730477dc62b02d7046105a95e8c37dc204943f24c70da12d33a88012ac7d2613),
            },
            Modification {
                location: "dir/c",
                previous_entry_mode: EntryMode(0o100644),
                previous_id: Sha256(b7bb7be9f73f0d01719f57dcb697689692e2ae9737f4bb59216902b9a0e2adb1),
                entry_mode: EntryMode(0o100644),
                id: Sha256(8cf8e3576bb1c632c9f2e1196d2aa6b1ae36c6f651d7619dca4894b784065819),
            },
        ]
        "#),
        _ => unreachable!(),
    }

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
    use gix_testtools::TestResult;
    use std::collections::HashMap;

    use gix::{
        diff::{
            Rewrites,
            rewrites::{Copies, CopySource},
        },
        object::tree::diff::Change,
    };
    use gix_ref::bstr::BStr;

    use crate::{object::tree::diff::tree_named, util::named_subrepo_opts};

    #[test]
    fn jj_realistic_renames() -> TestResult {
        // The fixture reconstructs trees containing SHA-1 object IDs.
        if gix_testtools::object_hash() == gix::hash::Kind::Sha256 {
            return Ok(());
        }
        let repo = named_subrepo_opts("make_diff_repos.sh", "jj-trackcopy-1", gix::open::Options::isolated())?;

        let mut expected = HashMap::<&BStr, (&BStr, u8)>::new();
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
            .for_each_to_obtain_tree(&to, |change| {
                if let Change::Rewrite {
                    source_location,
                    similarity: Some(similarity),
                    location,
                    ..
                } = change
                {
                    let similarity = similarity.percentage();
                    let v = expected.remove(location);
                    assert_eq!(v, Some((source_location, similarity)));
                }
                Ok(std::ops::ControlFlow::Continue(()))
            })?;

        assert_eq!(expected, HashMap::new());
        let out = out.expect("tracking enabled");
        assert_eq!(
            out.num_similarity_checks, 85,
            "Git's exhaustive rename and copy matching considers competing candidates"
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
                source_location: "cli/src/commands/chmod.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: None,
                source_id: Sha1(8f55dec5b81779d23539fa7146d713cc42df70f4),
                similarity: Some(
                    Similarity(
                        54363,
                    ),
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
            Rewrite {
                source_location: "cli/src/commands/cat.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: None,
                source_id: Sha1(f09e8b0e6bf963d8d6d5b578fea48ff4c9b723fb),
                similarity: Some(
                    Similarity(
                        54025,
                    ),
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
                source_location: "cli/tests/test_chmod_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: None,
                source_id: Sha1(c24ae8e04f53b84e09838d232b3e8c0167ccc010),
                similarity: Some(
                    Similarity(
                        53262,
                    ),
                ),
                entry_mode: EntryMode(0o100644),
                id: Sha1(8defe631bc82bf35a53cd25083f85664516f412f),
                location: "cli/tests/test_file_chmod_command.rs",
                relation: None,
                copy: false,
            },
            Rewrite {
                source_location: "cli/tests/test_cat_command.rs",
                source_entry_mode: EntryMode(0o100644),
                source_relation: None,
                source_id: Sha1(8c80364c37b7fc364778efb4214575536e6a1df4),
                similarity: Some(
                    Similarity(
                        46754,
                    ),
                ),
                entry_mode: EntryMode(0o100644),
                id: Sha1(45bb2cf6b7fa96a39c95301f619ca3e4cc3eb0f3),
                location: "cli/tests/test_file_print_command.rs",
                relation: None,
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
    fn jj_realistic_directory_rename() -> TestResult {
        // The test case only works (and is only needed) for SHA-1.
        // Ideally this can be ported to SHA-256 once rename tracking is par with Git.
        if gix_testtools::object_hash() == gix::hash::Kind::Sha256 {
            return Ok(());
        }
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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
                similarity: None,
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

fn tree_named(repo: &gix::Repository, rev_spec: impl AsRef<str>) -> gix::Tree<'_> {
    repo.rev_parse_single(rev_spec.as_ref())
        .unwrap()
        .object()
        .unwrap()
        .peel_to_kind(gix::object::Kind::Tree)
        .unwrap()
        .into_tree()
}
