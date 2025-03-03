use std::ffi::OsString;
use std::path::{Path, PathBuf};

use once_cell::sync::Lazy;

/// `usr`-like directory component names that MSYS2 may provide, other than for `/usr` itself.
///
/// These are the values of the "Prefix" column of the "Environments" and "Legacy Environments"
/// tables in the [MSYS2 Environments](https://www.msys2.org/docs/environments/) documentation,
/// with the leading `/` separator removed, except that this does not list `usr` itself.
///
/// On Windows, we prefer to use `sh` as provided by Git for Windows, when present. To find it, we
/// run `git --exec-path` to get a path that is usually `<platform>/libexec/git-core` in the Git
/// for Windows installation, where `<platform>` is something like `mingw64`. It is also acceptable
/// to find `sh` in an environment not provided by Git for Windows, such as an independent MSYS2
/// environment in which a `git` package has been installed. However, in an unusual installation,
/// or if the user has set a custom value of `GIT_EXEC_PATH`, the output of `git --exec-path` may
/// take a form other than `<platform>/libexec/git-core`, such that finding shell at a location
/// like `../../../bin/sh.exe` relative to it should not be attempted. We lower the risk by
/// checking that `<platform>` is a plausible value that is not likely to have any other meaning.
///
/// This involves two tradeoffs. First, it may be reasonable to find `sh.exe` in an environment
/// that is not MSYS2 at all, for which in principle the prefix could be different. But listing
/// more prefixes or matching a broad pattern of platform-like strings might be too broad. So only
/// prefixes that have been used in MSYS2 are considered.
///
/// Second, we don't recognize `usr` itself here, even though it is a plausible prefix. In MSYS2,
/// it is the prefix for MSYS2 non-native programs, i.e. those that use `msys-2.0.dll`. But unlike
/// the `<platform>` names we recognize, `usr` also has an effectively unbounded range of plausible
/// meanings on non-Unix systems (for example, what should we take `Z:\usr` to mean?), which might
/// occasionally relate to subdirectories with contents controlled by different *user accounts*.
///
/// If we start with a `libexec/git-core` directory that we already use and trust, and it is in a
/// directory with a name like `mingw64`, we infer that this `mingw64` directory has the expected
/// meaning and accordingly infer that its `usr` sibling, if present, is acceptable to treat as
/// though it is a first-level directory inside an MSYS2-like tree. So we are willing to traverse
/// down to `usr/sh.exe` and try to use it. But if the `libexec/git-core` we use and trust is in a
/// directory named `usr`, that `usr` directory may still not have the meaning we expect of `usr`.
///
/// Conditions for a privilege escalation attack or other serious malfunction seem far-fetched. If
/// further research finds the risk is low enough, `usr` may be added. But for now it is omitted.
const MSYS_USR_VARIANTS: &[&str] = &["mingw64", "mingw32", "clangarm64", "clang64", "clang32", "ucrt64"];

/// Find a Git for Windows installation directory based on `git --exec-path` output.
///
/// Currently this is used only for finding the path to an `sh.exe` associated with Git. This is
/// separate from `installation_config()` and `installation_config_prefix()` in `gix_path::env`.
/// This is *not* suitable for finding the highest-scoped configuration file, because that could be
/// installed in an unusual place, or customized via `GIT_CONFIG_SYSTEM` or `GIT_CONFIG_NOSYSTEM`,
/// all of which `installation_config()` should reflect. Likewise, `installation_config_prefix()`
/// has strong uses, such as to find a directory inside `ProgramData` containing configuration.
/// But it is possible that some marginal uses of `installation_config_prefix()`, if they do not
/// really relate to configuration, could be replaced with `git_for_windows_root()` in the future.
fn git_for_windows_root() -> Option<&'static Path> {
    static GIT_ROOT: Lazy<Option<PathBuf>> = Lazy::new(|| {
        super::core_dir()
            .filter(|core| {
                // Only use this if the directory structure resembles a Git installation. This
                // accepts installations of common types that are not broken when `GIT_EXEC_PATH`
                // is unset, as well as values of `GIT_EXEC_PATH` that are likely to be usable.
                core.is_absolute() && core.ends_with("libexec/git-core")
            })
            .and_then(|core| core.ancestors().nth(2))
            .filter(|prefix| {
                // Only use `libexec/git-core` from inside something `usr`-like, such as `mingw64`.
                // See `MSYS_USR_VARIANTS` for details and the rationale for this restriction.
                MSYS_USR_VARIANTS.iter().any(|name| prefix.ends_with(name))
            })
            .and_then(|prefix| prefix.parent())
            .map(Into::into)
    });
    GIT_ROOT.as_deref()
}

