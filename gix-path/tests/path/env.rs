use std::path::Path;

#[test]
fn exe_invocation() {
    let actual = gix_path::env::exe_invocation();
    assert!(
        !actual.as_os_str().is_empty(),
        "it finds something as long as git is installed somewhere on the system (or a default location)"
    );
}

#[test]
fn shell() {
    assert!(
        Path::new(gix_path::env::shell()).exists(),
        "On CI and on Unix we expect a usable path to the shell that exists on disk"
    );
}

#[cfg(windows)]
mod windows_installations {
    #[test]
    fn scoop_without_junctions() -> gix_error::TestResult {
        discover(
            "SCOOP",
            "no-junction",
            "no-junction/apps/git/2.50.0/cmd/git.exe",
            false,
            false,
        )
    }

    #[test]
    fn chocolatey_tools_location() -> gix_error::TestResult {
        discover(
            "ChocolateyToolsLocation",
            "chocolatey",
            "chocolatey/git/cmd/git.exe",
            false,
            false,
        )
    }

    #[test]
    fn active_conda_environment() -> gix_error::TestResult {
        discover("CONDA_PREFIX", "conda", "conda/Library/bin/git.exe", false, false)
    }

    #[test]
    fn winget_user_links() -> gix_error::TestResult {
        discover(
            "LocalAppData",
            "winget",
            "winget/Microsoft/WinGet/Links/git.exe",
            false,
            false,
        )
    }

    #[test]
    fn winget_machine_links() -> gix_error::TestResult {
        discover("ProgramW6432", "machine", "machine/WinGet/Links/git.exe", false, false)
    }

    #[test]
    fn msys_install_root() -> gix_error::TestResult {
        discover("GIT_INSTALL_ROOT", "msys", "msys/usr/bin/git.exe", false, false)
    }

    #[test]
    fn default_user_installation() -> gix_error::TestResult {
        discover(
            "USERPROFILE",
            "user",
            "user/scoop/apps/git/current/cmd/git.exe",
            false,
            false,
        )
    }

    #[test]
    fn custom_installation() -> gix_error::TestResult {
        discover("SCOOP", "custom", "custom/apps/git/current/cmd/git.exe", false, false)
    }

    #[test]
    fn default_global_installation() -> gix_error::TestResult {
        discover(
            "ProgramData",
            "global",
            "global/scoop/apps/git/current/cmd/git.exe",
            false,
            false,
        )
    }

    #[test]
    fn custom_global_installation() -> gix_error::TestResult {
        discover(
            "SCOOP_GLOBAL",
            "custom",
            "custom/apps/git/current/cmd/git.exe",
            false,
            false,
        )
    }

    #[test]
    fn git_install_root() -> gix_error::TestResult {
        discover("GIT_INSTALL_ROOT", "explicit", "explicit/cmd/git.exe", false, false)
    }

    #[test]
    fn missing_path() -> gix_error::TestResult {
        discover("SCOOP", "custom", "custom/apps/git/current/cmd/git.exe", false, true)
    }

    #[test]
    fn path_takes_precedence() -> gix_error::TestResult {
        discover("SCOOP", "custom", "git.exe", true, false)
    }

