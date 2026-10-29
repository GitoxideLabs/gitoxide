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
    fn path_takes_precedence() -> gix_error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("windows-git-installations.sh")?;
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let mut env = gix_testtools::Env::new().set("PATH", fixture.join("path").to_str().expect("UTF-8 fixture path"));
        for (variable, root) in [
            ("SCOOP", "custom"),
            ("CONDA_PREFIX", "conda"),
            ("ChocolateyToolsLocation", "chocolatey"),
            ("LocalAppData", "winget"),
            ("GIT_INSTALL_ROOT", "msys"),
        ] {
            env = env.set(variable, fixture.join(root).to_str().expect("UTF-8 fixture path"));
        }
        assert_eq!(
            gix_path::env::exe_invocation(),
            std::path::Path::new("git.exe"),
            "PATH takes precedence over package roots and host registrations"
        );
        drop(env);
        Ok(())
    }

    #[test]
    fn path_inside_a_package_installation_keeps_the_short_invocation() -> gix_error::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("windows-git-installations.sh")?;
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let root = fixture.join("conda");
        let bin = root.join("Library/bin");
        let _env = gix_testtools::Env::new()
            .set("PATH", bin.to_str().expect("UTF-8 fixture path"))
            .set("CONDA_PREFIX", root.to_str().expect("UTF-8 fixture path"));
        assert_eq!(
            gix_path::env::exe_invocation(),
            std::path::Path::new("git.exe"),
            "PATH provenance determines the invocation without searching alternative locations"
        );
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
fn installation_config() -> gix_error::TestResult {
    if gix_testtools::run_in_isolated_process()? {
        return Ok(());
    }
    let fixture = gix_testtools::scripted_fixture_read_only("make_native_installation.sh")?.canonicalize()?;
    let (bin, config) = if cfg!(windows) {
        ("windows-ucrt/prefix/cmd", "windows-ucrt/prefix/etc/gitconfig")
    } else {
        ("unix-lib/prefix/bin", "unix-lib/prefix/etc/gitconfig")
    };
    let path = fixture.join(bin);
    let _env = gix_testtools::Env::new()
        .set("PATH", path.to_str().expect("UTF-8 fixture path"))
        .set("EXEPATH", "")
        .set("PROGRAMDATA", "")
        .set("GIT_EXEC_PATH", "unrelated-helper-directory");
    let config = fixture.join(config);
    assert_eq!(gix_path::env::installation_config(), Some(config.as_path()));
    assert_eq!(gix_path::env::installation_config_prefix(), config.parent());
    assert_eq!(gix_path::env::system_config(), Some(config.as_path()));
    assert!(gix_path::env::installation_config_is_system());
    Ok(())
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
            "native installation discovery produces absolute paths"
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
        std::fs::create_dir(config.parent().expect("config has a parent"))?;
        std::fs::File::create_new(&config)?;
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

    #[test]
    fn git_in_cmd_directory_is_located_without_running_it() -> gix_testtools::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let installation = tempfile::tempdir()?;
        let cmd = installation.path().join("cmd");
        std::fs::create_dir(&cmd)?;
        std::fs::create_dir(installation.path().join("ucrt64"))?;
        // An empty file fails to launch, so any result below can't have come from running it.
        std::fs::File::create_new(cmd.join("git.exe"))?;
        let _env = gix_testtools::Env::new()
            .unset("EXEPATH")
            .unset("GIT_EXEC_PATH")
            .set("PATH", cmd.to_str().expect("UTF-8 temporary path"));
        let config = installation.path().join("etc/gitconfig");
        std::fs::create_dir(config.parent().expect("config has a parent"))?;
        std::fs::File::create_new(&config)?;
        assert_eq!(
            gix_path::env::system_prefix(),
            Some(installation.path().join("ucrt64").as_path()),
            "the only runtime directory next to `cmd` is the prefix"
        );
        assert_eq!(
            gix_path::env::installation_config(),
            Some(config.as_path()),
            "Git for Windows keeps its installation config in the top-level etc directory"
        );
        assert_eq!(
            gix_path::env::system_config(),
            Some(config.as_path()),
            "the same file is the system config"
        );
        assert!(
            gix_path::env::installation_config_is_system(),
            "the installation file has system scope"
        );
        Ok(())
    }

    #[test]
    fn git_in_cmd_directory_without_modern_config_has_no_installation_config() -> gix_testtools::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let installation = tempfile::tempdir()?;
        let cmd = installation.path().join("cmd");
        std::fs::create_dir(&cmd)?;
        std::fs::create_dir_all(installation.path().join("mingw64/etc"))?;
        std::fs::write(
            installation.path().join("mingw64/etc/gitconfig"),
            "[core]
                autocrlf = true
            ",
        )?;
        // A legacy runtime directory does not establish the modern configuration layout.
        std::fs::File::create_new(cmd.join("git.exe"))?;
        let _env = gix_testtools::Env::new()
            .unset("EXEPATH")
            .unset("PROGRAMDATA")
            .unset("GIT_EXEC_PATH")
            .set("PATH", cmd.to_str().expect("UTF-8 temporary path"));
        assert_eq!(
            gix_path::env::installation_config(),
            None,
            "installation config requires a recognized configuration file"
        );
        assert!(
            !gix_path::env::installation_config_is_system(),
            "an absent installation config has no scope"
        );
        Ok(())
    }

    #[test]
    fn prefix_is_the_one_git_reports() -> gix_testtools::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("windows-git-installations.sh")?;
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let _env = gix_testtools::Env::new().unset("EXEPATH").unset("GIT_EXEC_PATH");
        let spelled_alike = |path: &std::path::Path| path.to_string_lossy().to_lowercase().replace('/', "\\");
        let reported = gix_testtools::git(&fixture, "--exec-path")?;
        let core_dir = std::path::Path::new(reported.trim_end());
        assert_eq!(
            spelled_alike(gix_path::env::core_dir().expect("Git's core directory is found")),
            spelled_alike(core_dir),
            "inferred core directories must agree with an actual Git query"
        );
        let from_git = core_dir
            .ancestors()
            .nth(2)
            .expect("Git for Windows reports `<prefix>/libexec/git-core`");
        let prefix = gix_path::env::system_prefix().expect("a Git installation is present");
        assert_eq!(
            spelled_alike(prefix),
            spelled_alike(from_git),
            "however it is found, the prefix must be the one Git uses"
        );
        Ok(())
    }

    #[test]
    fn git_in_cmd_directory_does_not_override_git_exec_path() -> gix_testtools::TestResult {
        if gix_testtools::run_in_isolated_process()? {
            return Ok(());
        }
        let installation = tempfile::tempdir()?;
        let exec_path_root = tempfile::tempdir()?;
        for directory in ["cmd", "ucrt64"] {
            std::fs::create_dir(installation.path().join(directory))?;
        }
        std::fs::File::create_new(installation.path().join("cmd/git.exe"))?;
        let prefix = exec_path_root.path().join("mingw64");
        let _env = gix_testtools::Env::new()
            .unset("EXEPATH")
            .set(
                "GIT_EXEC_PATH",
                prefix.join("libexec/git-core").to_str().expect("UTF-8 temporary path"),
            )
            .set(
                "PATH",
                installation.path().join("cmd").to_str().expect("UTF-8 temporary path"),
            );
        assert_ne!(
            gix_path::env::system_prefix(),
            Some(installation.path().join("ucrt64").as_path()),
            "GIT_EXEC_PATH takes this decision out of the shortcut's hands"
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
        // Both directories exist, so EXEPATH must fall back to the installation in PATH.
        std::fs::create_dir_all(prefix.join("bin"))?;
        std::fs::File::create_new(prefix.join("bin/git.exe"))?;
        std::fs::create_dir_all(prefix.join("libexec/git-core"))?;
        let _env = gix_testtools::Env::new()
            .set("EXEPATH", installation.path().to_str().expect("UTF-8 temporary path"))
            .set("PATH", prefix.join("bin").to_str().expect("UTF-8 temporary path"));
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

#[test]
fn native_core_discovery_uses_explicit_paths_and_standard_layouts_without_git() -> gix_testtools::TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_native_installation.sh")?.canonicalize()?;
    let path = std::env::join_paths(["unix-lib/prefix/bin", "real/bin"])?;
    assert_eq!(
        gix_path::env::core_dir_from_environment(|name| (name == "PATH").then(|| path.clone()), &fixture),
        Some(fixture.join("unix-lib/prefix/lib/git-core")),
        "an earlier helper-only installation precedes later Git executables"
    );
    for (bin, core) in [
        ("unix-lib/prefix/bin", "unix-lib/prefix/lib/git-core"),
        ("unix-libexec/prefix/bin", "unix-libexec/prefix/libexec/git-core"),
        ("windows-ucrt/prefix/cmd", "windows-ucrt/prefix/ucrt64/libexec/git-core"),
        (
            "windows-mingw/prefix/cmd",
            "windows-mingw/prefix/mingw64/libexec/git-core",
        ),
    ] {
        assert_eq!(
            gix_path::env::core_dir_from_environment(|name| (name == "PATH").then(|| bin.into()), &fixture),
            Some(fixture.join(core)),
            "standard helper directories can exist without the Git executable"
        );
    }
    assert_eq!(
        gix_path::env::core_dir_from_environment(
            |name| match name {
                "GIT_EXEC_PATH" => Some("custom-core".into()),
                "PATH" => Some("unix-lib/prefix/bin".into()),
                _ => None,
            },
            &fixture
        ),
        Some(fixture.join("custom-core")),
        "an explicit relative exec path takes precedence"
    );
    #[cfg(unix)]
    assert_eq!(
        gix_path::env::core_dir_from_environment(|name| (name == "PATH").then(|| "symlink/bin".into()), &fixture),
        Some(fixture.join("real/libexec/git-core")),
        "resolve installation symlinks without invoking their executable"
    );
    #[cfg(windows)]
    assert_eq!(
        gix_path::env::core_dir_from_environment(
            |name| (name == "GIT_INSTALL_ROOT").then(|| fixture.join("windows-ucrt/prefix").into_os_string()),
            &fixture
        ),
        Some(fixture.join("windows-ucrt/prefix/ucrt64/libexec/git-core")),
        "native helper discovery shares the fallback installation roots with executable discovery"
    );
    Ok(())
}

#[test]
#[cfg(target_os = "macos")]
fn apple_helpers_use_the_selected_developer_installation() -> gix_testtools::TestResult {
    let fixture = gix_testtools::scripted_fixture_read_only("make_native_installation.sh")?.canonicalize()?;
    assert_eq!(
        gix_path::env::core_dir_from_environment(
            |name| match name {
                "PATH" => Some("/usr/bin".into()),
                "DEVELOPER_DIR" => Some(fixture.join("apple/Developer").into_os_string()),
                _ => None,
            },
            &fixture
        ),
        Some(fixture.join("apple/Developer/usr/libexec/git-core")),
        "the system launcher delegates helpers to the selected developer installation"
    );
    Ok(())
}
