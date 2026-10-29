use std::path::Path;

mod installation_config {
    use std::path::Path;

    use super::super::{ConfigPaths, config_paths_from_environment};

    fn discover(fixture: &Path, bin: &str) -> ConfigPaths {
        config_paths_from_environment(
            |name| match name {
                "PATH" => Some(bin.into()),
                // These select helpers or override configuration at a higher layer, and must
                // never change the installation being discovered.
                "GIT_EXEC_PATH" | "GIT_CONFIG_SYSTEM" | "GIT_CONFIG_GLOBAL" | "GIT_CONFIG" => {
                    Some(fixture.join("custom-core").into_os_string())
                }
                "GIT_CONFIG_NOSYSTEM" => Some("1".into()),
                _ => None,
            },
            fixture,
        )
    }

    #[test]
    #[cfg(unix)]
    fn configuration_discovery_uses_filesystem_prefixes_without_git() -> gix_testtools::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("make_native_installation.sh")?.canonicalize()?;
        let path = std::env::join_paths(["unix-lib/prefix/bin", "real/bin"])?;
        assert_eq!(
            discover(&fixture, path.to_str().expect("fixture paths are UTF-8")).system,
            Some(crate::os_string_into_bstring(
                fixture.join("unix-lib/prefix/etc/gitconfig").into_os_string()
            )?),
            "an earlier helper-only installation supplies the configuration too"
        );
        for bin in ["unix-lib/prefix/bin", "unix-libexec/prefix/bin"] {
            let config = fixture
                .join(bin)
                .parent()
                .expect("fixture bin has a prefix")
                .join("etc/gitconfig");
            let config = crate::os_string_into_bstring(config.into_os_string())?;
            assert_eq!(
                discover(&fixture, bin),
                ConfigPaths {
                    installation: Some(config.clone()),
                    installation_is_system: true,
                    system: Some(config),
                }
            );
        }
        let config = fixture.join("symlink/etc/gitconfig");
        assert_eq!(
            discover(&fixture, "symlink/bin").installation,
            Some(crate::os_string_into_bstring(config.into_os_string())?),
            "a package-manager symlink can keep configuration outside its versioned runtime"
        );
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn apple_installation_configuration_precedes_system_configuration() -> gix_testtools::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("make_native_installation.sh")?.canonicalize()?;
        assert_eq!(
            discover(&fixture, "apple/Developer/usr/bin"),
            ConfigPaths {
                installation: Some(crate::os_string_into_bstring(
                    fixture
                        .join("apple/Developer/usr/share/git-core/gitconfig")
                        .into_os_string()
                )?),
                installation_is_system: false,
                system: Some("/etc/gitconfig".into()),
            }
        );
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn missing_installation_retains_the_system_default() {
        assert_eq!(
            discover(Path::new("/"), "missing-git/bin"),
            ConfigPaths {
                installation: None,
                installation_is_system: false,
                system: Some("/etc/gitconfig".into()),
            }
        );
    }

    #[test]
    #[cfg(windows)]
    fn windows_configuration_uses_runtime_parent_and_program_data() -> gix_testtools::TestResult {
        let fixture = gix_testtools::scripted_fixture_read_only("make_native_installation.sh")?.canonicalize()?;
        for runtime in ["windows-ucrt", "windows-mingw"] {
            let config = fixture.join(runtime).join("prefix/etc/gitconfig");
            let paths = discover(&fixture, &format!("{runtime}/prefix/cmd"));
            assert_eq!(
                paths.system,
                Some(crate::os_string_into_bstring(config.into_os_string())?)
            );
            assert!(paths.installation_is_system);
        }
        let paths = config_paths_from_environment(
            |name| match name {
                "EXEPATH" => Some(fixture.join("windows-ucrt/prefix").into_os_string()),
                "PROGRAMDATA" => Some(fixture.join("program-data").into_os_string()),
                _ => None,
            },
            &fixture,
        );
        assert_eq!(
            paths.installation,
            Some(crate::os_string_into_bstring(
                fixture.join("program-data/Git/config").into_os_string()
            )?)
        );
        assert_eq!(
            paths.system,
            Some(crate::os_string_into_bstring(
                fixture.join("windows-ucrt/prefix/etc/gitconfig").into_os_string()
            )?)
        );
        Ok(())
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
