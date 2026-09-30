use std::{ffi::OsStr, io::ErrorKind};

use gix_fs::{walkdir_new, walkdir_sorted_new};

#[test]
fn hidden_entries_and_depth_limits() -> gix_testtools::Result {
    let dir = gix_testtools::scripted_fixture_read_only("walkdir.sh")?.join("hidden");

    let mut names = walkdir_new(&dir, false)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .map(|entry| entry.map(|entry| entry.file_name().into_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    names.sort();
    assert_eq!(
        names,
        [".hidden", "directory"],
        "hidden entries are included, while the root and descendants beyond max_depth are omitted"
    );
    Ok(())
}

#[test]
fn sorted_walk_uses_git_directory_order() -> gix_testtools::Result {
    let dir = gix_testtools::scripted_fixture_read_only("walkdir.sh")?.join("sorted");

    let names = walkdir_sorted_new(&dir, 1, false)
        .min_depth(1)
        .into_iter()
        .map(|entry| entry.map(|entry| entry.file_name().into_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        names,
        ["common.", "common", "common0"],
        "directories sort as if their names ended in a slash, and the depth limit excludes their children"
    );
    Ok(())
}

#[test]
fn unicode_precomposition_applies_to_names_and_paths() -> gix_testtools::Result {
    let dir = gix_testtools::scripted_fixture_read_only("walkdir.sh")?.join("unicode");
    let decomposed = "a\u{308}";
    let root = dir.join(decomposed);

    for (precompose_unicode, expected) in [(false, decomposed), (true, "ä")] {
        for walk in [
            walkdir_new(&root, precompose_unicode),
            walkdir_sorted_new(&root, 1, precompose_unicode),
        ] {
            let mut entries = walk.min_depth(1).max_depth(1).into_iter();
            let entry = entries.next().expect("the directory contains one file")?;
            assert_eq!(
                entry.file_name(),
                OsStr::new(expected),
                "file names follow the requested Unicode precomposition policy"
            );
            assert_eq!(
                entry.path(),
                dir.join(expected).join(expected),
                "the same policy applies to every component of the full path"
            );
            assert!(entry.file_type()?.is_file(), "entry metadata remains accessible");
            assert!(entries.next().is_none(), "only the direct child is returned");
        }
    }
    Ok(())
}

#[test]
fn missing_roots_yield_io_errors() -> gix_testtools::Result {
    let dir = gix_testtools::scripted_fixture_read_only("walkdir.sh")?;
    let mut entries = walkdir_new(&dir.join("missing"), false).into_iter();
    let err = entries
        .next()
        .expect("an inaccessible root produces an error item")
        .expect_err("missing directories cannot be traversed");
    assert_eq!(
        err.io_error().map(std::io::Error::kind),
        Some(ErrorKind::NotFound),
        "the underlying IO error is preserved"
    );
    assert!(entries.next().is_none(), "there are no entries below a missing root");
    Ok(())
}
