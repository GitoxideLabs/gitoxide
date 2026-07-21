use bstr::ByteSlice;

use crate::{File, KeyRef, file, file::init, parse::section, path::interpolate};

/// Represents the errors that may occur when calling [`File::from_env()`].
#[derive(Debug)]
#[expect(missing_docs)]
pub enum Error {
    IllformedUtf8 { index: usize, kind: &'static str },
    InvalidConfigCount { input: String },
    InvalidKeyId { key_id: usize },
    InvalidKeyValue { key_id: usize, key_val: String },
    InvalidValueId { value_id: usize },
    PathInterpolationError(interpolate::Error),
    Includes(init::includes::Error),
    Section(section::header::Error),
    SectionValue(file::section::value::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IllformedUtf8 { index, kind } => {
                write!(f, "Configuration {kind} at index {index} contained illformed UTF-8")
            }
            Error::InvalidConfigCount { input } => {
                write!(f, "GIT_CONFIG_COUNT was not a positive integer: {input}")
            }
            Error::InvalidKeyId { key_id } => write!(f, "GIT_CONFIG_KEY_{key_id} was not set"),
            Error::InvalidKeyValue { key_id, key_val } => {
                write!(f, "GIT_CONFIG_KEY_{key_id} was set to an invalid value: {key_val}")
            }
            Error::InvalidValueId { value_id } => write!(f, "GIT_CONFIG_VALUE_{value_id} was not set"),
            Error::PathInterpolationError(err) => std::fmt::Display::fmt(err, f),
            Error::Includes(err) => std::fmt::Display::fmt(err, f),
            Error::Section(err) => std::fmt::Display::fmt(err, f),
            Error::SectionValue(err) => std::fmt::Display::fmt(err, f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::IllformedUtf8 { .. }
            | Error::InvalidConfigCount { .. }
            | Error::InvalidKeyId { .. }
            | Error::InvalidKeyValue { .. }
            | Error::InvalidValueId { .. } => None,
            Error::PathInterpolationError(err) => err.source(),
            Error::Includes(err) => err.source(),
            Error::Section(err) => err.source(),
            Error::SectionValue(err) => err.source(),
        }
    }
}

impl From<interpolate::Error> for Error {
    fn from(err: interpolate::Error) -> Self {
        Error::PathInterpolationError(err)
    }
}

impl From<init::includes::Error> for Error {
    fn from(err: init::includes::Error) -> Self {
        Error::Includes(err)
    }
}

impl From<section::header::Error> for Error {
    fn from(err: section::header::Error) -> Self {
        Error::Section(err)
    }
}

impl From<file::section::value::Error> for Error {
    fn from(err: file::section::value::Error) -> Self {
        Error::SectionValue(err)
    }
}

/// Instantiation from environment variables
impl File {
    /// Generates a config from `GIT_CONFIG_*` environment variables or returns `Ok(None)` if no configuration was found.
    /// See [`git-config`'s documentation] for more information on the environment variables in question.
    ///
    /// With `options` configured, it's possible to resolve `include.path` or `includeIf.<condition>.path` directives as well.
    ///
    /// [`git-config`'s documentation]: https://git-scm.com/docs/git-config#Documentation/git-config.txt-GITCONFIGCOUNT
    pub fn from_env(options: init::Options<'_>) -> Result<Option<File>, Error> {
        use std::env;
        let count: usize = match env::var("GIT_CONFIG_COUNT") {
            Ok(v) => v.parse().map_err(|_| Error::InvalidConfigCount { input: v })?,
            Err(_) => return Ok(None),
        };

        if count == 0 {
            return Ok(None);
        }

        let meta = file::Metadata {
            path: None,
            source: crate::Source::Env,
            level: 0,
            trust: gix_sec::Trust::Full,
        };
        let mut config = File::new(meta);
        for i in 0..count {
            let key = gix_path::os_string_into_bstring(
                env::var_os(format!("GIT_CONFIG_KEY_{i}")).ok_or(Error::InvalidKeyId { key_id: i })?,
            )
            .map_err(|_| Error::IllformedUtf8 { index: i, kind: "key" })?;
            let value = env::var_os(format!("GIT_CONFIG_VALUE_{i}")).ok_or(Error::InvalidValueId { value_id: i })?;
            let key = KeyRef::parse_unvalidated(key.as_ref()).ok_or_else(|| Error::InvalidKeyValue {
                key_id: i,
                key_val: key.to_string(),
            })?;

            config
                .section_mut_or_create_new_inner(key.section_name, key.subsection_name)?
                .push(
                    key.value_name,
                    Some(
                        gix_path::os_str_into_bstr(&value)
                            .map_err(|_| Error::IllformedUtf8 {
                                index: i,
                                kind: "value",
                            })?
                            .as_bytes()
                            .into(),
                    ),
                )?;
        }

        let mut buf = Vec::new();
        init::includes::resolve(&mut config, &mut buf, options)?;
        Ok(Some(config))
    }
}
