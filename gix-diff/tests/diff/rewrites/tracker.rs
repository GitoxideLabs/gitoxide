use gix_diff::{
    Rewrites, rewrites,
    rewrites::{
        Copies, CopySource, Similarity,
        tracker::{
            ChangeKind,
            visit::{Source, SourceKind},
        },
    },
};
use gix_object::tree::EntryKind;
use gix_testtools::TestResult;
use pretty_assertions::assert_eq;

use crate::{
    hex_to_id,
    rewrites::{Change, ID},
};

#[test]
fn optional_ids_require_content_to_match() -> TestResult {
    #[derive(Clone)]
    struct OptionalId(Option<gix_hash::ObjectId>, ChangeKind);
    impl rewrites::tracker::Change for OptionalId {
        fn id(&self) -> Option<&gix_hash::oid> {
            self.0.as_deref()
        }
        fn kind(&self) -> ChangeKind {
            self.1
        }
        fn entry_mode(&self) -> gix_object::tree::EntryMode {
            EntryKind::Blob.into()
        }
    }
    for old_id in [None, Some(crate::fixture_hash_kind().null())] {
        for new_id in [None, Some(crate::fixture_hash_kind().null())] {
            for worktree in [false, true] {
                for percentage in [None, Some(0.0)] {
                    let mut tracker = rewrites::Tracker::new(Rewrites {
                        percentage,
                        ..Default::default()
                    });
                    tracker.try_push_change(OptionalId(old_id, ChangeKind::Deletion), "a".into());
                    tracker.try_push_change(OptionalId(new_id, ChangeKind::Addition), "b".into());
                    let mut platform = util::new_platform_no_worktree();
                    if worktree {
                        let root = crate::scripted_fixture_read_only("make_blob_repo.sh")?;
                        platform.filter.roots.old_root = Some(root.clone());
                        platform.filter.roots.new_root = Some(root);
                    }
                    let expect_match = worktree && percentage.is_some();
                    let mut emitted = 0;
                    tracker.emit(
                        |_, source| {
                            assert_eq!(
                                source.is_some(),
                                expect_match,
                                "unavailable IDs require a content comparison"
                            );
                            if let Some(source) = source {
                                assert_eq!(source.id, None, "even Some(null) is exposed as unavailable");
                                assert!(source.similarity.is_some(), "None == None never establishes identity");
                            }
                            emitted += 1;
                            std::ops::ControlFlow::Continue(())
                        },
                        &mut platform,
                        &gix_object::find::Never,
                        |_| Ok::<_, std::io::Error>(()),
                    )?;
                    assert_eq!(emitted, if expect_match { 1 } else { 2 });
                }
            }
        }
    }
    Ok(())
}

#[test]
fn unavailable_ids_do_not_match() {
    for mode in [EntryKind::Blob, EntryKind::Link, EntryKind::Commit] {
        for percentage in [None, Some(0.0), Some(0.5)] {
            for source_kind in [ChangeKind::Deletion, ChangeKind::Modification] {
                let mut track = util::new_tracker(Rewrites {
                    percentage,
                    copies: Some(Copies {
                        source: CopySource::FromSetOfModifiedFiles,
                        percentage,
                    }),
                    ..Default::default()
                });
                for (kind, path) in [(source_kind, "old"), (ChangeKind::Addition, "new")] {
                    let change = Change {
                        id: crate::fixture_hash_kind().null(),
                        kind,
                        mode: mode.into(),
                    };
                    track.try_push_change(change, path.into());
                }
                let mut unmatched = 0;
                util::assert_emit(&mut track, |_destination, source| {
                    assert!(source.is_none(), "unavailable IDs cannot establish content equality");
                    unmatched += 1;
                    std::ops::ControlFlow::Continue(())
                });
                assert_eq!(unmatched, 2, "both changes remain unmatched without content");
            }
        }
    }
}

#[test]
fn unavailable_destination_id_can_match_only_with_available_content() -> TestResult {
    for worktree in [false, true] {
        let mut track = util::new_tracker(Rewrites {
            percentage: Some(0.0),
            ..Default::default()
        });
        let objects = util::add_retained_blobs(&mut track, [(Change::deletion(), "old", "a\n")])?;
        track.try_push_change(
            Change {
                id: crate::fixture_hash_kind().null(),
                ..Change::addition()
            },
            "a".into(),
        );
        let mut platform = util::new_platform_no_worktree();
        if worktree {
            platform.filter.roots.new_root = Some(crate::scripted_fixture_read_only("make_blob_repo.sh")?);
        }
        let mut emitted = 0;
        track.emit(
            |_destination, source| {
                assert_eq!(
                    source.is_some(),
                    worktree,
                    "only available content can establish similarity"
                );
                if let Some(source) = source {
                    assert!((source.similarity.expect("matched by content").fraction() - 1.0).abs() < f32::EPSILON);
                }
                emitted += 1;
                std::ops::ControlFlow::Continue(())
            },
            &mut platform,
            &objects,
            |_push| Ok::<_, std::io::Error>(()),
        )?;
        assert_eq!(
            emitted,
            if worktree { 1 } else { 2 },
            "missing content leaves both changes unmatched"
        );
    }
    Ok(())
}

