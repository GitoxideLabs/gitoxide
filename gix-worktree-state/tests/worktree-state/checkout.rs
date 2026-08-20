#[cfg(unix)]
use std::os::unix::prelude::{MetadataExt, PermissionsExt};
use std::{
    fs,
    io::{ErrorKind, ErrorKind::AlreadyExists},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use gix_features::progress;
use gix_object::{Data, bstr::ByteSlice};
use gix_testtools::tempfile::TempDir;
use gix_worktree_state::checkout::Collision;
use std::sync::LazyLock;

use crate::{fixture_path, odb_at};

static DRIVER: LazyLock<PathBuf> =
    LazyLock::new(|| gix_testtools::build_example_for_test("gix-filter", "arrow", env!("CARGO_TARGET_TMPDIR")));

fn driver_exe() -> String {
    let mut exe = DRIVER.to_string_lossy().into_owned();
    if cfg!(windows) {
        exe = exe.replace('\\', "/");
    }
    exe
}

fn assure_is_empty(dir: impl AsRef<Path>) -> std::io::Result<()> {
    assert_eq!(std::fs::read_dir(dir)?.count(), 0);
    Ok(())
}

#[test]
fn submodules_are_instantiated_as_directories() -> crate::Result {
    let mut opts = opts_from_probe();
    opts.overwrite_existing = false;
    let (_source_tree, destination, _index, _outcome) = checkout_index_in_tmp_dir(opts.clone(), "make_mixed", None)?;

    for path in ["m1", "modules/m1"] {
        let sm = destination.path().join(path);
        assert!(sm.is_dir());
        assure_is_empty(sm)?;
    }

    Ok(())
}

#[test]
fn accidental_writes_through_symlinks_are_prevented_if_overwriting_is_forbidden() {
    let mut opts = opts_from_probe();
    // without overwrite mode, everything is safe.
    opts.overwrite_existing = false;
    let (source_tree, destination, _index, outcome) =
        checkout_index_in_tmp_dir(opts.clone(), "make_dangerous_symlink", None).unwrap();

    let source_files = dir_structure(&source_tree);
    let worktree_files = dir_structure(&destination);

    if opts.fs.ignore_case {
        assert_eq!(
            stripped_prefix(&source_tree, &source_files),
            stripped_prefix(&destination, &worktree_files),
        );
        if multi_threaded() {
            assert_eq!(outcome.collisions.len(), 2);
        } else {
            assert_eq!(
                outcome.collisions,
                vec![
                    Collision {
                        path: "FAKE-DIR".into(),
                        error_kind: AlreadyExists
                    },
                    Collision {
                        path: "FAKE-FILE".into(),
                        error_kind: AlreadyExists
                    }
                ]
            );
        }
    } else {
        let expected = ["A-dir/a", "A-file", "FAKE-DIR", "FAKE-FILE", "fake-dir/b", "fake-file"];
        assert_eq!(stripped_prefix(&source_tree, &source_files), paths(expected));
        assert_eq!(stripped_prefix(&destination, &worktree_files), paths(expected));
        assert!(outcome.collisions.is_empty());
    }
}

#[test]
fn writes_through_symlinks_are_prevented_even_if_overwriting_is_allowed() {
    let mut opts = opts_from_probe();
    // with overwrite mode
    opts.overwrite_existing = true;
    let (source_tree, destination, _index, outcome) =
        checkout_index_in_tmp_dir(opts.clone(), "make_dangerous_symlink", None).unwrap();

    let source_files = dir_structure(&source_tree);
    let worktree_files = dir_structure(&destination);

    if opts.fs.ignore_case {
        assert_eq!(
            stripped_prefix(&source_tree, &source_files),
            paths(["A-dir/a", "A-file", "fake-dir/b", "fake-file"]),
        );
        assert_eq!(
            stripped_prefix(&destination, &worktree_files),
            paths([
                if cfg!(windows) { r"A-dir\a" } else { "A-dir/a" },
                "A-file",
                "FAKE-DIR",
                "FAKE-FILE"
            ]),
        );
        assert!(outcome.collisions.is_empty());
    } else {
        let expected = ["A-dir/a", "A-file", "FAKE-DIR", "FAKE-FILE", "fake-dir/b", "fake-file"];
        assert_eq!(stripped_prefix(&source_tree, &source_files), paths(expected));
        assert_eq!(stripped_prefix(&destination, &worktree_files), paths(expected));
        assert!(outcome.collisions.is_empty());
    }
}

#[test]
fn nonexclusive_checkout_does_not_follow_terminal_symlinks() -> crate::Result {
    let mut opts = opts_from_probe();
    // the test needs filesystem symlink support;
    if !opts.fs.symlink {
        return Ok(());
    }
    opts.destination_is_initially_empty = false;

    for overwrite_existing in [false, true] {
        opts.overwrite_existing = overwrite_existing;
        let outside = gix_testtools::tempfile::tempdir_in(std::env::current_dir()?)?;
        let canary = outside.path().join("canary");
        std::fs::write(&canary, b"untouched")?;
        let (_source, destination, _index, outcome) = checkout_index_in_tmp_dir_opts(
            opts.clone(),
            "make_mixed_without_submodules_and_symlinks",
            None,
            |_| true,
            |destination| gix_fs::symlink::create(&canary, &destination.join("executable")),
        )?;

        assert!(
            outcome.errors.is_empty(),
            "checkout should not encounter non-collision errors"
        );
        let checked_out = destination.path().join("executable");
        if overwrite_existing {
            assert!(
                outcome.collisions.is_empty(),
                "forced checkout should replace the symlink"
            );
            assert!(
                checked_out.symlink_metadata()?.is_file(),
                "the symlink must become a regular file"
            );
            assert_eq!(
                std::fs::read(&checked_out)?,
                b"content",
                "the regular file must be checked out"
            );
        } else {
            assert_eq!(
                outcome.collisions.len(),
                1,
                "non-forced checkout should report the terminal symlink as a collision"
            );
            assert_eq!(
                outcome.collisions[0].path, "executable",
                "the collision should identify the terminal symlink"
            );
            #[cfg(windows)]
            assert_eq!(
                outcome.collisions[0].error_kind, AlreadyExists,
                "the collision error kind differs by platform and is only stable on Windows"
            );
            assert!(
                checked_out.symlink_metadata()?.file_type().is_symlink(),
                "non-forced checkout must leave the symlink in place"
            );
        }
        assert_eq!(
            std::fs::read(&canary)?,
            b"untouched",
            "the symlink target must stay unchanged"
        );
    }
    Ok(())
}

#[test]
fn delayed_driver_process() -> crate::Result {
    let mut opts = opts_from_probe();
    opts.filter_process_delay = gix_filter::driver::apply::Delay::Allow;
    setup_filter_pipeline(opts.filters.options_mut());
    let fs_supports_executable_bit = opts.fs.executable_bit;
    let (_source, destination, _index, outcome) = checkout_index_in_tmp_dir_opts(
        opts,
        "make_mixed_without_submodules_and_symlinks",
        None,
        |_| true,
        |_| Ok(()),
    )?;
    assert_eq!(outcome.collisions.len(), 0);
    assert_eq!(outcome.errors.len(), 0);
    assert_eq!(outcome.files_updated, 6);

    let dest = destination.path();
    assert_eq!(
        std::fs::read(dest.join("executable"))?.as_bstr(),
        "content",
        "unfiltered"
    );
    assert_eq!(
        std::fs::read(dest.join("filtered-executable"))?.as_bstr(),
        "➡filtered content",
        "filtered executable"
    );
    #[cfg(unix)]
    if fs_supports_executable_bit {
        assert_ne!(
            std::fs::metadata(dest.join("filtered-executable"))?.mode() & 0o111,
            0,
            "a delayed filter must not prevent executable bits from being applied"
        );
    }
    assert_eq!(
        std::fs::read(dest.join("dir").join("content"))?.as_bstr(),
        "➡other content\r\n"
    );
    assert_eq!(
        std::fs::read(dest.join("dir").join("sub-dir").join("file"))?.as_bstr(),
        "➡even other content\r\n"
    );
    Ok(())
}

#[test]
fn forgotten_delayed_path_is_a_corruption_error() {
    let mut opts = opts_from_probe();
    opts.filter_process_delay = gix_filter::driver::apply::Delay::Allow;
    setup_filter_pipeline(opts.filters.options_mut());
    opts.filters.options_mut().drivers[0].process = Some((driver_exe() + " process forget-delayed").into());

    let err = checkout_index_in_tmp_dir_opts(
        opts,
        "make_mixed_without_submodules_and_symlinks",
        None,
        |_| true,
        |_| Ok(()),
    )
    .expect_err("a required filter must not silently forget a delayed path");
    let err = err
        .downcast_ref::<gix_error::Error>()
        .expect("checkout errors retain their classification");
    assert!(err.is_corrupted());
}

#[cfg(unix)]
#[test]
fn delayed_driver_process_removes_obsolete_executable_bits() -> crate::Result {
    let mut opts = opts_from_probe();
    opts.destination_is_initially_empty = false;
    opts.filter_process_delay = gix_filter::driver::apply::Delay::Allow;
    setup_filter_pipeline(opts.filters.options_mut());
    let (_source, destination, _index, outcome) = checkout_index_in_tmp_dir_opts(
        opts,
        "make_mixed_without_submodules_and_symlinks",
        None,
        |_| true,
        |destination| {
            let filtered_non_executable = destination.join("empty");
            std::fs::write(&filtered_non_executable, [])?;
            std::fs::set_permissions(&filtered_non_executable, std::fs::Permissions::from_mode(0o755))?;
            Ok(())
        },
    )?;

    assert!(outcome.collisions.is_empty(), "regular files should not collide");
    assert!(outcome.errors.is_empty(), "checkout should succeed");
    assert_eq!(
        std::fs::metadata(destination.path().join("empty"))?.mode() & 0o111,
        0,
        "a delayed filter must not prevent obsolete executable bits from being removed"
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn nonexclusive_checkout_adjusts_executable_bits_in_both_directions() -> crate::Result {
    for overwrite_existing in [false, true] {
        let mut opts = opts_from_probe();
        opts.destination_is_initially_empty = false;
        opts.overwrite_existing = overwrite_existing;
        let (_source, destination, _index, outcome) = checkout_index_in_tmp_dir_opts(
            opts,
            "make_mixed_without_submodules_and_symlinks",
            None,
            |_| true,
            |destination| {
                let soon_non_executable = destination.join("empty");
                std::fs::write(&soon_non_executable, [])?;
                std::fs::set_permissions(&soon_non_executable, std::fs::Permissions::from_mode(0o755))?;

                let soon_executable = destination.join("executable");
                std::fs::write(&soon_executable, b"old content")?;
                std::fs::set_permissions(&soon_executable, std::fs::Permissions::from_mode(0o644))?;
                Ok(())
            },
        )?;

        assert!(outcome.collisions.is_empty(), "regular files should not collide");
        assert!(outcome.errors.is_empty(), "checkout should succeed");

        let non_executable_mode = std::fs::metadata(destination.path().join("empty"))?.mode();
        assert_eq!(
            non_executable_mode & 0o111,
            0,
            "checkout must remove obsolete executable bits with overwrite_existing={overwrite_existing}"
        );

        let executable_mode = std::fs::metadata(destination.path().join("executable"))?.mode();
        assert_ne!(
            executable_mode & 0o111,
            0,
            "checkout must add tracked executable bits with overwrite_existing={overwrite_existing}"
        );
    }
    Ok(())
}

#[test]
fn overwriting_files_and_lone_directories_works() -> crate::Result {
    for delay in [
        gix_filter::driver::apply::Delay::Allow,
        gix_filter::driver::apply::Delay::Forbid,
    ] {
        let mut opts = opts_from_probe();
        assert!(opts.fs.symlink, "The probe must detect to be able to generate symlinks");
        opts.overwrite_existing = true;
        opts.filter_process_delay = delay;
        opts.destination_is_initially_empty = false;
        setup_filter_pipeline(opts.filters.options_mut());
        let (source, destination, _index, outcome) = checkout_index_in_tmp_dir_opts(
            opts.clone(),
            "make_mixed",
            None,
            |_| true,
            |d| {
                let empty = d.join("empty");
                gix_fs::symlink::create(&d.join(".."), &empty)?; // empty is symlink to the directory above
                std::fs::write(d.join("executable"), b"longer content foo bar")?; // executable is regular file and has different content
                let dir = d.join("dir");
                std::fs::create_dir(&dir)?;
                std::fs::create_dir(dir.join("content"))?; // 'content' is a directory now

                let dir = dir.join("sub-dir");
                std::fs::create_dir(&dir)?;

                gix_fs::symlink::create(&empty, &dir.join("symlink"))?; // 'symlink' is a symlink to a directory.
                Ok(())
            },
        )?;

        assert!(outcome.collisions.is_empty());

        assert_eq!(
            stripped_prefix(&destination, &dir_structure(&destination)),
            paths(["dir/content", "dir/sub-dir/symlink", "empty", "executable"])
        );
        let meta = std::fs::symlink_metadata(destination.path().join("empty"))?;
        assert!(meta.is_file(), "'empty' is now a file");
        assert_eq!(meta.len(), 0, "'empty' is indeed empty");

        let exe = destination.path().join("executable");
        assert_eq!(std::fs::read(&exe)?, b"content", "'exe' has the correct content");

        let meta = std::fs::symlink_metadata(exe)?;
        assert!(meta.is_file());
        #[cfg(unix)]
        if opts.fs.executable_bit {
            let mode = meta.mode();
            assert_eq!(
                mode & 0o100,
                0o100,
                "executable bit set where supported ({:04o} & {:04o} = {:04o} should be {:04o})",
                mode,
                0o100,
                mode & 0o100,
                0o100
            );
            let umask_write = gix_testtools::umask() & 0o222;
            assert_eq!(
                mode & umask_write,
                0,
                "no excessive write bits are set ({:04o} & {:04o} = {:04o} should be {:04o})",
                mode,
                umask_write,
                mode & umask_write,
                0
            );
            assert_ne!(
                umask_write, 0,
                "test not meaningful unless runner umask restricts some writes"
            );
        }

        assert_eq!(
            std::fs::read(source.join("dir/content"))?.as_bstr(),
            "other content\n",
            "in the worktree, we have LF"
        );
        assert_eq!(
            std::fs::read(destination.path().join("dir/content"))?.as_bstr(),
            "➡other content\r\n",
            "autocrlf is enabled, so we get CRLF when checking out as the pipeline is active, and we have a filter"
        );

        let symlink = destination.path().join("dir/sub-dir/symlink");
        assert!(std::fs::symlink_metadata(&symlink)?.is_symlink());
        assert_eq!(
            std::fs::read(symlink)?.as_bstr(),
            "➡other content\r\n",
            "autocrlf is enabled"
        );
    }
    Ok(())
}

#[test]
fn symlinks_become_files_if_disabled() -> crate::Result {
    let mut opts = opts_from_probe();
    opts.fs.symlink = false;
    let (source_tree, destination, _index, outcome) =
        checkout_index_in_tmp_dir(opts.clone(), "make_mixed_without_submodules", None)?;

    assert_equality(&source_tree, &destination, opts.fs.symlink)?;
    assert!(outcome.collisions.is_empty());
    Ok(())
}

#[test]
fn symlinks_to_directories_are_usable() -> crate::Result {
    let opts = opts_from_probe();
    assert!(opts.fs.symlink, "The probe must detect to be able to generate symlinks");

    let (_source_tree, destination, _index, outcome) =
        checkout_index_in_tmp_dir(opts.clone(), "make_dir_symlink", None)?;
    let worktree_files = dir_structure(&destination);
    let worktree_files_stripped = stripped_prefix(&destination, &worktree_files);

    assert_eq!(worktree_files_stripped, paths(["symlink"]));
    let symlink_path = &worktree_files[0];
    assert!(
        symlink_path
            .symlink_metadata()
            .expect("symlink is on disk")
            .is_symlink()
    );
    assert!(
        symlink_path
            .metadata()
            .expect("metadata accessible through symlink")
            .is_dir()
    );
    assert_eq!(std::fs::read_link(symlink_path)?, Path::new("."));
    assert!(outcome.collisions.is_empty());
    Ok(())
}

#[test]
fn dangling_symlinks_can_be_created() -> crate::Result {
    let opts = opts_from_probe();
    assert!(opts.fs.symlink, "The probe must detect to be able to generate symlinks");

    for (fixture, symlink_name, target_name) in [
        ("make_dangling_symlink", "dangling", "non-existing-target"),
        (
            "make_dangling_symlink_to_windows_invalid",
            "dangling-qmarks-symlink",
            "???",
        ),
        (
            "make_dangling_symlink_to_windows_reserved",
            "dangling-con-symlink",
            "CON",
        ),
    ] {
        let (_source_tree, destination, _index, outcome) = checkout_index_in_tmp_dir(opts.clone(), fixture, None)?;
        let worktree_files = dir_structure(&destination);
        let worktree_files_stripped = stripped_prefix(&destination, &worktree_files);

        assert_eq!(worktree_files_stripped, paths([symlink_name]));
        let symlink_path = &worktree_files[0];
        assert!(
            symlink_path
                .symlink_metadata()
                .expect("dangling symlink is on disk")
                .is_symlink()
        );
        assert_eq!(std::fs::read_link(symlink_path)?, Path::new(target_name));
        assert!(outcome.collisions.is_empty());
    }

    Ok(())
}

#[test]
fn allow_or_disallow_symlinks() -> crate::Result {
    let mut opts = opts_from_probe();
    for allowed in &[false, true] {
        opts.fs.symlink = *allowed;
        let (source_tree, destination, _index, outcome) =
            checkout_index_in_tmp_dir(opts.clone(), "make_mixed_without_submodules", None)?;

        assert_equality(&source_tree, &destination, opts.fs.symlink)?;
        assert!(outcome.collisions.is_empty());
    }
    Ok(())
}

#[test]
fn keep_going_collects_results() {
    let mut opts = opts_from_probe();
    opts.keep_going = true;
    let count = AtomicUsize::default();
    let (_source_tree, destination, _index, outcome) = checkout_index_in_tmp_dir_opts(
        opts,
        "make_mixed_without_submodules",
        None,
        |_id| {
            count
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                    (current < 2).then_some(current + 1)
                })
                .is_err()
        },
        |_| Ok(()),
    )
    .unwrap();

    if multi_threaded() {
        assert_eq!(
            outcome.errors.len(),
            2,
            "content changes due to non-deterministic nature of racy threads"
        );
    } else {
        assert_eq!(
            outcome
                .errors
                .iter()
                .map(|r| r.path.to_path_lossy().into_owned())
                .collect::<Vec<_>>(),
            paths([".gitattributes", "dir/content"])
        );
    }

    if multi_threaded() {
        let actual = dir_structure(&destination);
        assert!(
            (2..=3).contains(&actual.len()),
            "it's 3 most of the time, but can be 2 of the 'empty' file is missing as the object couldn't be accessed.\
             It's unclear why there isn't more, as it would keep going"
        );
    } else {
        assert_eq!(
            stripped_prefix(&destination, &dir_structure(&destination)),
            paths([
                if cfg!(unix) {
                    "dir/sub-dir/symlink"
                } else {
                    r"dir\sub-dir\symlink"
                },
                "empty",
                "executable",
            ]),
            "some files could not be created"
        );
    }

    assert!(outcome.collisions.is_empty());
}

#[test]
fn no_case_related_collisions_on_case_sensitive_filesystem() {
    let opts = opts_from_probe();
    if opts.fs.ignore_case {
        eprintln!("Skipping case-sensitive testing on what would be a case-insensitive file system");
        return;
    }
    let (source_tree, destination, index, outcome) =
        checkout_index_in_tmp_dir(opts.clone(), "make_ignorecase_collisions", None).unwrap();

    assert!(outcome.collisions.is_empty());
    let num_files = assert_equality(&source_tree, &destination, opts.fs.symlink).unwrap();
    assert_eq!(
        num_files,
        index.entries().len() - 1,
        "it checks out all files (minus 1 to account for .gitattributes which is skipped in the worktree in our tests)"
    );
    assert!(
        destination.path().join(".gitattributes").is_file(),
        "we do have attributes even though, dot files are ignored in `assert_equality`"
    );
}

#[test]
fn safety_checks_dotdot_trees() {
    let mut opts = opts_from_probe();
    let err =
        checkout_index_in_tmp_dir(opts.clone(), "make_traverse_trees", Some("traverse_dotdot_trees")).unwrap_err();
    let expected_err_msg = "Input path \"../outside\" contains relative or absolute components";
    assert_eq!(err.to_string(), expected_err_msg);

    opts.keep_going = true;
    let (_source_tree, _destination, _index, outcome) =
        checkout_index_in_tmp_dir(opts, "make_traverse_trees", Some("traverse_dotdot_trees"))
            .expect("keep-going checks out as much as possible");
    assert_eq!(outcome.errors.len(), 1, "one path could not be checked out");
    assert_eq!(outcome.errors[0].error.to_string(), expected_err_msg);
}

#[test]
fn safety_checks_dotgit_trees() {
    let opts = opts_from_probe();
    let err =
        checkout_index_in_tmp_dir(opts.clone(), "make_traverse_trees", Some("traverse_dotgit_trees")).unwrap_err();
    assert_eq!(err.to_string(), "The .git name may never be used");
}

#[test]
fn safety_checks_dotgit_ntfs_stream() {
    let opts = opts_from_probe();
    let err =
        checkout_index_in_tmp_dir(opts.clone(), "make_traverse_trees", Some("traverse_dotgit_stream")).unwrap_err();
    assert_eq!(
        err.to_string(),
        "The .git name may never be used",
        "note how it is still discovered even though the path is `.git::$INDEX_ALLOCATION`"
    );
}

#[test]
fn collisions_are_detected_on_a_case_insensitive_filesystem_even_with_delayed_filters() {
    let mut opts = opts_from_probe();
    if !opts.fs.ignore_case {
        eprintln!("Skipping case-insensitive testing on what would be a case-sensitive file system");
        return;
    }
    setup_filter_pipeline(opts.filters.options_mut());
    opts.filter_process_delay = gix_filter::driver::apply::Delay::Allow;
    let (source_tree, destination, _index, outcome) =
        checkout_index_in_tmp_dir(opts, "make_ignorecase_collisions", None).unwrap();

    let source_files = dir_structure(&source_tree);
    assert_eq!(
        stripped_prefix(&source_tree, &source_files),
        paths(["d", "file_x", "link-to-X", "x"]),
        "plenty of collisions prevent a checkout"
    );

    let dest_files = dir_structure(&destination);
    if multi_threaded() {
        assert!(
            (4..=6).contains(&dest_files.len()),
            "due to the clash happening at nearly any time, and keep-going is false, we get a variance of files"
        );
    } else {
        assert_eq!(
            stripped_prefix(&destination, &dest_files),
            paths(["D/B", "D/C", "FILE_X", "X", "link-to-X"]),
            "we checkout files in order and generally handle collision detection differently, hence the difference"
        );
    }

    let error_kind = ErrorKind::AlreadyExists;
    #[cfg(windows)]
    let error_kind_dir = ErrorKind::PermissionDenied;
    #[cfg(not(windows))]
    let error_kind_dir = error_kind;

    if multi_threaded() {
        assert!(
            (5..=6).contains(&outcome.collisions.len()),
            "can only assert on number as it's racily creating files so unclear which one clashes, and due to keep-going = false there is variance"
        );
    } else {
        assert_eq!(
            outcome.collisions,
            vec![
                Collision {
                    path: "d".into(),
                    error_kind: error_kind_dir,
                },
                Collision {
                    path: "FILE_x".into(),
                    error_kind,
                },
                Collision {
                    path: "file_X".into(),
                    error_kind,
                },
                Collision {
                    path: "file_x".into(),
                    error_kind,
                },
                Collision {
                    path: "x".into(),
                    error_kind,
                },
            ],
            "these files couldn't be checked out"
        );
    }
}

fn multi_threaded() -> bool {
    gix_features::parallel::num_threads(None) > 1
}

fn assert_equality(source_tree: &Path, destination: &TempDir, allow_symlinks: bool) -> crate::Result<usize> {
    let source_files = dir_structure(source_tree);
    let worktree_files = dir_structure(destination);

    assert_eq!(
        stripped_prefix(source_tree, &source_files),
        stripped_prefix(destination, &worktree_files),
    );

    let mut count = 0;
    for (source_file, worktree_file) in source_files.iter().zip(worktree_files.iter()) {
        count += 1;
        if !allow_symlinks && source_file.is_symlink() {
            assert!(!worktree_file.is_symlink());
            assert_eq!(fs::read(worktree_file)?.to_path()?, fs::read_link(source_file)?);
        } else {
            assert_eq!(fs::read(source_file)?, fs::read(worktree_file)?);
            #[cfg(unix)]
            assert_eq!(
                fs::symlink_metadata(source_file)?.mode() & 0o700,
                fs::symlink_metadata(worktree_file)?.mode() & 0o700,
                "permissions of source and checked out file are comparable"
            );
        }
    }
    Ok(count)
}

pub fn dir_structure<P: AsRef<std::path::Path>>(path: P) -> Vec<std::path::PathBuf> {
    let path = path.as_ref();
    let mut files: Vec<_> = walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| e.path() == path || !e.file_name().to_string_lossy().starts_with('.'))
        .flatten()
        .filter_map(|e| (!e.path().symlink_metadata().map_or(true, |m| m.is_dir())).then(|| e.path().to_path_buf()))
        .collect();
    files.sort();
    files
}

