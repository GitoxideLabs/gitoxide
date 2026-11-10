use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

use super::git::{self, ConfigPaths};

pub(super) const EXE_NAME: &str = "git";
pub(super) const NULL_DEVICE: &str = "/dev/null";

pub(super) fn exe_invocation() -> &'static Path {
    Path::new(EXE_NAME)
}

pub(super) fn alternative_executable() -> Option<&'static Path> {
    None
}

pub(super) fn native_path(path: &Path) -> Option<PathBuf> {
    Some(path.to_owned())
}

pub(super) fn core_dir() -> Option<PathBuf> {
    git::core_dir_from_executable()
}

#[cfg(not(unix))]
pub(super) fn config_paths_without_git() -> Option<ConfigPaths> {
    None
}

#[cfg(unix)]
pub(super) fn config_paths_without_git() -> Option<ConfigPaths> {
    use std::os::unix::fs::PermissionsExt;

    for directory in std::env::split_paths(&std::env::var_os("PATH")?) {
        // The query runs from `/`, so relative PATH entries must be left to that query.
        if !directory.is_absolute() {
            return None;
        }
        let executable = directory.join(EXE_NAME);
        let metadata = match executable.metadata() {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => return None,
        };
        let execute_bits = metadata.permissions().mode() & 0o111;
        if !metadata.is_file() || execute_bits == 0 {
            continue;
        }
        // An execute bit for another user doesn't establish that this process can execute the file.
        if execute_bits != 0o111 {
            return None;
        }
        // Only the first executable counts, even when its layout is unknown.
        let resolved = executable.canonicalize().ok()?;
        let (config, is_system) = config_path_from_executable(&resolved)?;
        // Apple Git's installation config can depend on the invocation path instead of its symlink target.
        if !is_system && executable != resolved {
            return None;
        }
        return config_paths_at(config, is_system, system_config()?);
    }
    None
}

#[cfg(unix)]
fn config_paths_at(config: PathBuf, is_system: bool, system_config: &Path) -> Option<ConfigPaths> {
    if !config.is_file() {
        return (config == system_config).then(ConfigPaths::default);
    }
    let config = crate::os_string_into_bstring(config.into()).ok()?;
    Some(ConfigPaths {
        installation: Some(config.clone()),
        installation_is_system: is_system,
        system: is_system.then_some(config),
    })
}

#[cfg(unix)]
fn config_path_from_executable(executable: &Path) -> Option<(PathBuf, bool)> {
    // shortcut: recognized layouts assume standard build-time config paths; query custom layouts instead.
    #[cfg(target_os = "linux")]
    if executable == Path::new("/usr/bin/git") || executable == Path::new("/bin/git") {
        return Some(("/etc/gitconfig".into(), true));
    }

    let bin = executable.parent()?;
    if executable.file_name()? != EXE_NAME || bin.file_name()? != "bin" {
        return None;
    }
    let prefix = bin.parent()?;
    let package = prefix.parent()?;
    let cellar = package.parent()?;
    if package.file_name()? == "git" && cellar.file_name()? == "Cellar" {
        return Some((cellar.parent()?.join("etc/gitconfig"), true));
    }

    #[cfg(target_os = "macos")]
    if prefix.file_name()? == "usr" {
        let developer = prefix.parent()?;
        if developer.ends_with("Contents/Developer") || developer.ends_with("Library/Developer/CommandLineTools") {
            return Some((prefix.join("share/git-core/gitconfig"), false));
        }
    }
    None
}

pub(super) fn system_prefix() -> Option<&'static Path> {
    Some(Path::new("/"))
}

pub(super) fn system_config() -> Option<&'static Path> {
    Some(Path::new("/etc/gitconfig"))
}

pub(super) fn shell() -> super::Shell {
    super::Shell {
        program: "/bin/sh".into(),
        needs_posix_mode: false,
    }
}

pub(super) fn associated_program(_name: &str) -> Option<PathBuf> {
    None
}

pub(super) fn command(executable: impl AsRef<OsStr>) -> Command {
    Command::new(executable)
}

pub(super) fn config_query_cwd() -> PathBuf {
    "/".into()
}

#[cfg(all(test, unix))]
mod tests {
    use super::{ConfigPaths, config_paths_at};

    #[test]
    #[cfg(target_os = "linux")]
    fn distribution_config_is_outside_usr() {
        use super::config_path_from_executable;
        use std::path::Path;

        for executable in ["/usr/bin/git", "/bin/git"] {
            assert_eq!(
                config_path_from_executable(Path::new(executable)),
                Some(("/etc/gitconfig".into(), true)),
                "distribution Git uses system-scoped /etc/gitconfig"
            );
        }
        assert_eq!(
            config_path_from_executable(Path::new("/usr/local/bin/git")),
            None,
            "an arbitrary build prefix does not establish its config location"
        );
    }

    #[test]
    fn absent_system_config_is_no_installation_config() -> gix_error::TestResult {
        let root = tempfile::tempdir()?;
        let system = root.path().join("etc/gitconfig");
        assert_eq!(
            config_paths_at(system.clone(), true, &system),
            Some(ConfigPaths::default()),
            "Git reports no installation config when its system config doesn't exist"
        );
        assert_eq!(
            config_paths_at(root.path().join("prefix/etc/gitconfig"), true, &system),
            None,
            "a missing prefix config can mean a misidentified prefix, so Git is queried"
        );

        std::fs::create_dir_all(root.path().join("etc"))?;
        std::fs::write(&system, "")?;
        let expected = crate::os_string_into_bstring(system.clone().into()).expect("temporary paths are valid UTF-8");
        assert_eq!(
            config_paths_at(system.clone(), true, &system),
            Some(ConfigPaths {
                installation: Some(expected.clone()),
                installation_is_system: true,
                system: Some(expected),
            }),
            "an existing system config is the installation config"
        );
        Ok(())
    }
}
