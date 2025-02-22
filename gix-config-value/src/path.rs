use std::{borrow::Cow, path::PathBuf};

use bstr::BStr;

use crate::Path;

///
pub mod interpolate {
    use std::path::PathBuf;

    /// Options for interpolating paths with [`Path::interpolate()`][crate::Path::interpolate()].
    #[derive(Clone, Copy)]
    pub struct Context<'a> {
        /// The location where gitoxide or git is installed. If `None`, `%(prefix)` in paths will cause an error.
        pub git_install_dir: Option<&'a std::path::Path>,
        /// The home directory of the current user. If `None`, `~/` in paths will cause an error.
        pub home_dir: Option<&'a std::path::Path>,
        /// A function returning the home directory of a given user. If `None`, `~name/` in paths will cause an error.
        pub home_for_user: Option<fn(&str) -> Option<PathBuf>>,
    }

    impl Default for Context<'_> {
        fn default() -> Self {
            Context {
                git_install_dir: None,
                home_dir: None,
                home_for_user: Some(home_for_user),
            }
        }
    }

    /// The error returned by [`Path::interpolate()`][crate::Path::interpolate()].
    #[derive(Debug, thiserror::Error)]
    #[allow(missing_docs)]
    pub enum Error {
        #[error("{} is missing", .what)]
        Missing { what: &'static str },
        #[error("Ill-formed UTF-8 in {}", .what)]
        Utf8Conversion {
            what: &'static str,
            #[source]
            err: gix_path::Utf8Error,
        },
        #[error("Ill-formed UTF-8 in username")]
        UsernameConversion(#[from] std::str::Utf8Error),
        #[error("User interpolation is not available on this platform")]
        UserInterpolationUnsupported,
    }

    /// Obtain the home directory for the given user `name` or return `None` if the user wasn't found
    /// or any other error occurred.
    /// It can be used as `home_for_user` parameter in [`Path::interpolate()`][crate::Path::interpolate()].
    #[cfg_attr(windows, allow(unused_variables))]
    #[cfg_attr(all(target_family = "wasm", not(target_os = "emscripten")), allow(unused_variables))]
    pub fn home_for_user(name: &str) -> Option<PathBuf> {
        #[cfg(not(any(
            target_os = "android",
            target_os = "windows",
            all(target_family = "wasm", not(target_os = "emscripten"))
        )))]
        {
            let cname = std::ffi::CString::new(name).ok()?;
            // SAFETY: calling this in a threaded program that modifies the pw database is not actually safe.
            //         TODO: use the `*_r` version, but it's much harder to use.
            #[allow(unsafe_code)]
            let pwd = unsafe { libc::getpwnam(cname.as_ptr()) };
            if pwd.is_null() {
                None
            } else {
                use std::os::unix::ffi::OsStrExt;
                // SAFETY: pw_dir is a cstr and it lives as long as… well, we hope nobody changes the pw database while we are at it
                //         from another thread. Otherwise it lives long enough.
                #[allow(unsafe_code)]
                let cstr = unsafe { std::ffi::CStr::from_ptr((*pwd).pw_dir) };
                Some(std::ffi::OsStr::from_bytes(cstr.to_bytes()).into())
            }
        }
        #[cfg(any(
            target_os = "android",
            target_os = "windows",
            all(target_family = "wasm", not(target_os = "emscripten"))
        ))]
        {
            None
        }
    }
}

impl std::ops::Deref for Path<'_> {
    type Target = BStr;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref()
    }
}

impl AsRef<[u8]> for Path<'_> {
    fn as_ref(&self) -> &[u8] {
        self.value.as_ref()
    }
}

impl AsRef<BStr> for Path<'_> {
    fn as_ref(&self) -> &BStr {
        self.value.as_ref()
    }
}

impl<'a> From<Cow<'a, BStr>> for Path<'a> {
    fn from(value: Cow<'a, BStr>) -> Self {
        Path { value }
    }
}

