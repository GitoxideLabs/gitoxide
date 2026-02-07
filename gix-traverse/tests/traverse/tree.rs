use crate::hex_to_id;
use crate::util::fixture_odb;

fn odb() -> crate::Result<gix_odb::Handle> {
    fixture_odb("make_traversal_repo_for_trees.sh")
}

mod depthfirst {
    use gix_object::FindExt;
    use gix_traverse::{tree, tree::recorder::Location};

    use super::*;
    use crate::util::fixture_odb;

    #[test]
    fn full_path_and_filename() -> crate::Result {
        let db = odb()?;
        let mut state = gix_traverse::tree::depthfirst::State::default();
        let mut buf = state.pop_buf();
        let mut recorder = tree::Recorder::default();
        let tree = db
            .find_commit(&hex_to_id("85df34aa34848b8138b2b3dcff5fb5c2b734e0ce"), &mut buf)?
            .tree();

        gix_traverse::tree::depthfirst(tree, &mut state, &db, &mut recorder)?;
        insta::assert_debug_snapshot!(recorder.records, @r#"
        [
            Entry {
                mode: EntryMode(0o100644),
                filepath: "a",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "b",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "c",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o40000),
                filepath: "d",
                oid: Sha1(496d6428b9cf92981dc9495211e6e1120fb6f2ba),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "d/a",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o40000),
                filepath: "e",
                oid: Sha1(4277b6e69d25e5efa77c455340557b384a4c018a),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "e/b",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o40000),
                filepath: "f",
                oid: Sha1(70fb16fc77b03e16acb4a5b1a6caf79ba302919a),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "f/c",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o40000),
                filepath: "f/d",
                oid: Sha1(5805b676e247eb9a8046ad0c4d249cd2fb2513df),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "f/d/x",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "f/z",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
        ]
        "#);

        recorder.records.clear();
        recorder = recorder.track_location(Some(Location::FileName));
        gix_traverse::tree::depthfirst(tree, state, &db, &mut recorder)?;
        insta::assert_debug_snapshot!(recorder.records, @r#"
        [
            Entry {
                mode: EntryMode(0o100644),
                filepath: "a",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "b",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "c",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o40000),
                filepath: "d",
                oid: Sha1(496d6428b9cf92981dc9495211e6e1120fb6f2ba),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "a",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o40000),
                filepath: "e",
                oid: Sha1(4277b6e69d25e5efa77c455340557b384a4c018a),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "b",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o40000),
                filepath: "f",
                oid: Sha1(70fb16fc77b03e16acb4a5b1a6caf79ba302919a),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "c",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o40000),
                filepath: "d",
                oid: Sha1(5805b676e247eb9a8046ad0c4d249cd2fb2513df),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "x",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "z",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
        ]
        "#);
        Ok(())
    }

    #[test]
    fn more_difficult_fixture() -> crate::Result {
        let db = fixture_odb("make_traversal_repo_for_trees_depthfirst.sh")?;
        let mut state = gix_traverse::tree::depthfirst::State::default();
        let mut buf = state.pop_buf();
        let mut recorder = tree::Recorder::default();
        let tree = db
            .find_commit(&hex_to_id("fe63a8a9fb7c27c089835aae92cbda675523803a"), &mut buf)?
            .tree();

        gix_traverse::tree::depthfirst(tree, &mut state, &db, &mut recorder)?;
        insta::assert_debug_snapshot!(recorder.records.into_iter().filter(|e| e.mode.is_no_tree()).collect::<Vec<_>>(), @r#"
        [
            Entry {
                mode: EntryMode(0o100644),
                filepath: "a",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "b",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "c",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "d/a",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "e/b",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "f/FUNDING.yml",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "f/ISSUE_TEMPLATE/x",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "f/c",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "f/dependabot.yml",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
            Entry {
                mode: EntryMode(0o100644),
                filepath: "f/z",
                oid: Sha1(e69de29bb2d1d6434b8b29ae775ad8c2e48c5391),
            },
        ]
        "#);
        Ok(())
    }
}

mod breadthfirst {
    use gix_object::bstr::BString;
    use gix_odb::pack::FindExt;
    use gix_traverse::{tree, tree::recorder::Location};

    use super::*;

    #[test]
    fn full_path() -> crate::Result {
        let db = odb()?;
        let mut buf = Vec::new();
        let mut buf2 = Vec::new();
        let mut commit = db
            .find_commit_iter(&hex_to_id("85df34aa34848b8138b2b3dcff5fb5c2b734e0ce"), &mut buf)?
            .0;
        // Full paths - that's the default.
        let mut recorder = tree::Recorder::default();
        gix_traverse::tree::breadthfirst(
            db.find_tree_iter(&commit.tree_id().expect("a tree is available in a commit"), &mut buf2)?
                .0,
            tree::breadthfirst::State::default(),
            &db,
            &mut recorder,
        )?;

        use gix_object::tree::EntryKind::*;
        use gix_traverse::tree::recorder::Entry;
        assert_eq!(
            recorder.records,
            vec![
                Entry {
                    mode: Blob.into(),
                    filepath: "a".into(),
                    oid: hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
                },
                Entry {
                    mode: Blob.into(),
                    filepath: "b".into(),
                    oid: hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
                },
                Entry {
                    mode: Blob.into(),
                    filepath: "c".into(),
                    oid: hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
                },
                Entry {
                    mode: Tree.into(),
                    filepath: "d".into(),
                    oid: hex_to_id("496d6428b9cf92981dc9495211e6e1120fb6f2ba")
                },
                Entry {
                    mode: Tree.into(),
                    filepath: "e".into(),
                    oid: hex_to_id("4277b6e69d25e5efa77c455340557b384a4c018a")
                },
                Entry {
                    mode: Tree.into(),
                    filepath: "f".into(),
                    oid: hex_to_id("70fb16fc77b03e16acb4a5b1a6caf79ba302919a")
                },
                Entry {
                    mode: Blob.into(),
                    filepath: "d/a".into(),
                    oid: hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
                },
                Entry {
                    mode: Blob.into(),
                    filepath: "e/b".into(),
                    oid: hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
                },
                Entry {
                    mode: Blob.into(),
                    filepath: "f/c".into(),
                    oid: hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
                },
                Entry {
                    mode: Tree.into(),
                    filepath: "f/d".into(),
                    oid: hex_to_id("5805b676e247eb9a8046ad0c4d249cd2fb2513df")
                },
                Entry {
                    mode: Blob.into(),
                    filepath: "f/z".into(),
                    oid: hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
                },
                Entry {
                    mode: Blob.into(),
                    filepath: "f/d/x".into(),
                    oid: hex_to_id("e69de29bb2d1d6434b8b29ae775ad8c2e48c5391")
                }
            ]
        );
        Ok(())
    }

    #[test]
    fn filename_only() -> crate::Result<()> {
        let db = odb()?;
        let mut buf = Vec::new();
        let mut buf2 = Vec::new();
        let mut commit = db
            .find_commit_iter(&hex_to_id("85df34aa34848b8138b2b3dcff5fb5c2b734e0ce"), &mut buf)?
            .0;
        let mut recorder = tree::Recorder::default().track_location(Some(Location::FileName));
        gix_traverse::tree::breadthfirst(
            db.find_tree_iter(&commit.tree_id().expect("a tree is available in a commit"), &mut buf2)?
                .0,
            tree::breadthfirst::State::default(),
            &db,
            &mut recorder,
        )?;

        assert_eq!(
            recorder.records.into_iter().map(|e| e.filepath).collect::<Vec<_>>(),
            ["a", "b", "c", "d", "e", "f", "a", "b", "c", "d", "z", "x"]
                .into_iter()
                .map(BString::from)
                .collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn no_location() -> crate::Result<()> {
        let db = odb()?;
        let mut buf = Vec::new();
        let mut buf2 = Vec::new();
        let mut commit = db
            .find_commit_iter(&hex_to_id("85df34aa34848b8138b2b3dcff5fb5c2b734e0ce"), &mut buf)?
            .0;
        let mut recorder = tree::Recorder::default().track_location(None);
        gix_traverse::tree::breadthfirst(
            db.find_tree_iter(&commit.tree_id().expect("a tree is available in a commit"), &mut buf2)?
                .0,
            tree::breadthfirst::State::default(),
            &db,
            &mut recorder,
        )?;

        for path in recorder.records.into_iter().map(|e| e.filepath) {
            assert_eq!(path, "", "path should be empty as it's not tracked at all");
        }
        Ok(())
    }
}
