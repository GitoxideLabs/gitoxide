use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use std::sync::LazyLock;

mod git;

#[cfg(windows)]
#[path = "windows/mod.rs"]
mod platform;
#[cfg(not(windows))]
#[path = "non_windows.rs"]
mod platform;

/// Return the unoverridden location at which an installation-specific Git configuration file can be found, or
/// `None` if the binary could not be executed or its results could not be parsed.
///
/// The Git query ignores `GIT_CONFIG_SYSTEM` and `GIT_CONFIG_NOSYSTEM`; callers must apply these
/// variables according to their own environment-access policy.
///
/// ### Performance
///
/// This can invoke the Git binary. An unambiguous Git for Windows
/// installation identified by `EXEPATH`, or by `git.exe` in its `cmd` or `bin` directories,
/// avoids that invocation when its modern configuration file exists.
/// Prefixes derived from the selected executable are shared with core-directory, prefix, and shell discovery.
/// Registered MSYS2 Git packages use their known configuration path, translated by `cygpath`.
/// Their core and configuration paths share a single cached conversion.
/// On Unix, the first executable in `PATH` is resolved through symlinks. Known Homebrew Cellar
/// layouts, Linux `/usr/bin/git` and `/bin/git`, and direct Apple developer binaries avoid the query
/// when their expected configuration file exists. Apple's `/usr/bin/git` shim still queries Git,
/// leaving developer-directory and toolchain selection to the shim.
/// Unknown layouts, relative `PATH` entries, and restricted execute bits also query Git.
/// Indirect Apple Git invocations also query Git, as their config can differ from the symlink target's.
/// These shortcuts assume the recognized installations use their standard build-time config paths.
pub fn installation_config() -> Option<&'static Path> {
    git::install_config_path().and_then(|p| crate::from_byte_slice(p).ok())
}

/// Return whether [`installation_config()`] was reported with system scope by Git.
///
/// `GIT_CONFIG_SYSTEM` replaces system-scoped installation configuration, but not installation
/// configuration reported with another scope, such as Apple Git's `unknown` scope.
/// Returns `false` if no installation path was found or Git was too old to report scopes.
pub fn installation_config_is_system() -> bool {
    git::install_config_is_system()
}

/// Return the location at which git installation specific configuration files are located, or `None` if the binary
/// could not be executed or its results could not be parsed.
///
/// ### Performance
///
/// This shares the discovery and caching of [`installation_config()`].
pub fn installation_config_prefix() -> Option<&'static Path> {
    installation_config().map(git::config_to_base_path)
}

/// Return the unoverridden location of the system-wide Git configuration file.
///
/// On Windows this shares the single Git invocation used by [`installation_config()`].
/// The caller is responsible for applying `GIT_CONFIG_SYSTEM` and `GIT_CONFIG_NOSYSTEM`.
pub fn system_config() -> Option<&'static Path> {
    platform::system_config()
}

/// Return the shell that Git would use, the shell to execute commands from.
///
/// On Windows, this is the full path to `sh.exe` bundled with Git for Windows if we can find it.
/// If the bundled shell on Windows cannot be found, `sh.exe` is returned as the name of a shell,
/// as it could possibly be found in `PATH`. On Unix it's `/bin/sh` as the POSIX-compatible shell.
///
/// Use [`shell_command()`] when constructing a command. On Windows, it passes `--posix` to the
/// Git for Windows `bin/sh.exe` shim so its delegated `bash.exe` behaves like `sh`.
///
/// Note that the returned path might not be a path on disk, if it is a fallback path or if the
/// file was moved or deleted since the first time this function is called.
pub fn shell() -> &'static OsStr {
    &shell_configuration().program
}

struct Shell {
    program: OsString,
    /// Whether `program` requires `--posix` to behave like `sh`.
    needs_posix_mode: bool,
}

fn shell_configuration() -> &'static Shell {
    static SHELL: LazyLock<Shell> = LazyLock::new(platform::shell);
    &SHELL
}

/// Return a command configured to run the shell that Git would use.
///
/// On Windows, when [`shell()`] selected the Git for Windows `sh.exe` shim, this also requests
/// POSIX mode explicitly. The shim delegates to `bash.exe`, which otherwise behaves as Bash
/// rather than as `sh`. Other shells, including a caller-provided shell, must not receive this
/// Bash-specific option.
pub fn shell_command() -> std::process::Command {
    let shell = shell_configuration();
    let mut command = std::process::Command::new(&shell.program);
    if shell.needs_posix_mode {
        command.arg("--posix");
    }
    command
}

/// Return the name of the Git executable to invoke it.
///
/// If it's in the `PATH`, it will always be a short name.
///
/// Note that on Windows, we will find the executable in the `PATH` if it exists there, or search it
/// in alternative locations which when found yields the full path to it.
/// Fallbacks prioritize cheap probes, then expected installer prevalence. Fixed paths come first:
/// standard Git for Windows locations, `GIT_INSTALL_ROOT/cmd`, WinGet's user and machine `Links`
/// directories, `ChocolateyToolsLocation/git/cmd`, Scoop's user and global `current` installations,
/// the active Conda/Mamba environment's `CONDA_PREFIX/Library/bin`, and MSYS2's
/// `GIT_INSTALL_ROOT/usr/bin`. Next come direct Git for Windows registry registrations, including
/// custom roots. Directory scans for Scoop installations without junctions follow, with the broader
/// MSYS2 uninstall-registry scan last. An executable selected by `PATH` always takes precedence.
///
/// Scoop uses `SCOOP`/`SCOOP_GLOBAL`, defaulting to `USERPROFILE/scoop` and `ProgramData/scoop`.
/// When Scoop's `current` junction is absent, its most recently installed version is used.
/// Installation roots must be absolute paths. Registry discovery prefers current-user
/// registrations to machine registrations.
/// Fallback sources are inspected only when earlier locations do not contain Git, and the
/// selected executable is cached for the lifetime of the process.
/// File symlinks, including WinGet's portable links, are resolved for layout discovery while
/// retaining the selected invocation.
/// Plain Scoop shims are resolved through their `.shim` sidecars; shims with arguments or
/// unsupported metadata still require querying Git.
pub fn exe_invocation() -> &'static Path {
    platform::exe_invocation()
}

