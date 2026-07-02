mod get {
    use crate::file::{cow_str, mutable::multi_value::init_config};

    #[test]
    fn single_lines() -> crate::Result {
        let mut config = init_config();

        let value = config.raw_values_mut_by("core", None, "a")?;
        assert_eq!(value.get()?, vec![cow_str("b100"), cow_str("d"), cow_str("f"),]);
        Ok(())
    }

    #[test]
    fn multi_line() -> crate::Result {
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
            vec![cow_str("b100"), cow_str("db  c"), cow_str("f   a"),]
        );

        values.delete_all();
        assert!(values.get().is_err());

        Ok(())
    }
}

mod access {
    use crate::file::mutable::multi_value::init_config;

    #[test]
    fn non_empty_sizes() -> crate::Result {
        let mut config = init_config();
        assert_eq!(config.raw_values_mut_by("core", None, "a")?.len(), 3);
        assert!(!config.raw_values_mut_by("core", None, "a")?.is_empty());
        Ok(())
    }
}

mod set {
    use crate::file::{cow_str, mutable::multi_value::init_config};

    #[test]
    fn values_are_escaped() -> crate::Result {
        for value in ["a b", " a b", "a b\t", ";c", "#c", "a\nb\n\tc"] {
            let mut config = init_config();
            let mut values = config.raw_values_mut_by("core", None, "a")?;
            values.set_all(value);

            let config_str = config.to_string();
            let config: gix_config::File = config_str.parse()?;
            assert_eq!(
                config.raw_values("core.a")?,
                vec![cow_str(value), cow_str(value), cow_str(value)],
                "{config_str:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn single_at_start() -> crate::Result {
        let mut config = init_config();
        let mut values = config.raw_values_mut_by("core", None, "a")?;
        values.set_string_at(0, "Hello");
        assert_eq!(
            config.to_string(),
            "[core]\n    a = Hello\n    [core]\n        a =d\n        a= f\n"
        );
        Ok(())
    }

    #[test]
    fn single_at_end() -> crate::Result {
        let mut config = init_config();
        let mut values = config.raw_values_mut_by("core", None, "a")?;
        values.set_string_at(2, "Hello");
        assert_eq!(
            config.to_string(),
            "[core]\n    a = b\"100\"\n    [core]\n        a =d\n        a= Hello\n"
        );
        Ok(())
    }

    #[test]
    fn all() -> crate::Result {
        let mut config = init_config();
        let mut values = config.raw_values_mut_by("core", None, "a")?;
        values.set_all("Hello");
        assert_eq!(
            config.to_string(),
            "[core]\n    a = Hello\n    [core]\n        a= Hello\n        a =Hello\n"
        );
        Ok(())
    }

    #[test]
    fn all_empty() -> crate::Result {
        let mut config = init_config();
        let mut values = config.raw_values_mut_by("core", None, "a")?;
        values.set_all("");
        assert_eq!(
            config.to_string(),
            "[core]\n    a = \n    [core]\n        a= \n        a =\n"
        );
        Ok(())
    }
}

mod delete {
    use crate::file::mutable::multi_value::init_config;

    #[test]
    fn single_at_start_and_end() -> crate::Result {
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
    fn all() -> crate::Result {
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
