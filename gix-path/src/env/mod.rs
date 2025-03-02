use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use bstr::{BString, ByteSlice};
use once_cell::sync::Lazy;

use crate::env::git::EXE_NAME;

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
    static PATH: Lazy<OsString> = Lazy::new(|| {
        if cfg!(windows) {
            const MSYS_PREFIX_NAMES: &[&str] = &[
                "mingw64",
                "mingw32",
                "clangarm64",
                "clang64",
                "clang32",
                "ucrt64",
                "usr",
            ];
            const RAW_SUFFIXES: &[&str] = &[
                "/bin/sh.exe", // Usually a shim, which currently we prefer, if available.
                "/usr/bin/sh.exe",
            ];
            fn raw_join(path: &Path, raw_suffix: &str) -> OsString {
                let mut raw_path = OsString::from(path);
                raw_path.push(raw_suffix);
                raw_path
            }
            core_dir()
                .filter(|core| core.is_absolute() && core.ends_with("libexec/git-core"))
                .and_then(|core| core.ancestors().nth(2))
                .filter(|prefix| {
                    // Only use `libexec/git-core` from inside something `usr`-like, such as `mingw64`.
                    MSYS_PREFIX_NAMES.iter().any(|name| prefix.ends_with(name))
                })
                .and_then(|prefix| prefix.parent())
                .into_iter()
                .flat_map(|git_root| {
                    // Enumerate the locations where `sh.exe` usually is. To avoid breaking shell
                    // scripts that assume the shell's own path contains no `\`, and to produce
                    // more readable messages, append literally with `/` separators. The path from
                    // `git --exec-path` will already have all `/` separators (and no trailing `/`)
                    // unless it was explicitly overridden to an unusual value via `GIT_EXEC_PATH`.
                    RAW_SUFFIXES.iter().map(|raw_suffix| raw_join(git_root, raw_suffix))
                })
                .find(|raw_path| Path::new(raw_path).is_file())
                .unwrap_or_else(|| "sh.exe".into())
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
        static EXECUTABLE_PATH: Lazy<Option<PathBuf>> = Lazy::new(|| {
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

static GIT_CORE_DIR: Lazy<Option<PathBuf>> = Lazy::new(|| {
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
        static PREFIX: Lazy<Option<PathBuf>> = Lazy::new(|| {
            if let Some(root) = std::env::var_os("EXEPATH").map(PathBuf::from) {
                for candidate in ["mingw64", "mingw32"] {
                    let candidate = root.join(candidate);
                    if candidate.is_dir() {
                        return Some(candidate);
                    }
                }
            }

            let path = GIT_CORE_DIR.as_deref()?;
            let one_past_prefix = path.components().enumerate().find_map(|(idx, c)| {
                matches!(c,std::path::Component::Normal(name) if name.to_str() == Some("libexec")).then_some(idx)
            })?;
            Some(path.components().take(one_past_prefix.checked_sub(1)?).collect())
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
/// [`home::home_dir()`] for more complex ways of obtaining a home directory, particularly useful
/// on Windows.
///
/// The reason `HOME` is tried first is to allow Windows users to have a custom location for their
/// linux-style home, as otherwise they would have to accumulate dot files in a directory these are
/// inconvenient and perceived as clutter.
#[cfg(not(target_family = "wasm"))]
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(Into::into).or_else(home::home_dir)
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
