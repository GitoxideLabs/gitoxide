use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

use bstr::{BStr, BString};
#[cfg(windows)]
use std::env;
use std::sync::LazyLock;

#[cfg(windows)]
pub(super) mod registry;

/// Other places to find Git in, probing each source only after earlier candidates are exhausted.
#[cfg(windows)]
fn alternative_locations<I: IntoIterator<Item = PathBuf>>(
    var_os: impl Fn(&str) -> Option<std::ffi::OsString> + Copy,
    registered: impl FnOnce() -> I,
) -> impl Iterator<Item = PathBuf> {
    std::iter::once_with(move || locations_under_program_files(var_os))
        .flatten()
        .chain(locations_under_scoop(var_os))
        .chain(locations_under_package_managers(var_os))
        .chain(std::iter::once_with(registered).flatten())
}

#[cfg(windows)]
fn alternative_executable() -> Option<&'static Path> {
    static EXECUTABLE: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
        alternative_locations(|key| env::var_os(key), registry::installation_bins)
            .map(|prefix| prefix.join(EXE_NAME))
            .find(|path| path.is_file())
    });
    EXECUTABLE.as_deref()
}

#[cfg(windows)]
pub(super) fn locations_under_program_files<F>(mut var_os_func: F) -> Vec<PathBuf>
where
    F: FnMut(&str) -> Option<std::ffi::OsString>,
{
    // Should give a 64-bit program files path from a 32-bit or 64-bit process on a 64-bit system.
    let varname_64bit = "ProgramW6432";

    // Should give a 32-bit program files path from a 32-bit or 64-bit process on a 64-bit system.
    // This variable is x86-specific, but neither Git nor Rust target 32-bit ARM on Windows.
    let varname_x86 = "ProgramFiles(x86)";

    // Should give a 32-bit program files path on a 32-bit system. We also check this on a 64-bit
    // system, even though it *should* equal the process's architecture-specific variable, so that
    // we cover the case of a parent process that passes down an overly sanitized environment that
    // lacks the architecture-specific variable. On a 64-bit system, because parent and child
    // processes' architectures can be different, Windows sets the child's `ProgramFiles` variable
    // from whichever of the `ProgramW6432` or `ProgramFiles(x86)` variable corresponds to the
    // child's architecture. Only if the parent does not pass down the architecture-specific
    // variable corresponding to the child's architecture does the child receive its `ProgramFiles`
    // variable from `ProgramFiles` as passed down by the parent. But this behavior is not well
    // known. So the situation where a process only passes down `ProgramFiles` sometimes happens.
    let varname_current = "ProgramFiles";

    // Should give the user's local application data path on any system. If a user program files
    // directory exists for this user, then it should be the `Programs` subdirectory of this. If it
    // doesn't exist, or on a future or extremely strangely configured Windows setup where it is
    // somewhere else, it should still be safe to attempt to use it. (This differs from global
    // program files paths, which are usually subdirectories of the root of the system drive, which
    // limited user accounts can usually create their own arbitrarily named directories inside.)
    let varname_user_appdata_local = "LocalAppData";

    // Prefer native ARM64 over an emulated x86_64 executable when both are present.
    // For x86_64, Git for Windows 2.56 uses `ucrt64`; keep `mingw64` for older releases.
    let suffixes_64 = &[r"Git\clangarm64\bin", r"Git\ucrt64\bin", r"Git\mingw64\bin"][..];

    // 32-bit relative bin dirs. So far, this is only ever `mingw32`, not `clang32`.
    let suffixes_32 = &[r"Git\mingw32\bin"][..];

    // Whichever of the 64-bit or 32-bit relative bin better matches this process's architecture.
    // Unlike the system architecture, the process architecture is always known at compile time.
    #[cfg(target_pointer_width = "64")]
    let suffixes_current = suffixes_64;
    #[cfg(target_pointer_width = "32")]
    let suffixes_current = suffixes_32;

    // Bin dirs relative to a user's local application data directory. We try each architecture.
    let suffixes_user = &[
        r"Programs\Git\clangarm64\bin",
        r"Programs\Git\ucrt64\bin",
        r"Programs\Git\mingw64\bin",
        r"Programs\Git\mingw32\bin",
    ][..];

    let rules = [
        (varname_user_appdata_local, suffixes_user),
        (varname_64bit, suffixes_64),
        (varname_x86, suffixes_32),
        (varname_current, suffixes_current),
    ];

    let mut locations = vec![];

    for (varname, suffixes) in rules {
        let Some(program_files_dir) = var_os_func(varname).map(PathBuf::from).filter(|p| p.is_absolute()) else {
            // The environment variable is unset or somehow not an absolute path (e.g. an empty string).
            continue;
        };
        for suffix in suffixes {
            let location = program_files_dir.join(suffix);
            if !locations.contains(&location) {
                locations.push(location);
            }
        }
    }

    locations
}

