use gix_error::{OptionExt, Result};
use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::{Component, Path, PathBuf},
};

use bstr::{BStr, BString};
#[cfg(not(unix))]
use gix_error::ResultExt;

#[cfg(not(unix))]
fn utf8_error() -> gix_error::Message {
    gix_error::validation("Could not convert to UTF8 or from UTF8 due to ill-formed input")
}

/// Like [`into_bstr()`], but takes `OsStr` as input for a lossless, but fallible, conversion.
pub fn os_str_into_bstr(path: &OsStr) -> Result<&BStr> {
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::ffi::OsStrExt;
        path.as_bytes()
    };
    #[cfg(not(unix))]
    let bytes = std::str::from_utf8(path.as_encoded_bytes())
        .or_raise(utf8_error)?
        .as_bytes();
    Ok(bytes.into())
}

/// Like [`into_bstr()`], but takes `OsString` as input for a lossless, but fallible, conversion.
pub fn os_string_into_bstring(path: OsString) -> Result<BString> {
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::ffi::OsStringExt;
        path.into_vec()
    };
    #[cfg(not(unix))]
    let bytes = String::from_utf8(path.into_encoded_bytes())
        .or_raise(utf8_error)?
        .into_bytes();
    Ok(bytes.into())
}

/// Like [`into_bstr()`], but takes `Cow<OsStr>` as input for a lossless, but fallible, conversion.
pub fn try_os_str_into_bstr(path: Cow<'_, OsStr>) -> Result<Cow<'_, BStr>> {
    match path {
        Cow::Borrowed(path) => os_str_into_bstr(path).map(Cow::Borrowed),
        Cow::Owned(path) => os_string_into_bstring(path).map(Cow::Owned),
    }
}

/// Convert the given path either into its raw bytes on Unix or its UTF-8 encoded counterpart on non-Unix platforms.
///
/// On non-Unix platforms, if the source `Path` contains ill-formed, lone surrogates, the UTF-8 conversion will fail
/// causing a [`gix_error::Error`] with the encoding failure as its source to be returned.
pub fn into_bstr<'a>(path: impl Into<Cow<'a, Path>>) -> Result<Cow<'a, BStr>> {
    match path.into() {
        Cow::Owned(path) => os_string_into_bstring(path.into_os_string()).map(Cow::Owned),
        Cow::Borrowed(path) => os_str_into_bstr(path.as_os_str()).map(Cow::Borrowed),
    }
}

/// Join `path` to `base` such that they are separated with a `/`, i.e. `base/path`.
pub fn join_bstr_unix_pathsep<'a, 'b>(base: impl Into<Cow<'a, BStr>>, path: impl Into<&'b BStr>) -> Cow<'a, BStr> {
    let mut base = base.into();
    if !base.is_empty() && base.last() != Some(&b'/') {
        base.to_mut().push(b'/');
    }
    base.to_mut().extend_from_slice(path.into());
    base
}

/// Given `input` bytes, produce a `Path` from them ignoring encoding entirely if on Unix.
///
/// On non-Unix platforms, invalid UTF-8 returns an error with the encoding failure as its source.
pub fn from_byte_slice(input: &[u8]) -> Result<&Path> {
    #[cfg(unix)]
    let p = {
        use std::os::unix::ffi::OsStrExt;
        OsStr::from_bytes(input).as_ref()
    };
    #[cfg(not(unix))]
    let p = Path::new(std::str::from_utf8(input).or_raise(utf8_error)?);
    Ok(p)
}

/// Similar to [`from_byte_slice()`], but takes either borrowed or owned `input`.
pub fn from_bstr<'a>(input: impl Into<Cow<'a, BStr>>) -> Result<Cow<'a, Path>> {
    let input = input.into();
    match input {
        Cow::Borrowed(input) => from_byte_slice(input).map(Cow::Borrowed),
        Cow::Owned(input) => from_bstring(input).map(Cow::Owned),
    }
}

/// Similar to [`from_bstr()`], but takes and produces owned data.
pub fn from_bstring(input: impl Into<BString>) -> Result<PathBuf> {
    let input = input.into();
    #[cfg(unix)]
    let p = {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(input.into()).into()
    };
    #[cfg(not(unix))]
    let p = PathBuf::from(String::from_utf8(input.into()).or_raise(utf8_error)?);
    Ok(p)
}

