use std::{fs, sync::atomic::AtomicBool};

use gix_index::entry::{Flags, Mode};
use gix_object::Write;
use gix_testtools::TestResult;
use gix_utils::progress;
use gix_worktree_state::checkout::{Change, Options, apply_changes};

#[test]
fn applies_only_selected_changes_with_filters_and_sparse_marks() -> TestResult {
    let dir = gix_testtools::tempfile::tempdir()?;
    fs::write(dir.path().join("unchanged"), b"local change")?;
    fs::write(dir.path().join("untracked"), b"keep me")?;
    fs::create_dir(dir.path().join("old"))?;
    fs::write(dir.path().join("old/removed"), b"remove me")?;
    let (mut index, objects) = index(&[
        (".gitattributes", "written text eol=crlf\n", Mode::FILE),
        ("skipped", "excluded\n", Mode::FILE),
        ("sparse/", "", Mode::DIR),
        ("unchanged", "tracked\n", Mode::FILE),
        ("written", "filtered\n", Mode::FILE),
    ])?;
    index.entries_mut()[1].flags |= Flags::EXTENDED | Flags::SKIP_WORKTREE;
    index.entries_mut()[2].flags |= Flags::EXTENDED | Flags::SKIP_WORKTREE;
    index.entries_mut()[3].stat.size = 123;
    let previous = index.entries()[3].stat;

    let out = apply_changes(
        &mut index,
        &[
            Change::Write { entry_index: 4 },
            Change::Remove {
                path: "old/removed".into(),
            },
            Change::Write { entry_index: 1 },
            Change::Write { entry_index: 2 },
        ],
        dir.path(),
        objects,
        &progress::Discard,
        &progress::Discard,
        &AtomicBool::default(),
        options(),
    )?;

    assert!(
        out.errors.is_empty() && out.collisions.is_empty(),
        "the selected changes succeed"
    );
    assert_eq!(
        fs::read(dir.path().join("written"))?,
        b"filtered\r\n",
        "unselected attributes still configure checkout"
    );
    assert_eq!(
        fs::read(dir.path().join("unchanged"))?,
        b"local change",
        "unselected tracked files retain local edits"
    );
    assert_eq!(
        fs::read(dir.path().join("untracked"))?,
        b"keep me",
        "untracked files are untouched"
    );
    assert!(!dir.path().join("skipped").exists(), "sparse entries stay absent");
    assert!(
        !dir.path().join("sparse").exists(),
        "compressed sparse directories stay absent"
    );
    assert!(
        !dir.path().join(".gitattributes").exists(),
        "attribute lookup does not select the attribute file for writing"
    );
    assert!(
        !dir.path().join("old").exists() && dir.path().is_dir(),
        "empty tracked parents are pruned without removing the worktree root"
    );
    assert_eq!(
        index.entries()[3].stat,
        previous,
        "unselected cache metadata stays intact"
    );
    Ok(())
}

#[test]
fn explicit_removals_allow_file_directory_transitions() -> TestResult {
    let dir = gix_testtools::tempfile::tempdir()?;
    fs::write(dir.path().join("file-to-dir"), b"old file")?;
    fs::create_dir(dir.path().join("dir-to-file"))?;
    fs::write(dir.path().join("dir-to-file/old"), b"old child")?;
    fs::write(dir.path().join("dir-to-file/other"), b"other child")?;
    let (mut index, objects) = index(&[
        ("dir-to-file", "new file", Mode::FILE),
        ("file-to-dir/new", "new child", Mode::FILE),
    ])?;
    let out = apply_changes(
        &mut index,
        &[
            Change::Write { entry_index: 0 },
            Change::Write { entry_index: 1 },
            Change::Remove {
                path: "file-to-dir".into(),
            },
            Change::Remove {
                path: "dir-to-file/old".into(),
            },
            Change::Remove {
                path: "dir-to-file/other".into(),
            },
            Change::Remove {
                path: "already/missing".into(),
            },
        ],
        dir.path(),
        objects,
        &progress::Discard,
        &progress::Discard,
        &AtomicBool::default(),
        options(),
    )?;
    assert!(
        out.errors.is_empty() && out.collisions.is_empty(),
        "removals precede writes regardless of action order"
    );
    assert_eq!(
        fs::read(dir.path().join("dir-to-file"))?,
        b"new file",
        "an emptied tracked directory becomes a file"
    );
    assert_eq!(
        fs::read(dir.path().join("file-to-dir/new"))?,
        b"new child",
        "a tracked file becomes a directory"
    );
    Ok(())
}

