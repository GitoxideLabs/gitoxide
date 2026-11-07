use std::path::{Path, PathBuf};

fn fixture() -> gix_error::TestResult<PathBuf> {
    Ok(std::path::absolute(gix_testtools::scripted_fixture_read_only(
        "windows-git-layouts.sh",
    )?)?)
}

fn select(bin: &Path) -> gix_testtools::Env<'static> {
    gix_testtools::Env::new()
        .unset("EXEPATH")
        .unset("GIT_EXEC_PATH")
        .set("PATH", bin.to_str().expect("UTF-8 fixture path"))
}

fn assert_native_paths(root: &Path, runtime: &str) {
    assert_eq!(
        gix_path::env::core_dir(),
        Some(root.join(runtime).join("libexec/git-core").as_path()),
        "the installed core directory is found without launching Git"
    );
    assert_eq!(
        gix_path::env::exe_invocation(),
        Path::new("git.exe"),
        "the selected PATH entry retains its short invocation"
    );
    assert_eq!(
        gix_path::env::system_prefix(),
        Some(root.join(runtime).as_path()),
        "the selected runtime determines the prefix without launching the empty executable"
    );
    assert_eq!(
        gix_path::env::installation_config(),
        Some(root.join("etc/gitconfig").as_path()),
        "the modern installation configuration is found without launching Git"
    );
    assert_eq!(
        gix_path::env::system_config(),
        gix_path::env::installation_config(),
        "modern Git for Windows has one system configuration file"
    );
    assert!(
        gix_path::env::installation_config_is_system(),
        "the file has system scope"
    );
}

#[test]
fn runtime_bin_selects_its_own_prefix_in_a_mixed_installation() -> gix_error::TestResult {
    let root = fixture()?.join("mixed");
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(&root.join("ucrt64/bin"));
    assert_native_paths(&root, "ucrt64");
    Ok(())
}

#[test]
fn chocolatey_portable_bin_launcher() -> gix_error::TestResult {
    bin_launcher("chocolatey/git")
}

#[test]
fn conda_and_mamba_library_bin_launcher() -> gix_error::TestResult {
    bin_launcher("conda/Library")
}

fn bin_launcher(directory: &str) -> gix_error::TestResult {
    let root = fixture()?.join(directory);
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(&root.join("bin"));
    assert_native_paths(&root, "ucrt64");
    Ok(())
}

#[test]
fn core_directory_uses_the_selected_executable_instead_of_exepath() -> gix_error::TestResult {
    let fixture = fixture()?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(&fixture.join("mixed/ucrt64/bin")).set(
        "EXEPATH",
        fixture.join("conda/Library").to_str().expect("UTF-8 fixture path"),
    );
    assert_eq!(
        gix_path::env::core_dir(),
        Some(fixture.join("mixed/ucrt64/libexec/git-core").as_path()),
        "EXEPATH may describe a different Git installation"
    );
    Ok(())
}

#[test]
fn core_directory_override_requires_querying_git() -> gix_error::TestResult {
    let fixture = fixture()?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(&fixture.join("mixed/ucrt64/bin")).set("GIT_EXEC_PATH", "custom/git-core");
    assert_eq!(
        gix_path::env::core_dir(),
        None,
        "an override disables inference, and the placeholder executable cannot answer the query"
    );
    Ok(())
}

#[test]
fn incomplete_core_directory_requires_querying_git() -> gix_error::TestResult {
    unrecognized_core("incomplete/ucrt64/bin")
}

#[test]
fn unknown_core_directory_requires_querying_git() -> gix_error::TestResult {
    unrecognized_core("unknown")
}

fn unrecognized_core(bin: &str) -> gix_error::TestResult {
    let fixture = fixture()?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(&fixture.join(bin));
    assert_eq!(
        gix_path::env::core_dir(),
        None,
        "without an established core directory, discovery must query the placeholder Git and fail"
    );
    Ok(())
}

#[test]
fn winget_user_link() -> gix_error::TestResult {
    linked_installation("winget/Microsoft/WinGet/Links")
}

#[test]
fn winget_machine_link_chain() -> gix_error::TestResult {
    linked_installation("winget-machine/WinGet/Links")
}

fn linked_installation(bin: &str) -> gix_error::TestResult {
    let fixture = fixture()?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(&fixture.join(bin));
    assert_native_paths(&fixture.join("conda/Library"), "ucrt64");
    Ok(())
}

#[test]
fn symlink_location_does_not_override_unknown_target_layout() -> gix_error::TestResult {
    unrecognized_core("symlinked/ucrt64/bin")
}

#[test]
fn scoop_current_shim() -> gix_error::TestResult {
    scoop_shim("scoop", "apps/git/current")
}

#[test]
fn scoop_shim_without_junctions() -> gix_error::TestResult {
    scoop_shim("no-junction", "apps/git/2.56.0")
}

fn scoop_shim(directory: &str, installation: &str) -> gix_error::TestResult {
    let fixture = std::path::absolute(gix_testtools::scripted_fixture_read_only("windows-scoop-shims.sh")?)?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let root = fixture.join(directory);
    let _env = select(&root.join("shims")).set("GIT_INSTALL_ROOT", fixture.to_str().expect("UTF-8 fixture path"));
    assert_native_paths(&root.join(installation), "ucrt64");
    Ok(())
}

#[test]
fn scoop_shim_with_arguments_requires_querying_git() -> gix_error::TestResult {
    let fixture = std::path::absolute(gix_testtools::scripted_fixture_read_only("windows-scoop-shims.sh")?)?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(&fixture.join("args/shims"));
    assert_eq!(
        gix_path::env::core_dir(),
        None,
        "arguments can change Git's behavior, so the placeholder shim must be queried"
    );
    assert_eq!(
        gix_path::env::installation_config(),
        None,
        "a shim with arguments cannot establish the target's configuration layout"
    );
    Ok(())
}
