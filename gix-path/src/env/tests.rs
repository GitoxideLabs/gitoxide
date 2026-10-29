use std::path::{Path, PathBuf};

pub(super) struct CurrentDir(PathBuf);

/// This is a copy from the respective type in `gix-testtools` - deduplicate if it can ever be a dependency again.
impl CurrentDir {
    pub(super) fn set(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let previous = std::env::current_dir()?;
        std::env::set_current_dir(path)?;
        Ok(CurrentDir(previous))
    }
}

impl Drop for CurrentDir {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.0).expect("previous current directory is still accessible");
    }
}

mod system_prefix {
    use super::{
        super::{system_prefix_from_core_dir, system_prefix_from_exepath_var},
        CurrentDir,
    };
    use serial_test::serial;
    use std::{
        ffi::OsString,
        path::{Path, PathBuf},
    };

    #[test]
    fn core_dir_keeps_the_runtime_prefix() {
        for (core_dir, prefix) in [
            ("C:/Git/mingw64/libexec/git-core", "C:/Git/mingw64"),
            ("C:/Git/ucrt64/libexec/git-core", "C:/Git/ucrt64"),
            ("C:/Git/mingw32/libexec/git-core", "C:/Git/mingw32"),
            ("C:/Git/clangarm64/libexec/git-core", "C:/Git/clangarm64"),
            #[cfg(windows)]
            ("C:/msys64/usr/lib/git-core", "C:/msys64/usr"),
            ("C:/libexec/Git/ucrt64/libexec/git-core", "C:/libexec/Git/ucrt64"),
        ] {
            assert_eq!(
                system_prefix_from_core_dir(|| Some(Path::new(core_dir))),
                Some(PathBuf::from(prefix)),
                "the runtime prefix is the directory immediately above Git's libexec directory"
            );
        }
    }

    #[test]
    fn core_dir_without_a_runtime_prefix() {
        assert_eq!(system_prefix_from_core_dir(|| None), None);
        for core_dir in ["C:/Git/ucrt64/bin", "libexec/git-core"] {
            assert_eq!(
                system_prefix_from_core_dir(|| Some(Path::new(core_dir))),
                None,
                "a recognizable, nonempty prefix is required"
            );
        }
    }

    #[test]
    #[cfg(windows)]
    fn config_is_next_to_the_runtime_prefix() {
        for runtime in ["mingw64", "ucrt64", "mingw32", "clangarm64", "usr"] {
            assert_eq!(
                super::super::config_path_from_system_prefix(&Path::new("C:/Git").join(runtime)),
                PathBuf::from("C:/Git/etc/gitconfig"),
                "Git for Windows builds ETC_GITCONFIG as ../etc/gitconfig"
            );
        }
    }

    fn if_exepath(key: &str, value: impl Into<OsString>) -> Option<OsString> {
        match key {
            "EXEPATH" => Some(value.into()),
            _ => None,
        }
    }

    struct ExePath {
        _tempdir: tempfile::TempDir,
        path: PathBuf,
    }

    impl ExePath {
        fn new() -> Self {
            let tempdir = tempfile::tempdir().expect("can create new temporary directory");

            // This is just `tempdir.path()` unless it is relative, in which case it is resolved.
            let path = std::env::current_dir()
                .expect("can get current directory")
                .join(tempdir.path());

            Self {
                _tempdir: tempdir,
                path,
            }
        }

        fn create_subdir(&self, name: &str) -> PathBuf {
            let child = self.path.join(name);
            std::fs::create_dir(&child).expect("can create subdirectory");
            child
        }

        fn create_separate_subdirs(&self, names: &[&str]) {
            for name in names {
                self.create_subdir(name);
            }
        }

        fn create_separate_regular_files(&self, names: &[&str]) {
            for name in names {
                std::fs::File::create_new(self.path.join(name)).expect("can create new file");
            }
        }

        fn var_os_func(&self, key: &str) -> Option<OsString> {
            if_exepath(key, self.path.as_os_str())
        }
    }

    #[test]
    fn exepath_unset() {
        let outcome = system_prefix_from_exepath_var(|_| None);
        assert_eq!(outcome, None);
    }

    #[test]
    #[serial]
    fn exepath_no_relevant_subdir() {
        for names in [&[][..], &["clang64"][..]] {
            let exepath = ExePath::new();
            exepath.create_separate_subdirs(names);
            let outcome = system_prefix_from_exepath_var(|key| exepath.var_os_func(key));
            assert_eq!(outcome, None);
        }
    }

    #[test]
    #[serial]
    fn exepath_unambiguous_subdir() {
        for name in ["clangarm64", "ucrt64", "mingw64", "mingw32"] {
            let exepath = ExePath::new();
            let subdir = exepath.create_subdir(name);
            let outcome = system_prefix_from_exepath_var(|key| exepath.var_os_func(key));
            assert_eq!(outcome, Some(subdir), "{name} is an unambiguous Git for Windows prefix");
        }
    }

    #[test]
    #[serial]
    fn exepath_unambiguous_subdir_beside_strange_files() {
        for (dirname, filenames) in [
            ("clangarm64", ["ucrt64", "mingw64", "mingw32"]),
            ("ucrt64", ["clangarm64", "mingw64", "mingw32"]),
            ("mingw64", ["clangarm64", "ucrt64", "mingw32"]),
            ("mingw32", ["clangarm64", "ucrt64", "mingw64"]),
        ] {
            let exepath = ExePath::new();
            let subdir = exepath.create_subdir(dirname);
            exepath.create_separate_regular_files(&filenames);
            let outcome = system_prefix_from_exepath_var(|key| exepath.var_os_func(key));
            assert_eq!(
                outcome,
                Some(subdir),
                "only directories can be Git for Windows prefixes"
            );
        }
    }

    #[test]
    #[serial]
    fn exepath_ambiguous_subdir() {
        for names in [
            &["ucrt64", "mingw64"][..],
            &["ucrt64", "mingw32"][..],
            &["clangarm64", "ucrt64"][..],
            &["mingw32", "mingw64"][..],
            &["mingw32", "clangarm64"][..],
            &["mingw64", "clangarm64"][..],
            &["mingw32", "mingw64", "clangarm64"][..],
            &["clangarm64", "ucrt64", "mingw64", "mingw32"][..],
        ] {
            let exepath = ExePath::new();
            exepath.create_separate_subdirs(names);
            let outcome = system_prefix_from_exepath_var(|key| exepath.var_os_func(key));
            assert_eq!(
                outcome, None,
                "multiple prefixes require querying Git to disambiguate {names:?}"
            );
        }
    }

    #[test]
    #[serial]
    fn exepath_empty_string() {
        for name in ["clangarm64", "ucrt64", "mingw64", "mingw32"] {
            let exepath = ExePath::new();
            exepath.create_subdir(name);
            let _cwd = CurrentDir::set(&exepath.path).expect("can change to test dir");
            let outcome = system_prefix_from_exepath_var(|key| if_exepath(key, ""));
            assert_eq!(outcome, None);
        }
    }

    #[test]
    #[serial]
    fn exepath_nonempty_relative() -> gix_testtools::TestResult {
        for name in ["clangarm64", "ucrt64", "mingw64", "mingw32"] {
            let grandparent = tempfile::tempdir()?;
            let parent = grandparent.path().canonicalize()?.join("dir");
            std::fs::create_dir_all(parent.join(name))?;
            let _cwd = CurrentDir::set(grandparent.path())?;
            let outcome = system_prefix_from_exepath_var(|key| if_exepath(key, "dir"));
            assert_eq!(outcome, None);
        }
        Ok(())
    }
}
