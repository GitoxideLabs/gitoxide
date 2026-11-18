use bstr::ByteSlice;
use gix_index::{
    State,
    entry::{Flags, Mode, Stage, Stat},
};
use gix_testtools::TestResult;

use crate::Fixture;

fn without_metadata(source: &State) -> State {
    let mut target = State::new(source.object_hash());
    for entry in source.entries() {
        target.dangerously_push_entry(
            Stat::default(),
            entry.id,
            entry.stage().into(),
            entry.mode,
            entry.path(source),
        );
    }
    target
}

#[test]
fn only_unchanged_stage_zero_entries_reuse_stat_and_flags() -> TestResult {
    let hash = gix_testtools::object_hash();
    let blob_id = hash.empty_blob();
    let other_blob_id = gix_object::compute_hash(hash, gix_object::Kind::Blob, b"changed")?;
    let stat = Stat {
        size: 42,
        ino: 7,
        ..Stat::default()
    };
    let retained_flags = Flags::ASSUME_VALID | Flags::SKIP_WORKTREE | Flags::EXTENDED | Flags::UPTODATE;
    let mut source = State::new(hash);
    for (path, stage, flags) in [
        ("a-deleted", Stage::Unconflicted, Flags::empty()),
        (
            "b-kept",
            Stage::Unconflicted,
            retained_flags | Flags::FSMONITOR_VALID | Flags::PATHSPEC_MATCHED,
        ),
        ("c-changed", Stage::Unconflicted, retained_flags),
        ("d-mode", Stage::Unconflicted, retained_flags),
        ("e-conflict", Stage::Unconflicted, retained_flags),
        ("f-removal", Stage::Unconflicted, Flags::REMOVE),
        ("g-intent", Stage::Unconflicted, Flags::INTENT_TO_ADD | Flags::EXTENDED),
        ("h-old-conflict", Stage::Ours, retained_flags),
        ("z-deleted", Stage::Unconflicted, Flags::empty()),
    ] {
        source.dangerously_push_entry(stat, blob_id, flags | Flags::from_stage(stage), Mode::FILE, path.into());
    }
    let original = source.entries().to_vec();
    let mut target = State::new(hash);
    for (path, stage, mode, blob_id) in [
        ("0-added", Stage::Unconflicted, Mode::FILE, blob_id),
        ("b-kept", Stage::Unconflicted, Mode::FILE, blob_id),
        ("c-changed", Stage::Unconflicted, Mode::FILE, other_blob_id),
        ("d-mode", Stage::Unconflicted, Mode::FILE_EXECUTABLE, blob_id),
        ("e-conflict", Stage::Base, Mode::FILE, blob_id),
        ("e-conflict", Stage::Ours, Mode::FILE, blob_id),
        ("e-conflict", Stage::Theirs, Mode::FILE, blob_id),
        ("f-removal", Stage::Unconflicted, Mode::FILE, blob_id),
        ("g-intent", Stage::Unconflicted, Mode::FILE, blob_id),
        ("h-old-conflict", Stage::Unconflicted, Mode::FILE, blob_id),
    ] {
        target.dangerously_push_entry(Stat::default(), blob_id, stage.into(), mode, path.into());
    }
    target.copy_metadata_from(&source)?;
    for entry in target.entries() {
        let kept = entry.path(&target) == "b-kept";
        assert_eq!(
            entry.stat,
            if kept { stat } else { Stat::default() },
            "only unchanged stage-zero paths can reuse filesystem stat data: {}",
            entry.path(&target)
        );
        assert_eq!(
            entry.flags,
            if kept { retained_flags } else { entry.stage().into() },
            "other operation flags, intent-to-add, and removals cannot leak into the merge result: {}",
            entry.path(&target)
        );
    }
    assert_eq!(
        target.timestamp(),
        source.timestamp(),
        "retaining the old timestamp preserves racy-stat detection"
    );
    assert_eq!(
        source.entries(),
        original,
        "metadata reuse leaves the source index untouched"
    );
    Ok(())
}

