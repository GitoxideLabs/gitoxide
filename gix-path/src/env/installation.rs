use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

/// Find Git's helper directory from `environment` and filesystem installation layouts.
///
/// A nonempty `GIT_EXEC_PATH` takes precedence, even when the directory does not exist. Otherwise,
/// search installations associated with Git executables in `PATH`, resolving executable symlinks,
/// then conventional helper directories adjacent to `PATH`'s `bin` or `cmd` directories. The latter
/// permits installations containing external helpers without the Git executable. Git for Windows'
/// standard program-files, package-manager, and registered installations are also considered on Windows.
///
/// Relative paths are resolved against the supplied absolute `current_dir`. No command is run and
/// no process environment is read. Unusual compile-time installation layouts require `GIT_EXEC_PATH`.
pub fn core_dir_from_environment(
    mut environment: impl FnMut(&str) -> Option<OsString>,
    current_dir: &Path,
) -> Option<PathBuf> {
    if !current_dir.is_absolute() {
        return None;
    }
    if let Some(path) = environment("GIT_EXEC_PATH").filter(|path| !path.is_empty()) {
        return Some(current_dir.join(path));
    }
    let directories = search_directories(&mut environment, current_dir);
    for directory in &directories {
        let executable = directory.join(super::git::EXE_NAME);
        if executable.is_file() {
            #[cfg(target_os = "macos")]
            if directory == Path::new("/usr/bin")
                && let Some(prefix) = developer_prefix(&mut environment)
                && let Some(core) = core_from_bin(&prefix.join("bin"))
            {
                return Some(core);
            }
            if let Ok(resolved) = std::fs::canonicalize(&executable)
                && let Some(core) = resolved.parent().and_then(core_from_bin)
            {
                return Some(core);
            }
            if let Some(core) = core_from_bin(directory) {
                return Some(core);
            }
        }
    }
    directories.iter().find_map(|directory| core_from_bin(directory))
}

pub(super) fn search_directories(
    environment: &mut impl FnMut(&str) -> Option<OsString>,
    current_dir: &Path,
) -> Vec<PathBuf> {
    let paths = environment("PATH").unwrap_or_default();
    #[allow(unused_mut)]
    let mut directories: Vec<_> = std::env::split_paths(&paths)
        .map(|path| current_dir.join(path))
        .collect();
    #[cfg(windows)]
    {
        directories.extend(super::git::locations_under_program_files(&mut *environment));
        directories.extend(super::git::locations_under_scoop(&mut *environment));
        directories.extend(super::git::locations_under_package_managers(&mut *environment));
        directories.extend(super::git::registry::installation_bins());
    }
    directories
}

pub(super) fn core_from_bin(directory: &Path) -> Option<PathBuf> {
    if directory.ends_with("git-core") && directory.is_dir() {
        return Some(directory.into());
    }
    if !directory
        .file_name()
        .is_some_and(|name| ["bin", "sbin", "cmd"].iter().any(|candidate| name == *candidate))
    {
        return None;
    }
    let prefix = directory.parent()?;
    for suffix in ["libexec/git-core", "lib/git-core"] {
        let core = prefix.join(suffix);
        if core.is_dir() {
            return Some(core);
        }
    }
    // Git for Windows' `cmd/git.exe` and `bin/git.exe` are launchers for the installation's
    // architecture directory. Also supports helper-only installations reached through `cmd`.
    let architectures = if cfg!(target_arch = "aarch64") {
        ["clangarm64", "ucrt64", "mingw64", "mingw32", "clang64", "clang32"]
    } else if cfg!(target_pointer_width = "32") {
        ["mingw32", "clang32", "ucrt64", "mingw64", "clang64", "clangarm64"]
    } else {
        ["ucrt64", "mingw64", "clang64", "mingw32", "clang32", "clangarm64"]
    };
    for architecture in architectures {
        let core = prefix.join(architecture).join("libexec/git-core");
        if core.is_dir() {
            return Some(core);
        }
    }
    None
}

/// Apple's system Git launcher uses the developer directory selected by xcode-select.
#[cfg(target_os = "macos")]
pub(super) fn developer_prefix(environment: &mut impl FnMut(&str) -> Option<OsString>) -> Option<PathBuf> {
    let developer = environment("DEVELOPER_DIR")
        .map(PathBuf::from)
        .or_else(|| std::fs::read_link("/var/db/xcode_select_link").ok())
        .filter(|path| path.is_absolute())
        .or_else(|| {
            [
                "/Library/Developer/CommandLineTools",
                "/Applications/Xcode.app/Contents/Developer",
            ]
            .into_iter()
            .map(PathBuf::from)
            .find(|path| path.is_dir())
        })?;
    // DEVELOPER_DIR also accepts the path to Xcode.app itself.
    let developer = if developer.extension().is_some_and(|extension| extension == "app") {
        developer.join("Contents/Developer")
    } else {
        developer
    };
    Some(developer.join("usr"))
}
