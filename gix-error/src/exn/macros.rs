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

/// Creates an [`Exn`] and converts it to the function's result error type.
///
/// Shorthand for `return Err(Exn::from(err).into())`.
/// Works with both [`crate::Result`] and typed [`crate::ExnResult`].
/// Prefer to import the macro with `use gix_error::bail;` (or `use gix::error::bail;` in applications)
/// and invoke it as `bail!(...)` instead of using a qualified path for readability.
///
/// String literals and format arguments implicitly construct a formatted [`Message`](crate::Message),
/// including captured arguments like `bail!("invalid input: {input}")`.
/// Like [`message!`](crate::message!), this always formats the message; use [`message()`](crate::message())
/// inside `bail!` for a static message without formatting.
/// A string literal can be followed by method calls on the formatted [`Message`](crate::Message),
/// such as `bail!("invalid record at {offset}".corrupted().with("offset", offset))`.
/// Explicit format arguments follow the method chain: `bail!("invalid record at {}".corrupted(), offset)`.
/// These methods apply to the message after formatting, not to the string literal itself.
/// Parenthesize an ordinary error expression starting with a literal to avoid this shorthand,
/// for example `bail!(("bad".parse::<u32>().expect_err("invalid number")))`.
/// Without a classification builder, the shorthand leaves the error unclassified.
/// Classified constructors also work, such as `bail!(gix_error::validation("invalid input"))`.
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
/// use gix_error::{bail, ExnResult};
/// # fn wrapper() -> ExnResult<(), std::io::Error> {
/// match fs::read_to_string("/path/to/file") {
///     Ok(content) => println!("file contents: {content}"),
///     Err(err) => bail!(err),
/// }
/// # Ok(()) }
/// ```
///
/// ```
/// use gix_error::{bail, Result};
///
/// fn public_api() -> Result {
///     bail!(gix_error::validation("invalid input"));
/// }
/// assert!(public_api().expect_err("the input is invalid").is_validation());
/// ```
///
/// Return a formatted message with captured or explicit arguments:
///
/// ```
/// use gix_error::{bail, ExnMessageResult, Result};
///
/// fn captured(name: &str) -> Result {
///     bail!("unknown executable '{name}'");
/// }
///
/// fn explicit(name: &str) -> ExnMessageResult {
///     bail!("unknown executable '{}'", name);
/// }
/// # assert!(captured("other").is_err());
/// # assert!(explicit("other").is_err());
/// ```
///
/// Chain classification and metadata builders on a formatted message:
///
/// ```
/// use std::path::Path;
/// use gix_error::{bail, ExnMessageResult, MetadataValue, Result};
///
/// fn captured(path: &Path) -> Result {
///     bail!("Missing reference".not_found().with("path", path));
/// }
///
/// fn explicit(path: &Path) -> ExnMessageResult {
///     bail!("Missing reference at '{}'".not_found().with("path", path), path.display());
/// }
///
/// let path = Path::new("refs/heads/main");
/// let error = captured(path).expect_err("the reference is missing");
/// assert!(error.is_not_found());
/// assert_eq!(error.metadata().next().expect("reference details")["path"], MetadataValue::Path(path.into()));
/// let error = explicit(path).expect_err("the reference is missing");
/// assert_eq!(error.error().message, "Missing reference at 'refs/heads/main'");
/// assert!(error.is_not_found());
/// ```
#[macro_export]
macro_rules! bail {
    ($fmt:literal $(.$method:ident($($method_arg:tt)*))+ $(, $($arg:tt)*)?) => {
        $crate::bail!($crate::message!($fmt $(, $($arg)*)?) $(.$method($($method_arg)*))+)
    };
    ($fmt:literal $(,)?) => {
        $crate::bail!($crate::message!($fmt))
    };
    // Strip the grouping used to opt out of builder shorthand before it can trigger `unused_parens`.
    (($err:expr) $(,)?) => {
        $crate::bail!($err)
    };
    ($err:expr $(,)?) => {{
        return ::std::result::Result::Err($crate::Exn::from($err).into());
    }};
    ($fmt:expr, $($arg:tt)*) => {
        $crate::bail!($crate::message!($fmt, $($arg)*))
    };
}

/// Ensures `$cond` is met; otherwise return an error.
///
/// Shorthand for `if !$cond { bail!(...); }`.
/// Accepts the same error expressions, format arguments, and message builder chains as [`bail!`].
/// The condition is evaluated once; error, format, and builder arguments are evaluated only on failure.
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
/// # Ok::<(), gix_error::Error>(())
/// ```
///
/// Format a classified failure and attach metadata:
///
/// ```
/// use gix_error::{ensure, ExnMessageResult, MetadataValue, Result};
///
/// fn captured(count: usize) -> Result {
///     ensure!(count > 0, "Count must be positive, got {count}".validation().with_input(count));
///     Ok(())
/// }
///
/// fn explicit(count: usize) -> ExnMessageResult {
///     ensure!(count > 0, "Count must be positive, got {}".validation().with_input(count), count);
///     Ok(())
/// }
///
/// captured(1)?;
/// let error = captured(0).expect_err("zero violates the input constraint");
/// assert!(error.is_validation());
/// assert_eq!(error.metadata().next().expect("input details")["input"], MetadataValue::U64(0));
/// let error = explicit(0).expect_err("zero violates the input constraint");
/// assert_eq!(error.error().message, "Count must be positive, got 0");
/// assert!(error.is_validation());
/// # Ok::<(), gix_error::Error>(())
/// ```
#[macro_export]
macro_rules! ensure {
    ($cond:expr, $($err:tt)+) => {{
        if !bool::from($cond) {
            $crate::bail!($($err)+)
        }
    }};
}

/// Construct a [`Message`](crate::Message) from a string literal or format string.
/// Note that it always runs `format!()`, use the [`message()`](crate::message()) function for literals instead.
#[macro_export]
macro_rules! message {
    ($message_with_format_args:literal $(,)?) => {
        $crate::Message::new(format!($message_with_format_args))
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::Message::new(format!($fmt, $($arg)*))
    };
}