#[test]
fn racy_stat_sizes_remain_untrustworthy_after_serializing_a_new_index() -> TestResult {
    let hash = gix_testtools::object_hash();
    let mut source = State::new(hash);
    let stat = Stat {
        mtime: gix_index::entry::stat::Time { secs: 100, nsecs: 0 },
        size: 42,
        ..Stat::default()
    };
    source.set_timestamp(filetime::FileTime::from_unix_time(100, 0));
    source.dangerously_push_entry(stat, hash.empty_blob(), Flags::UPTODATE, Mode::FILE, "file".into());
    let mut target = without_metadata(&source);
    target.copy_metadata_from(&source)?;
    assert_eq!(
        target.entries()[0].stat.size,
        0,
        "smudging a racy entry forces a content check after the index is rewritten"
    );
    assert_eq!(
        target.entries()[0].stat.mtime,
        stat.mtime,
        "the remaining stat data is still carried forward"
    );
    let mut out = Vec::new();
    assert!(
        !target.entries()[0].flags.contains(Flags::UPTODATE),
        "racy entries still require a content check"
    );
    gix_index::File::from_state(target, "unused-index").write_to(&mut out, Default::default())?;
    let (rewritten, _) = State::from_bytes(
        &out,
        filetime::FileTime::from_unix_time(200, 0),
        hash,
        Default::default(),
    )?;
    assert!(
        !rewritten.entries()[0].stat.matches(&stat, Default::default()),
        "a later index timestamp cannot make a previously racy stat comparison pass"
    );
    assert_eq!(
        source.entries()[0].stat,
        stat,
        "the source's stat data is left untouched"
    );
    Ok(())
}

#[test]
fn tree_caches_keep_unaffected_subtrees_and_drop_directory_replacements() -> TestResult {
    let source = Fixture::Generated("v2_deeper_tree").open();
    let original = source.tree().expect("committing populates TREE").clone();
    let mut target = without_metadata(&source);
    target.copy_metadata_from(&source)?;
    assert_eq!(
        target.tree(),
        Some(&original),
        "an identical index keeps its complete cache tree"
    );

    target = without_metadata(&source);
    target.remove_entries(|_, path, _| path.starts_with(b"d/") || path == "sub/b/2");
    target
        .entry_mut_by_path_and_stage("sub/c/d/3".into(), Stage::Unconflicted)
        .expect("the fixture has a nested tracked file")
        .mode = Mode::FILE_EXECUTABLE;
    target.dangerously_push_entry(
        Stat::default(),
        source.object_hash().empty_blob(),
        Flags::empty(),
        Mode::FILE,
        "d".into(),
    );
    target.sort_entries();
    target.copy_metadata_from(&source)?;
    let tree = target.tree().expect("the cache is retained");
    assert!(tree.num_entries.is_none(), "changed paths invalidate the root");
    assert!(
        tree.children.iter().all(|child| child.name.as_slice() != b"d"),
        "a file replacing a directory removes that cached subtree"
    );
    let sub = tree
        .children
        .iter()
        .find(|child| child.name.as_slice() == b"sub")
        .expect("sub stays a directory");
    let original_sub = original
        .children
        .iter()
        .find(|child| child.name.as_slice() == b"sub")
        .expect("sub was cached");
    assert!(sub.num_entries.is_none(), "changes invalidate their ancestors");
    assert_eq!(
        sub.children.iter().find(|child| child.name.as_slice() == b"a"),
        original_sub.children.iter().find(|child| child.name.as_slice() == b"a"),
        "an unaffected sibling retains its cached tree ID and entry count"
    );
    let c = sub
        .children
        .iter()
        .find(|child| child.name.as_slice() == b"c")
        .expect("c remains cached");
    assert!(
        c.num_entries.is_none() && c.children[0].num_entries.is_none(),
        "every changed ancestor is invalid"
    );
    assert_eq!(
        source.tree(),
        Some(&original),
        "invalidation never mutates the source cache"
    );
    target.verify_extensions(false, gix_object::find::Never)?;
    Ok(())
}