impl<'a> Path<'a> {
    /// Interpolates this path into a path usable on the file system.
    ///
    /// If this path starts with `~/` or `~user/` or `%(prefix)/`
    ///  - `~/` is expanded to the value of `home_dir`. The caller can use the [dirs](https://crates.io/crates/dirs) crate to obtain it.
    ///    If it is required but not set, an error is produced.
    ///  - `~user/` to the specified user’s home directory, e.g `~alice` might get expanded to `/home/alice` on linux, but requires
    ///    the `home_for_user` function to be provided.
    ///    The interpolation uses `getpwnam` sys call and is therefore not available on windows.
    ///  - `%(prefix)/` is expanded to the location where `gitoxide` is installed.
    ///    This location is not known at compile time and therefore need to be
    ///    optionally provided by the caller through `git_install_dir`.
    ///
    /// Any other, non-empty path value is returned unchanged and error is returned in case of an empty path value or if required input
    /// wasn't provided.
    pub fn interpolate(
        self,
        interpolate::Context {
            git_install_dir,
            home_dir,
            home_for_user,
        }: interpolate::Context<'_>,
    ) -> Result<Cow<'a, std::path::Path>, interpolate::Error> {
        if self.is_empty() {
            return Err(interpolate::Error::Missing { what: "path" });
        }

        const PREFIX: &[u8] = b"%(prefix)/";
        const USER_HOME: &[u8] = b"~/";
        if self.starts_with(PREFIX) {
            let git_install_dir = git_install_dir.ok_or(interpolate::Error::Missing {
                what: "git install dir",
            })?;
            let (_prefix, path_without_trailing_slash) = self.split_at(PREFIX.len());
            let path_without_trailing_slash =
                gix_path::try_from_bstring(path_without_trailing_slash).map_err(|err| {
                    interpolate::Error::Utf8Conversion {
                        what: "path past %(prefix)",
                        err,
                    }
                })?;
            Ok(git_install_dir.join(path_without_trailing_slash).into())
        } else if self.starts_with(USER_HOME) {
            let home_path = home_dir.ok_or(interpolate::Error::Missing { what: "home dir" })?;
            let (_prefix, val) = self.split_at(USER_HOME.len());
            let val = gix_path::try_from_byte_slice(val).map_err(|err| interpolate::Error::Utf8Conversion {
                what: "path past ~/",
                err,
            })?;
            Ok(home_path.join(val).into())
        } else if self.starts_with(b"~") && self.contains(&b'/') {
            self.interpolate_user(home_for_user.ok_or(interpolate::Error::Missing {
                what: "home for user lookup",
            })?)
        } else {
            Ok(gix_path::from_bstr(self.value))
        }
    }

    #[cfg(any(target_os = "windows", target_os = "android"))]
    fn interpolate_user(
        self,
        _home_for_user: fn(&str) -> Option<PathBuf>,
    ) -> Result<Cow<'a, std::path::Path>, interpolate::Error> {
        Err(interpolate::Error::UserInterpolationUnsupported)
    }

    #[cfg(not(any(target_os = "windows", target_os = "android")))]
    fn interpolate_user(
        self,
        home_for_user: fn(&str) -> Option<PathBuf>,
    ) -> Result<Cow<'a, std::path::Path>, interpolate::Error> {
        let (_prefix, val) = self.split_at("/".len());
        let i = val
            .iter()
            .position(|&e| e == b'/')
            .ok_or(interpolate::Error::Missing { what: "/" })?;
        let (username, path_with_leading_slash) = val.split_at(i);
        let username = std::str::from_utf8(username)?;
        let home = home_for_user(username).ok_or(interpolate::Error::Missing { what: "pwd user info" })?;
        let path_past_user_prefix =
            gix_path::try_from_byte_slice(&path_with_leading_slash["/".len()..]).map_err(|err| {
                interpolate::Error::Utf8Conversion {
                    what: "path past ~user/",
                    err,
                }
            })?;
        Ok(home.join(path_past_user_prefix).into())
    }
}
