use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use bstr::{BStr, BString, ByteSlice};
use std::sync::LazyLock;

/// Other places to find Git in.
#[cfg(windows)]
pub(super) static ALTERNATIVE_LOCATIONS: LazyLock<Vec<PathBuf>> =
    LazyLock::new(|| locations_under_program_files(|key| std::env::var_os(key)));
#[cfg(not(windows))]
pub(super) static ALTERNATIVE_LOCATIONS: LazyLock<Vec<PathBuf>> = LazyLock::new(Vec::new);

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

    // 64-bit relative bin dirs. So far, this is always `mingw64` or `clangarm64`, not `urct64` or
    // `clang64`. We check `clangarm64` before `mingw64`, because in the strange case that both are
    // available, we don't want to skip over a native ARM64 executable for an emulated x86_64 one.
    let suffixes_64 = &[r"Git\clangarm64\bin", r"Git\mingw64\bin"][..];

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
pub(super) const EXE_NAME: &str = "git.exe";
#[cfg(not(windows))]
pub(super) const EXE_NAME: &str = "git";

#[derive(Debug, Default, Eq, PartialEq)]
struct ConfigPaths {
    installation: Option<BString>,
    system: Option<BString>,
}

/// Invoke the git executable to obtain the installation and system configuration paths, which are cached and returned.
///
/// The git executable is the one found in `PATH` or an alternative location.
static GIT_CONFIG_PATHS: LazyLock<ConfigPaths> = LazyLock::new(|| {
    #[cfg(windows)]
    if let Some(system_prefix) = super::system_prefix_from_exepath_var(|key| std::env::var_os(key)) {
        let installation_config = system_prefix
            .parent()
            .map(super::config_path_from_system_prefix)
            .and_then(|path| crate::os_string_into_bstring(path.into()).ok());
        let system_config =
            crate::os_string_into_bstring(super::config_path_from_system_prefix(&system_prefix).into()).ok();
        return ConfigPaths {
            installation: installation_config,
            system: system_config,
        };
    }
    let paths = config_paths_from_executable();
    #[cfg(windows)]
    if std::env::var_os("GIT_CONFIG_SYSTEM").is_some() || std::env::var_os("GIT_CONFIG_NOSYSTEM").is_some() {
        // The caller decides whether these overrides are permitted. Don't expose a path obtained
        // through ambient configuration when its environment accessor may intentionally hide it.
        return ConfigPaths { system: None, ..paths };
    }
    paths
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
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => ALTERNATIVE_LOCATIONS
            .iter()
            .find_map(|prefix| {
                let executable = prefix.join(EXE_NAME);
                executable.is_file().then_some(executable)
            })
            .and_then(|executable| config_paths_from_executable_at(executable).ok())
            .unwrap_or_default(),
        Err(_) => ConfigPaths::default(),
    }
}

fn config_paths_from_executable_at(executable: PathBuf) -> std::io::Result<ConfigPaths> {
    let mut cmd = git_cmd(executable.clone(), true);
    gix_trace::debug!(cmd = ?cmd, "invoking git for configuration paths");
    let output = cmd.output()?;

    if !output.status.success() {
        let mut cmd = git_cmd(executable, false);
        gix_trace::debug!(cmd = ?cmd, "invoking git for configuration paths (pre Git 2.26)");
        let output = cmd.output()?;
        return Ok(ConfigPaths {
            installation: first_file_from_config_with_origin(output.stdout.as_slice().into()).map(ToOwned::to_owned),
            ..Default::default()
        });
    }

    let (installation, system) = config_paths_from_config_with_origin(output.stdout.as_slice().into());
    Ok(ConfigPaths {
        installation: installation.map(ToOwned::to_owned),
        system: system.map(ToOwned::to_owned),
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
    cmd.args(["config", "-lz", "--show-origin"]);
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
/// The first file-backed origin is the installation path. The system path is the first system-scoped
/// origin distinct from it, or the installation path itself when no distinct system origin follows.
/// Non-file origins and incomplete records are ignored.
fn config_paths_from_config_with_origin(source: &BStr) -> (Option<&BStr>, Option<&BStr>) {
    let mut fields = source.split(|byte| *byte == 0);
    let mut installation = None;
    let mut system = None;
    while let (Some(scope), Some(origin), Some(_key)) = (fields.next(), fields.next(), fields.next()) {
        let Some(path) = origin.strip_prefix(b"file:").map(ByteSlice::as_bstr) else {
            continue;
        };
        let installation_path = *installation.get_or_insert(path);
        if scope == b"system" && system.is_none_or(|current| current == installation_path) {
            system = Some(path);
        }
    }
    (installation, system)
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