#[test]
fn invalid_actions_are_rejected_before_any_mutation() -> TestResult {
    for (paths, changes) in [
        (vec!["a"], vec![Change::Write { entry_index: 9 }]),
        (
            vec!["a"],
            vec![Change::Write { entry_index: 0 }, Change::Write { entry_index: 0 }],
        ),
        (
            vec!["a"],
            vec![
                Change::Remove { path: "gone".into() },
                Change::Remove { path: "gone".into() },
            ],
        ),
        (
            vec!["a", "a/b"],
            vec![Change::Write { entry_index: 0 }, Change::Write { entry_index: 1 }],
        ),
        (
            vec!["A", "a"],
            vec![Change::Write { entry_index: 0 }, Change::Write { entry_index: 1 }],
        ),
        (
            vec!["A", "a-", "a/b"],
            vec![Change::Write { entry_index: 0 }, Change::Write { entry_index: 2 }],
        ),
        (
            vec!["a"],
            vec![
                Change::Remove { path: "gone".into() },
                Change::Remove {
                    path: "gone/child".into(),
                },
            ],
        ),
        (
            vec!["a"],
            vec![Change::Remove {
                path: "../outside".into(),
            }],
        ),
        (vec![".git/config"], vec![Change::Write { entry_index: 0 }]),
        (vec!["a//b"], vec![Change::Write { entry_index: 0 }]),
        (vec!["a\0b"], vec![Change::Write { entry_index: 0 }]),
    ] {
        let dir = gix_testtools::tempfile::tempdir()?;
        fs::write(dir.path().join("canary"), b"untouched")?;
        let entries = paths.iter().map(|path| (*path, "new", Mode::FILE)).collect::<Vec<_>>();
        let (mut index, objects) = index(&entries)?;
        let changes = std::iter::once(Change::Remove { path: "canary".into() })
            .chain(changes)
            .collect::<Vec<_>>();
        let mut opts = options();
        opts.fs.ignore_case = true;
        opts.validate.protect_windows = false;
        opts.validate.protect_ntfs = false;
        let err = apply_changes(
            &mut index,
            &changes,
            dir.path(),
            objects,
            &progress::Discard,
            &progress::Discard,
            &AtomicBool::default(),
            opts,
        )
        .expect_err("malformed actions cannot be applied");
        assert!(
            gix_error::classify(&err).is_validation(),
            "invalid action errors retain their classification: {err}"
        );
        assert_eq!(
            fs::read(dir.path().join("canary"))?,
            b"untouched",
            "all actions are validated before a removal"
        );
    }
    Ok(())
}

#[test]
fn refuses_conflict_stages_before_removing_files() -> TestResult {
    let dir = gix_testtools::tempfile::tempdir()?;
    fs::write(dir.path().join("canary"), b"untouched")?;
    let (mut index, objects) = index(&[("conflicted", "ours", Mode::FILE)])?;
    index.entries_mut()[0].flags = Flags::from_stage(gix_index::entry::Stage::Ours);
    apply_changes(
        &mut index,
        &[
            Change::Remove { path: "canary".into() },
            Change::Write { entry_index: 0 },
        ],
        dir.path(),
        objects,
        &progress::Discard,
        &progress::Discard,
        &AtomicBool::default(),
        options(),
    )
    .expect_err("conflict stages require explicit worktree materialization first");
    assert_eq!(
        fs::read(dir.path().join("canary"))?,
        b"untouched",
        "unsupported stages fail before writes"
    );
    Ok(())
}

#[test]
fn removals_and_writes_never_recurse_through_blocking_directories() -> TestResult {
    for remove in [false, true] {
        let dir = gix_testtools::tempfile::tempdir()?;
        fs::create_dir(dir.path().join("blocked"))?;
        fs::write(dir.path().join("blocked/untracked"), b"keep me")?;
        let (mut index, objects) = index(&[("blocked", "new", Mode::FILE)])?;
        let mut changes = vec![Change::Write { entry_index: 0 }];
        if remove {
            changes.push(Change::Remove { path: "blocked".into() });
        }
        let mut opts = options();
        opts.overwrite_existing = true;
        opts.keep_going = true;
        let out = apply_changes(
            &mut index,
            &changes,
            dir.path(),
            objects,
            &progress::Discard,
            &progress::Discard,
            &AtomicBool::default(),
            opts,
        )?;
        assert!(
            !out.errors.is_empty() || !out.collisions.is_empty(),
            "a nonempty obstruction is reported"
        );
        assert_eq!(
            fs::read(dir.path().join("blocked/untracked"))?,
            b"keep me",
            "overwrite options never authorize recursive deletion"
        );
    }
    Ok(())
}

