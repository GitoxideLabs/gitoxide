mod get {
    use crate::TestResult;
    use crate::file::{bstring, mutable::multi_value::init_config};

    #[test]
    fn single_lines() -> TestResult {
        let mut config = init_config();

        let value = config.raw_values_mut_by("core", None, "a")?;
        assert_eq!(value.get()?, vec![bstring("b100"), bstring("d"), bstring("f"),]);
        Ok(())
    }

    #[test]
    fn multi_line() -> TestResult {
        let mut config: gix_config::File = r#"[core]
            a=b\
"100"
        [core]
            a=d\
"b  "\
c
            a=f\
   a"#
        .parse()?;

        let mut values = config.raw_values_mut_by("core", None, "a")?;
        assert_eq!(
            &*values.get()?,
            vec![bstring("b100"), bstring("db  c"), bstring("f   a"),]
        );

        values.delete_all();
        assert!(values.get().is_err());

        Ok(())
    }

    #[test]
    fn value_names_are_case_insensitive() -> TestResult {
        let mut config: gix_config::File = "[core]\nMixedCase = one\nMIXEDCASE = two".parse()?;
        assert_eq!(
            config.raw_values_mut_by("core", None, "mixedcase")?.get()?,
            vec![bstring("one"), bstring("two")]
        );
        Ok(())
    }
}

mod access {
    use crate::TestResult;
    use crate::file::mutable::multi_value::init_config;

    #[test]
    fn non_empty_sizes() -> TestResult {
        let mut config = init_config();
        assert_eq!(config.raw_values_mut_by("core", None, "a")?.len(), 3);
        assert!(!config.raw_values_mut_by("core", None, "a")?.is_empty());
        Ok(())
    }
}

mod set {
    use crate::TestResult;
    use crate::file::{bstring, mutable::multi_value::init_config};

    #[test]
    fn values_are_escaped() -> TestResult {
        for value in ["a b", " a b", "a b\t", ";c", "#c", "a\nb\n\tc"] {
            let mut config = init_config();
            let mut values = config.raw_values_mut_by("core", None, "a")?;
            values.set_all(value)?;

            let config_str = config.to_string();
            let config: gix_config::File = config_str.parse()?;
            assert_eq!(
                config.raw_values("core.a")?,
                vec![bstring(value), bstring(value), bstring(value)],
                "{config_str}"
            );
        }
        Ok(())
    }

    #[test]
    fn single_at_start() -> TestResult {
        let mut config = init_config();
        let mut values = config.raw_values_mut_by("core", None, "a")?;
        values.set_string_at(0, "Hello")?;
        assert_eq!(
            config.to_string(),
            "[core]\n    a = Hello\n    [core]\n        a =d\n        a= f\n"
        );
        Ok(())
    }

    #[test]
    fn single_at_end() -> TestResult {
        let mut config = init_config();
        let mut values = config.raw_values_mut_by("core", None, "a")?;
        values.set_string_at(2, "Hello")?;
        assert_eq!(
            config.to_string(),
            "[core]\n    a = b\"100\"\n    [core]\n        a =d\n        a= Hello\n"
        );
        Ok(())
    }

    #[test]
    fn all() -> TestResult {
        let mut config = init_config();
        let mut values = config.raw_values_mut_by("core", None, "a")?;
        values.set_all("Hello")?;
        assert_eq!(
            config.to_string(),
            "[core]\n    a = Hello\n    [core]\n        a= Hello\n        a =Hello\n"
        );
        Ok(())
    }

    #[test]
    fn all_empty() -> TestResult {
        let mut config = init_config();
        let mut values = config.raw_values_mut_by("core", None, "a")?;
        values.set_all("")?;
        assert_eq!(
            config.to_string(),
            "[core]\n    a = \n    [core]\n        a= \n        a =\n"
        );
        Ok(())
    }
}

mod delete {
    use crate::TestResult;
    use crate::file::mutable::multi_value::init_config;

