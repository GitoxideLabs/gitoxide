use std::{
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use bstr::{BString, ByteSlice};
use std::sync::LazyLock;

use crate::env::git::EXE_NAME;

mod auxiliary;
mod git;

/// Return the location at which installation specific git configuration file can be found, or `None`
/// if the binary could not be executed or its results could not be parsed.
///
/// ### Performance
///
/// This invokes the git binary which is slow on windows.
pub fn installation_config() -> Option<&'static Path> {
    git::install_config_path().and_then(|p| crate::try_from_byte_slice(p).ok())
}

/// Return the location at which git installation specific configuration files are located, or `None` if the binary
/// could not be executed or its results could not be parsed.
///
/// ### Performance
///
/// This invokes the git binary which is slow on windows.
pub fn installation_config_prefix() -> Option<&'static Path> {
    installation_config().map(git::config_to_base_path)
}

/// Return the shell that Git would use, the shell to execute commands from.
///
/// On Windows, this is the full path to `sh.exe` bundled with Git for Windows if we can find it.
/// If the bundled shell on Windows cannot be found, `sh.exe` is returned as the name of a shell,
/// as it could possibly be found in `PATH`. On Unix it's `/bin/sh` as the POSIX-compatible shell.
///
/// Note that the returned path might not be a path on disk, if it is a fallback path or if the
/// file was moved or deleted since the first time this function is called.
pub fn shell() -> &'static OsStr {
    static PATH: LazyLock<OsString> = LazyLock::new(|| {
        if cfg!(windows) {
            auxiliary::find_git_associated_windows_executable_with_fallback("sh")
        } else {
            "/bin/sh".into()
        }
    });
    PATH.as_ref()
}

/// Return the name of the Git executable to invoke it.
///
/// If it's in the `PATH`, it will always be a short name.
///
/// Note that on Windows, we will find the executable in the `PATH` if it exists there, or search it
/// in alternative locations which when found yields the full path to it.
pub fn exe_invocation() -> &'static Path {
    if cfg!(windows) {
        /// The path to the Git executable as located in the `PATH` or in other locations that it's
        /// known to be installed to. It's `None` if environment variables couldn't be read or if
        /// no executable could be found.
        static EXECUTABLE_PATH: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
            std::env::split_paths(&std::env::var_os("PATH")?)
                .chain(git::ALTERNATIVE_LOCATIONS.iter().map(Into::into))
                .find_map(|prefix| {
                    let full_path = prefix.join(EXE_NAME);
                    full_path.is_file().then_some(full_path)
                })
                .map(|exe_path| {
                    let is_in_alternate_location = git::ALTERNATIVE_LOCATIONS
                        .iter()
                        .any(|prefix| exe_path.strip_prefix(prefix).is_ok());
                    if is_in_alternate_location {
                        exe_path
                    } else {
                        EXE_NAME.into()
                    }
                })
        });
        EXECUTABLE_PATH.as_deref().unwrap_or(Path::new(git::EXE_NAME))
    } else {
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

static GIT_CORE_DIR: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    let mut cmd = std::process::Command::new(exe_invocation());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    let output = cmd.arg("--exec-path").output().ok()?;

    if !output.status.success() {
        return None;
    }

    BString::new(output.stdout)
        .strip_suffix(b"\n")?
        .to_path()
        .ok()?
        .to_owned()
        .into()
});

/// Return the directory obtained by calling `git --exec-path`.
///
/// Returns `None` if Git could not be found or if it returned an error.
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
/// Note that installations differ in which programs they provide as separate executables - builds
/// with `SKIP_DASHED_BUILT_INS`, like Git for Windows, omit programs for builtin subcommands, which
/// can then still be run through `git` itself.
pub fn core_dir_program(name: &str) -> Option<PathBuf> {
    let mut components = Path::new(name).components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_))) || components.next().is_some() {
        return None;
    }
    let path = core_dir()?.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    path.is_file().then_some(path)
}

fn system_prefix_from_core_dir<F>(core_dir_func: F) -> Option<PathBuf>
where
    F: Fn() -> Option<&'static Path>,
{
    let path = core_dir_func()?;
    let one_past_prefix = path.components().enumerate().find_map(|(idx, c)| {
        matches!(c,std::path::Component::Normal(name) if name.to_str() == Some("libexec")).then_some(idx)
    })?;
    Some(path.components().take(one_past_prefix.checked_sub(1)?).collect())
}

fn system_prefix_from_exepath_var<F>(var_os_func: F) -> Option<PathBuf>
where
    F: Fn(&str) -> Option<OsString>,
{
    // Only attempt this optimization if the `EXEPATH` variable is set to an absolute path.
    let root = var_os_func("EXEPATH").map(PathBuf::from).filter(|r| r.is_absolute())?;

    let mut candidates = ["clangarm64", "mingw64", "mingw32"]
        .iter()
        .map(|component| root.join(component))
        .filter(|candidate| candidate.is_dir());

    let path = candidates.next()?;
    match candidates.next() {
        Some(_) => None, // Multiple plausible candidates, so don't use the `EXEPATH` optimization.
        None => Some(path),
    }
}

/// Returns the platform dependent system prefix or `None` if it cannot be found (right now only on Windows).
///
/// ### Performance
///
/// On Windows, the slowest part is the launch of the Git executable in the PATH. This is often
/// avoided by inspecting the environment, when launched from inside a Git Bash MSYS2 shell.
///
/// ### When `None` is returned
///
/// This happens only Windows if the git binary can't be found at all for obtaining its executable
/// path, or if the git binary wasn't built with a well-known directory structure or environment.
pub fn system_prefix() -> Option<&'static Path> {
    if cfg!(windows) {
        static PREFIX: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
            system_prefix_from_exepath_var(|key| std::env::var_os(key))
                .or_else(|| system_prefix_from_core_dir(core_dir))
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
