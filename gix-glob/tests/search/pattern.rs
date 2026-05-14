mod list {
    use std::path::Path;

    use gix_glob::{
        pattern::Case,
        search::{
            Pattern,
            pattern::{List, Mapping},
        },
    };

    #[derive(Clone, PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Default)]
    struct Dummy;

    impl Pattern for Dummy {
        type Value = ();

        fn bytes_to_patterns(&self, _bytes: &[u8], _source: &Path) -> Vec<Mapping<Self::Value>> {
            vec![]
        }
    }

    #[test]
    fn from_bytes_base() {
        {
            let list = List::from_bytes(&[], "a/b/source".into(), None, Dummy);
            assert_eq!(list.base, None, "no root always means no-base, i.e. globals lists");
            assert_eq!(
                list.source.as_deref(),
                Some(Path::new("a/b/source")),
                "source is verbatim"
            );
        }

        {
            let cwd = std::env::current_dir().expect("cwd available");
            let list = List::from_bytes(&[], cwd.join("a/b/source"), Some(cwd.as_path()), Dummy);
            assert_eq!(
                list.base.as_ref().expect("set"),
                "a/b/",
                "bases are always relative, needs properly set root"
            );
            assert_eq!(
                list.source.as_deref(),
                Some(cwd.join("a/b/source").as_path()),
                "source is verbatim"
            );
        }

        {
            let list = List::from_bytes(&[], "a/b/source".into(), Some(Path::new("c/")), Dummy);
            assert_eq!(
                list.base, None,
                "if root doesn't contain source, it silently skips it as base"
            );
            assert_eq!(
                list.source.as_deref(),
                Some(Path::new("a/b/source")),
                "source is always verbatim"
            );
        }
    }

    #[test]
    fn strip_base_handle_recompute_basename_pos() {
        let list = List::from_bytes(&[], "a/b/source".into(), Some(Path::new("")), Dummy);
        assert_eq!(
            list.base.as_ref().expect("set"),
            "a/b/",
            "bases are computed if `root` is set, and always uses slashes"
        );
        let res = list.strip_base_handle_recompute_basename_pos("a/b/file".into(), Some(4), Case::Sensitive);
        assert_eq!(
            res,
            Some(("file".into(), None)),
            "files don't have a basename position anymore"
        );

        let res = list.strip_base_handle_recompute_basename_pos("a/B/c/File".into(), Some(6), Case::Fold);
        assert_eq!(
            res,
            Some(("c/File".into(), Some(2))),
            "otherwise the basename is recomputed, case folding is effective"
        );
    }

    #[test]
    fn from_file_that_does_not_exist() {
        let mut buf = Vec::new();
        for path in [
            Path::new(".").join("non-existing-dir").join("pattern-file"),
            Path::new("file").to_owned(),
        ] {
            let list = List::from_file(path, None, false, &mut buf, Dummy).expect("no io error");
            assert!(list.is_none(), "the file does not exist");
        }
    }

    #[test]
    fn from_file_that_is_a_directory() -> gix_testtools::Result<()> {
        let tmp = gix_testtools::tempfile::TempDir::new()?;
        let dir_path = tmp.path().join(".gitignore");
        std::fs::create_dir(&dir_path)?;
        let mut buf = Vec::new();
        let list = List::from_file(dir_path, None, false, &mut buf, Dummy).expect("no io error");
        assert!(list.is_none(), "directories are ignored just like Git does it");

        Ok(())
    }
}
