//! A module with utilities to turn byte slices with decimal numbers back into their
//! binary representation.
//!
//! ### Credits
//!
//! This module was ported from <https://github.com/niklasf/rust-btoi> version 0.4.3
//! see <https://github.com/GitoxideLabs/gitoxide/issues/729> for how it came to be in order
//! to save 2.2 seconds of per-core compile time by not compiling the `num-traits` crate
//! anymore.
//!
//! Licensed with compatible licenses [MIT] and [Apache]
//!
//! [MIT]: https://github.com/niklasf/rust-btoi/blob/master/LICENSE-MIT
//! [Apache]: https://github.com/niklasf/rust-btoi/blob/master/LICENSE-APACHE

/// An error that can occur when parsing an integer.
///
/// * No digits
/// * Invalid digit
/// * Overflow
/// * Underflow
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseIntegerError {
    kind: ErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ErrorKind {
    Empty,
    InvalidDigit,
    Overflow,
    Underflow,
}

impl ParseIntegerError {
    fn desc(&self) -> &str {
        match self.kind {
            ErrorKind::Empty => "cannot parse integer without digits",
            ErrorKind::InvalidDigit => "invalid digit found in slice",
            ErrorKind::Overflow => "number too large to fit in target type",
            ErrorKind::Underflow => "number too small to fit in target type",
        }
    }
}

impl std::fmt::Display for ParseIntegerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.desc().fmt(f)
    }
}

impl std::error::Error for ParseIntegerError {
    fn description(&self) -> &str {
        self.desc()
    }
}

/// Converts a byte slice to an integer. Signs are not allowed.
///
/// # Errors
///
/// Returns [`ParseIntegerError`] for any of the following conditions:
///
/// * `bytes` is empty
/// * not all characters of `bytes` are `0-9`
/// * the number overflows `I`
///
/// # Panics
///
/// Panics in the pathological case that there is no representation of `10`
/// in `I`.
///
/// # Examples
///
/// ```
/// # use gix_utils::btoi::to_unsigned;
/// assert_eq!(Ok(12345), to_unsigned(b"12345"));
/// assert!(to_unsigned::<u8>(b"+1").is_err()); // only btoi allows signs
/// assert!(to_unsigned::<u8>(b"256").is_err()); // overflow
/// ```
#[track_caller]
pub fn to_unsigned<I: MinNumTraits>(bytes: &[u8]) -> Result<I, ParseIntegerError> {
    to_unsigned_with_radix(bytes, 10)
}

/// Converts a byte slice in a given base to an integer. Signs are not allowed.
///
/// # Errors
///
/// Returns [`ParseIntegerError`] for any of the following conditions:
///
/// * `bytes` is empty
/// * not all characters of `bytes` are `0-9`, `a-z` or `A-Z`
/// * not all characters refer to digits in the given `radix`
/// * the number overflows `I`
///
/// # Panics
///
/// Panics if `radix` is not in the range `2..=36` (or in the pathological
/// case that there is no representation of `radix` in `I`).
///
/// # Examples
///
/// ```
/// # use gix_utils::btoi::to_unsigned_with_radix;
/// assert_eq!(Ok(255), to_unsigned_with_radix(b"ff", 16));
/// assert_eq!(Ok(42), to_unsigned_with_radix(b"101010", 2));
/// ```
pub fn to_unsigned_with_radix<I: MinNumTraits>(bytes: &[u8], radix: u32) -> Result<I, ParseIntegerError> {
    assert!(
        (2..=36).contains(&radix),
        "radix must lie in the range 2..=36, found {radix}"
    );

    let base = I::from_u32(radix).expect("radix can be represented as integer");

    if bytes.is_empty() {
        return Err(ParseIntegerError { kind: ErrorKind::Empty });
    }

    let mut result = I::ZERO;

    for &digit in bytes {
        let x = match char::from(digit).to_digit(radix).and_then(I::from_u32) {
            Some(x) => x,
            None => {
                return Err(ParseIntegerError {
                    kind: ErrorKind::InvalidDigit,
                });
            }
        };
        result = match result.checked_mul(base) {
            Some(result) => result,
            None => {
                return Err(ParseIntegerError {
                    kind: ErrorKind::Overflow,
                });
            }
        };
        result = match result.checked_add(x) {
            Some(result) => result,
            None => {
                return Err(ParseIntegerError {
                    kind: ErrorKind::Overflow,
                });
            }
        };
    }

    Ok(result)
}

