// Copyright 2025 FastLabs Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

/// Creates an [`Exn`] and returns it as [`Result`].
///
/// Shorthand for `return Err(Exn::from(err))`.
///
/// # Examples
///
/// Create an [`Exn`] from [`Error`]:
///
/// [`Exn`]: crate::Exn
/// [`Error`]: std::error::Error
///
/// ```
/// use std::fs;
///
/// use gix_error::bail;
/// # fn wrapper() -> Result<(), gix_error::Exn<std::io::Error>> {
/// match fs::read_to_string("/path/to/file") {
///     Ok(content) => println!("file contents: {content}"),
///     Err(err) => bail!(err),
/// }
/// # Ok(()) }
/// ```
#[macro_export]
macro_rules! bail {
    ($err:expr) => {{
        return ::std::result::Result::Err($crate::Exn::from($err));
    }};
}

/// Ensures `$cond` is met; otherwise return an error.
///
/// Shorthand for `if !$cond { bail!(...); }`.
///
/// # Examples
///
/// Create an [`Exn`] from an [`Error`]:
///
/// [`Exn`]: crate::Exn
/// [`Error`]: std::error::Error
///
/// ```
/// # fn has_permission(_: &u32, _: &u32) -> bool { true }
/// # type User = u32;
/// # let user = 0;
/// # type Resource = u32;
/// # let resource = 0;
/// use std::error::Error;
/// use std::fmt;
///
/// use gix_error::ensure;
///
/// #[derive(Debug)]
/// struct PermissionDenied(User, Resource);
///
/// impl fmt::Display for PermissionDenied {
///     fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
///         write!(fmt, "permission denied")
///     }
/// }
///
/// impl Error for PermissionDenied {}
///
/// ensure!(
///     has_permission(&user, &resource),
///     PermissionDenied(user, resource),
/// );
/// # Ok(())
/// ```
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $err:expr $(,)?) => {{
        if !bool::from($cond) {
            $crate::bail!($err)
        }
    }};
}
