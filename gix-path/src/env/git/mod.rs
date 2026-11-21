use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use bstr::{BStr, BString};
use std::sync::LazyLock;

use super::platform;
pub(super) use super::platform::EXE_NAME;

#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct ConfigPaths {
    pub(super) installation: Option<BString>,
    pub(super) installation_is_system: bool,
    pub(super) system: Option<BString>,
}

/// Cache conventional configuration locations without executing Git. Environment overrides of
/// configuration files and helper paths are intentionally applied by callers, not discovery.
pub(super) static GIT_CONFIG_PATHS: LazyLock<ConfigPaths> = LazyLock::new(|| {
    platform::config_paths_without_git().unwrap_or_else(|| {
        std::env::current_dir()
            .ok()
            .map(|cwd| config_paths_from_environment(|key| std::env::var_os(key), &cwd))
            .unwrap_or_default()
    })
});

/// Discover the selected installation's core directory without executing Git.
#[cfg(not(windows))]
pub(super) fn core_dir_from_executable() -> Option<PathBuf> {
    super::core_dir_from_environment(|key| std::env::var_os(key), &std::env::current_dir().ok()?)
}

fn config_paths_from_environment(
    mut environment: impl FnMut(&str) -> Option<OsString>,
    current_dir: &Path,
) -> ConfigPaths {
    if !current_dir.is_absolute() {
        return ConfigPaths::default();
    }
    let directories = super::installation::search_directories(&mut environment, current_dir);
    let mut prefixes = Vec::new();
    #[cfg(windows)]
    if let Some(prefix) = platform::system_prefix_from_exepath_var(&mut environment) {
        prefixes.push(prefix);
    }
    if prefixes.is_empty()
        && let Some(directory) = directories.iter().find(|directory| {
            directory.join(EXE_NAME).is_file() || super::installation::core_from_bin(directory).is_some()
        })
    {
        let executable = directory.join(EXE_NAME);
        let resolved = || crate::realpath_opts(&executable, current_dir, crate::realpath::MAX_SYMLINKS).ok();
        #[cfg(not(windows))]
        {
            // Preserve a package manager's stable symlink prefix before trying the versioned
            // installation itself, where helpers may live but configuration usually does not.
            if let Some(prefix) = directory.parent() {
                prefixes.push(prefix.to_path_buf());
            }
            if let Some(prefix) = resolved().as_deref().and_then(Path::parent).and_then(Path::parent)
                && !prefixes.iter().any(|existing| existing == prefix)
            {
                prefixes.push(prefix.to_path_buf());
            }
        }
        #[cfg(windows)]
        {
            let prefix = if executable.is_file() {
                resolved()
                    .as_deref()
                    .and_then(platform::system_prefix_from_git_executable)
            } else {
                super::installation::core_from_bin(directory)
                    .and_then(|core| core.parent().and_then(Path::parent).map(Path::to_path_buf))
            };
            prefixes.extend(prefix);
        }
    }
    #[cfg(unix)]
    if let Some(directory) = directories.iter().find(|directory| directory.join(EXE_NAME).is_file())
        && let Ok(executable) =
            crate::realpath_opts(&directory.join(EXE_NAME), current_dir, crate::realpath::MAX_SYMLINKS)
        && let Some((config, is_system)) = platform::config_path_from_executable(&executable)
        && config.is_file()
    {
        let system = if is_system {
            config.clone()
        } else {
            PathBuf::from("/etc/gitconfig")
        };
        return ConfigPaths {
            installation: crate::os_string_into_bstring(config.into()).ok(),
            installation_is_system: is_system,
            system: crate::os_string_into_bstring(system.into()).ok(),
        };
    }
    #[cfg(target_os = "macos")]
    if prefixes.iter().any(|prefix| prefix == Path::new("/usr"))
        && let Some(prefix) = super::installation::developer_prefix(&mut environment)
    {
        prefixes.push(prefix);
    }
    let system = if cfg!(windows) {
        prefixes
            .first()
            .map(|prefix| prefix.parent().unwrap_or(prefix).join("etc/gitconfig"))
    } else {
        Some(
            prefixes
                .iter()
                .map(|prefix| {
                    if prefix == Path::new("/usr") {
                        PathBuf::from("/etc/gitconfig")
                    } else {
                        prefix.join("etc/gitconfig")
                    }
                })
                .find(|path| path.is_file())
                .unwrap_or_else(|| "/etc/gitconfig".into()),
        )
    };
    let installation = prefixes
        .iter()
        .map(|prefix| prefix.join("share/git-core/gitconfig"))
        .find(|path| path.is_file());
    let installation_is_system = installation.is_none();
    #[cfg(windows)]
    let installation = installation.or_else(|| {
        system.as_ref()?;
        environment("PROGRAMDATA")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .map(|path| path.join("Git/config"))
            .filter(|path| path.is_file())
    });
    let installation = installation.or_else(|| system.as_ref().filter(|path| path.is_file()).cloned());
    ConfigPaths {
        installation_is_system: installation_is_system && installation.is_some(),
        installation: installation.and_then(|path| crate::os_string_into_bstring(path.into()).ok()),
        system: system.and_then(|path| crate::os_string_into_bstring(path.into()).ok()),
    }
}

/// Try to find configuration associated with a conventional Git installation.
pub(super) fn install_config_path() -> Option<&'static BStr> {
    let _span = gix_trace::detail!("gix_path::git::install_config_path()");
    GIT_CONFIG_PATHS.installation.as_ref().map(AsRef::as_ref)
}

pub(super) fn install_config_is_system() -> bool {
    GIT_CONFIG_PATHS.installation_is_system
}

/// Given `config_path` as obtained from `install_config_path()`, return the path of the git installation base.
pub(super) fn config_to_base_path(config_path: &Path) -> &Path {
    config_path
        .parent()
        .expect("config file paths always have a file name to pop")
}

#[cfg(test)]
mod tests;