#[test]
fn removals_do_not_follow_symlinks() -> TestResult {
    let dir = gix_testtools::tempfile::tempdir()?;
    if !gix_fs::Capabilities::probe_dir(dir.path()).symlink {
        return Ok(());
    }
    let outside = gix_testtools::tempfile::tempdir()?;
    fs::write(outside.path().join("canary"), b"untouched")?;
    fs::write(outside.path().join("other"), b"also untouched")?;
    gix_fs::symlink::create(outside.path(), &dir.path().join("link"))?;
    let (mut index, objects) = index(&[("link", "replacement", Mode::FILE)])?;
    apply_changes(
        &mut index,
        &[Change::Remove {
            path: "link/canary".into(),
        }],
        dir.path(),
        objects.clone(),
        &progress::Discard,
        &progress::Discard,
        &AtomicBool::default(),
        options(),
    )
    .expect_err("a parent symlink must never be traversed for deletion");
    let mut opts = options();
    opts.keep_going = true;
    let out = apply_changes(
        &mut index,
        &[
            Change::Remove {
                path: "link/canary".into(),
            },
            Change::Remove {
                path: "link/other".into(),
            },
        ],
        dir.path(),
        objects.clone(),
        &progress::Discard,
        &progress::Discard,
        &AtomicBool::default(),
        opts,
    )?;
    assert_eq!(
        out.errors.len(),
        2,
        "continuing after a failed parent check cannot cache the symlink as a directory"
    );
    assert_eq!(
        fs::read(outside.path().join("other"))?,
        b"also untouched",
        "later removals still reject the symlinked parent"
    );
    apply_changes(
        &mut index,
        &[Change::Remove { path: "link".into() }, Change::Write { entry_index: 0 }],
        dir.path(),
        objects,
        &progress::Discard,
        &progress::Discard,
        &AtomicBool::default(),
        options(),
    )?;
    assert_eq!(
        fs::read(outside.path().join("canary"))?,
        b"untouched",
        "deleting a terminal symlink only removes the link"
    );
    assert_eq!(
        fs::read(dir.path().join("link"))?,
        b"replacement",
        "an explicitly removed symlink can be replaced without touching its target"
    );
    Ok(())
}

#[test]
fn interruption_leaves_the_worktree_untouched() -> TestResult {
    let dir = gix_testtools::tempfile::tempdir()?;
    fs::write(dir.path().join("canary"), b"untouched")?;
    let (mut index, objects) = index(&[("written", "new", Mode::FILE)])?;
    let out = apply_changes(
        &mut index,
        &[
            Change::Remove { path: "canary".into() },
            Change::Write { entry_index: 0 },
        ],
        dir.path(),
        objects,
        &progress::Discard,
        &progress::Discard,
        &AtomicBool::new(true),
        options(),
    )?;
    assert_eq!(out.files_updated, 0, "interruption prevents checkout work");
    assert_eq!(
        fs::read(dir.path().join("canary"))?,
        b"untouched",
        "interruption prevents removals"
    );
    assert!(!dir.path().join("written").exists(), "interruption prevents writes");
    Ok(())
}

fn options() -> Options {
    let mut options = Options::new(gix_filter::Pipeline::new(
        Default::default(),
        gix_testtools::object_hash(),
        Default::default(),
    ));
    options.thread_limit = Some(1);
    options
}

fn index(
    entries: &[(&str, &str, Mode)],
) -> Result<(gix_index::State, gix_odb::memory::Proxy<gix_object::find::Never>), gix_error::Error> {
    let hash = gix_testtools::object_hash();
    let objects = gix_odb::memory::Proxy::new(gix_object::find::Never, hash);
    let mut index = gix_index::State::new(hash);
    for (path, content, mode) in entries {
        index.dangerously_push_entry(
            Default::default(),
            objects.write_buf(
                if *mode == Mode::DIR {
                    gix_object::Kind::Tree
                } else {
                    gix_object::Kind::Blob
                },
                content.as_bytes(),
            )?,
            Flags::empty(),
            *mode,
            (*path).into(),
        );
    }
    index.sort_entries();
    Ok((index, objects))
}