#[cfg(windows)]
pub(super) fn locations_under_scoop(
    mut var_os: impl FnMut(&str) -> Option<std::ffi::OsString>,
) -> impl Iterator<Item = PathBuf> {
    let mut absolute = move |name| var_os(name).map(PathBuf::from).filter(|path| path.is_absolute());
    // Scoop exports GIT_INSTALL_ROOT even when its root was configured outside the environment.
    // Find the stable `current` installation without relying on Scoop's shims being on PATH.
    [
        ("GIT_INSTALL_ROOT", None),
        ("SCOOP", Some("USERPROFILE")),
        ("SCOOP_GLOBAL", Some("ProgramData")),
    ]
    .into_iter()
    .filter_map(move |(variable, default_base)| {
        let root = absolute(variable).or_else(|| absolute(default_base?).map(|base| base.join("scoop")))?;
        Some((root, default_base.is_some()))
    })
    .flat_map(|(root, is_scoop)| {
        if is_scoop {
            let app = root.join("apps/git");
            let current = app.join("current/cmd");
            let version = (!current.join(EXE_NAME).is_file())
                .then(|| scoop_version_without_junction(&app))
                .flatten();
            version.into_iter().chain(Some(current))
        } else {
            None.into_iter().chain(Some(root.join("cmd")))
        }
    })
}

#[cfg(windows)]
fn scoop_version_without_junction(app: &Path) -> Option<PathBuf> {
    // Scoop's Select-CurrentVersion/Get-InstalledVersion uses install-marker timestamps,
    // not version ordering. A completed downgrade can be newer than a retained upgrade.
    std::fs::read_dir(app)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_str()?.to_ascii_lowercase();
            if name == "current" || (name.starts_with('_') && name.contains(".old")) {
                return None;
            }
            let directory = entry.path();
            let modified = ["scoop-install.json", "install.json"]
                .iter()
                .filter_map(|marker| std::fs::metadata(directory.join(marker)).ok())
                .filter(std::fs::Metadata::is_file)
                .filter_map(|metadata| metadata.modified().ok())
                .min()?;
            Some((modified, directory))
        })
        .max()
        .map(|(_, directory)| directory.join("cmd"))
}

#[cfg(windows)]
pub(super) fn locations_under_package_managers(
    mut var_os: impl FnMut(&str) -> Option<std::ffi::OsString>,
) -> impl Iterator<Item = PathBuf> {
    // These are stable entry points, including WinGet's links for custom portable roots.
    // Only the active Conda/Mamba environment participates; do not pick an inactive environment.
    [
        ("CONDA_PREFIX", "Library/bin"),
        ("ChocolateyToolsLocation", "git/cmd"),
        ("LocalAppData", "Microsoft/WinGet/Links"),
        ("ProgramW6432", "WinGet/Links"),
        ("ProgramFiles", "WinGet/Links"),
        ("GIT_INSTALL_ROOT", "usr/bin"),
    ]
    .into_iter()
    .filter_map(move |(variable, suffix)| {
        var_os(variable)
            .map(PathBuf::from)
            .filter(|root| root.is_absolute())
            .map(|root| root.join(suffix))
    })
}

#[cfg(windows)]
struct Executable {
    path: PathBuf,
    from_path: bool,
}

#[cfg(windows)]
impl Executable {
    fn invocation(&self) -> &Path {
        if self.from_path {
            Path::new(EXE_NAME)
        } else {
            &self.path
        }
    }
}

#[cfg(windows)]
fn find_executable(path: Option<&std::ffi::OsStr>, fallback: impl FnOnce() -> Option<PathBuf>) -> Option<Executable> {
    path.into_iter()
        .flat_map(env::split_paths)
        .map(|prefix| prefix.join(EXE_NAME))
        .find(|path| path.is_file())
        .map(|path| (path, true))
        .or_else(|| fallback().map(|path| (path, false)))
        .and_then(|(path, from_path)| {
            std::path::absolute(path)
                .ok()
                .map(|path| Executable { path, from_path })
        })
}