    /// Assert Git discovery in an isolated child process with host installation variables cleared.
    ///
    /// Set `variable` to the fixture-relative `root` to select the installation to discover.
    /// Compare the invocation directly with `expected` when `on_path` is true; otherwise,
    /// resolve `expected` relative to the fixture. Unless `unset_path` is true, set `PATH`
    /// to the fixture's Git directory when `on_path` is true, or to an empty string otherwise.
    fn discover(variable: &str, root: &str, expected: &str, on_path: bool, unset_path: bool) -> gix_error::TestResult {
        // Fixture creation can invoke Git and initialize discovery caches. Complete it in the
        // parent so the isolated child only reads the fixture before setting its environment.
        let fixture = std::path::absolute(gix_testtools::scripted_fixture_read_only(
            "windows-git-installations.sh",
        )?)?;
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let mut env = gix_testtools::Env::new();
        for variable in [
            "ProgramFiles",
            "ProgramFiles(x86)",
            "ProgramW6432",
            "LocalAppData",
            "SCOOP",
            "SCOOP_GLOBAL",
            "GIT_INSTALL_ROOT",
            "USERPROFILE",
            "ProgramData",
            "ChocolateyToolsLocation",
            "CONDA_PREFIX",
            "PATH",
        ] {
            env = env.unset(variable);
        }
        env = env.set(variable, fixture.join(root).to_str().expect("UTF-8 fixture path"));
        if on_path {
            for (variable, root) in [
                ("CONDA_PREFIX", "conda"),
                ("ChocolateyToolsLocation", "chocolatey"),
                ("LocalAppData", "winget"),
                ("GIT_INSTALL_ROOT", "msys"),
            ] {
                env = env.set(variable, fixture.join(root).to_str().expect("UTF-8 fixture path"));
            }
        }
        if !unset_path {
            env = env.set(
                "PATH",
                if on_path {
                    fixture.join("path").to_str().expect("UTF-8 fixture path").to_owned()
                } else {
                    String::new()
                },
            );
        }
        let expected = if on_path {
            expected.into()
        } else {
            fixture.join(expected)
        };
        assert_eq!(
            gix_path::env::exe_invocation(),
            expected,
            "Package installations are fallbacks when Git is absent from PATH, including when PATH is unset"
        );
        drop(env);
        Ok(())
    }
}

#[test]
fn shell_absolute() {
    assert!(
        Path::new(gix_path::env::shell()).is_absolute(),
        "On CI and on Unix we currently expect the path to the shell always to be absolute"
    );
}

#[test]
fn shell_unix_path() {
    let shell = gix_path::env::shell()
        .to_str()
        .expect("This test depends on the shell path being valid Unicode");
    assert!(
        !shell.contains('\\'),
        "The path to the shell should have no backslashes, barring strange `GIT_EXEC_PATH` values"
    );
}

#[test]
fn installation_config() {
    assert_ne!(
        gix_path::env::installation_config().map(|p| p.components().count()),
        gix_path::env::installation_config_prefix().map(|p| p.components().count()),
        "the prefix is a bit shorter than the installation config path itself"
    );
}

#[test]
fn core_dir() {
    assert!(
        gix_path::env::core_dir()
            .expect("Git is always in PATH when we run tests")
            .is_dir(),
        "The core directory is a valid directory"
    );
}

#[test]
fn core_dir_program() -> gix_testtools::TestResult {
    let core_dir = gix_path::env::core_dir().expect("Git is always in PATH when we run tests");
    let programs = std::fs::read_dir(core_dir)?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|entry| {
            let name = entry.file_name();
            let stem = name.to_str()?.strip_suffix(std::env::consts::EXE_SUFFIX)?;
            stem.starts_with("git-").then(|| stem.to_owned())
        });
    // Which programs are present as separate executables depends on how Git was built -
    // Git for Windows, for instance, may not provide programs for builtin subcommands.
    for stem in programs {
        let path =
            gix_path::env::core_dir_program(&stem).expect("a program listed in the core directory is found there");
        assert!(path.is_file(), "the returned path refers to an existing file");
        assert!(
            path.is_absolute(),
            "the path is absolute as it is based on `git --exec-path`"
        );
    }
    assert_eq!(
        gix_path::env::core_dir_program("git-program-that-does-not-exist"),
        None,
        "programs that don't exist in the core directory are not found"
    );

    for name in ["", ".", "..", "../git", "git/program", "/git"] {
        assert_eq!(
            gix_path::env::core_dir_program(name),
            None,
            "program names with path components are rejected"
        );
        assert_eq!(
            gix_path::env::installation_program(name),
            None,
            "installation program names with path components are rejected"
        );
    }

    #[cfg(windows)]
    assert_eq!(
        gix_path::env::core_dir_program(r"git\program"),
        None,
        "Windows path separators in program names are rejected"
    );
    #[cfg(windows)]
    assert_eq!(
        gix_path::env::installation_program(r"git\program"),
        None,
        "Windows path separators in installation program names are rejected"
    );
    Ok(())
}