/// Returns the fully qualified path in the *xdg-home* directory (or equivalent in the home dir) to
/// `file`, accessing `env_var(<name>)` to learn where these bases are.
///
/// Note that the `HOME` directory should ultimately come from [`home_dir()`] as it handles Windows
/// correctly. The same can be achieved by using [`var()`] as `env_var`.
pub fn xdg_config(file: &str, env_var: &mut dyn FnMut(&str) -> Option<OsString>) -> Option<PathBuf> {
    env_var("XDG_CONFIG_HOME")
        .map(|home| {
            let mut p = PathBuf::from(home);
            p.push("git");
            p.push(file);
            p
        })
        .or_else(|| {
            env_var("HOME").map(|home| {
                let mut p = PathBuf::from(home);
                p.push(".config");
                p.push("git");
                p.push(file);
                p
            })
        })
}

static GIT_CORE_DIR: LazyLock<Option<PathBuf>> = LazyLock::new(platform::core_dir);

/// Return the directory that `git --exec-path` would report.
///
/// Known Git for Windows layouts avoid starting Git when their core directory exists and
/// `GIT_EXEC_PATH` is unset. Other installations and overrides are queried from Git.
/// Registered MSYS2 Git packages also avoid the Git query, translating their known core path
/// through the same installation's `cygpath` to respect custom mounts.
/// This conversion is shared with configuration discovery.
/// Returns `None` if Git could not be found, returned an error, or its path could not be translated.
pub fn core_dir() -> Option<&'static Path> {
    GIT_CORE_DIR.as_deref()
}

/// Return the path at which the Git-provided program with bare `name` resides within the [`core_dir()`],
/// or `None` if it doesn't exist there or if Git could not be found.
///
/// This is the location `git` itself uses to find the programs implementing its subcommands, and is
/// useful to invoke programs like `git-upload-pack` that are shipped with Git but aren't necessarily
/// present in `PATH`.
///
/// Use [`installation_program()`] to also search Git for Windows' wider installation for bundled
/// programs such as `sh` and `vim`.
///
/// Note that installations differ in which programs they provide as separate executables - builds
/// with `SKIP_DASHED_BUILT_INS`, like Git for Windows, omit programs for builtin subcommands, which
/// can then still be run through `git` itself.
pub fn core_dir_program(name: &str) -> Option<PathBuf> {
    if !is_bare_program_name(name) {
        return None;
    }
    let path = core_dir()?.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    path.is_file().then_some(path)
}

/// Return the path at which a program distributed with Git resides, or `None` if it cannot be found.
///
/// Unlike [`core_dir_program()`], which searches only [`core_dir()`], this also searches the `bin`
/// and `usr/bin` directories of the Git for Windows installation on Windows. These contain programs
/// such as `sh`, `vim`, and, in some versions, `vi`.
///
/// Only a bare program `name` without path separators is accepted.
pub fn installation_program(name: &str) -> Option<PathBuf> {
    if !is_bare_program_name(name) {
        return None;
    }

    core_dir_program(name).or_else(|| platform::associated_program(name))
}

fn is_bare_program_name(name: &str) -> bool {
    let mut components = Path::new(name).components();
    matches!(components.next(), Some(std::path::Component::Normal(_))) && components.next().is_none()
}

/// Returns the platform dependent system prefix or `None` if it cannot be found (right now only on Windows).
///
/// On Windows this is Git's runtime prefix, such as `C:/Program Files/Git/ucrt64` or
/// `C:/Program Files/Git/mingw64`. Git for Windows keeps its system configuration and attributes
/// in the sibling `etc` directory. On other platforms this returns `/`.
///
/// ### Performance
///
/// On Windows, the slowest part is the launch of the Git executable in the PATH. This is often
/// avoided by inspecting the environment, when launched from inside a Git Bash MSYS2 shell,
/// or by finding Git for Windows' `cmd` or `bin` directories in the PATH, including the
/// launchers installed by Chocolatey portable and Conda/Mamba.
///
/// ### When `None` is returned
///
/// This happens only Windows if the git binary can't be found at all for obtaining its executable
/// path, or if the git binary wasn't built with a well-known directory structure or environment.
pub fn system_prefix() -> Option<&'static Path> {
    platform::system_prefix()
}

/// Returns `$HOME` or `None` if it cannot be found.
#[cfg(target_family = "wasm")]
pub fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").map(PathBuf::from).ok()
}

/// Tries to obtain the home directory from `HOME` on all platforms, but falls back to
/// [`std::env::home_dir()`] for more complex ways of obtaining a home directory, particularly useful
/// on Windows.
///
/// The reason `HOME` is tried first is to allow Windows users to have a custom location for their
/// linux-style home, as otherwise they would have to accumulate dot files in a directory these are
/// inconvenient and perceived as clutter.
#[cfg(not(target_family = "wasm"))]
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(Into::into).or_else(std::env::home_dir)
}

/// Returns the contents of an environment variable of `name` with some special handling for
/// certain environment variables (like `HOME`) for platform compatibility.
pub fn var(name: &str) -> Option<OsString> {
    if name == "HOME" {
        home_dir().map(PathBuf::into_os_string)
    } else {
        std::env::var_os(name)
    }
}

#[cfg(test)]
mod tests;