#[test]
fn untracked_caches_invalidate_changed_directories_and_ignore_descendants() -> TestResult {
    let source = Fixture::Generated("untracked_cache_nested").open();
    let original = source.untracked().expect("status populates UNTR");
    let mut target = without_metadata(&source);
    target.copy_metadata_from(&source)?;
    assert_eq!(
        format!("{:#?}", target.untracked()),
        format!("{:#?}", source.untracked()),
        "an identical index preserves all untracked-cache information"
    );

    for changed in [
        "tracked-dir-with-ignore/tracked-file",
        "tracked-dir-with-ignore/.gitignore",
    ] {
        let mut target = without_metadata(&source);
        target
            .entry_mut_by_path_and_stage(changed.into(), Stage::Unconflicted)
            .expect("the fixture tracks both paths")
            .mode = Mode::FILE_EXECUTABLE;
        target.copy_metadata_from(&source)?;
        let cache = target.untracked().expect("UNTR is retained");
        assert_eq!(
            cache.identifier(),
            original.identifier(),
            "location identity is preserved"
        );
        assert!(
            cache.directories()[0].stat.is_none(),
            "collapsed untracked directories require ancestor invalidation"
        );
        for (directory, old) in cache.directories().iter().zip(original.directories()) {
            let affected = directory.name == "tracked-dir-with-ignore"
                || (changed.ends_with(".gitignore") && directory.name.contains_str("nested-untracked-dir"))
                || (changed.ends_with(".gitignore") && directory.name == "deep-untracked-dir");
            if affected {
                assert!(
                    directory.stat.is_none() && directory.untracked_entries.is_empty() && !directory.check_only,
                    "changed directories and inherited ignore results must be rescanned: {}",
                    directory.name
                );
            } else if !directory.name.is_empty() {
                assert_eq!(
                    format!("{directory:?}"),
                    format!("{old:?}"),
                    "unaffected directory caches stay useful"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn adapted_untracked_caches_match_gits_uncached_listing_after_tracking_changes() -> TestResult {
    let source = Fixture::Generated("untracked_cache_nested").open();
    let root = source
        .path()
        .parent()
        .and_then(std::path::Path::parent)
        .expect("the generated index lives in .git");
    let mut target = without_metadata(&source);
    target.remove_entries(|_, path, _| path == "tracked-dir-with-ignore/tracked-file");
    target.dangerously_push_entry(
        Stat::default(),
        source.object_hash().empty_blob(),
        Flags::empty(),
        Mode::FILE,
        "tracked-dir-with-ignore/visible-untracked-file".into(),
    );
    target.sort_entries();
    target.copy_metadata_from(&source)?;
    let temporary = gix_testtools::tempfile::TempDir::new()?;
    let mut index = gix_index::File::from_state(target, temporary.path().join("cached-index"));
    index.write(gix_index::write::Options {
        extensions: gix_index::write::Extensions::AllUnconditionally,
        ..Default::default()
    })?;
    let uncached_path = temporary.path().join("uncached-index");
    let mut uncached_bytes = Vec::new();
    index.write_to(
        &mut uncached_bytes,
        gix_index::write::Options {
            extensions: gix_index::write::Extensions::None,
            ..Default::default()
        },
    )?;
    std::fs::write(&uncached_path, uncached_bytes)?;
    let listing = |path| {
        gix_testtools::git_command(root)
            .args([
                "--no-optional-locks",
                "status",
                "--porcelain=v1",
                "--untracked-files=normal",
                "-z",
            ])
            .env("GIT_INDEX_FILE", path)
            .output()
    };
    let cached = listing(index.path())?;
    let uncached = listing(&uncached_path)?;
    assert!(
        cached.status.success() && uncached.status.success(),
        "Git reads both indexes: {cached:?}, {uncached:?}"
    );
    assert_eq!(
        cached.stdout, uncached.stdout,
        "invalidated directory caches agree with scanning the worktree from scratch"
    );
    let paths: Vec<_> = cached.stdout.split(|byte| *byte == 0).collect();
    assert!(
        paths.contains(&b"?? tracked-dir-with-ignore/tracked-file".as_slice()),
        "a removed index entry is now untracked"
    );
    assert!(
        !paths.contains(&b"?? tracked-dir-with-ignore/visible-untracked-file".as_slice()),
        "a newly tracked entry leaves the untracked listing"
    );
    Ok(())
}

#[test]
fn metadata_requires_sorted_indexes_with_matching_hashes() -> TestResult {
    let source = Fixture::Generated("v4_more_files_IEOT").open();
    let mut target = without_metadata(&source);
    target.copy_metadata_from(&source)?;
    assert_eq!(
        target.version(),
        source.version(),
        "the source's in-memory index version is retained"
    );

    let mut unsorted = State::new(source.object_hash());
    for path in ["b", "a"] {
        unsorted.dangerously_push_entry(
            Stat::default(),
            source.object_hash().empty_blob(),
            Flags::empty(),
            Mode::FILE,
            path.into(),
        );
    }
    assert!(
        target.copy_metadata_from(&unsorted).is_err(),
        "the sorted walk rejects an unsorted source"
    );
    assert!(
        unsorted.copy_metadata_from(&source).is_err(),
        "the sorted walk rejects an unsorted destination"
    );
    let different_hash = if source.object_hash() == gix_hash::Kind::Sha1 {
        gix_hash::Kind::Sha256
    } else {
        gix_hash::Kind::Sha1
    };
    assert!(
        target.copy_metadata_from(&State::new(different_hash)).is_err(),
        "caches and object IDs cannot cross hash kinds"
    );
    Ok(())
}
