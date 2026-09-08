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
        "refs/heads/other",
        "the script's own configuration still applies"
    );
    assert_eq!(
        gix_testtools::git(dir.path().join("repo"), "config --get maintenance.auto")?.trim(),
        "false",
        "`git()` runs with the same isolation"
    );
    let repo = dir.path().join("repo");
    assert!(
        gix_testtools::git(&repo, "config --global leak.test yes").is_err(),
        "writing global configuration from a fixture fails instead of leaking into other fixtures"
    );
    assert!(
        gix_testtools::git(&repo, "config --get leak.test").is_err(),
        "the isolation is restored right away"
    );
    Ok(())
}