#[cfg(windows)]
fn executable() -> Option<&'static Executable> {
    static EXECUTABLE: LazyLock<Option<Executable>> = LazyLock::new(|| {
        find_executable(env::var_os("PATH").as_deref(), || {
            alternative_executable().map(ToOwned::to_owned)
        })
    });
    EXECUTABLE.as_ref()
}

/// Keep the selected path and its origin without reverse lookup through unused fallbacks.
#[cfg(windows)]
pub(super) fn exe_path() -> Option<&'static Path> {
    executable().map(|exe| exe.path.as_path())
}

#[cfg(windows)]
pub(super) fn exe_invocation() -> &'static Path {
    executable().map_or(Path::new(EXE_NAME), Executable::invocation)
}

/// MSYS Git uses Unix paths. Bind conversion and companion lookup to its own runtime.
#[cfg(windows)]
pub(super) fn msys_bin() -> Option<&'static Path> {
    msys_bin_at(exe_path()?)
}

#[cfg(windows)]
fn msys_bin_at(executable: &Path) -> Option<&Path> {
    let bin = executable.parent()?;
    let mut components = bin.components().rev();
    let is_msys_bin = [b"bin", b"usr"].into_iter().all(|name| {
        components
            .next()
            .is_some_and(|component| component.as_os_str().as_encoded_bytes().eq_ignore_ascii_case(name))
    });
    (bin.is_absolute() && is_msys_bin && bin.join("msys-2.0.dll").is_file() && bin.join("cygpath.exe").is_file())
        .then_some(bin)
}

#[cfg(windows)]
pub(super) const EXE_NAME: &str = "git.exe";
#[cfg(not(windows))]
pub(super) const EXE_NAME: &str = "git";

#[derive(Debug, Default, Eq, PartialEq)]
struct ConfigPaths {
    installation: Option<BString>,
    installation_is_system: bool,
    system: Option<BString>,
}

/// Cache conventional configuration locations without executing Git. Environment overrides of
/// configuration files and helper paths are intentionally applied by callers, not discovery.
static GIT_CONFIG_PATHS: LazyLock<ConfigPaths> = LazyLock::new(|| {
    std::env::current_dir()
        .ok()
        .map(|cwd| config_paths_from_environment(|key| std::env::var_os(key), &cwd))
        .unwrap_or_default()
});

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
    if let Some(prefix) = super::system_prefix_from_exepath_var(&mut environment) {
        prefixes.push(prefix);
    }
    if prefixes.is_empty() {
        if let Some(directory) = directories.iter().find(|directory| directory.join(EXE_NAME).is_file()) {
            // Preserve a package manager's stable symlink prefix before trying the versioned
            // installation itself, where helpers may live but configuration usually does not.
            if let Some(prefix) = directory.parent() {
                prefixes.push(prefix.to_path_buf());
            }
            if let Ok(executable) = std::fs::canonicalize(directory.join(EXE_NAME))
                && let Some(prefix) = executable.parent().and_then(Path::parent)
                && !prefixes.iter().any(|existing| existing == prefix)
            {
                prefixes.push(prefix.to_path_buf());
            }
            #[cfg(windows)]
            if let Some(core) = super::installation::core_from_bin(directory)
                && let Some(prefix) = core.parent().and_then(Path::parent)
            {
                prefixes.insert(0, prefix.to_path_buf());
            }
        } else if let Some(core) = directories
            .iter()
            .find_map(|dir| super::installation::core_from_bin(dir))
            && let Some(prefix) = core.parent().and_then(Path::parent)
        {
            prefixes.push(prefix.to_path_buf());
        }
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
            .map(|prefix| super::config_path_from_system_prefix(prefix))
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
        environment("PROGRAMDATA")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .map(|path| path.join("Git/config"))
            .filter(|path| path.is_file())
    });
    let installation = installation.or_else(|| system.as_ref().filter(|path| cfg!(windows) || path.is_file()).cloned());
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

pub(super) fn system_config_path() -> Option<&'static BStr> {
    let _span = gix_trace::detail!("gix_path::git::system_config_path()");
    GIT_CONFIG_PATHS.system.as_ref().map(AsRef::as_ref)
}

/// Given `config_path` as obtained from `install_config_path()`, return the path of the git installation base.
pub(super) fn config_to_base_path(config_path: &Path) -> &Path {
    config_path
        .parent()
        .expect("config file paths always have a file name to pop")
}

#[cfg(test)]
mod tests;