fn checkout_index_in_tmp_dir(
    opts: gix_worktree_state::checkout::Options,
    name: &str,
    subdir_name: Option<&str>,
) -> crate::Result<(PathBuf, TempDir, gix_index::File, gix_worktree_state::checkout::Outcome)> {
    checkout_index_in_tmp_dir_opts(opts, name, subdir_name, |_d| true, |_| Ok(()))
}

fn checkout_index_in_tmp_dir_opts(
    opts: gix_worktree_state::checkout::Options,
    script_name: &str,
    subdir_name: Option<&str>,
    allow_return_object: impl FnMut(&gix_hash::oid) -> bool + Send + Clone,
    prep_dest: impl Fn(&Path) -> std::io::Result<()>,
) -> crate::Result<(PathBuf, TempDir, gix_index::File, gix_worktree_state::checkout::Outcome)> {
    let source_tree = {
        let root = fixture_path(script_name);
        if let Some(name) = subdir_name {
            root.join(name)
        } else {
            root
        }
    };
    let git_dir = source_tree.join(".git");
    let mut index = gix_index::File::at(
        git_dir.join("index"),
        gix_testtools::object_hash(),
        false,
        Default::default(),
    )?;
    let odb = odb_at(git_dir.join("objects"))?.into_inner().into_arc()?;
    let destination = gix_testtools::tempfile::tempdir_in(std::env::current_dir()?)?;
    prep_dest(destination.path()).expect("preparation must succeed");

    #[derive(Clone)]
    struct MaybeFind<Allow: Clone, Find: Clone> {
        allow: std::cell::RefCell<Allow>,
        objects: Find,
    }

    impl<Allow, Find> gix_object::Find for MaybeFind<Allow, Find>
    where
        Allow: FnMut(&gix_hash::oid) -> bool + Send + Clone,
        Find: gix_object::Find + Send + Clone,
    {
        fn try_find<'a>(
            &self,
            id: &gix_hash::oid,
            buf: &'a mut Vec<u8>,
        ) -> Result<Option<Data<'a>>, gix_object::find::Error> {
            if (self.allow.borrow_mut())(id) {
                self.objects.try_find(id, buf)
            } else {
                Ok(None)
            }
        }
    }

    let db = MaybeFind {
        allow: allow_return_object.into(),
        objects: odb,
    };
    let outcome = gix_worktree_state::checkout(
        &mut index,
        destination.path(),
        db,
        &progress::Discard,
        &progress::Discard,
        &AtomicBool::default(),
        opts,
    )
    .map_err(gix_error::Exn::into_error)?;
    Ok((source_tree, destination, index, outcome))
}

