use gix_diff::{
    Rewrites,
    blob::DiffLineStats,
    rewrites,
    rewrites::{
        Copies, CopySource,
        tracker::{
            ChangeKind,
            visit::{Source, SourceKind},
        },
    },
    tree::visit::Relation,
};
use gix_object::tree::EntryKind;
use pretty_assertions::assert_eq;

use crate::{
    hex_to_id,
    rewrites::{Change, NULL_ID},
};

#[test]
fn rename_by_id() -> crate::Result {
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
                    id: *NULL_ID,
                    kind: SourceKind::Rename,
                    location: "b".into(),
                    change: &Change::deletion(),
                    diff: None,
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
fn copy_by_similarity_reports_limit_if_encountered() -> crate::Result {
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
fn copy_by_id() -> crate::Result {
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
                    id,
                    kind: SourceKind::Copy,
                    location: "a".into(),
                    change: &Change {
                        id,
                        ..Change::modification()
                    },
                    diff: None,
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
fn copy_by_id_search_in_all_sources() -> crate::Result {
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
                    id: content_id,
                    kind: SourceKind::Copy,
                    location: "a-src".into(),
                    change: &Change {
                        id: content_id,
                        ..Change::modification()
                    },
                    diff: None,
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
fn copy_by_50_percent_similarity() -> crate::Result {
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
                id,
                kind: SourceKind::Copy,
                location: "a".into(),
                change: &Change {
                    id,
                    ..Change::modification()
                },
                diff: Some(DiffLineStats {
                    removals: 0,
                    insertions: 1,
                    before: 1,
                    after: 2,
                    similarity: 0.6666667,
                }),
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
fn copy_by_id_in_additions_only() -> crate::Result {
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
fn rename_by_similarity_reports_limit_if_encountered() -> crate::Result {
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
fn rename_by_50_percent_similarity() -> crate::Result {
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
                            id,
                            kind: SourceKind::Rename,
                            location: "a".into(),
                            change: &Change {
                                id,
                                ..Change::deletion()
                            },
                            diff: Some(DiffLineStats {
                                removals: 1,
                                insertions: 1,
                                before: 2,
                                after: 2,
                                similarity: 0.53846157
                            })
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

    // The rename tracker currently sorts by hash, hence the outcome is hash-dependent.
    let expected = match crate::fixture_hash_kind() {
        gix_hash::Kind::Sha1 => rewrites::Outcome {
            options: rewrites,
            num_similarity_checks: 1,
            ..Default::default()
        },
        gix_hash::Kind::Sha256 => rewrites::Outcome {
            options: rewrites,
            num_similarity_checks: 2,
            ..Default::default()
        },
        _ => todo!(),
    };
    assert_eq!(
        out, expected,
        "the first attempt already yields the one pair, so it doesn't participate anymore\
 - we don't have best candidates yet, thus only one check"
    );

    Ok(())
}

#[test]
fn directories_without_relation_are_ignored() -> crate::Result {
    let mut track = util::new_tracker(Default::default());
    for mode in [EntryKind::Tree, EntryKind::Commit] {
        let tree_without_relation = Change {
            id: *NULL_ID,
            kind: ChangeKind::Deletion,
            mode: mode.into(),
            relation: None,
        };
        assert_eq!(
            track.try_push_change(tree_without_relation, "dir".into()),
            Some(tree_without_relation),
            "trees and submodules are ignored, particularly when they have no relation"
        );
    }
    Ok(())
}

#[test]
fn directory_renames_by_id_can_fail_gracefully() -> crate::Result {
    let rename_by_similarity = Rewrites {
        copies: None,
        percentage: Some(0.5),
        limit: 0,
        track_empty: false,
    };
    let mut track = util::new_tracker(rename_by_similarity);
    let tree_dst_id = 1;
    let tree_id = hex_to_id(
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    assert!(
        track
            .try_push_change(
                Change {
                    id: tree_id,
                    kind: ChangeKind::Addition,
                    mode: EntryKind::Tree.into(),
                    relation: Some(Relation::Parent(tree_dst_id)),
                },
                "d-renamed".into()
            )
            .is_none()
    );

    let tree_src_id = 3;
    let tree_id = hex_to_id(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    assert!(
        track
            .try_push_change(
                Change {
                    id: tree_id,
                    kind: ChangeKind::Addition,
                    mode: EntryKind::Tree.into(),
                    relation: Some(Relation::Parent(tree_src_id)),
                },
                "d".into()
            )
            .is_none()
    );
    let odb = util::add_retained_blobs(
        &mut track,
        [
            (Change::deletion_in_tree(tree_src_id), "d/a", "a"),
            (Change::deletion_in_tree(tree_src_id), "d/c", "c"),
            (Change::deletion_in_tree(tree_src_id), "d/subdir/d", "d"),
            (Change::addition_in_tree(tree_dst_id), "d-renamed/a", "a"),
            (Change::addition_in_tree(tree_dst_id), "d-renamed/subdir/c", "c"),
            (Change::deletion(), "a", "first\nsecond\n"),
            (Change::addition(), "b", "firt\nsecond\n"),
        ],
    )?;

    // The rename tracker currently sorts by hash, hence the outcome is hash-dependent.
    let mut calls = 0;
    match crate::fixture_hash_kind() {
        gix_hash::Kind::Sha1 => {
            let out = util::assert_emit_with_objects(
                &mut track,
                |dst, src| {
                    match calls {
                        0..=2 => {
                            let src = src.unwrap();
                            let (expected_src, expected_dst) =
                                &[("d/a", "d-renamed/a"), ("d/c", "d-renamed/subdir/c"), ("a", "b")][calls];
                            assert_eq!(src.location, expected_src);
                            assert_eq!(dst.location, expected_dst);
                        }
                        3 => {
                            assert_eq!(src.unwrap().location, "d");
                            assert_eq!(
                                dst.location, "d-renamed",
                                "it can now track modified and renamed directories"
                            );
                        }
                        4 => {
                            assert_eq!(src, None);
                            assert_eq!(dst.change.kind, ChangeKind::Deletion);
                            assert_eq!(dst.location, "d/subdir/d");
                        }
                        _ => unreachable!("Should have expected emission call {calls}"),
                    }
                    calls += 1;
                    std::ops::ControlFlow::Continue(())
                },
                &odb,
            );
            assert_eq!(
                out,
                rewrites::Outcome {
                    options: rename_by_similarity,
                    num_similarity_checks: 1,
                    ..Default::default()
                }
            );
        }
        gix_hash::Kind::Sha256 => {
            let out = util::assert_emit_with_objects(
                &mut track,
                |dst, src| {
                    match calls {
                        0..=2 => {
                            let src = src.unwrap();
                            let (expected_src, expected_dst) =
                                &[("d/c", "d-renamed/subdir/c"), ("d/a", "d-renamed/a"), ("a", "b")][calls];
                            assert_eq!(src.location, expected_src);
                            assert_eq!(dst.location, expected_dst);
                        }
                        3 => {
                            assert_eq!(src.unwrap().location, "d");
                            assert_eq!(
                                dst.location, "d-renamed",
                                "it can now track modified and renamed directories"
                            );
                        }
                        4 => {
                            assert_eq!(src, None);
                            assert_eq!(dst.change.kind, ChangeKind::Deletion);
                            assert_eq!(dst.location, "d/subdir/d");
                        }
                        _ => unreachable!("Should have expected emission call {calls}"),
                    }
                    calls += 1;
                    std::ops::ControlFlow::Continue(())
                },
                &odb,
            );
            assert_eq!(
                out,
                rewrites::Outcome {
                    options: rename_by_similarity,
                    num_similarity_checks: 2,
                    ..Default::default()
                }
            );
            assert_eq!(calls, 5, "Should not have too few calls");
        }
        _ => todo!(),
    }
    assert_eq!(calls, 5, "Should not have too few calls");
    Ok(())
}

#[test]
fn simple_directory_rename_by_id() -> crate::Result {
    let renames_by_identity = Rewrites {
        copies: None,
        percentage: None,
        limit: 0,
        track_empty: false,
    };
    let mut track = util::new_tracker(renames_by_identity);
    let tree_dst_id = 1;
    assert!(
        track
            .try_push_change(Change::tree_addition(tree_dst_id), "d-renamed".into())
            .is_none()
    );
    let tree_src_id = 3;
    assert!(
        track
            .try_push_change(Change::tree_deletion(tree_src_id), "d".into())
            .is_none()
    );
    let tree_id = hex_to_id(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    assert!(
        track
            .try_push_change(
                Change {
                    id: tree_id, /* does not matter for trees */
                    kind: ChangeKind::Deletion,
                    mode: EntryKind::Tree.into(),
                    relation: Some(Relation::ChildOfParent(tree_src_id)),
                },
                "d/subdir".into(),
            )
            .is_none(),
        "trees that are children are kept and matched. That way, they can quickly be pruned which is done first.\
        Those who don't need them can prune them in a later step."
    );
    assert!(
        track
            .try_push_change(
                Change {
                    id: tree_id,
                    kind: ChangeKind::Addition,
                    mode: EntryKind::Tree.into(),
                    relation: Some(Relation::ChildOfParent(tree_dst_id)),
                },
                "d-renamed/subdir".into(),
            )
            .is_none()
    );
    let _odb = util::add_retained_blobs(
        &mut track,
        [
            (Change::deletion_in_tree(tree_src_id), "d/a", "a"),
            (Change::deletion_in_tree(tree_src_id), "d/b", "b"),
            (Change::deletion_in_tree(tree_src_id), "d/c", "c"),
            (Change::deletion_in_tree(tree_src_id), "d/subdir/d", "d"),
            (Change::addition_in_tree(tree_dst_id), "d-renamed/a", "a"),
            (Change::addition_in_tree(tree_dst_id), "d-renamed/b", "b"),
            (Change::addition_in_tree(tree_dst_id), "d-renamed/c", "c"),
            (Change::addition_in_tree(tree_dst_id), "d-renamed/subdir/d", "d"),
            (Change::deletion(), "a", "first\nsecond\n"),
            (Change::addition(), "b", "firt\nsecond\n"),
        ],
    );

    let mut calls = 0;
    match crate::fixture_hash_kind() {
        gix_hash::Kind::Sha1 => {
            let out = util::assert_emit(&mut track, |dst, src| {
                match calls {
                    0 => {
                        let src = src.unwrap();
                        assert_eq!(src.location, "d");
                        assert_eq!(src.entry_mode.kind(), EntryKind::Tree);
                        assert_eq!(src.change.relation, Some(Relation::Parent(3)));
                        assert_eq!(dst.location, "d-renamed", "it found the renamed directory");
                        assert_eq!(dst.change.relation, Some(Relation::Parent(1)));
                        assert_eq!(dst.change.mode.kind(), EntryKind::Tree);
                    }
                    1..=5 => {
                        let src = src.unwrap();
                        let (expected_src, expected_dst) = &[
                            ("d/a", "d-renamed/a"),
                            ("d/c", "d-renamed/c"),
                            ("d/b", "d-renamed/b"),
                            ("d/subdir", "d-renamed/subdir"),
                            ("d/subdir/d", "d-renamed/subdir/d"),
                        ][calls - 1];
                        assert_eq!(src.location, expected_src);
                        assert_eq!(dst.location, expected_dst);
                    }
                    6 => {
                        assert_eq!(src, None);
                        assert_eq!(dst.location, "a");
                    }
                    7 => {
                        assert_eq!(src, None);
                        assert_eq!(dst.location, "b");
                    }
                    _ => unreachable!("Should have expected emission call {calls}"),
                }
                calls += 1;
                std::ops::ControlFlow::Continue(())
            });
            assert_eq!(
                out,
                rewrites::Outcome {
                    options: renames_by_identity,
                    ..Default::default()
                }
            );
        }
        gix_hash::Kind::Sha256 => {
            let out = util::assert_emit(&mut track, |dst, src| {
                match calls {
                    0 => {
                        let src = src.unwrap();
                        assert_eq!(src.location, "d");
                        assert_eq!(src.entry_mode.kind(), EntryKind::Tree);
                        assert_eq!(src.change.relation, Some(Relation::Parent(3)));
                        assert_eq!(dst.location, "d-renamed", "it found the renamed directory");
                        assert_eq!(dst.change.relation, Some(Relation::Parent(1)));
                        assert_eq!(dst.change.mode.kind(), EntryKind::Tree);
                    }
                    1..=5 => {
                        let src = src.unwrap();
                        let (expected_src, expected_dst) = &[
                            ("d/subdir/d", "d-renamed/subdir/d"),
                            ("d/b", "d-renamed/b"),
                            ("d/subdir", "d-renamed/subdir"),
                            ("d/c", "d-renamed/c"),
                            ("d/a", "d-renamed/a"),
                        ][calls - 1];
                        assert_eq!(src.location, expected_src);
                        assert_eq!(dst.location, expected_dst);
                    }
                    6 => {
                        assert_eq!(src, None);
                        assert_eq!(dst.location, "a");
                    }
                    7 => {
                        assert_eq!(src, None);
                        assert_eq!(dst.location, "b");
                    }
                    _ => unreachable!("Should have expected emission call {calls}"),
                }
                calls += 1;
                std::ops::ControlFlow::Continue(())
            });
            assert_eq!(
                out,
                rewrites::Outcome {
                    options: renames_by_identity,
                    ..Default::default()
                }
            );
        }
        _ => todo!(),
    }
    assert_eq!(calls, 8, "Should not have too few calls");
    Ok(())
}

#[test]
fn remove_only() -> crate::Result {
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
fn add_only() -> crate::Result {
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
fn rename_tracking_is_order_independent() -> crate::Result {
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
        pairs.sort();
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
fn copy_source_selection_is_order_independent() -> crate::Result {
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
            pairs.sort();
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

/// Return every ordering of `items`, used to prove order-independence.
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
    use gix_diff::{
        Rewrites, rewrites,
        rewrites::tracker::visit::{Destination, Source},
        tree::visit::Action,
    };

    use crate::{rewrites::Change, util::ObjectDb};

    /// Add `blobs` `(change, location, data)` to tracker that will all be retained. Note that the `id` of the respective change will be adjusted to match.
    pub fn add_retained_blobs<'a>(
        tracker: &mut rewrites::Tracker<Change>,
        blobs: impl IntoIterator<Item = (Change, &'a str, &'a str)>,
    ) -> crate::Result<ObjectDb> {
        let mut db = ObjectDb::default();
        for (mut change, location, data) in blobs {
            change.id = db.insert(data)?;
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
                |cb| -> Result<(), std::io::Error> {
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

    fn new_platform_no_worktree() -> gix_diff::blob::Platform {
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
            gix_filter::Pipeline::default(),
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