/// `bin` directory paths to try relative to the root of a Git for Windows or MSYS2 installation.
///
/// These are ordered so that a shim is preferred over a non-shim when they are tried in order.
const BIN_DIR_FRAGMENTS: &[&str] = &["bin", "usr/bin"];

/// Obtain a path to an executable command on Windows associated with Git, if one can be found.
///
/// The resulting path uses only `/` separators so long as the path obtained from `git --exec-path`
/// does, which is the case unless it is overridden by setting `GIT_EXEC_PATH` to an unusual value.
///
/// This is currently only used (and only heavily exercised in tests) for finding `sh.exe`. It may
/// be used to find other executables in the future, but may need adjustment. In particular,
/// depending on desired semantics, it should possibly also check a `cmd` directory; directories
/// like `<platform>/bin`, for any applicable variants (such as `mingw64`); and `super::core_dir()`
/// itself, which it could safely check even if its value is not safe for inferring other paths.
fn find_git_associated_windows_executable(stem: &str) -> Option<OsString> {
    let git_root = git_for_windows_root()?;

    BIN_DIR_FRAGMENTS
        .iter()
        .map(|bin_dir_fragment| {
            // Perform explicit raw concatenation with `/` to avoid introducing any `\` separators.
            let mut raw_path = OsString::from(git_root);
            raw_path.push("/");
            raw_path.push(bin_dir_fragment);
            raw_path.push("/");
            raw_path.push(stem);
            raw_path.push(".exe");
            raw_path
        })
        .find(|raw_path| Path::new(raw_path).is_file())
}

/// Like `find_associated_windows_executable`, but if not found, fall back to a simple filename.
pub(super) fn find_git_associated_windows_executable_with_fallback(stem: &str) -> OsString {
    find_git_associated_windows_executable(stem).unwrap_or_else(|| {
        let mut raw_path = OsString::from(stem);
        raw_path.push(".exe");
        raw_path
    })
}

#[cfg(all(windows, test))]
mod tests {
    use std::path::Path;

    /// Some commands with `.exe` files in `bin` and `usr/bin` that should be found.
    ///
    /// Tests are expected to run with a full Git for Windows installation (not MinGit).
    const SHOULD_FIND: &[&str] = &[
        "sh", "bash", "dash", "diff", "tar", "less", "sed", "awk", "perl", "cygpath",
    ];

    /// Shouldn't find anything nonexistent, or only in PATH or in `bin`s we don't mean to search.
    ///
    /// If dirs like `mingsw64/bin` are added, `git-credential-manager` should be moved to `SHOULD_FIND`.
    /// Likewise, if `super::core_dir()` is added, `git-daemon` should be moved to `SHOULD_FIND`.
    const SHOULD_NOT_FIND: &[&str] = &[
        "nonexistent-command",
        "cmd",
        "powershell",
        "explorer",
        "git-credential-manager",
        "git-daemon",
    ];

    #[test]
    fn find_git_associated_windows_executable() {
        for stem in SHOULD_FIND {
            let path = super::find_git_associated_windows_executable(stem);
            assert!(path.is_some(), "should find {stem:?}");
        }
    }

    #[test]
    fn find_git_associated_windows_executable_no_extra() {
        for stem in SHOULD_NOT_FIND {
            let path = super::find_git_associated_windows_executable(stem);
            assert_eq!(path, None, "should not find {stem:?}");
        }
    }

    #[test]
    fn find_git_associated_windows_executable_with_fallback() {
        for stem in SHOULD_FIND {
            let path = super::find_git_associated_windows_executable_with_fallback(stem);
            assert!(Path::new(&path).is_absolute(), "should find {stem:?}");
        }
    }

    #[test]
    fn find_git_associated_windows_executable_with_fallback_falls_back() {
        for stem in SHOULD_NOT_FIND {
            let path = super::find_git_associated_windows_executable_with_fallback(stem)
                .to_str()
                .expect("valid Unicode")
                .to_owned();
            assert_eq!(path, format!("{stem}.exe"), "should fall back for {stem:?}");
        }
    }
}