fn stripped_prefix(prefix: impl AsRef<Path>, source_files: &[PathBuf]) -> Vec<&Path> {
    source_files.iter().flat_map(|p| p.strip_prefix(&prefix)).collect()
}

fn probe_gitoxide_dir() -> crate::Result<gix_fs::Capabilities> {
    Ok(gix_fs::Capabilities::probe(
        &gix_discover::upwards(".".as_ref())
            .map_err(gix_discover::upwards::Error::into_error)?
            .0
            .into_repository_and_work_tree_directories()
            .0,
    ))
}

fn opts_from_probe() -> gix_worktree_state::checkout::Options {
    static CAPABILITIES: LazyLock<gix_fs::Capabilities> = LazyLock::new(|| probe_gitoxide_dir().unwrap());

    gix_worktree_state::checkout::Options {
        fs: *CAPABILITIES,
        destination_is_initially_empty: true,
        thread_limit: gix_features::parallel::num_threads(None).into(),
        ..Default::default()
    }
}

fn paths<'a>(p: impl IntoIterator<Item = &'a str>) -> Vec<PathBuf> {
    p.into_iter().map(PathBuf::from).collect()
}

fn setup_filter_pipeline(opts: &mut gix_filter::pipeline::Options) {
    opts.eol_config.auto_crlf = gix_filter::eol::AutoCrlf::Enabled;
    opts.drivers = vec![gix_filter::Driver {
        name: "arrow".into(),
        clean: None,
        smudge: None,
        process: Some((driver_exe() + " process").into()),
        required: true,
    }];
}

#[test]
fn checkout_truncates_existing_longer_files() -> crate::Result {
    let mut opts = opts_from_probe();
    opts.overwrite_existing = false;
    opts.destination_is_initially_empty = false;

    // Use existing fixture and modify one file to be longer
    let (_source_tree, destination, _index, _outcome) = checkout_index_in_tmp_dir_opts(
        opts.clone(),
        "make_mixed_without_submodules_and_symlinks",
        None,
        |_| true,
        |dest| {
            // Create a longer version of the "executable" file before checkout
            let file_path = dest.join("executable");
            std::fs::create_dir_all(dest)?;
            std::fs::write(
                &file_path,
                b"This is much longer content that should be truncated to match git's version",
            )?;
            Ok(())
        },
    )?;

    let file_path = destination.path().join("executable");
    let final_content = std::fs::read(&file_path)?;
    assert_eq!(
        final_content[..].as_bstr(),
        "content",
        "File content should match git's version"
    );

    Ok(())
}