fn replace<'a>(path: impl Into<Cow<'a, BStr>>, find: u8, replace: u8) -> Cow<'a, BStr> {
    let path = path.into();
    match path {
        Cow::Owned(mut path) => {
            for b in path.iter_mut().filter(|b| **b == find) {
                *b = replace;
            }
            path.into()
        }
        Cow::Borrowed(path) => {
            if !path.contains(&find) {
                return path.into();
            }
            let mut path = path.to_owned();
            for b in path.iter_mut().filter(|b| **b == find) {
                *b = replace;
            }
            path.into()
        }
    }
}

/// Assures the given bytes use the native path separator.
pub fn to_native_separators<'a>(path: impl Into<Cow<'a, BStr>>) -> Cow<'a, BStr> {
    #[cfg(not(windows))]
    let p = to_unix_separators(path);
    #[cfg(windows)]
    let p = to_windows_separators(path);
    p
}

/// Convert paths with slashes to backslashes on Windows and do nothing on Unix,
/// returning an error for invalid UTF-8 on non-Unix platforms.
pub fn to_native_path_on_windows<'a>(path: impl Into<Cow<'a, BStr>>) -> Result<Cow<'a, std::path::Path>> {
    #[cfg(not(windows))]
    {
        crate::from_bstr(path)
    }
    #[cfg(windows)]
    {
        crate::from_bstr(to_windows_separators(path))
    }
}

/// Replace Windows path separators with slashes, but only do so on Windows.
pub fn to_unix_separators_on_windows<'a>(path: impl Into<Cow<'a, BStr>>) -> Cow<'a, BStr> {
    #[cfg(windows)]
    {
        to_unix_separators(path)
    }
    #[cfg(not(windows))]
    {
        path.into()
    }
}

/// Replace Windows path separators with slashes, which typically resembles a Unix path, unconditionally.
///
/// **Note** Do not use these and prefer the conditional versions of this method.
pub fn to_unix_separators<'a>(path: impl Into<Cow<'a, BStr>>) -> Cow<'a, BStr> {
    replace(path, b'\\', b'/')
}

/// Find slashes and replace them with backslashes, unconditionally.
///
/// **Note** Do not use these and prefer the conditional versions of this method.
pub fn to_windows_separators<'a>(path: impl Into<Cow<'a, BStr>>) -> Cow<'a, BStr> {
    replace(path, b'/', b'\\')
}

/// Resolve relative components virtually, eliminating intermediate `..` without accessing the filesystem.
///
/// For example, this turns `a/./b/c/.././..` into `a`, and turns `/a/../b/..` into `/`.
///
/// ```
/// # fn main() {
/// # use std::path::Path;
/// # use gix_path::normalize;
/// for (input, expected) in [
///     ("a/./b/c/.././..", "a"),
///     ("/a/../b/..", "/"),
///     ("/base/a/..", "/base"),
///     ("./a/..", "."),
///     ("./a/../..", "/"),
///     (".///", ".///"),
///     ("a//b", "a//b"),
///     ("/base/../base", "/base"),
/// ] {
///     let input = Path::new(input);
///     let expected = Path::new(expected);
///     assert_eq!(normalize(input.into(), Path::new("/cwd")), Some(expected.into()));
/// }
/// # }
/// ```
///
/// Leading `.` components as well as duplicate separators are left untouched.
///
/// This is particularly useful when manipulating paths that are based on user input, and not
/// resolving intermediate symlinks keeps the path similar to what the user provided. If that's not
/// desirable, use [`realpath()`](crate::realpath()) instead.
///
/// Note that we will use the `current_dir` if we run out of path components to pop off, which
/// is expected to be absolute as typical return value of `std::env::current_dir()` or
/// `gix_fs::current_dir(…)` when `core.precomposeUnicode` is known. As a `current_dir` like `/c`
/// can be exhausted by paths like `../../r`, `None` will be returned to indicate the inability to
/// produce a logically consistent path.
pub fn normalize<'a>(path: Cow<'a, Path>, current_dir: &Path) -> Option<Cow<'a, Path>> {
    normalize_inner(path, current_dir, false)
}

/// Like [`normalize()`], but treats `..` components beyond the filesystem root as no-ops.
/// Returns an error if normalization exhausts an empty or relative `current_dir`.
pub fn normalize_saturating<'a>(path: Cow<'a, Path>, current_dir: &Path) -> Result<Cow<'a, Path>> {
    normalize_inner(path, current_dir, true)
        .ok_or_raise(|| gix_error::validation("Cannot normalize a path after exhausting the current directory"))
}

