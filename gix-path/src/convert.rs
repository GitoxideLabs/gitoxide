use std::{
    borrow::Cow,
    ffi::{OsStr, OsString},
    path::{Component, Path, PathBuf},
};

use bstr::{BStr, BString};

#[derive(Debug)]
/// The error type returned by [`into_bstr()`] and others may suffer from failed conversions from or to bytes.
pub struct Utf8Error;

impl std::fmt::Display for Utf8Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Could not convert to UTF8 or from UTF8 due to ill-formed input")
    }
}

impl std::error::Error for Utf8Error {}

/// Like [`into_bstr()`], but takes `OsStr` as input for a lossless, but fallible, conversion.
pub fn os_str_into_bstr(path: &OsStr) -> Result<&BStr, Utf8Error> {
    let path = try_into_bstr(Cow::Borrowed(path.as_ref()))?;
    match path {
        Cow::Borrowed(path) => Ok(path),
        Cow::Owned(_) => unreachable!("borrowed cows stay borrowed"),
    }
}

/// Like [`into_bstr()`], but takes `OsString` as input for a lossless, but fallible, conversion.
pub fn os_string_into_bstring(path: OsString) -> Result<BString, Utf8Error> {
    let path = try_into_bstr(Cow::Owned(path.into()))?;
    match path {
        Cow::Borrowed(_path) => unreachable!("borrowed cows stay borrowed"),
        Cow::Owned(path) => Ok(path),
    }
}

/// Like [`into_bstr()`], but takes `Cow<OsStr>` as input for a lossless, but fallible, conversion.
pub fn try_os_str_into_bstr(path: Cow<'_, OsStr>) -> Result<Cow<'_, BStr>, Utf8Error> {
    match path {
        Cow::Borrowed(path) => os_str_into_bstr(path).map(Cow::Borrowed),
        Cow::Owned(path) => os_string_into_bstring(path).map(Cow::Owned),
    }
}

/// Convert the given path either into its raw bytes on Unix or its UTF-8 encoded counterpart on Windows.
///
/// On Windows, if the source `Path`` contains ill-formed, lone surrogates, the UTF-8 conversion will fail
/// causing `Utf8Error` to be returned.
pub fn try_into_bstr<'a>(path: impl Into<Cow<'a, Path>>) -> Result<Cow<'a, BStr>, Utf8Error> {
    let path = path.into();
    let path_str = match path {
        Cow::Owned(path) => Cow::Owned({
            #[cfg(unix)]
            let p: BString = {
                use std::os::unix::ffi::OsStringExt;
                path.into_os_string().into_vec().into()
            };
            #[cfg(target_os = "wasi")]
            let p: BString = {
                use std::os::wasi::ffi::OsStringExt;
                path.into_os_string().into_vec().into()
            };
            #[cfg(not(any(unix, target_os = "wasi")))]
            let p: BString = path.into_os_string().into_string().map_err(|_| Utf8Error)?.into();
            p
        }),
        Cow::Borrowed(path) => Cow::Borrowed({
            #[cfg(unix)]
            let p: &BStr = {
                use std::os::unix::ffi::OsStrExt;
                path.as_os_str().as_bytes().into()
            };
            #[cfg(target_os = "wasi")]
            let p: &BStr = {
                use std::os::wasi::ffi::OsStrExt;
                path.as_os_str().as_bytes().into()
            };
            #[cfg(not(any(unix, target_os = "wasi")))]
            let p: &BStr = path.to_str().ok_or(Utf8Error)?.as_bytes().into();
            p
        }),
    };
    Ok(path_str)
}

