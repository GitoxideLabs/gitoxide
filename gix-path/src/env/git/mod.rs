use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use bstr::{BStr, BString, ByteSlice};
use std::sync::LazyLock;

use super::platform::{self, EXE_NAME, NULL_DEVICE};

#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct ConfigPaths {
    pub(super) installation: Option<BString>,
    pub(super) installation_is_system: bool,
    pub(super) system: Option<BString>,
}

/// Obtain and cache the installation and system configuration paths.
///
/// An unambiguous `EXEPATH`, or `git.exe` in the `cmd` directory of an unambiguous installation,
/// identifies Git for Windows' modern configuration location if that file exists.
/// Known Unix installation layouts also avoid the query when their configuration file exists.
/// Otherwise, query the Git executable found in `PATH` or an alternative location.
pub(super) static GIT_CONFIG_PATHS: LazyLock<ConfigPaths> =
    LazyLock::new(|| platform::config_paths_without_git().unwrap_or_else(config_paths_from_executable));

/// Query the selected Git executable for its core directory without changing its working directory.
pub(super) fn core_dir_from_executable() -> Option<PathBuf> {
    let mut cmd = platform::command(super::exe_invocation());

    let output = cmd.arg("--exec-path").output().ok()?;

    if !output.status.success() {
        return None;
    }

    platform::native_path(BString::new(output.stdout).strip_suffix(b"\n")?.to_path().ok()?)
}

fn config_paths_from_executable() -> ConfigPaths {
    let executable = PathBuf::from(EXE_NAME);
    match config_paths_from_executable_at(executable) {
        Ok(paths) => paths,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => platform::alternative_executable()
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
        platform::native_path(crate::from_byte_slice(path).ok()?)
            .and_then(|path| crate::os_string_into_bstring(path.into()).ok())
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
    let mut cmd = platform::command(executable);
    // We will try to run `git` from a location fairly high in the filesystem, in the hope it may
    // be faster if we are deeply nested, on a slow disk, or in a directory that has been deleted.
    let cwd = platform::config_query_cwd();
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

/// Given `config_path` as obtained from `install_config_path()`, return the path of the git installation base.
pub(super) fn config_to_base_path(config_path: &Path) -> &Path {
    config_path
        .parent()
        .expect("config file paths always have a file name to pop")
}

#[cfg(test)]
mod tests;