#[test]
#[cfg_attr(
    not(windows),
    ignore = "Git's bundled bin directories are specific to Git for Windows"
)]
fn installation_program_finds_programs_outside_the_core_directory() {
    let vim =
        gix_path::env::installation_program("vim").expect("Git for Windows bundles vim outside its core directory");
    assert!(vim.is_absolute(), "the bundled program path is absolute");
    assert!(vim.is_file(), "the bundled program exists");
    assert_ne!(
        vim.parent(),
        gix_path::env::core_dir(),
        "vim is found in a bundled bin directory, not the Git core directory"
    );
}

#[test]
fn system_prefix() {
    assert_ne!(
        gix_path::env::system_prefix(),
        None,
        "git should be present when running tests"
    );
}

#[cfg(windows)]
mod windows_prefix {
    #[test]
    fn mixed_installation_keeps_mingw64_active() -> gix_testtools::TestResult {
        Ok(mixed_installation("mingw64")?)
    }

    #[test]
    fn mixed_installation_keeps_ucrt64_active() -> gix_testtools::TestResult {
        Ok(mixed_installation("ucrt64")?)
    }

    #[test]
    fn shortcut_uses_the_same_installation_and_system_config() -> gix_testtools::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let installation = tempfile::tempdir()?;
        std::fs::create_dir(installation.path().join("ucrt64"))?;
        let _env =
            gix_testtools::Env::new().set("EXEPATH", installation.path().to_str().expect("UTF-8 temporary path"));
        let config = installation.path().join("etc/gitconfig");
        assert_eq!(
            gix_path::env::installation_config(),
            Some(config.as_path()),
            "Git for Windows keeps its installation config in the top-level etc directory"
        );
        assert_eq!(
            gix_path::env::system_config(),
            Some(config.as_path()),
            "the EXEPATH shortcut must report that same file as the system config"
        );
        assert!(
            gix_path::env::installation_config_is_system(),
            "the installation file has system scope"
        );
        Ok(())
    }

    fn mixed_installation(runtime: &str) -> gix_testtools::Result {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let installation = tempfile::tempdir()?;
        for directory in ["mingw64", "ucrt64"] {
            std::fs::create_dir(installation.path().join(directory))?;
        }
        let prefix = installation.path().join(runtime);
        // Let the real Git report the selected runtime without copying an installation.
        // Both directories exist, so EXEPATH must fall back to this --exec-path result.
        let _env = gix_testtools::Env::new()
            .set("EXEPATH", installation.path().to_str().expect("UTF-8 temporary path"))
            .set(
                "GIT_EXEC_PATH",
                prefix.join("libexec/git-core").to_str().expect("UTF-8 temporary path"),
            );
        assert_eq!(
            gix_path::env::system_prefix(),
            Some(prefix.as_path()),
            "an extra runtime directory must not change the prefix selected by Git"
        );
        Ok(())
    }
}

#[test]
fn home_dir() {
    assert_ne!(
        gix_path::env::home_dir(),
        None,
        "we find a home on every system these tests execute"
    );
}

mod xdg_config {
    use std::ffi::OsStr;

    #[test]
    fn prefers_xdg_config_bases() {
        let actual = gix_path::env::xdg_config("test", &mut |n| {
            (n == OsStr::new("XDG_CONFIG_HOME")).then(|| "marker".into())
        })
        .expect("set");
        #[cfg(unix)]
        assert_eq!(actual.to_str(), Some("marker/git/test"));
        #[cfg(windows)]
        assert_eq!(actual.to_str(), Some(r"marker\git\test"));
    }

    #[test]
    fn falls_back_to_home() {
        let actual = gix_path::env::xdg_config("test", &mut |n| (n == OsStr::new("HOME")).then(|| "marker".into()))
            .expect("set");
        #[cfg(unix)]
        assert_eq!(actual.to_str(), Some("marker/.config/git/test"));
        #[cfg(windows)]
        assert_eq!(actual.to_str(), Some(r"marker\.config\git\test"));
    }
}