#[test]
fn unavailable_empty_ids_respect_track_empty_and_missing_resources() -> TestResult {
    let root = crate::scripted_fixture_read_only("make_blob_repo.sh")?;
    for track_empty in [false, true] {
        for percentage in [None, Some(0.0), Some(0.5)] {
            for (old_path, new_path, both_present) in [
                ("missing-old", "missing-new", false),
                ("missing-old", "empty-new", false),
                ("empty-old", "missing-new", false),
                ("empty-old", "empty-new", true),
            ] {
                let case = format!("{old_path} -> {new_path}, track_empty={track_empty}, percentage={percentage:?}");
                let mut track = util::new_tracker(Rewrites {
                    track_empty,
                    percentage,
                    ..Default::default()
                });
                for (kind, path) in [(ChangeKind::Deletion, old_path), (ChangeKind::Addition, new_path)] {
                    assert!(
                        track
                            .try_push_change(
                                Change {
                                    id: crate::fixture_hash_kind().null(),
                                    kind,
                                    mode: EntryKind::Blob.into(),
                                },
                                path.into(),
                            )
                            .is_none(),
                        "unknown IDs must be retained until their content is available: {case}"
                    );
                }
                let mut platform = util::new_platform_no_worktree();
                platform.filter.roots.old_root = Some(root.clone());
                platform.filter.roots.new_root = Some(root.clone());
                let expect_match = both_present && track_empty && percentage.is_some();
                let mut emitted = 0;
                track.emit(
                    |destination, source| {
                        assert_eq!(
                            source.is_some(),
                            expect_match,
                            "empty content needs permission to match; missing content and unknown identity never match: {case}"
                        );
                        if let Some(source) = source {
                            assert_eq!(source.kind, SourceKind::Rename, "content establishes a rename: {case}");
                            assert_eq!(source.location, old_path, "the empty source is matched: {case}");
                            assert_eq!(destination.location, new_path, "the empty destination is matched: {case}");
                            assert_eq!(
                                source.similarity.expect("unhashed resources can only match by content"),
                                Similarity::from_score(60_000).expect("perfect similarity is valid"),
                                "two present empty buffers have finite, perfect similarity: {case}"
                            );
                        }
                        emitted += 1;
                        std::ops::ControlFlow::Continue(())
                    },
                    &mut platform,
                    &gix_object::find::Never,
                    |_push| Ok::<_, std::io::Error>(()),
                )?;
                assert_eq!(
                    emitted,
                    if expect_match { 1 } else { 2 },
                    "unmatched changes must both be emitted: {case}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn rename_by_id() -> TestResult {
    // Limits are only applied when doing rewrite-checks
    for limit in [0, 1] {
        let rewrites = Rewrites {
            copies: None,
            percentage: None,
            limit,
            track_empty: false,
        };
        let mut track = util::new_tracker(rewrites);
        assert!(
            track.try_push_change(Change::modification(), "a".into()).is_some(),
            "modifications play no role in rename tracking"
        );
        assert!(
            track.try_push_change(Change::deletion(), "b".into()).is_none(),
            "recorded for later matching"
        );
        assert!(
            track.try_push_change(Change::addition(), "c".into()).is_none(),
            "recorded for later matching"
        );
        let mut called = false;
        let out = util::assert_emit(&mut track, |dst, src| {
            assert!(!called, "only one rename pair is expected");
            called = true;
            assert_eq!(
                src.unwrap(),
                Source {
                    entry_mode: EntryKind::Blob.into(),
                    id: Some(*ID),
                    kind: SourceKind::Rename,
                    location: "b".into(),
                    change: &Change::deletion(),
                    similarity: None,
                }
            );
            assert_eq!(dst.location, "c");
            std::ops::ControlFlow::Continue(())
        });
        assert_eq!(
            out,
            rewrites::Outcome {
                options: rewrites,
                ..Default::default()
            },
            "no similarity check was performed, it was all matched by id"
        );
    }
    Ok(())
}

#[test]
fn gitlinks_are_renamed_only_by_id() {
    let id = hex_to_id(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    let other_id = hex_to_id(
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    let gitlink = |id, kind| Change {
        id,
        kind,
        mode: EntryKind::Commit.into(),
    };

    let mut tracker = util::new_tracker(Default::default());
    assert!(
        tracker
            .try_push_change(gitlink(id, ChangeKind::Deletion), "old".into())
            .is_none()
    );
    assert!(
        tracker
            .try_push_change(gitlink(id, ChangeKind::Addition), "new".into())
            .is_none()
    );
    let mut matched = false;
    util::assert_emit(&mut tracker, |destination, source| {
        assert_eq!(destination.location, "new");
        assert_eq!(source.expect("equal gitlink IDs form an exact rename").location, "old");
        matched = true;
        std::ops::ControlFlow::Continue(())
    });
    assert!(matched, "the exact gitlink rename was emitted");

    let mut tracker = util::new_tracker(Default::default());
    assert!(
        tracker
            .try_push_change(gitlink(id, ChangeKind::Deletion), "old".into())
            .is_none()
    );
    assert!(
        tracker
            .try_push_change(gitlink(other_id, ChangeKind::Addition), "new".into())
            .is_none()
    );
    let mut unmatched = 0;
    util::assert_emit(&mut tracker, |_destination, source| {
        assert!(source.is_none(), "gitlink IDs are never compared by blob similarity");
        unmatched += 1;
        std::ops::ControlFlow::Continue(())
    });
    assert_eq!(unmatched, 2, "both unrelated gitlink changes remain unmatched");
}

#[test]
fn copy_by_similarity_reports_limit_if_encountered() -> TestResult {
    let rewrites = Rewrites {
        copies: Some(Copies {
            source: CopySource::FromSetOfModifiedFiles,
            percentage: Some(0.5),
        }),
        percentage: None,
        limit: 1,
        track_empty: false,
    };
    let mut track = util::new_tracker(rewrites);
    let odb = util::add_retained_blobs(
        &mut track,
        [
            (Change::modification(), "a", "a\n"),
            (Change::addition(), "a-cpy-1", "a"),
            (Change::addition(), "a-cpy-2", "a"),
            (Change::modification(), "d", "ab"),
        ],
    )?;

    let mut calls = 0;
    let out = util::assert_emit_with_objects(
        &mut track,
        |dst, src| {
            assert!(src.is_none());
            match calls {
                0 => assert_eq!(dst.location, "a"),
                1 => assert_eq!(dst.location, "a-cpy-1"),
                2 => assert_eq!(dst.location, "a-cpy-2"),
                3 => assert_eq!(dst.location, "d"),
                _ => panic!("too many emissions"),
            }
            calls += 1;
            std::ops::ControlFlow::Continue(())
        },
        odb,
    );
    assert_eq!(
        out,
        rewrites::Outcome {
            options: rewrites,
            num_similarity_checks_skipped_for_copy_tracking_due_to_limit: 4,
            ..Default::default()
        },
        "no similarity check was performed at all - all or nothing"
    );
    Ok(())
}

#[test]
fn copy_by_id() -> TestResult {
    // Limits are only applied when doing rewrite-checks
    for limit in [0, 1] {
        let rewrites = Rewrites {
            copies: Some(Copies {
                source: CopySource::FromSetOfModifiedFiles,
                percentage: None,
            }),
            percentage: None,
            limit,
            track_empty: false,
        };
        let mut track = util::new_tracker(rewrites);
        let odb = util::add_retained_blobs(
            &mut track,
            [
                (Change::modification(), "a", "a"),
                (Change::addition(), "a-cpy-1", "a"),
                (Change::addition(), "a-cpy-2", "a"),
                (Change::modification(), "d", "a"),
            ],
        )?;

        let mut calls = 0;
        let out = util::assert_emit_with_objects(
            &mut track,
            |dst, src| {
                let id = hex_to_id(
                    "2e65efe2a145dda7ee51d1741299f848e5bf752e",
                    "eb337bcee2061c5313c9a1392116b6c76039e9e30d71467ae359b36277e17dc7",
                );
                let source_a = Source {
                    entry_mode: EntryKind::Blob.into(),
                    id: Some(id),
                    kind: SourceKind::Copy,
                    location: "a".into(),
                    change: &Change {
                        id,
                        ..Change::modification()
                    },
                    similarity: None,
                };
                match calls {
                    0 => {
                        assert_eq!(src.unwrap(), source_a);
                        assert_eq!(
                            dst.location, "a-cpy-1",
                            "it just finds the first possible match in order, ignoring other candidates"
                        );
                    }
                    1 => {
                        assert_eq!(src.unwrap(), source_a, "copy-sources can be used multiple times");
                        assert_eq!(dst.location, "a-cpy-2");
                    }
                    2 => {
                        assert!(src.is_none());
                        assert_eq!(dst.location, "a", "copy sources retain their own modification");
                    }
                    3 => {
                        assert!(src.is_none());
                        assert_eq!(dst.location, "d");
                    }
                    _ => panic!("too many emissions"),
                }
                calls += 1;
                std::ops::ControlFlow::Continue(())
            },
            odb,
        );
        assert_eq!(
            out,
            rewrites::Outcome {
                options: rewrites,
                ..Default::default()
            },
            "no similarity check was performed, it was all matched by id"
        );
    }
    Ok(())
}

#[test]
fn copy_by_id_search_in_all_sources() -> TestResult {
    // Limits are only applied when doing rewrite-checks
    for limit in [0, 1] {
        let rewrites = Rewrites {
            copies: Some(Copies {
                source: CopySource::FromSetOfModifiedFilesAndAllSources,
                percentage: None,
            }),
            percentage: None,
            limit,
            track_empty: false,
        };
        let mut track = util::new_tracker(rewrites);
        let odb = util::add_retained_blobs(
            &mut track,
            [
                (Change::addition(), "a-cpy-1", "a"),
                (Change::addition(), "a-cpy-2", "a"),
            ],
        )?;

        let content_id = hex_to_id(
            "2e65efe2a145dda7ee51d1741299f848e5bf752e",
            "eb337bcee2061c5313c9a1392116b6c76039e9e30d71467ae359b36277e17dc7",
        );

        let mut calls = 0;
        let out = util::assert_emit_with_objects_and_sources(
            &mut track,
            |dst, src| {
                let source_a = Source {
                    entry_mode: EntryKind::Blob.into(),
                    id: Some(content_id),
                    kind: SourceKind::Copy,
                    location: "a-src".into(),
                    change: &Change {
                        id: content_id,
                        ..Change::modification()
                    },
                    similarity: None,
                };
                match calls {
                    0 => {
                        assert_eq!(src.unwrap(), source_a);
                        assert_eq!(
                            dst.location, "a-cpy-1",
                            "it just finds the first possible match in order, ignoring other candidates"
                        );
                    }
                    1 => {
                        assert_eq!(src.unwrap(), source_a, "copy-sources can be used multiple times");
                        assert_eq!(dst.location, "a-cpy-2");
                    }
                    2 => {
                        assert!(src.is_none());
                        assert_eq!(dst.location, "d");
                    }
                    _ => panic!("too many emissions"),
                }
                calls += 1;
                std::ops::ControlFlow::Continue(())
            },
            odb,
            [(
                {
                    let mut c = Change::modification();
                    c.id = content_id;
                    c
                },
                "a-src",
            )],
        );
        assert_eq!(
            out,
            rewrites::Outcome {
                options: rewrites,
                ..Default::default()
            },
            "no similarity check was performed, it was all matched by id"
        );
    }
    Ok(())
}

#[test]
fn copy_by_50_percent_similarity() -> TestResult {
    let rewrites = Rewrites {
        copies: Some(Copies {
            source: CopySource::FromSetOfModifiedFiles,
            percentage: Some(0.5),
        }),
        percentage: None,
        limit: 0,
        track_empty: false,
    };
    let mut track = util::new_tracker(rewrites);
    let odb = util::add_retained_blobs(
        &mut track,
        [
            (Change::modification(), "a", "a\n"),
            (Change::addition(), "a-cpy-1", "a\nb"),
            (Change::addition(), "a-cpy-2", "a\nc"),
            (Change::modification(), "d", "a"),
        ],
    )?;

    let mut calls = 0;
    let out = util::assert_emit_with_objects(
        &mut track,
        |dst, src| {
            let id = hex_to_id(
                "78981922613b2afb6025042ff6bd878ac1994e85",
                "f8625e43f9e04f24291f77cdbe4c71b3c2a3b0003f60419b3ed06a058d766c8b",
            );
            let source_a = Source {
                entry_mode: EntryKind::Blob.into(),
                id: Some(id),
                kind: SourceKind::Copy,
                location: "a".into(),
                change: &Change {
                    id,
                    ..Change::modification()
                },
                similarity: Similarity::from_score(40000),
            };
            match calls {
                0 => {
                    assert_eq!(
                        src.unwrap(),
                        source_a,
                        "it finds the first possible source, no candidates"
                    );
                    assert_eq!(dst.location, "a-cpy-1");
                }
                1 => {
                    assert_eq!(src.unwrap(), source_a, "the same source can be reused as well");
                    assert_eq!(dst.location, "a-cpy-2");
                }
                2 => {
                    assert!(src.is_none());
                    assert_eq!(dst.location, "a", "copy sources retain their own modification");
                }
                3 => {
                    assert!(src.is_none());
                    assert_eq!(dst.location, "d");
                }
                _ => panic!("too many emissions"),
            }
            calls += 1;
            std::ops::ControlFlow::Continue(())
        },
        odb,
    );
    assert_eq!(
        out,
        rewrites::Outcome {
            options: rewrites,
            num_similarity_checks: 4,
            ..Default::default()
        },
        "no similarity check was performed, it was all matched by id"
    );
    Ok(())
}

#[test]
fn copy_by_id_in_additions_only() -> TestResult {
    let rewrites = Rewrites {
        copies: Some(Copies {
            source: CopySource::FromSetOfModifiedFiles,
            percentage: None,
        }),
        percentage: None,
        limit: 0,
        track_empty: false,
    };
    let mut track = util::new_tracker(rewrites);
    let odb = util::add_retained_blobs(
        &mut track,
        [
            (Change::modification(), "a", "a"),
            (Change::modification(), "a-cpy-1", "a"),
        ],
    )?;

    let mut calls = 0;
    let out = util::assert_emit_with_objects(
        &mut track,
        |dst, src| {
            match calls {
                0 => {
                    assert!(src.is_none());
                    assert_eq!(dst.location, "a");
                }
                1 => {
                    assert!(src.is_none());
                    assert_eq!(
                        dst.location, "a-cpy-1",
                        "copy detection is only done for additions, not within modifications"
                    );
                }
                _ => panic!("too many emissions"),
            }
            calls += 1;
            std::ops::ControlFlow::Continue(())
        },
        odb,
    );
    assert_eq!(
        out,
        rewrites::Outcome {
            options: rewrites,
            ..Default::default()
        },
        "no similarity check was performed, it was all matched by id"
    );
    Ok(())
}

#[test]
fn rename_by_similarity_reports_limit_if_encountered() -> TestResult {
    let rewrites = Rewrites {
        copies: None,
        percentage: Some(0.5),
        limit: 1,
        track_empty: false,
    };
    let mut track = util::new_tracker(rewrites);
    let odb = util::add_retained_blobs(
        &mut track,
        [
            (Change::deletion(), "a", "first\nsecond\n"),
            (Change::addition(), "b", "firt\nsecond\n"),
            (Change::addition(), "c", "second\nunrelated\n"),
        ],
    )?;

    let mut calls = 0;
    let out = util::assert_emit_with_objects(
        &mut track,
        |dst, src| {
            assert!(src.is_none());
            match calls {
                0 => assert_eq!(dst.location, "a"),
                1 => assert_eq!(dst.location, "b"),
                2 => assert_eq!(dst.location, "c"),
                _ => panic!("too many elements emitted"),
            }
            calls += 1;
            std::ops::ControlFlow::Continue(())
        },
        odb,
    );
    assert_eq!(
        out,
        rewrites::Outcome {
            options: rewrites,
            num_similarity_checks_skipped_for_rename_tracking_due_to_limit: 2,
            ..Default::default()
        },
        "no similarity check was performed at all - all or nothing"
    );
    Ok(())
}

#[test]
fn rename_by_similarity_limit_applies_per_side() -> TestResult {
    let rewrites = Rewrites {
        copies: None,
        percentage: Some(0.5),
        limit: 2,
        track_empty: false,
    };
    let mut track = util::new_tracker(rewrites);
    let odb = util::add_retained_blobs(
        &mut track,
        [
            (Change::deletion(), "a", "a1\na2\na3\n"),
            (Change::deletion(), "b", "b1\nb2\nb3\n"),
            (Change::addition(), "c", "a1\na2\nA3\n"),
            (Change::addition(), "d", "b1\nb2\nB3\n"),
        ],
    )?;

    let mut pairs = Vec::new();
    let out = util::assert_emit_with_objects(
        &mut track,
        |dst, src| {
            pairs.push((src.map(|src| src.location.to_string()), dst.location.to_string()));
            std::ops::ControlFlow::Continue(())
        },
        odb,
    );
    pairs.sort();
    assert_eq!(
        pairs,
        [
            (Some("a".to_string()), "c".to_string()),
            (Some("b".to_string()), "d".to_string()),
        ],
        "2 sources x 2 destinations fit into a limit of 2, as it allows 2*2 combinations"
    );
    assert_eq!(out.num_similarity_checks_skipped_for_rename_tracking_due_to_limit, 0);
    Ok(())
}

#[test]
fn rename_by_similarity_is_skipped_above_squared_limit() -> TestResult {
    let rewrites = Rewrites {
        copies: None,
        percentage: Some(0.5),
        limit: 2,
        track_empty: false,
    };
    let mut track = util::new_tracker(rewrites);
    let odb = util::add_retained_blobs(
        &mut track,
        [
            (Change::deletion(), "a", "a1\na2\na3\n"),
            (Change::deletion(), "b", "b1\nb2\nb3\n"),
            (Change::deletion(), "e", "e1\ne2\ne3\n"),
            (Change::addition(), "c", "a1\na2\nA3\n"),
            (Change::addition(), "d", "b1\nb2\nB3\n"),
        ],
    )?;

    let out = util::assert_emit_with_objects(
        &mut track,
        |_dst, src| {
            assert!(src.is_none(), "no pairing is attempted above the limit");
            std::ops::ControlFlow::Continue(())
        },
        odb,
    );
    assert_eq!(
        out,
        rewrites::Outcome {
            options: rewrites,
            num_similarity_checks_skipped_for_rename_tracking_due_to_limit: 6,
            ..Default::default()
        },
        "3 sources x 2 destinations exceed the 2*2 combinations a limit of 2 allows"
    );
    Ok(())
}

#[test]
fn rename_by_50_percent_similarity() -> TestResult {
    let rewrites = Rewrites {
        copies: None,
        percentage: Some(0.5),
        limit: 0,
        track_empty: false,
    };
    let mut track = util::new_tracker(rewrites);
    let odb = util::add_retained_blobs(
        &mut track,
        [
            (Change::deletion(), "a", "first\nsecond\n"),
            (Change::addition(), "b", "firt\nsecond\n"),
            (Change::addition(), "c", "second\nunrelated\n"),
        ],
    )?;

    let mut calls = 0;
    let out = util::assert_emit_with_objects(
        &mut track,
        |dst, src| {
            match calls {
                0 => {
                    let id = hex_to_id(
                        "66a52ee7a1d803dc57859c3e95ac9dcdc87c0164",
                        "fdf2fd32de1d33ee5744979b40b07e0930a12c59dfbc5b3b6d34b807f944ba6e",
                    );
                    assert_eq!(
                        src.unwrap(),
                        Source {
                            entry_mode: EntryKind::Blob.into(),
                            id: Some(id),
                            kind: SourceKind::Rename,
                            location: "a".into(),
                            change: &Change {
                                id,
                                ..Change::deletion()
                            },
                            similarity: Similarity::from_score(32307)
                        }
                    );
                    assert_eq!(dst.location, "b");
                }
                1 => {
                    assert!(src.is_none(), "pair already found");
                    assert_eq!(dst.location, "c");
                }
                _ => panic!("too many elements emitted"),
            }
            calls += 1;
            std::ops::ControlFlow::Continue(())
        },
        odb,
    );

    assert_eq!(
        out,
        rewrites::Outcome {
            options: rewrites,
            num_similarity_checks: 2,
            ..Default::default()
        },
        "both destinations are scored before selecting the globally strongest pair"
    );

    Ok(())
}

#[test]
fn rename_by_similarity_prefers_stronger_match_over_same_filename_match() -> TestResult {
    let rewrites = Rewrites {
        copies: None,
        percentage: Some(0.5),
        limit: 0,
        track_empty: false,
    };
    let mut track = util::new_tracker(rewrites);
    let odb = util::add_retained_blobs(
        &mut track,
        [
            (Change::deletion(), "old/foo", "1\n2\n3\n4\n5\n"),
            (Change::deletion(), "bar", "1\n2\n3\n4\n5\n6\n7\n"),
            (Change::addition(), "new/foo", "1\n2\n3\n4\n5\n6\n7\n8\n"),
        ],
    )?;

    let mut calls = 0;
    let out = util::assert_emit_with_objects(
        &mut track,
        |dst, src| {
            match calls {
                0 => {
                    let src = src.expect("destination should find a rename source");
                    assert_eq!(
                        src.location, "bar",
                        "the strongest similarity match wins before basename is used as a tie-breaker"
                    );
                    assert_eq!(dst.location, "new/foo");
                    assert!(
                        src.similarity.expect("inexact matches include their score").fraction() > 0.8,
                        "the selected source should be the high-similarity candidate"
                    );
                }
                1 => {
                    assert_eq!(src, None, "the weaker same-filename source remains unmatched");
                    assert_eq!(dst.location, "old/foo");
                }
                _ => panic!("too many elements emitted"),
            }
            calls += 1;
            std::ops::ControlFlow::Continue(())
        },
        odb,
    );
    assert_eq!(
        out,
        rewrites::Outcome {
            options: rewrites,
            num_similarity_checks: 3,
            ..Default::default()
        },
        "the basename candidate is below the stricter threshold, so the matrix compares both sources"
    );
    assert_eq!(calls, 2, "both the rename and the unmatched deletion should be emitted");
    Ok(())
}

#[test]
fn directories_are_not_tracked() -> TestResult {
    let mut track = util::new_tracker(Rewrites {
        copies: Some(Copies::default()),
        ..Default::default()
    });
    for kind in [ChangeKind::Addition, ChangeKind::Deletion, ChangeKind::Modification] {
        let directory = Change {
            id: *ID,
            kind,
            mode: EntryKind::Tree.into(),
        };
        assert_eq!(
            track.try_push_change(directory, "dir".into()),
            Some(directory),
            "the generic tracker leaves directory changes to its caller"
        );
    }
    util::assert_emit(&mut track, |_, _| panic!("directory changes were not retained"));
    Ok(())
}

#[test]
fn remove_only() -> TestResult {
    let mut track = util::new_tracker(Default::default());
    assert!(
        track.try_push_change(Change::deletion(), "a".into()).is_none(),
        "recorded for later matching"
    );
    let mut called = false;
    let out = util::assert_emit(&mut track, |dst, src| {
        assert!(!called);
        called = true;
        assert_eq!(src, None, "there is just a single deletion, no pair");
        assert_eq!(dst.location, "a");
        assert_eq!(dst.change.kind, ChangeKind::Deletion);
        std::ops::ControlFlow::Continue(())
    });
    assert_eq!(out, Default::default());
    assert!(called);
    Ok(())
}

#[test]
fn add_only() -> TestResult {
    let mut track = util::new_tracker(Default::default());
    assert!(
        track.try_push_change(Change::addition(), "a".into()).is_none(),
        "recorded for later matching - note that this is the starting point of a matching run"
    );
    let mut called = false;
    let out = util::assert_emit(&mut track, |dst, src| {
        assert!(!called);
        called = true;
        assert!(src.is_none(), "there is just a single addition, no pair");
        assert_eq!(dst.location, "a");
        assert_eq!(dst.change.kind, ChangeKind::Addition);
        std::ops::ControlFlow::Continue(())
    });
    assert_eq!(out, Default::default());
    Ok(())
}

#[test]
fn many_additions_without_sources() -> TestResult {
    use std::fmt::Write;

    // Destinations should only be compared to possible sources, not to all other destinations, which would take
    // quadratic time.
    let rewrites = Rewrites {
        copies: Some(Copies {
            source: CopySource::FromSetOfModifiedFiles,
            percentage: Some(0.5),
        }),
        percentage: Some(0.5),
        limit: 0,
        track_empty: false,
    };
    let mut track = util::new_tracker(rewrites);
    let num_additions: usize = 100_000;
    let mut location = String::with_capacity(5);
    for idx in 0..num_additions {
        location.clear();
        write!(&mut location, "{idx}")?;
        assert!(
            track
                .try_push_change(Change::addition(), location.as_str().into())
                .is_none(),
            "recorded for later matching"
        );
    }
    let mut calls = 0;
    let out = util::assert_emit(&mut track, |dst, src| {
        calls += 1;
        assert!(src.is_none(), "there are no sources to pair with");
        assert_eq!(
            dst.change.kind,
            ChangeKind::Addition,
            "only the additions are emitted as destinations"
        );
        std::ops::ControlFlow::Continue(())
    });
    assert_eq!(calls, num_additions, "each addition results in one emitted destination");
    assert_eq!(
        out.num_similarity_checks, 0,
        "without possible sources, no similarity checks are performed"
    );
    Ok(())
}

#[test]
fn rename_tracking_is_order_independent() -> TestResult {
    // #1832: exactly one of several identical-content additions can be matched as the rename of a
    // deletion. Which one is chosen must not depend on the order in which items are pushed - but
    // the parallel dirwalk and index-traversal threads deliver them in a nondeterministic order,
    // which is what makes the corresponding `gix-status` test intermittently fail on CI.
    let renames_by_identity = Rewrites {
        copies: None,
        percentage: None,
        limit: 0,
        track_empty: false,
    };
    let changes = vec![
        (Change::deletion(), "src", "identical\n"),
        (Change::addition(), "a-dest", "identical\n"),
        (Change::addition(), "b-dest", "identical\n"),
    ];

    let mut reference: Option<Vec<(String, Option<String>)>> = None;
    for order in permutations(changes.clone()) {
        let mut track = util::new_tracker(renames_by_identity);
        util::add_retained_blobs(&mut track, order.iter().copied())?;
        let mut pairs = Vec::new();
        util::assert_emit(&mut track, |dst, src| {
            pairs.push((dst.location.to_string(), src.map(|src| src.location.to_string())));
            std::ops::ControlFlow::Continue(())
        });
        match &reference {
            None => reference = Some(pairs),
            Some(reference) => assert_eq!(
                &pairs, reference,
                "rename tracking must produce the same result regardless of the input order"
            ),
        }
    }
    Ok(())
}

#[test]
fn copy_source_selection_is_order_independent() -> TestResult {
    // #1832, exhaustive-copy variant: with copies searched against all sources - including the whole
    // source tree that is pushed in during `emit` - the source chosen for an identical-content
    // destination must not depend on the order items were pushed. This also exercises the second
    // sort (after `push_source_tree`) that the rename test above never reaches.
    let rewrites = Rewrites {
        copies: Some(Copies {
            source: CopySource::FromSetOfModifiedFilesAndAllSources,
            percentage: None,
        }),
        percentage: None,
        limit: 0,
        track_empty: false,
    };
    // The blob id of "a"; all destinations and sources share it, so the copy source is ambiguous and
    // would otherwise be picked based on push order.
    let content_id = hex_to_id(
        "2e65efe2a145dda7ee51d1741299f848e5bf752e",
        "eb337bcee2061c5313c9a1392116b6c76039e9e30d71467ae359b36277e17dc7",
    );

    let mut reference: Option<Vec<(String, Option<String>)>> = None;
    for dests in permutations(vec!["a-cpy-1", "a-cpy-2"]) {
        for sources in permutations(vec!["a-src-1", "a-src-2"]) {
            let mut track = util::new_tracker(rewrites);
            let odb = util::add_retained_blobs(
                &mut track,
                dests.iter().map(|location| (Change::addition(), *location, "a")),
            )?;
            let mut pairs = Vec::new();
            util::assert_emit_with_objects_and_sources(
                &mut track,
                |dst, src| {
                    pairs.push((dst.location.to_string(), src.map(|src| src.location.to_string())));
                    std::ops::ControlFlow::Continue(())
                },
                odb,
                sources.iter().map(|location| {
                    (
                        Change {
                            id: content_id,
                            ..Change::modification()
                        },
                        *location,
                    )
                }),
            );
            match &reference {
                None => reference = Some(pairs),
                Some(reference) => assert_eq!(
                    &pairs, reference,
                    "copy source selection must be independent of the input order"
                ),
            }
        }
    }
    Ok(())
}

/// Inefficient but small implemenation, for use with small inputs only.
fn permutations<T: Clone>(items: Vec<T>) -> Vec<Vec<T>> {
    if items.len() <= 1 {
        return vec![items];
    }
    let mut out = Vec::new();
    for idx in 0..items.len() {
        let mut rest = items.clone();
        let head = rest.remove(idx);
        for mut perm in permutations(rest) {
            perm.insert(0, head.clone());
            out.push(perm);
        }
    }
    out
}
mod util {
    use crate::Result;
    use gix_diff::{
        Rewrites, rewrites,
        rewrites::tracker::visit::{Destination, Source},
        tree::visit::Action,
    };

    use crate::{
        rewrites::Change,
        util::{ObjectDb, insert, object_db},
    };

    /// Add `blobs` `(change, location, data)` to tracker that will all be retained. Note that the `id` of the respective change will be adjusted to match.
    pub fn add_retained_blobs<'a>(
        tracker: &mut rewrites::Tracker<Change>,
        blobs: impl IntoIterator<Item = (Change, &'a str, &'a str)>,
    ) -> Result<ObjectDb> {
        let db = object_db();
        for (mut change, location, data) in blobs {
            change.id = insert(&db, data)?;
            assert!(
                tracker.try_push_change(change, location.into()).is_none(),
                "input changes must be tracked"
            );
        }
        Ok(db)
    }

    pub fn assert_emit(
        tracker: &mut rewrites::Tracker<Change>,
        cb: impl FnMut(Destination<'_, Change>, Option<Source<'_, Change>>) -> Action,
    ) -> rewrites::Outcome {
        assert_emit_with_objects(tracker, cb, gix_object::find::Never)
    }

    pub fn assert_emit_with_objects(
        tracker: &mut rewrites::Tracker<Change>,
        cb: impl FnMut(Destination<'_, Change>, Option<Source<'_, Change>>) -> Action,
        objects: impl gix_object::FindObjectOrHeader,
    ) -> rewrites::Outcome {
        assert_emit_with_objects_and_sources(tracker, cb, objects, None)
    }

    pub fn assert_emit_with_objects_and_sources<'a>(
        tracker: &mut rewrites::Tracker<Change>,
        cb: impl FnMut(Destination<'_, Change>, Option<Source<'_, Change>>) -> Action,
        objects: impl gix_object::FindObjectOrHeader,
        sources: impl IntoIterator<Item = (Change, &'a str)>,
    ) -> rewrites::Outcome {
        let mut sources: Vec<_> = sources.into_iter().collect();
        tracker
            .emit(
                cb,
                &mut new_platform_no_worktree(),
                &objects,
                |cb| -> std::result::Result<(), std::io::Error> {
                    let sources = std::mem::take(&mut sources);
                    if sources.is_empty() {
                        panic!("Should not access more sources unless these are specified");
                    }
                    for (src, location) in sources {
                        cb(src, location.into());
                    }
                    Ok(())
                },
            )
            .expect("emit doesn't fail")
    }

    pub fn new_tracker(rewrites: Rewrites) -> rewrites::Tracker<Change> {
        rewrites::Tracker::new(rewrites)
    }

    pub fn new_platform_no_worktree() -> gix_diff::blob::Platform {
        let root = crate::scripted_fixture_read_only("make_blob_repo.sh").expect("valid fixture");
        let attributes = gix_worktree::Stack::new(
            root,
            gix_worktree::stack::State::AttributesStack(gix_worktree::stack::state::Attributes::new(
                Default::default(),
                None,
                gix_worktree::stack::state::attributes::Source::IdMapping,
                Default::default(),
            )),
            gix_worktree::glob::pattern::Case::Sensitive,
            Vec::new(),
            Vec::new(),
        );
        let filter = gix_diff::blob::Pipeline::new(
            Default::default(),
            gix_filter::Pipeline::new(Default::default(), gix_testtools::object_hash(), Default::default()),
            Vec::new(),
            Default::default(),
        );
        gix_diff::blob::Platform::new(
            Default::default(),
            filter,
            gix_diff::blob::pipeline::Mode::ToGit,
            attributes,
        )
    }
}
