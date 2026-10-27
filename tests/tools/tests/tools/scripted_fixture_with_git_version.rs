use gix_testtools::TestResult;

#[test]
fn version_gate_checks_the_git_used_inside_bash() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    // An exported Bash function models Git Bash resolving a different Git than a direct command.
    let _environment =
        gix_testtools::Env::new().set("BASH_FUNC_git%%", "() { printf '%s\\n' 'git version 1.2.3.test'; }");
    let (major, minor, patch) = *gix_testtools::GIT_BASH_VERSION;
    assert_ne!(
        (major, minor, patch),
        *gix_testtools::GIT_VERSION,
        "the regression must distinguish Bash's Git from the direct Git command"
    );
    let mut checked_version = None;
    let fixture = gix_testtools::scripted_fixture_read_only_with_args_with_git_version(
        "make_git_version.sh",
        [format!("{major}.{minor}.{patch}")],
        |version| {
            checked_version = Some(version);
            true
        },
    )?
    .expect("compatible Git allows fixture generation");
    let (major, minor, patch) = checked_version.expect("the version gate is evaluated");
    assert_eq!(
        (major, minor, patch),
        *gix_testtools::GIT_BASH_VERSION,
        "fixture gates expose the Git version resolved by Bash"
    );
    let actual = std::fs::read_to_string(fixture.join("git-version"))?;
    let suffix = actual
        .strip_prefix(&format!("git version {major}.{minor}.{patch}"))
        .expect("the script uses the Git version inspected by the gate");
    assert!(
        suffix.starts_with(['.', ' ', '\n', '\r']),
        "vendor suffixes and line endings follow the complete patch version"
    );
    Ok(())
}

#[test]
fn incompatible_read_only_fixtures_without_an_archive_are_unavailable() -> TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    assert!(
        gix_testtools::scripted_fixture_read_only_with_args_with_git_version(
            "make_git_version.sh",
            ["incompatible-without-archive"],
            |_| false,
        )?
        .is_none(),
        "incompatible Git never runs the fixture script when its archive is missing"
    );
    Ok(())
}
