use std::path::Path;

mod exe_info {
    use bstr::ByteSlice;
    use std::{
        ffi::{OsStr, OsString},
        path::{Path, PathBuf},
    };

    use serial_test::serial;

    #[cfg(unix)]
    use crate::env::git::ConfigPaths;
    use crate::env::{
        git::{NULL_DEVICE, config_paths_from_executable, config_paths_from_executable_at},
        tests::CurrentDir,
    };

    fn exe_info() -> Option<bstr::BString> {
        config_paths_from_executable().installation
    }

    #[cfg(unix)]
    fn fake_git(script: &str) -> (tempfile::TempDir, PathBuf) {
        use std::os::unix::fs::PermissionsExt;

        let tempdir = tempfile::tempdir().expect("can create fake Git directory");
        let executable = tempdir.path().join("git");
        std::fs::write(&executable, script).expect("can write fake Git");
        let mut permissions = std::fs::metadata(&executable)
            .expect("fake Git has metadata")
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&executable, permissions).expect("can make fake Git executable");
        (tempdir, executable)
    }

    #[cfg(unix)]
    fn invocations(executable: &Path) -> Vec<String> {
        std::fs::read_to_string(executable.with_extension("log"))
            .expect("invocation log exists")
            .lines()
            .map(ToOwned::to_owned)
            .collect()
    }

    /// This is a copy from the respective type in `gix-testtools` - deduplicate if it can ever be a dependency again.
    struct Env(Vec<(OsString, Option<OsString>)>);

    impl Env {
        fn new() -> Self {
            Env(Vec::new())
        }

        fn set(mut self, key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
            let key = key.as_ref().to_owned();
            self.0.push((key.clone(), std::env::var_os(&key)));
            // SAFETY: All tests which mutate the process environment are serialized.
            unsafe { std::env::set_var(key, value) };
            self
        }

        fn unset(mut self, key: impl AsRef<OsStr>) -> Self {
            let key = key.as_ref().to_owned();
            self.0.push((key.clone(), std::env::var_os(&key)));
            // SAFETY: All tests which mutate the process environment are serialized.
            unsafe { std::env::remove_var(key) };
            self
        }
    }

    impl Drop for Env {
        fn drop(&mut self) {
            for (key, value) in self.0.drain(..).rev() {
                // SAFETY: All tests which mutate the process environment are serialized.
                unsafe {
                    if let Some(value) = value {
                        std::env::set_var(key, value);
                    } else {
                        std::env::remove_var(key);
                    }
                }
            }
        }
    }

    fn local_config_repo() -> tempfile::TempDir {
        let repo = tempfile::tempdir().expect("can create repository directory");
        initialize_repo_without_git(repo.path());
        repo
    }

    /// Create just enough repository metadata for Git to discover its local configuration.
    ///
    /// This fixture is written directly so its setup remains independent of the ambient and
    /// repository-local Git configuration whose influence the test is meant to detect.
    fn initialize_repo_without_git(repo: &Path) {
        let git_dir = repo.join(".git");
        std::fs::create_dir_all(git_dir.join("objects")).expect("can create object directory");
        std::fs::create_dir_all(git_dir.join("refs")).expect("can create references directory");
        std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("can write repository HEAD");
        std::fs::write(
            git_dir.join("config"),
            "[core]\n\trepositoryFormatVersion = 0\n\tbare = false\n[foo]\n\tbar = baz\n",
        )
        .expect("can write local test configuration");
    }

    /// Wrapper for a valid path to a plausible location, kept from accidentally existing (until drop).
    #[derive(Debug)]
    struct NonexistentLocation {
        _empty: tempfile::TempDir,
        nonexistent: PathBuf,
    }

    impl NonexistentLocation {
        fn new() -> Self {
            let empty = tempfile::tempdir().expect("can create new temporary subdirectory");

            let nonexistent = empty
                .path()
                .canonicalize()
                .expect("path to the new directory works")
                .join("nonexistent");

            assert!(!nonexistent.exists(), "Test bug: Need nonexistent directory");

            Self {
                _empty: empty,
                nonexistent,
            }
        }

        fn path(&self) -> &Path {
            &self.nonexistent
        }
    }

    fn set_temp_env_vars(path: &Path) -> Env {
        let path_str = path.to_str().expect("valid Unicode");

        let env = Env::new()
            .set("TMPDIR", path_str) // Mainly for Unix.
            .set("TMP", path_str) // Mainly for Windows.
            .set("TEMP", path_str); // Mainly for Windows, too.

        assert_eq!(
            std::env::temp_dir(),
            path,
            "Possible test bug: Temp dir path may not have been customized successfully"
        );

        env
    }

    fn unset_windows_directory_vars() -> Env {
        Env::new().unset("windir").unset("SystemRoot")
    }

    fn check_exe_info() -> gix_error::TestResult {
        let paths = config_paths_from_executable_at(crate::env::exe_invocation().to_owned())?;
        if let Some(path) = paths.installation.map(crate::from_bstring).transpose()? {
            assert!(path.is_absolute(), "Git reports an absolute installation path");
            assert!(
                path.exists(),
                "It should exist on disk, since `git config` just found an entry there"
            );
        }
        Ok(())
    }

    #[test]
    #[serial]
    fn with_unmodified_environment() -> gix_error::TestResult {
        let _isolation = gix_testtools::isolate_git_environment()?;
        check_exe_info()
    }

    #[test]
    #[serial]
    fn tolerates_broken_temp() -> gix_error::TestResult {
        let _isolation = gix_testtools::isolate_git_environment()?;
        let non = NonexistentLocation::new();
        let _env = set_temp_env_vars(non.path());
        check_exe_info()
    }

    #[test]
    #[serial]
    fn tolerates_oversanitized_env() -> gix_error::TestResult {
        let _isolation = gix_testtools::isolate_git_environment()?;
        // This test runs on all systems, but it is only checking for a Windows regression. Also, on
        // Windows, having both a broken temp dir and an over-sanitized environment is not supported.
        let _env = unset_windows_directory_vars();
        check_exe_info()
    }

    #[test]
    #[serial]
    fn tolerates_git_config_env_var() -> gix_error::TestResult {
        let _isolation = gix_testtools::isolate_git_environment()?;
        let _env = Env::new().set("GIT_CONFIG", NULL_DEVICE);
        check_exe_info()
    }

    #[test]
    #[serial]
    fn same_result_with_broken_temp() {
        let with_unmodified_temp = exe_info();

        let with_nonexistent_temp = {
            let non = NonexistentLocation::new();
            let _env = set_temp_env_vars(non.path());
            exe_info()
        };

        assert_eq!(with_unmodified_temp, with_nonexistent_temp);
    }

    #[test]
    #[serial]
    fn same_result_with_oversanitized_env() {
        let with_unmodified_env = exe_info();

        let with_oversanitized_env = {
            let _env = unset_windows_directory_vars();
            exe_info()
        };

        assert_eq!(with_unmodified_env, with_oversanitized_env);
    }

    #[test]
    #[serial]
    fn same_result_with_git_config_env_var() {
        let with_unmodified_env = exe_info();

        let with_git_config_env_var = {
            let _env = Env::new().set("GIT_CONFIG", NULL_DEVICE);
            exe_info()
        };

        assert_eq!(with_unmodified_env, with_git_config_env_var);
    }

    #[test]
    #[serial]
    fn configuration_query_ignores_ambient_config_and_local_repo() {
        let expected = config_paths_from_executable();
        let repo = local_config_repo();
        let config_path = repo.path().join(".git").join("config");
        let _cwd = CurrentDir::set(repo.path()).expect("can change to repo dir");
        let _env = Env::new()
            .set("GIT_CONFIG_NOSYSTEM", "1")
            .set("GIT_CONFIG_SYSTEM", &config_path)
            .set("GIT_CONFIG_GLOBAL", &config_path)
            .set("GIT_CONFIG", &config_path)
            .set("GIT_CONFIG_COUNT", "1")
            .set("GIT_CONFIG_KEY_0", "include.path")
            .set("GIT_CONFIG_VALUE_0", &config_path)
            .set("GIT_CONFIG_PARAMETERS", "invalid");

        assert_eq!(
            config_paths_from_executable(),
            expected,
            "configuration discovery is independent of ambient overrides and repository-local configuration"
        );
    }

    #[test]
    #[cfg(unix)]
    fn empty_config_has_no_installation_path() -> gix_error::TestResult {
        let (_tempdir, executable) = fake_git(
            "#!/bin/sh
exit 0
",
        );
        assert_eq!(
            config_paths_from_executable_at(executable)?,
            ConfigPaths::default(),
            "a successful query with no config entries has no discoverable installation path"
        );
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn one_scoped_query_finds_both_config_paths() -> gix_testtools::TestResult {
        let (_tempdir, executable) = fake_git(
            r#"#!/bin/sh
printf '%s\n' "$*" >> "${0}.log"
printf 'unknown\000file:/installation/gitconfig\000core.one\000system\000file:/system/gitconfig\000core.two\000'
"#,
        );

        assert_eq!(
            config_paths_from_executable_at(executable.clone())?,
            ConfigPaths {
                installation: Some("/installation/gitconfig".into()),
                installation_is_system: false,
                system: Some("/system/gitconfig".into()),
            }
        );
        let invocations = invocations(&executable);
        assert_eq!(
            invocations.len(),
            1,
            "a successful scoped query obtains both paths in one invocation"
        );
        assert!(
            invocations[0].contains("--no-includes"),
            "included files must not be mistaken for top-level configuration paths"
        );
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn retries_without_scope_for_old_git() -> gix_testtools::TestResult {
        let (_tempdir, executable) = fake_git(
            r#"#!/bin/sh
printf '%s\n' "$*" >> "${0}.log"
case " $* " in
  *" --show-scope "*) exit 129 ;;
esac
printf 'file:/legacy/gitconfig\000core.one\000'
"#,
        );

        assert_eq!(
            config_paths_from_executable_at(executable.clone())?,
            ConfigPaths {
                installation: Some("/legacy/gitconfig".into()),
                ..Default::default()
            },
            "the legacy query preserves installation-config discovery"
        );
        let invocations = invocations(&executable);
        assert_eq!(invocations.len(), 2, "old Git is retried once");
        assert!(
            invocations.first().is_some_and(|line| line.contains("--show-scope")),
            "the first query requests scopes"
        );
        assert!(
            invocations.get(1).is_some_and(|line| !line.contains("--show-scope")),
            "the fallback query omits unsupported scope reporting"
        );
        Ok(())
    }

    #[test]
    fn config_paths_from_config_with_origin() {
        let macos = "unknown\0file:/Applications/Xcode.app/Contents/Developer/usr/share/git-core/gitconfig\0credential.helper\0global\0file:/Users/byron/.gitconfig\0push.default\0";
        let win_msys = "system\0file:C:/git-sdk-64/etc/gitconfig\0core.symlinks\0system\0file:C:/git-sdk-64/etc/gitconfig\0core.autocrlf\0";
        let win_cmd = "system\0file:C:/Program Files/Git/etc/gitconfig\0diff.astextplain.textconv\0system\0file:C:/Program Files/Git/etc/gitconfig\0filter.lfs.clean\0";
        let win_cmd_with_system = "system\0file:C:/Program Files/Git/etc/gitconfig\0diff.astextplain.textconv\0system\0file:C:/ProgramData/Git/config\0core.autocrlf\0";
        let win_msys_old = "system\0file:C:\\ProgramData/Git/config\0diff.astextplain.textconv\0system\0file:C:\\ProgramData/Git/config\0filter.lfs.clean\0";
        let linux = "global\0file:/home/parallels/.gitconfig\0core.excludesfile\0";
        let bogus = "something unexpected";
        let empty = "";

        for (source, expected) in [
            (
                macos,
                (
                    Some("/Applications/Xcode.app/Contents/Developer/usr/share/git-core/gitconfig"),
                    false,
                    None,
                ),
            ),
            (
                win_msys,
                (
                    Some("C:/git-sdk-64/etc/gitconfig"),
                    true,
                    Some("C:/git-sdk-64/etc/gitconfig"),
                ),
            ),
            (
                win_msys_old,
                (
                    Some(r"C:\ProgramData/Git/config"),
                    true,
                    Some(r"C:\ProgramData/Git/config"),
                ),
            ),
            (
                win_cmd,
                (
                    Some("C:/Program Files/Git/etc/gitconfig"),
                    true,
                    Some("C:/Program Files/Git/etc/gitconfig"),
                ),
            ),
            (
                win_cmd_with_system,
                (
                    Some("C:/Program Files/Git/etc/gitconfig"),
                    true,
                    Some("C:/ProgramData/Git/config"),
                ),
            ),
            (linux, (Some("/home/parallels/.gitconfig"), false, None)),
            (bogus, (None, false, None)),
            (empty, (None, false, None)),
        ] {
            let actual = crate::env::git::config_paths_from_config_with_origin(source.into());
            assert_eq!(
                (
                    actual.0.map(|path| path.to_str().expect("test paths are UTF-8")),
                    actual.1,
                    actual.2.map(|path| path.to_str().expect("test paths are UTF-8")),
                ),
                expected
            );
        }
    }
}

#[test]
fn config_to_base_path() {
    for (input, expected) in [
        (
            "/Applications/Xcode.app/Contents/Developer/usr/share/git-core/gitconfig",
            "/Applications/Xcode.app/Contents/Developer/usr/share/git-core",
        ),
        ("C:/git-sdk-64/etc/gitconfig", "C:/git-sdk-64/etc"),
        (r"C:\ProgramData/Git/config", r"C:\ProgramData/Git"),
        ("C:/Program Files/Git/etc/gitconfig", "C:/Program Files/Git/etc"),
    ] {
        assert_eq!(super::config_to_base_path(Path::new(input)), Path::new(expected));
    }
}
