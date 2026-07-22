use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use bstr::ByteVec;
use gix_path::{realpath::Error, realpath_opts};
use gix_testtools::tempfile;

#[test]
fn fuzzed_timeout() -> crate::Result {
    let path = PathBuf::from(std::fs::read("tests/fixtures/fuzzed/54k-path-components.path")?.into_string()?);
    assert_eq!(path.components().count(), 54862);
    let start = std::time::Instant::now();
    assert!(matches!(
        gix_path::realpath_opts(&path, Path::new("/cwd"), gix_path::realpath::MAX_SYMLINKS).unwrap_err(),
        gix_path::realpath::Error::ExcessiveComponentCount {
            max_symlink_checks: 2048
        }
    ));
    assert!(
        start.elapsed() < Duration::from_millis(if cfg!(windows) { 2000 } else { 1000 }),
        "took too long: {:.02} , we can't take too much time for this, and should keep the amount of work reasonable\
        as paths can be part of URls which sometimes are canonicalized",
        start.elapsed().as_secs_f32()
    );
    Ok(())
}

#[test]
fn assorted() -> crate::Result {
    let cwd = tempfile::tempdir()?;
    let cwd = cwd.path();
    let symlinks_disabled = 0;

    assert!(
        matches!(
            realpath_opts("".as_ref(), cwd, symlinks_disabled),
            Err(Error::EmptyPath)
        ),
        "Empty path is not allowed"
    );

    assert_eq!(
        realpath_opts("b/.git".as_ref(), cwd, symlinks_disabled)?,
        cwd.join("b").join(".git"),
        "relative paths are prefixed with current dir"
    );

    assert_eq!(
        realpath_opts("b//.git".as_ref(), cwd, symlinks_disabled)?,
        cwd.join("b").join(".git"),
        "empty path components are ignored"
    );

    assert_eq!(
        realpath_opts("./tmp/.git".as_ref(), cwd, symlinks_disabled)?,
        cwd.join("tmp").join(".git"),
        "path starting with dot is relative and is prefixed with current dir"
    );

    assert_eq!(
        realpath_opts("./tmp/a/./.git".as_ref(), cwd, symlinks_disabled)?,
        cwd.join("tmp").join("a").join(".git"),
        "all ./ path components are ignored unless they the one at the beginning of the path"
    );

    assert_eq!(
        realpath_opts("./b/../tmp/.git".as_ref(), cwd, symlinks_disabled)?,
        cwd.join("tmp").join(".git"),
        "dot dot goes to parent path component"
    );

    {
        #[cfg(not(windows))]
        let absolute_path = Path::new("/c/d/.git");
        #[cfg(windows)]
        let absolute_path = Path::new(r"C:\c\d\.git");
        assert_eq!(
            realpath_opts(absolute_path, cwd, symlinks_disabled)?,
            absolute_path,
            "absolute path without symlinks has nothing to resolve and remains unchanged"
        );
    }

    Ok(())
}

#[test]
fn link_cycle_is_detected() -> crate::Result {
    let tmp_dir = canonicalized_tempdir()?;
    let dir = tmp_dir.path();
    let link_name = "link";
    let link_destination = dir.join(link_name);
    let link_path = dir.join(link_name);
    create_symlink(&link_path, link_destination)?;
    let max_symlinks = 8;

    assert!(
        matches!(
            realpath_opts(&link_path.join(".git"), "".as_ref(), max_symlinks),
            Err(Error::MaxSymlinksExceeded { max_symlinks: 8 })
        ),
        "link cycle is detected"
    );
    Ok(())
}

#[test]
fn symlink_with_absolute_path_gets_expanded() -> crate::Result {
    let tmp_dir = canonicalized_tempdir()?;
    let dir = tmp_dir.path();
    let link_from = dir.join("a").join("b").join("tmp_p_q_link");
    let link_to = dir.join("p").join("q");
    create_symlink(&link_from, &link_to)?;
    let max_symlinks = 8;
    assert_eq!(
        realpath_opts(&link_from.join(".git"), tmp_dir.path(), max_symlinks)?,
        link_to.join(".git"),
        "symlink with absolute path gets expanded"
    );
    Ok(())
}

#[test]
fn symlink_to_relative_path_gets_expanded_into_absolute_path() -> crate::Result {
    let cwd = canonicalized_tempdir()?;
    let dir = cwd.path();
    let link_name = "pq_link";
    create_symlink(dir.join("r").join(link_name), Path::new("p").join("q"))?;
    assert_eq!(
        realpath_opts(&Path::new(link_name).join(".git"), &dir.join("r"), 8)?,
        dir.join("r").join("p").join("q").join(".git"),
        "symlink to relative path gets expanded into absolute path"
    );
    Ok(())
}

#[test]
fn symlink_processing_is_disabled_if_the_value_is_zero() -> crate::Result {
    let cwd = canonicalized_tempdir()?;
    let link_name = "x_link";
    create_symlink(cwd.path().join(link_name), Path::new("link destination does not exist"))?;
    assert!(
        matches!(
            realpath_opts(&Path::new(link_name).join(".git"), cwd.path(), 0),
            Err(Error::MaxSymlinksExceeded { max_symlinks: 0 })
        ),
        "symlink processing is disabled if the value is zero"
    );
    Ok(())
}

fn create_symlink(from: impl AsRef<Path>, to: impl AsRef<Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(from.as_ref().parent().unwrap())?;

    #[cfg(not(windows))]
    {
        std::os::unix::fs::symlink(to, from)
    }

    #[cfg(windows)]
    std::os::windows::fs::symlink_file(to, from)
}

fn canonicalized_tempdir() -> crate::Result<tempfile::TempDir> {
    let canonicalized_tempdir = gix_path::realpath(std::env::temp_dir())?;
    Ok(tempfile::tempdir_in(canonicalized_tempdir)?)
}
