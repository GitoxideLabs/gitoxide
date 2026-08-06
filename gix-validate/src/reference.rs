use bstr::{BStr, BString, ByteSlice};

///
pub mod name {
    use bstr::BString;
    use std::convert::Infallible;

    /// The error used in [name()][super::name()] and [`name_partial()`][super::name_partial()]
    #[derive(Debug)]
    #[expect(missing_docs)]
    #[non_exhaustive]
    pub enum Error {
        InvalidByte { byte: BString },
        StartsWithSlash,
        RepeatedSlash,
        RepeatedDot,
        LockFileSuffix,
        ReflogPortion,
        Asterisk,
        StartsWithDot,
        EndsWithDot,
        EndsWithSlash,
        Empty,
        SomeLowercase,
        Reserved { name: BString },
    }

    impl std::fmt::Display for Error {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Error::InvalidByte { byte } => write!(f, "Reference name contains invalid byte: {byte:?}"),
                Error::StartsWithSlash => write!(f, "Reference name cannot start with a slash"),
                Error::RepeatedSlash => write!(f, "Reference name cannot contain repeated slashes"),
                Error::RepeatedDot => write!(f, "Reference name cannot contain repeated dots"),
                Error::LockFileSuffix => write!(f, "Reference name cannot end with '.lock'"),
                Error::ReflogPortion => write!(f, "Reference name cannot contain '@{{'"),
                Error::Asterisk => write!(f, "Reference name cannot contain '*'"),
                Error::StartsWithDot => write!(f, "Reference name cannot start with a dot"),
                Error::EndsWithDot => write!(f, "Reference name cannot end with a dot"),
                Error::EndsWithSlash => write!(f, "Reference name cannot end with a slash"),
                Error::Empty => write!(f, "Reference name cannot be empty"),
                Error::SomeLowercase => write!(f, "Standalone references must be all uppercased, like 'HEAD'"),
                Error::Reserved { name } => write!(f, "Reference name is reserved and cannot be used: {name:?}"),
            }
        }
    }

    impl std::error::Error for Error {}

    impl From<crate::tag::name::Error> for Error {
        fn from(err: crate::tag::name::Error) -> Self {
            match err {
                crate::tag::name::Error::InvalidByte { byte } => Error::InvalidByte { byte },
                crate::tag::name::Error::StartsWithSlash => Error::StartsWithSlash,
                crate::tag::name::Error::RepeatedSlash => Error::RepeatedSlash,
                crate::tag::name::Error::RepeatedDot => Error::RepeatedDot,
                crate::tag::name::Error::LockFileSuffix => Error::LockFileSuffix,
                crate::tag::name::Error::ReflogPortion => Error::ReflogPortion,
                crate::tag::name::Error::Asterisk => Error::Asterisk,
                crate::tag::name::Error::StartsWithDot => Error::StartsWithDot,
                crate::tag::name::Error::EndsWithDot => Error::EndsWithDot,
                crate::tag::name::Error::EndsWithSlash => Error::EndsWithSlash,
                crate::tag::name::Error::Empty => Error::Empty,
            }
        }
    }

    impl From<Infallible> for Error {
        fn from(_: Infallible) -> Self {
            unreachable!("this impl is needed to allow passing a known valid partial path as parameter")
        }
    }
}

/// Validate a reference name running all the tests in the book. This disallows lower-case references like `lower`, but also allows
/// ones like `HEAD`, and `refs/lower`.
pub fn name(path: &BStr) -> Result<&BStr, name::Error> {
    match validate(path, Mode::Complete)? {
        None => Ok(path),
        Some(_) => {
            unreachable!("Without sanitization, there is no chance a sanitized version is returned.")
        }
    }
}

/// Validate a reference name for use as a local branch.
///
/// This is like [`name()`], but also rejects `refs/heads/HEAD`, matching Git's branch-specific validation.
pub fn branch_name(path: &BStr) -> Result<&BStr, name::Error> {
    let path = name(path)?;
    if path == "refs/heads/HEAD" {
        return Err(name::Error::Reserved { name: path.into() });
    }
    Ok(path)
}

/// Validate a partial reference name. As it is assumed to be partial, names like `some-name` is allowed
/// even though these would be disallowed with when using [`name()`].
pub fn name_partial(path: &BStr) -> Result<&BStr, name::Error> {
    match validate(path, Mode::Partial)? {
        None => Ok(path),
        Some(_) => {
            unreachable!("Without sanitization, there is no chance a sanitized version is returned.")
        }
    }
}

/// The infallible version of [`name_partial()`] which instead of failing, alters `path` and returns it to be a valid
/// partial name, which would also pass [`name_partial()`].
///
/// Note that an empty `path` is replaced with a `-` in order to be valid.
pub fn name_partial_or_sanitize(path: &BStr) -> BString {
    validate(path, Mode::PartialSanitize)
        .expect("BUG: errors cannot happen as any issue is fixed instantly")
        .expect("we always rebuild the path")
}

enum Mode {
    Complete,
    Partial,
    /// like Partial, but instead of failing, a sanitized version is returned.
    PartialSanitize,
}

fn validate(path: &BStr, mode: Mode) -> Result<Option<BString>, name::Error> {
    let mut out = crate::tag::name_inner(
        path,
        match mode {
            Mode::Complete | Mode::Partial => crate::tag::Mode::Validate,
            Mode::PartialSanitize => crate::tag::Mode::Sanitize,
        },
    )?;
    // `@` alone is shorthand for `HEAD` in revspecs, so Git refuses it as a reference name.
    // Git substitutes it with `-` when sanitizing it, which is what we do here too. Note that `@` stays
    // valid as a component, which is why `refs/heads/@` and a tag named `@` are both fine.
    if out.as_ref().map_or(path, |b| b.as_bstr()) == "@" {
        match out.as_mut() {
            Some(out) => out[0] = b'-',
            None => return Err(name::Error::Reserved { name: path.into() }),
        }
    }
    if let Mode::Complete = mode {
        let input = out.as_ref().map_or(path, |b| b.as_bstr());
        let saw_slash = input.find_byte(b'/').is_some();
        if !saw_slash && !input.iter().all(|c| c.is_ascii_uppercase() || *c == b'_') {
            return Err(name::Error::SomeLowercase);
        }
    }
    Ok(out)
}