/// Converts a byte slice to an integer.
///
/// Like [`to_unsigned`], but numbers may optionally start with a sign (`-` or `+`).
///
/// # Errors
///
/// Returns [`ParseIntegerError`] for any of the following conditions:
///
/// * `bytes` has no digits
/// * not all characters of `bytes` are `0-9`, excluding an optional leading
///   sign
/// * the number overflows or underflows `I`
///
/// # Panics
///
/// Panics in the pathological case that there is no representation of `10`
/// in `I`.
///
/// # Examples
///
/// ```
/// # use gix_utils::btoi::to_signed;
/// assert_eq!(Ok(123), to_signed(b"123"));
/// assert_eq!(Ok(123), to_signed(b"+123"));
/// assert_eq!(Ok(-123), to_signed(b"-123"));
///
/// assert!(to_signed::<u8>(b"123456789").is_err()); // overflow
/// assert!(to_signed::<u8>(b"-1").is_err()); // underflow
///
/// assert!(to_signed::<i32>(b" 42").is_err()); // leading space
/// ```
pub fn to_signed<I: MinNumTraits>(bytes: &[u8]) -> Result<I, ParseIntegerError> {
    to_signed_with_radix(bytes, 10)
}

/// Converts a byte slice in a given base to an integer.
///
/// Like [`to_unsigned_with_radix`], but numbers may optionally start with a sign
/// (`-` or `+`).
///
/// # Errors
///
/// Returns [`ParseIntegerError`] for any of the following conditions:
///
/// * `bytes` has no digits
/// * not all characters of `bytes` are `0-9`, `a-z`, `A-Z`, excluding an
///   optional leading sign
/// * not all characters refer to digits in the given `radix`, excluding an
///   optional leading sign
/// * the number overflows or underflows `I`
///
/// # Panics
///
/// Panics if `radix` is not in the range `2..=36` (or in the pathological
/// case that there is no representation of `radix` in `I`).
///
/// # Examples
///
/// ```
/// # use gix_utils::btoi::to_signed_with_radix;
/// assert_eq!(Ok(10), to_signed_with_radix(b"a", 16));
/// assert_eq!(Ok(10), to_signed_with_radix(b"+a", 16));
/// assert_eq!(Ok(-42), to_signed_with_radix(b"-101010", 2));
/// ```
pub fn to_signed_with_radix<I: MinNumTraits>(bytes: &[u8], radix: u32) -> Result<I, ParseIntegerError> {
    assert!(
        (2..=36).contains(&radix),
        "radix must lie in the range 2..=36, found {radix}"
    );

    let base = I::from_u32(radix).expect("radix can be represented as integer");

    if bytes.is_empty() {
        return Err(ParseIntegerError { kind: ErrorKind::Empty });
    }

    let digits = match bytes[0] {
        b'+' => return to_unsigned_with_radix(&bytes[1..], radix),
        b'-' => &bytes[1..],
        _ => return to_unsigned_with_radix(bytes, radix),
    };

    if digits.is_empty() {
        return Err(ParseIntegerError { kind: ErrorKind::Empty });
    }

    let mut result = I::ZERO;

    for &digit in digits {
        let x = match char::from(digit).to_digit(radix).and_then(I::from_u32) {
            Some(x) => x,
            None => {
                return Err(ParseIntegerError {
                    kind: ErrorKind::InvalidDigit,
                });
            }
        };
        result = match result.checked_mul(base) {
            Some(result) => result,
            None => {
                return Err(ParseIntegerError {
                    kind: ErrorKind::Underflow,
                });
            }
        };
        result = match result.checked_sub(x) {
            Some(result) => result,
            None => {
                return Err(ParseIntegerError {
                    kind: ErrorKind::Underflow,
                });
            }
        };
    }

    Ok(result)
}

/// minimal subset of traits used by [`to_signed_with_radix`] and [`to_unsigned_with_radix`]
pub trait MinNumTraits: Sized + Copy + TryFrom<u32> {
    /// the 0 value for this type
    const ZERO: Self;
    /// convert from a unsigned 32-bit word
    fn from_u32(n: u32) -> Option<Self> {
        Self::try_from(n).ok()
    }
    /// the checked multiplication operation for this type
    fn checked_mul(self, rhs: Self) -> Option<Self>;
    /// the chekced addition operation for this type
    fn checked_add(self, rhs: Self) -> Option<Self>;
    /// the checked subtraction operation for this type
    fn checked_sub(self, v: Self) -> Option<Self>;
}

macro_rules! impl_checked {
    ($f:ident) => {
        fn $f(self, rhs: Self) -> Option<Self> {
            Self::$f(self, rhs)
        }
    };
}

macro_rules! min_num_traits {
    ($t:ty) => {
        impl MinNumTraits for $t {
            const ZERO: Self = 0;
            impl_checked!(checked_add);
            impl_checked!(checked_mul);
            impl_checked!(checked_sub);
        }
    };
}

min_num_traits!(i32);
min_num_traits!(i64);
min_num_traits!(u64);
min_num_traits!(u8);
min_num_traits!(usize);
