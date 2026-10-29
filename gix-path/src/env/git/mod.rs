use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use bstr::{BStr, BString, ByteSlice};
use std::sync::LazyLock;

#[cfg(windows)]
mod registry;

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
fn locations_under_program_files<F>(var_os_func: F) -> Vec<PathBuf>
where
    F: Fn(&str) -> Option<std::ffi::OsString>,
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
fn locations_under_scoop(var_os: impl Fn(&str) -> Option<std::ffi::OsString>) -> impl Iterator<Item = PathBuf> {
    let absolute = move |name| var_os(name).map(PathBuf::from).filter(|path| path.is_absolute());
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
fn locations_under_package_managers(
    var_os: impl Fn(&str) -> Option<std::ffi::OsString>,
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

/// Convert paths emitted by MSYS Git to paths usable by native Windows callers.
pub(super) fn native_path(path: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    return native_path_with(path, msys_bin);
    #[cfg(not(windows))]
    Some(path.to_owned())
}

#[cfg(windows)]
fn native_path_with<'a>(path: &Path, msys_bin: impl FnOnce() -> Option<&'a Path>) -> Option<PathBuf> {
    if path.is_absolute() || !path.as_os_str().as_encoded_bytes().starts_with(b"/") {
        return Some(path.to_owned());
    }
    if let Some(bin) = msys_bin() {
        use std::os::windows::process::CommandExt;
        let output = Command::new(bin.join("cygpath.exe"))
            .args(["-am", "--"])
            .arg(path)
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let path = output.stdout.strip_suffix(b"\n")?.to_path().ok()?.to_owned();
        return path.is_absolute().then_some(path);
    }
    Some(path.to_owned())
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

/// Obtain and cache the installation and system configuration paths.
///
/// An unambiguous `EXEPATH` identifies Git for Windows' conventional configuration location.
/// Otherwise, query the Git executable found in `PATH` or an alternative location.
static GIT_CONFIG_PATHS: LazyLock<ConfigPaths> = LazyLock::new(|| {
    #[cfg(windows)]
    if let Some(system_prefix) = super::system_prefix_from_exepath_var(|key| std::env::var_os(key)) {
        let system_config =
            crate::os_string_into_bstring(super::config_path_from_system_prefix(&system_prefix).into()).ok();
        return ConfigPaths {
            installation: system_config.clone(),
            installation_is_system: true,
            system: system_config,
        };
    }
    config_paths_from_executable()
});

// There are a number of ways to refer to the null device on Windows, but they are not all equally
// well supported. Git for Windows rejects `\\.\NUL` and `\\.\nul`. On Windows 11 ARM64 (and maybe
// some others), it rejects even the legacy name `NUL`, when capitalized. But it always accepts the
// lower-case `nul`, handling it in various path checks, some of which are done case-sensitively.
#[cfg(windows)]
const NULL_DEVICE: &str = "nul";
#[cfg(not(windows))]
const NULL_DEVICE: &str = "/dev/null";

fn config_paths_from_executable() -> ConfigPaths {
    let executable = PathBuf::from(EXE_NAME);
    match config_paths_from_executable_at(executable) {
        Ok(paths) => paths,
        #[cfg(windows)]
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => alternative_executable()
            .and_then(|executable| config_paths_from_executable_at(executable.to_owned()).ok())
            .unwrap_or_default(),
        Err(_) => ConfigPaths::default(),
    }
}

fn config_paths_from_executable_at(executable: PathBuf) -> std::io::Result<ConfigPaths> {
    let mut cmd = git_cmd(executable.clone(), true);
    gix_trace::debug!(cmd = ?cmd, "invoking git for configuration paths");
    let output = cmd.output()?;
    let native = |path: &BStr| {
        native_path(crate::from_byte_slice(path).ok()?).and_then(|path| crate::os_string_into_bstring(path.into()).ok())
    };

    if !output.status.success() {
        let mut cmd = git_cmd(executable, false);
        gix_trace::debug!(cmd = ?cmd, "invoking git for configuration paths (pre Git 2.26)");
        let output = cmd.output()?;
        return Ok(ConfigPaths {
            installation: first_file_from_config_with_origin(output.stdout.as_slice().into()).and_then(native),
            ..Default::default()
        });
    }

    let (installation, installation_is_system, system) =
        config_paths_from_config_with_origin(output.stdout.as_slice().into());
    Ok(ConfigPaths {
        installation: installation.and_then(native),
        installation_is_system,
        system: system.and_then(native),
    })
}

fn git_cmd(executable: PathBuf, show_scope: bool) -> Command {
    let mut cmd = Command::new(executable);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    // We will try to run `git` from a location fairly high in the filesystem, in the hope it may
    // be faster if we are deeply nested, on a slow disk, or in a directory that has been deleted.
    let cwd = if cfg!(windows) {
        // We try the Windows directory (usually `C:\Windows`) first. It is given by `SystemRoot`,
        // except in rare cases where our own parent has not passed down that environment variable.
        env::var_os("SystemRoot")
            .or_else(|| env::var_os("windir"))
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(env::temp_dir)
    } else {
        "/".into()
    };
    // We can't use `--system` here, because scopes treated higher than the
    // system scope are possible. This commonly happens on macOS with Apple Git, where the config
    // file under `/Library` or `/Applications` is shown as an "unknown" scope but takes precedence
    // over the system scope. Although `GIT_CONFIG_NOSYSTEM` suppresses this scope along with the
    // system scope, passing `--system` selects only the system scope and not this "unknown" scope.
    cmd.args(["config", "-lz", "--show-origin", "--no-includes"]);
    if show_scope {
        cmd.arg("--show-scope");
    }
    cmd.arg("--name-only")
        .current_dir(cwd)
        .env_remove("GIT_CONFIG")
        .env_remove("GIT_DISCOVERY_ACROSS_FILESYSTEM")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_CONFIG_SYSTEM")
        .env_remove("GIT_CONFIG_NOSYSTEM")
        .env_remove("GIT_CONFIG_COUNT")
        .env_remove("GIT_CONFIG_PARAMETERS")
        // Discover stable, unoverridden paths; callers apply the configuration environment they permit.
        .env("GIT_CONFIG_GLOBAL", NULL_DEVICE)
        .env("GIT_DIR", NULL_DEVICE) // Avoid getting local-scope config.
        .env("GIT_WORK_TREE", NULL_DEVICE) // Avoid confusion when debugging.
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    cmd
}

fn first_file_from_config_with_origin(source: &BStr) -> Option<&BStr> {
    let file = source.strip_prefix(b"file:")?;
    let end_pos = file.find_byte(b'\0')?;
    file[..end_pos].as_bstr().into()
}

/// Parse NUL-separated scope, origin, and key records produced by `git config --show-scope --show-origin`.
///
/// The first file-backed origin is the installation path, and the second return value indicates
/// whether it had system scope. The system path is the first system-scoped origin distinct from
/// the installation path, or the installation path itself when no distinct system origin follows.
/// Non-file origins and incomplete records are ignored.
fn config_paths_from_config_with_origin(source: &BStr) -> (Option<&BStr>, bool, Option<&BStr>) {
    let mut fields = source.split(|byte| *byte == 0);
    let mut installation = None;
    let mut installation_is_system = false;
    let mut system = None;
    while let (Some(scope), Some(origin), Some(_key)) = (fields.next(), fields.next(), fields.next()) {
        let Some(path) = origin.strip_prefix(b"file:").map(ByteSlice::as_bstr) else {
            continue;
        };
        let is_installation = installation.is_none();
        let installation_path = *installation.get_or_insert(path);
        if is_installation {
            installation_is_system = scope == b"system";
        }
        if scope == b"system" && system.is_none_or(|current| current == installation_path) {
            system = Some(path);
        }
    }
    (installation, installation_is_system, system)
}

/// Try to find the file that contains Git configuration coming with the Git installation.
///
/// This returns the configuration associated with the `git` executable found in the current `PATH`
/// or an alternative location, or `None` if no `git` executable was found or there were other
/// errors during execution.
pub(super) fn install_config_path() -> Option<&'static BStr> {
    let _span = gix_trace::detail!("gix_path::git::install_config_path()");
    GIT_CONFIG_PATHS.installation.as_ref().map(AsRef::as_ref)
}

pub(super) fn install_config_is_system() -> bool {
    GIT_CONFIG_PATHS.installation_is_system
}

pub(super) fn system_config_path() -> Option<&'static BStr> {
    let _span = gix_trace::detail!("gix_path::git::system_config_path()");
    static FALLBACK: LazyLock<Option<BString>> = LazyLock::new(|| {
        super::system_prefix()
            .map(super::config_path_from_system_prefix)
            .and_then(|path| crate::os_string_into_bstring(path.into()).ok())
    });
    GIT_CONFIG_PATHS
        .system
        .as_ref()
        .or_else(|| FALLBACK.as_ref())
        .map(AsRef::as_ref)
}

/// Given `config_path` as obtained from `install_config_path()`, return the path of the git installation base.
pub(super) fn config_to_base_path(config_path: &Path) -> &Path {
    config_path
        .parent()
        .expect("config file paths always have a file name to pop")
}

#[cfg(test)]
mod tests;
