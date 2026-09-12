mod apply_environment {
    use std::path::{Path, PathBuf};

    use gix_prompt::{Mode, Options};
    use gix_testtools::Env;
    use serial_test::serial;

    #[test]
    #[serial]
    fn git_askpass_overrides_everything_and_ssh_askpass_does_not() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = Env::new()
            .set("GIT_ASKPASS", "override")
            .set("SSH_ASKPASS", "does not matter");

        assert_eq!(
            Options {
                askpass: Some(PathBuf::from("current")),
                ..Default::default()
            }
            .apply_environment(true, true, false)
            .askpass
            .expect("set"),
            Path::new("override")
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn git_askpass_is_used_first_and_sets_unset_askpass_values() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = Env::new()
            .set("GIT_ASKPASS", "from-env")
            .set("SSH_ASKPASS", "does not matter");

        assert_eq!(
            Options::default()
                .apply_environment(true, true, false)
                .askpass
                .expect("set"),
            Path::new("from-env")
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn ssh_askpass_is_used_as_fallback() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = Env::new().unset("GIT_ASKPASS").set("SSH_ASKPASS", "fallback");

        assert_eq!(
            Options {
                mode: Mode::Visible,
                ..Default::default()
            }
            .apply_environment(true, true, false)
            .askpass
            .expect("set"),
            Path::new("fallback")
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn ssh_askpass_does_not_override_current_value() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = Env::new().unset("GIT_ASKPASS").set("SSH_ASKPASS", "fallback");

        assert_eq!(
            Options {
                askpass: Some(PathBuf::from("current")),
                ..Default::default()
            }
            .apply_environment(true, true, false)
            .askpass
            .expect("set"),
            Path::new("current")
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn mode_is_left_untouched_if_terminal_prompt_is_trueish() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = Env::new().set("GIT_TERMINAL_PROMPT", "true");

        assert_eq!(
            Options {
                mode: Mode::Hidden,
                ..Default::default()
            }
            .apply_environment(false, false, true)
            .mode,
            Mode::Hidden
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn mode_is_disabled_if_terminal_prompt_is_falseish() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = Env::new().set("GIT_TERMINAL_PROMPT", "0");

        assert_eq!(
            Options {
                mode: Mode::Hidden,
                ..Default::default()
            }
            .apply_environment(false, false, true)
            .mode,
            Mode::Disable
        );
        Ok(())
    }

    #[test]
    #[serial]
    fn mode_is_unchanged_if_git_terminal_prompt_is_not_set() -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = Env::new().unset("GIT_TERMINAL_PROMPT");
        assert_eq!(
            Options {
                mode: Mode::Hidden,
                ..Default::default()
            }
            .apply_environment(false, false, true)
            .mode,
            Mode::Hidden
        );
        Ok(())
    }
}
