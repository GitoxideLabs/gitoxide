use std::path::{Path, PathBuf};

fn fixture() -> gix_error::TestResult<PathBuf> {
    Ok(gix_testtools::scripted_fixture_read_only("unix-git-layouts.sh")?.canonicalize()?)
}

fn select(path: impl AsRef<std::ffi::OsStr>) -> gix_testtools::Env<'static> {
    gix_testtools::Env::new().set("PATH", path.as_ref().to_str().expect("UTF-8 fixture path"))
}

fn assert_inferred(root: &Path) {
    assert_eq!(
        gix_path::env::installation_config(),
        Some(root.join("etc/gitconfig").as_path()),
        "Homebrew's configuration belongs to its prefix, not the versioned Cellar directory"
    );
    assert!(
        gix_path::env::installation_config_is_system(),
        "Homebrew's installation configuration has system scope"
    );
}

fn assert_system_default() {
    let config = Path::new("/etc/gitconfig");
    assert_eq!(
        gix_path::env::installation_config(),
        config.is_file().then_some(config),
        "unknown or absent installation configuration retains the native system default"
    );
    assert_eq!(
        gix_path::env::installation_config_is_system(),
        config.is_file(),
        "an existing default configuration has system scope"
    );
}

#[test]
fn homebrew_cellar_without_spawning() -> gix_error::TestResult {
    let root = fixture()?.join("homebrew");
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(root.join("Cellar/git/2.56.0/bin"));
    assert_inferred(&root);
    Ok(())
}

#[test]
fn homebrew_symlink_chain_without_spawning() -> gix_error::TestResult {
    let root = fixture()?.join("homebrew");
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(root.join("bin"));
    assert_inferred(&root);
    Ok(())
}

#[test]
fn homebrew_discovery_ignores_config_overrides() -> gix_error::TestResult {
    let root = fixture()?.join("homebrew");
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(root.join("bin"))
        .set("GIT_CONFIG_SYSTEM", "/overridden/gitconfig")
        .set("GIT_CONFIG_NOSYSTEM", "1");
    assert_inferred(&root);
    Ok(())
}

#[test]
fn first_unknown_executable_retains_system_default() -> gix_error::TestResult {
    let root = fixture()?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(std::env::join_paths([root.join("unknown"), root.join("homebrew/bin")])?);
    assert_system_default();
    Ok(())
}

#[test]
fn missing_homebrew_config_retains_system_default() -> gix_error::TestResult {
    let root = fixture()?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(root.join("incomplete/Cellar/git/2.56.0/bin"));
    assert_system_default();
    Ok(())
}

#[test]
fn arbitrary_prefix_with_config_is_inferred() -> gix_error::TestResult {
    let root = fixture()?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(root.join("custom/bin"));
    assert_inferred(&root.join("custom"));
    Ok(())
}

#[test]
fn broken_symlinks_and_non_executable_entries_are_skipped() -> gix_error::TestResult {
    let root = fixture()?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(std::env::join_paths([
        root.join("broken"),
        root.join("non-executable"),
        root.join("homebrew/bin"),
    ])?);
    assert_inferred(&root.join("homebrew"));
    Ok(())
}

#[test]
fn restricted_execute_permissions_do_not_require_execution() -> gix_error::TestResult {
    let root = fixture()?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(std::env::join_paths([
        root.join("restricted/Cellar/git/2.56.0/bin"),
        root.join("unknown"),
    ])?);
    assert_inferred(&root.join("restricted"));
    Ok(())
}

#[test]
fn relative_path_entry_is_resolved_without_execution() -> gix_error::TestResult {
    let root = fixture()?;
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let _env = select(std::env::join_paths([
        PathBuf::from("gix-test-relative-path"),
        root.join("homebrew/bin"),
    ])?);
    assert_inferred(&root.join("homebrew"));
    Ok(())
}

#[cfg(target_os = "macos")]
mod apple {
    use super::{fixture, select};
    use std::path::Path;

    fn assert_inferred(developer: &Path) {
        assert_eq!(
            gix_path::env::installation_config(),
            Some(developer.join("usr/share/git-core/gitconfig").as_path()),
            "Apple Git's installation config belongs to the selected developer directory"
        );
        assert!(
            !gix_path::env::installation_config_is_system(),
            "Apple Git's installation config has unknown scope"
        );
    }

    #[test]
    fn direct_xcode_without_spawning() -> gix_error::TestResult {
        direct("Xcode.app/Contents/Developer")
    }

    #[test]
    fn direct_command_line_tools_without_spawning() -> gix_error::TestResult {
        direct("Library/Developer/CommandLineTools")
    }

    fn direct(directory: &str) -> gix_error::TestResult {
        let developer = fixture()?.join(directory);
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = select(developer.join("usr/bin"));
        assert_inferred(&developer);
        Ok(())
    }

    #[test]
    fn symlink_to_developer_git_is_inferred() -> gix_error::TestResult {
        let root = fixture()?;
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = select(root.join("apple-link"));
        assert_inferred(&root.join("Xcode.app/Contents/Developer"));
        Ok(())
    }

    #[test]
    fn symlink_to_installed_apple_git_uses_its_target() -> gix_error::TestResult {
        let root = fixture()?;
        if gix_testtools::run_in_isolated_process()? || !root.join("real-apple-link/git").is_file() {
            return Ok(());
        }
        let _env = select(root.join("real-apple-link"))
            .unset("DEVELOPER_DIR")
            .unset("SDKROOT")
            .unset("TOOLCHAINS");
        assert_inferred(Path::new("/Library/Developer/CommandLineTools"));
        Ok(())
    }

    #[test]
    fn shim_with_developer_symlink_uses_selected_directory() -> gix_error::TestResult {
        let root = fixture()?;
        if gix_testtools::run_in_isolated_process()? || !root.join("real-developer-link/usr/bin/git").is_file() {
            return Ok(());
        }
        let _env = select("/usr/bin").unset("SDKROOT").unset("TOOLCHAINS").set(
            "DEVELOPER_DIR",
            root.join("real-developer-link").to_str().expect("UTF-8 fixture path"),
        );
        assert_inferred(&root.join("real-developer-link"));
        Ok(())
    }

    #[test]
    fn shim_with_developer_directory_is_inferred() -> gix_error::TestResult {
        let root = fixture()?;
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = select("/usr/bin").unset("SDKROOT").unset("TOOLCHAINS").set(
            "DEVELOPER_DIR",
            root.join("Xcode.app/Contents/Developer")
                .to_str()
                .expect("UTF-8 fixture path"),
        );
        assert_inferred(&root.join("Xcode.app/Contents/Developer"));
        Ok(())
    }
}