fn normalize_inner<'a>(path: Cow<'a, Path>, current_dir: &Path, saturate_at_root: bool) -> Option<Cow<'a, Path>> {
    use std::path::Component::ParentDir;

    if !path.components().any(|c| matches!(c, ParentDir)) {
        return Some(path);
    }
    let mut current_dir_opt = Some(current_dir);
    let was_relative = path.is_relative();
    let components = path.components();
    let mut path = PathBuf::new();
    for component in components {
        if let ParentDir = component {
            while matches!(path.components().next_back(), Some(Component::CurDir)) {
                path.pop();
            }
            if path.as_os_str().is_empty() {
                path.push(current_dir_opt.take()?);
            }
            if !path.pop() && !saturate_at_root {
                return None;
            }
        } else {
            path.push(component);
        }
    }

    if (path.as_os_str().is_empty() || path == current_dir) && was_relative {
        Cow::Borrowed(Path::new("."))
    } else {
        path.into()
    }
    .into()
}

/// Like [`normalize()`], but also removes `.` components and duplicate or trailing separators.
///
/// If cleaning leaves no components, `current_dir` is returned. Already-clean borrowed paths remain borrowed.
pub fn normalize_and_clean<'a>(path: Cow<'a, Path>, current_dir: &Path) -> Option<Cow<'a, Path>> {
    fn needs_cleaning(path: &Path) -> bool {
        use std::path::Component::CurDir;

        if path.as_os_str().is_empty() || path.components().any(|component| matches!(component, CurDir)) {
            return true;
        }

        let mut components = path
            .as_os_str()
            .as_encoded_bytes()
            .split(|byte| std::path::is_separator(*byte as char));
        let Some(first) = components.next() else { return true };
        first == b"." || components.any(|component| component.is_empty() || component == b".")
    }

    let path = normalize(path, current_dir)?;
    if !needs_cleaning(path.as_ref()) {
        return Some(path);
    }

    let mut cleaned: PathBuf = path
        .components()
        .filter(|component| !matches!(component, Component::CurDir))
        .collect();
    if cleaned.as_os_str().is_empty() {
        cleaned.push(current_dir);
    }
    if cleaned.as_os_str() == path.as_os_str() {
        Some(path)
    } else {
        Some(Cow::Owned(cleaned))
    }
}

/// Rebuild `relative_path` to be relative to `prefix`, such as the user's current working directory.
///
/// Both inputs must be [normalized](normalize()). Relative inputs must use the same implicit root;
/// absolute inputs have their shared filesystem root stripped internally. This only manipulates path
/// components, without accessing the filesystem or resolving symbolic links. Comparisons are case-sensitive.
///
/// Return `relative_path` unchanged if `prefix` is empty, the inputs mix absolute and relative paths,
/// or their filesystem roots differ (for example, different Windows drives or UNC shares).
/// Anchor mixed inputs to a common root first if a relative result is needed.
///
/// For example, target `/repo/m1` and prefix `/repo/a/b` produce `../../m1`.
pub fn relativize_with_prefix<'a>(relative_path: &'a Path, prefix: &Path) -> Cow<'a, Path> {
    if prefix.as_os_str().is_empty() || relative_path.is_absolute() != prefix.is_absolute() {
        return Cow::Borrowed(relative_path);
    }
    let (relative_path, prefix) = if prefix.is_absolute() {
        let root = prefix
            .ancestors()
            .last()
            .expect("absolute paths have a filesystem root");
        match (relative_path.strip_prefix(root), prefix.strip_prefix(root)) {
            (Ok(path), Ok(prefix)) => (path, prefix),
            _ => return Cow::Borrowed(relative_path),
        }
    } else {
        (relative_path, prefix)
    };
    debug_assert!(
        relative_path.components().all(|c| matches!(c, Component::Normal(_))),
        "BUG: all input is expected to be normalized, but relative_path was not"
    );
    debug_assert!(
        prefix.components().all(|c| matches!(c, Component::Normal(_))),
        "BUG: all input is expected to be normalized, but prefix was not"
    );

    let mut buf = PathBuf::new();
    let mut rpc = relative_path.components().peekable();
    let mut equal_thus_far = true;
    for pcomp in prefix.components() {
        if equal_thus_far && let (Component::Normal(pname), Some(Component::Normal(rpname))) = (pcomp, rpc.peek()) {
            if &pname == rpname {
                rpc.next();
                continue;
            } else {
                equal_thus_far = false;
            }
        }
        buf.push(Component::ParentDir);
    }
    buf.extend(rpc);
    if buf.as_os_str().is_empty() {
        Cow::Borrowed(Path::new("."))
    } else {
        Cow::Owned(buf)
    }
}
