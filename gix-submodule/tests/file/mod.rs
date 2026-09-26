use crate::Result;

fn submodule(bytes: &str) -> gix_submodule::File {
    gix_submodule::File::from_bytes(bytes.as_bytes(), None, &Default::default()).expect("valid module")
}

mod is_active_platform {
    use crate::Result;
    use std::str::FromStr;

    fn module_file(name: &str) -> Result<gix_submodule::File> {
        let modules = gix_testtools::scripted_fixture_read_only("basic.sh")?
            .join(name)
            .join(".gitmodules");
        Ok(gix_submodule::File::from_bytes(
            std::fs::read(&modules)?.as_slice(),
            modules,
            &Default::default(),
        )?)
    }

    use bstr::{BStr, ByteSlice};

    fn multi_modules() -> Result<gix_submodule::File> {
        module_file("multiple")
    }

    fn assume_valid_active_state<'a>(
        module: &'a gix_submodule::File,
        config: &'a gix_config::File,
        defaults: gix_pathspec::Defaults,
    ) -> Result<Vec<(&'a str, bool)>> {
        assume_valid_active_state_with_attrs(module, config, defaults, |_, _, _, _| {
            unreachable!("shouldn't be called")
        })
    }

    fn assume_valid_active_state_with_attrs<'a>(
        module: &'a gix_submodule::File,
        config: &'a gix_config::File,
        defaults: gix_pathspec::Defaults,
        mut attributes: impl FnMut(
            &BStr,
            gix_pathspec::attributes::glob::pattern::Case,
            bool,
            &mut gix_pathspec::attributes::search::Outcome,
        ) -> bool
        + 'a,
    ) -> Result<Vec<(&'a str, bool)>> {
        let mut platform = module.is_active_platform(config, defaults)?;
        Ok(module
            .names()
            .map(|name| {
                (
                    name.to_str().expect("valid"),
                    platform.is_active(config, name, &mut attributes).expect("valid"),
                )
            })
            .collect())
    }

    #[test]
    fn without_submodule_in_index() -> Result {
        let module = module_file("not-a-submodule")?;
        assert_eq!(
            module.names().map(ToOwned::to_owned).collect::<Vec<_>>(),
            ["submodule"],
            "entries can be read"
        );
        Ok(())
    }

    #[test]
    fn without_any_additional_settings_all_are_inactive_if_they_have_a_url() -> Result {
        let module = multi_modules()?;
        assert_eq!(
            assume_valid_active_state(&module, &Default::default(), Default::default())?,
            &[
                ("submodule", false),
                ("a/b", false),
                (".a/..c", false),
                (r"a/d\", false),
                (r"a\e", false)
            ]
        );
        Ok(())
    }

    #[test]
    fn submodules_with_active_config_are_considered_active_or_inactive() -> crate::Result {
        let module = multi_modules()?;
        assert_eq!(
            assume_valid_active_state(
                &module,
                &gix_config::File::from_str(
                    "[submodule.submodule]\n active = 0\n url = set \n[submodule \"a/b\"]\n active = false \n url = set \n[submodule \".a/..c\"] active = 1"
                )?,
                Default::default()
            )?,
            &[
                ("submodule", false),
                ("a/b", false),
                (".a/..c", true),
                (r"a/d\", false),
                (r"a\e", false)
            ]
        );
        Ok(())
    }

    #[test]
    fn submodules_with_active_config_override_pathspecs() -> crate::Result {
        let module = multi_modules()?;
        assert_eq!(
            assume_valid_active_state(
                &module,
                &gix_config::File::from_str(
                    "[submodule.submodule]\n active = 0\n[submodule]\n active = *\n[submodule]\n active = :!a*"
                )?,
                Default::default()
            )?,
            &[
                ("submodule", false),
                ("a/b", false),
                (".a/..c", true),
                (r"a/d\", false),
                (r"a\e", false)
            ]
        );
        Ok(())
    }

    #[test]
    fn pathspecs_matter_even_if_they_do_not_match() -> crate::Result {
        let module = multi_modules()?;
        assert_eq!(
            assume_valid_active_state(
                &module,
                &gix_config::File::from_str("[submodule]\n active = submodule ")?,
                Default::default()
            )?,
            &[
                ("submodule", true),
                ("a/b", false),
                (".a/..c", false),
                (r"a/d\", false),
                (r"a\e", false)
            ]
        );
        assert_eq!(
            assume_valid_active_state(
                &module,
                &gix_config::File::from_str("[submodule]\n active = :!submodule ")?,
                Default::default()
            )?,
            &[
                ("submodule", false),
                ("a/b", true),
                (".a/..c", true),
                (r"a/d\", true),
                (r"a\e", true)
            ]
        );
        Ok(())
    }
}

mod path {

    use crate::file::submodule;

    fn submodule_path(value: &str) -> gix_error::Error {
        let module = submodule(&format!("[submodule.a]\npath = {value}"));
        module.path("a".into()).unwrap_err()
    }

    #[test]
    fn valid() -> crate::Result {
        let module = submodule("[submodule.a]\n path = relative/path/submodule");
        assert_eq!(module.path("a".into())?, "relative/path/submodule");
        Ok(())
    }

    #[test]
    fn validate_upon_retrieval() {
        let mut message_diagnostics = Vec::new();
        let absolute = submodule_path(if cfg!(windows) {
            r"c:\\hello"
        } else {
            r"/definitely/absolute\\"
        });
        insta::assert_debug_snapshot!(gix_testtools::redact_debug_snapshot(&absolute.error(), &[(if cfg!(windows) { r"c:\hello" } else { r"/definitely/absolute\" }, "<absolute-path>")]), "validate upon retrieval", @r#"
        Message {
            message: "The path of submodule 'a' needs to be relative",
            class: Validation,
            values: {"input": Bytes("<absolute-path>")},
        }
        "#);
        insta::assert_debug_snapshot!(submodule_path("").error(), "validate upon retrieval", @r#"
        Message {
            message: "The submodule 'a' was missing its 'path' field or it was empty",
            class: Validation,
        }
        "#);
        insta::assert_debug_snapshot!(submodule_path("../attack").error(), "validate upon retrieval", @r#"
        Message {
            message: "The path would lead outside of the repository worktree",
            class: Validation,
            values: {"input": Bytes("../attack")},
        }
        "#);

        {
            let module = submodule("[submodule.a]\n path");
            message_diagnostics.push(gix_testtools::redact_debug_snapshot(
                &(module.path("a".into()).expect_err("the input must be rejected")),
                &[],
            ));
        }

        {
            let module = submodule("[submodule.a]\n");
            message_diagnostics.push(gix_testtools::redact_debug_snapshot(
                &(module.path("a".into()).expect_err("the input must be rejected")),
                &[],
            ));
        }
        insta::assert_debug_snapshot!(message_diagnostics, "validate upon retrieval", @"
        [
            The submodule 'a' was missing its 'path' field or it was empty,
            The submodule 'a' was missing its 'path' field or it was empty,
        ]
        ");
    }
}

mod url {

    use crate::file::submodule;

    fn submodule_url(value: &str) -> gix_error::Error {
        let module = submodule(&format!("[submodule.a]\nurl = {value}"));
        module.url("a".into()).unwrap_err()
    }

    #[test]
    fn valid() -> crate::Result {
        let module = submodule("[submodule.a]\n url = path-to-repo");
        assert_eq!(module.url("a".into())?.to_bstring(), "path-to-repo");
        Ok(())
    }

    #[test]
    fn validate_upon_retrieval() {
        let mut message_diagnostics = Vec::new();
        insta::assert_debug_snapshot!(submodule_url(""), "validate upon retrieval", @"The submodule 'a' was missing its 'url' field or it was empty");
        {
            let module = submodule("[submodule.a]\n url");
            message_diagnostics.push(gix_testtools::redact_debug_snapshot(
                &(module.url("a".into()).expect_err("the input must be rejected").error()),
                &[],
            ));
        }

        {
            let module = submodule("[submodule.a]\n");
            message_diagnostics.push(gix_testtools::redact_debug_snapshot(
                &(module.url("a".into()).expect_err("the input must be rejected").error()),
                &[],
            ));
        }

        insta::assert_debug_snapshot!(submodule_url("file://"), "validate upon retrieval", @r#"
        The url of submodule 'a' could not be parsed, "input"="file://"
        |
        └─ URL does not specify a path to a repository, "input"="file://"
        "#);
        insta::assert_debug_snapshot!(message_diagnostics, "validate upon retrieval", @r#"
        [
            Message {
                message: "The submodule 'a' was missing its 'url' field or it was empty",
                class: Validation,
            },
            Message {
                message: "The submodule 'a' was missing its 'url' field or it was empty",
                class: Validation,
            },
        ]
        "#);
    }
}

mod update {
    use std::str::FromStr;

    use gix_submodule::config::Update;

    use crate::file::submodule;

    fn submodule_update(value: &str) -> gix_error::Error {
        let module = submodule(&format!("[submodule.a]\nupdate = {value}"));
        module.update("a".into()).unwrap_err()
    }

    #[test]
    fn default() {
        assert_eq!(Update::default(), Update::Checkout, "as defined in the docs");
    }

    #[test]
    fn valid() -> crate::Result {
        for (valid, expected) in [
            ("checkout", Update::Checkout),
            ("rebase", Update::Rebase),
            ("merge", Update::Merge),
            ("none", Update::None),
        ] {
            let module = submodule(&format!("[submodule.a]\n update = {valid}"));
            assert_eq!(module.update("a".into())?.expect("present"), expected);
        }
        Ok(())
    }

    #[test]
    fn valid_in_overrides() -> crate::Result {
        let mut module = submodule("[submodule.a]\n update = merge");
        let repo_config = gix_config::File::from_str("[submodule.a]\n update = !dangerous")?;
        let prev_names = module.names().map(ToOwned::to_owned).collect::<Vec<_>>();
        module
            .append_submodule_overrides(&repo_config)
            .expect("the fixture fits into the backing buffer");

        assert_eq!(
            module.update("a".into())?.expect("present"),
            Update::Command("dangerous".into()),
            "overridden values are picked up and make commands possible - these are local"
        );
        assert_eq!(
            module.names().map(ToOwned::to_owned).collect::<Vec<_>>(),
            prev_names,
            "Appending more configuration sections doesn't affect name listing"
        );
        Ok(())
    }

    #[test]
    fn validate_upon_retrieval() {
        insta::assert_debug_snapshot!(submodule_update("").error(), "validate upon retrieval", @r#"
        Message {
            message: "The 'update' field of submodule 'a' was invalid",
            class: Validation,
            values: {"input": Bytes("")},
        }
        "#);
        insta::assert_debug_snapshot!(submodule_update("bogus").error(), "validate upon retrieval", @r#"
        Message {
            message: "The 'update' field of submodule 'a' was invalid",
            class: Validation,
            values: {"input": Bytes("bogus")},
        }
        "#);
        insta::assert_debug_snapshot!(submodule_update("!dangerous").error(), "forbidden unless it's an override", @r#"
        Message {
            message: "The 'update' field of submodule 'a' tried to set a command to be shared",
            class: Validation,
            values: {"input": Bytes("dangerous")},
        }
        "#);
    }

    /// Reproducer for GHSA-f26g-jm89-4g65 and GHSA-97pq-9mjg-9fcj: `.gitmodules` may carry
    /// `submodule.<name>.update = !command`, while a same-named local section without the winning
    /// `update` value makes `File::update()` treat the command as trusted and expose it as
    /// `Update::Command`.
    #[test]
    fn modules_command_is_authorized_by_unrelated_same_named_override() -> crate::Result {
        let mut module = submodule("[submodule.a]\n update = !dangerous");
        let repo_config = gix_config::File::from_str("[submodule.a]\n url = trusted-local-override")?;
        module
            .append_submodule_overrides(&repo_config)
            .expect("the fixture fits into the backing buffer");

        let err = module.update("a".into()).expect_err("the shared command is invalid");
        let err = err
            .downcast_any_ref::<gix_error::Message>()
            .expect("the validation message is retained");
        assert_eq!(
            err.values.get("input"),
            Some(&gix_error::MetadataValue::from(b"dangerous".as_slice()))
        );
        insta::assert_debug_snapshot!(err, "a same-named local section must not authorize a command that still originates from .gitmodules", @r#"
        Message {
            message: "The 'update' field of submodule 'a' tried to set a command to be shared",
            class: Validation,
            values: {"input": Bytes("dangerous")},
        }
        "#);
        Ok(())
    }
}

mod fetch_recurse {
    use gix_submodule::config::FetchRecurse;

    use crate::file::submodule;

    #[test]
    fn default() {
        assert_eq!(
            FetchRecurse::default(),
            FetchRecurse::OnDemand,
            "as defined in git codebase actually"
        );
    }

    #[test]
    fn valid() -> crate::Result {
        for (valid, expected) in [
            ("yes", FetchRecurse::Always),
            ("true", FetchRecurse::Always),
            ("", FetchRecurse::Never),
            ("no", FetchRecurse::Never),
            ("false", FetchRecurse::Never),
            ("on-demand", FetchRecurse::OnDemand),
        ] {
            let module = submodule(&format!("[submodule.a]\n fetchRecurseSubmodules = {valid}"));
            assert_eq!(module.fetch_recurse("a".into())?.expect("present"), expected);
        }
        let module = submodule("[submodule.a]\n fetchRecurseSubmodules");
        assert_eq!(
            module.fetch_recurse("a".into())?.expect("present"),
            FetchRecurse::Always,
            "no value means true, which means to always recurse"
        );
        Ok(())
    }

    #[test]
    fn validate_upon_retrieval() -> crate::Result {
        for invalid in ["foo", "ney", "On-demand"] {
            let module = submodule(&format!("[submodule.a]\n fetchRecurseSubmodules = \"{invalid}\""));
            assert!(module.fetch_recurse("a".into()).is_err());
        }
        Ok(())
    }
}

mod ignore {
    use crate::Result;
    use gix_submodule::config::Ignore;

    use crate::file::submodule;

    #[test]
    fn default() {
        assert_eq!(Ignore::default(), Ignore::None, "as defined in the docs");
    }

    #[test]
    fn valid() -> Result {
        for (valid, expected) in [
            ("all", Ignore::All),
            ("dirty", Ignore::Dirty),
            ("untracked", Ignore::Untracked),
            ("none", Ignore::None),
        ] {
            let module = submodule(&format!("[submodule.a]\n ignore = {valid}"));
            assert_eq!(module.ignore("a".into())?.expect("present"), expected);
        }
        let module = submodule("[submodule.a]\n ignore");
        assert!(
            module.ignore("a".into())?.is_none(),
            "no value is interpreted as non-existing string, hence the caller will see None"
        );
        Ok(())
    }

    #[test]
    fn validate_upon_retrieval() -> Result {
        for invalid in ["All", ""] {
            let module = submodule(&format!("[submodule.a]\n ignore = \"{invalid}\""));
            assert!(module.ignore("a".into()).is_err());
        }
        Ok(())
    }
}

mod branch {
    use crate::Result;
    use gix_submodule::config::Branch;

    use crate::file::submodule;

    #[test]
    fn valid() -> Result {
        for (valid, expected) in [
            (".", Branch::CurrentInSuperproject),
            ("", Branch::Name("HEAD".into())),
            ("master", Branch::Name("master".into())),
            ("feature/a", Branch::Name("feature/a".into())),
            (
                "abcde12345abcde12345abcde12345abcde12345",
                Branch::Name("abcde12345abcde12345abcde12345abcde12345".into()),
            ),
        ] {
            let module = submodule(&format!("[submodule.a]\n branch = {valid}"));
            assert_eq!(module.branch("a".into())?.expect("present"), expected);
        }
        let module = submodule("[submodule.a]\n branch");
        assert!(
            module.branch("a".into())?.is_none(),
            "no value implies it's not set, but the caller will then default"
        );
        Ok(())
    }

    #[test]
    fn validate_upon_retrieval() -> Result {
        let module = submodule("[submodule.a]\n branch = /invalid");
        assert!(module.branch("a".into()).is_err());
        Ok(())
    }
}

#[test]
fn shallow() -> Result {
    let module = submodule("[submodule.a]\n shallow");
    assert_eq!(
        module.shallow("a".into())?,
        Some(true),
        "shallow is a simple boolean without anything special (yet)"
    );
    Ok(())
}

mod append_submodule_overrides {
    use crate::Result;
    use std::str::FromStr;

    use crate::file::submodule;

    #[test]
    fn last_of_multiple_values_wins() -> Result {
        let mut module = submodule("[submodule.a] url = from-module");
        let repo_config = gix_config::File::from_str(
            "[submodule.a]\n url = a\n url = b\n ignore = x\n [submodule.a]\n url = c\n[submodule.b] url = not-relevant",
        )?;
        module
            .append_submodule_overrides(&repo_config)
            .expect("the fixture fits into the backing buffer");
        Ok(())
    }
}

mod baseline;
