mod get {
    use crate::Result;
    use crate::file::{bstring, mutable::multi_value::init_config};

    #[test]
    fn single_lines() -> Result {
        let mut config = init_config();

        let value = config.raw_values_mut_by("core", None, "a")?;
        assert_eq!(value.get()?, vec![bstring("b100"), bstring("d"), bstring("f"),]);
        Ok(())
    }

    #[test]
    fn multi_line() -> Result {
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
    fn value_names_are_case_insensitive() -> Result {
        let mut config: gix_config::File = "[core]\nMixedCase = one\nMIXEDCASE = two".parse()?;
        assert_eq!(
            config.raw_values_mut_by("core", None, "mixedcase")?.get()?,
            vec![bstring("one"), bstring("two")]
        );
        Ok(())
    }
}

mod access {
    use crate::Result;
    use crate::file::mutable::multi_value::init_config;

    #[test]
    fn non_empty_sizes() -> Result {
        let mut config = init_config();
        assert_eq!(config.raw_values_mut_by("core", None, "a")?.len(), 3);
        assert!(!config.raw_values_mut_by("core", None, "a")?.is_empty());
        Ok(())
    }
}

mod set {
    use crate::Result;
    use crate::file::{bstring, mutable::multi_value::init_config};

    #[test]
    fn values_are_escaped() -> Result {
        for value in ["a b", " a b", "a b\t", ";c", "#c", "a\nb\n\tc"] {
            let mut config = init_config();
            let mut values = config.raw_values_mut_by("core", None, "a")?;
            values.set_all(value)?;

            let config_str = config.to_string();
            let config: gix_config::File = config_str.parse()?;
            assert_eq!(
                config.raw_values("core.a")?,
                vec![bstring(value), bstring(value), bstring(value)],
                "{config_str:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn single_at_start() -> Result {
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
    fn single_at_end() -> Result {
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
    fn all() -> Result {
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
    fn all_empty() -> Result {
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
    use crate::Result;
    use crate::file::mutable::multi_value::init_config;

    #[test]
    fn single_at_start_and_end() -> Result {
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
    fn all() -> Result {
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
