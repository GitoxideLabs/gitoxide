use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use std::sync::LazyLock;

mod auxiliary;
mod git;
mod installation;
pub use installation::core_dir_from_environment;

/// Return the unoverridden configuration path of a conventional Git installation.
///
/// Discovery uses `PATH`, filesystem layouts, and on Windows `EXEPATH` and program-files
/// locations. No executable is invoked. Unknown compile-time layouts require callers to supply
/// configuration paths explicitly. Results are cached on first use.
///
/// `GIT_EXEC_PATH`, `GIT_CONFIG_SYSTEM`, and `GIT_CONFIG_NOSYSTEM` do not affect discovery;
/// callers must apply configuration overrides according to their own environment-access policy.
pub fn installation_config() -> Option<&'static Path> {
    git::install_config_path().and_then(|p| crate::from_byte_slice(p).ok())
}

/// Return whether [`installation_config()`] has system scope.
///
/// `GIT_CONFIG_SYSTEM` replaces system-scoped installation configuration, but not installation
/// configuration reported with another scope, such as Apple Git's `unknown` scope.
/// Returns `false` if no installation path was found.
pub fn installation_config_is_system() -> bool {
    git::install_config_is_system()
}

/// Return the directory containing [`installation_config()`], or `None` if no installation was found.
///
/// ### Performance
///
/// This shares the discovery and caching of [`installation_config()`].
pub fn installation_config_prefix() -> Option<&'static Path> {
    installation_config().map(git::config_to_base_path)
}

/// Return the unoverridden location of the system-wide Git configuration file.
///
/// This shares native filesystem discovery and caching with [`installation_config()`].
/// The caller is responsible for applying `GIT_CONFIG_SYSTEM` and `GIT_CONFIG_NOSYSTEM`.
pub fn system_config() -> Option<&'static Path> {
    git::system_config_path().and_then(|p| crate::from_byte_slice(p).ok())
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
    static SHELL: LazyLock<Shell> = LazyLock::new(|| {
        if cfg!(windows) {
            let shell = auxiliary::find_git_associated_windows_executable_with_fallback("sh");
            Shell {
                program: shell.program,
                needs_posix_mode: shell.is_shim,
            }
        } else {
            Shell {
                program: "/bin/sh".into(),
                needs_posix_mode: false,
            }
        }
    });
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
/// These include standard Git for Windows installations and Scoop's user and global installations
/// (`SCOOP`/`SCOOP_GLOBAL`, defaulting to `USERPROFILE/scoop` and `ProgramData/scoop`), as well as
/// `GIT_INSTALL_ROOT`. Installation roots must be absolute paths.
/// When Scoop's `current` junction is absent, its most recently installed version is used.
/// Also searched are `ChocolateyToolsLocation/git/cmd`, the active Conda/Mamba environment's
/// `CONDA_PREFIX/Library/bin`, and WinGet's user and machine `Links` directories. For an MSYS2
/// installation outside `PATH`, `GIT_INSTALL_ROOT` can name its root (containing `usr/bin`).
/// Finally, registered Git for Windows and MSYS2 installations are searched, including custom
/// roots, preferring current-user registrations to machine registrations.
/// Fallback sources are inspected only when earlier locations do not contain Git, and the
/// selected executable is cached for the lifetime of the process.
pub fn exe_invocation() -> &'static Path {
    #[cfg(windows)]
    {
        git::exe_invocation()
    }
    #[cfg(not(windows))]
    {
        Path::new("git")
    }
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

static GIT_CORE_DIR: LazyLock<Option<PathBuf>> =
    LazyLock::new(|| core_dir_from_environment(|name| std::env::var_os(name), &std::env::current_dir().ok()?));

/// Return Git's helper directory using cached environment and filesystem installation discovery.
///
/// No command is invoked. See [`core_dir_from_environment()`] for the search order and support for
/// custom layouts through `GIT_EXEC_PATH`.
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

    core_dir_program(name).or_else(|| {
        if cfg!(windows) {
            auxiliary::find_git_associated_windows_executable(name).map(|executable| executable.program.into())
        } else {
            None
        }
    })
}

fn is_bare_program_name(name: &str) -> bool {
    let mut components = Path::new(name).components();
    matches!(components.next(), Some(std::path::Component::Normal(_))) && components.next().is_none()
}

fn system_prefix_from_core_dir<'a, F>(core_dir_func: F) -> Option<PathBuf>
where
    F: Fn() -> Option<&'a Path>,
{
    core_dir_func()?
        .ancestors()
        .find(|path| path.file_name() == Some(OsStr::new("libexec")) || (cfg!(windows) && path.ends_with("usr/lib")))?
        .parent()
        .filter(|prefix| !prefix.as_os_str().is_empty())
        .map(Path::to_path_buf)
}

fn system_prefix_from_exepath_var<F>(mut var_os_func: F) -> Option<PathBuf>
where
    F: FnMut(&str) -> Option<OsString>,
{
    // Only attempt this optimization if the `EXEPATH` variable is set to an absolute path.
    let root = var_os_func("EXEPATH").map(PathBuf::from).filter(|r| r.is_absolute())?;

    let mut candidates = ["clangarm64", "ucrt64", "mingw64", "mingw32"]
        .iter()
        .map(|component| root.join(component))
        .filter(|candidate| candidate.is_dir());

    let path = candidates.next()?;
    match candidates.next() {
        Some(_) => None, // Multiple plausible candidates, so don't use the `EXEPATH` optimization.
        None => Some(path),
    }
}

/// Return Git's conventional system configuration path relative to its runtime `prefix`.
///
/// Git for Windows builds this path as `../etc/gitconfig`, beside its runtime directory. On Unix,
/// [`system_prefix()`] is `/`, yielding `/etc/gitconfig`. This is used by the `EXEPATH` shortcut
/// and native installation discovery.
fn config_path_from_system_prefix(prefix: &Path) -> PathBuf {
    let prefix = if cfg!(windows) {
        prefix.parent().unwrap_or(prefix)
    } else {
        prefix
    };
    prefix.join("etc/gitconfig")
}

/// Returns the platform dependent system prefix or `None` if it cannot be found (right now only on Windows).
///
/// On Windows this is Git's runtime prefix, such as `C:/Program Files/Git/ucrt64` or
/// `C:/Program Files/Git/mingw64`. Git for Windows keeps its system configuration and attributes
/// in the sibling `etc` directory. On other platforms this returns `/`.
///
/// Discovery only reads environment and conventional filesystem layouts; no Git executable is invoked.
/// Returns `None` on Windows if no conventional runtime directory can be identified.
pub fn system_prefix() -> Option<&'static Path> {
    if cfg!(windows) {
        static PREFIX: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
            system_prefix_from_exepath_var(|key| std::env::var_os(key)).or_else(|| {
                let core = core_dir_from_environment(
                    |key| (key != "GIT_EXEC_PATH").then(|| std::env::var_os(key)).flatten(),
                    &std::env::current_dir().ok()?,
                );
                system_prefix_from_core_dir(|| core.as_deref())
            })
        });
        PREFIX.as_deref()
    } else {
        Path::new("/").into()
    }
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
