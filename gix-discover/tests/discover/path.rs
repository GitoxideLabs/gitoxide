mod from_git_dir_file {
    use crate::Result;
    use std::{
        io::Write,
        path::{Path, PathBuf},
    };

    use gix_testtools::tempfile::NamedTempFile;

    #[cfg(not(windows))]
    #[test]
    fn absolute_path_unix() -> Result {
        let (path, _) = write_and_read(b"gitdir: /absolute/path/.git")?;
        assert_eq!(path, Path::new("/absolute/path/.git"));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn absolute_path_windows() -> Result {
        let (path, _) = write_and_read(b"gitdir: C:/absolute/path/.git")?;
        assert_eq!(path, Path::new("C:/absolute/path/.git"));

        let (path, _) = write_and_read(br"gitdir: C:\absolute\path\.git")?;
        assert_eq!(path, Path::new(r"C:\absolute\path\.git"));
        Ok(())
    }

    #[test]
    fn relative_path_is_made_absolute_relative_to_containing_dir() -> Result {
        let (path, gitdir_file) = write_and_read(b"gitdir: relative/path")?;
        assert_eq!(path, gitdir_file.parent().unwrap().join(Path::new("relative/path")));
        Ok(())
    }

    fn write_and_read(content: &[u8]) -> Result<(PathBuf, PathBuf)> {
        let file = gitdir_with_content(content)?;
        Ok((gix_discover::path::from_gitdir_file(file.path())?, file.path().into()))
    }

    fn gitdir_with_content(content: &[u8]) -> std::io::Result<NamedTempFile> {
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(content)?;
        Ok(file)
    }
}

mod from_plain_file_relative_to_file {
    use crate::Result;
    use crate::path::plain_file_with_content;
    use std::path::{Path, PathBuf};

    #[test]
    fn relative_path_is_made_absolute_relative_to_containing_dir() -> Result {
        let (path, plain_file) = write_and_read(b"relative/path\n")?;
        assert_eq!(path, plain_file.parent().unwrap().join(Path::new("relative/path")));
        Ok(())
    }

    #[test]
    fn empty_or_whitespace_only_path_is_invalid() -> Result {
        let mut error_snapshots = Vec::new();
        for content in [b"".as_slice(), b"   \n".as_slice()] {
            let file = plain_file_with_content(content)?;
            let err = gix_discover::path::from_plain_file_relative_to_file(file.path())
                .expect("file exists")
                .expect_err("empty paths must be rejected");
            error_snapshots.push(gix_testtools::redact_debug_snapshot(
                &(err),
                &[(&(file.path()).to_string_lossy(), "<path-file>")],
            ));
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::InvalidData,
                "empty plain path files are malformed, just like in Git"
            );
        }
        insta::assert_debug_snapshot!(error_snapshots, "empty or whitespace only path is invalid", @r#"
        [
            Custom {
                kind: InvalidData,
                error: "Refusing to read an empty path from '<path-file>'",
            },
            Custom {
                kind: InvalidData,
                error: "Refusing to read an empty path from '<path-file>'",
            },
        ]
        "#);
        Ok(())
    }

    fn write_and_read(content: &[u8]) -> Result<(PathBuf, PathBuf)> {
        let file = plain_file_with_content(content)?;
        Ok((
            gix_discover::path::from_plain_file_relative_to_file(file.path())
                .expect("file exists")
                .expect("valid plain path"),
            file.path().into(),
        ))
    }
}

mod from_plain_file {
    use crate::Result;
    use crate::path::plain_file_with_content;

    #[test]
    fn empty_or_whitespace_only_path_is_invalid() -> Result {
        let mut error_snapshots = Vec::new();
        for content in [b"".as_slice(), b"   \n".as_slice()] {
            let file = plain_file_with_content(content)?;
            let err = gix_discover::path::from_plain_file(file.path())
                .expect("file exists")
                .expect_err("empty paths must be rejected");
            error_snapshots.push(gix_testtools::redact_debug_snapshot(
                &(err),
                &[(&(file.path()).to_string_lossy(), "<path-file>")],
            ));
            assert_eq!(
                err.kind(),
                std::io::ErrorKind::InvalidData,
                "empty plain path files are malformed, just like in Git"
            );
        }
        insta::assert_debug_snapshot!(error_snapshots, "empty or whitespace only path is invalid", @r#"
        [
            Custom {
                kind: InvalidData,
                error: "Refusing to read an empty path from '<path-file>'",
            },
            Custom {
                kind: InvalidData,
                error: "Refusing to read an empty path from '<path-file>'",
            },
        ]
        "#);
        Ok(())
    }
}

fn plain_file_with_content(content: &[u8]) -> std::io::Result<tempfile::NamedTempFile> {
    let mut file = tempfile::NamedTempFile::new()?;
    std::io::Write::write_all(&mut file, content)?;
    Ok(file)
}

#[test]
fn repository_kind() {
    use gix_discover::path::{RepositoryKind::*, repository_kind};
    assert_eq!(repository_kind("hello".as_ref()), None);
    assert_eq!(repository_kind(".git".as_ref()), Some(Common));
    assert_eq!(repository_kind("foo/.git".as_ref()), Some(Common));
    assert_eq!(
        repository_kind("foo/other.git".as_ref()),
        None,
        "it makes no assumption beyond the standard name, nor does it consider suffixes"
    );
    assert_eq!(repository_kind(".git/modules".as_ref()), None);
    assert_eq!(
        repository_kind(".git/modules/actual-submodule".as_ref()),
        Some(Submodule)
    );
    assert_eq!(
        repository_kind(".git/worktrees/actual-worktree".as_ref()),
        Some(LinkedWorktree)
    );
}
