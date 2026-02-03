mod from_git_dir_file {
    use std::{
        io::Write,
        path::{Path, PathBuf},
    };

    use gix_testtools::tempfile::NamedTempFile;

    #[cfg(not(windows))]
    #[test]
    fn absolute_path_unix() -> crate::Result {
        let (path, _) = write_and_read(b"gitdir: /absolute/path/.git")?;
        assert_eq!(path, Path::new("/absolute/path/.git"));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn absolute_path_windows() -> crate::Result {
        let (path, _) = write_and_read(b"gitdir: C:/absolute/path/.git")?;
        assert_eq!(path, Path::new("C:/absolute/path/.git"));

        let (path, _) = write_and_read(br"gitdir: C:\absolute\path\.git")?;
        assert_eq!(path, Path::new(r"C:\absolute\path\.git"));
        Ok(())
    }

    #[test]
    fn relative_path_is_made_absolute_relative_to_containing_dir() -> crate::Result {
        let (path, gitdir_file) = write_and_read(b"gitdir: relative/path")?;
        assert_eq!(path, gitdir_file.parent().unwrap().join(Path::new("relative/path")));
        Ok(())
    }

    fn write_and_read(content: &[u8]) -> crate::Result<(PathBuf, PathBuf)> {
        let file = gitdir_with_content(content)?;
        Ok((gix_discover::path::from_gitdir_file(file.path())?, file.path().into()))
    }

    fn gitdir_with_content(content: &[u8]) -> std::io::Result<NamedTempFile> {
        let mut file = tempfile::NamedTempFile::new()?;
        file.write_all(content)?;
        Ok(file)
    }
}

#[test]
fn repository_kind() {
    use gix_discover::path::{repository_kind, RepositoryKind::*};
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