    #[test]
    fn single_at_start_and_end() -> TestResult {
        let mut config = init_config();
        {
            let mut values = config.raw_values_mut_by("core", None, "a")?;
            values.delete(0);
            assert_eq!(
                config.to_string(),
                "[core]\n    \n    [core]\n        a =d\n        a= f\n",
            );
        }

        let mut values = config.raw_values_mut_by("core", None, "a")?;
        values.delete(1);
        assert_eq!(config.to_string(), "[core]\n    \n    [core]\n        a =d\n        ");
        Ok(())
    }

    #[test]
    fn all() -> TestResult {
        let mut config = init_config();
        let mut values = config.raw_values_mut_by("core", None, "a")?;
        values.delete_all();
        values.delete_all();
        assert!(values.get().is_err());
        assert_eq!(config.to_string(), "[core]\n    \n    [core]\n        \n        ");
        Ok(())
    }
}

fn init_config() -> gix_config::File {
    r#"[core]
    a = b"100"
    [core]
        a =d
        a= f"#
        .parse()
        .unwrap()
}

mod replace {
    use crate::TestResult;

    #[test]
    fn selected_values_collapse_at_the_last_match_like_git() -> TestResult {
        let input = "[core]\n a = first\n keep = before\n a = second\n\
                     [other]\n marker = between\n\
                     [core]\n a\n keep = after\n a =\n a = tail\n";
        let dir = gix_testtools::tempfile::tempdir()?;
        for (input, indices, pattern, replacement) in [
            (input, &[][..], Some("^missing$"), "new"),
            (input, &[0][..], Some("^first$"), "new"),
            (input, &[0, 1][..], Some("^(first|second)$"), "new"),
            (input, &[0, 1, 2, 3, 4][..], None, "new"),
            (input, &[2, 3][..], Some("!."), ""),
            ("[core]\n a\n keep = neighbor\n", &[0][..], None, "new"),
            (input, &[1, 4][..], Some("^(second|tail)$"), " #new\nvalue"),
        ] {
            std::fs::write(dir.path().join("git-config"), input)?;
            let mut git = gix_testtools::git_command(dir.path());
            git.args(["config", "--file", "git-config", "--replace-all", "core.a", replacement]);
            if let Some(pattern) = pattern {
                git.arg(pattern);
            }
            let result = git.output()?;
            assert!(
                result.status.success(),
                "Git accepts {pattern:?}: {}",
                String::from_utf8_lossy(&result.stderr)
            );

            let mut file = gix_config::File::try_from(input)?;
            let before = file.to_bstring();
            file.raw_values_mut("core.a")?
                .replace_at_indices(indices, replacement)?;
            if indices.is_empty() {
                assert_eq!(
                    file.to_bstring(),
                    before,
                    "an empty selection leaves the file unchanged"
                );
                file.section_mut("core", None)?.push("a", replacement)?;
            }
            std::fs::write(dir.path().join("actual-config"), file.to_bstring())?;
            assert_eq!(
                gix_testtools::git(dir.path(), "config --file actual-config --list")?,
                gix_testtools::git(dir.path(), "config --file git-config --list")?,
                "{indices:?}: replacements, implicit values, and neighboring entries retain Git's order"
            );
        }
        Ok(())
    }

    #[test]
    fn invalid_indices_leave_all_values_unchanged() -> TestResult {
        for indices in [&[usize::MAX][..], &[0, 3][..], &[1, 1][..], &[2, 0][..]] {
            let mut file = super::init_config();
            let before = file.to_bstring();
            let err = file
                .raw_values_mut("core.a")?
                .replace_at_indices(indices, "new")
                .expect_err("indices must be sorted, unique, and in range");
            assert!(err.is_validation(), "invalid indices are validation errors");
            assert_eq!(
                file.to_bstring(),
                before,
                "{indices:?} must be validated before any mutation"
            );
        }
        Ok(())
    }
}
