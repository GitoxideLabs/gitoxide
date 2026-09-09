mod isolation {
    use gix_testtools::{Creation, Result};

    #[test]
    fn script_configuration_adds_to_the_isolation() -> Result {
        let dir = gix_testtools::scripted_fixture_writable_with_args(
            "make_config_isolation.sh",
            None::<String>,
            Creation::Execute,
        )?;

        assert_eq!(
            std::fs::read_to_string(dir.path().join("maintenance-auto"))?.trim(),
            "false",
            "the isolation survives a script setting GIT_CONFIG_COUNT for itself"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("head"))?.trim(),
            "refs/heads/main",
            "isolation takes precedence over GIT_CONFIG_COUNT for shared keys"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("custom"))?.trim(),
            "present",
            "the script's own configuration still applies"
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("override-head"))?.trim(),
            "refs/heads/other",
            "explicit git -c options can override isolation"
        );
        assert_eq!(
            gix_testtools::git(dir.path().join("repo"), "config --get maintenance.auto")?.trim(),
            "false",
            "`git()` runs with the same isolation"
        );
        Ok(())
    }
}
mod repository;
mod rust_fixture;
mod scripted_fixture_with_post;
