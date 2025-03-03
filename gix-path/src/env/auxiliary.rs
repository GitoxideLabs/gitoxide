use std::ffi::OsString;
use std::path::Path;

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
/// Second, we don't recognize `usr` itself here, even though is a plausible prefix. In MSYS2, it
/// is the prefix for MSYS2 non-native programs, i.e. those that use `msys-2.0.dll`. But unlike the
/// `<platform>` names we recognize, `usr` also has an effectively unbounded range of plausible
/// meanings on non-Unix systems, which may occasionally relate to subdirectories whose contents
/// are controlled by different user accounts.
///
/// If we start with a `libexec/git-core` directory that we already use and trust, and it is in a
/// directory with a name like `mingw64`, we infer that this `mingw64` directory has the expected
/// meaning and that its `usr` sibling, if present, is acceptable to treat as though it is a
/// first-level directory inside an MSYS2-like tree. So we are willing to traverse down to
/// `usr/sh.exe` and attempt to use it. But if the `libexec/git-core` we use and trust is inside a
/// directory named `usr`, that `usr` directory may still not have the meaning we expect of `usr`.
///
/// The conditions for a privilege escalation attack or other serious malfunction seem unlikely. If
/// research indicates the risk is low enough, `usr` may be added. But for now it is omitted.
const MSYS_USR_VARIANTS: &[&str] = &["mingw64", "mingw32", "clangarm64", "clang64", "clang32", "ucrt64"];

/// Shell path fragments to concatenate to the root of a Git for Windows or MSYS2 installation.
///
/// These look like absolute Unix-style paths, but the leading `/` separators are present because
/// they simplify forming paths like `C:/Program Files/Git` obtained by removing trailing
/// components from the output of `git --exec-path`.
const RAW_SH_EXE_PATH_SUFFIXES: &[&str] = &[
    "/bin/sh.exe", // Usually a shim, which currently we prefer, if available.
    "/usr/bin/sh.exe",
];

///
fn raw_join(path: &Path, raw_suffix: &str) -> OsString {
    let mut raw_path = OsString::from(path);
    raw_path.push(raw_suffix);
    raw_path
}

///
pub(super) fn find_sh_on_windows() -> Option<OsString> {
    super::core_dir()
        .filter(|core| core.is_absolute() && core.ends_with("libexec/git-core"))
        .and_then(|core| core.ancestors().nth(2))
        .filter(|prefix| {
            // Only use `libexec/git-core` from inside something `usr`-like, such as `mingw64`.
            MSYS_USR_VARIANTS.iter().any(|name| prefix.ends_with(name))
        })
        .and_then(|prefix| prefix.parent())
        .into_iter()
        .flat_map(|git_root| {
            // Enumerate locations where `sh.exe` usually is. To avoid breaking scripts that assume the
            // shell's own path contains no `\`, and so messages are more readable, append literally
            // with `/` separators. The path from `git --exec-path` already uses `/` separators (and no
            // trailing `/`) unless explicitly overridden to an unusual value via `GIT_EXEC_PATH`.
            RAW_SH_EXE_PATH_SUFFIXES
                .iter()
                .map(|raw_suffix| raw_join(git_root, raw_suffix))
        })
        .find(|raw_path| Path::new(raw_path).is_file())
}