/// Similar to [`try_into_bstr()`] but **panics** if malformed surrogates are encountered on Windows.
pub fn into_bstr<'a>(path: impl Into<Cow<'a, Path>>) -> Cow<'a, BStr> {
    try_into_bstr(path).expect("prefix path doesn't contain ill-formed UTF-8")
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
/// On Windows, the input is required to be valid UTF-8, which is guaranteed if we wrote it before.
/// There are some potential Git versions and Windows installations which produce malformed UTF-16
/// if certain emojis are in the path. It's as rare as it sounds, but possible.
pub fn try_from_byte_slice(input: &[u8]) -> Result<&Path, Utf8Error> {
    #[cfg(unix)]
    let p = {
        use std::os::unix::ffi::OsStrExt;
        OsStr::from_bytes(input).as_ref()
    };
    #[cfg(target_os = "wasi")]
    let p: &Path = {
        use std::os::wasi::ffi::OsStrExt;
        OsStr::from_bytes(input).as_ref()
    };
    #[cfg(not(any(unix, target_os = "wasi")))]
    let p = Path::new(std::str::from_utf8(input).map_err(|_| Utf8Error)?);
    Ok(p)
}

/// Similar to [`from_byte_slice()`], but takes either borrowed or owned `input`.
pub fn try_from_bstr<'a>(input: impl Into<Cow<'a, BStr>>) -> Result<Cow<'a, Path>, Utf8Error> {
    let input = input.into();
    match input {
        Cow::Borrowed(input) => try_from_byte_slice(input).map(Cow::Borrowed),
        Cow::Owned(input) => try_from_bstring(input).map(Cow::Owned),
    }
}

/// Similar to [`try_from_bstr()`], but **panics** if malformed surrogates are encountered on Windows.
pub fn from_bstr<'a>(input: impl Into<Cow<'a, BStr>>) -> Cow<'a, Path> {
    try_from_bstr(input).expect("prefix path doesn't contain ill-formed UTF-8")
}

/// Similar to [`try_from_bstr()`], but takes and produces owned data.
pub fn try_from_bstring(input: impl Into<BString>) -> Result<PathBuf, Utf8Error> {
    let input = input.into();
    #[cfg(unix)]
    let p = {
        use std::os::unix::ffi::OsStringExt;
        std::ffi::OsString::from_vec(input.into()).into()
    };
    #[cfg(target_os = "wasi")]
    let p: PathBuf = {
        use std::os::wasi::ffi::OsStringExt;
        std::ffi::OsString::from_vec(input.into()).into()
    };
    #[cfg(not(any(unix, target_os = "wasi")))]
    let p = {
        use bstr::ByteVec;
        PathBuf::from(
            {
                let v: Vec<_> = input.into();
                v
            }
            .into_string()
            .map_err(|_| Utf8Error)?,
        )
    };
    Ok(p)
}

/// Similar to [`try_from_bstring()`], but will **panic** if there is ill-formed UTF-8 in the `input`.
pub fn from_bstring(input: impl Into<BString>) -> PathBuf {
    try_from_bstring(input).expect("well-formed UTF-8 on windows")
}

/// Similar to [`try_from_byte_slice()`], but will **panic** if there is ill-formed UTF-8 in the `input`.
pub fn from_byte_slice(input: &[u8]) -> &Path {
    try_from_byte_slice(input).expect("well-formed UTF-8 on windows")
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
/// but **panic** if unpaired surrogates are encountered on Windows.
pub fn to_native_path_on_windows<'a>(path: impl Into<Cow<'a, BStr>>) -> Cow<'a, std::path::Path> {
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
            let path_was_dot = path == Path::new(".");
            if path.as_os_str().is_empty() || path_was_dot {
                path.push(current_dir_opt.take()?);
            }
            if !path.pop() {
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

/// Rebuild the worktree-relative `relative_path` to be relative to `prefix`, which is the
/// worktree-relative path equivalent to the position of the user, or current working directory.
///
/// This is a no-op if `prefix` is empty.
///
/// Note that both `relative_path` and `prefix` are assumed to be [normalized](normalize()), and
/// failure to do so will lead to incorrect results.
///
/// Note that both input paths are expected to be equal in terms of case too, as comparisons will
/// be case-sensitive.
pub fn relativize_with_prefix<'a>(relative_path: &'a Path, prefix: &Path) -> Cow<'a, Path> {
    if prefix.as_os_str().is_empty() {
        return Cow::Borrowed(relative_path);
    }
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
        if equal_thus_far {
            if let (Component::Normal(pname), Some(Component::Normal(rpname))) = (pcomp, rpc.peek()) {
                if &pname == rpname {
                    rpc.next();
                    continue;
                } else {
                    equal_thus_far = false;
                }
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
