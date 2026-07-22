//! Functions for expanding repository paths.
use std::path::{Path, PathBuf};

use bstr::{BStr, BString, ByteSlice};

/// Whether a repository is resolving for the current user, or the given one.
#[derive(PartialEq, Eq, Debug, Hash, Ord, PartialOrd, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ForUser {
    /// The currently logged in user.
    Current,
    /// The user with the given name.
    Name(BString),
}

impl From<ForUser> for Option<BString> {
    fn from(v: ForUser) -> Self {
        match v {
            ForUser::Name(user) => Some(user),
            ForUser::Current => None,
        }
    }
}

/// The error used by [`parse()`], [`with()`] and [`expand_path()`](crate::expand_path()).
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    IllformedUtf8 { path: BString },
    MissingHome { user: Option<BString> },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IllformedUtf8 { path } => {
                write!(f, "UTF8 conversion on non-unix system failed for path: {path:?}")
            }
            Error::MissingHome { user } => {
                let user: std::borrow::Cow<'_, str> = match user {
                    Some(user) => format!("user '{user}'").into(),
                    None => "current user".into(),
                };
                write!(f, "Home directory could not be obtained for {user}")
            }
        }
    }
}

impl std::error::Error for Error {}

fn path_segments(path: &BStr) -> Option<impl Iterator<Item = &[u8]>> {
    if path.starts_with(b"/") {
        Some(path[1..].split(|c| *c == b'/'))
    } else {
        None
    }
}

/// Parse user information from the given `path`, returning `(possible user information, adjusted input path)`.
///
/// Supported formats for user extraction are…
/// * `~/repopath` - the currently logged in user's home.
/// * `~user/repopath` - the repository in the given user's home.
pub fn parse(path: &BStr) -> Result<(Option<ForUser>, BString), Error> {
    Ok(path_segments(path)
        .and_then(|mut iter| {
            iter.next().map(|segment| {
                if segment.starts_with(b"~") {
                    let eu = if segment.len() == 1 {
                        Some(ForUser::Current)
                    } else {
                        Some(ForUser::Name(segment[1..].into()))
                    };
                    (
                        eu,
                        format!(
                            "/{}",
                            iter.map(|s| s.as_bstr().to_str_lossy()).collect::<Vec<_>>().join("/")
                        )
                        .into(),
                    )
                } else {
                    (None, path.into())
                }
            })
        })
        .unwrap_or_else(|| (None, path.into())))
}

/// Expand `path` for use in a shell and return the expanded path.
pub fn for_shell(path: BString) -> BString {
    use bstr::ByteVec;
    match parse(path.as_slice().as_bstr()) {
        Ok((user, mut path)) => match user {
            Some(ForUser::Current) => {
                path.insert(0, b'~');
                path
            }
            Some(ForUser::Name(mut user)) => {
                user.insert(0, b'~');
                user.append(path.as_vec_mut());
                user
            }
            None => path,
        },
        Err(_) => path,
    }
}

/// Expand `path` for the given `user`, which can be obtained by [`parse()`], resolving them with `home_for_user(&user)`.
///
/// For the common case consider using [`expand_path()]` instead.
pub fn with(
    user: Option<&ForUser>,
    path: &BStr,
    home_for_user: impl FnOnce(&ForUser) -> Option<PathBuf>,
) -> Result<PathBuf, Error> {
    fn make_relative(path: &Path) -> PathBuf {
        path.components().skip(1).collect()
    }
    let path = gix_path::try_from_byte_slice(path).map_err(|_| Error::IllformedUtf8 { path: path.to_owned() })?;
    Ok(match user {
        Some(user) => home_for_user(user)
            .ok_or_else(|| Error::MissingHome {
                user: user.to_owned().into(),
            })?
            .join(make_relative(path)),
        None => path.into(),
    })
}
