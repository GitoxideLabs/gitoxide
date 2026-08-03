//! Functions for expanding repository paths.
use std::path::{Path, PathBuf};

use bstr::{BStr, BString, ByteSlice};

/// The user whose home directory a repository path refers to.
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
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs)]
pub enum Error {
    #[error("UTF8 conversion on non-unix system failed for path: {path:?}")]
    IllformedUtf8 { path: BString },
    #[error("Home directory could not be obtained for {}", match user {Some(user) => format!("user '{user}'"), None => "current user".into()})]
    MissingHome { user: Option<BString> },
}

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
/// * `/~/repopath` - the currently logged-in user's home, returning `/repopath`.
/// * `/~user/repopath` - the named user's home, returning `/repopath`.
///
/// Paths without a leading slash or home marker are returned unchanged without user information.
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

/// Convert a leading URL-style `/~/` or `/~user/` path into shell-style `~/` or `~user/` syntax.
///
/// Other paths are returned unchanged.
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

/// Expand `path` for the given `user`, which can be obtained with [`parse()`], resolving the home directory with
/// `home_for_user(&user)`.
///
/// When `user` is present, `path` must be the slash-prefixed adjusted path returned by [`parse()`]; it is joined below
/// the resolved home directory. With no user, `path` is returned as a platform path without home expansion.
///
/// For the common case consider using [`crate::expand_path()`] instead.
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
